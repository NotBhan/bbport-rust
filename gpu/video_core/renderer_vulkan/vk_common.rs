//! The Vulkan dispatch loader, ported from
//! `legacy/gpu/shadps4/video_core/renderer_vulkan/vk_common.cpp`.
//!
//! The C++ compiles the VMA implementation and stores the Vulkan-Hpp dispatch loader here.
//! The Rust port uses `ash`, which keeps the loader per instance, so this module holds the
//! one piece of process state the C++ had: the instance the dispatch loader was built for.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// The Vulkan instance handle the loader was built for; raw handles are not `Send`, but the
/// instance outlives every thread that reads it.
#[derive(Debug, Clone, Copy)]
struct InstanceHandle(*mut c_void);

unsafe impl Send for InstanceHandle {}

static LOADER_INSTANCE: Mutex<Option<InstanceHandle>> = Mutex::new(None);
static LOADER_READY: AtomicBool = AtomicBool::new(false);

/// `VULKAN_HPP_DEFAULT_DISPATCH_LOADER_DYNAMIC_STORAGE`: the loader is ready for `instance`.
///
/// # Safety
/// `instance` must be a live `VkInstance` handle.
pub unsafe fn set_dispatch_instance(instance: *mut c_void) {
    *LOADER_INSTANCE
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(InstanceHandle(instance));
    LOADER_READY.store(true, Ordering::Release);
}

/// The instance the dispatch loader was initialised with, if any.
pub fn dispatch_instance() -> Option<*mut c_void> {
    LOADER_INSTANCE
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .map(|handle| handle.0)
}

/// Whether the dispatch loader has been initialised (the C++ storage is always present).
pub fn dispatch_loader_ready() -> bool {
    LOADER_READY.load(Ordering::Acquire)
}

/// A `vk::ShaderModule` as the vendored code's Vulkan-Hpp wrapper carries it.
pub type ShaderModule = u64;
/// A `vk::Device` handle.
pub type DeviceHandle = u64;
/// Raw Vulkan handle aliases used by the header ports; actual lifetimes are owned by ash.
pub type VkImage = u64;
pub type VkImageView = u64;
pub type VkBuffer = u64;
pub type VkSemaphore = u64;
pub type VkPipeline = u64;
pub type VkPipelineLayout = u64;
pub type VkDescriptorSetLayout = u64;
pub type VkShaderModule = u64;
pub type VkCommandBuffer = u64;
pub type VkFormat = i32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_loader_records_the_instance_it_was_built_for() {
        let instance = 0x1234usize as *mut c_void;
        unsafe { set_dispatch_instance(instance) };
        assert!(dispatch_loader_ready());
        assert_eq!(dispatch_instance(), Some(instance));
    }
}

// The header's build configuration. The Vulkan-Hpp switches (`VULKAN_HPP_DISPATCH_LOADER_DYNAMIC`
// and the `NO_*` pair) have no counterpart here because the port binds Vulkan through `ash`,
// which always dispatches dynamically and never throws; what remains are the settings that
// still say something about the port.

/// `VK_ENABLE_BETA_EXTENSIONS`.
pub const VK_ENABLE_BETA_EXTENSIONS: bool = true;
/// `VK_NO_PROTOTYPES`: entry points are resolved at run time, not linked.
pub const NO_PROTOTYPES: bool = true;
/// `VMA_STATIC_VULKAN_FUNCTIONS`.
pub const VMA_STATIC_VULKAN_FUNCTIONS: bool = false;
/// `VMA_DYNAMIC_VULKAN_FUNCTIONS`.
pub const VMA_DYNAMIC_VULKAN_FUNCTIONS: bool = true;

#[cfg(test)]
mod build_config_tests {
    use super::*;

    #[test]
    fn the_allocator_handles_are_dynamic_only() {
        // The switches are compile-time constants, so they are checked as such.
        const {
            assert!(VK_ENABLE_BETA_EXTENSIONS);
            assert!(NO_PROTOTYPES);
            assert!(VMA_DYNAMIC_VULKAN_FUNCTIONS);
            assert!(!VMA_STATIC_VULKAN_FUNCTIONS);
        }
    }
}
