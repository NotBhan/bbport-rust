//! FSR 4 upscaler types, ported from
//! `shadps4/video_core/renderer_vulkan/vk_fsr4.h`.

use crate::gpu::video_core::renderer_vulkan::vk_common::{VkCommandBuffer, VkImage, VkImageView};

/// `Fsr4Upscaler::Image`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Image {
    pub image: VkImage,
    pub view: VkImageView,
    pub width: u32,
    pub height: u32,
}

/// `Fsr4Upscaler::Frame`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub cmdbuf: VkCommandBuffer,
    pub color: Image,
    pub depth: Image,
    pub motion: Image,
    pub output: Image,
    pub render_width: u32,
    pub render_height: u32,
    pub preset: i32,
    pub jitter: [f32; 2],
    pub frame_ms: f32,
    pub near_plane: f32,
    pub far_plane: f32,
    pub vertical_fov: f32,
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
            preset: 0,
            jitter: [0.0; 2],
            frame_ms: 0.0,
            near_plane: 0.0,
            far_plane: 0.0,
            vertical_fov: 0.0,
            sharpness: 0.0,
            sharpen: false,
            reset: false,
            auto_exposure: false,
        }
    }
}

/// `Fsr4Upscaler`.
pub struct Fsr4Upscaler {
    problem: Option<String>,
    fatal: bool,
}

impl Fsr4Upscaler {
    pub fn new() -> Self {
        Self {
            problem: None,
            fatal: false,
        }
    }

    /// `Record`; implementations report why they cannot run through `problem`.
    pub fn record(&mut self, frame: &Frame, recorder: &mut dyn Fsr4Recorder) -> bool {
        if frame.output.image == 0 || frame.render_width == 0 || frame.render_height == 0 {
            self.problem = Some("invalid FSR 4 frame".to_string());
            return false;
        }
        if !recorder.record(frame) {
            self.problem = Some("FSR 4 model unavailable".to_string());
            self.fatal = true;
            return false;
        }
        self.problem = None;
        true
    }

    pub fn problem(&self) -> Option<&str> {
        self.problem.as_deref()
    }

    pub fn fatal(&self) -> bool {
        self.fatal
    }
}

impl Default for Fsr4Upscaler {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Fsr4Recorder {
    fn record(&mut self, frame: &Frame) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Ok;

    impl Fsr4Recorder for Ok {
        fn record(&mut self, _: &Frame) -> bool {
            true
        }
    }

    #[test]
    fn fsr4_frames_validate_before_recording() {
        let mut upscaler = Fsr4Upscaler::new();
        let mut frame = Frame::default();
        assert!(!upscaler.record(&frame, &mut Ok));
        assert_eq!(upscaler.problem(), Some("invalid FSR 4 frame"));
        frame.output.image = 1;
        frame.render_width = 1920;
        frame.render_height = 1080;
        assert!(upscaler.record(&frame, &mut Ok));
        assert!(!upscaler.fatal());
        assert_eq!(core::mem::size_of::<Image>(), 24);
    }
}

pub const MODEL_PASS_COUNT: usize = 12;
pub const VK_PASS_COUNT: usize = 16;
pub const MAX_OUTPUT_WIDTH: u32 = 7680;
pub const MAX_OUTPUT_HEIGHT: u32 = 4320;
pub const INITIALIZER_BYTES: usize = 89216;
pub const PRE_PASS_WEIGHTS_BYTES: usize = 1024;

/// `FfxFsr4ModelPreset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ModelPreset {
    NativeAa = 0,
    Quality = 1,
    Balanced = 2,
    Performance = 3,
    UltraPerformance = 4,
    Drs = 5,
}

impl ModelPreset {
    /// `ffxFsr4ModelPresetName`.
    pub fn name(self) -> &'static str {
        match self {
            Self::NativeAa => "native",
            Self::Quality => "quality",
            Self::Balanced => "balanced",
            Self::Performance => "performance",
            Self::UltraPerformance => "ultraperf",
            Self::Drs => "drs",
        }
    }

    /// `ModelPreset` from the settings preset.
    pub fn from_setting(preset: i32) -> Self {
        match preset {
            0 => Self::NativeAa,
            1 => Self::Quality,
            2 => Self::Balanced,
            3 => Self::Performance,
            _ => Self::UltraPerformance,
        }
    }
}

/// `AssetDir`; `BB_FSR4_DIR` overrides `fsr4_shaders`.
pub fn asset_dir() -> String {
    std::env::var("BB_FSR4_DIR")
        .ok()
        .filter(|dir| !dir.is_empty())
        .unwrap_or_else(|| "fsr4_shaders".to_string())
}

/// `ffxFsr4V07SelectAssetTier`.
pub fn select_asset_tier(width: u32, height: u32) -> Option<&'static str> {
    if width == 0 || height == 0 || width > MAX_OUTPUT_WIDTH || height > MAX_OUTPUT_HEIGHT {
        return None;
    }
    if width > 3840 || height > 2160 {
        Some("4320")
    } else if width > 1920 || height > 1080 {
        Some("2160")
    } else {
        Some("1080")
    }
}

/// `FfxFsr4V07AssetSet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetSet {
    pub preset: ModelPreset,
    pub tier: String,
    pub pre: String,
    pub post: String,
    pub initializer: String,
    pub pre_pass_weights: String,
    pub model: Vec<String>,
    pub rcas: String,
    pub spd_auto_exposure: String,
}

/// `ffxFsr4V07BuildAssetSet`.
pub fn build_asset_set(preset: ModelPreset, width: u32, height: u32) -> Option<AssetSet> {
    let tier = select_asset_tier(width, height)?;
    let model = preset.name();
    Some(AssetSet {
        preset,
        tier: tier.to_string(),
        pre: format!("fsr4_model_v07_i8_{model}_{tier}_pre.spv"),
        post: format!("fsr4_model_v07_i8_{model}_{tier}_post.spv"),
        initializer: format!("fsr4_model_v07_i8_{model}_initializers.bin"),
        pre_pass_weights: format!("fsr4_model_v07_i8_{model}_pre_weights.bin"),
        model: (1..=MODEL_PASS_COUNT)
            .map(|pass| format!("fsr4_model_v07_i8_{model}_{tier}_pass{pass}.spv"))
            .collect(),
        rcas: "rcas.spv".to_string(),
        spd_auto_exposure: "spd_auto_exposure.spv".to_string(),
    })
}

/// The assets loaded from disk (SPIR-V kept as raw bytes).
#[derive(Debug, Default)]
pub struct LoadedAssets {
    pub shaders: Vec<Vec<u8>>,
    pub initializer: Vec<u8>,
    pub pre_pass_weights: Vec<u8>,
    pub optimized: u32,
}

/// Loads an asset set, preferring `opt/` unless `BB_FSR4_OPT=0`.
pub fn load_asset_set(
    dir: &str,
    set: &AssetSet,
    read: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) -> Result<LoadedAssets, String> {
    let use_opt = std::env::var("BB_FSR4_OPT")
        .map(|value| value != "0")
        .unwrap_or(true);
    let mut out = LoadedAssets::default();
    let mut load = |name: &str, out: &mut Vec<u8>| -> Result<(), String> {
        if use_opt {
            if let Some(data) = read(&format!("{dir}/opt/{name}")) {
                out.extend_from_slice(&data);
                return Ok(());
            }
        }
        match read(&format!("{dir}/{name}")) {
            Some(data) => {
                out.extend_from_slice(&data);
                Ok(())
            }
            None => Err(format!(
                "missing {dir}/{name} (run tools/fetch_fsr4_assets.sh)"
            )),
        }
    };
    let mut pre = Vec::new();
    load(&set.pre, &mut pre)?;
    out.shaders.push(pre);
    for name in &set.model {
        let mut code = Vec::new();
        load(name, &mut code)?;
        out.shaders.push(code);
    }
    for name in [&set.post, &set.rcas, &set.spd_auto_exposure] {
        let mut code = Vec::new();
        load(name, &mut code)?;
        out.shaders.push(code);
    }
    load(&set.initializer, &mut out.initializer)?;
    load(&set.pre_pass_weights, &mut out.pre_pass_weights)?;
    if out.initializer.len() != INITIALIZER_BYTES {
        return Err("model weights have an unexpected size".to_string());
    }
    if out.pre_pass_weights.len() != PRE_PASS_WEIGHTS_BYTES {
        return Err("model weights have an unexpected size".to_string());
    }
    Ok(out)
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn fsr4_asset_sets_select_tier_and_preset() {
        assert_eq!(select_asset_tier(1920, 1080), Some("1080"));
        assert_eq!(select_asset_tier(1921, 1080), Some("2160"));
        assert_eq!(select_asset_tier(3841, 2160), Some("4320"));
        assert_eq!(select_asset_tier(7681, 4320), None);
        assert_eq!(ModelPreset::Quality.name(), "quality");
        let set = build_asset_set(ModelPreset::Quality, 2560, 1440).unwrap();
        assert_eq!(set.tier, "2160");
        assert!(set.pre.contains("quality_2160_pre"));
        assert_eq!(set.model.len(), MODEL_PASS_COUNT);
        assert!(set.model[11].contains("pass12"));
        let mut read = |name: &str| {
            if name.contains("initializers") {
                Some(vec![0; INITIALIZER_BYTES])
            } else if name.contains("pre_weights") {
                Some(vec![0; PRE_PASS_WEIGHTS_BYTES])
            } else {
                Some(vec![1, 0, 0, 0])
            }
        };
        let loaded = load_asset_set("x", &set, &mut read).unwrap();
        assert_eq!(loaded.shaders.len(), VK_PASS_COUNT);
        assert_eq!(loaded.initializer.len(), INITIALIZER_BYTES);
    }
}
