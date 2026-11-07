//! Port of the C++ renderer in `gpu/**` (milestone 3). The C++ sources are kept in
//! `legacy/gpu/` as the reference; `amdgpu/`, `video_core/`, `renderer_vulkan/` and `shim/`
//! will mirror them here.

pub mod amdgpu;
pub mod bbgpu;
pub mod bbnet;
pub mod common;
pub mod core;
pub mod dlss_bridge;
pub mod shader_recompiler;
pub mod shim;
pub mod tools;
pub mod video_core;
