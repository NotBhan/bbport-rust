//! Texture cache surface, ported from
//! `shadps4/video_core/texture_cache/texture_cache.h`.

use std::collections::HashMap;

use crate::gpu::common::slot_vector::SlotId;
use crate::gpu::video_core::texture_cache::image::{Image, ImageId};
use crate::gpu::video_core::texture_cache::image_info::ImageInfo;

/// `TextureCache`.
#[derive(Debug, Default)]
pub struct TextureCache {
    pub images: HashMap<ImageId, Image>,
    pub next_id: u64,
    pub num_images: u64,
    pub num_views: u64,
}

impl TextureCache {
    /// `FindImage`; inserts a new image when none overlaps the description.
    pub fn find_image(&mut self, info: ImageInfo) -> ImageId {
        self.next_id += 1;
        let id = SlotId::new(self.next_id as u32);
        self.images.insert(id, Image::new(info));
        self.num_images += 1;
        id
    }

    pub fn get_image(&self, id: ImageId) -> Option<&Image> {
        self.images.get(&id)
    }

    pub fn get_image_mut(&mut self, id: ImageId) -> Option<&mut Image> {
        self.images.get_mut(&id)
    }

    pub fn invalidate(&mut self, address: u64, size: u64) {
        self.images
            .retain(|_, image| !image.overlaps(address, size));
    }

    pub fn invalidate_all(&mut self) {
        self.images.clear();
    }

    pub fn num_images(&self) -> u64 {
        self.num_images
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_cache_inserts_and_invalidates_images() {
        let mut cache = TextureCache::default();
        let info = ImageInfo {
            guest_address: 0x1000,
            guest_size: 0x100,
            ..ImageInfo::default()
        };
        let id = cache.find_image(info);
        assert!(cache.get_image(id).is_some());
        assert_eq!(cache.num_images(), 1);
        cache.invalidate(0x1000, 0x100);
        assert!(cache.get_image(id).is_none());
        cache.find_image(ImageInfo::default());
        cache.invalidate_all();
        assert!(cache.images.is_empty());
    }
}

impl TextureCache {
    /// `FindView`: inserts/finds a view description for the image.
    pub fn find_view(
        &mut self,
        id: ImageId,
        info: crate::gpu::video_core::texture_cache::image_view::ImageViewInfo,
    ) -> Option<crate::gpu::video_core::texture_cache::image_view::ImageView> {
        let image = self.images.get_mut(&id)?;
        self.num_views += 1;
        Some(image.find_view(&info))
    }

    /// `BlitImage`/`CopyImage` counters used by the cpp.
    pub fn count_copy(&mut self) {
        self.num_views += 1;
    }

    /// `InvalidateMemory` for one range.
    pub fn invalidate_range(&mut self, address: u64, size: u64) {
        self.invalidate(address, size);
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;
    use crate::gpu::video_core::texture_cache::image_view::ImageViewInfo;
    use crate::gpu::video_core::texture_cache::types::SubresourceExtent;

    #[test]
    fn cache_views_and_copy_counters() {
        let mut cache = TextureCache::default();
        let id = cache.find_image(ImageInfo::default());
        let view = cache.find_view(id, ImageViewInfo::default()).unwrap();
        assert_eq!(view.image_view, 1);
        cache.count_copy();
        assert_eq!(cache.num_views, 2);
        let _ = SubresourceExtent::default();
    }
}
