//! A slot-indexed vector, ported from
//! `legacy/gpu/shadps4/common/slot_vector.h`.
//!
//! The C++ reserves 1 GiB of address space and commits 2 MiB chunks so entries never move. Rust
//! forbids keeping a reference across an insertion, so the port uses a `Vec<Option<T>>` with the
//! same free-list behaviour and chunked growth.

use std::hash::{Hash, Hasher};

/// `Common::SlotId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotId {
    index: u32,
}

impl Default for SlotId {
    fn default() -> Self {
        Self {
            index: Self::INVALID_INDEX,
        }
    }
}

impl SlotId {
    /// `SlotId::INVALID_INDEX`.
    pub const INVALID_INDEX: u32 = u32::MAX;

    /// `SlotId(u32)`.
    pub const fn new(index: u32) -> Self {
        Self { index }
    }

    /// The raw index.
    pub const fn index(self) -> u32 {
        self.index
    }

    /// `operator bool`: false only for the invalid id.
    pub const fn is_valid(self) -> bool {
        self.index != Self::INVALID_INDEX
    }
}

impl From<u32> for SlotId {
    fn from(index: u32) -> Self {
        Self::new(index)
    }
}

/// `Common::SlotVector`.
pub struct SlotVector<T> {
    values: Vec<Option<T>>,
    free_list: Vec<u32>,
}

impl<T> SlotVector<T> {
    /// `ChunkBytes`, committed at a time.
    pub const CHUNK_BYTES: usize = 2 << 20;
    /// `ReservedBytes`, the address-space budget.
    pub const RESERVED_BYTES: usize = 1 << 30;

    /// `SlotVector()`: the first chunk is added immediately.
    pub fn new() -> Self {
        let mut vector = SlotVector {
            values: Vec::new(),
            free_list: Vec::new(),
        };
        vector.add_chunk();
        vector
    }

    fn chunk_entries() -> usize {
        let entry_size = std::mem::size_of::<T>().max(1);
        (Self::CHUNK_BYTES / entry_size).max(1)
    }

    /// `AddChunk`: grows by one chunk and pushes the new indices on the free list.
    fn add_chunk(&mut self) {
        let old_capacity = self.values.len();
        let new_capacity = old_capacity + Self::chunk_entries();
        assert!(
            new_capacity.saturating_mul(std::mem::size_of::<T>().max(1)) <= Self::RESERVED_BYTES,
            "SlotVector out of space"
        );
        self.values.resize_with(new_capacity, || None);
        for index in (old_capacity..new_capacity).rev() {
            self.free_list.push(index as u32);
        }
    }

    /// `is_allocated`.
    pub fn is_allocated(&self, id: SlotId) -> bool {
        self.values
            .get(id.index as usize)
            .is_some_and(|slot| slot.is_some())
    }

    /// `insert`: a free slot takes the value.
    pub fn insert(&mut self, value: T) -> SlotId {
        if self.free_list.is_empty() {
            self.add_chunk();
        }
        let index = self.free_list.pop().expect("a free slot was added");
        self.values[index as usize] = Some(value);
        SlotId::new(index)
    }

    /// `erase`.
    pub fn erase(&mut self, id: SlotId) {
        self.values[id.index as usize] = None;
        self.free_list.push(id.index);
    }

    /// `size`.
    pub fn size(&self) -> usize {
        self.values.len() - self.free_list.len()
    }

    /// `operator[](id)`.
    pub fn get(&self, id: SlotId) -> &T {
        self.values[id.index as usize]
            .as_ref()
            .expect("the slot is allocated")
    }

    /// The mutable `operator[](id)`.
    pub fn get_mut(&mut self, id: SlotId) -> &mut T {
        self.values[id.index as usize]
            .as_mut()
            .expect("the slot is allocated")
    }

    /// `begin()` iteration over the allocated values.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.values.iter().flatten()
    }

    /// The mutable iteration.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.values.iter_mut().flatten()
    }
}

impl<T> Default for SlotVector<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> std::ops::Index<SlotId> for SlotVector<T> {
    type Output = T;

    fn index(&self, id: SlotId) -> &T {
        self.get(id)
    }
}

impl<T> std::ops::IndexMut<SlotId> for SlotVector<T> {
    fn index_mut(&mut self, id: SlotId) -> &mut T {
        self.get_mut(id)
    }
}

/// The hash the C++ specializes for `SlotId`.
pub fn hash_slot_id(id: SlotId) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.index.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_insert_takes_a_free_slot_and_erase_returns_it() {
        let mut vector = SlotVector::new();
        assert_eq!(vector.size(), 0);
        let first = vector.insert(10u32);
        let second = vector.insert(20u32);
        assert_ne!(first, second);
        assert!(vector.is_allocated(first));
        assert_eq!(vector[first], 10);
        assert_eq!(vector[second], 20);
        assert_eq!(vector.size(), 2);
        vector.erase(first);
        assert!(!vector.is_allocated(first));
        assert_eq!(vector.size(), 1);
        let reused = vector.insert(30u32);
        assert_eq!(reused, first, "the erased slot comes back");
        assert_eq!(vector[reused], 30);
    }

    #[test]
    fn iteration_skips_the_free_slots() {
        let mut vector = SlotVector::new();
        let a = vector.insert(1u32);
        let b = vector.insert(2u32);
        let c = vector.insert(3u32);
        vector.erase(b);
        let values: Vec<u32> = vector.iter().copied().collect();
        assert_eq!(values, vec![1, 3]);
        for value in vector.iter_mut() {
            *value += 10;
        }
        assert_eq!(vector[a], 11);
        assert_eq!(vector[c], 13);
    }

    #[test]
    fn the_invalid_id_is_the_default() {
        assert_eq!(SlotId::default().index(), SlotId::INVALID_INDEX);
        assert!(!SlotId::default().is_valid());
        assert!(SlotId::new(0).is_valid());
        assert_eq!(SlotId::from(7), SlotId::new(7));
    }
}
