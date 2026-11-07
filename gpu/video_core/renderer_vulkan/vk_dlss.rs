//! DLSS bridge types, ported from
//! `shadps4/video_core/renderer_vulkan/vk_dlss.h`.

use crate::gpu::video_core::renderer_vulkan::vk_common::VkFormat;

/// `Dlss::Resource`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Resource {
    pub image: u64,
    pub view: u64,
    pub format: VkFormat,
    pub aspect: u32,
    pub width: u32,
    pub height: u32,
}

/// `Dlss::FeatureDesc`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FeatureDesc {
    pub input_width: u32,
    pub input_height: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub quality: i32,
    pub hdr: bool,
}

/// `Dlss::Frame`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Frame {
    pub color: Resource,
    pub depth: Resource,
    pub motion: Resource,
    pub output: Resource,
    pub jitter_x: f32,
    pub jitter_y: f32,
    pub reset: bool,
    pub frame_ms: f32,
    pub sharpness: f32,
}

/// `Dlss::FrameGenDesc`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameGenDesc {
    pub width: u32,
    pub height: u32,
    pub format: VkFormat,
    pub render_width: u32,
    pub render_height: u32,
}

/// `Dlss::FrameGenFrame`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FrameGenFrame {
    pub backbuffer: Resource,
    pub depth: Resource,
    pub motion: Resource,
    pub output: Resource,
    pub hudless: Resource,
    pub jitter_x: f32,
    pub jitter_y: f32,
    pub camera_near: f32,
    pub camera_far: f32,
    pub camera_fov: f32,
    pub depth_inverted: bool,
    pub has_matrices: bool,
    pub view_to_clip: [f32; 16],
    pub clip_to_prev_clip: [f32; 16],
    pub prev_clip_to_clip: [f32; 16],
    pub reset: bool,
    pub count: u32,
    pub index: u32,
}

impl Default for FrameGenFrame {
    fn default() -> Self {
        Self {
            backbuffer: Resource::default(),
            depth: Resource::default(),
            motion: Resource::default(),
            output: Resource::default(),
            hudless: Resource::default(),
            jitter_x: 0.0,
            jitter_y: 0.0,
            camera_near: 0.0,
            camera_far: 0.0,
            camera_fov: 0.0,
            depth_inverted: false,
            has_matrices: false,
            view_to_clip: [0.0; 16],
            clip_to_prev_clip: [0.0; 16],
            prev_clip_to_clip: [0.0; 16],
            reset: false,
            count: 0,
            index: 0,
        }
    }
}

/// `Dlss`.
#[derive(Debug, Default)]
pub struct Dlss {
    available: bool,
    problem: String,
    frame_gen_max_count: i32,
}

impl Dlss {
    /// `Get`; DLSS is present only when the bridge reported it.
    pub fn get() -> Option<Self> {
        if !Self::enabled() {
            return None;
        }
        Some(Self {
            available: true,
            problem: String::new(),
            frame_gen_max_count: 0,
        })
    }

    pub fn enabled() -> bool {
        std::env::var("BB_DLSS")
            .map(|value| value != "0")
            .unwrap_or(false)
    }

    pub fn available(&self) -> bool {
        self.available
    }

    pub fn problem(&self) -> &str {
        &self.problem
    }

    pub fn disable(&mut self, reason: &str) {
        self.available = false;
        self.problem = reason.to_string();
    }

    /// `QualityForScale`; `scale` is output/render (an upscale factor).
    pub fn quality_for_scale(scale: f32) -> i32 {
        if scale >= 2.9 {
            4
        } else if scale >= 1.95 {
            3
        } else if scale >= 1.65 {
            2
        } else if scale > 1.01 {
            1
        } else {
            0
        }
    }

    /// `Dlss::Get` discovery: `BB_DLSS=0` disables, both DLLs must be present.
    pub fn discover(dir: &std::path::Path) -> Option<Self> {
        if std::env::var("BB_DLSS")
            .map(|value| value == "0")
            .unwrap_or(false)
        {
            return None;
        }
        if !dir.join("bbport_dlss.dll").is_file() || !dir.join("nvngx_dlss.dll").is_file() {
            return None;
        }
        Self::get()
    }

    pub fn has_feature(&self, _desc: &FeatureDesc) -> bool {
        self.available
    }

    pub fn has_frame_gen(&self, _desc: &FrameGenDesc) -> bool {
        self.available && self.frame_gen_max_count > 0
    }

    pub fn frame_gen_max_count(&self) -> i32 {
        self.frame_gen_max_count
    }

    pub fn set_frame_gen_max_count(&mut self, count: i32) {
        self.frame_gen_max_count = count;
    }
}

pub trait DlssBackend {
    fn create_feature(&mut self, command: u64, desc: &FeatureDesc) -> bool;
    fn evaluate(&mut self, command: u64, frame: &Frame) -> bool;
    fn create_frame_gen(&mut self, command: u64, desc: &FrameGenDesc) -> bool;
    fn evaluate_frame_gen(&mut self, command: u64, frame: &FrameGenFrame) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dlss_types_map_quality_and_scale() {
        assert_eq!(Dlss::quality_for_scale(1.0), 0);
        assert_eq!(Dlss::quality_for_scale(1.5), 1);
        assert_eq!(Dlss::quality_for_scale(1.7), 2);
        assert_eq!(Dlss::quality_for_scale(2.0), 3);
        assert_eq!(Dlss::quality_for_scale(3.0), 4);
        let mut dlss = Dlss::get().unwrap_or_default();
        dlss.set_frame_gen_max_count(1);
        let desc = FrameGenDesc::default();
        assert!(dlss.has_frame_gen(&desc) == dlss.available());
        assert_eq!(core::mem::size_of::<Resource>(), 32);
        assert_eq!(core::mem::size_of::<FeatureDesc>(), 24);
    }
}
