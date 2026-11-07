//! Indirect dispatch guard, ported from
//! `shadps4/video_core/renderer_vulkan/vk_indirect_guard.h`.

use std::sync::atomic::{AtomicU32, Ordering};

pub const NUM_SLOTS: u32 = 1024;
pub const MAX_GROUPS: u32 = 4_000_000;

/// `IndirectGuard`; the Vulkan objects live in the renderer, the slot bookkeeping is here.
#[derive(Debug)]
pub struct IndirectGuard {
    next_slot: AtomicU32,
    reported: AtomicU32,
    pub max_count: [u32; 3],
    pub max_total: u32,
}

impl Default for IndirectGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl IndirectGuard {
    pub fn new() -> Self {
        Self {
            next_slot: AtomicU32::new(0),
            reported: AtomicU32::new(0),
            max_count: [0; 3],
            max_total: 0,
        }
    }

    /// `Enabled`; `BB_INDIRECT_GUARD=0` disables the guard.
    pub fn enabled() -> bool {
        std::env::var("BB_INDIRECT_GUARD")
            .map(|value| value != "0")
            .unwrap_or(true)
    }

    /// `CheckArgs`; returns the slot a dispatch reads its counts from, or `None` when the
    /// counts are implausible and the dispatch must be skipped.
    pub fn check_args(&self, count: [u32; 3]) -> Option<u32> {
        let total = count[0].saturating_add(count[1]).saturating_add(count[2]);
        let limit = |index: usize| {
            if self.max_count[index] == 0 {
                MAX_GROUPS
            } else {
                self.max_count[index]
            }
        };
        if (0..3).any(|index| count[index] > limit(index)) || total > self.max_total.max(MAX_GROUPS)
        {
            self.reported.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        Some(self.next_slot.fetch_add(1, Ordering::Relaxed) % NUM_SLOTS)
    }

    pub fn reported(&self) -> u32 {
        self.reported.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implausible_counts_are_rejected_and_counted() {
        let guard = IndirectGuard::new();
        assert_eq!(guard.check_args([1, 2, 3]), Some(0));
        assert_eq!(guard.check_args([1, 2, 3]), Some(1));
        assert_eq!(guard.check_args([u32::MAX, 1, 1]), None);
        assert_eq!(guard.reported(), 1);
        assert_eq!(NUM_SLOTS, 1024);
        assert!(IndirectGuard::enabled());
    }
}
