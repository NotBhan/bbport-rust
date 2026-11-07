//! Texture image descriptions, ported from
//! `shadps4/video_core/texture_cache/image_info.h`.

use crate::gpu::amdgpu::resource::ImageType;
use crate::gpu::amdgpu::tiling::{ArrayMode, TileMode};
use crate::gpu::video_core::texture_cache::types::{Extent2D, Extent3D, SubresourceExtent};

pub const IS_VOLUME: u32 = 1 << 0;
pub const IS_TILED: u32 = 1 << 1;
pub const IS_POW2: u32 = 1 << 2;
pub const IS_BLOCK: u32 = 1 << 3;
pub const IS_DEPTH: u32 = 1 << 4;
pub const HAS_STENCIL: u32 = 1 << 5;

/// `ImageProperties`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImageProperties {
    pub flags: u32,
}

impl ImageProperties {
    pub fn is_volume(&self) -> bool {
        self.flags & IS_VOLUME != 0
    }
    pub fn is_tiled(&self) -> bool {
        self.flags & IS_TILED != 0
    }
    pub fn is_pow2(&self) -> bool {
        self.flags & IS_POW2 != 0
    }
    pub fn is_block(&self) -> bool {
        self.flags & IS_BLOCK != 0
    }
    pub fn is_depth(&self) -> bool {
        self.flags & IS_DEPTH != 0
    }
    pub fn has_stencil(&self) -> bool {
        self.flags & HAS_STENCIL != 0
    }
}

/// `ImageInfo::meta_info`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetaInfo {
    pub cmask_addr: u64,
    pub fmask_addr: u64,
    pub htile_addr: u64,
    pub htile_clear_mask: i32,
}

impl Default for MetaInfo {
    fn default() -> Self {
        Self {
            cmask_addr: 0,
            fmask_addr: 0,
            htile_addr: 0,
            htile_clear_mask: -1,
        }
    }
}

/// `ImageInfo::MipInfo`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MipInfo {
    pub size: u32,
    pub pitch: u32,
    pub height: u32,
    pub offset: u32,
}

/// `ImageInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageInfo {
    pub meta_info: MetaInfo,
    pub props: ImageProperties,
    pub pixel_format: i32,
    pub kind: ImageType,
    pub resources: SubresourceExtent,
    pub size: Extent3D,
    pub num_bits: u32,
    pub num_samples: u32,
    pub pitch: u32,
    pub tile_mode: TileMode,
    pub array_mode: ArrayMode,
    pub mips_layout: [MipInfo; 16],
    pub guest_address: u64,
    pub guest_size: u32,
    pub bank_swizzle: u8,
    pub alt_tile: bool,
    pub stencil_addr: u64,
    pub stencil_size: u32,
}

impl Default for ImageInfo {
    fn default() -> Self {
        let mut info = Self {
            meta_info: MetaInfo::default(),
            props: ImageProperties::default(),
            pixel_format: 0,
            kind: ImageType::Color2D,
            resources: SubresourceExtent::default(),
            size: Extent3D {
                width: 1,
                height: 1,
                depth: 1,
            },
            num_bits: 0,
            num_samples: 1,
            pitch: 0,
            tile_mode: TileMode::DisplayLinearAligned,
            array_mode: ArrayMode::ArrayLinearAligned,
            mips_layout: [MipInfo::default(); 16],
            guest_address: 0,
            guest_size: 0,
            bank_swizzle: 0,
            alt_tile: false,
            stencil_addr: 0,
            stencil_size: 0,
        };
        info.update_size();
        info
    }
}

impl ImageInfo {
    pub fn is_tiled(&self) -> bool {
        self.tile_mode != TileMode::DisplayLinearAligned
    }

    pub fn block_dim(&self) -> Extent2D {
        let dim = if self.props.is_block() { 2 } else { 0 };
        Extent2D {
            width: self.pitch >> dim,
            height: self.size.height >> dim,
        }
    }

    pub fn update_size(&mut self) {
        if self.props.is_block() {
            self.size.width = self.size.width.max(1);
        }
    }

    pub fn mip_of(&self, info: &ImageInfo) -> i32 {
        self.guest_address as i32 - info.guest_address as i32
    }

    pub fn slice_of(&self, info: &ImageInfo, mip: i32) -> i32 {
        (self.guest_address as i32 - info.guest_address as i32) - mip
    }

    pub fn is_compatible(&self, info: &ImageInfo) -> bool {
        self.kind == info.kind
            && self.pixel_format == info.pixel_format
            && self.size == info.size
            && self.num_samples == info.num_samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_info_defaults_and_properties() {
        let mut info = ImageInfo::default();
        assert_eq!(info.size.width, 1);
        assert!(!info.is_tiled());
        assert!(info.is_compatible(&ImageInfo::default()));
        info.props.flags = IS_BLOCK | IS_DEPTH;
        assert!(info.props.is_block());
        assert!(info.props.is_depth());
        assert_eq!(
            info.block_dim(),
            Extent2D {
                width: 0,
                height: 0
            }
        );
        info.pitch = 64;
        info.size.height = 8;
        assert_eq!(
            info.block_dim(),
            Extent2D {
                width: 16,
                height: 2
            }
        );
        assert!(!info.is_compatible(&ImageInfo::default()));
    }
}
