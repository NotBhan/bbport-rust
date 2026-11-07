//! Video-core settings, ported from `legacy/gpu/shim/core/emulator_settings.h`.
//! Read once from `BB_*` environment variables; defaults match shadPS4 except the pipeline
//! cache, which is on.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum GpuReadbacksMode {
    Disabled = 0,
    Relaxed = 1,
    Precise = 2,
}

fn display_refresh() -> &'static OnceLock<u32> {
    static HZ: OnceLock<u32> = OnceLock::new();
    &HZ
}

/// `BbDisplayRefreshHz()`: the primary display refresh rate, 60 when unknown.
pub fn display_refresh_hz() -> u32 {
    *display_refresh().get_or_init(|| 60)
}

/// The video backend reports the real refresh rate here.
pub fn set_display_refresh_hz(hz: u32) {
    let _ = display_refresh().set(hz.max(1));
}

/// `EmulatorSettingsImpl::Flag`: "1", "y" or "t" enable; anything else falls back.
pub fn flag(name: &str, fallback: bool) -> bool {
    match std::env::var(name) {
        Ok(value) => matches!(value.as_bytes().first(), Some(b'1' | b'y' | b't')),
        Err(_) => fallback,
    }
}

/// `EmulatorSettingsImpl::Number`.
pub fn number(name: &str, fallback: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(fallback)
}

/// `EmulatorSettingsImpl::Text`; an empty value is treated as unset.
pub fn text(name: &str, fallback: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn cached_u32(cell: &'static OnceLock<u32>, name: &str, fallback: i64) -> u32 {
    *cell.get_or_init(|| number(name, fallback).max(0) as u32)
}

fn cached_i32(cell: &'static OnceLock<i32>, name: &str, fallback: i64) -> i32 {
    *cell.get_or_init(|| number(name, fallback) as i32)
}

fn cached_bool(cell: &'static OnceLock<bool>, name: &str, fallback: bool) -> bool {
    *cell.get_or_init(|| flag(name, fallback))
}

pub struct EmulatorSettingsImpl {
    shadnet_session_disabled: AtomicBool,
}

impl Default for EmulatorSettingsImpl {
    fn default() -> Self {
        Self {
            shadnet_session_disabled: AtomicBool::new(false),
        }
    }
}

impl EmulatorSettingsImpl {
    pub fn get_gpu_id(&self) -> i32 {
        static VALUE: OnceLock<i32> = OnceLock::new();
        cached_i32(&VALUE, "BB_GPU_ID", -1)
    }

    pub fn get_internal_screen_width(&self) -> u32 {
        static VALUE: OnceLock<u32> = OnceLock::new();
        cached_u32(&VALUE, "BB_INTERNAL_WIDTH", 1920)
    }

    pub fn get_internal_screen_height(&self) -> u32 {
        static VALUE: OnceLock<u32> = OnceLock::new();
        cached_u32(&VALUE, "BB_INTERNAL_HEIGHT", 1080)
    }

    pub fn get_present_mode(&self) -> String {
        text("BB_PRESENT_MODE", "Mailbox")
    }

    pub fn get_rcas_attenuation(&self) -> i32 {
        static VALUE: OnceLock<i32> = OnceLock::new();
        cached_i32(&VALUE, "BB_RCAS_ATTENUATION", 250)
    }

    /// Relaxed by default: without readbacks FaceGen reads stale GPU-written vertices.
    pub fn get_readbacks_mode(&self) -> u32 {
        static VALUE: OnceLock<u32> = OnceLock::new();
        cached_u32(&VALUE, "BB_READBACKS", GpuReadbacksMode::Relaxed as i64)
    }

    /// `BB_VBLANK_HZ=0` runs vblank at 480 Hz so a finished frame is shown within ~2 ms.
    pub fn get_vblank_frequency(&self) -> u32 {
        static VALUE: OnceLock<u32> = OnceLock::new();
        *VALUE.get_or_init(|| {
            let hz = number("BB_VBLANK_HZ", 60);
            if hz > 0 {
                hz as u32
            } else {
                480
            }
        })
    }

    /// Frames per second the present thread lets through; 0 = no limit.
    pub fn get_frame_limit(&self) -> u32 {
        static VALUE: OnceLock<u32> = OnceLock::new();
        *VALUE.get_or_init(|| {
            let limit = number("BB_FPS_LIMIT", -1);
            if limit >= 0 {
                return limit as u32;
            }
            let fps = text("BB_FPS", "");
            match fps.as_str() {
                "30" => return 30,
                "60" => return 60,
                "90" => return 90,
                "uncap" => return display_refresh_hz().min(120),
                "" => {}
                other => {
                    if let Ok(parsed) = other.parse::<i64>() {
                        if parsed >= 0 {
                            return parsed as u32;
                        }
                    }
                }
            }
            let vblank = number("BB_VBLANK_HZ", 60);
            if vblank > 0 {
                vblank as u32
            } else {
                display_refresh_hz().min(120)
            }
        })
    }

    pub fn is_copy_gpu_buffers(&self) -> bool {
        static VALUE: OnceLock<bool> = OnceLock::new();
        cached_bool(&VALUE, "BB_COPY_GPU_BUFFERS", true)
    }
    pub fn is_direct_memory_access_enabled(&self) -> bool {
        static VALUE: OnceLock<bool> = OnceLock::new();
        cached_bool(&VALUE, "BB_DIRECT_MEMORY_ACCESS", false)
    }
    pub fn is_dump_shaders(&self) -> bool {
        static VALUE: OnceLock<bool> = OnceLock::new();
        cached_bool(&VALUE, "BB_DUMP_SHADERS", false)
    }
    pub fn is_fsr_enabled(&self) -> bool {
        static VALUE: OnceLock<bool> = OnceLock::new();
        cached_bool(&VALUE, "BB_FSR1", false)
    }
    pub fn is_hdr_allowed(&self) -> bool {
        static VALUE: OnceLock<bool> = OnceLock::new();
        cached_bool(&VALUE, "BB_HDR", false)
    }
    pub fn is_hdr_gamma_enabled(&self) -> bool {
        static VALUE: OnceLock<bool> = OnceLock::new();
        cached_bool(&VALUE, "BB_HDR_GAMMA", true)
    }
    pub fn is_null_gpu(&self) -> bool {
        static VALUE: OnceLock<bool> = OnceLock::new();
        cached_bool(&VALUE, "BB_NULL_GPU", false)
    }
    pub const fn is_patch_shaders(&self) -> bool {
        false
    }
    pub const fn is_pipeline_cache_archived(&self) -> bool {
        false
    }
    pub fn is_pipeline_cache_enabled(&self) -> bool {
        static VALUE: OnceLock<bool> = OnceLock::new();
        cached_bool(&VALUE, "BB_PIPELINE_CACHE", true)
    }
    pub fn is_rcas_enabled(&self) -> bool {
        static VALUE: OnceLock<bool> = OnceLock::new();
        cached_bool(&VALUE, "BB_RCAS", true)
    }
    pub fn is_readback_linear_images_enabled(&self) -> bool {
        static VALUE: OnceLock<bool> = OnceLock::new();
        cached_bool(&VALUE, "BB_READBACK_LINEAR", false)
    }
    pub const fn is_renderdoc_enabled(&self) -> bool {
        false
    }
    pub const fn is_shader_collect(&self) -> bool {
        false
    }
    pub const fn is_userfaultfd_tracking(&self) -> bool {
        false
    }
    pub const fn is_vk_crash_diagnostic_enabled(&self) -> bool {
        false
    }
    pub fn is_vk_guest_markers_enabled(&self) -> bool {
        static VALUE: OnceLock<bool> = OnceLock::new();
        cached_bool(&VALUE, "BB_VK_MARKERS", false)
    }
    pub fn is_vk_host_markers_enabled(&self) -> bool {
        static VALUE: OnceLock<bool> = OnceLock::new();
        cached_bool(&VALUE, "BB_VK_MARKERS", false)
    }
    pub const fn is_vk_validation_core_enabled(&self) -> bool {
        true
    }
    pub fn is_vk_validation_enabled(&self) -> bool {
        static VALUE: OnceLock<bool> = OnceLock::new();
        cached_bool(&VALUE, "BB_VK_VALIDATION", false)
    }
    pub const fn is_vk_validation_gpu_enabled(&self) -> bool {
        false
    }
    pub fn is_vk_validation_sync_enabled(&self) -> bool {
        static VALUE: OnceLock<bool> = OnceLock::new();
        cached_bool(&VALUE, "BB_VK_VALIDATION_SYNC", false)
    }
    pub fn is_connected_to_network(&self) -> bool {
        static VALUE: OnceLock<bool> = OnceLock::new();
        cached_bool(&VALUE, "BB_ONLINE", false)
    }
    pub fn is_shadnet_enabled(&self) -> bool {
        self.is_connected_to_network() && !self.shadnet_session_disabled.load(Ordering::Relaxed)
    }
    pub fn set_shadnet_session_disabled(&self, value: bool) {
        self.shadnet_session_disabled
            .store(value, Ordering::Relaxed);
    }
    pub fn get_shadnet_server(&self) -> String {
        text("BB_SHADNET_SERVER", "127.0.0.1:31313")
    }
    pub fn get_shadnet_web_api_server(&self) -> String {
        text("BB_SHADNET_WEBAPI", "http://127.0.0.1:31315")
    }
    pub fn is_upnp_enabled(&self) -> bool {
        static VALUE: OnceLock<bool> = OnceLock::new();
        cached_bool(&VALUE, "BB_UPNP", true)
    }

    /// Background work should not spin; kept here so timing knobs live with the settings.
    pub fn frame_limit_interval(&self) -> Option<Duration> {
        let limit = self.get_frame_limit();
        (limit > 0).then(|| Duration::from_secs_f64(1.0 / f64::from(limit)))
    }
}

pub fn emulator_settings() -> &'static EmulatorSettingsImpl {
    static SETTINGS: OnceLock<EmulatorSettingsImpl> = OnceLock::new();
    SETTINGS.get_or_init(EmulatorSettingsImpl::default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_read_the_first_character_like_the_c_port() {
        std::env::set_var("BB_TEST_FLAG_ON", "yes");
        std::env::set_var("BB_TEST_FLAG_OFF", "0");
        assert!(flag("BB_TEST_FLAG_ON", false));
        assert!(!flag("BB_TEST_FLAG_OFF", true));
        assert!(flag("BB_TEST_FLAG_MISSING", true));
        std::env::remove_var("BB_TEST_FLAG_ON");
        std::env::remove_var("BB_TEST_FLAG_OFF");
    }

    #[test]
    fn numbers_and_text_fall_back_when_unset() {
        assert_eq!(number("BB_TEST_NUMBER_MISSING", 42), 42);
        assert_eq!(text("BB_TEST_TEXT_MISSING", "Mailbox"), "Mailbox");
        std::env::set_var("BB_TEST_NUMBER_SET", "17");
        assert_eq!(number("BB_TEST_NUMBER_SET", 42), 17);
        std::env::remove_var("BB_TEST_NUMBER_SET");
    }

    #[test]
    fn pure_getters_match_the_c_defaults() {
        let settings = EmulatorSettingsImpl::default();
        assert!(settings.is_vk_validation_core_enabled());
        assert!(!settings.is_vk_validation_gpu_enabled());
        assert!(!settings.is_patch_shaders());
        assert!(!settings.is_renderdoc_enabled());
        assert_eq!(display_refresh_hz(), 60);
    }
}
