//! A chunked object pool, ported from `legacy/gpu/shadps4/common/object_pool.h`. The C++ hands
//! out pointers into raw storage and runs the destructors itself; the port keeps chunks of
//! values and drops them whole, with the same squashing on `ReleaseContents`.

/// `Common::ObjectPool<T>`.
#[derive(Debug)]
pub struct ObjectPool<T> {
    chunks: Vec<Vec<T>>,
    new_chunk_size: usize,
    total_created: usize,
}

impl<T> Default for ObjectPool<T> {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl<T> ObjectPool<T> {
    /// The C++'s default chunk size.
    pub const DEFAULT_CHUNK_SIZE: usize = 1024;

    /// `ObjectPool(size_t chunk_size)`: the first chunk is allocated up front.
    pub fn new(chunk_size: usize) -> Self {
        let size = chunk_size.max(1);
        Self {
            chunks: vec![Vec::with_capacity(size)],
            new_chunk_size: size,
            total_created: 0,
        }
    }

    /// `ObjectPool::Create`: the value lands in the pool and a reference to it comes back, the
    /// way the C++ returns a pointer into its storage.
    pub fn create(&mut self, value: T) -> &mut T {
        self.total_created += 1;
        let chunk = self.free_chunk();
        chunk.push(value);
        chunk.last_mut().expect("the value was just pushed")
    }

    /// The C++ fills a new chunk when the current one is full.
    fn free_chunk(&mut self) -> &mut Vec<T> {
        let last_is_full = self
            .chunks
            .last()
            .map(|chunk| chunk.len() == chunk.capacity())
            .unwrap_or(true);
        if last_is_full {
            self.chunks.push(Vec::with_capacity(self.new_chunk_size));
        }
        self.chunks.last_mut().expect("a chunk is always present")
    }

    /// `ObjectPool::ReleaseContents`: the values are dropped, and a full root chunk is replaced
    /// by one big enough for everything the pool has handed out.
    pub fn release_contents(&mut self) {
        if self.chunks.is_empty() {
            return;
        }
        let root_is_full = self.chunks[0].len() == self.chunks[0].capacity();
        let total = if root_is_full {
            self.chunks
                .iter()
                .map(|chunk| chunk.capacity())
                .sum::<usize>()
        } else {
            self.chunks[0].capacity()
        };
        self.chunks.clear();
        self.chunks.push(Vec::with_capacity(total.max(1)));
        self.chunks.shrink_to_fit();
        self.total_created = 0;
    }

    /// How many objects the pool has handed out since the last release.
    pub fn created(&self) -> usize {
        self.total_created
    }

    /// How many chunks the pool is holding.
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// How many objects fit before a new chunk is needed.
    pub fn capacity(&self) -> usize {
        self.chunks.iter().map(|chunk| chunk.capacity()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.total_created == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static DROPPED: AtomicU64 = AtomicU64::new(0);

    struct Tracked(#[allow(dead_code)] u32);

    impl Drop for Tracked {
        fn drop(&mut self) {
            DROPPED.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn a_pool_hands_out_values_and_grows_by_a_chunk_at_a_time() {
        // Plain values: the drop counter belongs to the other test.
        let mut pool = ObjectPool::new(4);
        assert!(pool.is_empty());
        assert_eq!(pool.chunk_count(), 1);
        for index in 0..4u32 {
            let value = pool.create(index);
            assert_eq!(*value, index);
        }
        assert_eq!(pool.chunk_count(), 1, "four fit in the first chunk");
        pool.create(4u32);
        assert_eq!(pool.chunk_count(), 2, "the fifth opens a new chunk");
        assert_eq!(pool.created(), 5);
        assert_eq!(pool.capacity(), 8);

        // Referencing a value from a later chunk works the same way.
        let mut pool = ObjectPool::new(1);
        let first = pool.create(1u32);
        *first += 1;
        assert_eq!(*pool.create(2u32), 2);
        assert_eq!(pool.chunk_count(), 2);
    }

    #[test]
    fn releasing_drops_the_values_and_keeps_the_allocation() {
        DROPPED.store(0, Ordering::Relaxed);
        let mut pool = ObjectPool::new(2);
        pool.create(Tracked(1));
        pool.create(Tracked(2));
        pool.release_contents();
        assert_eq!(DROPPED.load(Ordering::Relaxed), 2);
        assert_eq!(pool.created(), 0);
        assert_eq!(pool.chunk_count(), 1);
        assert_eq!(pool.capacity(), 2, "the half-filled root keeps its size");

        // A full root chunk is squashed into a single bigger one.
        let mut pool = ObjectPool::new(2);
        for index in 0..4 {
            pool.create(index);
        }
        assert_eq!(pool.chunk_count(), 2);
        pool.release_contents();
        assert_eq!(pool.chunk_count(), 1);
        assert_eq!(pool.capacity(), 4);
        assert!(pool.is_empty());
    }
}
