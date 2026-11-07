//! An adaptive mutex, ported from `legacy/gpu/shadps4/common/adaptive_mutex.h`. The C++ uses
//! the non-portable `PTHREAD_MUTEX_ADAPTIVE_NP`; the port asks pthreads for the same protocol
//! and leaves the rest of the type alone.

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;

/// `Common::AdaptiveMutex`: a mutex that spins briefly before parking the thread.
pub struct AdaptiveMutex {
    mutex: UnsafeCell<libc::pthread_mutex_t>,
}

// The mutex serialises access; the cell is only reached through the lock.
unsafe impl Send for AdaptiveMutex {}
unsafe impl Sync for AdaptiveMutex {}

impl Default for AdaptiveMutex {
    fn default() -> Self {
        Self::new()
    }
}

impl AdaptiveMutex {
    pub fn new() -> Self {
        unsafe {
            let mut attr = MaybeUninit::<libc::pthread_mutexattr_t>::uninit();
            let mut mutex = MaybeUninit::<libc::pthread_mutex_t>::uninit();
            assert_eq!(
                libc::pthread_mutexattr_init(attr.as_mut_ptr()),
                0,
                "the attributes initialise"
            );
            let attr = attr.assume_init_mut();
            // The C++'s `PTHREAD_ADAPTIVE_MUTEX_INITIALIZER_NP` sets exactly this.
            libc::pthread_mutexattr_settype(attr, libc::PTHREAD_MUTEX_ADAPTIVE_NP);
            let result = libc::pthread_mutex_init(mutex.as_mut_ptr(), attr);
            libc::pthread_mutexattr_destroy(attr);
            assert_eq!(result, 0, "the mutex initialises");
            Self {
                mutex: UnsafeCell::new(mutex.assume_init()),
            }
        }
    }

    /// `lock()`.
    pub fn lock(&self) {
        unsafe { libc::pthread_mutex_lock(self.mutex.get()) };
    }

    /// `unlock()`.
    pub fn unlock(&self) {
        unsafe { libc::pthread_mutex_unlock(self.mutex.get()) };
    }

    /// `try_lock()`: true when this call took the lock.
    pub fn try_lock(&self) -> bool {
        unsafe { libc::pthread_mutex_trylock(self.mutex.get()) == 0 }
    }

    /// The Rust spelling of an RAII `lock_guard`.
    pub fn guard(&self) -> AdaptiveMutexGuard<'_> {
        self.lock();
        AdaptiveMutexGuard { mutex: self }
    }
}

impl Drop for AdaptiveMutex {
    fn drop(&mut self) {
        unsafe { libc::pthread_mutex_destroy(self.mutex.get()) };
    }
}

/// The Rust spelling of an RAII `lock_guard` around an [`AdaptiveMutex`].
pub struct AdaptiveMutexGuard<'a> {
    mutex: &'a AdaptiveMutex,
}

impl Drop for AdaptiveMutexGuard<'_> {
    fn drop(&mut self) {
        self.mutex.unlock();
    }
}

impl<'a> AdaptiveMutexGuard<'a> {
    pub fn mutex(&self) -> &AdaptiveMutex {
        self.mutex
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    #[test]
    fn the_mutex_is_exclusive_and_the_guard_releases_it() {
        let mutex = AdaptiveMutex::new();
        assert!(mutex.try_lock());
        assert!(
            !mutex.try_lock(),
            "the adaptive protocol still refuses a second holder"
        );
        mutex.unlock();
        {
            let _guard = mutex.guard();
            assert!(!mutex.try_lock());
        }
        assert!(mutex.try_lock(), "the guard released it");
        mutex.unlock();
    }

    #[test]
    fn holder_threads_serialise_a_shared_counter() {
        let mutex = Arc::new(AdaptiveMutex::new());
        let counter = Arc::new(AtomicU64::new(0));
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let mutex = Arc::clone(&mutex);
                let counter = Arc::clone(&counter);
                std::thread::spawn(move || {
                    for _ in 0..2_000 {
                        let _guard = mutex.guard();
                        counter.fetch_add(1, Ordering::Relaxed);
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(counter.load(Ordering::Relaxed), 8_000);
    }
}
