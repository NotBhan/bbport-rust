//! Primitive and viewport registers, ported from
//! `legacy/gpu/shadps4/video_core/amdgpu/regs_primitive.h`.

#![allow(non_upper_case_globals)]

/// `AmdGpu::NUM_VIEWPORTS`.
pub const NUM_VIEWPORTS: usize = 16;
/// `AmdGpu::NUM_CLIP_PLANES`.
pub const NUM_CLIP_PLANES: usize = 6;

/// `AmdGpu::ClipSpace`.
pub mod clip_space {
    pub const MinusWToW: u32 = 0;
    pub const ZeroToW: u32 = 1;
}

/// `AmdGpu::PrimKillCond`.
pub mod prim_kill_cond {
    pub const AllVtx: u32 = 0;
    pub const AnyVtx: u32 = 1;
}

/// `AmdGpu::PolygonMode`.
pub mod polygon_mode {
    pub const Point: u32 = 0;
    pub const Line: u32 = 1;
    pub const Fill: u32 = 2;
}

/// `AmdGpu::ProvokingVtxLast`.
pub mod provoking_vtx_last {
    pub const First: u32 = 0;
    pub const Last: u32 = 1;
}

/// `AmdGpu::CullMode`.
pub mod cull_mode {
    pub const None: u32 = 0;
    pub const Front: u32 = 1;
    pub const Back: u32 = 2;
    pub const FrontAndBack: u32 = 3;
}

/// `AmdGpu::FrontFace`.
pub mod front_face {
    pub const CounterClockwise: u32 = 0;
    pub const Clockwise: u32 = 1;
}

macro_rules! bitfield {
    ($name:ident { $($field:ident : $offset:expr, $bits:expr),* $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        #[repr(transparent)]
        pub struct $name(pub u32);

        impl $name {
            pub const fn from_raw(raw: u32) -> Self {
                Self(raw)
            }

            pub const fn raw(self) -> u32 {
                self.0
            }

            $(
                pub fn $field(self) -> u32 {
                    (self.0 >> $offset) & ((1u32 << $bits) - 1)
                }
            )*
        }
    };
}

bitfield!(ClipperControl {
    user_clip_plane_enable: 0, 6,
    clip_disable: 16, 1,
    clip_space: 19, 1,
    vtx_kill_or: 21, 1,
    dx_rasterization_kill: 22, 1,
    dx_linear_attr_clip_enable: 24, 1,
    zclip_near_disable: 26, 1,
    zclip_far_disable: 27, 1,
});

impl ClipperControl {
    pub fn zclip_enable(self) -> bool {
        if self.zclip_near_disable() != self.zclip_far_disable() {
            return false;
        }
        self.zclip_near_disable() == 0
    }
}

bitfield!(PolygonControl {
    cull_front: 0, 1,
    cull_back: 1, 1,
    front_face: 2, 1,
    enable_polygon_mode: 3, 2,
    polygon_mode_front: 5, 3,
    polygon_mode_back: 8, 3,
    enable_polygon_offset_front: 11, 1,
    enable_polygon_offset_back: 12, 1,
    enable_polygon_offset_para: 13, 1,
    enable_window_offset: 16, 1,
    provoking_vtx_last: 19, 1,
    persp_corr_dis: 20, 1,
    multi_prim_ib_ena: 21, 1,
});

impl PolygonControl {
    pub fn poly_mode(self) -> u32 {
        if self.enable_polygon_mode() != 0 {
            self.polygon_mode_front()
        } else {
            polygon_mode::Fill
        }
    }

    pub fn culling_mode(self) -> u32 {
        self.cull_front() | (self.cull_back() << 1)
    }

    pub fn needs_bias(self) -> bool {
        self.enable_polygon_offset_back() != 0
            || self.enable_polygon_offset_front() != 0
            || self.enable_polygon_offset_para() != 0
    }
}

bitfield!(VsOutputControl {
    clip_distance_enable: 0, 8,
    cull_distance_enable: 8, 8,
    use_vtx_point_size: 16, 1,
    use_vtx_edge_flag: 17, 1,
    use_vtx_render_target_idx: 18, 1,
    use_vtx_viewport_idx: 19, 1,
    use_vtx_kill_flag: 20, 1,
    vs_out_misc_enable: 21, 1,
    vs_out_ccdist0_enable: 22, 1,
    vs_out_ccdist1_enable: 23, 1,
    vs_out_misc_side_bus_ena: 24, 1,
    use_vtx_gs_cut_flag: 25, 1,
});

impl VsOutputControl {
    pub fn is_clip_dist_enabled(self, index: u32) -> bool {
        (self.clip_distance_enable() >> index) & 1 != 0
    }

    pub fn is_cull_dist_enabled(self, index: u32) -> bool {
        (self.cull_distance_enable() >> index) & 1 != 0
    }
}

/// `AmdGpu::LineControl`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct LineControl {
    pub width_fixed_point: u32,
}

impl LineControl {
    pub fn width(&self) -> f32 {
        self.width_fixed_point as f32 / 8.0
    }
}

bitfield!(ModeControl {
    msaa_enable: 0, 1,
    vport_scissor_enable: 1, 1,
    line_stripple_enable: 2, 1,
    send_unlit_stiles_to_pkr: 3, 1,
});

/// `AmdGpu::Scissor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Scissor {
    pub top_left_x: i16,
    pub top_left_y: i16,
    pub bottom_right_x: i16,
    pub bottom_right_y: i16,
}

impl Scissor {
    pub fn clamp(value: i16) -> u16 {
        value.max(0) as u16
    }

    pub fn width(&self) -> u32 {
        (i32::from(Self::clamp(self.bottom_right_x)) - i32::from(Self::clamp(self.top_left_x)))
            as u32
    }

    pub fn height(&self) -> u32 {
        (i32::from(Self::clamp(self.bottom_right_y)) - i32::from(Self::clamp(self.top_left_y)))
            as u32
    }
}

/// `AmdGpu::WindowOffset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct WindowOffset(pub u32);

impl WindowOffset {
    pub fn window_x_offset(self) -> i32 {
        (self.0 as u16 as i16) as i32
    }

    pub fn window_y_offset(self) -> i32 {
        ((self.0 >> 16) as u16 as i16) as i32
    }
}

/// `AmdGpu::ViewportScissor`, two packed words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct ViewportScissor {
    pub top_left: u32,
    pub bottom_right: u32,
}

impl ViewportScissor {
    fn top_left_x(self) -> u32 {
        self.top_left & 0x7fff
    }

    fn top_left_y(self) -> u32 {
        (self.top_left >> 15) & 0x7fff
    }

    pub fn window_offset_disable(self) -> bool {
        (self.top_left >> 30) & 1 != 0
    }

    fn bottom_right_x(self) -> u32 {
        self.bottom_right & 0x7fff
    }

    fn bottom_right_y(self) -> u32 {
        (self.bottom_right >> 15) & 0x7fff
    }

    pub fn width(self) -> u32 {
        self.bottom_right_x().saturating_sub(self.top_left_x())
    }

    pub fn height(self) -> u32 {
        self.bottom_right_y().saturating_sub(self.top_left_y())
    }
}

/// `AmdGpu::ViewportDepth`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct ViewportDepth {
    pub zmin: f32,
    pub zmax: f32,
}

/// `AmdGpu::ViewportBounds`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct ViewportBounds {
    pub xscale: f32,
    pub xoffset: f32,
    pub yscale: f32,
    pub yoffset: f32,
    pub zscale: f32,
    pub zoffset: f32,
}

bitfield!(ViewportControl {
    xscale_enable: 0, 1,
    xoffset_enable: 1, 1,
    yscale_enable: 2, 1,
    yoffset_enable: 3, 1,
    zscale_enable: 4, 1,
    zoffset_enable: 5, 1,
    xy_transformed: 8, 1,
    z_transformed: 9, 1,
    w_transformed: 10, 1,
    perfcounter_ref: 11, 1,
});

/// `AmdGpu::ClipUserData`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct ClipUserData {
    pub data_x: u32,
    pub data_y: u32,
    pub data_z: u32,
    pub data_w: u32,
}

bitfield!(AaConfig {
    msaa_num_samples: 0, 3,
    aa_mask_centroid_dtmn: 4, 1,
    max_sample_dst: 13, 4,
    msaa_exposed_samples: 20, 3,
    detail_to_exposed_mode: 24, 2,
});

impl AaConfig {
    pub fn num_samples(self) -> u32 {
        1 << self.msaa_num_samples()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polygon_control_decodes_its_modes() {
        let control = PolygonControl::from_raw(1 | (1 << 1) | (1 << 3) | (1 << 5));
        assert_eq!(control.culling_mode(), cull_mode::FrontAndBack);
        assert_eq!(control.poly_mode(), 1);
        assert!(!control.needs_bias());
        assert!(PolygonControl::from_raw(1 << 13).needs_bias());
    }

    #[test]
    fn the_clipper_needs_both_zclip_disables_to_match() {
        assert!(ClipperControl::default().zclip_enable());
        assert!(!ClipperControl::from_raw(1 << 26).zclip_enable());
        assert!(!ClipperControl::from_raw((1 << 26) | (1 << 27)).zclip_enable());
    }

    #[test]
    fn a_viewport_scissor_keeps_its_fifteen_bit_fields() {
        let scissor = ViewportScissor {
            top_left: 2 | (3 << 15),
            bottom_right: 10 | (12 << 15),
        };
        assert_eq!(scissor.width(), 8);
        assert_eq!(scissor.height(), 9);
        assert!(!scissor.window_offset_disable());
    }
}
