//! FSR host pass, ported from
//! `shadps4/video_core/renderer_vulkan/host_passes/fsr_pass.h`.

/// `FsrPass::Settings`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub enable: bool,
    pub use_rcas: bool,
    pub rcas_attenuation: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enable: true,
            use_rcas: true,
            rcas_attenuation: 0.25,
        }
    }
}

/// `FsrPass::Img`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Img {
    pub id: u8,
    pub dirty: bool,
    pub intermediary_image: u64,
    pub output_image: u64,
}

/// `FsrPass`.
#[derive(Debug, Default)]
pub struct FsrPass {
    pub device: u64,
    pub num_images: u32,
    pub cur_size: (u32, u32),
    pub cur_image: u32,
    pub available_imgs: Vec<Img>,
}

impl FsrPass {
    pub fn create(&mut self, device: u64, num_images: u32) {
        self.device = device;
        self.num_images = num_images;
        self.available_imgs = vec![Img::default(); num_images as usize];
    }

    /// `Render`; returns the output view produced by the pass.
    pub fn render(
        &mut self,
        input: u64,
        input_size: (u32, u32),
        output_size: (u32, u32),
        settings: Settings,
        hdr: bool,
        recorder: &mut dyn FsrPassRecorder,
    ) -> u64 {
        if !settings.enable || input == 0 {
            return input;
        }
        if self.cur_size != output_size {
            self.resize_and_invalidate(output_size.0, output_size.1);
        }
        let index = self.cur_image as usize % self.available_imgs.len().max(1);
        self.cur_image = self.cur_image.wrapping_add(1);
        recorder.render(input, input_size, output_size, settings, hdr, index)
    }

    fn resize_and_invalidate(&mut self, width: u32, height: u32) {
        self.cur_size = (width, height);
        for image in &mut self.available_imgs {
            image.dirty = true;
        }
    }
}

pub trait FsrPassRecorder {
    fn render(
        &mut self,
        input: u64,
        input_size: (u32, u32),
        output_size: (u32, u32),
        settings: Settings,
        hdr: bool,
        image_index: usize,
    ) -> u64;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder(u32);

    impl FsrPassRecorder for Recorder {
        fn render(
            &mut self,
            _: u64,
            _: (u32, u32),
            _: (u32, u32),
            _: Settings,
            _: bool,
            _: usize,
        ) -> u64 {
            self.0 += 1;
            99
        }
    }

    #[test]
    fn fsr_passes_resize_and_cycle_images() {
        let mut pass = FsrPass::default();
        pass.create(1, 2);
        let mut recorder = Recorder::default();
        assert_eq!(
            pass.render(
                5,
                (8, 8),
                (16, 16),
                Settings::default(),
                true,
                &mut recorder
            ),
            99
        );
        assert_eq!(pass.cur_size, (16, 16));
        assert_eq!(recorder.0, 1);
        let off = Settings {
            enable: false,
            ..Settings::default()
        };
        assert_eq!(
            pass.render(5, (8, 8), (16, 16), off, false, &mut recorder),
            5
        );
    }
}
