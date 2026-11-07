//! `gettimeofday` with the explicit guest `timeval` layout, ported from
//! `legacy/src/runtime_time.c`. Host pointers never escape this boundary.

use crate::src::runtime::stop;
use std::ffi::{c_char, CStr};

/// Host `struct timezone` layout used by `gettimeofday`.
#[repr(C)]
struct HostTimezone {
    minuteswest: i32,
    dsttime: i32,
}

extern "C" {
    fn gettimeofday(tv: *mut libc::timeval, tz: *mut HostTimezone) -> i32;
}

/// Guest `timeval`: two 64-bit fields.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestTimeval {
    pub seconds: i64,
    pub microseconds: i64,
}

/// Guest `timezone`: two 32-bit fields.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestTimezone {
    pub minuteswest: i32,
    pub dsttime: i32,
}

pub const NID_GETTIMEOFDAY: &str = "n88vx3C5nW8#I#J";

/// # Safety
/// `value` and `zone` must be null or valid guest memory for one `GuestTimeval` /
/// `GuestTimezone`.
pub unsafe extern "C" fn guest_gettimeofday(
    value: *mut GuestTimeval,
    zone: *mut GuestTimezone,
) -> i32 {
    let mut host: libc::timeval = std::mem::zeroed();
    let mut host_zone = HostTimezone {
        minuteswest: 0,
        dsttime: 0,
    };
    if gettimeofday(&mut host, &mut host_zone) != 0 {
        stop("host gettimeofday failed");
    }
    if !zone.is_null() {
        zone.write(GuestTimezone {
            minuteswest: host_zone.minuteswest,
            dsttime: host_zone.dsttime,
        });
    }
    if !value.is_null() {
        value.write(GuestTimeval {
            seconds: host.tv_sec as i64,
            microseconds: host.tv_usec as i64,
        });
    }
    0
}

/// Guest NID to function address, like `runtime_time_resolve`.
#[allow(function_casts_as_integer)]
pub fn resolve(name: &str) -> usize {
    if name == NID_GETTIMEOFDAY {
        guest_gettimeofday as usize
    } else {
        0
    }
}

/// C ABI entry point the loader calls while the legacy `runtime_time.c` is replaced.
///
/// # Safety
/// `name` must be a NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn runtime_time_resolve(name: *const c_char) -> usize {
    if name.is_null() {
        return 0;
    }
    match CStr::from_ptr(name).to_str() {
        Ok(name) => resolve(name),
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_the_guest_nid_only() {
        assert_eq!(resolve(NID_GETTIMEOFDAY), guest_gettimeofday as usize);
        assert_eq!(resolve("something else"), 0);
        let nid = std::ffi::CString::new(NID_GETTIMEOFDAY).unwrap();
        assert_eq!(
            unsafe { runtime_time_resolve(nid.as_ptr()) },
            guest_gettimeofday as usize
        );
        assert_eq!(unsafe { runtime_time_resolve(std::ptr::null()) }, 0);
    }

    #[test]
    fn writes_both_guest_structures() {
        let mut value = GuestTimeval {
            seconds: 0,
            microseconds: 0,
        };
        let mut zone = GuestTimezone {
            minuteswest: 0,
            dsttime: 0,
        };
        let result = unsafe { guest_gettimeofday(&mut value, &mut zone) };
        assert_eq!(result, 0);
        // The host clock is well past 2001; the microseconds stay in range.
        assert!(value.seconds > 1_000_000_000);
        assert!((0..1_000_000).contains(&value.microseconds));
        let _ = zone;
    }

    #[test]
    fn null_pointers_are_allowed() {
        assert_eq!(
            unsafe { guest_gettimeofday(std::ptr::null_mut(), std::ptr::null_mut()) },
            0
        );
    }
}
