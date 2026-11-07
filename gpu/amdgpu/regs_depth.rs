//! Depth/stencil registers, ported from
//! `legacy/gpu/shadps4/video_core/amdgpu/regs_depth.h`.

#![allow(non_upper_case_globals)]

use crate::gpu::amdgpu::tiling::TileMode;

/// `AmdGpu::ZOrder`.
pub mod z_order {
    pub const LateZ: u32 = 0;
    pub const EarlyZLateZ: u32 = 1;
    pub const ReZ: u32 = 2;
    pub const EarlyZReZ: u32 = 3;
}

/// `AmdGpu::ConservativeDepth`.
pub mod conservative_depth {
    pub const Any: u32 = 0;
    pub const LessThanZ: u32 = 1;
    pub const GreaterThanZ: u32 = 2;
}

/// `AmdGpu::CompareFunc`.
pub mod compare_func {
    pub const Never: u32 = 0;
    pub const Less: u32 = 1;
    pub const Equal: u32 = 2;
    pub const LessEqual: u32 = 3;
    pub const Greater: u32 = 4;
    pub const NotEqual: u32 = 5;
    pub const GreaterEqual: u32 = 6;
    pub const Always: u32 = 7;
}

/// `AmdGpu::StencilFunc`.
pub mod stencil_func {
    pub const Keep: u32 = 0;
    pub const Zero: u32 = 1;
    pub const Ones: u32 = 2;
    pub const ReplaceTest: u32 = 3;
    pub const ReplaceOp: u32 = 4;
    pub const AddClamp: u32 = 5;
    pub const SubClamp: u32 = 6;
    pub const Invert: u32 = 7;
    pub const AddWrap: u32 = 8;
    pub const SubWrap: u32 = 9;
    pub const And: u32 = 10;
    pub const Or: u32 = 11;
    pub const Xor: u32 = 12;
    pub const Nand: u32 = 13;
    pub const Nor: u32 = 14;
    pub const Xnor: u32 = 15;
}

/// `AmdGpu::ForceEnable`.
pub mod force_enable {
    pub const Off: u32 = 0;
    pub const Enable: u32 = 1;
    pub const Disable: u32 = 2;
}

/// `AmdGpu::ForceSumm`.
pub mod force_summ {
    pub const Off: u32 = 0;
    pub const MinZ: u32 = 1;
    pub const MaxZ: u32 = 2;
    pub const Both: u32 = 3;
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

bitfield!(DepthShaderControl {
    z_export_enable: 0, 1,
    stencil_test_val_export_enable: 1, 1,
    stencil_op_val_export_enable: 2, 1,
    z_order: 4, 2,
    kill_enable: 6, 1,
    coverage_to_mask_enable: 7, 1,
    mask_export_enable: 8, 1,
    exec_on_hier_fail: 9, 1,
    exec_on_noop: 10, 1,
    alpha_to_mask_disable: 11, 1,
    depth_before_shader: 12, 1,
    conservative_z_export: 13, 2,
});

bitfield!(DepthControl {
    stencil_enable: 0, 1,
    depth_enable: 1, 1,
    depth_write_enable: 2, 1,
    depth_bounds_enable: 3, 1,
    depth_func: 4, 3,
    backface_enable: 7, 1,
    stencil_ref_func: 8, 3,
    stencil_bf_func: 20, 3,
    enable_color_writes_on_depth_fail: 30, 1,
    disable_color_writes_on_depth_pass: 31, 1,
});

bitfield!(StencilControl {
    stencil_fail_front: 0, 4,
    stencil_zpass_front: 4, 4,
    stencil_zfail_front: 8, 4,
    stencil_fail_back: 12, 4,
    stencil_zpass_back: 16, 4,
    stencil_zfail_back: 20, 4,
});

/// `AmdGpu::StencilRefMask`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct StencilRefMask {
    pub stencil_test_val: u8,
    pub stencil_mask: u8,
    pub stencil_write_mask: u8,
    pub stencil_op_val: u8,
}

bitfield!(DepthRenderControl {
    depth_clear_enable: 0, 1,
    stencil_clear_enable: 1, 1,
    depth_copy: 2, 1,
    stencil_copy: 3, 1,
    resummarize_enable: 4, 1,
    stencil_compress_disable: 5, 1,
    depth_compress_disable: 6, 1,
    copy_centroid: 7, 1,
    copy_sample: 8, 1,
    decompress_enable: 9, 1,
});

bitfield!(DepthView {
    slice_start: 0, 11,
    slice_max: 13, 11,
    z_read_only: 24, 1,
    stencil_read_only: 25, 1,
});

impl DepthView {
    pub fn num_slices(self) -> u32 {
        self.slice_max() + 1
    }
}

bitfield!(DepthRenderOverride {
    force_hiz_enable: 0, 2,
    force_his_enable0: 2, 2,
    force_his_enable1: 4, 2,
    force_shader_z_order: 6, 1,
    fast_z_disable: 7, 1,
    fast_stencil_disable: 8, 1,
    noop_cull_disable: 9, 1,
    force_color_kill: 10, 1,
    force_z_read: 11, 1,
    force_stencil_read: 12, 1,
    force_full_z_range: 13, 2,
    force_qc_smask_conflict: 15, 1,
    disable_viewport_clamp: 16, 1,
    ignore_sc_zrange: 17, 1,
    disable_fully_covered: 18, 1,
    force_z_limit_summ: 19, 2,
    max_tiles_in_dtt: 21, 5,
    disable_tile_rate_tiles: 26, 1,
    force_z_dirty: 27, 1,
    force_stencil_dirty: 28, 1,
    force_z_valid: 29, 1,
    force_stencil_valid: 30, 1,
    preserve_compression: 31, 1,
});

bitfield!(Eqaa {
    max_anchor_samples: 0, 1,
    ps_iter_samples: 4, 3,
    mask_export_num_samples: 8, 3,
    alpha_to_mask_num_samples: 12, 3,
    high_quality_intersections: 16, 1,
    incoherent_eqaa_reads: 17, 1,
    interpolate_comp_z: 18, 1,
    interpolate_src_z: 19, 1,
    static_anchor_associations: 20, 1,
    alpha_to_mask_eqaa_disable: 21, 1,
    overrasterization_amount: 24, 3,
    enable_postz_overrasterization: 27, 1,
});

/// `DepthBuffer::ZFormat`.
pub mod z_format {
    pub const Invalid: u32 = 0;
    pub const Z16: u32 = 1;
    pub const Z32Float: u32 = 3;
}

/// `DepthBuffer::StencilFormat`.
pub mod stencil_format {
    pub const Invalid: u32 = 0;
    pub const Stencil8: u32 = 1;
}

bitfield!(ZInfo {
    format: 0, 2,
    num_samples: 2, 2,
    tile_split: 13, 3,
    tile_mode_index: 20, 3,
    decompress_on_n_zplanes: 23, 4,
    allow_expclear: 27, 1,
    read_size: 28, 1,
    tile_surface_enable: 29, 1,
    clear_disallowed: 30, 1,
    zrange_precision: 31, 1,
});

bitfield!(StencilInfo {
    format: 0, 1,
});

bitfield!(DepthSize {
    pitch_tile_max: 0, 11,
    height_tile_max: 11, 11,
});

bitfield!(DepthSlice {
    tile_max: 0, 22,
});

/// `AmdGpu::DepthBuffer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DepthBuffer {
    pub z_info: ZInfo,
    pub stencil_info: StencilInfo,
    pub z_read_base: u32,
    pub stencil_read_base: u32,
    pub z_write_base: u32,
    pub stencil_write_base: u32,
    pub depth_size: DepthSize,
    pub depth_slice: DepthSlice,
}

impl DepthBuffer {
    pub fn depth_valid(&self) -> bool {
        self.depth_address() != 0 && self.z_info.format() != z_format::Invalid
    }

    pub fn stencil_valid(&self) -> bool {
        self.stencil_address() != 0 && self.stencil_info.format() != stencil_format::Invalid
    }

    pub fn depth_write_valid(&self) -> bool {
        self.depth_write_address() != 0 && self.z_info.format() != z_format::Invalid
    }

    pub fn stencil_write_valid(&self) -> bool {
        self.stencil_write_address() != 0 && self.stencil_info.format() != stencil_format::Invalid
    }

    pub fn pitch(&self) -> u32 {
        (self.depth_size.pitch_tile_max() + 1) << 3
    }

    pub fn height(&self) -> u32 {
        (self.depth_size.height_tile_max() + 1) << 3
    }

    pub fn depth_address(&self) -> u64 {
        u64::from(self.z_read_base) << 8
    }

    pub fn stencil_address(&self) -> u64 {
        u64::from(self.stencil_read_base) << 8
    }

    pub fn depth_write_address(&self) -> u64 {
        u64::from(self.z_write_base) << 8
    }

    pub fn stencil_write_address(&self) -> u64 {
        u64::from(self.stencil_write_base) << 8
    }

    pub fn num_samples(&self) -> u32 {
        1 << self.z_info.num_samples()
    }

    pub fn num_bits(&self) -> u32 {
        if self.z_info.format() == z_format::Z32Float {
            32
        } else {
            16
        }
    }

    pub fn depth_slice_size(&self) -> u32 {
        assert!(
            self.z_info.format() != z_format::Invalid,
            "invalid depth format"
        );
        let bytes_per_element = self.num_bits() >> 3;
        (self.depth_slice.tile_max() + 1) * 64 * bytes_per_element * self.num_samples()
    }

    pub fn tile_mode(&self) -> TileMode {
        TileMode::from_raw(self.z_info.tile_mode_index()).unwrap_or(TileMode::DisplayLinearAligned)
    }

    pub fn is_tiled(&self) -> bool {
        let mode = self.tile_mode();
        mode != TileMode::DisplayLinearAligned && mode != TileMode::DisplayLinearGeneral
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_control_fields_sit_at_their_offsets() {
        let control = DepthControl::from_raw((compare_func::LessEqual << 4) | (1 << 1) | (1 << 2));
        assert_eq!(control.depth_func(), compare_func::LessEqual);
        assert_ne!(control.depth_enable(), 0);
        assert_eq!(control.stencil_enable(), 0);
        assert_eq!(control.depth_write_enable(), 1);
    }

    #[test]
    fn a_depth_buffer_decodes_its_size_and_addresses() {
        let buffer = DepthBuffer {
            z_info: ZInfo::from_raw(z_format::Z32Float),
            z_read_base: 0x1000,
            depth_size: DepthSize::from_raw(3 | (5 << 11)),
            depth_slice: DepthSlice::from_raw(2),
            ..DepthBuffer::default()
        };
        assert!(buffer.depth_valid());
        assert_eq!(buffer.depth_address(), 0x1000 << 8);
        assert_eq!(buffer.pitch(), 4 << 3);
        assert_eq!(buffer.height(), 6 << 3);
        assert_eq!(buffer.num_bits(), 32);
        assert_eq!(buffer.depth_slice_size(), 3 * 64 * 4);
    }

    #[test]
    fn a_depth_view_counts_its_slices() {
        assert_eq!(DepthView::from_raw(7 << 13).num_slices(), 8);
        assert_eq!(DepthView::default().num_slices(), 1);
    }
}
