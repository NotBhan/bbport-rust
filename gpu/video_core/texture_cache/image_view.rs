//! Texture image views, ported from
//! `shadps4/video_core/texture_cache/image_view.h`.

use crate::gpu::amdgpu::resource::ImageType;
use crate::gpu::video_core::renderer_vulkan::vk_common::{VkFormat, VkImageView};
use crate::gpu::video_core::texture_cache::types::SubresourceRange;

/// `ImageViewInfo`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageViewInfo {
    pub kind: ImageType,
    pub format: VkFormat,
    pub range: SubresourceRange,
    pub mapping: ComponentMapping,
    pub min_lod: u32,
    pub is_storage: bool,
}

/// `vk::ComponentMapping`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ComponentMapping {
    pub r: i32,
    pub g: i32,
    pub b: i32,
    pub a: i32,
}

impl Default for ImageViewInfo {
    fn default() -> Self {
        Self {
            kind: ImageType::Color2D,
            format: 0,
            range: SubresourceRange::default(),
            mapping: ComponentMapping::default(),
            min_lod: 0,
            is_storage: false,
        }
    }
}

/// `ImageView`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImageView {
    pub info: ImageViewInfo,
    pub image_view: VkImageView,
}

impl ImageView {
    pub fn new(info: ImageViewInfo, image_view: VkImageView) -> Self {
        Self { info, image_view }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_view_info_defaults_to_a_color_2d_view() {
        let info = ImageViewInfo::default();
        assert_eq!(info.kind, ImageType::Color2D);
        assert_eq!(info.format, 0);
        assert_eq!(info.min_lod, 0);
        assert!(!info.is_storage);
        let view = ImageView::new(info, 5);
        assert_eq!(view.image_view, 5);
        assert_eq!(view.info, info);
    }
}
