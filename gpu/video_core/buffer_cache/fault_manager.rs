//! Sparse buffer fault handling, ported from
//! `shadps4/video_core/buffer_cache/fault_manager.h`.

use crate::gpu::video_core::buffer_cache::buffer::{Buffer, MemoryType};
use crate::gpu::video_core::buffer_cache::range_set::RangeSet;

pub const MAX_PENDING_FAULTS: usize = 8;

/// `FaultManager`.
#[derive(Debug)]
pub struct FaultManager {
    pub scheduler_id: u64,
    pub buffer_cache_id: u64,
    pub fault_ranges: RangeSet,
    pub sparse_pagesize: u64,
    pub sparse_num_pages: u64,
    pub fault_buffer_size: u64,
    pub fault_buffer: Buffer,
    pub download_buffer: Buffer,
    pub fault_areas: [u64; MAX_PENDING_FAULTS],
    pub current_area: u32,
}

impl FaultManager {
    pub fn new(
        scheduler_id: u64,
        buffer_cache_id: u64,
        sparse_pagebits: u32,
        sparse_num_pages: u64,
    ) -> Self {
        let sparse_pagesize = 1u64 << sparse_pagebits;
        let fault_buffer_size = MAX_PENDING_FAULTS as u64 * 8;
        Self {
            scheduler_id,
            buffer_cache_id,
            fault_ranges: RangeSet::default(),
            sparse_pagesize,
            sparse_num_pages,
            fault_buffer_size,
            fault_buffer: Buffer::new(0, fault_buffer_size as usize, MemoryType::HostCached),
            download_buffer: Buffer::new(0, fault_buffer_size as usize, MemoryType::HostCached),
            fault_areas: [0; MAX_PENDING_FAULTS],
            current_area: 0,
        }
    }

    pub fn fault_buffer(&self) -> &Buffer {
        &self.fault_buffer
    }

    /// `ProcessFaultBuffer`; the buffer cache services each reported page.
    pub fn process_fault_buffer(&mut self, cache: &mut dyn FaultHandler) {
        for area in self.fault_areas {
            if area != 0 {
                cache.handle_fault(area, self.sparse_pagesize);
            }
        }
        self.fault_areas = [0; MAX_PENDING_FAULTS];
        self.current_area = 0;
    }
}

pub trait FaultHandler {
    fn handle_fault(&mut self, address: u64, page_size: u64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Cache {
        faults: Vec<(u64, u64)>,
    }

    impl FaultHandler for Cache {
        fn handle_fault(&mut self, address: u64, page_size: u64) {
            self.faults.push((address, page_size));
        }
    }

    #[test]
    fn faults_are_reported_and_cleared() {
        let mut manager = FaultManager::new(1, 2, 12, 256);
        assert_eq!(manager.sparse_pagesize, 4096);
        assert_eq!(manager.fault_buffer_size, 64);
        manager.fault_areas[0] = 0x1000;
        manager.fault_areas[1] = 0x2000;
        let mut cache = Cache::default();
        manager.process_fault_buffer(&mut cache);
        assert_eq!(cache.faults.len(), 2);
        assert_eq!(cache.faults[1], (0x2000, 4096));
        assert_eq!(manager.fault_areas, [0; MAX_PENDING_FAULTS]);
    }
}
