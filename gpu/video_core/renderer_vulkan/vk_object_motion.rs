//! Object motion vectors, ported from
//! `shadps4/video_core/renderer_vulkan/vk_object_motion.h`.

use std::collections::VecDeque;

use crate::gpu::video_core::renderer_vulkan::motion_history::{
    Draw, GateKey, History, IndexRangeCache, IndexRangeKey, Result_,
};

pub const MAX_TARGETS: usize = 4;
pub const FRAME_SLOTS: usize = 4;
pub const PARAMS_PER_FRAME: u32 = 8192;
pub const POSITIONS_PER_FRAME: u32 = 4 << 20;

pub type DrawInfo = Draw;

/// One render-target-sized motion image.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Target {
    pub image: u64,
    pub view: u64,
    pub width: u32,
    pub height: u32,
    pub written: bool,
    pub layout: i32,
    pub used_frame: u64,
}

/// `ObjectMotion`.
#[derive(Debug)]
pub struct ObjectMotion {
    pub enabled: bool,
    pub history: History,
    pub index_ranges: IndexRangeCache,
    pub frame: u64,
    pub params_used: u32,
    pub params_ticks: [u64; FRAME_SLOTS],
    pub targets: VecDeque<Target>,
    pub read_target: Option<Target>,
}

impl ObjectMotion {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            history: History::new(POSITIONS_PER_FRAME),
            index_ranges: IndexRangeCache::default(),
            frame: 0,
            params_used: 0,
            params_ticks: [0; FRAME_SLOTS],
            targets: VecDeque::new(),
            read_target: None,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// `PrepareDraw`; element 0 is reserved for "motion off".
    pub fn prepare_draw(&mut self, _draw: &Draw) -> u32 {
        if !self.enabled {
            return 0;
        }
        self.params_used = (self.params_used + 1).min(PARAMS_PER_FRAME - 1);
        self.params_used
    }

    /// `Moving`.
    pub fn moving(&mut self, key: GateKey, palette: u64) -> bool {
        self.history.moving(key, palette)
    }

    /// `IndexRange`.
    pub fn index_range(&mut self, key: &IndexRangeKey, scan: impl FnOnce() -> Result_) -> Result_ {
        self.index_ranges.get(key, self.frame, scan)
    }

    /// `Attach`; records a write of the motion attachment at the render-target size.
    pub fn attach(&mut self, width: u32, height: u32) {
        let frame = self.frame;
        let target = self.get_target(width, height);
        target.written = true;
        target.used_frame = frame;
        target.layout = 1;
        let snapshot = *target;
        self.read_target = Some(snapshot);
    }

    /// `OnFrameStart`.
    pub fn on_frame_start(&mut self) {
        self.frame += 1;
        self.params_used = 0;
        self.history.next_frame();
        for target in &mut self.targets {
            target.written = false;
        }
    }

    /// `InvalidateHistory`.
    pub fn invalidate_history(&mut self) {
        self.history.next_frame();
        self.history.next_frame();
    }

    /// `PrepareRead`.
    pub fn prepare_read(&mut self, width: u32, height: u32) -> (u64, bool) {
        let target = self.get_target(width, height);
        target.layout = 1;
        let valid = target.written;
        let snapshot = *target;
        self.read_target = Some(snapshot);
        (snapshot.view, valid)
    }

    /// `View`.
    pub fn view(&self) -> u64 {
        self.read_target.map_or(0, |target| target.view)
    }

    /// `Image`.
    pub fn image(&self, width: u32, height: u32) -> u64 {
        self.find(width, height)
            .filter(|target| target.written)
            .map_or(0, |target| target.image)
    }

    fn find(&self, width: u32, height: u32) -> Option<Target> {
        self.targets
            .iter()
            .copied()
            .find(|target| target.width == width && target.height == height)
    }

    fn get_target(&mut self, width: u32, height: u32) -> &mut Target {
        if let Some(index) = self
            .targets
            .iter()
            .position(|target| target.width == width && target.height == height)
        {
            return &mut self.targets[index];
        }
        if self.targets.len() >= MAX_TARGETS {
            self.targets.pop_front();
        }
        self.targets.push_back(Target {
            image: self.targets.len() as u64 + 1,
            view: self.targets.len() as u64 + 101,
            width,
            height,
            ..Target::default()
        });
        self.targets.back_mut().expect("pushed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_targets_are_per_size_and_frames_reset_writes() {
        let mut motion = ObjectMotion::new(true);
        assert!(motion.enabled());
        assert_eq!(motion.prepare_draw(&Draw::default()), 1);
        motion.attach(1920, 1080);
        assert!(motion.image(1920, 1080) != 0);
        assert_eq!(motion.prepare_read(1920, 1080), (motion.view(), true));
        motion.on_frame_start();
        assert_eq!(motion.image(1920, 1080), 0);
        assert!(!motion.prepare_read(1920, 1080).1);
        for _ in 0..MAX_TARGETS + 1 {
            motion.attach(1280, 720);
        }
        assert!(motion.targets.len() <= MAX_TARGETS);
    }
}
