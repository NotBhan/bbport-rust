//! CPU/GPU page-region modification tracking, ported from
//! `shadps4/video_core/buffer_cache/region_manager.h`.

use std::sync::Mutex;

use crate::gpu::video_core::buffer_cache::region_definitions::{
    RegionBits, Type, NUM_PAGES_PER_REGION, TRACKER_BYTES_PER_PAGE,
};

/// `RegionManager`.
#[derive(Debug)]
pub struct RegionManager {
    pub tracker_id: u64,
    pub cpu_addr: u64,
    pub cpu: RegionBits,
    pub gpu: RegionBits,
    pub writeable: RegionBits,
    pub readable: RegionBits,
    pub lock: Mutex<()>,
    pub write_faults: u64,
}

impl Default for RegionManager {
    fn default() -> Self {
        Self {
            tracker_id: 0,
            cpu_addr: 0,
            cpu: RegionBits::new(),
            gpu: RegionBits::new(),
            writeable: RegionBits::new(),
            readable: RegionBits::new(),
            lock: Mutex::new(()),
            write_faults: 0,
        }
    }
}

impl RegionManager {
    pub fn new(tracker_id: u64, cpu_addr: u64) -> Self {
        Self {
            tracker_id,
            cpu_addr,
            ..Self::default()
        }
    }

    pub fn set_cpu_address(&mut self, cpu_addr: u64) {
        self.cpu_addr = cpu_addr;
    }

    pub fn cpu_addr(&self) -> u64 {
        self.cpu_addr
    }

    pub fn sanitize_address(address: i64) -> usize {
        address.max(0) as usize
    }

    pub fn region_bits(&self, kind: Type) -> &RegionBits {
        match kind {
            Type::Cpu => &self.cpu,
            Type::Gpu => &self.gpu,
        }
    }

    pub fn region_bits_mut(&mut self, kind: Type) -> &mut RegionBits {
        match kind {
            Type::Cpu => &mut self.cpu,
            Type::Gpu => &mut self.gpu,
        }
    }

    fn page_range(&self, dirty_addr: u64, size: u64) -> Option<(usize, usize)> {
        let offset = dirty_addr.wrapping_sub(self.cpu_addr);
        let start = Self::sanitize_address(offset as i64) / TRACKER_BYTES_PER_PAGE as usize;
        let end_offset = offset.saturating_add(size);
        let end = (end_offset as usize).div_ceil(TRACKER_BYTES_PER_PAGE as usize);
        (start < NUM_PAGES_PER_REGION as usize && end > start).then_some((start, end))
    }

    pub fn change_region_state(&mut self, kind: Type, enable: bool, dirty_addr: u64, size: u64) {
        let Some((start, end)) = self.page_range(dirty_addr, size) else {
            return;
        };
        if kind == Type::Cpu && enable {
            self.write_faults += 1;
        }
        let bits = self.region_bits_mut(kind);
        if enable {
            bits.set_range(start, end);
        } else {
            bits.unset_range(start, end);
        }
    }

    pub fn is_region_modified(&self, kind: Type, offset: u64, size: u64) -> bool {
        let Some((start, end)) = self.page_range(self.cpu_addr + offset, size) else {
            return false;
        };
        (start..end).any(|page| self.region_bits(kind).get(page))
    }

    /// `ExtendWriteFault`; pages without GPU data are made writable again.
    pub fn extend_write_fault(&mut self, window_addr: u64, size: u64) {
        let Some((start, end)) = self.page_range(window_addr, size) else {
            return;
        };
        for page in start..end {
            if !self.gpu.get(page) {
                self.writeable.set(page);
            }
        }
    }

    pub fn for_each_modified_range(
        &self,
        kind: Type,
        offset: u64,
        size: u64,
        mut func: impl FnMut(u64, u64),
    ) {
        let Some((start, end)) = self.page_range(self.cpu_addr + offset, size) else {
            return;
        };
        for page in start..end {
            if self.region_bits(kind).get(page) {
                func(
                    self.cpu_addr + page as u64 * TRACKER_BYTES_PER_PAGE,
                    TRACKER_BYTES_PER_PAGE,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_states_change_per_page() {
        let mut region = RegionManager::new(1, 0x10_0000);
        region.change_region_state(Type::Cpu, true, 0x10_0000, 0x2000);
        assert!(region.is_region_modified(Type::Cpu, 0, 0x1000));
        assert!(region.is_region_modified(Type::Cpu, 0x1000, 0x1000));
        assert!(!region.is_region_modified(Type::Cpu, 0x3000, 0x1000));
        region.change_region_state(Type::Cpu, false, 0x10_0000, 0x2000);
        assert!(!region.is_region_modified(Type::Cpu, 0, 0x1000));
        assert_eq!(region.write_faults, 1);
        let mut ranges = Vec::new();
        region.change_region_state(Type::Gpu, true, 0x10_0000, 0x1000);
        region.for_each_modified_range(Type::Gpu, 0, 0x2000, |start, size| {
            ranges.push((start, size))
        });
        assert_eq!(ranges, vec![(0x10_0000, 4096)]);
    }
}
