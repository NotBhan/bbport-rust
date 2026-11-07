//! Range locking over a slice of mutexes, ported from
//! `legacy/gpu/shadps4/common/range_lock.h`.
//!
//! The C++ template works on any forward iterator of lock-like objects and uses the boost
//! algorithm to avoid deadlock when two threads lock overlapping ranges. The port covers the
//! usage in the page manager: a slice of `Mutex<()>`, acquired in index order so overlapping
//! ranges cannot deadlock.

use std::sync::{Mutex, MutexGuard};

/// `Common::RangeLockGuard`: holds every lock of the range until dropped or released.
pub struct RangeLockGuard<'a> {
    guards: Vec<MutexGuard<'a, ()>>,
}

impl<'a> RangeLockGuard<'a> {
    /// `release`.
    pub fn release(&mut self) {
        self.guards.clear();
    }

    /// The number of held locks, for tests and diagnostics.
    pub fn len(&self) -> usize {
        self.guards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.guards.is_empty()
    }
}

/// `Common::LockRange`.
pub fn lock_range(locks: &[Mutex<()>]) -> RangeLockGuard<'_> {
    RangeLockGuard {
        guards: locks.iter().map(|lock| lock.lock().unwrap()).collect(),
    }
}

/// `Common::TryLockRange`.
pub fn try_lock_range(locks: &[Mutex<()>]) -> Option<RangeLockGuard<'_>> {
    let mut guards = Vec::with_capacity(locks.len());
    for lock in locks {
        guards.push(lock.try_lock().ok()?);
    }
    Some(RangeLockGuard { guards })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn a_range_holds_every_lock_until_released() {
        let locks: Vec<Mutex<()>> = (0..3).map(|_| Mutex::new(())).collect();
        let mut guard = lock_range(&locks);
        assert_eq!(guard.len(), 3);
        assert!(locks[0].try_lock().is_err());
        guard.release();
        assert!(guard.is_empty());
        assert!(locks[0].try_lock().is_ok());
    }

    #[test]
    fn a_try_lock_range_fails_when_any_lock_is_held() {
        let locks: Arc<Vec<Mutex<()>>> = Arc::new((0..2).map(|_| Mutex::new(())).collect());
        let held = locks[1].lock().unwrap();
        assert!(try_lock_range(&locks).is_none());
        drop(held);
        let guard = try_lock_range(&locks).expect("all locks are free");
        assert_eq!(guard.len(), 2);
    }
}
