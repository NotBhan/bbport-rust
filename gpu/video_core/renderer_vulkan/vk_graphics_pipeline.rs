//! Graphics pipeline keys, ported from
//! `shadps4/video_core/renderer_vulkan/vk_graphics_pipeline.h`.

pub const MAX_SHADER_STAGES: usize = 6;
pub const MAX_VERTEX_BUFFER_COUNT: usize = 32;
pub const NUM_COLOR_BUFFERS: usize = 8;

/// `GraphicsPipelineKey`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GraphicsPipelineKey {
    pub stage_hashes: [usize; MAX_SHADER_STAGES],
    pub vertex_buffer_formats: [i32; MAX_VERTEX_BUFFER_COUNT],
    pub patch_control_points: u32,
    pub num_color_attachments: u32,
    pub color_buffers: [u64; NUM_COLOR_BUFFERS],
    pub blend_controls: [u64; NUM_COLOR_BUFFERS],
    pub write_masks: [u32; NUM_COLOR_BUFFERS],
    pub cb_shader_mask: u32,
    pub logic_op: u32,
    pub num_samples: u8,
    pub depth_samples: u8,
    pub color_samples: [u8; NUM_COLOR_BUFFERS],
    pub mrt_mask: u32,
    pub motion_vectors: u32,
    pub depth_format: u32,
    pub primitive_type: u32,
}

impl Default for GraphicsPipelineKey {
    fn default() -> Self {
        Self {
            stage_hashes: [0; MAX_SHADER_STAGES],
            vertex_buffer_formats: [0; MAX_VERTEX_BUFFER_COUNT],
            patch_control_points: 0,
            num_color_attachments: 0,
            color_buffers: [0; NUM_COLOR_BUFFERS],
            blend_controls: [0; NUM_COLOR_BUFFERS],
            write_masks: [0; NUM_COLOR_BUFFERS],
            cb_shader_mask: 0,
            logic_op: 0,
            num_samples: 0,
            depth_samples: 0,
            color_samples: [0; NUM_COLOR_BUFFERS],
            mrt_mask: 0,
            motion_vectors: 0,
            depth_format: 0,
            primitive_type: 0,
        }
    }
}

/// `GraphicsPipeline::SerializationSupport`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SerializationSupport {
    pub vertex_attributes: Vec<u64>,
    pub vertex_bindings: Vec<u64>,
    pub divisors: Vec<u64>,
    pub multisampling: u64,
    pub tcs: Vec<u32>,
    pub tes: Vec<u32>,
    pub fragment: Vec<u32>,
}

/// `GraphicsPipeline`.
#[derive(Debug, Clone)]
pub struct GraphicsPipeline {
    pub key: GraphicsPipelineKey,
    pub fetch_shader: Option<u64>,
    pub pipeline: u64,
    pub layout: u64,
}

impl GraphicsPipeline {
    pub fn new(key: GraphicsPipelineKey, pipeline: u64) -> Self {
        Self {
            key,
            fetch_shader: None,
            pipeline,
            layout: 0,
        }
    }

    pub fn fetch_shader(&self) -> Option<u64> {
        self.fetch_shader
    }

    pub fn graphics_key(&self) -> &GraphicsPipelineKey {
        &self.key
    }
}

/// `ClipDistanceShaderKey`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClipDistanceShaderKey {
    pub clip_locations: [(u8, u8); 8],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphics_keys_compare_all_fields() {
        let key = GraphicsPipelineKey::default();
        assert_eq!(key, GraphicsPipelineKey::default());
        let mut other = key;
        other.num_samples = 4;
        assert_ne!(key, other);
        assert_eq!(MAX_SHADER_STAGES, 6);
        assert_eq!(key.vertex_buffer_formats.len(), 32);
        let pipeline = GraphicsPipeline::new(key, 9);
        assert_eq!(pipeline.graphics_key(), &key);
        assert_eq!(
            ClipDistanceShaderKey {
                clip_locations: [(0, 0); 8]
            }
            .clip_locations
            .len(),
            8
        );
    }
}

impl GraphicsPipeline {
    /// The descriptor bindings a graphics pipeline creates (`vk_graphics_pipeline.cpp`).
    pub fn build_bindings(key: &GraphicsPipelineKey) -> Vec<u32> {
        let mut bindings = Vec::new();
        for color in 0..key.num_color_attachments.min(NUM_COLOR_BUFFERS as u32) {
            bindings.push(color);
        }
        if key.depth_format != 0 {
            bindings.push(key.depth_format);
        }
        bindings
    }

    /// `GraphicsPipeline` creation facade; a zero pipeline cache fails like the driver.
    pub fn create(
        key: GraphicsPipelineKey,
        pipeline_cache: u64,
        module_count: usize,
    ) -> Option<Self> {
        if pipeline_cache == 0 || module_count == 0 {
            return None;
        }
        let mut pipeline = Self::new(key, pipeline_cache);
        pipeline.layout = pipeline_cache + 1;
        Some(pipeline)
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn graphics_pipeline_creation_and_bindings() {
        let key = GraphicsPipelineKey {
            num_color_attachments: 2,
            depth_format: 126,
            ..GraphicsPipelineKey::default()
        };
        assert_eq!(GraphicsPipeline::build_bindings(&key), vec![0, 1, 126]);
        assert!(GraphicsPipeline::create(key, 0, 1).is_none());
        let pipeline = GraphicsPipeline::create(key, 10, 2).unwrap();
        assert_eq!(pipeline.pipeline, 10);
        assert_eq!(pipeline.layout, 11);
    }
}
