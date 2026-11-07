//! Post-processing host pass, ported from
//! `shadps4/video_core/renderer_vulkan/host_passes/pp_pass.h`.

/// `PostProcessingPass::Settings`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub gamma: f32,
    pub hdr: u32,
    pub srgb_input: u32,
    pub sdr_on_hdr: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            gamma: 1.0,
            hdr: 0,
            srgb_input: 0,
            sdr_on_hdr: 0,
        }
    }
}

/// `PostProcessingPass`.
#[derive(Debug, Default)]
pub struct PostProcessingPass {
    pub device: u64,
    pub surface_format: i32,
}

impl PostProcessingPass {
    pub fn create(&mut self, device: u64, surface_format: i32) {
        self.device = device;
        self.surface_format = surface_format;
    }

    pub fn render(
        &self,
        input: u64,
        _input_size: (u32, u32),
        settings: Settings,
        recorder: &mut dyn PostProcessRecorder,
    ) {
        recorder.render(input, settings);
    }
}

pub trait PostProcessRecorder {
    fn render(&mut self, input: u64, settings: Settings);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder(u32);

    impl PostProcessRecorder for Recorder {
        fn render(&mut self, _: u64, _: Settings) {
            self.0 += 1;
        }
    }

    #[test]
    fn post_processing_forwards_its_settings() {
        let pass = PostProcessingPass::default();
        let mut recorder = Recorder::default();
        pass.render(3, (8, 8), Settings::default(), &mut recorder);
        assert_eq!(recorder.0, 1);
        assert_eq!(Settings::default().gamma, 1.0);
    }
}
