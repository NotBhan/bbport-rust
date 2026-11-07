//! Texture image state, ported from `shadps4/video_core/texture_cache/image.h`.

use crate::gpu::common::slot_vector::SlotId;
use crate::gpu::video_core::texture_cache::image_info::ImageInfo;
use crate::gpu::video_core::texture_cache::image_view::{ImageView, ImageViewInfo};

pub type ImageId = SlotId;

pub const FLAG_MAYBE_CPU_DIRTY: u32 = 1 << 0;
pub const FLAG_CPU_DIRTY: u32 = 1 << 1;
pub const FLAG_GPU_DIRTY: u32 = 1 << 2;
pub const FLAG_DIRTY: u32 = FLAG_MAYBE_CPU_DIRTY | FLAG_CPU_DIRTY | FLAG_GPU_DIRTY;
pub const FLAG_GPU_MODIFIED: u32 = 1 << 3;
pub const FLAG_REGISTERED: u32 = 1 << 6;
pub const FLAG_PICKED: u32 = 1 << 7;

/// `Image::State`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct State {
    pub pl_stage: u64,
    pub access_mask: u32,
    pub layout: i32,
}

/// `Image::BackingImage`.
#[derive(Debug, Clone, Default)]
pub struct BackingImage {
    pub image: u64,
    pub state: State,
    pub image_view_infos: Vec<ImageViewInfo>,
    pub views: Vec<ImageView>,
    pub last_view: u32,
    pub num_samples: u32,
    pub size_bytes: u64,
}

/// `Image`.
#[derive(Debug, Clone)]
pub struct Image {
    pub flags: u32,
    pub is_bound: bool,
    pub is_target: bool,
    pub needs_rebind: bool,
    pub force_general: bool,
    pub usage_texture: bool,
    pub usage_storage: bool,
    pub usage_render_target: bool,
    pub usage_depth_target: bool,
    pub usage_vo_surface: bool,
    pub scene_proxy: bool,
    pub track_addr: u64,
    pub track_addr_end: u64,
    pub backing: usize,
    pub depth_id: Option<ImageId>,
    pub depth_uid: u64,
    pub image_uid: u64,
    pub info: ImageInfo,
    pub aspect_mask: u32,
    pub backing_images: Vec<BackingImage>,
    pub lru_id: u64,
    pub hash: u64,
}

impl Image {
    pub fn new(info: ImageInfo) -> Self {
        Self {
            flags: FLAG_DIRTY,
            is_bound: false,
            is_target: false,
            needs_rebind: false,
            force_general: false,
            usage_texture: false,
            usage_storage: false,
            usage_render_target: false,
            usage_depth_target: false,
            usage_vo_surface: false,
            scene_proxy: false,
            track_addr: 0,
            track_addr_end: 0,
            backing: 0,
            depth_id: None,
            depth_uid: 0,
            image_uid: 0,
            info,
            aspect_mask: 1,
            backing_images: vec![BackingImage::default()],
            lru_id: 0,
            hash: 0,
        }
    }

    pub fn overlaps(&self, cpu_addr: u64, size: u64) -> bool {
        let end = cpu_addr.saturating_add(size);
        self.info.guest_address < end
            && cpu_addr
                < self
                    .info
                    .guest_address
                    .saturating_add(self.info.guest_size as u64)
    }

    pub fn get_image(&self) -> u64 {
        self.backing_images
            .get(self.backing)
            .map_or(0, |backing| backing.image)
    }

    pub fn get_host_image_size(&self) -> u64 {
        self.backing_images
            .get(self.backing)
            .map_or(0, |backing| backing.size_bytes)
    }

    pub fn is_tracked(&self) -> bool {
        self.track_addr != 0 && self.track_addr_end != 0
    }

    pub fn safe_to_download(&self) -> bool {
        self.flags & FLAG_GPU_MODIFIED != 0 && self.flags & FLAG_DIRTY == 0
    }

    pub fn associate_depth(&mut self, depth_image_id: ImageId, depth_uid: u64) {
        self.depth_id = Some(depth_image_id);
        self.depth_uid = depth_uid;
    }

    pub fn disassociate_depth(&mut self) {
        self.depth_id = None;
        self.depth_uid = 0;
    }

    /// `FindView`; reuses a view with the same description.
    pub fn find_view(&mut self, view_info: &ImageViewInfo) -> ImageView {
        let backing = &mut self.backing_images[self.backing];
        if let Some((index, view)) = backing
            .views
            .iter()
            .enumerate()
            .find(|(_, view)| &view.info == view_info)
        {
            backing.last_view = index as u32;
            return *view;
        }
        let view = ImageView::new(*view_info, backing.image + backing.views.len() as u64 + 1);
        backing.views.push(view);
        backing.image_view_infos.push(*view_info);
        backing.last_view = backing.views.len() as u32 - 1;
        view
    }

    /// `GetBarriers`; returns the barriers the image needs to reach `dst_layout`.
    pub fn get_barriers(
        &self,
        dst_layout: i32,
        dst_mask: u32,
        dst_stage: u64,
    ) -> Vec<(i32, u32, u64)> {
        let state = self
            .backing_images
            .get(self.backing)
            .map_or(State::default(), |backing| backing.state);
        if state.layout == dst_layout && state.access_mask == dst_mask {
            return Vec::new();
        }
        vec![(dst_layout, dst_mask, dst_stage)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_track_overlap_views_and_depth() {
        let info = ImageInfo {
            guest_address: 0x1000,
            guest_size: 0x100,
            ..ImageInfo::default()
        };
        let mut image = Image::new(info);
        assert!(image.overlaps(0x1080, 0x20));
        assert!(!image.overlaps(0x1200, 0x10));
        assert!(!image.is_tracked());
        image.track_addr = 0x1000;
        image.track_addr_end = 0x1100;
        assert!(image.is_tracked());
        assert!(!image.safe_to_download());
        let view = image.find_view(&ImageViewInfo::default());
        assert_eq!(view.image_view, 1);
        assert_eq!(
            image.find_view(&ImageViewInfo::default()).image_view,
            view.image_view
        );
        image.associate_depth(SlotId::default(), 5);
        assert_eq!(image.depth_uid, 5);
        assert_eq!(image.get_barriers(1, 2, 3), vec![(1, 2, 3)]);
    }
}
