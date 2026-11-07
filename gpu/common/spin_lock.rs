//! A spin lock, ported from `legacy/gpu/shadps4/common/spin_lock.h`. The C++ class spins
//! instead of entering the supervisor; the Rust port keeps that and adds a guard so the lock
//! is released on the way out of a scope, as the C++ RAII callers do.

use std::sync::atomic::{AtomicBool, Ordering};

/// `Common::SpinLock`: for short critical sections only.
#[derive(Debug, Default)]
pub struct SpinLock {
    locked: AtomicBool,
}

impl SpinLock {
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
        }
    }

    /// `lock()`: spins until the flag is ours, pausing between attempts.
    pub fn lock(&self) {
        while !self.try_lock() {
            thread_pause();
        }
    }

    /// `unlock()`.
    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }

    /// `try_lock()`: true when this call took the lock.
    pub fn try_lock(&self) -> bool {
        !self.locked.swap(true, Ordering::Acquire)
    }

    /// Takes the lock and returns a guard that releases it when dropped.
    pub fn guard(&self) -> SpinLockGuard<'_> {
        self.lock();
        SpinLockGuard { lock: self }
    }

    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Acquire)
    }
}

/// The Rust spelling of an RAII `lock_guard` around a [`SpinLock`].
#[derive(Debug)]
pub struct SpinLockGuard<'a> {
    lock: &'a SpinLock,
}

impl Drop for SpinLockGuard<'_> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

impl<'a> SpinLockGuard<'a> {
    pub fn lock(&self) -> &SpinLock {
        self.lock
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn the_flag_is_owned_by_one_holder_at_a_time() {
        let lock = SpinLock::new();
        assert!(!lock.is_locked());
        assert!(lock.try_lock());
        assert!(lock.is_locked());
        assert!(!lock.try_lock(), "the flag is already taken");
        lock.unlock();
        assert!(!lock.is_locked());
        {
            let _guard = lock.guard();
            assert!(lock.is_locked());
        }
        assert!(!lock.is_locked(), "the guard released it");
    }

    #[test]
    fn spinning_holders_serialise_a_shared_counter() {
        let lock = Arc::new(SpinLock::new());
        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let lock = Arc::clone(&lock);
                let counter = Arc::clone(&counter);
                std::thread::spawn(move || {
                    for _ in 0..2_000 {
                        let _guard = lock.guard();
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

/// The anonymous `ThreadPause()` in `spin_lock.cpp`: the architecture's hint that this thread
/// is spinning, so the core can give its resources to the other hyperthread.
pub fn thread_pause() {
    #[cfg(target_arch = "x86_64")]
    {
        core::arch::x86_64::_mm_pause();
    }
    #[cfg(target_arch = "aarch64")]
    {
        // SAFETY: `yield` has no operands and touches no memory.
        unsafe {
            core::arch::asm!("yield");
        }
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    std::hint::spin_loop();
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn the_pause_is_callable_from_a_spin() {
        for _ in 0..64 {
            thread_pause();
        }
        // The lock still takes and releases after pausing inside it.
        let lock = SpinLock::new();
        lock.lock();
        lock.unlock();
        assert!(!lock.is_locked());
    }
}
