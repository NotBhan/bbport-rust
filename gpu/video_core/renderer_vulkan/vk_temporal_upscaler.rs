//! Temporal upscaler interface, ported from
//! `shadps4/video_core/renderer_vulkan/vk_temporal_upscaler.h`.

use super::vk_frame_generation::FrameGenCamera;

/// `TemporalUpscaler::Settings`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Settings {
    pub enabled: bool,
    pub sharpness: f32,
    pub reset: bool,
    pub hdr: bool,
}

/// The upscaler inputs.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Inputs {
    pub render_width: u32,
    pub render_height: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub jitter: [f32; 2],
    pub frame_ms: f32,
    pub camera: FrameGenCamera,
    pub motion_valid: bool,
}

/// The temporal upscaler state; Vulkan providers implement `TemporalUpscalerBackend`.
#[derive(Debug, Default)]
pub struct TemporalUpscaler {
    pub settings: Settings,
    pub inputs: Inputs,
    pub frame: u64,
    pub problem: Option<String>,
}

impl TemporalUpscaler {
    pub fn enabled(&self) -> bool {
        self.settings.enabled
    }

    pub fn set_settings(&mut self, settings: Settings) {
        self.settings = settings;
    }

    pub fn begin_frame(&mut self, inputs: Inputs) {
        if inputs.render_width != self.inputs.render_width
            || inputs.render_height != self.inputs.render_height
            || inputs.output_width != self.inputs.output_width
            || inputs.output_height != self.inputs.output_height
        {
            self.settings.reset = true;
        }
        self.inputs = inputs;
        self.frame += 1;
    }

    pub fn record(&mut self, backend: &mut dyn TemporalUpscalerBackend) -> bool {
        if !self.settings.enabled {
            return false;
        }
        if self.inputs.render_width == 0 || self.inputs.output_width == 0 {
            self.problem = Some("invalid temporal upscale sizes".to_string());
            return false;
        }
        if !backend.record(&self.inputs, self.settings) {
            self.problem = Some("temporal upscaler unavailable".to_string());
            return false;
        }
        self.problem = None;
        true
    }
}

pub trait TemporalUpscalerBackend {
    fn record(&mut self, inputs: &Inputs, settings: Settings) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Backend;
    impl TemporalUpscalerBackend for Backend {
        fn record(&mut self, _: &Inputs, _: Settings) -> bool {
            true
        }
    }

    #[test]
    fn temporal_upscaler_tracks_size_changes_and_frames() {
        let mut upscaler = TemporalUpscaler::default();
        assert!(!upscaler.record(&mut Backend));
        upscaler.set_settings(Settings {
            enabled: true,
            ..Settings::default()
        });
        upscaler.begin_frame(Inputs {
            render_width: 1920,
            render_height: 1080,
            output_width: 3840,
            output_height: 2160,
            ..Inputs::default()
        });
        assert_eq!(upscaler.frame, 1);
        assert!(upscaler.record(&mut Backend));
        upscaler.begin_frame(Inputs {
            render_width: 1280,
            render_height: 720,
            output_width: 3840,
            output_height: 2160,
            ..Inputs::default()
        });
        assert!(upscaler.settings.reset);
    }
}

/// The providers `vk_temporal_upscaler.cpp` can select.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpscalerProvider {
    None,
    Fsr31,
    Fsr4,
    Fsr411,
    Dlss,
}

impl TemporalUpscaler {
    /// `Create`: chooses the strongest available provider.
    pub fn choose_provider(
        &mut self,
        fsr411: bool,
        fsr4: bool,
        dlss: bool,
        fsr31: bool,
    ) -> UpscalerProvider {
        let provider = if dlss {
            UpscalerProvider::Dlss
        } else if fsr411 {
            UpscalerProvider::Fsr411
        } else if fsr4 {
            UpscalerProvider::Fsr4
        } else if fsr31 {
            UpscalerProvider::Fsr31
        } else {
            UpscalerProvider::None
        };
        if provider == UpscalerProvider::None {
            self.problem = Some("no temporal upscaler available".to_string());
        } else {
            self.problem = None;
        }
        provider
    }

    pub fn record_frame(&mut self, backend: &mut dyn TemporalUpscalerBackend) -> bool {
        self.record(backend)
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn temporal_provider_choice_prefers_dlss() {
        let mut upscaler = TemporalUpscaler::default();
        assert_eq!(
            upscaler.choose_provider(false, false, false, false),
            UpscalerProvider::None
        );
        assert!(upscaler.problem.is_some());
        assert_eq!(
            upscaler.choose_provider(true, true, true, true),
            UpscalerProvider::Dlss
        );
        assert_eq!(
            upscaler.choose_provider(true, true, false, true),
            UpscalerProvider::Fsr411
        );
        assert!(upscaler.problem.is_none());
    }
}
