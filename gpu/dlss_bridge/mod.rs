//! ABI of the runtime-loaded DLSS bridge (`bbport_dlss.dll`), ported from
//! `legacy/gpu/dlss_bridge/bbport_dlss_bridge.h`.
//!
//! The port itself contains no NVIDIA code: the renderer loads this table at run time when the
//! bridge and `nvngx_dlss.dll` are present, and keeps its other upscalers otherwise. Vulkan
//! handles are represented with their C scalar types so this module has no dependency on a
//! Vulkan binding crate.

use std::ffi::{c_char, c_void};

/// ABI 2 adds DLSS Frame Generation (the `FrameGen*` members); an ABI 1 bridge has DLSS only.
pub const BBPORT_DLSS_BRIDGE_ABI: u32 = 2;

pub type VkImage = u64;
pub type VkImageView = u64;
pub type VkFormat = u32;
pub type VkInstance = *mut c_void;
pub type VkPhysicalDevice = *mut c_void;
pub type VkDevice = *mut c_void;
pub type VkCommandBuffer = *mut c_void;
pub type VkGetInstanceProcAddr =
    Option<unsafe extern "C" fn(VkInstance, *const c_char) -> *mut c_void>;
pub type VkGetDeviceProcAddr = Option<unsafe extern "C" fn(VkDevice, *const c_char) -> *mut c_void>;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VkImageSubresourceRange {
    pub aspect_mask: u32,
    pub base_mip_level: u32,
    pub level_count: u32,
    pub base_array_layer: u32,
    pub layer_count: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VkExtensionProperties {
    pub extension_name: [c_char; 256],
    pub spec_version: u32,
}

pub type BbDlssLogFn = Option<unsafe extern "C" fn(warning: i32, message: *const c_char)>;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BbDlssImage {
    pub image: VkImage,
    pub view: VkImageView,
    pub range: VkImageSubresourceRange,
    pub format: VkFormat,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BbDlssFeature {
    pub input_width: u32,
    pub input_height: u32,
    pub output_width: u32,
    pub output_height: u32,
    /// 0 DLAA, 1 Quality, 2 Balanced, 3 Performance, 4 Ultra Performance.
    pub quality: i32,
    pub depth_inverted: i32,
    /// `NVSDK_NGX_DLSS_Hint_Render_Preset`, 0 = driver default.
    pub preset: u32,
    /// 1: linear HDR colour with automatic exposure, 0: tonemapped colour.
    pub hdr: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BbDlssEvaluate {
    pub color: BbDlssImage,
    pub depth: BbDlssImage,
    pub motion: BbDlssImage,
    pub output: BbDlssImage,
    pub jitter_x: f32,
    pub jitter_y: f32,
    pub reset: i32,
    pub frame_ms: f32,
    /// 0..1; 0 leaves the output unsharpened.
    pub sharpness: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BbDlssgFeature {
    /// Backbuffer: input and output colour.
    pub width: u32,
    pub height: u32,
    pub format: VkFormat,
    /// Depth and motion vectors.
    pub render_width: u32,
    pub render_height: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BbDlssgEvaluate {
    /// Sampled, general layout: the finished frame (UI included).
    pub backbuffer: BbDlssImage,
    /// Sampled, hardware depth values (a depth or R32F image).
    pub depth: BbDlssImage,
    /// Sampled, render-resolution pixels, current to previous.
    pub motion: BbDlssImage,
    /// Storage, general layout: the generated frame.
    pub output: BbDlssImage,
    /// Sampled: the backbuffer before the UI; image null when none.
    pub hudless: BbDlssImage,
    pub jitter_x: f32,
    pub jitter_y: f32,
    pub camera_near: f32,
    pub camera_far: f32,
    /// Vertical, radians.
    pub camera_fov: f32,
    pub depth_inverted: i32,
    pub reset: i32,
    /// Frames generated for this pair (1: 2x .. 3: 4x).
    pub multi_frame_count: u32,
    /// 1..=multi_frame_count, in order.
    pub multi_frame_index: u32,
    /// Row-major, row vectors (`v * m`), D3D clip space (y up, depth 0 at the near plane).
    pub has_matrices: i32,
    pub view_to_clip: [[f32; 4]; 4],
    pub clip_to_prev_clip: [[f32; 4]; 4],
    pub prev_clip_to_clip: [[f32; 4]; 4],
}

/// The bridge's function table (`BbDlssApi`).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BbDlssApi {
    pub abi: u32,
    /// UTF-16 paths: the directory with `nvngx_dlss.dll`, and a writable data directory.
    pub configure: Option<
        unsafe extern "C" fn(
            dll_directory: *const u16,
            data_directory: *const u16,
            log: BbDlssLogFn,
        ) -> i32,
    >,
    /// Vulkan extensions NGX needs (DLSS and, when the GPU has it, Frame Generation).
    pub instance_extensions: Option<
        unsafe extern "C" fn(count: *mut u32, extensions: *mut *const VkExtensionProperties) -> i32,
    >,
    pub device_extensions: Option<
        unsafe extern "C" fn(
            instance: VkInstance,
            physical: VkPhysicalDevice,
            count: *mut u32,
            extensions: *mut *const VkExtensionProperties,
        ) -> i32,
    >,
    pub initialize: Option<
        unsafe extern "C" fn(
            instance: VkInstance,
            physical: VkPhysicalDevice,
            device: VkDevice,
            get_instance_proc: VkGetInstanceProcAddr,
            get_device_proc: VkGetDeviceProcAddr,
        ) -> i32,
    >,
    pub create_feature: Option<
        unsafe extern "C" fn(command: VkCommandBuffer, feature: *const BbDlssFeature) -> i32,
    >,
    pub evaluate: Option<
        unsafe extern "C" fn(command: VkCommandBuffer, evaluate: *const BbDlssEvaluate) -> i32,
    >,
    /// The caller must have waited for the GPU work that used the feature.
    pub release_feature: Option<unsafe extern "C" fn()>,
    pub shutdown: Option<unsafe extern "C" fn()>,
    /// Frames Frame Generation can make per pair after Initialize: 0, 1 (2x), 3 (up to 4x).
    pub frame_gen_max_count: Option<unsafe extern "C" fn() -> i32>,
    pub frame_gen_create: Option<
        unsafe extern "C" fn(command: VkCommandBuffer, feature: *const BbDlssgFeature) -> i32,
    >,
    /// Call `multi_frame_count` times per rendered frame, index 1, 2, ...
    pub frame_gen_evaluate: Option<
        unsafe extern "C" fn(command: VkCommandBuffer, evaluate: *const BbDlssgEvaluate) -> i32,
    >,
    /// The caller must have waited for the GPU work that used the feature.
    pub frame_gen_release: Option<unsafe extern "C" fn()>,
}

pub type BbDlssGetApiFn = Option<unsafe extern "C" fn() -> *const BbDlssApi>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn abi_constant_matches_the_header() {
        assert_eq!(BBPORT_DLSS_BRIDGE_ABI, 2);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn layout_matches_the_c_header_on_64_bit() {
        assert_eq!(size_of::<VkImageSubresourceRange>(), 20);
        assert_eq!(size_of::<VkExtensionProperties>(), 260);
        assert_eq!(size_of::<BbDlssImage>(), 48);
        assert_eq!(size_of::<BbDlssFeature>(), 32);
        assert_eq!(size_of::<BbDlssEvaluate>(), 216);
        assert_eq!(size_of::<BbDlssgFeature>(), 20);
        assert_eq!(size_of::<BbDlssgEvaluate>(), 472);
        assert_eq!(size_of::<BbDlssApi>(), 104);
    }
}

/// The loaded bridge state (`bbport_dlss_bridge.cpp`).
#[derive(Debug, Default)]
pub struct BridgeState {
    pub abi: u32,
    pub loaded: bool,
    pub frame_generation_available: bool,
}

impl BridgeState {
    pub fn loaded(abi: u32) -> Self {
        Self {
            abi,
            loaded: true,
            frame_generation_available: abi >= 2,
        }
    }

    pub fn supports_frame_generation(&self) -> bool {
        self.loaded && self.frame_generation_available
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn bridge_abi_controls_frame_generation() {
        assert!(!BridgeState::default().loaded);
        let bridge = BridgeState::loaded(1);
        assert!(bridge.loaded);
        assert!(!bridge.supports_frame_generation());
        assert!(BridgeState::loaded(2).supports_frame_generation());
        assert_eq!(BBPORT_DLSS_BRIDGE_ABI, 2);
    }
}
