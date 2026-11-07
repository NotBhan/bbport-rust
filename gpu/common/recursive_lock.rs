//! The recursive-lock bookkeeping, ported from `legacy/gpu/shadps4/common/recursive_lock.cpp`.
//! The C++ keeps one thread-local table of lock states; the port keeps the same table, keyed
//! by the lock's address, with the same asserts.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;

/// `Common::Detail::RecursiveLockType`: an idle lock, and the two ways a caller can hold it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecursiveLockType {
    None,
    Shared,
    Exclusive,
}

#[derive(Debug, Clone, Copy)]
struct RecursiveLockState {
    lock_type: RecursiveLockType,
    count: u32,
}

thread_local! {
    static LOCKS: RefCell<HashMap<usize, RecursiveLockState>> = RefCell::new(HashMap::new());
}

/// `Detail::IncrementRecursiveLock`: true when this call is the one that has to take the lock.
pub fn increment_recursive_lock(mutex: *mut c_void, lock_type: RecursiveLockType) -> bool {
    LOCKS.with(|locks| {
        let mut locks = locks.borrow_mut();
        let state = locks.entry(mutex as usize).or_insert(RecursiveLockState {
            lock_type: RecursiveLockType::None,
            count: 0,
        });
        if state.count == 0 {
            assert_eq!(
                state.lock_type,
                RecursiveLockType::None,
                "an idle lock has no holder"
            );
            state.lock_type = lock_type;
        }
        assert_eq!(
            state.lock_type, lock_type,
            "the lock is held the other way in this thread"
        );
        let first = state.count == 0;
        state.count += 1;
        first
    })
}

/// `Detail::DecrementRecursiveLock`: true when this call released the lock.
pub fn decrement_recursive_lock(mutex: *mut c_void, lock_type: RecursiveLockType) -> bool {
    LOCKS.with(|locks| {
        let mut locks = locks.borrow_mut();
        let state = locks
            .get_mut(&(mutex as usize))
            .expect("the lock is held before it is released");
        assert_eq!(state.lock_type, lock_type, "the holder's type matches");
        assert!(state.count > 0, "the holder count is positive");
        state.count -= 1;
        if state.count == 0 {
            locks.remove(&(mutex as usize));
            return true;
        }
        false
    })
}

/// How many locks this thread holds. The C++ table is private; the port exposes the size for
/// the same reason a debugger would look at it.
pub fn held_locks() -> usize {
    LOCKS.with(|locks| locks.borrow().len())
}

/// `Common::RecursiveScopedLock` and `Common::RecursiveSharedLock` need this from the mutex
/// they wrap: the C++'s `std::unique_lock` and `std::shared_lock`.
pub trait RecursiveMutex {
    fn lock_exclusive(&self);
    fn unlock_exclusive(&self);
    fn lock_shared(&self);
    fn unlock_shared(&self);
}

/// `Common::RecursiveScopedLock<MutexType>`.
pub struct RecursiveScopedLock<'a, M: RecursiveMutex> {
    mutex: &'a M,
    held: bool,
}

impl<'a, M: RecursiveMutex> RecursiveScopedLock<'a, M> {
    /// The C++ constructor: this call takes the mutex only if it is the outermost holder.
    pub fn new(mutex: &'a M) -> Self {
        let held = increment_recursive_lock(
            mutex as *const M as *mut c_void,
            RecursiveLockType::Exclusive,
        );
        if held {
            mutex.lock_exclusive();
        }
        Self { mutex, held }
    }

    pub fn holds_the_mutex(&self) -> bool {
        self.held
    }
}

impl<M: RecursiveMutex> Drop for RecursiveScopedLock<'_, M> {
    fn drop(&mut self) {
        decrement_recursive_lock(
            self.mutex as *const M as *mut c_void,
            RecursiveLockType::Exclusive,
        );
        if self.held {
            self.mutex.unlock_exclusive();
        }
    }
}

/// `Common::RecursiveSharedLock<MutexType>`.
pub struct RecursiveSharedLock<'a, M: RecursiveMutex> {
    mutex: &'a M,
    held: bool,
}

impl<'a, M: RecursiveMutex> RecursiveSharedLock<'a, M> {
    pub fn new(mutex: &'a M) -> Self {
        let held =
            increment_recursive_lock(mutex as *const M as *mut c_void, RecursiveLockType::Shared);
        if held {
            mutex.lock_shared();
        }
        Self { mutex, held }
    }

    pub fn holds_the_mutex(&self) -> bool {
        self.held
    }
}

impl<M: RecursiveMutex> Drop for RecursiveSharedLock<'_, M> {
    fn drop(&mut self) {
        decrement_recursive_lock(
            self.mutex as *const M as *mut c_void,
            RecursiveLockType::Shared,
        );
        if self.held {
            self.mutex.unlock_shared();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn only_the_outermost_holder_takes_the_lock() {
        let mut lock = 0u8;
        let address = &mut lock as *mut u8 as *mut c_void;
        assert_eq!(held_locks(), 0);
        assert!(increment_recursive_lock(
            address,
            RecursiveLockType::Exclusive
        ));
        assert!(!increment_recursive_lock(
            address,
            RecursiveLockType::Exclusive
        ));
        assert_eq!(held_locks(), 1);
        assert!(!decrement_recursive_lock(
            address,
            RecursiveLockType::Exclusive
        ));
        assert!(decrement_recursive_lock(
            address,
            RecursiveLockType::Exclusive
        ));
        assert_eq!(held_locks(), 0, "the last release forgets the lock");
    }

    #[test]
    fn the_two_kinds_of_holder_do_not_mix_and_the_table_is_per_thread() {
        let mut lock = 0u8;
        let address = &mut lock as *mut u8 as *mut c_void;
        assert!(increment_recursive_lock(address, RecursiveLockType::Shared));
        let mismatch = std::panic::catch_unwind(|| {
            increment_recursive_lock(address, RecursiveLockType::Exclusive)
        });
        assert!(mismatch.is_err(), "the C++ asserts on the mismatch");
        assert!(decrement_recursive_lock(address, RecursiveLockType::Shared));

        let mut other = 0u8;
        let other_address = &mut other as *mut u8 as *mut c_void;
        increment_recursive_lock(other_address, RecursiveLockType::Shared);
        assert_eq!(held_locks(), 1);
        // The address travels as an integer: the lock it names is the same one.
        let other_as_integer = other_address as usize;
        std::thread::spawn(move || {
            let other_address = other_as_integer as *mut c_void;
            assert_eq!(held_locks(), 0, "another thread starts with no locks");
            assert!(increment_recursive_lock(
                other_address,
                RecursiveLockType::Exclusive
            ));
            assert_eq!(held_locks(), 1);
        })
        .join()
        .unwrap();
        assert_eq!(held_locks(), 1, "this thread still holds its own");
        decrement_recursive_lock(other_address, RecursiveLockType::Shared);
    }

    #[derive(Default)]
    struct CountingMutex {
        shared_locks: Cell<u32>,
        shared_unlocks: Cell<u32>,
        exclusive_locks: Cell<u32>,
        exclusive_unlocks: Cell<u32>,
    }

    impl RecursiveMutex for CountingMutex {
        fn lock_exclusive(&self) {
            self.exclusive_locks.set(self.exclusive_locks.get() + 1);
        }

        fn unlock_exclusive(&self) {
            self.exclusive_unlocks.set(self.exclusive_unlocks.get() + 1);
        }

        fn lock_shared(&self) {
            self.shared_locks.set(self.shared_locks.get() + 1);
        }

        fn unlock_shared(&self) {
            self.shared_unlocks.set(self.shared_unlocks.get() + 1);
        }
    }

    #[test]
    fn the_guards_take_the_mutex_once_and_release_it_once() {
        let mutex = CountingMutex::default();
        {
            let outer = RecursiveScopedLock::new(&mutex);
            assert!(outer.holds_the_mutex(), "the outermost holder takes it");
            {
                let inner = RecursiveScopedLock::new(&mutex);
                assert!(!inner.holds_the_mutex(), "the nested holder does not");
            }
            assert_eq!(mutex.exclusive_locks.get(), 1);
            assert_eq!(
                mutex.exclusive_unlocks.get(),
                0,
                "the outer holder still holds it"
            );
        }
        assert_eq!(mutex.exclusive_locks.get(), 1);
        assert_eq!(mutex.exclusive_unlocks.get(), 1);
        assert_eq!(
            held_locks(),
            0,
            "the lock is forgotten when the last holder leaves"
        );

        let mutex = CountingMutex::default();
        {
            let first = RecursiveSharedLock::new(&mutex);
            let second = RecursiveSharedLock::new(&mutex);
            assert!(first.holds_the_mutex() && !second.holds_the_mutex());
            assert_eq!(mutex.shared_locks.get(), 1);
        }
        assert_eq!(mutex.shared_unlocks.get(), 1);
        assert_eq!(
            mutex.exclusive_locks.get(),
            0,
            "a shared holder never takes it exclusively"
        );
    }
}
