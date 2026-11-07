//! A reader-priority shared mutex, ported from
//! `legacy/gpu/shadps4/common/shared_first_mutex.h`.

use std::sync::{Condvar, Mutex, MutexGuard};

#[derive(Debug, Default)]
struct State {
    readers: i32,
    writer_active: bool,
}

/// `Common::SharedFirstMutex`: like `std::shared_mutex`, but a waiting reader wins over a
/// waiting writer.
#[derive(Debug, Default)]
pub struct SharedFirstMutex {
    state: Mutex<State>,
    cv: Condvar,
}

impl SharedFirstMutex {
    pub const fn new() -> Self {
        Self {
            state: Mutex::new(State {
                readers: 0,
                writer_active: false,
            }),
            cv: Condvar::new(),
        }
    }

    fn lock_state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    /// `lock`.
    pub fn lock(&self) {
        let mut state = self.lock_state();
        while state.writer_active || state.readers != 0 {
            state = self
                .cv
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }
        state.writer_active = true;
    }

    /// `try_lock`.
    pub fn try_lock(&self) -> bool {
        let mut state = self.lock_state();
        if state.writer_active || state.readers > 0 {
            return false;
        }
        state.writer_active = true;
        true
    }

    /// `unlock`.
    pub fn unlock(&self) {
        let mut state = self.lock_state();
        state.writer_active = false;
        drop(state);
        self.cv.notify_all();
    }

    /// `lock_shared`.
    pub fn lock_shared(&self) {
        let mut state = self.lock_state();
        while state.writer_active {
            state = self
                .cv
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }
        state.readers += 1;
    }

    /// `try_lock_shared`.
    pub fn try_lock_shared(&self) -> bool {
        let mut state = self.lock_state();
        if state.writer_active {
            return false;
        }
        state.readers += 1;
        true
    }

    /// `unlock_shared`.
    pub fn unlock_shared(&self) {
        let mut state = self.lock_state();
        state.readers -= 1;
        if state.readers == 0 {
            drop(state);
            self.cv.notify_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_writer_excludes_readers_and_other_writers() {
        let mutex = SharedFirstMutex::new();
        assert!(mutex.try_lock());
        assert!(!mutex.try_lock_shared());
        assert!(!mutex.try_lock());
        mutex.unlock();
        assert!(mutex.try_lock_shared());
        assert!(mutex.try_lock_shared());
        mutex.unlock_shared();
        assert!(!mutex.try_lock(), "a reader is still active");
        mutex.unlock_shared();
        assert!(mutex.try_lock());
        mutex.unlock();
    }

    #[test]
    fn lock_waits_for_the_writer_to_finish() {
        let mutex = std::sync::Arc::new(SharedFirstMutex::new());
        mutex.lock();
        let other = mutex.clone();
        let thread = std::thread::spawn(move || {
            other.lock_shared();
            other.unlock_shared();
        });
        // Give the thread a moment to block, then release the writer.
        std::thread::sleep(std::time::Duration::from_millis(10));
        mutex.unlock();
        thread.join().unwrap();
    }
}
