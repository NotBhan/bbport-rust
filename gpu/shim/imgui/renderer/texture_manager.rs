//! No ImGui overlay in the port; this header repeats the `imgui_core.h` hooks, ported from
//! `legacy/gpu/shim/imgui/renderer/texture_manager.h`.

pub use super::imgui_core::{
    initialize, must_keep_drawing, on_surface_format_change, shutdown, Instance, TextureManager,
    VkDevice, VkFormat, WindowSdl,
};
