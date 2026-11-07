//! The shader compiler's profile, ported from
//! `legacy/gpu/shadps4/shader_recompiler/profile.h`: what the host's backend can do, which the
//! compiler reads to pick its SPIR-V features. The header's default member initialisers are the
//! struct's `Default`, of which only the SPIR-V version starts non-zero.

/// `Shader::Profile`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub max_viewport_width: u32,
    pub max_viewport_height: u32,
    pub max_shared_memory_size: u32,
    pub supported_spirv: u32,
    pub subgroup_size: u32,
    pub sparse_page_shift: u32,
    pub support_int8: bool,
    pub support_int16: bool,
    pub support_int64: bool,
    pub support_float16: bool,
    pub support_float64: bool,
    pub supports_denorm_behavior_independence: bool,
    pub supports_rounding_mode_independence: bool,
    pub support_fp16_denorm_preserve: bool,
    pub support_fp16_denorm_flush: bool,
    pub support_fp16_round_to_zero: bool,
    pub support_fp32_denorm_preserve: bool,
    pub support_fp32_denorm_flush: bool,
    pub support_fp32_round_to_zero: bool,
    pub support_fp64_denorm_preserve: bool,
    pub support_fp64_denorm_flush: bool,
    pub support_fp64_round_to_zero: bool,
    pub support_fp16_signed_zero_inf_nan_preserve: bool,
    pub support_fp32_signed_zero_inf_nan_preserve: bool,
    pub support_fp64_signed_zero_inf_nan_preserve: bool,
    pub supports_image_load_store_lod: bool,
    pub supports_native_cube_calc: bool,
    pub supports_trinary_minmax: bool,
    pub supports_buffer_fp32_atomic_min_max: bool,
    pub supports_image_fp32_atomic_min_max: bool,
    pub supports_buffer_int64_atomics: bool,
    pub supports_shared_int64_atomics: bool,
    pub supports_workgroup_explicit_memory_layout: bool,
    pub supports_amd_shader_explicit_vertex_parameter: bool,
    pub supports_fragment_shader_barycentric: bool,
    pub supports_shader_subgroup_clock: bool,
    pub has_broken_spirv_clamp: bool,
    pub lower_left_origin_mode: bool,
    pub needs_manual_interpolation: bool,
    /// bbport: NVIDIA without fragment shader barycentrics (Pascal and older).
    pub needs_integer_interpolation_fix: bool,
    pub needs_lds_barriers: bool,
    pub needs_buffer_offsets: bool,
    pub needs_unorm_fixup: bool,
    pub needs_clip_distance_emulation: bool,
    pub supports_shader_stencil_export: bool,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            max_viewport_width: 0,
            max_viewport_height: 0,
            max_shared_memory_size: 0,
            supported_spirv: 0x0001_0000,
            subgroup_size: 0,
            sparse_page_shift: 0,
            support_int8: false,
            support_int16: false,
            support_int64: false,
            support_float16: false,
            support_float64: false,
            supports_denorm_behavior_independence: false,
            supports_rounding_mode_independence: false,
            support_fp16_denorm_preserve: false,
            support_fp16_denorm_flush: false,
            support_fp16_round_to_zero: false,
            support_fp32_denorm_preserve: false,
            support_fp32_denorm_flush: false,
            support_fp32_round_to_zero: false,
            support_fp64_denorm_preserve: false,
            support_fp64_denorm_flush: false,
            support_fp64_round_to_zero: false,
            support_fp16_signed_zero_inf_nan_preserve: false,
            support_fp32_signed_zero_inf_nan_preserve: false,
            support_fp64_signed_zero_inf_nan_preserve: false,
            supports_image_load_store_lod: false,
            supports_native_cube_calc: false,
            supports_trinary_minmax: false,
            supports_buffer_fp32_atomic_min_max: false,
            supports_image_fp32_atomic_min_max: false,
            supports_buffer_int64_atomics: false,
            supports_shared_int64_atomics: false,
            supports_workgroup_explicit_memory_layout: false,
            supports_amd_shader_explicit_vertex_parameter: false,
            supports_fragment_shader_barycentric: false,
            supports_shader_subgroup_clock: false,
            has_broken_spirv_clamp: false,
            lower_left_origin_mode: false,
            needs_manual_interpolation: false,
            needs_integer_interpolation_fix: false,
            needs_lds_barriers: false,
            needs_buffer_offsets: false,
            needs_unorm_fixup: false,
            needs_clip_distance_emulation: false,
            supports_shader_stencil_export: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_spirv_version_starts_non_zero() {
        let profile = Profile::default();
        assert_eq!(profile.supported_spirv, 0x0001_0000, "SPIR-V 1.0");
        assert_eq!(profile.max_viewport_width, 0);
        assert_eq!(profile.max_viewport_height, 0);
        assert_eq!(profile.max_shared_memory_size, 0);
        assert_eq!(profile.subgroup_size, 0);
        assert_eq!(profile.sparse_page_shift, 0);
        assert!(!profile.support_int8 && !profile.support_float64);
        assert!(!profile.has_broken_spirv_clamp);
        assert!(!profile.needs_clip_distance_emulation);
        // The C++ gives the struct a defaulted `operator==`, so two defaults are equal.
        assert_eq!(profile, Profile::default());
        let mut other = profile;
        other.subgroup_size = 64;
        assert_ne!(profile, other);
    }
}
