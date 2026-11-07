//! Profiler-disabled Tracy Vulkan hooks, ported from `legacy/gpu/shim/tracy/TracyVulkan.hpp`.

pub type TracyVkCtx = *mut std::ffi::c_void;

pub fn collect(_context: TracyVkCtx, _command: *mut std::ffi::c_void) {}
pub fn destroy(_context: TracyVkCtx) {}

pub struct VkCtxScope;

impl VkCtxScope {
    pub fn new<T>(_args: T) -> Self {
        Self
    }
}
