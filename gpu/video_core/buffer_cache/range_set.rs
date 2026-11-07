//! Address range sets and maps, ported from
//! `legacy/gpu/shadps4/video_core/buffer_cache/range_set.h`.
//!
//! The C++ builds on Boost.Icl; the port keeps sorted, non-overlapping half-open intervals and
//! implements the same add/subtract/contains/intersects/iteration behaviour directly.

use crate::gpu::common::types::VAddr;

/// `VideoCore::RangeSet`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RangeSet {
    ranges: Vec<(VAddr, VAddr)>,
}

impl RangeSet {
    /// `RangeSet::Add`.
    pub fn add(&mut self, base_address: VAddr, size: usize) {
        if size == 0 {
            return;
        }
        let end_address = base_address + size;
        let mut ranges = std::mem::take(&mut self.ranges);
        ranges.push((base_address, end_address));
        ranges.sort_unstable();
        let mut merged: Vec<(VAddr, VAddr)> = Vec::with_capacity(ranges.len());
        for (start, end) in ranges {
            if let Some(last) = merged.last_mut() {
                if start <= last.1 {
                    last.1 = last.1.max(end);
                    continue;
                }
            }
            merged.push((start, end));
        }
        self.ranges = merged;
    }

    /// `RangeSet::Subtract`.
    pub fn subtract(&mut self, base_address: VAddr, size: usize) {
        if size == 0 {
            return;
        }
        let end_address = base_address + size;
        let mut result = Vec::with_capacity(self.ranges.len() + 1);
        for (start, end) in self.ranges.drain(..) {
            if end <= base_address || start >= end_address {
                result.push((start, end));
                continue;
            }
            if start < base_address {
                result.push((start, base_address));
            }
            if end > end_address {
                result.push((end_address, end));
            }
        }
        self.ranges = result;
    }

    /// `RangeSet::Clear`.
    pub fn clear(&mut self) {
        self.ranges.clear();
    }

    /// `RangeSet::Contains`.
    pub fn contains(&self, base_address: VAddr, size: usize) -> bool {
        let end_address = base_address + size;
        self.ranges
            .iter()
            .any(|&(start, end)| start <= base_address && end_address <= end)
    }

    /// `RangeSet::Intersects`.
    pub fn intersects(&self, base_address: VAddr, size: usize) -> bool {
        let end_address = base_address + size;
        self.ranges
            .iter()
            .any(|&(start, end)| start < end_address && base_address < end)
    }

    /// `RangeSet::ForEach`.
    pub fn for_each(&self, mut func: impl FnMut(VAddr, VAddr)) {
        for &(start, end) in &self.ranges {
            func(start, end);
        }
    }

    /// `RangeSet::ForEachInRange`.
    pub fn for_each_in_range(
        &self,
        base_address: VAddr,
        size: usize,
        mut func: impl FnMut(VAddr, VAddr),
    ) {
        let end_address = base_address + size;
        for &(start, end) in &self.ranges {
            if end <= base_address || start >= end_address {
                continue;
            }
            func(start.max(base_address), end.min(end_address));
        }
    }

    /// `RangeSet::ForEachNotInRange`.
    pub fn for_each_not_in_range(
        &self,
        base_address: VAddr,
        size: usize,
        mut func: impl FnMut(VAddr, usize),
    ) {
        let end_address = base_address + size;
        let mut cursor = base_address;
        self.for_each_in_range(base_address, size, |start, end| {
            if start > cursor {
                func(cursor, start - cursor);
            }
            cursor = end;
        });
        if cursor != end_address {
            func(cursor, end_address - cursor);
        }
    }
}

/// `VideoCore::RangeMap`.
#[derive(Debug, Clone, Default)]
pub struct RangeMap<T> {
    ranges: Vec<(VAddr, VAddr, T)>,
}

impl<T: Clone> RangeMap<T> {
    /// `RangeMap::Add`: later writes replace the covered interval.
    pub fn add(&mut self, base_address: VAddr, size: usize, value: T) {
        if size == 0 {
            return;
        }
        let end_address = base_address + size;
        let mut result: Vec<(VAddr, VAddr, T)> = Vec::with_capacity(self.ranges.len() + 1);
        for (start, end, old) in self.ranges.drain(..) {
            if end <= base_address || start >= end_address {
                result.push((start, end, old));
                continue;
            }
            if start < base_address {
                result.push((start, base_address, old.clone()));
            }
            if end > end_address {
                result.push((end_address, end, old));
            }
        }
        result.push((base_address, end_address, value));
        result.sort_by_key(|(start, _, _)| *start);
        self.ranges = result;
    }

    /// `RangeMap::Subtract`.
    pub fn subtract(&mut self, base_address: VAddr, size: usize) {
        if size == 0 {
            return;
        }
        let end_address = base_address + size;
        let mut result = Vec::with_capacity(self.ranges.len() + 1);
        for (start, end, value) in self.ranges.drain(..) {
            if end <= base_address || start >= end_address {
                result.push((start, end, value));
                continue;
            }
            if start < base_address {
                result.push((start, base_address, value.clone()));
            }
            if end > end_address {
                result.push((end_address, end, value));
            }
        }
        self.ranges = result;
    }

    /// `RangeMap::Clear`.
    pub fn clear(&mut self) {
        self.ranges.clear();
    }

    /// `RangeMap::Contains`.
    pub fn contains(&self, base_address: VAddr, size: usize) -> bool {
        let end_address = base_address + size;
        self.ranges
            .iter()
            .any(|(start, end, _)| *start <= base_address && end_address <= *end)
    }

    /// `RangeMap::Intersects`.
    pub fn intersects(&self, base_address: VAddr, size: usize) -> bool {
        let end_address = base_address + size;
        self.ranges
            .iter()
            .any(|(start, end, _)| *start < end_address && base_address < *end)
    }

    /// `RangeMap::ForEach`.
    pub fn for_each(&self, mut func: impl FnMut(VAddr, VAddr, &T)) {
        for (start, end, value) in &self.ranges {
            func(*start, *end, value);
        }
    }

    /// `RangeMap::ForEachInRange`.
    pub fn for_each_in_range(
        &self,
        base_address: VAddr,
        size: usize,
        mut func: impl FnMut(VAddr, VAddr, &T),
    ) {
        let end_address = base_address + size;
        for (start, end, value) in &self.ranges {
            if *end <= base_address || *start >= end_address {
                continue;
            }
            func((*start).max(base_address), (*end).min(end_address), value);
        }
    }

    /// `RangeMap::ForEachNotInRange`.
    pub fn for_each_not_in_range(
        &self,
        base_address: VAddr,
        size: usize,
        mut func: impl FnMut(VAddr, usize),
    ) {
        let end_address = base_address + size;
        let mut cursor = base_address;
        self.for_each_in_range(base_address, size, |start, end, _| {
            if start > cursor {
                func(cursor, start - cursor);
            }
            cursor = end;
        });
        if cursor != end_address {
            func(cursor, end_address - cursor);
        }
    }
}

/// `VideoCore::SplitRangeMap`. The split map differs from `RangeMap` only in how Boost keeps
/// adjacent segments; the port keeps the same sorted-segment representation and API.
#[derive(Debug, Clone, Default)]
pub struct SplitRangeMap<T> {
    inner: RangeMap<T>,
}

impl<T: Clone> SplitRangeMap<T> {
    pub fn add(&mut self, base_address: VAddr, size: usize, value: T) {
        self.inner.add(base_address, size, value);
    }

    pub fn subtract(&mut self, base_address: VAddr, size: usize) {
        self.inner.subtract(base_address, size);
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn contains(&self, base_address: VAddr, size: usize) -> bool {
        self.inner.contains(base_address, size)
    }

    pub fn intersects(&self, base_address: VAddr, size: usize) -> bool {
        self.inner.intersects(base_address, size)
    }

    pub fn for_each(&self, func: impl FnMut(VAddr, VAddr, &T)) {
        self.inner.for_each(func);
    }

    pub fn for_each_in_range(
        &self,
        base_address: VAddr,
        size: usize,
        func: impl FnMut(VAddr, VAddr, &T),
    ) {
        self.inner.for_each_in_range(base_address, size, func);
    }

    pub fn for_each_not_in_range(
        &self,
        base_address: VAddr,
        size: usize,
        func: impl FnMut(VAddr, usize),
    ) {
        self.inner.for_each_not_in_range(base_address, size, func);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adding_merges_overlapping_and_touching_ranges() {
        let mut set = RangeSet::default();
        set.add(0, 4);
        set.add(2, 6);
        set.add(10, 2);
        set.add(12, 2);
        let mut ranges = Vec::new();
        set.for_each(|start, end| ranges.push((start, end)));
        assert_eq!(ranges, vec![(0, 8), (10, 14)]);
        assert!(set.contains(1, 6));
        assert!(!set.contains(1, 8));
        assert!(set.intersects(7, 4));
        assert!(!set.intersects(8, 2));
    }

    #[test]
    fn subtracting_splits_a_range() {
        let mut set = RangeSet::default();
        set.add(0, 10);
        set.subtract(3, 4);
        let mut ranges = Vec::new();
        set.for_each(|start, end| ranges.push((start, end)));
        assert_eq!(ranges, vec![(0, 3), (7, 10)]);
        set.clear();
        assert!(!set.intersects(0, 1));
    }

    #[test]
    fn the_gaps_of_a_range_are_reported() {
        let mut set = RangeSet::default();
        set.add(2, 2);
        set.add(6, 2);
        let mut gaps = Vec::new();
        set.for_each_not_in_range(0, 10, |start, size| gaps.push((start, size)));
        assert_eq!(gaps, vec![(0, 2), (4, 2), (8, 2)]);
    }

    #[test]
    fn a_range_map_replaces_and_splits_values() {
        let mut map = RangeMap::default();
        map.add(0, 10, 1u32);
        map.add(4, 2, 2u32);
        let mut entries = Vec::new();
        map.for_each(|start, end, value| entries.push((start, end, *value)));
        assert_eq!(entries, vec![(0, 4, 1), (4, 6, 2), (6, 10, 1)]);
        assert!(map.contains(4, 2));
        assert!(!map.contains(3, 2));
        map.subtract(0, 5);
        assert!(!map.intersects(0, 5));
        assert!(map.intersects(5, 1));
    }

    #[test]
    fn a_split_range_map_has_the_same_behaviour() {
        let mut map = SplitRangeMap::default();
        map.add(0, 4, 7u32);
        assert!(map.contains(0, 4));
        let mut values = Vec::new();
        map.for_each(|_, _, value| values.push(*value));
        assert_eq!(values, vec![7]);
    }
}
