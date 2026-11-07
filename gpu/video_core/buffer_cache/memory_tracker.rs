//! L2 page-table memory tracker, ported from
//! `shadps4/video_core/buffer_cache/memory_tracker.h`.

use std::collections::HashMap;

use crate::gpu::video_core::buffer_cache::region_definitions::{
    Type, TRACKER_HIGHER_PAGE_BITS, TRACKER_HIGHER_PAGE_MASK, TRACKER_HIGHER_PAGE_SIZE,
};
use crate::gpu::video_core::buffer_cache::region_manager::RegionManager;

pub const MAX_CPU_PAGE_BITS: u32 = 40;
pub const NUM_HIGH_PAGES: usize = 1 << (MAX_CPU_PAGE_BITS - TRACKER_HIGHER_PAGE_BITS);
pub const MANAGER_POOL_SIZE: usize = 32;

/// `MemoryTracker`.
#[derive(Debug, Default)]
pub struct MemoryTracker {
    pub top_tier: HashMap<usize, RegionManager>,
    pub tracker_id: u64,
}

impl MemoryTracker {
    pub fn new(tracker_id: u64) -> Self {
        Self {
            tracker_id,
            ..Self::default()
        }
    }

    pub fn is_region_cpu_modified(&mut self, address: u64, size: u64) -> bool {
        let mut found = false;
        self.iterate(address, size, true, |manager, offset, size| {
            found |= manager.is_region_modified(Type::Cpu, offset, size);
        });
        found
    }

    pub fn is_region_gpu_modified(&mut self, address: u64, size: u64) -> bool {
        let mut found = false;
        self.iterate(address, size, false, |manager, offset, size| {
            found |= manager.is_region_modified(Type::Gpu, offset, size);
        });
        found
    }

    pub fn mark_region_as_cpu_modified(&mut self, address: u64, size: u64) {
        self.iterate(address, size, false, |manager, offset, size| {
            manager.change_region_state(Type::Cpu, true, manager.cpu_addr + offset, size);
        });
    }

    pub fn unmark_region_as_gpu_modified(&mut self, address: u64, size: u64) {
        self.iterate(address, size, false, |manager, offset, size| {
            manager.change_region_state(Type::Gpu, false, manager.cpu_addr + offset, size);
        });
    }

    pub fn mark_region_as_gpu_modified(&mut self, address: u64, size: u64) {
        self.iterate(address, size, false, |manager, offset, size| {
            manager.change_region_state(Type::Gpu, true, manager.cpu_addr + offset, size);
        });
    }

    pub fn extend_write_fault(&mut self, address: u64, size: u64) {
        self.iterate(address, size, false, |manager, offset, size| {
            manager.extend_write_fault(manager.cpu_addr + offset, size);
        });
    }

    /// `ForEachDownloadRange`.
    pub fn for_each_download_range(
        &mut self,
        address: u64,
        size: u64,
        mut func: impl FnMut(u64, u64),
    ) {
        self.iterate(address, size, false, |manager, offset, size| {
            manager.for_each_modified_range(Type::Gpu, offset, size, &mut func);
        });
    }

    fn iterate(
        &mut self,
        address: u64,
        size: u64,
        create: bool,
        mut func: impl FnMut(&mut RegionManager, u64, u64),
    ) {
        let mut remaining = size;
        let mut page_index = (address >> TRACKER_HIGHER_PAGE_BITS) as usize;
        let mut page_offset = address & TRACKER_HIGHER_PAGE_MASK;
        while remaining > 0 {
            let copy_amount = (TRACKER_HIGHER_PAGE_SIZE - page_offset).min(remaining);
            if !self.top_tier.contains_key(&page_index) && create {
                self.create_region(page_index);
            }
            if let Some(manager) = self.top_tier.get_mut(&page_index) {
                func(manager, page_offset, copy_amount);
            }
            page_index += 1;
            page_offset = 0;
            remaining -= copy_amount;
        }
    }

    fn create_region(&mut self, page_index: usize) {
        let base = (page_index as u64) << TRACKER_HIGHER_PAGE_BITS;
        self.top_tier
            .insert(page_index, RegionManager::new(self.tracker_id, base));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_tracker_marks_both_sides() {
        let mut tracker = MemoryTracker::new(1);
        assert!(!tracker.is_region_cpu_modified(0x20_0000, 0x1000));
        tracker.mark_region_as_cpu_modified(0x20_0000, 0x2000);
        assert!(tracker.is_region_cpu_modified(0x20_0000, 0x1000));
        assert!(!tracker.is_region_cpu_modified(0x20_4000, 0x1000));
        tracker.mark_region_as_gpu_modified(0x20_0000, 0x1000);
        assert!(tracker.is_region_gpu_modified(0x20_0000, 0x1000));
        tracker.unmark_region_as_gpu_modified(0x20_0000, 0x1000);
        assert!(!tracker.is_region_gpu_modified(0x20_0000, 0x1000));
        let mut ranges = 0;
        tracker.for_each_download_range(0x20_0000, 0x1000, |_, _| ranges += 1);
        assert_eq!(ranges, 0);
        assert_eq!(MANAGER_POOL_SIZE, 32);
    }
}
