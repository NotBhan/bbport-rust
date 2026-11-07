//! Display buffer descriptions, ported from
//! `shadps4/core/libraries/videoout/buffer.h`.

pub const MAX_DISPLAY_BUFFERS: usize = 16;
pub const MAX_DISPLAY_BUFFER_GROUPS: usize = 4;

/// `PixelFormat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PixelFormat {
    Unknown = 0,
    A8R8G8B8Srgb = 0x8000_0000,
    A8B8G8R8Srgb = 0x8000_2200,
    A2R10G10B10 = 0x8806_0000,
    A2R10G10B10Srgb = 0x8800_0000,
    A2R10G10B10Bt2020Pq = 0x8874_0000,
    A16R16G16B16Float = 0xC106_0000,
}

/// `TilingMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum TilingMode {
    Tile = 0,
    Linear = 1,
}

/// `GetPixelFormatString`.
pub fn pixel_format_string(format: PixelFormat) -> &'static str {
    match format {
        PixelFormat::A8R8G8B8Srgb => "A8R8G8B8Srgb",
        PixelFormat::A8B8G8R8Srgb => "A8B8G8R8Srgb",
        PixelFormat::A2R10G10B10 => "A2R10G10B10",
        PixelFormat::A2R10G10B10Srgb => "A2R10G10B10Srgb",
        PixelFormat::A2R10G10B10Bt2020Pq => "A2R10G10B10Bt2020Pq",
        PixelFormat::A16R16G16B16Float => "A16R16G16B16Float",
        PixelFormat::Unknown => "Unknown",
    }
}

/// `BufferAttribute`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferAttribute {
    pub pixel_format: PixelFormat,
    pub tiling_mode: TilingMode,
    pub aspect_ratio: i32,
    pub width: u32,
    pub height: u32,
    pub pitch_in_pixel: u32,
    pub option: u32,
    pub reserved0: u32,
    pub reserved1: u64,
}

/// `BufferAttributeGroup`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferAttributeGroup {
    pub is_occupied: bool,
    pub attrib: BufferAttribute,
}

/// `VideoOutBuffer`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoOutBuffer {
    pub group_index: i32,
    pub address_left: usize,
    pub address_right: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_format_values_and_names_match() {
        assert_eq!(PixelFormat::A8R8G8B8Srgb as u32, 0x80000000);
        assert_eq!(PixelFormat::A16R16G16B16Float as u32, 0xC1060000);
        assert_eq!(pixel_format_string(PixelFormat::A2R10G10B10), "A2R10G10B10");
        assert_eq!(pixel_format_string(PixelFormat::Unknown), "Unknown");
        assert_eq!(TilingMode::Linear as i32, 1);
        assert_eq!(MAX_DISPLAY_BUFFERS, 16);
        assert_eq!(MAX_DISPLAY_BUFFER_GROUPS, 4);
    }

    #[test]
    fn the_buffer_layouts_are_stable() {
        assert_eq!(core::mem::size_of::<BufferAttribute>(), 40);
        assert_eq!(core::mem::size_of::<BufferAttributeGroup>(), 48);
        assert_eq!(core::mem::size_of::<VideoOutBuffer>(), 24);
        assert_eq!(core::mem::offset_of!(VideoOutBuffer, address_left), 8);
    }
}
