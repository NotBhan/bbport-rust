//! `fsr4-bench` options, ported from `legacy/gpu/tools/fsr4_bench.cpp`.

use crate::gpu::video_core::renderer_vulkan::vk_fsr4::ModelPreset;

/// Parsed options; defaults match the tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BenchOptions {
    pub render_width: u32,
    pub render_height: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub preset: i32,
    pub frames: u32,
    pub stats: bool,
    pub fsr411: bool,
}

impl Default for BenchOptions {
    fn default() -> Self {
        Self {
            render_width: 2260,
            render_height: 1272,
            output_width: 3840,
            output_height: 2160,
            preset: 2,
            frames: 900,
            stats: false,
            fsr411: false,
        }
    }
}

fn parse_extent(value: &str) -> Option<(u32, u32)> {
    let (width, height) = value.split_once('x')?;
    Some((width.parse().ok()?, height.parse().ok()?))
}

impl BenchOptions {
    pub fn parse(args: &[String]) -> Self {
        let mut options = Self::default();
        let mut positional = 0;
        for argument in &args[1..] {
            if argument == "--fsr411" {
                options.fsr411 = true;
                continue;
            }
            if argument == "--stats" {
                options.stats = true;
                continue;
            }
            match positional {
                0 => {
                    if let Some((width, height)) = parse_extent(argument) {
                        options.render_width = width;
                        options.render_height = height;
                    }
                }
                1 => {
                    if let Some((width, height)) = parse_extent(argument) {
                        options.output_width = width;
                        options.output_height = height;
                    }
                }
                2 => options.preset = argument.parse().unwrap_or(options.preset),
                3 => options.frames = argument.parse().unwrap_or(options.frames),
                _ => {}
            }
            positional += 1;
        }
        options
    }

    pub fn model_preset(&self) -> ModelPreset {
        ModelPreset::from_setting(self.preset)
    }

    pub fn asset_dir() -> String {
        crate::gpu::video_core::renderer_vulkan::vk_fsr4::asset_dir()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bench_arguments_parse_like_the_tool() {
        let options = BenchOptions::parse(&[
            "fsr4-bench".into(),
            "1920x1080".into(),
            "3840x2160".into(),
            "1".into(),
            "30".into(),
            "--stats".into(),
            "--fsr411".into(),
        ]);
        assert_eq!((options.render_width, options.render_height), (1920, 1080));
        assert_eq!((options.output_width, options.output_height), (3840, 2160));
        assert_eq!(options.preset, 1);
        assert_eq!(options.frames, 30);
        assert!(options.stats);
        assert!(options.fsr411);
        assert_eq!(options.model_preset(), ModelPreset::Quality);
        assert_eq!(BenchOptions::default().frames, 900);
    }
}
