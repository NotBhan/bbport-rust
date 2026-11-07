//! Texture-cache address and copy types, ported from
//! `shadps4/video_core/texture_cache/types.h`.

/// `VideoCore::ImageId`.
pub type ImageId = crate::gpu::common::slot_vector::SlotId;
/// `VideoCore::ImageViewId`.
pub type ImageViewId = crate::gpu::common::slot_vector::SlotId;

/// `Offset2D`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Offset2D {
    pub x: i32,
    pub y: i32,
}

/// `Offset3D`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Offset3D {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// `Region2D`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Region2D {
    pub start: Offset2D,
    pub end: Offset2D,
}

/// `Extent2D`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Extent2D {
    pub width: u32,
    pub height: u32,
}

/// `Extent3D`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Extent3D {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

/// `SubresourceLayers`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubresourceLayers {
    pub base_level: i32,
    pub base_layer: i32,
    pub num_layers: i32,
}

impl Default for SubresourceLayers {
    fn default() -> Self {
        Self {
            base_level: 0,
            base_layer: 0,
            num_layers: 1,
        }
    }
}

/// `SubresourceBase`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct SubresourceBase {
    pub level: u32,
    pub layer: u32,
}

/// `SubresourceExtent`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SubresourceExtent {
    pub levels: u32,
    pub layers: u32,
}

impl Default for SubresourceExtent {
    fn default() -> Self {
        Self {
            levels: 1,
            layers: 1,
        }
    }
}

/// `SubresourceRange`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct SubresourceRange {
    pub base: SubresourceBase,
    pub extent: SubresourceExtent,
}

/// `ImageCopy`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImageCopy {
    pub src_subresource: SubresourceLayers,
    pub dst_subresource: SubresourceLayers,
    pub src_offset: Offset3D,
    pub dst_offset: Offset3D,
    pub extent: Extent3D,
}

/// `BufferImageCopy`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BufferImageCopy {
    pub buffer_offset: usize,
    pub buffer_size: usize,
    pub buffer_row_length: u32,
    pub buffer_image_height: u32,
    pub image_subresource: SubresourceLayers,
    pub image_offset: Offset3D,
    pub image_extent: Extent3D,
}

/// `BufferCopy`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BufferCopy {
    pub src_offset: u64,
    pub dst_offset: u64,
    pub size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layouts_match_the_header() {
        assert_eq!(core::mem::size_of::<Offset2D>(), 8);
        assert_eq!(core::mem::size_of::<Offset3D>(), 12);
        assert_eq!(core::mem::size_of::<Region2D>(), 16);
        assert_eq!(core::mem::size_of::<Extent2D>(), 8);
        assert_eq!(core::mem::size_of::<Extent3D>(), 12);
        assert_eq!(core::mem::size_of::<SubresourceLayers>(), 12);
        assert_eq!(core::mem::size_of::<SubresourceRange>(), 16);
        assert_eq!(core::mem::size_of::<ImageCopy>(), 60);
        assert_eq!(core::mem::size_of::<BufferImageCopy>(), 64);
        assert_eq!(core::mem::size_of::<BufferCopy>(), 24);
        assert_eq!(SubresourceLayers::default().num_layers, 1);
    }
}
