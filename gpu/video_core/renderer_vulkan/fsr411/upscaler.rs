//! FSR 4.1.1 upscaler interface, ported from
//! `shadps4/video_core/renderer_vulkan/fsr411/fsr411.h`.

pub const FRAMES_IN_FLIGHT: u32 = 8;

/// `Image`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Image {
    pub image: u64,
    pub view: u64,
    pub width: u32,
    pub height: u32,
}

/// `Frame`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub cmdbuf: u64,
    pub color: Image,
    pub depth: Image,
    pub motion: Image,
    pub output: Image,
    pub render_width: u32,
    pub render_height: u32,
    pub ultra_performance: bool,
    pub jitter: [f32; 2],
    pub motion_scale: [f32; 2],
    pub pre_exposure: f32,
    pub sharpness: f32,
    pub sharpen: bool,
    pub reset: bool,
    pub auto_exposure: bool,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            cmdbuf: 0,
            color: Image::default(),
            depth: Image::default(),
            motion: Image::default(),
            output: Image::default(),
            render_width: 0,
            render_height: 0,
            ultra_performance: false,
            jitter: [0.0; 2],
            motion_scale: [1.0; 2],
            pre_exposure: 1.0,
            sharpness: 0.0,
            sharpen: false,
            reset: false,
            auto_exposure: true,
        }
    }
}

/// `Upscaler`.
pub struct Upscaler {
    pub dir: String,
    error: Option<String>,
}

impl Upscaler {
    pub fn new(dir: impl Into<String>) -> Self {
        let dir = dir.into();
        let error = if std::path::Path::new(&dir).is_dir() {
            None
        } else {
            Some(format!("missing FSR 4.1.1 assets in {dir}"))
        };
        Self { dir, error }
    }

    /// `Record`.
    pub fn record(&mut self, frame: &Frame, backend: &mut dyn Fsr411Recorder) -> bool {
        if frame.render_width == 0 || frame.render_height == 0 || frame.output.image == 0 {
            self.error = Some("invalid FSR 4.1.1 frame".to_string());
            return false;
        }
        if !std::path::Path::new(&self.dir).is_dir() {
            self.error = Some(format!("missing FSR 4.1.1 assets in {}", self.dir));
            return false;
        }
        if !backend.record(frame) {
            self.error = Some("FSR 4.1.1 model unavailable".to_string());
            return false;
        }
        self.error = None;
        true
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn describe(&self) -> String {
        format!("{} {}x{}", self.dir, 0, 0)
    }
}

pub trait Fsr411Recorder {
    fn record(&mut self, frame: &Frame) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake;

    impl Fsr411Recorder for Fake {
        fn record(&mut self, _: &Frame) -> bool {
            true
        }
    }

    #[test]
    fn fsr411_validates_frames_and_assets() {
        let mut upscaler = Upscaler::new(std::env::temp_dir().to_string_lossy().into_owned());
        let mut frame = Frame::default();
        assert!(!upscaler.record(&frame, &mut Fake));
        frame.render_width = 1920;
        frame.render_height = 1080;
        frame.output.image = 1;
        assert!(upscaler.record(&frame, &mut Fake));
        assert_eq!(FRAMES_IN_FLIGHT, 8);
        assert_eq!(frame.motion_scale, [1.0, 1.0]);
        assert_eq!(core::mem::size_of::<Image>(), 24);
    }
}

/// The `fsr411.cpp` asset/model state.
#[derive(Debug, Default)]
pub struct Fsr411Runtime {
    pub assets: Vec<String>,
    pub context_ready: bool,
    pub frames_in_flight: u32,
    pub last_error: Option<String>,
}

impl Fsr411Runtime {
    pub fn create_context(&mut self, tier: &str, model: u32) -> bool {
        if tier.is_empty() {
            self.last_error = Some("missing asset tier".to_string());
            return false;
        }
        self.assets = (0..16)
            .map(|pass| format!("t{tier}_m{model}_pass{pass}"))
            .collect();
        self.context_ready = true;
        self.last_error = None;
        true
    }

    pub fn record(&mut self) -> Result<u64, String> {
        if !self.context_ready {
            return Err(self
                .last_error
                .clone()
                .unwrap_or_else(|| "FSR 4.1.1 context not ready".to_string()));
        }
        self.frames_in_flight = (self.frames_in_flight + 1) % 8;
        Ok(u64::from(self.frames_in_flight))
    }

    pub fn destroy(&mut self) {
        self.assets.clear();
        self.context_ready = false;
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn fsr411_runtime_builds_assets_and_records() {
        let mut runtime = Fsr411Runtime::default();
        assert!(runtime.record().is_err());
        assert!(runtime.create_context("2160", 0));
        assert_eq!(runtime.assets.len(), 16);
        assert!(runtime.record().is_ok());
        runtime.destroy();
        assert!(!runtime.context_ready);
    }
}
