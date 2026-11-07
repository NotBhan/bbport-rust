//! No ImGui overlay in the port; the renderer hooks are no-ops, ported from
//! `legacy/gpu/shim/imgui/renderer/imgui_core.h`.

use std::ffi::c_void;

pub struct Instance;
pub struct WindowSdl;
pub type VkFormat = u32;
pub type VkDevice = *mut c_void;

pub fn initialize(_instance: &Instance, _window: &WindowSdl, _image_count: u32, _format: VkFormat) {
}

pub fn on_surface_format_change(_format: VkFormat) {}
pub fn shutdown(_device: VkDevice) {}
pub const fn must_keep_drawing() -> bool {
    false
}

pub struct TextureManager;

impl TextureManager {
    pub fn submit() {}
}
