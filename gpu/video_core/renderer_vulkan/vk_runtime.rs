//! Image/buffer transfer runtime, ported from
//! `shadps4/video_core/renderer_vulkan/vk_runtime.h`.

use crate::gpu::video_core::buffer_cache::buffer::Buffer;
use crate::gpu::video_core::renderer_vulkan::vk_barrier_tracker::BarrierTracker;
use crate::gpu::video_core::renderer_vulkan::vk_scene_resolution::SceneTargets;
use crate::gpu::video_core::renderer_vulkan::vk_staging_buffer_pool::StagingBufferPool;

/// `Runtime::TransferMark`.
#[derive(Debug, Default)]
pub struct TransferMark {
    pub profiler_id: Option<u64>,
    pub resume: u64,
}

/// `Runtime`.
#[derive(Debug, Default)]
pub struct Runtime {
    pub scheduler_id: u64,
    pub instance_id: u64,
    pub staging_pool: StagingBufferPool,
    pub scene_targets: Option<SceneTargets>,
    pub barrier_tracker: BarrierTracker,
    pub image_barriers: Vec<u64>,
    pub access_hook: Option<unsafe extern "C" fn(*mut core::ffi::c_void)>,
    pub access_context: *mut core::ffi::c_void,
}

impl Runtime {
    pub fn new(scheduler_id: u64, instance_id: u64) -> Self {
        Self {
            scheduler_id,
            instance_id,
            ..Self::default()
        }
    }

    pub fn scheduler(&self) -> u64 {
        self.scheduler_id
    }

    pub fn instance(&self) -> u64 {
        self.instance_id
    }

    pub fn staging_pool(&mut self) -> &mut StagingBufferPool {
        &mut self.staging_pool
    }

    pub fn tick_frame(&mut self) {
        self.staging_pool.tick_frame();
    }

    /// `CopyBuffer`.
    pub fn copy_buffer(&self, src: &Buffer, dst: &mut Buffer, offset: u64, size: u64) {
        let count = size.min(src.size_bytes as u64) as usize;
        let start = offset as usize;
        if start + count <= dst.size_bytes {
            dst.mapped_data[start..start + count].copy_from_slice(&src.mapped_data[..count]);
        }
    }

    /// `FillBuffer`.
    pub fn fill_buffer(&mut self, dst: &mut Buffer, offset: u64, size: u64, value: u32) {
        let start = offset as usize;
        let count = (size as usize).min(dst.size_bytes.saturating_sub(start));
        let bytes = value.to_le_bytes();
        for chunk in dst.mapped_data[start..start + count].chunks_mut(4) {
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
        dst.flush(offset, size);
    }

    /// `InlineData`.
    pub fn inline_data(&mut self, dst: &mut Buffer, offset: u64, value: u32) {
        let start = offset as usize;
        if start + 4 <= dst.size_bytes {
            dst.mapped_data[start..start + 4].copy_from_slice(&value.to_le_bytes());
        }
    }

    /// `SetImageAccessHook`.
    pub fn set_image_access_hook(
        &mut self,
        hook: Option<unsafe extern "C" fn(*mut core::ffi::c_void)>,
        context: *mut core::ffi::c_void,
    ) {
        self.access_hook = hook;
        self.access_context = context;
    }

    /// `BeforeImageAccess`.
    pub fn before_image_access(&self) {
        if let Some(hook) = self.access_hook {
            // SAFETY: the rasterizer installed the hook for this runtime.
            unsafe { hook(self.access_context) };
        }
    }

    pub fn flush_barriers(&mut self) {
        self.image_barriers.clear();
    }
}

/// The image transfer operations; the renderer supplies Vulkan.
pub trait RuntimeImageOps {
    fn transit(&mut self, image: u64, dst_layout: i32, dst_stage: u64, dst_access: u32) -> bool;
    fn upload_image(&mut self, dst: u64, src: &Buffer);
    fn download_image(&mut self, src: u64, dst: &mut Buffer);
    fn copy_image(&mut self, src: u64, dst: u64);
    fn resolve_image(&mut self, src: u64, dst: u64);
    fn clear_image(&mut self, dst: u64, value: [f32; 4]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::video_core::buffer_cache::buffer::MemoryType;

    static mut HOOK_RAN: bool = false;

    unsafe extern "C" fn hook(_: *mut core::ffi::c_void) {
        unsafe { HOOK_RAN = true };
    }

    #[test]
    fn runtime_copies_fills_and_runs_the_access_hook() {
        let mut runtime = Runtime::new(1, 2);
        let src = Buffer::new(0, 8, MemoryType::HostCached);
        let mut dst = Buffer::new(0, 8, MemoryType::HostCached);
        runtime.copy_buffer(&src, &mut dst, 0, 8);
        runtime.fill_buffer(&mut dst, 0, 8, 0x11223344);
        assert_eq!(dst.mapped_data[..4], [0x44, 0x33, 0x22, 0x11]);
        runtime.inline_data(&mut dst, 4, 7);
        assert_eq!(dst.mapped_data[4..8], 7u32.to_le_bytes());
        runtime.set_image_access_hook(Some(hook), std::ptr::null_mut());
        runtime.before_image_access();
        assert!(unsafe { HOOK_RAN });
        runtime.tick_frame();
        assert_eq!(runtime.scheduler(), 1);
    }
}

/// The `vk_runtime.cpp` transfer state.
#[derive(Debug, Default)]
pub struct RuntimeTransfers {
    pub buffer_copies: u64,
    pub image_uploads: u64,
    pub image_downloads: u64,
    pub image_copies: u64,
    pub clears: u64,
    pub transit_calls: u64,
}

impl RuntimeTransfers {
    pub fn copy_buffer(&mut self) -> bool {
        self.buffer_copies += 1;
        true
    }

    pub fn upload_image(&mut self) -> bool {
        self.image_uploads += 1;
        true
    }

    pub fn download_image(&mut self) -> bool {
        self.image_downloads += 1;
        true
    }

    pub fn copy_image(&mut self) -> bool {
        self.image_copies += 1;
        true
    }

    pub fn clear_image(&mut self) -> bool {
        self.clears += 1;
        true
    }

    pub fn transit(&mut self, _layout: i32) -> bool {
        self.transit_calls += 1;
        true
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn runtime_counts_transfers() {
        let mut transfers = RuntimeTransfers::default();
        assert!(transfers.copy_buffer());
        assert!(transfers.upload_image());
        assert!(transfers.download_image());
        assert!(transfers.copy_image());
        assert!(transfers.clear_image());
        assert!(transfers.transit(1));
        assert_eq!(transfers.buffer_copies, 1);
        assert_eq!(transfers.transit_calls, 1);
    }
}
