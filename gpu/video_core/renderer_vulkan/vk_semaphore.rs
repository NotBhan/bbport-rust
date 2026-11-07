//! Timeline semaphore, ported from
//! `shadps4/video_core/renderer_vulkan/vk_semaphore.h`.

use std::sync::atomic::{AtomicU64, Ordering};

/// `Semaphore`.
#[derive(Debug)]
pub struct Semaphore {
    pub handle: u64,
    gpu_tick: AtomicU64,
    current_tick: AtomicU64,
}

impl Semaphore {
    pub fn new(handle: u64) -> Self {
        Self {
            handle,
            gpu_tick: AtomicU64::new(0),
            current_tick: AtomicU64::new(1),
        }
    }

    pub fn current_tick(&self) -> u64 {
        self.current_tick.load(Ordering::Acquire)
    }

    pub fn known_gpu_tick(&self) -> u64 {
        self.gpu_tick.load(Ordering::Acquire)
    }

    pub fn is_free(&self, tick: u64) -> bool {
        self.known_gpu_tick() >= tick
    }

    pub fn next_tick(&self) -> u64 {
        self.current_tick.fetch_add(1, Ordering::Release)
    }

    pub fn handle(&self) -> u64 {
        self.handle
    }

    /// `Refresh`; the scheduler records the GPU's completed tick.
    pub fn refresh(&self, completed_tick: u64) {
        let mut current = self.gpu_tick.load(Ordering::Acquire);
        while current < completed_tick {
            match self.gpu_tick.compare_exchange_weak(
                current,
                completed_tick,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }

    /// `Refresh` from a semaphore counter read (the Vulkan device in the renderer).
    pub fn refresh_from(&self, counter: impl Fn() -> u64) {
        self.refresh(counter());
    }

    /// `Wait`; refresh the known tick, then wait for the GPU to reach `tick`.
    pub fn wait(&self, tick: u64) {
        while !self.is_free(tick) {
            std::thread::yield_now();
        }
    }

    /// `Wait` with an explicit counter read, as `vkWaitSemaphores` does.
    pub fn wait_with(&self, tick: u64, counter: impl Fn() -> u64) {
        if self.is_free(tick) {
            return;
        }
        self.refresh_from(&counter);
        while !self.is_free(tick) {
            self.refresh_from(&counter);
            std::thread::yield_now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_advance_and_free_when_the_gpu_reports() {
        let semaphore = Semaphore::new(3);
        assert_eq!(semaphore.handle(), 3);
        assert_eq!(semaphore.next_tick(), 1);
        assert_eq!(semaphore.next_tick(), 2);
        assert!(!semaphore.is_free(2));
        semaphore.refresh(1);
        assert!(semaphore.is_free(1));
        assert!(!semaphore.is_free(2));
        semaphore.refresh(2);
        semaphore.wait(2);
        assert_eq!(semaphore.known_gpu_tick(), 2);
    }
}
