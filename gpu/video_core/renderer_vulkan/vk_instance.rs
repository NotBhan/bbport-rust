//! Vulkan instance/device information, ported from
//! `shadps4/video_core/renderer_vulkan/vk_instance.h`.

/// The device features the renderer reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeviceFeatures {
    pub subgroup_size_64: bool,
    pub anisotropic_filtering: bool,
    pub custom_border_color: bool,
    pub storage_image_extended_usages: bool,
}

/// The device limits the renderer reads.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DeviceLimits {
    pub max_sampler_anisotropy: f32,
    pub max_sampler_lod_bias: f32,
    pub max_push_descriptors: u32,
    pub storage_min_alignment: u64,
}

/// `Instance` state; ash owns the real handles.
#[derive(Debug, Default)]
pub struct Instance {
    pub features: DeviceFeatures,
    pub limits: DeviceLimits,
    pub device_name: String,
    pub vendor_id: u32,
    pub api_version: u32,
}

impl Instance {
    pub fn is_subgroup_size_64_supported(&self) -> bool {
        self.features.subgroup_size_64
    }

    pub fn is_anisotropic_filtering_supported(&self) -> bool {
        self.features.anisotropic_filtering
    }

    pub fn is_custom_border_color_supported(&self) -> bool {
        self.features.custom_border_color
    }

    pub fn max_sampler_anisotropy(&self) -> f32 {
        self.limits.max_sampler_anisotropy
    }

    pub fn max_sampler_lod_bias(&self) -> f32 {
        self.limits.max_sampler_lod_bias
    }

    pub fn max_push_descriptors(&self) -> u32 {
        self.limits.max_push_descriptors
    }

    pub fn storage_min_alignment(&self) -> u64 {
        self.limits.storage_min_alignment
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_reports_features_and_limits() {
        let instance = Instance {
            features: DeviceFeatures {
                subgroup_size_64: true,
                anisotropic_filtering: true,
                ..DeviceFeatures::default()
            },
            limits: DeviceLimits {
                max_sampler_anisotropy: 16.0,
                max_push_descriptors: 32,
                ..DeviceLimits::default()
            },
            ..Instance::default()
        };
        assert!(instance.is_subgroup_size_64_supported());
        assert!(instance.is_anisotropic_filtering_supported());
        assert_eq!(instance.max_sampler_anisotropy(), 16.0);
        assert_eq!(instance.max_push_descriptors(), 32);
    }
}

/// `CollectExtensions`: checks every required extension is present.
pub fn collect_extensions(required: &[&str], available: &[&str]) -> Vec<String> {
    let mut missing = Vec::new();
    for name in required {
        if !available.contains(name) {
            missing.push((*name).to_string());
        }
    }
    missing
}

impl Instance {
    pub fn supports_extensions(&self, required: &[&str], available: &[&str]) -> bool {
        collect_extensions(required, available).is_empty()
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn instance_extension_collection_reports_missing() {
        assert!(collect_extensions(&["a", "b"], &["a", "b", "c"]).is_empty());
        assert_eq!(collect_extensions(&["a", "x"], &["a"]), vec!["x"]);
        assert!(Instance::default().supports_extensions(&[], &[]));
    }
}
