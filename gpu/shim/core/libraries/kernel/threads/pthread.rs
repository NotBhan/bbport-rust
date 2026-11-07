//! Co-op pthread surface, ported from
//! `legacy/gpu/shim/core/libraries/kernel/threads/pthread.h`.
//! Threads are real guest threads from the C runtime; mutexes and condition variables are host
//! objects, as in the C++ port.

use std::ffi::{c_char, c_int, c_void};
use std::sync::{Condvar, Mutex, MutexGuard};
use std::thread::ThreadId;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum PthreadMutexType {
    ErrorCheck = 1,
    Recursive = 2,
    #[default]
    Normal = 3,
    AdaptiveNp = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(u32)]
pub enum SchedPolicy {
    Fifo = 1,
    #[default]
    Other = 2,
    RoundRobin = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SchedParam {
    pub sched_priority: c_int,
}

pub const ORBIS_KERNEL_PRIO_FIFO_DEFAULT: c_int = 700;
pub const ORBIS_KERNEL_PRIO_FIFO_HIGHEST: c_int = 256;
pub const ORBIS_KERNEL_PRIO_FIFO_LOWEST: c_int = 767;

/// The runtime's guest thread handle.
pub type PthreadT = *mut c_void;
pub type PthreadEntryFunc = extern "C" fn(*mut c_void) -> *mut c_void;

pub struct BbNetMutexAttr {
    pub kind: PthreadMutexType,
}

struct MutexInner {
    locked: bool,
    owner: Option<ThreadId>,
    depth: u32,
}

pub struct BbNetMutex {
    inner: Mutex<MutexInner>,
    cond: Condvar,
    kind: PthreadMutexType,
}

#[derive(Default)]
pub struct BbNetCond {
    generation: Mutex<u64>,
    cond: Condvar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BbNetThreadAttr {
    pub stack_size: usize,
    pub inheritsched: bool,
    pub policy: SchedPolicy,
    pub param: SchedParam,
    pub affinity: u64,
}

#[derive(Default)]
pub struct BbNetCondAttr;

pub type PthreadMutexT = *mut BbNetMutex;
pub type PthreadMutexAttrT = *mut BbNetMutexAttr;
pub type PthreadCondT = *mut BbNetCond;
pub type PthreadCondAttrT = *mut BbNetCondAttr;
pub type PthreadAttrT = *mut BbNetThreadAttr;

fn guard<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

impl BbNetMutex {
    pub fn new(kind: PthreadMutexType) -> Self {
        Self {
            inner: Mutex::new(MutexInner {
                locked: false,
                owner: None,
                depth: 0,
            }),
            cond: Condvar::new(),
            kind,
        }
    }

    /// All return 0 or a POSIX errno value, as shadPS4's `posix_*` functions do.
    pub fn lock(&self) -> c_int {
        let current = std::thread::current().id();
        let mut inner = guard(&self.inner);
        loop {
            if !inner.locked {
                inner.locked = true;
                inner.owner = Some(current);
                inner.depth = 1;
                return 0;
            }
            if inner.owner == Some(current) {
                return match self.kind {
                    PthreadMutexType::Recursive => {
                        inner.depth += 1;
                        0
                    }
                    _ => libc::EDEADLK,
                };
            }
            if self.kind == PthreadMutexType::ErrorCheck {
                return libc::EBUSY;
            }
            inner = self
                .cond
                .wait(inner)
                .unwrap_or_else(|error| error.into_inner());
        }
    }

    pub fn try_lock(&self) -> c_int {
        let current = std::thread::current().id();
        let mut inner = guard(&self.inner);
        if !inner.locked {
            inner.locked = true;
            inner.owner = Some(current);
            inner.depth = 1;
            return 0;
        }
        if inner.owner == Some(current) && self.kind == PthreadMutexType::Recursive {
            inner.depth += 1;
            return 0;
        }
        libc::EBUSY
    }

    pub fn unlock(&self) -> c_int {
        let current = std::thread::current().id();
        let mut inner = guard(&self.inner);
        if !inner.locked || inner.owner != Some(current) {
            return libc::EPERM;
        }
        if inner.depth > 1 {
            inner.depth -= 1;
            return 0;
        }
        inner.locked = false;
        inner.owner = None;
        inner.depth = 0;
        drop(inner);
        self.cond.notify_one();
        0
    }
}

impl BbNetCond {
    pub fn signal(&self) {
        let mut generation = guard(&self.generation);
        *generation = generation.wrapping_add(1);
        drop(generation);
        self.cond.notify_one();
    }

    /// Waits for the next signal, releasing `mutex` (which the caller must hold) meanwhile.
    pub fn wait(&self, mutex: &BbNetMutex, timeout: Option<Duration>) -> c_int {
        let current = std::thread::current().id();
        if guard(&mutex.inner).owner != Some(current) {
            return libc::EPERM;
        }
        let generation = *guard(&self.generation);
        let saved_depth = {
            let mut inner = guard(&mutex.inner);
            let depth = inner.depth;
            inner.locked = false;
            inner.owner = None;
            inner.depth = 0;
            depth
        };
        mutex.cond.notify_one();
        let result = {
            let inner = guard(&self.generation);
            if *inner != generation {
                0
            } else {
                match timeout {
                    Some(timeout) => {
                        let (inner, wait) = self
                            .cond
                            .wait_timeout(inner, timeout)
                            .unwrap_or_else(|error| error.into_inner());
                        if wait.timed_out() && *inner == generation {
                            libc::ETIMEDOUT
                        } else {
                            0
                        }
                    }
                    None => {
                        let _inner = self
                            .cond
                            .wait(inner)
                            .unwrap_or_else(|error| error.into_inner());
                        0
                    }
                }
            }
        };
        let relock = mutex.lock();
        guard(&mutex.inner).depth = saved_depth;
        if relock != 0 {
            relock
        } else {
            result
        }
    }
}

/// # Safety
/// `attr` must point to writable storage; the initialized pointer is freed by
/// `posix_pthread_mutexattr_destroy`.
pub unsafe fn posix_pthread_mutexattr_init(attr: *mut PthreadMutexAttrT) -> c_int {
    if attr.is_null() {
        return libc::EINVAL;
    }
    unsafe {
        *attr = Box::into_raw(Box::new(BbNetMutexAttr {
            kind: PthreadMutexType::Normal,
        }));
    }
    0
}

/// # Safety
/// `attr` must point to a pointer initialized by `posix_pthread_mutexattr_init`.
pub unsafe fn posix_pthread_mutexattr_settype(
    attr: *mut PthreadMutexAttrT,
    kind: PthreadMutexType,
) -> c_int {
    let Some(attr) = (unsafe { attr.as_mut() }) else {
        return libc::EINVAL;
    };
    let Some(handle) = (unsafe { attr.as_mut() }) else {
        return libc::EINVAL;
    };
    handle.kind = kind;
    0
}

/// # Safety
/// `attr` must point to a pointer initialized by `posix_pthread_mutexattr_init`.
pub unsafe fn posix_pthread_mutexattr_destroy(attr: *mut PthreadMutexAttrT) -> c_int {
    let Some(attr) = (unsafe { attr.as_mut() }) else {
        return libc::EINVAL;
    };
    if attr.is_null() {
        return libc::EINVAL;
    }
    let handle = std::mem::replace(attr, std::ptr::null_mut());
    drop(unsafe { Box::from_raw(handle) });
    0
}

/// # Safety
/// `mutex` must point to writable storage; `attr` may be null or an initialized attribute.
pub unsafe fn sce_pthread_mutex_init(
    mutex: *mut PthreadMutexT,
    attr: PthreadMutexAttrT,
    _name: *const c_char,
) -> c_int {
    if mutex.is_null() {
        return libc::EINVAL;
    }
    let kind = unsafe { attr.as_ref() }
        .map(|attr| attr.kind)
        .unwrap_or_default();
    unsafe {
        *mutex = Box::into_raw(Box::new(BbNetMutex::new(kind)));
    }
    0
}

/// # Safety
/// `mutex` must point to a pointer initialized by `sce_pthread_mutex_init`.
pub unsafe fn posix_pthread_mutex_destroy(mutex: *mut PthreadMutexT) -> c_int {
    let Some(slot) = (unsafe { mutex.as_mut() }) else {
        return libc::EINVAL;
    };
    if slot.is_null() {
        return libc::EINVAL;
    }
    let handle = std::mem::replace(slot, std::ptr::null_mut());
    drop(unsafe { Box::from_raw(handle) });
    0
}

/// # Safety
/// `mutex` must point to a pointer initialized by `sce_pthread_mutex_init`.
pub unsafe fn posix_pthread_mutex_lock(mutex: *mut PthreadMutexT) -> c_int {
    let Some(slot) = (unsafe { mutex.as_ref() }) else {
        return libc::EINVAL;
    };
    match unsafe { slot.as_ref() } {
        Some(mutex) => mutex.lock(),
        None => libc::EINVAL,
    }
}

/// # Safety
/// `mutex` must point to a pointer initialized by `sce_pthread_mutex_init`.
pub unsafe fn posix_pthread_mutex_trylock(mutex: *mut PthreadMutexT) -> c_int {
    let Some(slot) = (unsafe { mutex.as_ref() }) else {
        return libc::EINVAL;
    };
    match unsafe { slot.as_ref() } {
        Some(mutex) => mutex.try_lock(),
        None => libc::EINVAL,
    }
}

/// # Safety
/// `mutex` must point to a pointer initialized by `sce_pthread_mutex_init` and held by the
/// caller.
pub unsafe fn posix_pthread_mutex_unlock(mutex: *mut PthreadMutexT) -> c_int {
    let Some(slot) = (unsafe { mutex.as_ref() }) else {
        return libc::EINVAL;
    };
    match unsafe { slot.as_ref() } {
        Some(mutex) => mutex.unlock(),
        None => libc::EINVAL,
    }
}

/// # Safety
/// `attr` must point to writable storage; the pointer is freed by the destroy call.
pub unsafe fn posix_pthread_condattr_init(attr: *mut PthreadCondAttrT) -> c_int {
    if attr.is_null() {
        return libc::EINVAL;
    }
    unsafe {
        *attr = Box::into_raw(Box::new(BbNetCondAttr));
    }
    0
}

/// # Safety
/// `attr` must point to a pointer initialized by `posix_pthread_condattr_init`.
pub unsafe fn posix_pthread_condattr_destroy(attr: *mut PthreadCondAttrT) -> c_int {
    let Some(attr) = (unsafe { attr.as_mut() }) else {
        return libc::EINVAL;
    };
    if attr.is_null() {
        return libc::EINVAL;
    }
    let handle = std::mem::replace(attr, std::ptr::null_mut());
    drop(unsafe { Box::from_raw(handle) });
    0
}

/// # Safety
/// `cond` must point to writable storage.
pub unsafe fn sce_pthread_cond_init(
    cond: *mut PthreadCondT,
    _attr: PthreadCondAttrT,
    _name: *const c_char,
) -> c_int {
    if cond.is_null() {
        return libc::EINVAL;
    }
    unsafe {
        *cond = Box::into_raw(Box::new(BbNetCond::default()));
    }
    0
}

/// # Safety
/// `cond` must point to a pointer initialized by `sce_pthread_cond_init`.
pub unsafe fn posix_pthread_cond_destroy(cond: *mut PthreadCondT) -> c_int {
    let Some(slot) = (unsafe { cond.as_mut() }) else {
        return libc::EINVAL;
    };
    if slot.is_null() {
        return libc::EINVAL;
    }
    let handle = std::mem::replace(slot, std::ptr::null_mut());
    drop(unsafe { Box::from_raw(handle) });
    0
}

/// # Safety
/// `cond` must point to a pointer initialized by `sce_pthread_cond_init`.
pub unsafe fn posix_pthread_cond_signal(cond: *mut PthreadCondT) -> c_int {
    let Some(slot) = (unsafe { cond.as_ref() }) else {
        return libc::EINVAL;
    };
    match unsafe { slot.as_ref() } {
        Some(cond) => {
            cond.signal();
            0
        }
        None => libc::EINVAL,
    }
}

/// # Safety
/// `cond` must point to a pointer initialized by `sce_pthread_cond_init`, and `mutex` must
/// point to a mutex handle held by the caller.
pub unsafe fn posix_pthread_cond_wait(cond: *mut PthreadCondT, mutex: *mut PthreadMutexT) -> c_int {
    let (Some(cond), Some(mutex)) = (unsafe { cond.as_ref() }, unsafe { mutex.as_ref() }) else {
        return libc::EINVAL;
    };
    let (Some(cond), Some(mutex)) = (unsafe { cond.as_ref() }, unsafe { mutex.as_ref() }) else {
        return libc::EINVAL;
    };
    cond.wait(mutex, None)
}

/// # Safety
/// `cond` must point to a pointer initialized by `sce_pthread_cond_init`, `mutex` must point to
/// a mutex handle held by the caller, and `usec` is a relative timeout.
pub unsafe fn posix_pthread_cond_reltimedwait_np(
    cond: *mut PthreadCondT,
    mutex: *mut PthreadMutexT,
    usec: u64,
) -> c_int {
    let (Some(cond), Some(mutex)) = (unsafe { cond.as_ref() }, unsafe { mutex.as_ref() }) else {
        return libc::EINVAL;
    };
    let (Some(cond), Some(mutex)) = (unsafe { cond.as_ref() }, unsafe { mutex.as_ref() }) else {
        return libc::EINVAL;
    };
    cond.wait(mutex, Some(Duration::from_micros(usec)))
}

/// # Safety
/// `attr` must point to writable storage.
pub unsafe fn posix_pthread_attr_init(attr: *mut PthreadAttrT) -> c_int {
    if attr.is_null() {
        return libc::EINVAL;
    }
    unsafe {
        *attr = Box::into_raw(Box::new(BbNetThreadAttr::default()));
    }
    0
}

/// # Safety
/// `attr` must point to a pointer initialized by `posix_pthread_attr_init`.
pub unsafe fn posix_pthread_attr_destroy(attr: *mut PthreadAttrT) -> c_int {
    let Some(attr) = (unsafe { attr.as_mut() }) else {
        return libc::EINVAL;
    };
    if attr.is_null() {
        return libc::EINVAL;
    }
    let handle = std::mem::replace(attr, std::ptr::null_mut());
    drop(unsafe { Box::from_raw(handle) });
    0
}

/// # Safety
/// `attr` must point to a pointer initialized by `posix_pthread_attr_init`.
pub unsafe fn posix_pthread_attr_setstacksize(attr: *mut PthreadAttrT, stack_size: usize) -> c_int {
    let Some(slot) = (unsafe { attr.as_mut() }) else {
        return libc::EINVAL;
    };
    match unsafe { slot.as_mut() } {
        Some(attr) => {
            attr.stack_size = stack_size;
            0
        }
        None => libc::EINVAL,
    }
}

/// # Safety
/// `attr` must point to a pointer initialized by `posix_pthread_attr_init`.
pub unsafe fn posix_pthread_attr_setinheritsched(attr: *mut PthreadAttrT, inherit: c_int) -> c_int {
    let Some(slot) = (unsafe { attr.as_mut() }) else {
        return libc::EINVAL;
    };
    match unsafe { slot.as_mut() } {
        Some(attr) => {
            attr.inheritsched = inherit != 0;
            0
        }
        None => libc::EINVAL,
    }
}

/// # Safety
/// `attr` must point to a pointer initialized by `posix_pthread_attr_init`.
pub unsafe fn posix_pthread_attr_setschedpolicy(
    attr: *mut PthreadAttrT,
    policy: SchedPolicy,
) -> c_int {
    let Some(slot) = (unsafe { attr.as_mut() }) else {
        return libc::EINVAL;
    };
    match unsafe { slot.as_mut() } {
        Some(attr) => {
            attr.policy = policy;
            0
        }
        None => libc::EINVAL,
    }
}

/// # Safety
/// `attr` must point to an initialized pointer and `param` must be readable.
pub unsafe fn posix_pthread_attr_setschedparam(
    attr: *mut PthreadAttrT,
    param: *const SchedParam,
) -> c_int {
    let (Some(slot), Some(param)) = (unsafe { attr.as_ref() }, unsafe { param.as_ref() }) else {
        return libc::EINVAL;
    };
    let Some(attr) = (unsafe { slot.as_mut() }) else {
        return libc::EINVAL;
    };
    attr.param = *param;
    0
}

/// # Safety
/// `attr` must point to a pointer initialized by `posix_pthread_attr_init`.
pub unsafe fn sce_pthread_attr_setaffinity(attr: *mut PthreadAttrT, mask: u64) -> c_int {
    let Some(slot) = (unsafe { attr.as_mut() }) else {
        return libc::EINVAL;
    };
    match unsafe { slot.as_mut() } {
        Some(attr) => {
            attr.affinity = mask;
            0
        }
        None => libc::EINVAL,
    }
}

struct SendArg(*mut c_void);
// The guest contract hands ownership of `arg` to the new thread.
unsafe impl Send for SendArg {}

fn run_entry(start: PthreadEntryFunc, arg: SendArg) -> SendArg {
    SendArg(start(arg.0))
}

/// # Safety
/// `thread` must be writable; `start` must be a valid guest entry point and `arg` stays owned by
/// the new thread until it returns.
pub unsafe fn posix_pthread_create_name_np(
    thread: *mut PthreadT,
    _attr: PthreadAttrT,
    start: PthreadEntryFunc,
    arg: *mut c_void,
    _name: *const c_char,
) -> c_int {
    if thread.is_null() {
        return libc::EINVAL;
    }
    let arg = SendArg(arg);
    let handle = std::thread::spawn(move || run_entry(start, arg));
    unsafe {
        *thread = Box::into_raw(Box::new(handle)).cast::<c_void>();
    }
    0
}

/// # Safety
/// `thread` must come from `posix_pthread_create_name_np`; `ret` may be null.
pub unsafe fn posix_pthread_join(thread: PthreadT, ret: *mut *mut c_void) -> c_int {
    if thread.is_null() {
        return libc::EINVAL;
    }
    let handle = unsafe { Box::from_raw(thread.cast::<std::thread::JoinHandle<SendArg>>()) };
    let value = handle
        .join()
        .map(|arg| arg.0)
        .unwrap_or(std::ptr::null_mut());
    if !ret.is_null() {
        unsafe {
            *ret = value;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recursive_mutex_locks_and_unlocks_twice() {
        unsafe {
            let mut attr: PthreadMutexAttrT = std::ptr::null_mut();
            assert_eq!(posix_pthread_mutexattr_init(&mut attr), 0);
            assert_eq!(
                posix_pthread_mutexattr_settype(&mut attr, PthreadMutexType::Recursive),
                0
            );
            let mut mutex: PthreadMutexT = std::ptr::null_mut();
            assert_eq!(
                sce_pthread_mutex_init(&mut mutex, attr, std::ptr::null()),
                0
            );
            assert_eq!(posix_pthread_mutex_lock(&mut mutex), 0);
            assert_eq!(posix_pthread_mutex_lock(&mut mutex), 0);
            assert_eq!(posix_pthread_mutex_unlock(&mut mutex), 0);
            assert_eq!(posix_pthread_mutex_unlock(&mut mutex), 0);
            assert_eq!(posix_pthread_mutex_destroy(&mut mutex), 0);
            assert!(mutex.is_null());
            assert_eq!(posix_pthread_mutex_destroy(&mut mutex), libc::EINVAL);
            assert_eq!(posix_pthread_mutexattr_destroy(&mut attr), 0);
            assert!(attr.is_null());
            assert_eq!(posix_pthread_mutexattr_destroy(&mut attr), libc::EINVAL);
        }
    }

    #[test]
    fn cond_destroy_clears_the_guest_handle() {
        unsafe {
            let mut cond: PthreadCondT = std::ptr::null_mut();
            assert_eq!(
                sce_pthread_cond_init(&mut cond, std::ptr::null_mut(), std::ptr::null()),
                0
            );
            assert!(!cond.is_null());
            assert_eq!(posix_pthread_cond_signal(&mut cond), 0);
            assert_eq!(posix_pthread_cond_destroy(&mut cond), 0);
            assert!(cond.is_null());
            assert_eq!(posix_pthread_cond_signal(&mut cond), libc::EINVAL);
            assert_eq!(posix_pthread_cond_destroy(&mut cond), libc::EINVAL);
        }
    }

    #[test]
    fn a_condition_variable_wakes_a_waiter() {
        use std::sync::Arc;
        let mutex = Arc::new(BbNetMutex::new(PthreadMutexType::Normal));
        let cond = Arc::new(BbNetCond::default());
        let waiter = {
            let mutex = Arc::clone(&mutex);
            let cond = Arc::clone(&cond);
            std::thread::spawn(move || {
                mutex.lock();
                let result = cond.wait(&mutex, Some(Duration::from_secs(2)));
                mutex.unlock();
                result
            })
        };
        std::thread::sleep(Duration::from_millis(20));
        mutex.lock();
        cond.signal();
        mutex.unlock();
        assert_eq!(waiter.join().unwrap(), 0);
    }

    #[test]
    fn a_created_thread_returns_its_argument() {
        extern "C" fn entry(arg: *mut c_void) -> *mut c_void {
            arg
        }
        unsafe {
            let mut thread: PthreadT = std::ptr::null_mut();
            let arg = 0x1234usize as *mut c_void;
            assert_eq!(
                posix_pthread_create_name_np(
                    &mut thread,
                    std::ptr::null_mut(),
                    entry,
                    arg,
                    std::ptr::null()
                ),
                0
            );
            let mut ret: *mut c_void = std::ptr::null_mut();
            assert_eq!(posix_pthread_join(thread, &mut ret), 0);
            assert_eq!(ret, arg);
        }
    }
}
