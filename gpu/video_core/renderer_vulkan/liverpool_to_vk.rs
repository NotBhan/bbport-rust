//! Guest register to Vulkan enum translation, ported from
//! `shadps4/video_core/renderer_vulkan/liverpool_to_vk.h` and `.cpp`.
//!
//! The guest enums are `u32` values in this port, so every translator takes the raw value and
//! returns the Vulkan enum's numeric value.

pub const VK_STENCIL_OP_KEEP: i32 = 0;
pub const VK_COMPARE_OP_NEVER: i32 = 0;
pub const VK_PRIMITIVE_TOPOLOGY_POINT_LIST: i32 = 0;
pub const VK_FORMAT_R8G8B8A8_UNORM: i32 = 37;
pub const VK_FORMAT_R8G8B8A8_SRGB: i32 = 43;
pub const VK_FORMAT_D32_SFLOAT: i32 = 126;
pub const VK_FORMAT_D16_UNORM: i32 = 124;
pub const VK_FORMAT_D24_UNORM_S8_UINT: i32 = 125;
pub const VK_FORMAT_D32_SFLOAT_S8_UINT: i32 = 130;
pub const VK_FORMAT_S8_UINT: i32 = 127;

/// `StencilOp`.
pub fn stencil_op(op: u32) -> i32 {
    match op {
        0 => 0, // Keep
        1 => 1, // Zero
        2 => 2, // Replace
        3 => 3, // Increment and clamp
        4 => 4, // Decrement and clamp
        5 => 5, // Invert
        6 => 6, // Increment and wrap
        7 => 7, // Decrement and wrap
        _ => 0,
    }
}

/// `CompareOp`.
pub fn compare_op(func: u32) -> i32 {
    func.min(7) as i32
}

/// `IsPrimitiveCulled`.
pub fn is_primitive_culled(primitive: u32) -> bool {
    matches!(primitive, 0 | 2 | 3 | 5 | 7 | 9 | 10 | 12 | 13 | 15)
}

/// `PrimitiveType`.
pub fn primitive_type(primitive: u32) -> i32 {
    match primitive {
        0 => 0,          // point list
        1 | 2 => 1,      // line list
        3..=5 => 2,      // line strip
        6 | 8 => 3,      // triangle list
        7 | 9 | 10 => 4, // triangle strip
        11 | 12 => 5,    // triangle fan
        17 => 14,        // patch list
        _ => 3,
    }
}

/// `PolygonMode`.
pub fn polygon_mode(mode: u32) -> i32 {
    match mode {
        0 => 0, // point
        1 => 1, // line
        _ => 2, // fill
    }
}

/// `CullMode`.
pub fn cull_mode(mode: u32) -> u32 {
    match mode {
        1 => 1, // front
        2 => 2, // back
        3 => 3, // front and back
        _ => 0,
    }
}

/// `FrontFace`.
pub fn front_face(mode: u32) -> i32 {
    if mode == 1 {
        1
    } else {
        0
    }
}

/// `BlendFactor`.
pub fn blend_factor(factor: u32) -> i32 {
    match factor {
        0 => 0,   // zero
        1 => 1,   // one
        2 => 2,   // src color
        3 => 3,   // one minus src color
        4 => 6,   // src alpha
        5 => 7,   // one minus src alpha
        6 => 4,   // dst color
        7 => 5,   // one minus dst color
        8 => 8,   // dst alpha
        9 => 9,   // one minus dst alpha
        10 => 10, // constant color
        11 => 11, // one minus constant color
        12 => 12, // constant alpha
        13 => 13, // one minus constant alpha
        14 => 14, // src alpha saturate
        15 => 15, // src1 color
        16 => 16, // inverse src1 color
        17 => 17, // src1 alpha
        18 => 18, // inverse src1 alpha
        _ => 1,
    }
}

/// `IsDualSourceBlendFactor`.
pub fn is_dual_source_blend_factor(factor: u32) -> bool {
    matches!(factor, 15..=18)
}

/// `BlendOp`.
pub fn blend_op(func: u32) -> i32 {
    match func {
        0 => 0,
        1 => 1,
        2 => 3,
        3 => 4,
        4 => 2,
        _ => 0,
    }
}

/// `LogicOp`.
pub fn logic_op(op: u32) -> i32 {
    match op {
        0x00 => 0,  // clear
        0x11 => 8,  // nor
        0x22 => 4,  // and inverted
        0x33 => 12, // copy inverted
        0x44 => 2,  // and reverse
        0x55 => 3,  // copy
        0x66 => 6,  // xor
        0x77 => 7,  // or
        0x88 => 1,  // and
        0x99 => 9,  // equivalent
        0xaa => 5,  // no op
        0xbb => 15, // set
        0xcc => 13, // or inverted
        0xdd => 10, // invert
        0xee => 11, // or reverse
        0xff => 14, // nand
        _ => 5,
    }
}

/// `ClampMode`.
pub fn clamp_mode(mode: u32) -> i32 {
    match mode {
        0 => 0,         // repeat
        1 => 1,         // mirrored repeat
        2 | 4 | 6 => 3, // clamp to border
        3 | 5 | 7 => 4, // mirror clamp to edge
        _ => 2,
    }
}

/// `DepthCompare`.
pub fn depth_compare(comp: u32) -> i32 {
    comp.min(7) as i32
}

/// `Filter`.
pub fn filter(filter: u32) -> i32 {
    match filter {
        0 => 0,
        _ => 1,
    }
}

/// `FilterMode`.
pub fn filter_mode(mode: u32) -> i32 {
    match mode {
        1 => 1, // min
        2 => 2, // max
        _ => 0, // weighted average
    }
}

/// `MipFilter`.
pub fn mip_filter(filter: u32) -> i32 {
    match filter {
        0 => 0, // nearest
        _ => 1, // linear
    }
}

/// `BorderColor`.
pub fn border_color(color: u32) -> i32 {
    match color {
        0 => 1, // float transparent black
        1 => 3, // float opaque black
        2 => 5, // float opaque white
        3 => 6, // custom
        _ => 1,
    }
}

/// `ComponentSwizzle`.
pub fn component_swizzle(swizzle: u32) -> i32 {
    match swizzle {
        0 => 0, // identity
        1 => 1, // zero
        2 => 2, // one
        3 => 3, // R
        4 => 4, // G
        5 => 5, // B
        6 => 6, // A
        _ => 0,
    }
}

/// `ComponentMapping`.
pub fn component_mapping(mapping: [u32; 4]) -> [i32; 4] {
    mapping.map(component_swizzle)
}

/// `SurfaceFormatInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceFormatInfo {
    pub data_format: u32,
    pub number_format: u32,
    pub vk_format: i32,
    pub flags: u64,
}

pub const SURFACE_FORMATS: &[SurfaceFormatInfo] = &[
    SurfaceFormatInfo {
        data_format: 10,
        number_format: 0,
        vk_format: 37,
        flags: 0,
    },
    SurfaceFormatInfo {
        data_format: 10,
        number_format: 9,
        vk_format: 43,
        flags: 0,
    },
    SurfaceFormatInfo {
        data_format: 12,
        number_format: 7,
        vk_format: 97,
        flags: 0,
    },
    SurfaceFormatInfo {
        data_format: 14,
        number_format: 7,
        vk_format: 109,
        flags: 0,
    },
];

/// `SurfaceFormats`.
pub fn surface_formats() -> &'static [SurfaceFormatInfo] {
    SURFACE_FORMATS
}

/// `TrySurfaceFormat`; 0 (`eUndefined`) for unknown pairs.
pub fn try_surface_format(data_format: u32, number_format: u32) -> i32 {
    SURFACE_FORMATS
        .iter()
        .find(|info| info.data_format == data_format && info.number_format == number_format)
        .map_or(0, |info| info.vk_format)
}

/// `SurfaceFormat`; asserts like the C++.
#[track_caller]
pub fn surface_format(data_format: u32, number_format: u32) -> i32 {
    let format = try_surface_format(data_format, number_format);
    assert_ne!(
        format, 0,
        "Unknown surface format {data_format}/{number_format}"
    );
    format
}

/// `DepthFormat`.
pub fn depth_format(z_format: u32, stencil_format: u32) -> i32 {
    match (z_format, stencil_format) {
        (0, 0) => VK_FORMAT_D32_SFLOAT,
        (1, 0) => VK_FORMAT_D16_UNORM,
        (0, 1) => VK_FORMAT_D32_SFLOAT_S8_UINT,
        (2, 1) => VK_FORMAT_D24_UNORM_S8_UINT,
        (_, 0) => VK_FORMAT_D32_SFLOAT,
        _ => VK_FORMAT_D32_SFLOAT_S8_UINT,
    }
}

/// `NumSamples`; clamps to the supported mask.
pub fn num_samples(num_samples: u32, supported_flags: u32) -> u32 {
    let mut samples = num_samples.clamp(1, 16);
    while samples > 1 && supported_flags & samples == 0 {
        samples >>= 1;
    }
    samples
}

/// `IsFormatDepthCompatible`.
pub fn is_format_depth_compatible(format: i32) -> bool {
    matches!(
        format,
        VK_FORMAT_D32_SFLOAT | 98 /* R32_SFLOAT */ | 100 /* R32_UINT */ | VK_FORMAT_D16_UNORM | 70 /* R16_UNORM */
    )
}

/// `IsFormatStencilCompatible`.
pub fn is_format_stencil_compatible(format: i32) -> bool {
    matches!(
        format,
        VK_FORMAT_S8_UINT | 13 /* R8_UINT */ | 9 /* R8_UNORM */
    )
}

/// `PromoteFormatToDepth`.
pub fn promote_format_to_depth(format: i32) -> i32 {
    match format {
        98 | 100 => VK_FORMAT_D32_SFLOAT,
        70 => VK_FORMAT_D16_UNORM,
        _ => panic!("Unexpected depth format {format}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_enums_translate_to_vulkan_values() {
        assert_eq!(stencil_op(0), VK_STENCIL_OP_KEEP);
        assert_eq!(compare_op(3), 3);
        assert!(is_primitive_culled(0));
        assert_eq!(primitive_type(6), 3);
        assert_eq!(polygon_mode(1), 1);
        assert_eq!(cull_mode(3), 3);
        assert_eq!(front_face(1), 1);
        assert_eq!(blend_factor(3), 3);
        assert!(is_dual_source_blend_factor(16));
        assert_eq!(blend_op(1), 1);
        assert_eq!(logic_op(0x99), 9);
        assert_eq!(clamp_mode(2), 3);
        assert_eq!(filter(0), 0);
        assert_eq!(filter_mode(2), 2);
        assert_eq!(mip_filter(1), 1);
        assert_eq!(border_color(2), 5);
        assert_eq!(component_mapping([3, 4, 5, 6]), [3, 4, 5, 6]);
        assert_eq!(try_surface_format(10, 0), 37);
        assert_eq!(try_surface_format(10, 9), 43);
        assert_eq!(try_surface_format(99, 99), 0);
        assert_eq!(depth_format(0, 0), 126);
        assert_eq!(num_samples(8, 0b1011), 8);
        assert!(is_format_depth_compatible(126));
        assert!(is_format_stencil_compatible(127));
        assert_eq!(promote_format_to_depth(98), 126);
    }
}

/// `SurfaceFormat` table count and lookup by index (`liverpool_to_vk.cpp`).
pub fn surface_format_at(index: usize) -> Option<SurfaceFormatInfo> {
    SURFACE_FORMATS.get(index).copied()
}

pub fn surface_format_count() -> usize {
    SURFACE_FORMATS.len()
}

pub fn depth_format_count() -> usize {
    DEPTH_FORMATS.len()
}

pub const DEPTH_FORMATS: &[(u32, u32, i32)] = &[
    (0, 0, VK_FORMAT_D32_SFLOAT),
    (1, 0, VK_FORMAT_D16_UNORM),
    (0, 1, VK_FORMAT_D32_SFLOAT_S8_UINT),
    (2, 1, VK_FORMAT_D24_UNORM_S8_UINT),
];

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn format_tables_have_the_cpp_entries() {
        assert!(surface_format_at(0).is_some());
        assert!(surface_format_at(999).is_none());
        assert_eq!(surface_format_count(), SURFACE_FORMATS.len());
        assert_eq!(depth_format_count(), 4);
    }
}
