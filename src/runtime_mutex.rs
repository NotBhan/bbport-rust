//! Opaque PS4 mutex/condition-variable handles over host pthreads, ported from
//! `legacy/src/runtime_mutex.c`.
//!
//! The guest sees a handle word (its own memory): 0/1 mean "statically initialized, create on
//! first use", 2 means invalid/destroyed, and any other value is the address of the host
//! object. Priority protocols need a PS4 scheduler, so the value is validated and recorded
//! only.

use crate::src::runtime::{orbis, stop};
use std::ffi::{c_char, c_void, CStr};
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex as StdMutex;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestAttr {
    pub kind: i32,
}

pub struct GuestMutex {
    native: libc::pthread_mutex_t,
}

pub struct GuestCond {
    native: libc::pthread_cond_t,
}

/// PS4 `struct timespec` is `{int64 sec, int64 nsec}`, identical to Linux x86-64.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestTimespec {
    pub sec: i64,
    pub nsec: i64,
}

pub const ERR_TIMED_OUT: i32 = 0x8002_003cu32 as i32;

static CREATED: AtomicUsize = AtomicUsize::new(0);
static LOCKS: AtomicUsize = AtomicUsize::new(0);
static UNLOCKS: AtomicUsize = AtomicUsize::new(0);
static CONDS: AtomicUsize = AtomicUsize::new(0);
static WAITS: AtomicUsize = AtomicUsize::new(0);
static WAKEUPS: AtomicUsize = AtomicUsize::new(0);
static STATIC_INIT: StdMutex<()> = StdMutex::new(());

fn orbis_error(error: i32) -> i32 {
    if error == 0 {
        return 0;
    }
    let code = match error {
        libc::EPERM => 1,
        libc::ENOMEM => 12,
        libc::EBUSY => 16,
        libc::EINVAL => 22,
        libc::EAGAIN => 35,
        libc::EDEADLK => 11,
        other => stop(&format!("unmapped host pthread error {other}")),
    };
    orbis(code)
}

unsafe fn slot_load(slot: *mut usize) -> usize {
    (*(slot as *const AtomicUsize)).load(Ordering::Acquire)
}

unsafe fn slot_store(slot: *mut usize, value: usize) {
    (*(slot as *const AtomicUsize)).store(value, Ordering::Release)
}

// --- Attributes ----------------------------------------------------------------------------

/// # Safety
/// `out` must be null or valid guest memory for one pointer.
pub unsafe extern "C" fn attr_init(out: *mut *mut GuestAttr) -> i32 {
    if out.is_null() {
        return orbis_error(libc::EINVAL);
    }
    out.write(Box::into_raw(Box::new(GuestAttr { kind: 1 })));
    0
}

/// # Safety
/// `attr` must be null or a valid guest pointer-to-pointer.
pub unsafe extern "C" fn attr_type(attr: *mut *mut GuestAttr, kind: i32) -> i32 {
    if attr.is_null() || (*attr).is_null() || !(1..=4).contains(&kind) {
        return orbis_error(libc::EINVAL);
    }
    (**attr).kind = kind;
    0
}

/// Priority protocols need a PS4 scheduler; the value is validated and recorded only.
///
/// # Safety
/// `attr` must be null or a valid guest pointer-to-pointer.
pub unsafe extern "C" fn attr_protocol(attr: *mut *mut GuestAttr, protocol: i32) -> i32 {
    if attr.is_null() || (*attr).is_null() || !(0..=2).contains(&protocol) {
        return orbis_error(libc::EINVAL);
    }
    0
}

/// # Safety
/// `attr` must be null or a valid guest pointer-to-pointer.
pub unsafe extern "C" fn attr_destroy(attr: *mut *mut GuestAttr) -> i32 {
    if attr.is_null() || (*attr).is_null() {
        return orbis_error(libc::EINVAL);
    }
    drop(Box::from_raw(*attr));
    *attr = ptr::null_mut();
    0
}

// --- Mutexes -------------------------------------------------------------------------------

/// # Safety
/// `out` and `attr` must be null or valid guest memory; `name` is ignored.
pub unsafe extern "C" fn mutex_init(
    out: *mut *mut GuestMutex,
    attr: *mut *mut GuestAttr,
    _name: *const c_char,
) -> i32 {
    if out.is_null() || (!attr.is_null() && (*attr).is_null()) {
        return orbis_error(libc::EINVAL);
    }
    let kind = if attr.is_null() { 1 } else { (**attr).kind };
    if !(1..=4).contains(&kind) {
        return orbis_error(libc::EINVAL);
    }
    let mut native_attr = MaybeUninit::<libc::pthread_mutexattr_t>::uninit();
    let mut error = libc::pthread_mutexattr_init(native_attr.as_mut_ptr());
    if error != 0 {
        return orbis_error(error);
    }
    let mut native_attr = native_attr.assume_init();
    let native_type = match kind {
        2 => libc::PTHREAD_MUTEX_RECURSIVE,
        3 => libc::PTHREAD_MUTEX_NORMAL,
        _ => libc::PTHREAD_MUTEX_ERRORCHECK,
    };
    error = libc::pthread_mutexattr_settype(&mut native_attr, native_type);
    if error == 0 {
        let mut native = MaybeUninit::<libc::pthread_mutex_t>::uninit();
        error = libc::pthread_mutex_init(native.as_mut_ptr(), &native_attr);
        libc::pthread_mutexattr_destroy(&mut native_attr);
        if error != 0 {
            return orbis_error(error);
        }
        out.write(Box::into_raw(Box::new(GuestMutex {
            native: native.assume_init(),
        })));
        CREATED.fetch_add(1, Ordering::Relaxed);
        return 0;
    }
    libc::pthread_mutexattr_destroy(&mut native_attr);
    orbis_error(error)
}

/// Statically initialized handles (0/1) are created once, even under contention.
///
/// # Safety
/// `slot` must be null or valid guest memory for one handle word.
pub unsafe extern "C" fn ensure_mutex(slot: *mut usize) -> i32 {
    if slot.is_null() {
        return orbis_error(libc::EINVAL);
    }
    let value = slot_load(slot);
    if value == 2 {
        return orbis_error(libc::EINVAL);
    }
    if value >= 2 {
        return 0;
    }
    let _guard = STATIC_INIT
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if slot_load(slot) < 2 {
        let mut created: *mut GuestMutex = ptr::null_mut();
        let error = mutex_init(&mut created, ptr::null_mut(), ptr::null());
        if error != 0 {
            return error;
        }
        slot_store(slot, created as usize);
    }
    0
}

/// # Safety
/// `slot` must be null or valid guest memory for one handle word.
pub unsafe extern "C" fn mutex_lock(slot: *mut usize) -> i32 {
    let error = ensure_mutex(slot);
    if error != 0 {
        return error;
    }
    let native = &mut (*(slot_load(slot) as *mut GuestMutex)).native;
    let error = orbis_error(libc::pthread_mutex_lock(native));
    if error == 0 {
        LOCKS.fetch_add(1, Ordering::Relaxed);
    }
    error
}

/// # Safety
/// `slot` must be null or valid guest memory for one handle word.
pub unsafe extern "C" fn mutex_trylock(slot: *mut usize) -> i32 {
    let error = ensure_mutex(slot);
    if error != 0 {
        return error;
    }
    let native = &mut (*(slot_load(slot) as *mut GuestMutex)).native;
    let error = orbis_error(libc::pthread_mutex_trylock(native));
    if error == 0 {
        LOCKS.fetch_add(1, Ordering::Relaxed);
    }
    error
}

/// # Safety
/// `slot` must be null or valid guest memory for one handle word.
pub unsafe extern "C" fn mutex_unlock(slot: *mut usize) -> i32 {
    if slot.is_null() || slot_load(slot) == 2 {
        return orbis_error(libc::EINVAL);
    }
    if slot_load(slot) < 2 {
        return orbis_error(libc::EPERM);
    }
    let native = &mut (*(slot_load(slot) as *mut GuestMutex)).native;
    let error = orbis_error(libc::pthread_mutex_unlock(native));
    if error == 0 {
        UNLOCKS.fetch_add(1, Ordering::Relaxed);
    }
    error
}

/// # Safety
/// `slot` must be null or valid guest memory for one handle word.
pub unsafe extern "C" fn mutex_destroy(slot: *mut usize) -> i32 {
    if slot.is_null() || slot_load(slot) == 2 {
        return orbis_error(libc::EINVAL);
    }
    if slot_load(slot) < 2 {
        return 0;
    }
    let pointer = slot_load(slot) as *mut GuestMutex;
    let error = libc::pthread_mutex_destroy(&mut (*pointer).native);
    if error == 0 {
        drop(Box::from_raw(pointer));
        slot_store(slot, 2);
    }
    orbis_error(error)
}

fn deadline_after(microseconds: u32) -> Result<libc::timespec, ()> {
    let mut end = MaybeUninit::<libc::timespec>::uninit();
    if unsafe { libc::clock_gettime(libc::CLOCK_REALTIME, end.as_mut_ptr()) } != 0 {
        return Err(());
    }
    let mut end = unsafe { end.assume_init() };
    let nanos = end.tv_nsec as u64 + (microseconds as u64 % 1_000_000) * 1_000;
    end.tv_sec += (microseconds as u64 / 1_000_000 + nanos / 1_000_000_000) as libc::time_t;
    end.tv_nsec = (nanos % 1_000_000_000) as libc::c_long;
    Ok(end)
}

fn timed_error(error: i32) -> i32 {
    if error == libc::ETIMEDOUT {
        ERR_TIMED_OUT
    } else {
        orbis_error(error)
    }
}

/// # Safety
/// `slot` must be null or valid guest memory for one handle word.
pub unsafe extern "C" fn mutex_timedlock(slot: *mut usize, microseconds: u32) -> i32 {
    let error = ensure_mutex(slot);
    if error != 0 {
        return error;
    }
    let Ok(end) = deadline_after(microseconds) else {
        return orbis_error(libc::EINVAL);
    };
    let native = &mut (*(slot_load(slot) as *mut GuestMutex)).native;
    let error = timed_error(libc::pthread_mutex_timedlock(native, &end));
    if error == 0 {
        LOCKS.fetch_add(1, Ordering::Relaxed);
    }
    error
}

// --- Condition variables -------------------------------------------------------------------

/// # Safety
/// `out` must be null or valid guest memory for one pointer.
pub unsafe extern "C" fn cond_init(
    out: *mut *mut GuestCond,
    _attr: *mut *mut c_void,
    _name: *const c_char,
) -> i32 {
    if out.is_null() {
        return orbis_error(libc::EINVAL);
    }
    let mut native = MaybeUninit::<libc::pthread_cond_t>::uninit();
    let error = libc::pthread_cond_init(native.as_mut_ptr(), ptr::null());
    if error != 0 {
        return orbis_error(error);
    }
    out.write(Box::into_raw(Box::new(GuestCond {
        native: native.assume_init(),
    })));
    CONDS.fetch_add(1, Ordering::Relaxed);
    0
}

/// # Safety
/// `slot` must be null or valid guest memory for one handle word.
pub unsafe extern "C" fn ensure_cond(slot: *mut usize) -> i32 {
    if slot.is_null() {
        return orbis_error(libc::EINVAL);
    }
    let value = slot_load(slot);
    if value == 2 {
        return orbis_error(libc::EINVAL);
    }
    if value >= 2 {
        return 0;
    }
    let _guard = STATIC_INIT
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if slot_load(slot) < 2 {
        let mut created: *mut GuestCond = ptr::null_mut();
        let error = cond_init(&mut created, ptr::null_mut(), ptr::null());
        if error != 0 {
            return error;
        }
        slot_store(slot, created as usize);
    }
    0
}

/// # Safety
/// `slot` must be null or valid guest memory for one handle word.
pub unsafe extern "C" fn cond_destroy(slot: *mut usize) -> i32 {
    if slot.is_null() {
        return orbis_error(libc::EINVAL);
    }
    if slot_load(slot) < 2 {
        return 0;
    }
    let pointer = slot_load(slot) as *mut GuestCond;
    let error = libc::pthread_cond_destroy(&mut (*pointer).native);
    if error == 0 {
        drop(Box::from_raw(pointer));
        slot_store(slot, 0);
    }
    orbis_error(error)
}

/// # Safety
/// Both slots must be null or valid guest memory for one handle word each.
pub unsafe extern "C" fn cond_wait(cond_slot: *mut usize, mutex_slot: *mut usize) -> i32 {
    let error = ensure_cond(cond_slot);
    if error != 0 {
        return error;
    }
    if mutex_slot.is_null() || slot_load(mutex_slot) < 3 {
        return orbis_error(libc::EINVAL);
    }
    WAITS.fetch_add(1, Ordering::Relaxed);
    let cond = &mut (*(slot_load(cond_slot) as *mut GuestCond)).native;
    let mutex = &mut (*(slot_load(mutex_slot) as *mut GuestMutex)).native;
    orbis_error(libc::pthread_cond_wait(cond, mutex))
}

unsafe fn cond_wait_until(
    cond_slot: *mut usize,
    mutex_slot: *mut usize,
    end: &libc::timespec,
) -> i32 {
    let error = ensure_cond(cond_slot);
    if error != 0 {
        return error;
    }
    if mutex_slot.is_null() || slot_load(mutex_slot) < 3 {
        return orbis_error(libc::EINVAL);
    }
    WAITS.fetch_add(1, Ordering::Relaxed);
    let cond = &mut (*(slot_load(cond_slot) as *mut GuestCond)).native;
    let mutex = &mut (*(slot_load(mutex_slot) as *mut GuestMutex)).native;
    timed_error(libc::pthread_cond_timedwait(cond, mutex, end))
}

/// # Safety
/// Both slots must be null or valid guest memory for one handle word each.
pub unsafe extern "C" fn cond_timedwait(
    cond_slot: *mut usize,
    mutex_slot: *mut usize,
    microseconds: u32,
) -> i32 {
    let Ok(end) = deadline_after(microseconds) else {
        return orbis_error(libc::EINVAL);
    };
    cond_wait_until(cond_slot, mutex_slot, &end)
}

/// # Safety
/// `slot` must be null or valid guest memory for one handle word.
pub unsafe extern "C" fn cond_signal(slot: *mut usize) -> i32 {
    let error = ensure_cond(slot);
    if error != 0 {
        return error;
    }
    WAKEUPS.fetch_add(1, Ordering::Relaxed);
    let cond = &mut (*(slot_load(slot) as *mut GuestCond)).native;
    orbis_error(libc::pthread_cond_signal(cond))
}

/// # Safety
/// `slot` must be null or valid guest memory for one handle word.
pub unsafe extern "C" fn cond_broadcast(slot: *mut usize) -> i32 {
    let error = ensure_cond(slot);
    if error != 0 {
        return error;
    }
    WAKEUPS.fetch_add(1, Ordering::Relaxed);
    let cond = &mut (*(slot_load(slot) as *mut GuestCond)).native;
    orbis_error(libc::pthread_cond_broadcast(cond))
}

/// # Safety
/// Both slots must be null or valid guest memory; `abs` must be null or a valid
/// `GuestTimespec`.
pub unsafe extern "C" fn cond_abs_timedwait(
    cond_slot: *mut usize,
    mutex_slot: *mut usize,
    abs: *const GuestTimespec,
) -> i32 {
    if abs.is_null() {
        return orbis_error(libc::EINVAL);
    }
    let abs = *abs;
    if !(0..1_000_000_000).contains(&abs.nsec) {
        return orbis_error(libc::EINVAL);
    }
    let end = libc::timespec {
        tv_sec: abs.sec as libc::time_t,
        tv_nsec: abs.nsec as libc::c_long,
    };
    cond_wait_until(cond_slot, mutex_slot, &end)
}

// --- POSIX wrappers ------------------------------------------------------------------------

/// POSIX pthread APIs return positive guest errno values, unlike `scePthread`.
fn posix_result(result: i32) -> i32 {
    if result != 0 {
        (result as u32 & 0xffff) as i32
    } else {
        0
    }
}

pub fn report() {
    println!(
        "Runtime: mutexes created={}, locks={}, unlocks={}",
        CREATED.load(Ordering::Relaxed),
        LOCKS.load(Ordering::Relaxed),
        UNLOCKS.load(Ordering::Relaxed)
    );
    println!(
        "Runtime: condition variables created={}, waits={}, wakeups={}",
        CONDS.load(Ordering::Relaxed),
        WAITS.load(Ordering::Relaxed),
        WAKEUPS.load(Ordering::Relaxed)
    );
}

/// Guest NID to function address, like `runtime_mutex_resolve`.
#[allow(function_casts_as_integer)]
pub fn resolve(name: &str) -> usize {
    match name {
        "0TyVk4MSLt0#I#J" => posix_cond_init as usize,
        "RXXqi4CtF8w#I#J" => posix_cond_destroy as usize,
        "Op8TBGY5KHg#I#J" => posix_cond_wait as usize,
        "27bAgiJmOh0#I#J" => posix_cond_timedwait as usize,
        "2MOy+rUfuhQ#I#J" => posix_cond_signal as usize,
        "mkx2fVhNMsg#I#J" => posix_cond_broadcast as usize,
        "2Tb92quprl0#p#J" => cond_init as usize,
        "g+PZd2hiacg#p#J" => cond_destroy as usize,
        "WKAXJ4XBPQ4#p#J" => cond_wait as usize,
        "BmMjYxmew1w#p#J" => cond_timedwait as usize,
        "kDh-NfxgMtE#p#J" => cond_signal as usize,
        "JGgj7Uvrl+A#p#J" => cond_broadcast as usize,
        "IafI2PxcPnQ#p#J" => mutex_timedlock as usize,
        "dQHWEsJtoE4#I#J" => posix_attr_init as usize,
        "mDmgMOGVUqg#I#J" => posix_attr_type as usize,
        "HF7lK46xzjY#I#J" => posix_attr_destroy as usize,
        "ttHNfU+qDBU#I#J" => posix_mutex_init as usize,
        "ltCfaGr2JGE#I#J" => posix_mutex_destroy as usize,
        "7H0iTOciTLo#I#J" => posix_mutex_lock as usize,
        "K-jXhbt2gn4#I#J" => posix_mutex_trylock as usize,
        "2Z+PpY6CaJg#I#J" => posix_mutex_unlock as usize,
        "F8bUHwAG284#p#J" => attr_init as usize,
        "iMp8QpE+XO4#p#J" => attr_type as usize,
        "smWEktiyyG0#p#J" => attr_destroy as usize,
        "1FGvU0i9saQ#p#J" => attr_protocol as usize,
        "cmo1RIYva9o#p#J" => mutex_init as usize,
        "9UK1vLZQft4#p#J" => mutex_lock as usize,
        "upoVrzMHFeE#p#J" => mutex_trylock as usize,
        "tn3VlD0hG60#p#J" => mutex_unlock as usize,
        "2Of0f+3mhhE#p#J" => mutex_destroy as usize,
        _ => 0,
    }
}

// The POSIX wrappers mask the Orbis error to the guest errno value.
unsafe extern "C" fn posix_attr_init(out: *mut *mut GuestAttr) -> i32 {
    posix_result(attr_init(out))
}
unsafe extern "C" fn posix_attr_type(attr: *mut *mut GuestAttr, kind: i32) -> i32 {
    posix_result(attr_type(attr, kind))
}
unsafe extern "C" fn posix_attr_destroy(attr: *mut *mut GuestAttr) -> i32 {
    posix_result(attr_destroy(attr))
}
unsafe extern "C" fn posix_mutex_init(out: *mut *mut GuestMutex, attr: *mut *mut GuestAttr) -> i32 {
    posix_result(mutex_init(out, attr, ptr::null()))
}
unsafe extern "C" fn posix_mutex_destroy(slot: *mut usize) -> i32 {
    posix_result(mutex_destroy(slot))
}
unsafe extern "C" fn posix_mutex_lock(slot: *mut usize) -> i32 {
    posix_result(mutex_lock(slot))
}
unsafe extern "C" fn posix_mutex_trylock(slot: *mut usize) -> i32 {
    posix_result(mutex_trylock(slot))
}
unsafe extern "C" fn posix_mutex_unlock(slot: *mut usize) -> i32 {
    posix_result(mutex_unlock(slot))
}
unsafe extern "C" fn posix_cond_init(out: *mut *mut GuestCond, attr: *mut *mut c_void) -> i32 {
    posix_result(cond_init(out, attr, ptr::null()))
}
unsafe extern "C" fn posix_cond_destroy(slot: *mut usize) -> i32 {
    posix_result(cond_destroy(slot))
}
unsafe extern "C" fn posix_cond_wait(cond: *mut usize, mutex: *mut usize) -> i32 {
    posix_result(cond_wait(cond, mutex))
}
unsafe extern "C" fn posix_cond_timedwait(
    cond: *mut usize,
    mutex: *mut usize,
    abs: *const GuestTimespec,
) -> i32 {
    posix_result(cond_abs_timedwait(cond, mutex, abs))
}
unsafe extern "C" fn posix_cond_signal(slot: *mut usize) -> i32 {
    posix_result(cond_signal(slot))
}
unsafe extern "C" fn posix_cond_broadcast(slot: *mut usize) -> i32 {
    posix_result(cond_broadcast(slot))
}

/// C ABI entry point the loader calls while the legacy `runtime_mutex.c` is replaced.
///
/// # Safety
/// `name` must be a NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn runtime_mutex_resolve(name: *const c_char) -> usize {
    if name.is_null() {
        return 0;
    }
    match CStr::from_ptr(name).to_str() {
        Ok(name) => resolve(name),
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "C" fn runtime_mutex_report() {
    report();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes_lifecycle() {
        unsafe {
            let mut attr: *mut GuestAttr = ptr::null_mut();
            assert_eq!(attr_init(&mut attr), 0);
            assert!(!attr.is_null());
            assert_eq!(attr_type(&mut attr, 2), 0);
            assert_eq!((*attr).kind, 2);
            assert_eq!(attr_type(&mut attr, 9), orbis_error(libc::EINVAL));
            assert_eq!(attr_protocol(&mut attr, 3), orbis_error(libc::EINVAL));
            assert_eq!(attr_destroy(&mut attr), 0);
            assert!(attr.is_null());
            assert_eq!(attr_destroy(&mut attr), orbis_error(libc::EINVAL));
        }
    }

    #[test]
    fn mutex_lock_unlock_trylock_and_destroy() {
        unsafe {
            let mut mutex: *mut GuestMutex = ptr::null_mut();
            assert_eq!(mutex_init(&mut mutex, ptr::null_mut(), ptr::null()), 0);
            assert_eq!(mutex_lock_slot(&mut mutex), 0);
            assert_eq!(mutex_unlock_slot(&mut mutex), 0);
            assert_eq!(mutex_trylock_slot(&mut mutex), 0);
            assert_eq!(mutex_unlock_slot(&mut mutex), 0);
            assert_eq!(mutex_destroy_slot(&mut mutex), 0);
        }

        // The guest-facing functions take the address of the guest's handle word.
        unsafe fn as_slot(pointer: &mut *mut GuestMutex) -> *mut usize {
            pointer as *mut *mut GuestMutex as *mut usize
        }
        unsafe {
            let mut mutex: *mut GuestMutex = ptr::null_mut();
            let slot = as_slot(&mut mutex);
            assert_eq!(mutex_lock(slot), 0);
            assert_eq!(mutex_unlock(slot), 0);
            assert_eq!(mutex_destroy(slot), 0);
            assert_eq!(slot_load(slot), 2);
            assert_eq!(mutex_lock(slot), orbis_error(libc::EINVAL));
            assert_eq!(mutex_unlock(slot), orbis_error(libc::EINVAL));
        }
    }

    #[test]
    fn static_handle_is_created_once() {
        unsafe {
            let mut slot: usize = 0;
            assert_eq!(mutex_lock(&mut slot), 0);
            assert!(slot_load(&mut slot) > 2);
            assert_eq!(mutex_unlock(&mut slot), 0);
            assert_eq!(mutex_destroy(&mut slot), 0);
            assert_eq!(slot_load(&mut slot), 2);
        }
    }

    #[test]
    fn destroyed_condition_handle_is_rejected() {
        let mut cond: usize = 2;
        let mut mutex: usize = 0;
        unsafe {
            assert_eq!(ensure_mutex(&mut mutex), 0);
            assert_eq!(mutex_lock(&mut mutex), 0);
            assert_eq!(cond_signal(&mut cond), orbis_error(libc::EINVAL));
            assert_eq!(cond_broadcast(&mut cond), orbis_error(libc::EINVAL));
            assert_eq!(cond_wait(&mut cond, &mut mutex), orbis_error(libc::EINVAL));
            assert_eq!(mutex_unlock(&mut mutex), 0);
            assert_eq!(mutex_destroy(&mut mutex), 0);
        }
    }

    #[test]
    fn timed_lock_reports_timeout() {
        use std::sync::atomic::AtomicBool;
        use std::sync::Arc;
        let slot = Arc::new(AtomicUsize::new(0));
        let started = Arc::new(AtomicBool::new(false));
        let holder_slot = Arc::clone(&slot);
        let holder_started = Arc::clone(&started);
        let holder = std::thread::spawn(move || unsafe {
            let pointer = holder_slot.as_ref() as *const AtomicUsize as *mut usize;
            assert_eq!(mutex_lock(pointer), 0);
            holder_started.store(true, Ordering::Release);
            std::thread::sleep(std::time::Duration::from_millis(200));
            assert_eq!(mutex_unlock(pointer), 0);
        });
        while !started.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        unsafe {
            let pointer = slot.as_ref() as *const AtomicUsize as *mut usize;
            assert_eq!(mutex_timedlock(pointer, 5000), ERR_TIMED_OUT);
        }
        holder.join().unwrap();
        unsafe {
            let pointer = slot.as_ref() as *const AtomicUsize as *mut usize;
            assert_eq!(mutex_destroy(pointer), 0);
        }
    }

    #[test]
    fn condition_variable_wakes_a_waiter() {
        use std::sync::atomic::AtomicBool;
        use std::sync::Arc;
        let mutex_slot = Arc::new(AtomicUsize::new(0));
        let cond_slot = Arc::new(AtomicUsize::new(0));
        let ready = Arc::new(AtomicBool::new(false));
        let waiter_mutex = Arc::clone(&mutex_slot);
        let waiter_cond = Arc::clone(&cond_slot);
        let waiter_ready = Arc::clone(&ready);
        let waiter = std::thread::spawn(move || unsafe {
            let mutex = waiter_mutex.as_ref() as *const AtomicUsize as *mut usize;
            let cond = waiter_cond.as_ref() as *const AtomicUsize as *mut usize;
            assert_eq!(mutex_lock(mutex), 0);
            while !waiter_ready.load(Ordering::Acquire) {
                assert_eq!(cond_wait(cond, mutex), 0);
            }
            assert_eq!(mutex_unlock(mutex), 0);
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        unsafe {
            let mutex = mutex_slot.as_ref() as *const AtomicUsize as *mut usize;
            let cond = cond_slot.as_ref() as *const AtomicUsize as *mut usize;
            assert_eq!(mutex_lock(mutex), 0);
            ready.store(true, Ordering::Release);
            assert_eq!(cond_signal(cond), 0);
            assert_eq!(mutex_unlock(mutex), 0);
        }
        waiter.join().unwrap();
        unsafe {
            assert_eq!(
                cond_destroy(cond_slot.as_ref() as *const AtomicUsize as *mut usize),
                0
            );
            assert_eq!(
                mutex_destroy(mutex_slot.as_ref() as *const AtomicUsize as *mut usize),
                0
            );
        }
    }

    #[test]
    fn posix_wrappers_mask_the_orbis_error() {
        assert_eq!(posix_result(0), 0);
        assert_eq!(posix_result(0x8002_0016u32 as i32), 22);
        unsafe {
            let mut attr: *mut GuestAttr = ptr::null_mut();
            assert_eq!(attr_init(&mut attr), 0);
            assert_eq!(posix_attr_destroy(&mut attr), 0);
        }
    }

    #[test]
    fn resolves_every_guest_nid() {
        for nid in [
            "0TyVk4MSLt0#I#J",
            "IafI2PxcPnQ#p#J",
            "cmo1RIYva9o#p#J",
            "tn3VlD0hG60#p#J",
        ] {
            assert_ne!(resolve(nid), 0, "{nid}");
        }
        assert_eq!(resolve("unrelated#q#q"), 0);
    }

    // Helpers that treat a `*mut GuestMutex` local as the guest handle slot.
    unsafe fn mutex_lock_slot(pointer: &mut *mut GuestMutex) -> i32 {
        mutex_lock(pointer as *mut *mut GuestMutex as *mut usize)
    }
    unsafe fn mutex_trylock_slot(pointer: &mut *mut GuestMutex) -> i32 {
        mutex_trylock(pointer as *mut *mut GuestMutex as *mut usize)
    }
    unsafe fn mutex_unlock_slot(pointer: &mut *mut GuestMutex) -> i32 {
        mutex_unlock(pointer as *mut *mut GuestMutex as *mut usize)
    }
    unsafe fn mutex_destroy_slot(pointer: &mut *mut GuestMutex) -> i32 {
        mutex_destroy(pointer as *mut *mut GuestMutex as *mut usize)
    }
}
