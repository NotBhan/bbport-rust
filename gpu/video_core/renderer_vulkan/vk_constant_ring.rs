//! Constant ring for small guest buffers, ported from
//! `shadps4/video_core/renderer_vulkan/vk_constant_ring.h`.

pub const CAPACITY: u64 = 32 << 20;
const STAMP_COUNT: usize = 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct StampEntry {
    end: u64,
    tick: u64,
}

/// `ConstantRing`.
#[derive(Debug, Clone)]
pub struct ConstantRing {
    capacity: u64,
    position: u64,
    retired: u64,
    pending_end: u64,
    pending_tick: u64,
    last_published: u64,
    stamps: [StampEntry; STAMP_COUNT],
    stamps_head: u64,
    stamps_tail: u64,
}

impl Default for ConstantRing {
    fn default() -> Self {
        Self::new(CAPACITY)
    }
}

impl ConstantRing {
    pub fn new(capacity: u64) -> Self {
        assert!(capacity > 0);
        Self {
            capacity,
            position: 0,
            retired: 0,
            pending_end: 0,
            pending_tick: 0,
            last_published: 0,
            stamps: [StampEntry::default(); STAMP_COUNT],
            stamps_head: 0,
            stamps_tail: 0,
        }
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    /// Stage A: `size` bytes at `alignment`; `None` when the ring is still in use.
    pub fn allocate(
        &mut self,
        size: u64,
        alignment: u64,
        is_free: impl Fn(u64) -> bool,
    ) -> Option<u64> {
        assert!(alignment.is_power_of_two());
        let mut at = (self.position + alignment - 1) & !(alignment - 1);
        if at % self.capacity + size > self.capacity {
            at += self.capacity - at % self.capacity;
        }
        let in_use =
            |ring: &Self| at + size > ring.capacity && at + size - ring.capacity > ring.retired;
        if in_use(self) {
            self.retire(is_free);
            if in_use(self) {
                return None;
            }
        }
        self.position = at + size;
        Some(at % self.capacity)
    }

    /// Stage A: the position the next packet's allocations end at.
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Stage B: stamps the end of a recorded packet; stamps of one tick are merged.
    pub fn stamp(&mut self, end: u64, tick: u64) {
        if end == self.pending_end {
            return;
        }
        if tick != self.pending_tick && self.pending_end != self.last_published {
            self.publish(self.pending_end, self.pending_tick);
        }
        self.pending_end = end;
        self.pending_tick = tick;
    }

    fn publish(&mut self, end: u64, tick: u64) {
        if self.stamps_head - self.stamps_tail >= STAMP_COUNT as u64 {
            return;
        }
        self.stamps[(self.stamps_head as usize) % STAMP_COUNT] = StampEntry { end, tick };
        self.stamps_head += 1;
        self.last_published = end;
    }

    /// Retires stamps whose submission tick the GPU has passed.
    pub fn retire(&mut self, is_free: impl Fn(u64) -> bool) {
        let mut at = self.stamps_tail;
        while at < self.stamps_head && is_free(self.stamps[(at as usize) % STAMP_COUNT].tick) {
            self.retired = self.stamps[(at as usize) % STAMP_COUNT].end;
            at += 1;
        }
        self.stamps_tail = at;
    }

    pub fn retired(&self) -> u64 {
        self.retired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocations_advance_and_wrap() {
        let mut ring = ConstantRing::new(1024);
        assert_eq!(ring.allocate(512, 64, |_| true), Some(0));
        assert_eq!(ring.allocate(512, 64, |_| true), Some(512));
        assert_eq!(ring.position(), 1024);
        assert_eq!(ring.allocate(512, 64, |_| true), None);
    }

    #[test]
    fn stamps_retire_when_their_tick_is_free() {
        let mut ring = ConstantRing::new(1024);
        assert_eq!(ring.allocate(512, 1, |_| true), Some(0));
        assert_eq!(ring.allocate(512, 1, |_| true), Some(512));
        ring.stamp(1024, 4);
        ring.stamp(1536, 4);
        ring.stamp(2048, 5);
        assert_eq!(ring.retired(), 0);
        ring.retire(|tick| tick < 5);
        assert_eq!(ring.retired(), 1536);
        assert_eq!(ring.allocate(512, 1, |tick| tick < 5), Some(0));
        assert_eq!(ring.retired(), 1536);
    }
}
