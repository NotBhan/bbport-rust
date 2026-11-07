//! MSAA blit pipelines, ported from
//! `shadps4/video_core/texture_cache/blit_helper.h`.

use crate::gpu::video_core::renderer_vulkan::vk_common::{VkFormat, VkImage};

pub const MAX_MS_PIPELINES: usize = 6;

/// `BlitHelper::MsPipelineKey`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MsPipelineKey {
    pub num_samples: u32,
    pub attachment_format: VkFormat,
    pub src_msaa: bool,
}

/// `BlitHelper`; the Vulkan pipelines are supplied by the renderer.
#[derive(Debug, Default)]
pub struct BlitHelper {
    pub color_to_ms_depth_pl: Vec<(MsPipelineKey, u64)>,
    pub ms_image_copy_pl: Vec<(MsPipelineKey, u64)>,
}

impl BlitHelper {
    pub fn reinterpret_color_as_ms_depth(
        &mut self,
        width: u32,
        height: u32,
        key: MsPipelineKey,
        source: VkImage,
        dest: VkImage,
        record: &mut dyn BlitRecorder,
    ) {
        record.reinterpret(width, height, key, source, dest);
    }

    pub fn copy_between_ms_images(
        &mut self,
        width: u32,
        height: u32,
        key: MsPipelineKey,
        source: VkImage,
        dest: VkImage,
        record: &mut dyn BlitRecorder,
    ) {
        record.copy(width, height, key, source, dest);
    }

    pub fn pipeline_count(&self) -> usize {
        self.color_to_ms_depth_pl.len() + self.ms_image_copy_pl.len()
    }
}

pub trait BlitRecorder {
    fn reinterpret(
        &mut self,
        width: u32,
        height: u32,
        key: MsPipelineKey,
        source: VkImage,
        dest: VkImage,
    );
    fn copy(&mut self, width: u32, height: u32, key: MsPipelineKey, source: VkImage, dest: VkImage);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder {
        calls: usize,
    }

    impl BlitRecorder for Recorder {
        fn reinterpret(&mut self, _: u32, _: u32, _: MsPipelineKey, _: VkImage, _: VkImage) {
            self.calls += 1;
        }
        fn copy(&mut self, _: u32, _: u32, _: MsPipelineKey, _: VkImage, _: VkImage) {
            self.calls += 1;
        }
    }

    #[test]
    fn ms_pipeline_keys_order_by_sample_count_and_format() {
        let a = MsPipelineKey {
            num_samples: 2,
            attachment_format: 1,
            src_msaa: true,
        };
        let b = MsPipelineKey {
            num_samples: 4,
            ..a
        };
        assert!(a < b);
        assert_eq!(MAX_MS_PIPELINES, 6);
        let mut helper = BlitHelper::default();
        let mut recorder = Recorder::default();
        helper.reinterpret_color_as_ms_depth(8, 8, a, 1, 2, &mut recorder);
        helper.copy_between_ms_images(8, 8, b, 1, 2, &mut recorder);
        assert_eq!(recorder.calls, 2);
        assert_eq!(helper.pipeline_count(), 0);
    }
}

impl BlitHelper {
    /// `CreateColorToMSDepthPipeline` / `CreateMsCopyPipeline`; caches one pipeline per key.
    pub fn get_or_create_pipeline(&mut self, key: MsPipelineKey, depth: bool) -> u64 {
        let total = self.color_to_ms_depth_pl.len() + self.ms_image_copy_pl.len() + 1;
        let pipelines = if depth {
            &mut self.color_to_ms_depth_pl
        } else {
            &mut self.ms_image_copy_pl
        };
        if let Some((_, pipeline)) = pipelines.iter().find(|(entry, _)| *entry == key) {
            return *pipeline;
        }
        let pipeline = total as u64;
        pipelines.push((key, pipeline));
        pipeline
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn blit_pipelines_are_cached_by_key() {
        let mut helper = BlitHelper::default();
        let key = MsPipelineKey {
            num_samples: 4,
            attachment_format: 37,
            src_msaa: true,
        };
        let first = helper.get_or_create_pipeline(key, true);
        assert_eq!(helper.get_or_create_pipeline(key, true), first);
        assert_ne!(helper.get_or_create_pipeline(key, false), first);
        assert_eq!(helper.pipeline_count(), 2);
    }
}
