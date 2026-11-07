//! VideoOut API types, ported from `shadps4/core/libraries/videoout/video_out.h`.

pub const SCE_VIDEO_OUT_BUS_TYPE_MAIN: i32 = 0;
pub const SCE_VIDEO_OUT_BUS_TYPE_AUX_SOCIAL_SCREEN: i32 = 5;
pub const SCE_VIDEO_OUT_BUS_TYPE_AUX_GAME_LIVE_STREAMING: i32 = 6;

pub const SCE_VIDEO_OUT_REFRESH_RATE_UNKNOWN: i32 = 0;
pub const SCE_VIDEO_OUT_REFRESH_RATE_23_98HZ: i32 = 1;
pub const SCE_VIDEO_OUT_REFRESH_RATE_50HZ: i32 = 2;
pub const SCE_VIDEO_OUT_REFRESH_RATE_59_94HZ: i32 = 3;
pub const SCE_VIDEO_OUT_REFRESH_RATE_119_88HZ: i32 = 13;
pub const SCE_VIDEO_OUT_REFRESH_RATE_89_91HZ: i32 = 35;
pub const SCE_VIDEO_OUT_REFRESH_RATE_ANY: i64 = 0xFFFF_FFFF_FFFF_FFFFu64 as i64;

pub const SCE_VIDEO_OUT_PIXEL_FORMAT_A8R8G8B8_SRGB: i32 = 0x8000_0000u32 as i32;
pub const SCE_VIDEO_OUT_PIXEL_FORMAT_A8B8G8R8_SRGB: i32 = 0x8000_2200u32 as i32;
pub const SCE_VIDEO_OUT_PIXEL_FORMAT_A2R10G10B10: i32 = 0x8806_0000u32 as i32;
pub const SCE_VIDEO_OUT_PIXEL_FORMAT_A2R10G10B10_SRGB: i32 = 0x8800_0000u32 as i32;
pub const SCE_VIDEO_OUT_PIXEL_FORMAT_A2R10G10B10_BT2020_PQ: i32 = 0x8874_0000u32 as i32;
pub const SCE_VIDEO_OUT_PIXEL_FORMAT_A16R16G16B16_FLOAT: i32 = 0xC106_0000u32 as i32;
pub const SCE_VIDEO_OUT_PIXEL_FORMAT_YCBCR420_BT709: i32 = 0x0832_2200;

pub const SCE_VIDEO_OUT_BUFFER_ATTRIBUTE_OPTION_NONE: i32 = 0;
pub const SCE_VIDEO_OUT_BUFFER_ATTRIBUTE_OPTION_VR: i32 = 7;
pub const SCE_VIDEO_OUT_BUFFER_ATTRIBUTE_OPTION_STRICT_COLORIMETRY: i32 = 8;

pub const ORBIS_VIDEO_OUT_DEVICE_CAPABILITY_BT2020_PQ: i32 = 0x80;

/// `OrbisVideoOutColorimetry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OrbisVideoOutColorimetry {
    Bt2020Pq = 12,
    Any = 0xFF,
}

/// `OrbisVideoOutEventId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum OrbisVideoOutEventId {
    Flip = 0,
    Vblank = 1,
    PreVblankStart = 2,
    SetMode = 8,
    Position = 12,
}

/// `OrbisVideoOutInternalEventId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum OrbisVideoOutInternalEventId {
    Flip = 0x6,
    Vblank = 0x7,
    SetMode = 0x51,
    Position = 0x58,
    PreVblankStart = 0x59,
    SysVblank = 0x63,
}

/// `AspectRatioMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum AspectRatioMode {
    Ratio16_9 = 0,
}

/// `FlipStatus`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlipStatus {
    pub count: u64,
    pub process_time: u64,
    pub tsc: u64,
    pub flip_arg: i64,
    pub submit_tsc: u64,
    pub reserved0: u64,
    pub gc_queue_num: i32,
    pub flip_pending_num: i32,
    pub current_buffer: i32,
    pub reserved1: u32,
}

impl Default for FlipStatus {
    fn default() -> Self {
        Self {
            count: 0,
            process_time: 0,
            tsc: 0,
            flip_arg: -1,
            submit_tsc: 0,
            reserved0: 0,
            gc_queue_num: 0,
            flip_pending_num: 0,
            current_buffer: -1,
            reserved1: 0,
        }
    }
}

/// `SceVideoOutResolutionStatus`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceVideoOutResolutionStatus {
    pub full_width: i32,
    pub full_height: i32,
    pub pane_width: i32,
    pub pane_height: i32,
    pub refresh_rate: u64,
    pub screen_size_in_inch: f32,
    pub flags: u16,
    pub reserved0: u16,
    pub reserved1: [u32; 3],
}

impl Default for SceVideoOutResolutionStatus {
    fn default() -> Self {
        Self {
            full_width: 1280,
            full_height: 720,
            pane_width: 1280,
            pane_height: 720,
            refresh_rate: SCE_VIDEO_OUT_REFRESH_RATE_59_94HZ as u64,
            screen_size_in_inch: 50.0,
            flags: 0,
            reserved0: 0,
            reserved1: [0; 3],
        }
    }
}

/// `SceVideoOutVblankStatus`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SceVideoOutVblankStatus {
    pub count: u64,
    pub process_time: u64,
    pub tsc: u64,
    pub reserved: [u64; 1],
    pub flags: u8,
    pub pad1: [u8; 7],
}

/// `SceVideoOutDeviceCapabilityInfo`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SceVideoOutDeviceCapabilityInfo {
    pub capability: u64,
}

/// `SceVideoOutColorSettings`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SceVideoOutColorSettings {
    pub gamma: f32,
    pub reserved: [u32; 3],
}

/// `OrbisVideoOutEventData`: the packed `time`/`count`/`flip_arg` bitfield.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisVideoOutEventData {
    pub raw: u64,
}

impl OrbisVideoOutEventData {
    pub fn time(&self) -> u64 {
        self.raw & 0xFFF
    }

    pub fn count(&self) -> u64 {
        (self.raw >> 12) & 0xF
    }

    pub fn flip_arg(&self) -> u64 {
        self.raw >> 16
    }
}

/// Some of the VideoOut entry points; the guest HLE layer implements the rest.
pub trait VideoOutApi {
    fn set_flip_rate(&mut self, handle: i32, rate: i32) -> i32;
    fn is_flip_pending(&mut self, handle: i32) -> i32;
    fn wait_vblank(&mut self, handle: i32) -> i32;
    fn submit_flip(&mut self, handle: i32, buffer_index: i32, flip_mode: i32, flip_arg: i64)
        -> i32;
    fn get_flip_status(&mut self, handle: i32, status: &mut FlipStatus) -> i32;
    fn get_resolution_status(
        &mut self,
        handle: i32,
        status: &mut SceVideoOutResolutionStatus,
    ) -> i32;
    fn open(&mut self, user_id: i32, bus_type: i32, index: i32) -> i32;
    fn close(&mut self, handle: i32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_constants_match_the_header() {
        assert_eq!(SCE_VIDEO_OUT_REFRESH_RATE_ANY, -1i64);
        assert_eq!(SCE_VIDEO_OUT_PIXEL_FORMAT_YCBCR420_BT709, 0x08322200);
        assert_eq!(OrbisVideoOutColorimetry::Bt2020Pq as u8, 12);
        assert_eq!(OrbisVideoOutInternalEventId::SysVblank as i16, 0x63);
    }

    #[test]
    fn the_status_structs_have_their_layouts() {
        assert_eq!(core::mem::size_of::<FlipStatus>(), 64);
        assert_eq!(core::mem::size_of::<SceVideoOutResolutionStatus>(), 48);
        assert_eq!(core::mem::size_of::<SceVideoOutVblankStatus>(), 40);
        assert_eq!(core::mem::size_of::<SceVideoOutColorSettings>(), 16);
        assert_eq!(FlipStatus::default().flip_arg, -1);
        assert_eq!(SceVideoOutResolutionStatus::default().full_width, 1280);
    }

    #[test]
    fn the_event_data_bitfield_is_packed() {
        let data = OrbisVideoOutEventData {
            raw: 0x1234_5678_9ABC_DEF0,
        };
        assert_eq!(data.time(), 0xEF0);
        assert_eq!(data.count(), 0xD);
        assert_eq!(data.flip_arg(), 0x1234_5678_9ABC);
    }
}

/// The `video_out.cpp` state: registered display buffers and flip count.
#[derive(Debug, Default)]
pub struct VideoOutState {
    pub buffers: Vec<u64>,
    pub flips: u64,
    pub open: bool,
    pub port: i32,
    pub width: u32,
    pub height: u32,
    pub pixel_format: u32,
    pub tiling: u32,
}

impl VideoOutState {
    pub fn open(&mut self, port: i32, width: u32, height: u32) -> i32 {
        self.open = true;
        self.port = port;
        self.width = width;
        self.height = height;
        0
    }

    pub fn close(&mut self) -> i32 {
        self.open = false;
        0
    }

    pub fn register_buffer(&mut self, address: u64) -> usize {
        if !self.buffers.contains(&address) {
            self.buffers.push(address);
        }
        self.buffers.len() - 1
    }

    pub fn flip(&mut self, address: u64) -> i32 {
        if !self.open || !self.buffers.contains(&address) {
            return -1;
        }
        self.flips += 1;
        0
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn videoout_registers_buffers_and_flips() {
        let mut state = VideoOutState::default();
        assert_eq!(state.flip(0x1000), -1);
        assert_eq!(state.open(0, 1920, 1080), 0);
        assert_eq!(state.register_buffer(0x1000), 0);
        assert_eq!(state.flip(0x1000), 0);
        assert_eq!(state.flips, 1);
        assert_eq!(state.close(), 0);
    }
}
