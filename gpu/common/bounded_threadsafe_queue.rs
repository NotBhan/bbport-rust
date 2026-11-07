//! Bounded thread-safe queues, ported from
//! `legacy/gpu/shadps4/common/bounded_threadsafe_queue.h`.
//!
//! The C++ SPSC queue is a lock-free ring; the port keeps the same bounded try/wait semantics
//! behind a mutex and two condition variables, which also makes the MPSC/MPMC wrappers safe
//! without trusting the caller to keep one side single-threaded. The stop-token pop becomes a
//! timeout variant returning `None`.

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

/// `Common::detail::DefaultCapacity`.
pub const DEFAULT_CAPACITY: usize = 0x1000;

#[derive(Debug)]
struct QueueState<T> {
    items: VecDeque<T>,
}

/// `Common::SPSCQueue`.
#[derive(Debug)]
pub struct SPSCQueue<T, const CAPACITY: usize = DEFAULT_CAPACITY> {
    state: Mutex<QueueState<T>>,
    not_empty: Condvar,
    not_full: Condvar,
}

impl<T, const CAPACITY: usize> Default for SPSCQueue<T, CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const CAPACITY: usize> SPSCQueue<T, CAPACITY> {
    /// `SPSCQueue()`, with the header's power-of-two capacity assertion.
    pub fn new() -> Self {
        assert!(
            CAPACITY.is_power_of_two(),
            "Capacity must be a power of two."
        );
        Self {
            state: Mutex::new(QueueState {
                items: VecDeque::with_capacity(CAPACITY),
            }),
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, QueueState<T>> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    /// `TryEmplace`.
    pub fn try_emplace(&self, value: T) -> bool {
        let mut state = self.lock();
        if state.items.len() == CAPACITY {
            return false;
        }
        state.items.push_back(value);
        drop(state);
        self.not_empty.notify_one();
        true
    }

    /// `EmplaceWait`.
    pub fn emplace_wait(&self, value: T) {
        let mut state = self.lock();
        while state.items.len() == CAPACITY {
            state = self
                .not_full
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }
        state.items.push_back(value);
        drop(state);
        self.not_empty.notify_one();
    }

    /// `TryPop`.
    pub fn try_pop(&self) -> Option<T> {
        let mut state = self.lock();
        let value = state.items.pop_front();
        drop(state);
        if value.is_some() {
            self.not_full.notify_one();
        }
        value
    }

    /// `PopWait`.
    pub fn pop_wait(&self) -> T {
        let mut state = self.lock();
        while state.items.is_empty() {
            state = self
                .not_empty
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }
        let value = state.items.pop_front().expect("the queue is not empty");
        drop(state);
        self.not_full.notify_one();
        value
    }

    /// The stop-token `PopWait`: `None` when the timeout expires or the queue is empty.
    pub fn pop_wait_timeout(&self, timeout: Duration) -> Option<T> {
        let start = Instant::now();
        let mut state = self.lock();
        while state.items.is_empty() {
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                return None;
            }
            let (next, result) = self
                .not_empty
                .wait_timeout(state, remaining)
                .unwrap_or_else(|error| error.into_inner());
            state = next;
            if result.timed_out() && state.items.is_empty() {
                return None;
            }
        }
        let value = state.items.pop_front();
        drop(state);
        self.not_full.notify_one();
        value
    }

    /// The number of items currently queued.
    pub fn len(&self) -> usize {
        self.lock().items.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.lock().items.is_empty()
    }
}

/// `Common::MPSCQueue`: producers are serialized, the consumer side is forwarded.
#[derive(Debug)]
pub struct MPSCQueue<T, const CAPACITY: usize = DEFAULT_CAPACITY> {
    queue: SPSCQueue<T, CAPACITY>,
    write_mutex: Mutex<()>,
}

impl<T, const CAPACITY: usize> MPSCQueue<T, CAPACITY> {
    pub fn new() -> Self {
        Self {
            queue: SPSCQueue::new(),
            write_mutex: Mutex::new(()),
        }
    }

    pub fn try_emplace(&self, value: T) -> bool {
        let _guard = self
            .write_mutex
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.queue.try_emplace(value)
    }

    pub fn emplace_wait(&self, value: T) {
        let _guard = self
            .write_mutex
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.queue.emplace_wait(value);
    }

    pub fn try_pop(&self) -> Option<T> {
        self.queue.try_pop()
    }

    pub fn pop_wait(&self) -> T {
        self.queue.pop_wait()
    }

    pub fn pop_wait_timeout(&self, timeout: Duration) -> Option<T> {
        self.queue.pop_wait_timeout(timeout)
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl<T, const CAPACITY: usize> Default for MPSCQueue<T, CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

/// `Common::MPMCQueue`: both sides are serialized.
#[derive(Debug)]
pub struct MPMCQueue<T, const CAPACITY: usize = DEFAULT_CAPACITY> {
    queue: SPSCQueue<T, CAPACITY>,
    write_mutex: Mutex<()>,
    read_mutex: Mutex<()>,
}

impl<T, const CAPACITY: usize> MPMCQueue<T, CAPACITY> {
    pub fn new() -> Self {
        Self {
            queue: SPSCQueue::new(),
            write_mutex: Mutex::new(()),
            read_mutex: Mutex::new(()),
        }
    }

    pub fn try_emplace(&self, value: T) -> bool {
        let _guard = self
            .write_mutex
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.queue.try_emplace(value)
    }

    pub fn emplace_wait(&self, value: T) {
        let _guard = self
            .write_mutex
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.queue.emplace_wait(value);
    }

    pub fn try_pop(&self) -> Option<T> {
        let _guard = self
            .read_mutex
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.queue.try_pop()
    }

    pub fn pop_wait(&self) -> T {
        let _guard = self
            .read_mutex
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.queue.pop_wait()
    }

    pub fn pop_wait_timeout(&self, timeout: Duration) -> Option<T> {
        let _guard = self
            .read_mutex
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.queue.pop_wait_timeout(timeout)
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl<T, const CAPACITY: usize> Default for MPMCQueue<T, CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_queue_rejects_try_emplace() {
        let queue = SPSCQueue::<u32, 2>::new();
        assert!(queue.try_emplace(1));
        assert!(queue.try_emplace(2));
        assert!(!queue.try_emplace(3));
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.try_pop(), Some(1));
        assert!(queue.try_emplace(3));
        assert_eq!(queue.try_pop(), Some(2));
        assert_eq!(queue.try_pop(), Some(3));
        assert_eq!(queue.try_pop(), None);
    }

    #[test]
    fn a_waiting_consumer_wakes_for_a_push() {
        let queue = std::sync::Arc::new(SPSCQueue::<u32, 4>::new());
        let producer = queue.clone();
        let thread = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(10));
            producer.emplace_wait(42);
        });
        assert_eq!(queue.pop_wait(), 42);
        thread.join().unwrap();
    }

    #[test]
    fn a_waiting_producer_wakes_for_a_pop() {
        let queue = std::sync::Arc::new(SPSCQueue::<u32, 1>::new());
        assert!(queue.try_emplace(1));
        let producer = queue.clone();
        let thread = std::thread::spawn(move || producer.emplace_wait(2));
        assert_eq!(queue.pop_wait(), 1);
        assert_eq!(queue.pop_wait(), 2);
        thread.join().unwrap();
    }

    #[test]
    fn the_timeout_matches_the_stop_token_case() {
        let queue = SPSCQueue::<u32, 4>::new();
        assert_eq!(queue.pop_wait_timeout(Duration::from_millis(1)), None);
        queue.emplace_wait(5);
        assert_eq!(queue.pop_wait_timeout(Duration::from_millis(1)), Some(5));
    }

    #[test]
    fn the_multi_producer_and_multi_consumer_wrappers_forward() {
        let mpsc = MPSCQueue::<u32, 4>::new();
        assert!(mpsc.try_emplace(1));
        assert_eq!(mpsc.try_pop(), Some(1));
        let mpmc = MPMCQueue::<u32, 4>::new();
        assert!(mpmc.try_emplace(2));
        assert_eq!(mpmc.len(), 1);
        assert_eq!(mpmc.try_pop(), Some(2));
        assert!(mpmc.is_empty());
    }
}
