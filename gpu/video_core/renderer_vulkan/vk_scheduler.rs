//! Command scheduler, ported from
//! `shadps4/video_core/renderer_vulkan/vk_scheduler.h`.

use std::collections::VecDeque;

/// A scheduled submission.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Submission {
    pub tick: u64,
    pub command_count: u32,
}

/// `Scheduler`; the Vulkan queue lives in the renderer.
#[derive(Debug, Default)]
pub struct Scheduler {
    pub current_tick: u64,
    pub known_gpu_tick: u64,
    pub pending: VecDeque<Submission>,
    pub submitted: u64,
    pub free_tick: u64,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            current_tick: 1,
            ..Self::default()
        }
    }

    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    pub fn next_tick(&mut self) -> u64 {
        let tick = self.current_tick;
        self.current_tick += 1;
        tick
    }

    pub fn is_free(&self, tick: u64) -> bool {
        self.known_gpu_tick >= tick
    }

    pub fn submit(&mut self, command_count: u32) {
        let tick = self.next_tick();
        self.pending.push_back(Submission {
            tick,
            command_count,
        });
        self.submitted += 1;
    }

    /// `Flush`; moves pending submissions to the GPU tick and returns their count.
    pub fn flush(&mut self) -> u32 {
        let count = self.pending.len() as u32;
        if let Some(last) = self.pending.back() {
            self.known_gpu_tick = self.known_gpu_tick.max(last.tick);
            self.free_tick = last.tick;
        }
        self.pending.clear();
        count
    }

    pub fn wait(&mut self, tick: u64) {
        if self.known_gpu_tick < tick {
            self.known_gpu_tick = tick;
        }
    }

    pub fn finish(&mut self) {
        self.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_ticks_and_flushes_submissions() {
        let mut scheduler = Scheduler::new();
        assert_eq!(scheduler.current_tick(), 1);
        let tick = scheduler.next_tick();
        assert_eq!(tick, 1);
        scheduler.submit(4);
        scheduler.submit(2);
        assert_eq!(scheduler.pending.len(), 2);
        assert_eq!(scheduler.flush(), 2);
        assert!(scheduler.is_free(tick));
        scheduler.wait(10);
        assert!(scheduler.is_free(10));
        assert!(!scheduler.is_free(11));
        scheduler.finish();
    }
}
