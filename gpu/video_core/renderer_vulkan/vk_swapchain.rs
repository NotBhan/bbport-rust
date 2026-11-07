//! Swapchain state, ported from
//! `shadps4/video_core/renderer_vulkan/vk_swapchain.h`.

/// `Swapchain`.
#[derive(Debug, Default)]
pub struct Swapchain {
    pub swapchain: u64,
    pub surface: u64,
    pub surface_format: i64,
    pub view_format: i32,
    pub present_mode: u32,
    pub width: u32,
    pub height: u32,
    pub image_count: u32,
    pub image_index: u32,
    pub frame_index: u32,
    pub needs_recreation: bool,
    pub needs_hdr: bool,
    pub supports_hdr: bool,
    pub images: Vec<u64>,
    pub image_views: Vec<u64>,
    pub image_acquired: Vec<u64>,
    pub present_ready: Vec<u64>,
}

pub const PRESENT_MODE_FIFO: u32 = 0;
pub const PRESENT_MODE_FIFO_RELAXED: u32 = 1;

impl Swapchain {
    pub fn new(surface: u64, supports_hdr: bool) -> Self {
        Self {
            surface,
            supports_hdr,
            ..Self::default()
        }
    }

    /// `Create`/`Recreate`.
    pub fn create(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.image_count = self.image_count.max(2);
        self.images.resize(self.image_count as usize, 0);
        self.image_views.resize(self.image_count as usize, 0);
        self.image_acquired.resize(self.image_count as usize, 0);
        self.present_ready.resize(self.image_count as usize, 0);
        self.frame_index %= self.image_count.max(1);
        self.image_index %= self.image_count.max(1);
        self.needs_recreation = false;
    }

    pub fn recreate(&mut self, width: u32, height: u32) {
        self.create(width, height);
    }

    /// `AcquireNextImage`.
    pub fn acquire_next_image(&mut self) -> bool {
        if self.images.is_empty() {
            return false;
        }
        self.image_index = (self.image_index + 1) % self.image_count;
        self.frame_index = (self.frame_index + 1) % self.image_count;
        true
    }

    /// `Present`.
    pub fn present(&mut self) -> bool {
        if self.images.is_empty() {
            return false;
        }
        self.image_index = (self.image_index + 1) % self.image_count;
        true
    }

    pub fn surface(&self) -> u64 {
        self.surface
    }

    pub fn image(&self) -> u64 {
        self.images
            .get(self.image_index as usize)
            .copied()
            .unwrap_or(0)
    }

    pub fn image_view(&self) -> u64 {
        self.image_views
            .get(self.image_index as usize)
            .copied()
            .unwrap_or(0)
    }

    pub fn surface_format(&self) -> i64 {
        self.surface_format
    }

    pub fn handle(&self) -> u64 {
        self.swapchain
    }

    pub fn extent(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn image_acquired_semaphore(&self) -> u64 {
        self.image_acquired
            .get(self.frame_index as usize)
            .copied()
            .unwrap_or(0)
    }

    pub fn present_ready_semaphore(&self) -> u64 {
        self.present_ready
            .get(self.image_index as usize)
            .copied()
            .unwrap_or(0)
    }

    pub fn has_hdr(&self) -> bool {
        self.supports_hdr
    }

    pub fn set_hdr(&mut self, hdr: bool) {
        self.needs_hdr = hdr;
    }

    pub fn get_hdr(&self) -> bool {
        self.needs_hdr
    }

    /// `WaitsForDisplay`.
    pub fn waits_for_display(&self) -> bool {
        self.present_mode == PRESENT_MODE_FIFO || self.present_mode == PRESENT_MODE_FIFO_RELAXED
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swapchain_images_cycle_and_fifo_waits() {
        let mut chain = Swapchain::new(3, true);
        assert!(!chain.acquire_next_image());
        chain.image_count = 3;
        chain.create(1920, 1080);
        assert!(chain.acquire_next_image());
        assert_eq!(chain.extent(), (1920, 1080));
        assert!(chain.present());
        assert!(chain.has_hdr());
        chain.set_hdr(true);
        assert!(chain.get_hdr());
        chain.present_mode = PRESENT_MODE_FIFO;
        assert!(chain.waits_for_display());
        chain.present_mode = 2;
        assert!(!chain.waits_for_display());
        assert_eq!(chain.image_count, 3);
    }
}
