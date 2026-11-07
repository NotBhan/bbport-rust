//! Frame generation, ported from
//! `shadps4/video_core/renderer_vulkan/vk_frame_generation.h`.

/// `FrameGenCamera`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameGenCamera {
    pub near_plane: f32,
    pub far_plane: f32,
    pub vertical_fov: f32,
    pub has_matrices: bool,
    pub view_to_clip: [f32; 16],
    pub clip_to_prev_clip: [f32; 16],
    pub prev_clip_to_clip: [f32; 16],
}

impl Default for FrameGenCamera {
    fn default() -> Self {
        Self {
            near_plane: 0.1,
            far_plane: 10000.0,
            vertical_fov: 1.0,
            has_matrices: false,
            view_to_clip: [0.0; 16],
            clip_to_prev_clip: [0.0; 16],
            prev_clip_to_clip: [0.0; 16],
        }
    }
}

/// `FrameGenerator::Output`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Output {
    pub image: u64,
    pub view: u64,
}

/// `FrameGenerator`.
#[derive(Debug, Default)]
pub struct FrameGenerator {
    pub wanted: bool,
    pub wanted_count: u32,
    pub motion_valid: bool,
    pub jitter: [f32; 2],
    pub camera: FrameGenCamera,
    pub hudless: Option<(u64, u32, u32)>,
    pub frames_generated: u32,
}

impl FrameGenerator {
    pub const MAX_GENERATED: u32 = 3;
    pub const COLOR_FORMAT: i32 = 37; // VK_FORMAT_R8G8B8A8_UNORM

    pub fn new(wanted: bool, wanted_count: u32) -> Self {
        Self {
            wanted,
            wanted_count: wanted_count.min(Self::MAX_GENERATED),
            ..Self::default()
        }
    }

    /// `Wanted`.
    pub fn wanted(&self) -> bool {
        self.wanted
    }

    /// `WantedCount`.
    pub fn wanted_count(&self) -> u32 {
        self.wanted_count.min(Self::MAX_GENERATED)
    }

    /// `Skip`.
    pub fn skip(&mut self) {
        self.motion_valid = false;
        self.hudless = None;
    }

    /// `Release`.
    pub fn release(&mut self) {
        self.wanted = false;
        self.motion_valid = false;
        self.hudless = None;
    }

    /// `BeginCapture`; returns the motion view to write.
    pub fn begin_capture(&mut self, depth: u64, width: u32, height: u32) -> Option<u64> {
        if !self.wanted {
            return None;
        }
        let _ = (depth, width, height);
        Some(1)
    }

    /// `EndCapture`.
    pub fn end_capture(&mut self, jitter: [f32; 2], motion_valid: bool, camera: FrameGenCamera) {
        self.jitter = jitter;
        self.motion_valid = motion_valid;
        self.camera = camera;
    }

    /// `CaptureHudless`.
    pub fn capture_hudless(&mut self, source: u64, width: u32, height: u32) {
        if self.wanted && source != 0 {
            self.hudless = Some((source, width, height));
        }
    }

    /// `Hudless`.
    pub fn hudless(&self) -> Option<(u64, u32, u32)> {
        self.hudless
    }

    /// `Generate`.
    pub fn generate(
        &mut self,
        color: u64,
        hudless: u64,
        width: u32,
        height: u32,
        outputs: &[Output],
        backend: &mut dyn FrameGenBackend,
    ) -> u32 {
        if !self.wanted || color == 0 || width == 0 || height == 0 {
            return 0;
        }
        let count = self.wanted_count().min(outputs.len() as u32);
        for (index, output) in outputs.iter().take(count as usize).enumerate() {
            if !backend.generate(color, hudless, output.image, index as u32) {
                return index as u32;
            }
        }
        self.frames_generated += count;
        count
    }
}

pub trait FrameGenBackend {
    fn generate(&mut self, color: u64, hudless: u64, output: u64, index: u32) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Backend(u32);

    impl FrameGenBackend for Backend {
        fn generate(&mut self, _: u64, _: u64, _: u64, _: u32) -> bool {
            self.0 += 1;
            true
        }
    }

    #[test]
    fn frame_generation_counts_are_capped() {
        let mut generator = FrameGenerator::new(true, 9);
        assert!(generator.wanted());
        assert_eq!(generator.wanted_count(), FrameGenerator::MAX_GENERATED);
        assert!(generator.begin_capture(1, 1920, 1080).is_some());
        generator.end_capture([0.5, 0.5], true, FrameGenCamera::default());
        generator.capture_hudless(2, 1920, 1080);
        let outputs = [Output { image: 3, view: 4 }; 3];
        let mut backend = Backend(0);
        assert_eq!(
            generator.generate(1, 2, 1920, 1080, &outputs, &mut backend),
            3
        );
        assert_eq!(backend.0, 3);
        generator.release();
        assert!(!generator.wanted());
    }
}

/// The provider chosen by the cpp: FSR 4.1.1 replay, DLSS frame generation or FSR 3.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameGenProvider {
    None,
    Fsr411,
    Dlss,
    Fsr31,
}

/// Selects a provider from the settings and available bridge capabilities.
pub fn choose_provider(
    wanted: bool,
    dlss_available: bool,
    fsr411_assets: bool,
    fsr31: bool,
) -> FrameGenProvider {
    if !wanted {
        return FrameGenProvider::None;
    }
    if dlss_available {
        FrameGenProvider::Dlss
    } else if fsr411_assets {
        FrameGenProvider::Fsr411
    } else if fsr31 {
        FrameGenProvider::Fsr31
    } else {
        FrameGenProvider::None
    }
}

impl FrameGenerator {
    /// `WantedCount` for a DLSS multiplier (frames per game frame minus one).
    pub fn set_dlss_multiplier(&mut self, multiplier: u32) {
        self.wanted_count = multiplier.saturating_sub(1).min(Self::MAX_GENERATED);
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn frame_gen_provider_selection_prefers_dlss() {
        assert_eq!(
            choose_provider(false, true, true, true),
            FrameGenProvider::None
        );
        assert_eq!(
            choose_provider(true, true, true, true),
            FrameGenProvider::Dlss
        );
        assert_eq!(
            choose_provider(true, false, true, true),
            FrameGenProvider::Fsr411
        );
        assert_eq!(
            choose_provider(true, false, false, true),
            FrameGenProvider::Fsr31
        );
        let mut generator = FrameGenerator::new(true, 0);
        generator.set_dlss_multiplier(4);
        assert_eq!(generator.wanted_count(), 3);
    }
}
