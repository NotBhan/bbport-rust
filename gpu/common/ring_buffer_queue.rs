//! A fixed-size ring buffer, ported from `legacy/gpu/shadps4/common/ring_buffer_queue.h`. The
//! C++ stores default-constructed values and overwrites the oldest once the ring is full; the
//! port keeps empty slots instead, so `T` need not be default-constructible.

/// `RingBufferQueue<T>`.
#[derive(Debug, Clone)]
pub struct RingBufferQueue<T> {
    storage: Vec<Option<T>>,
    begin: u64,
    size: u64,
}

impl<T> RingBufferQueue<T> {
    /// `RingBufferQueue(u64 size)`.
    pub fn new(size: u64) -> Self {
        Self {
            storage: (0..size).map(|_| None).collect(),
            begin: 0,
            size: 0,
        }
    }

    /// `Push`: writes at the end of the queue, dropping the oldest entry when the ring is full.
    pub fn push(&mut self, item: T) {
        let capacity = self.storage.len() as u64;
        if capacity == 0 {
            return;
        }
        let index = ((self.begin + self.size) % capacity) as usize;
        self.storage[index] = Some(item);
        if self.size < capacity {
            self.size += 1;
        } else {
            self.begin = (self.begin + 1) % capacity;
        }
    }

    /// `Pop`: takes the oldest entry.
    pub fn pop(&mut self) -> Option<T> {
        if self.size == 0 {
            return None;
        }
        let index = self.begin as usize;
        let value = self.storage[index].take();
        self.begin = (self.begin + 1) % self.storage.len() as u64;
        self.size -= 1;
        value
    }

    /// `Peek`: the oldest entry, left in place.
    pub fn peek(&self) -> Option<&T> {
        if self.size == 0 {
            return None;
        }
        self.storage[self.begin as usize].as_ref()
    }

    /// `Clear`.
    pub fn clear(&mut self) {
        for slot in &mut self.storage {
            *slot = None;
        }
        self.begin = 0;
        self.size = 0;
    }

    /// How many entries the ring holds.
    pub fn len(&self) -> u64 {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// The ring's fixed capacity.
    pub fn capacity(&self) -> u64 {
        self.storage.len() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static DROPPED: AtomicUsize = AtomicUsize::new(0);

    struct Tracked(u32);

    impl Drop for Tracked {
        fn drop(&mut self) {
            DROPPED.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn the_ring_is_first_in_first_out() {
        let mut queue = RingBufferQueue::new(3);
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.pop(), None);
        assert_eq!(queue.peek(), None);
        queue.push(1);
        queue.push(2);
        assert_eq!(queue.peek(), Some(&1));
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn a_full_ring_drops_the_oldest_entry() {
        let mut queue = RingBufferQueue::new(2);
        queue.push(1);
        queue.push(2);
        queue.push(3);
        assert_eq!(queue.len(), 2, "the ring stays at its capacity");
        assert_eq!(queue.pop(), Some(2), "the oldest entry was overwritten");
        assert_eq!(queue.pop(), Some(3));

        // Wrapping around the end of the storage many times keeps the order.
        let mut queue = RingBufferQueue::new(4);
        for value in 0..100 {
            queue.push(value);
        }
        assert_eq!(queue.len(), 4);
        assert_eq!(queue.pop(), Some(96));
        assert_eq!(queue.pop(), Some(97));
        assert_eq!(queue.pop(), Some(98));
        assert_eq!(queue.pop(), Some(99));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn the_overwritten_entry_is_dropped_and_clearing_empties_the_ring() {
        DROPPED.store(0, Ordering::Relaxed);
        let mut queue = RingBufferQueue::new(2);
        queue.push(Tracked(1));
        queue.push(Tracked(2));
        queue.push(Tracked(3));
        assert_eq!(DROPPED.load(Ordering::Relaxed), 1, "the overwritten value");
        assert_eq!(queue.peek().map(|value| value.0), Some(2));
        queue.clear();
        assert!(queue.is_empty());
        assert_eq!(
            DROPPED.load(Ordering::Relaxed),
            3,
            "clearing drops the rest"
        );
        assert!(queue.pop().is_none());

        // A zero-sized ring accepts nothing rather than dividing by zero.
        let mut empty = RingBufferQueue::<u8>::new(0);
        empty.push(1);
        assert_eq!(empty.pop(), None);
        assert_eq!(empty.capacity(), 0);
    }
}
