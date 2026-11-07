//! POSIX helpers the renderer uses for diagnostics and thread identity, ported from
//! `legacy/gpu/shim/bb_win_compat.h`. On Linux this returns the kernel thread id.

/// The current OS thread id (Windows: `GetCurrentThreadId`, otherwise `gettid`).
#[cfg(windows)]
pub fn gettid() -> i32 {
    unsafe extern "system" {
        fn GetCurrentThreadId() -> u32;
    }
    unsafe { GetCurrentThreadId() as i32 }
}

#[cfg(target_os = "linux")]
pub fn gettid() -> i32 {
    unsafe { libc::syscall(libc::SYS_gettid) as i32 }
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn gettid() -> i32 {
    std::process::id() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_id_is_positive_and_stable_on_this_thread() {
        assert!(gettid() > 0);
        assert_eq!(gettid(), gettid());
    }
}
