//! A fixed-object slab heap, ported from `legacy/gpu/shadps4/common/slab_heap.h`.
//!
//! The C++ runs an intrusive free list over caller-provided memory. The safe port keeps the
//! same index-oriented behaviour with an `Option<T>` per object: `allocate` pops a free index,
//! `free` returns the value and pushes the index back.

/// `Common::SlabHeap<T>`.
#[derive(Debug, Clone)]
pub struct SlabHeap<T> {
    objects: Vec<Option<T>>,
    free: Vec<usize>,
    object_size: usize,
}

impl<T> SlabHeap<T> {
    /// The C++ `Initialize(obj_size, memory, memory_size)` over a typed object array.
    pub fn new(object_count: usize) -> Self {
        let mut free = Vec::with_capacity(object_count);
        // The C++ frees from the end, leaving the first object on top of the free list.
        for index in (0..object_count).rev() {
            free.push(index);
        }
        Self {
            objects: (0..object_count).map(|_| None).collect(),
            free,
            object_size: std::mem::size_of::<T>(),
        }
    }

    /// `Initialize(memory, memory_size)` from a byte budget.
    pub fn with_memory_size(memory_size: usize) -> Self {
        let object_size = std::mem::size_of::<T>().max(1);
        Self::new(memory_size / object_size)
    }

    /// `Allocate`, with the value constructed into the slot.
    pub fn allocate(&mut self, value: T) -> Option<usize> {
        let index = self.free.pop()?;
        self.objects[index] = Some(value);
        Some(index)
    }

    /// `Free`.
    pub fn free(&mut self, index: usize) -> Option<T> {
        let value = self.objects[index].take();
        if value.is_some() {
            self.free.push(index);
        }
        value
    }

    /// `Contains` for an object index.
    pub fn contains(&self, index: usize) -> bool {
        index < self.objects.len()
    }

    /// The object at a slot.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.objects.get(index).and_then(Option::as_ref)
    }

    /// The mutable object at a slot.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.objects.get_mut(index).and_then(Option::as_mut)
    }

    /// `GetSlabHeapSize`.
    pub fn slab_heap_size(&self) -> usize {
        self.objects.len()
    }

    /// `GetObjectSize`.
    pub fn object_size(&self) -> usize {
        self.object_size
    }

    /// `GetObjectIndex`, over slot indices.
    pub fn object_index(&self, index: usize) -> usize {
        index
    }

    /// `GetNumRemaining`, which the C++ only computes in debug builds.
    pub fn num_remaining(&self) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocation_walks_the_slots_in_order() {
        let mut heap = SlabHeap::<u32>::new(3);
        assert_eq!(heap.allocate(10), Some(0));
        assert_eq!(heap.allocate(20), Some(1));
        assert_eq!(heap.allocate(30), Some(2));
        assert_eq!(heap.allocate(40), None);
        assert_eq!(heap.get(0), Some(&10));
        assert_eq!(heap.get(2), Some(&30));
        assert_eq!(heap.slab_heap_size(), 3);
        assert_eq!(heap.object_size(), 4);
        assert_eq!(heap.num_remaining(), 0);
    }

    #[test]
    fn freed_slots_come_back_in_lifo_order() {
        let mut heap = SlabHeap::<u32>::new(2);
        let first = heap.allocate(1).unwrap();
        let second = heap.allocate(2).unwrap();
        assert_eq!(heap.free(first), Some(1));
        assert_eq!(heap.free(first), None);
        assert_eq!(heap.allocate(3), Some(first));
        assert_eq!(heap.get(second), Some(&2));
    }

    #[test]
    fn a_byte_budget_rounds_down_to_whole_objects() {
        let heap = SlabHeap::<u32>::with_memory_size(10);
        assert_eq!(heap.slab_heap_size(), 2);
        assert!(heap.contains(1));
        assert!(!heap.contains(2));
    }
}
