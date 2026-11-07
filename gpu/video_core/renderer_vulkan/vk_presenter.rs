//! Swapchain presentation types, ported from
//! `shadps4/video_core/renderer_vulkan/vk_presenter.h`.

use crate::gpu::video_core::renderer_vulkan::vk_frame_generation::FrameGenerator;

pub const MAX_GENERATED: usize = FrameGenerator::MAX_GENERATED as usize;

/// `Frame::Generated`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Generated {
    pub allocation: u64,
    pub image: u64,
    pub view: u64,
}

/// `Frame`.
#[derive(Debug, Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub allocation: u64,
    pub image: u64,
    pub image_view: u64,
    pub present_done: u64,
    pub ready_semaphore: u64,
    pub ready_tick: u64,
    pub is_hdr: bool,
    pub id: u8,
    pub generated: [Generated; MAX_GENERATED],
    pub generated_count: u32,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            allocation: 0,
            image: 0,
            image_view: 0,
            present_done: 0,
            ready_semaphore: 0,
            ready_tick: 0,
            is_hdr: false,
            id: 0,
            generated: [Generated::default(); MAX_GENERATED],
            generated_count: 0,
        }
    }
}

/// `SchedulerType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerType {
    Draw,
    Present,
    CpuFlip,
}

/// `Presenter`; the Vulkan objects are supplied by the renderer, the counters live here.
#[derive(Debug, Default)]
pub struct Presenter {
    pub expected_ratio: f32,
    pub expected_frame_width: u32,
    pub expected_frame_height: u32,
    pub frame_gen_limit: u32,
    pub game_frame_ms: f64,
    pub last_submit_frame: usize,
    pub present_frames: Vec<Frame>,
    pub vo_buffers_addr: Vec<u64>,
}

impl Presenter {
    pub fn new() -> Self {
        Self {
            expected_ratio: 1920.0 / 1080.0,
            expected_frame_width: 1920,
            expected_frame_height: 1080,
            game_frame_ms: 16.7,
            ..Self::default()
        }
    }

    pub fn set_hdr(&mut self, supported: bool, enable: bool) -> bool {
        if !supported {
            return false;
        }
        enable
    }

    pub fn is_video_out_surface(&self, address: u64) -> bool {
        self.vo_buffers_addr.contains(&address)
    }

    pub fn register_video_out_surface(&mut self, address: u64) -> u64 {
        if !self.vo_buffers_addr.contains(&address) {
            self.vo_buffers_addr.push(address);
        }
        address
    }

    pub fn set_expected_game_size(&mut self, width: u32, height: u32) {
        if width != 0 && height != 0 {
            self.expected_frame_width = width;
            self.expected_frame_height = height;
            self.expected_ratio = width as f32 / height as f32;
        }
    }

    /// `PrepareFrame`; reuses a present frame of the same size.
    pub fn prepare_frame(&mut self, width: u32, height: u32) -> usize {
        if let Some(index) = self
            .present_frames
            .iter()
            .position(|frame| frame.width == width && frame.height == height)
        {
            return index;
        }
        self.present_frames.push(Frame {
            width,
            height,
            ..Frame::default()
        });
        self.present_frames.len() - 1
    }

    /// `PrepareBlankFrame`.
    pub fn prepare_blank_frame(&mut self) -> Frame {
        Frame {
            width: self.expected_frame_width,
            height: self.expected_frame_height,
            ..Frame::default()
        }
    }

    /// `UpdateFrameGenLimit`.
    pub fn update_frame_gen_limit(&mut self, wanted: bool, present_ms: f64) {
        if !wanted {
            self.frame_gen_limit = 0;
            return;
        }
        let budget = self.game_frame_ms - present_ms;
        self.frame_gen_limit = if budget <= 0.0 {
            0
        } else {
            (budget / present_ms.max(0.001)).floor().clamp(0.0, 3.0) as u32
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presenter_frames_are_reused_and_limits_paced() {
        let mut presenter = Presenter::new();
        let first = presenter.prepare_frame(1920, 1080);
        let second = presenter.prepare_frame(1920, 1080);
        assert_eq!(first, second);
        assert_eq!(presenter.register_video_out_surface(0x1000), 0x1000);
        assert!(presenter.is_video_out_surface(0x1000));
        presenter.set_expected_game_size(1280, 720);
        assert_eq!(presenter.expected_frame_width, 1280);
        presenter.game_frame_ms = 16.0;
        presenter.update_frame_gen_limit(true, 8.0);
        assert_eq!(presenter.frame_gen_limit, 1);
        presenter.update_frame_gen_limit(false, 0.0);
        assert_eq!(presenter.frame_gen_limit, 0);
        assert_eq!(MAX_GENERATED, 3);
    }
}

/// The presenter's frame queue (`vk_presenter.cpp`).
#[derive(Debug, Default)]
pub struct PresentQueue {
    pub free: Vec<usize>,
    pub ready: std::collections::VecDeque<usize>,
    pub presented: u64,
    pub generated_presented: u64,
}

impl PresentQueue {
    pub fn add_free(&mut self, index: usize) {
        if !self.free.contains(&index) {
            self.free.push(index);
        }
    }

    pub fn acquire(&mut self) -> Option<usize> {
        self.free.pop()
    }

    pub fn submit(&mut self, index: usize, generated: u32) {
        self.ready.push_back(index);
        self.generated_presented += u64::from(generated);
    }

    pub fn present(&mut self) -> Option<usize> {
        let index = self.ready.pop_front()?;
        self.presented += 1;
        self.add_free(index);
        Some(index)
    }
}

impl Presenter {
    /// `SetExpectedGameSize`.
    pub fn expect_size(&mut self, width: u32, height: u32) {
        self.set_expected_game_size(width, height);
    }

    /// `UpdateFrameGenLimit` from the present timing.
    pub fn update_limit(&mut self, wanted: bool, present_ms: f64) {
        self.update_frame_gen_limit(wanted, present_ms);
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn present_queue_recycles_frames() {
        let mut queue = PresentQueue::default();
        queue.add_free(0);
        assert_eq!(queue.acquire(), Some(0));
        assert_eq!(queue.acquire(), None);
        queue.submit(0, 2);
        assert_eq!(queue.present(), Some(0));
        assert_eq!(queue.presented, 1);
        assert_eq!(queue.generated_presented, 2);
        assert!(!queue.free.is_empty());
    }
}
