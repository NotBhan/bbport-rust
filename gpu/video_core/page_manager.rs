//! GPU page watcher manager, ported from `shadps4/video_core/page_manager.h`.

use crate::gpu::video_core::buffer_cache::region_definitions::{
    RegionBits, NUM_PAGES_PER_REGION, TRACKER_BYTES_PER_PAGE,
};

pub const PM_PAGE_BITS: u32 = 12;
pub const PM_PAGE_SIZE: u64 = TRACKER_BYTES_PER_PAGE;
pub const PAGES_PER_LOCK: u64 = NUM_PAGES_PER_REGION;

/// `PageManager`; the platform watcher implementation owns the registered ranges.
#[derive(Debug, Default)]
pub struct PageManager {
    pub registered: Vec<(u64, u64)>,
}

impl PageManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// `OnGpuMap`.
    pub fn on_gpu_map(&mut self, address: u64, size: u64) {
        self.registered.push((Self::page_addr(address), size));
    }

    /// `OnGpuUnmap`; drops every registered range touching the range.
    pub fn on_gpu_unmap(&mut self, address: u64, size: u64) {
        let start = Self::page_addr(address);
        let end = address.saturating_add(size);
        self.registered.retain(|&(range_start, range_size)| {
            range_start + range_size <= start || range_start >= end
        });
    }

    /// `UpdatePageWatchers`; the watcher backend toggles tracking for the touched pages.
    pub fn update_page_watchers(
        &self,
        addr: u64,
        size: u64,
        track: bool,
        watcher: &mut dyn PageWatcher,
    ) {
        let mut page = Self::page_addr(addr);
        let end = addr.saturating_add(size);
        while page < end {
            watcher.set_watch(page, track);
            page += PM_PAGE_SIZE;
        }
    }

    /// `UpdatePageWatchersForRegion`.
    pub fn update_page_watchers_for_region(
        &self,
        base_addr: u64,
        mask: &RegionBits,
        track: bool,
        watcher: &mut dyn PageWatcher,
    ) {
        for index in 0..mask.size() {
            if mask.get(index) {
                watcher.set_watch(base_addr + index as u64 * PM_PAGE_SIZE, track);
            }
        }
    }

    /// `GetPageAddr`.
    pub const fn page_addr(addr: u64) -> u64 {
        addr & !(PM_PAGE_SIZE - 1)
    }

    /// `GetNextPageAddr`.
    pub const fn next_page_addr(addr: u64) -> u64 {
        (addr + 1 + PM_PAGE_SIZE - 1) & !(PM_PAGE_SIZE - 1)
    }
}

pub trait PageWatcher {
    fn set_watch(&mut self, page: u64, track: bool);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Watcher(Vec<(u64, bool)>);

    impl PageWatcher for Watcher {
        fn set_watch(&mut self, page: u64, track: bool) {
            self.0.push((page, track));
        }
    }

    #[test]
    fn pages_align_and_watchers_follow_the_range() {
        assert_eq!(PageManager::page_addr(0x1234), 0x1000);
        assert_eq!(PageManager::next_page_addr(0x1234), 0x2000);
        let manager = PageManager::new();
        let mut watcher = Watcher::default();
        manager.update_page_watchers(0x1000, 0x2000, true, &mut watcher);
        assert_eq!(watcher.0, vec![(0x1000, true), (0x2000, true)]);
        let mut bits = RegionBits::default();
        bits.set(1);
        let mut watcher = Watcher::default();
        manager.update_page_watchers_for_region(0x10000, &bits, false, &mut watcher);
        assert_eq!(watcher.0, vec![(0x11000, false)]);
    }
}
