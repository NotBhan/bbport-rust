//! A fixed-capacity slot array, ported from `legacy/gpu/shadps4/common/slot_array.h`.

/// `Common::SlotArray`: indices are handed out in order and returned on destruction.
#[derive(Debug, Clone)]
pub struct SlotArray<T, const N: usize> {
    cursor: usize,
    free_indices: Vec<u32>,
    resources: Vec<Option<T>>,
}

impl<T, const N: usize> Default for SlotArray<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> SlotArray<T, N> {
    /// `SlotArray()`: every index starts free.
    pub fn new() -> Self {
        Self {
            cursor: 0,
            free_indices: (0..N as u32).collect(),
            resources: (0..N).map(|_| None).collect(),
        }
    }

    /// `Create`.
    pub fn create(&mut self, value: T) -> Option<u32> {
        if !self.has_free_slots() {
            return None;
        }
        let index = self.free_indices[self.cursor];
        self.resources[index as usize] = Some(value);
        self.cursor += 1;
        Some(index)
    }

    /// `Destroy`.
    pub fn destroy(&mut self, index: u32) -> bool {
        let slot = &mut self.resources[index as usize];
        if slot.is_none() {
            return false;
        }
        self.cursor -= 1;
        self.free_indices[self.cursor] = index;
        *slot = None;
        true
    }

    /// `Get`.
    pub fn get(&self, index: u32) -> Option<&T> {
        self.resources[index as usize].as_ref()
    }

    /// The mutable `Get`.
    pub fn get_mut(&mut self, index: u32) -> Option<&mut T> {
        self.resources[index as usize].as_mut()
    }

    /// `HasFreeSlots`.
    pub fn has_free_slots(&self) -> bool {
        self.cursor < self.free_indices.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indices_are_handed_out_in_order_and_reused() {
        let mut array = SlotArray::<u32, 2>::new();
        assert_eq!(array.create(10), Some(0));
        assert_eq!(array.create(20), Some(1));
        assert_eq!(array.create(30), None, "the capacity is reached");
        assert_eq!(array.get(0), Some(&10));
        assert_eq!(array.get(1), Some(&20));
        assert!(array.destroy(0));
        assert!(!array.destroy(0));
        assert_eq!(array.get(0), None);
        assert_eq!(array.create(40), Some(0));
        assert_eq!(array.get(0), Some(&40));
        *array.get_mut(0).unwrap() = 41;
        assert_eq!(array.get(0), Some(&41));
    }
}
