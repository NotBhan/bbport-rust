//! Compute pipeline keys, ported from
//! `shadps4/video_core/renderer_vulkan/vk_compute_pipeline.h`.

use std::hash::Hash;

use crate::gpu::video_core::renderer_vulkan::vk_common::{VkPipeline, VkShaderModule};

/// `ComputePipelineKey`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ComputePipelineKey {
    pub value: usize,
}

/// `ComputePipeline::SerializationSupport`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SerializationSupport {
    pub dummy: u32,
}

/// `ComputePipeline`.
#[derive(Debug, Clone, Copy)]
pub struct ComputePipeline {
    pub compute_key: ComputePipelineKey,
    pub pipeline: VkPipeline,
    pub module: VkShaderModule,
    pub preloading: bool,
}

impl ComputePipeline {
    pub fn new(
        pipeline_cache: u64,
        compute_key: ComputePipelineKey,
        module: VkShaderModule,
        preloading: bool,
    ) -> Self {
        Self {
            compute_key,
            pipeline: pipeline_cache,
            module,
            preloading,
        }
    }

    pub fn key(&self) -> ComputePipelineKey {
        self.compute_key
    }

    /// The descriptor bindings a compute pipeline creates, in binding order.
    pub fn build_bindings(
        buffers: u32,
        images: &[(u32, bool)],
        samplers: u32,
    ) -> Vec<ComputeBinding> {
        let mut bindings = Vec::new();
        let mut binding = 0;
        for _ in 0..buffers {
            bindings.push(ComputeBinding {
                binding,
                descriptor_type: ComputeDescriptorType::StorageBuffer,
                count: 1,
            });
            binding += 1;
        }
        for &(count, written) in images {
            bindings.push(ComputeBinding {
                binding,
                descriptor_type: if written {
                    ComputeDescriptorType::StorageImage
                } else {
                    ComputeDescriptorType::SampledImage
                },
                count,
            });
            binding += count;
        }
        for _ in 0..samplers {
            bindings.push(ComputeBinding {
                binding,
                descriptor_type: ComputeDescriptorType::Sampler,
                count: 1,
            });
            binding += 1;
        }
        bindings
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeDescriptorType {
    StorageBuffer,
    StorageImage,
    SampledImage,
    Sampler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComputeBinding {
    pub binding: u32,
    pub descriptor_type: ComputeDescriptorType,
    pub count: u32,
}

impl ComputePipelineKey {
    /// `Serialize`; the key is its value.
    pub fn serialize(self) -> [u8; core::mem::size_of::<usize>()] {
        self.value.to_ne_bytes()
    }

    /// `Deserialize`.
    pub fn deserialize(bytes: [u8; core::mem::size_of::<usize>()]) -> Self {
        Self {
            value: usize::from_ne_bytes(bytes),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

    #[test]
    fn the_compute_key_hashes_and_compares_by_value() {
        let key = ComputePipelineKey { value: 7 };
        assert_eq!(key, ComputePipelineKey { value: 7 });
        assert_ne!(key, ComputePipelineKey { value: 8 });
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        assert_ne!(hasher.finish(), 0);
        let pipeline = ComputePipeline::new(9, key, 10, false);
        assert_eq!(pipeline.key(), key);
        assert_eq!(SerializationSupport::default().dummy, 0);
        assert_eq!(ComputePipelineKey::deserialize(key.serialize()), key);
        let bindings = ComputePipeline::build_bindings(1, &[(2, true)], 1);
        assert_eq!(bindings.len(), 3);
        assert_eq!(bindings[1].binding, 1);
        assert_eq!(bindings[2].binding, 3);
    }
}
