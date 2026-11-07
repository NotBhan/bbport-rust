//! Vulkan format compatibility, ported from
//! `legacy/gpu/shadps4/video_core/texture_cache/host_compatibility.h`.
//!
//! Vulkan lets a view use a format other than the image's when the two are *compatible*:
//! same component count and widths, differing only in how the components are interpreted.
//! Depth-only and depth-stencil formats form two classes of their own.

/// The format's class: what the components are and how wide they are.
fn format_class(format: &str) -> FormatClass {
    if format.contains("_S8_UINT") {
        return FormatClass::DepthStencil;
    }
    if format.starts_with('D') || format.starts_with("X8_D24") {
        return FormatClass::DepthOnly;
    }
    if format.contains("_BLOCK") {
        // A block-compressed format: the whole string is already the class.
        return FormatClass::Exact(format.to_string());
    }
    match format.split_once('_') {
        // The interpretation suffix is the part after the components ("R8G8B8A8_SRGB").
        Some((components, _)) => FormatClass::Components(components.to_string()),
        None => FormatClass::Exact(format.to_string()),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum FormatClass {
    Components(String),
    DepthOnly,
    DepthStencil,
    Exact(String),
}

/// `VideoCore::IsVulkanFormatCompatible`: whether a view may use `view` over a `base` image.
pub fn is_vulkan_format_compatible(base: &str, view: &str) -> bool {
    if base == view {
        return true;
    }
    match (format_class(base), format_class(view)) {
        (FormatClass::DepthStencil, FormatClass::DepthStencil) => true,
        (FormatClass::DepthOnly, FormatClass::DepthOnly) => true,
        (FormatClass::Components(left), FormatClass::Components(right)) => left == right,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_differing_only_in_interpretation_are_compatible() {
        assert!(is_vulkan_format_compatible(
            "R8G8B8A8_UNORM",
            "R8G8B8A8_SRGB"
        ));
        assert!(is_vulkan_format_compatible(
            "B8G8R8A8_SRGB",
            "B8G8R8A8_UNORM"
        ));
        assert!(is_vulkan_format_compatible(
            "R16G16B16A16_SFLOAT",
            "R16G16B16A16_SNORM"
        ));
        assert!(is_vulkan_format_compatible("R32_UINT", "R32_SFLOAT"));
        assert!(is_vulkan_format_compatible(
            "R8G8B8A8_UNORM",
            "R8G8B8A8_UNORM"
        ));
    }

    #[test]
    fn different_components_or_classes_are_not() {
        assert!(!is_vulkan_format_compatible(
            "R8G8B8A8_UNORM",
            "B8G8R8A8_UNORM"
        ));
        assert!(!is_vulkan_format_compatible(
            "R8G8B8A8_UNORM",
            "R8G8B8_UNORM"
        ));
        assert!(!is_vulkan_format_compatible("R16_UNORM", "R8_UNORM"));
        assert!(!is_vulkan_format_compatible(
            "BC1_RGBA_UNORM_BLOCK",
            "BC1_RGBA_SRGB_BLOCK"
        ));
        assert!(!is_vulkan_format_compatible("R8G8B8A8_UNORM", "D32_SFLOAT"));
    }

    #[test]
    fn the_depth_classes_accept_each_other() {
        assert!(is_vulkan_format_compatible(
            "D24_UNORM_S8_UINT",
            "D32_SFLOAT_S8_UINT"
        ));
        assert!(is_vulkan_format_compatible("D16_UNORM", "D32_SFLOAT"));
        assert!(is_vulkan_format_compatible(
            "X8_D24_UNORM_PACK32",
            "D16_UNORM"
        ));
        assert!(!is_vulkan_format_compatible(
            "D16_UNORM",
            "D24_UNORM_S8_UINT"
        ));
    }
}
