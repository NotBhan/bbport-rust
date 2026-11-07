//! Vulkan platform helpers, ported from
//! `shadps4/video_core/renderer_vulkan/vk_platform.h`.
//!
//! The Vulkan-Hpp `vk::` types are replaced by success flags at this boundary; the
//! renderer supplies the actual handles.

pub const TARGET_VULKAN_API_VERSION: u32 = (1 << 22) | (3 << 12);

/// `Check` for a bare result.
#[track_caller]
pub fn check(result_success: bool) {
    assert!(result_success, "vk::Result failure");
}

/// `Check` for a result/value pair.
#[track_caller]
pub fn check_value<T>(result_success: bool, value: T) -> T {
    assert!(result_success, "vk::Result failure");
    value
}

/// `Check` with a context message.
#[track_caller]
pub fn check_msg(result_success: bool, message: &str) {
    assert!(result_success, "Failed to {message}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_platform_constants_match_the_header() {
        assert_eq!(TARGET_VULKAN_API_VERSION, 0x0040_3000);
        check(true);
        assert_eq!(check_value(true, 7), 7);
        check_msg(true, "create device");
    }

    #[test]
    #[should_panic(expected = "Failed to create device")]
    fn check_msg_reports_its_context() {
        check_msg(false, "create device");
    }
}
