//! Thread identity, ported from `legacy/gpu/bbnet/include/bb_win_compat.h`. The C++ maps
//! Windows' `GetCurrentThreadId` onto `gettid`; the port asks the host for the same thing.

/// `gettid()`: the calling thread's id, as the POSIX call or the platform equivalent.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn get_tid() -> i32 {
    unsafe { libc::gettid() }
}

/// On platforms without `gettid`, the port reports the running host thread's index.
#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub fn get_tid() -> i32 {
    use std::cell::Cell;
    thread_local! {
        static ID: Cell<i32> = const { Cell::new(0) };
    }
    ID.with(|id| {
        if id.get() == 0 {
            id.set(next_thread_index());
        }
        id.get()
    })
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn next_thread_index() -> i32 {
    use std::sync::atomic::{AtomicI32, Ordering};
    static NEXT: AtomicI32 = AtomicI32::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_thread_id_is_stable_within_a_thread_and_distinct_between_live_ones() {
        let first = get_tid();
        assert!(first > 0, "a thread id is positive");
        assert_eq!(get_tid(), first, "the same thread keeps its id");
        let other = std::thread::spawn(get_tid).join().unwrap();
        assert!(other > 0);
        assert_ne!(other, first, "two live threads have different ids");
    }
}
