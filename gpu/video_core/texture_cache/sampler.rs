//! Sampler handles, ported from `shadps4/video_core/texture_cache/sampler.h`.

/// The sampler's Vulkan handle, abstracted from `vk::Sampler`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SamplerHandle(pub u64);

/// `VideoCore::Sampler`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sampler {
    pub handle: SamplerHandle,
    pub lru_id: usize,
}

impl Sampler {
    pub fn new(handle: SamplerHandle) -> Self {
        Self { handle, lru_id: 0 }
    }

    /// `Handle`.
    pub fn handle(&self) -> SamplerHandle {
        self.handle
    }
}

/// The sampler's state derived from the guest descriptor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SamplerParams {
    pub is_depth: bool,
    pub extra_lod_bias: f32,
}

impl Default for SamplerParams {
    fn default() -> Self {
        Self {
            is_depth: false,
            extra_lod_bias: 0.0,
        }
    }
}

/// The device-side factory; the Vulkan instance implements it.
pub trait SamplerFactory {
    fn create_sampler(&mut self, params: &SamplerParams, border_color_base: u64) -> SamplerHandle;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sampler_handles_keep_their_lru_id() {
        let mut sampler = Sampler::new(SamplerHandle(0x1234));
        assert_eq!(sampler.handle(), SamplerHandle(0x1234));
        assert_eq!(sampler.lru_id, 0);
        sampler.lru_id = 7;
        assert_eq!(sampler.lru_id, 7);
        assert_eq!(SamplerParams::default().extra_lod_bias, 0.0);
    }
}

/// The Vulkan sampler creation parameters derived from the guest descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct SamplerInfo {
    pub mag_filter: i32,
    pub min_filter: i32,
    pub mipmap_mode: i32,
    pub address_mode_u: i32,
    pub address_mode_v: i32,
    pub address_mode_w: i32,
    pub mip_lod_bias: f32,
    pub anisotropy_enable: bool,
    pub max_anisotropy: f32,
    pub compare_enable: bool,
    pub compare_op: i32,
    pub min_lod: f32,
    pub max_lod: f32,
    pub custom_border_color: bool,
    pub custom_border_color_index: u64,
    pub unnormalized_coordinates: bool,
}

impl SamplerInfo {
    /// `Sampler::Sampler`; filters and clamp modes are kept as their guest enum raw values.
    pub fn from_guest(
        sampler: &crate::gpu::amdgpu::resource::Sampler,
        max_lod_bias: f32,
        max_anisotropy_supported: f32,
        anisotropy_supported: bool,
        is_depth: bool,
        extra_lod_bias: f32,
    ) -> Self {
        let anisotropy_enable = anisotropy_supported
            && (crate::gpu::amdgpu::resource::is_aniso_filter(sampler.xy_mag_filter())
                || crate::gpu::amdgpu::resource::is_aniso_filter(sampler.xy_min_filter()));
        let max_anisotropy = if anisotropy_enable {
            sampler.get_max_aniso().clamp(1.0, max_anisotropy_supported)
        } else {
            1.0
        };
        let lod_bias = (sampler.get_lod_bias() + extra_lod_bias).clamp(-max_lod_bias, max_lod_bias);
        Self {
            mag_filter: sampler.xy_mag_filter() as i32,
            min_filter: sampler.xy_min_filter() as i32,
            mipmap_mode: sampler.mip_filter() as i32,
            address_mode_u: sampler.clamp_x() as i32,
            address_mode_v: sampler.clamp_y() as i32,
            address_mode_w: sampler.clamp_z() as i32,
            mip_lod_bias: lod_bias,
            anisotropy_enable,
            max_anisotropy,
            compare_enable: is_depth,
            compare_op: sampler.depth_compare_func() as i32,
            min_lod: sampler.get_min_lod(),
            max_lod: sampler.get_max_lod(),
            custom_border_color: false,
            custom_border_color_index: sampler.border_color_ptr(),
            unnormalized_coordinates: false,
        }
    }
}

#[cfg(test)]
mod sampler_info_tests {
    use super::*;
    use crate::gpu::amdgpu::resource::Sampler as GuestSampler;

    #[test]
    fn sampler_info_clamps_lod_bias_and_anisotropy() {
        let guest = GuestSampler::default();
        let info = SamplerInfo::from_guest(&guest, 2.0, 8.0, true, true, 0.5);
        assert_eq!(info.mip_lod_bias, 0.5);
        assert_eq!(info.max_anisotropy, 1.0);
        assert!(info.compare_enable);
        assert!(!info.unnormalized_coordinates);
    }
}
