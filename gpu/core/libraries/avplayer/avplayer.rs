//! AvPlayer public types, ported from
//! `shadps4/core/libraries/avplayer/avplayer.h`.

use core::ffi::c_void;

/// `AvPlayerHandle`.
pub type AvPlayerHandle = *mut c_void;

/// `AvPlayerUriType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AvPlayerUriType {
    Source = 0,
}

/// `AvPlayerUri`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AvPlayerUri {
    pub name: *const i8,
    pub length: u32,
}

/// `AvPlayerSourceType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AvPlayerSourceType {
    Unknown = 0,
    FileMp4 = 1,
    Hls = 8,
}

/// `AvPlayerStreamType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AvPlayerStreamType {
    Video = 0,
    Audio = 1,
    TimedText = 2,
    Unknown = 3,
}

/// `AvPlayerSourceDetails`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AvPlayerSourceDetails {
    pub uri: AvPlayerUri,
    pub reserved1: [u8; 64],
    pub source_type: AvPlayerSourceType,
    pub reserved2: [u8; 44],
}

/// `AvPlayerAudio`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AvPlayerAudio {
    pub channel_count: u16,
    pub reserved1: [u8; 2],
    pub sample_rate: u32,
    pub size: u32,
    pub language_code: [u8; 4],
}

/// `AvPlayerVideo`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AvPlayerVideo {
    pub width: u32,
    pub height: u32,
    pub aspect_ratio: f32,
    pub language_code: [u8; 4],
}

/// `AvPlayerTextPosition`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AvPlayerTextPosition {
    pub top: u16,
    pub left: u16,
    pub bottom: u16,
    pub right: u16,
}

/// `AvPlayerTimedText`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AvPlayerTimedText {
    pub language_code: [u8; 4],
    pub text_size: u16,
    pub font_size: u16,
    pub position: AvPlayerTextPosition,
}

/// `AvPlayerStreamDetails`.
#[repr(C)]
#[derive(Clone, Copy)]
pub union AvPlayerStreamDetails {
    pub reserved: [u8; 16],
    pub audio: AvPlayerAudio,
    pub video: AvPlayerVideo,
    pub subs: AvPlayerTimedText,
}

/// `AvPlayerFrameInfo`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AvPlayerFrameInfo {
    pub p_data: *mut u8,
    pub reserved: [u8; 4],
    pub timestamp: u64,
    pub details: AvPlayerStreamDetails,
}

/// `AvPlayerStreamInfo`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AvPlayerStreamInfo {
    pub stream_type: AvPlayerStreamType,
    pub reserved: [u8; 4],
    pub details: AvPlayerStreamDetails,
    pub duration: u64,
    pub start_time: u64,
}

impl Default for AvPlayerStreamInfo {
    fn default() -> Self {
        Self {
            stream_type: AvPlayerStreamType::Unknown,
            reserved: [0; 4],
            details: AvPlayerStreamDetails { reserved: [0; 16] },
            duration: 0,
            start_time: 0,
        }
    }
}

/// `AvPlayerAudioEx`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AvPlayerAudioEx {
    pub channel_count: u16,
    pub reserved: [u8; 2],
    pub sample_rate: u32,
    pub size: u32,
    pub language_code: [u8; 4],
    pub reserved1: [u8; 64],
}

impl Default for AvPlayerAudioEx {
    fn default() -> Self {
        Self {
            channel_count: 0,
            reserved: [0; 2],
            sample_rate: 0,
            size: 0,
            language_code: [0; 4],
            reserved1: [0; 64],
        }
    }
}

/// `AvPlayerVideoEx`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AvPlayerVideoEx {
    pub width: u32,
    pub height: u32,
    pub aspect_ratio: f32,
    pub language_code: [u8; 4],
    pub framerate: u32,
    pub crop_left_offset: u32,
    pub crop_right_offset: u32,
    pub crop_top_offset: u32,
    pub crop_bottom_offset: u32,
    pub pitch: u32,
    pub luma_bit_depth: u8,
    pub chroma_bit_depth: u8,
    pub video_full_range_flag: bool,
    pub reserved1: [u8; 37],
}

impl Default for AvPlayerVideoEx {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            aspect_ratio: 0.0,
            language_code: [0; 4],
            framerate: 0,
            crop_left_offset: 0,
            crop_right_offset: 0,
            crop_top_offset: 0,
            crop_bottom_offset: 0,
            pitch: 0,
            luma_bit_depth: 0,
            chroma_bit_depth: 0,
            video_full_range_flag: false,
            reserved1: [0; 37],
        }
    }
}

/// `AvPlayerTimedTextEx`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AvPlayerTimedTextEx {
    pub language_code: [u8; 4],
    pub reserved: [u8; 12],
    pub reserved1: [u8; 64],
}

impl Default for AvPlayerTimedTextEx {
    fn default() -> Self {
        Self {
            language_code: [0; 4],
            reserved: [0; 12],
            reserved1: [0; 64],
        }
    }
}

/// `AvPlayerStreamDetailsEx`.
#[repr(C)]
#[derive(Clone, Copy)]
pub union AvPlayerStreamDetailsEx {
    pub audio: AvPlayerAudioEx,
    pub video: AvPlayerVideoEx,
    pub subs: AvPlayerTimedTextEx,
    pub reserved1: [u8; 80],
}

/// `AvPlayerFrameInfoEx`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AvPlayerFrameInfoEx {
    pub p_data: *mut c_void,
    pub reserved: [u8; 4],
    pub timestamp: u64,
    pub details: AvPlayerStreamDetailsEx,
}

pub type AvPlayerAllocate = unsafe extern "C" fn(*mut c_void, u32, u32) -> *mut c_void;
pub type AvPlayerDeallocate = unsafe extern "C" fn(*mut c_void, *mut c_void);
pub type AvPlayerAllocateTexture = unsafe extern "C" fn(*mut c_void, u32, u32) -> *mut c_void;
pub type AvPlayerDeallocateTexture = unsafe extern "C" fn(*mut c_void, *mut c_void);

/// `AvPlayerMemAllocator`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AvPlayerMemAllocator {
    pub object_ptr: *mut c_void,
    pub allocate: Option<AvPlayerAllocate>,
    pub deallocate: Option<AvPlayerDeallocate>,
    pub allocate_texture: Option<AvPlayerAllocateTexture>,
    pub deallocate_texture: Option<AvPlayerDeallocateTexture>,
}

pub type AvPlayerOpenFile = unsafe extern "C" fn(*mut c_void, *const i8) -> i32;
pub type AvPlayerCloseFile = unsafe extern "C" fn(*mut c_void) -> i32;
pub type AvPlayerReadOffsetFile = unsafe extern "C" fn(*mut c_void, *mut u8, u64, u32) -> i32;
pub type AvPlayerSizeFile = unsafe extern "C" fn(*mut c_void) -> u64;

/// `AvPlayerFileReplacement`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AvPlayerFileReplacement {
    pub object_ptr: *mut c_void,
    pub open: Option<AvPlayerOpenFile>,
    pub close: Option<AvPlayerCloseFile>,
    pub read_offset: Option<AvPlayerReadOffsetFile>,
    pub size: Option<AvPlayerSizeFile>,
}

/// `AvPlayerEvents`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AvPlayerEvents {
    StateStop = 0x01,
    StateReady = 0x02,
    StatePlay = 0x03,
    StatePause = 0x04,
    StateBuffering = 0x05,
    TimedTextDelivery = 0x10,
    WarningId = 0x20,
    Encryption = 0x30,
    DrmError = 0x40,
}

pub type AvPlayerEventCallback =
    unsafe extern "C" fn(*mut c_void, AvPlayerEvents, i32, *mut c_void);

/// `AvPlayerEventReplacement`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AvPlayerEventReplacement {
    pub object_ptr: *mut c_void,
    pub event_callback: Option<AvPlayerEventCallback>,
}

/// `AvPlayerDebuglevels`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AvPlayerDebuglevels {
    None = 0,
    Info = 1,
    Warnings = 2,
    All = 3,
}

/// `AvPlayerInitData`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AvPlayerInitData {
    pub memory_replacement: AvPlayerMemAllocator,
    pub file_replacement: AvPlayerFileReplacement,
    pub event_replacement: AvPlayerEventReplacement,
    pub debug_level: AvPlayerDebuglevels,
    pub base_priority: u32,
    pub num_output_video_framebuffers: i32,
    pub auto_start: bool,
    pub reserved: [u8; 3],
    pub default_language: *const i8,
}

impl Default for AvPlayerInitData {
    fn default() -> Self {
        Self {
            memory_replacement: AvPlayerMemAllocator::default(),
            file_replacement: AvPlayerFileReplacement::default(),
            event_replacement: AvPlayerEventReplacement::default(),
            debug_level: AvPlayerDebuglevels::None,
            base_priority: 0,
            num_output_video_framebuffers: 0,
            auto_start: false,
            reserved: [0; 3],
            default_language: std::ptr::null(),
        }
    }
}

/// `AvPlayerInitDataEx`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AvPlayerInitDataEx {
    pub this_size: usize,
    pub memory_replacement: AvPlayerMemAllocator,
    pub file_replacement: AvPlayerFileReplacement,
    pub event_replacement: AvPlayerEventReplacement,
    pub default_language: *const i8,
    pub debug_level: AvPlayerDebuglevels,
    pub audio_decoder_priority: u32,
    pub audio_decoder_affinity: u32,
    pub video_decoder_priority: u32,
    pub video_decoder_affinity: u32,
    pub demuxer_priority: u32,
    pub demuxer_affinity: u32,
    pub controller_priority: u32,
    pub controller_affinity: u32,
    pub http_streaming_priority: u32,
    pub http_streaming_affinity: u32,
    pub file_streaming_priority: u32,
    pub file_streaming_affinity: u32,
    pub num_output_video_framebuffers: i32,
    pub auto_start: bool,
    pub reserved: [u8; 3],
}

/// `AvPlayerVideoDecoderType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AvPlayerVideoDecoderType {
    Default = 0,
    Reserved1 = 1,
    Software = 2,
    Software2 = 3,
}

/// `AvPlayerAudioDecoderType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AvPlayerAudioDecoderType {
    Default = 0,
    Reserved1 = 1,
    Reserved2 = 2,
}

/// The decoder type union.
#[repr(C)]
#[derive(Clone, Copy)]
pub union AvPlayerDecoderType {
    pub video_type: AvPlayerVideoDecoderType,
    pub audio_type: AvPlayerAudioDecoderType,
    pub reserved: [u8; 4],
}

/// `AvcSw2` decoder parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AvPlayerAvcSw2 {
    pub cpu_affinity_mask: i32,
    pub cpu_thread_priority: i32,
    pub decode_pipeline_depth: u8,
    pub compute_pipe_id: u8,
    pub compute_queue_id: u8,
    pub enable_interlaced: u8,
    pub reserved: [u8; 16],
}

/// `Aac` decoder parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AvPlayerAac {
    pub audio_channel_order: u8,
    pub reserved: [u8; 27],
}

/// The decoder parameter union.
#[repr(C)]
#[derive(Clone, Copy)]
pub union AvPlayerDecoderParams {
    pub avc_sw2: AvPlayerAvcSw2,
    pub aac: AvPlayerAac,
    pub reserved: [u8; 28],
}

/// `AvPlayerDecoderInit`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AvPlayerDecoderInit {
    pub decoder_type: AvPlayerDecoderType,
    pub decoder_params: AvPlayerDecoderParams,
}

/// `AvPlayerHTTPCtx`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AvPlayerHTTPCtx {
    pub http_context_id: u32,
    pub ssl_context_id: u32,
}

/// `AvPlayerPostInitData`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AvPlayerPostInitData {
    pub demux_video_buffer_size: u32,
    pub video_decoder_init: AvPlayerDecoderInit,
    pub audio_decoder_init: AvPlayerDecoderInit,
    pub http_context: AvPlayerHTTPCtx,
    pub reserved: [u8; 56],
}

/// `AvPlayerAvSyncMode`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum AvPlayerAvSyncMode {
    #[default]
    Default = 0,
    None = 1,
}

/// `AvPlayerLogCallback`.
pub type AvPlayerLogCallback = unsafe extern "C" fn(*mut c_void, *const i8, *mut c_void) -> i32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_public_structures_have_their_guest_layouts() {
        assert_eq!(core::mem::size_of::<AvPlayerUri>(), 16);
        assert_eq!(core::mem::size_of::<AvPlayerSourceDetails>(), 128);
        assert_eq!(core::mem::size_of::<AvPlayerAudio>(), 16);
        assert_eq!(core::mem::size_of::<AvPlayerVideo>(), 16);
        assert_eq!(core::mem::size_of::<AvPlayerTimedText>(), 16);
        assert_eq!(core::mem::size_of::<AvPlayerStreamDetails>(), 16);
        assert_eq!(core::mem::size_of::<AvPlayerFrameInfo>(), 40);
        assert_eq!(core::mem::size_of::<AvPlayerStreamInfo>(), 40);
        assert_eq!(core::mem::size_of::<AvPlayerMemAllocator>(), 40);
        assert_eq!(core::mem::size_of::<AvPlayerFileReplacement>(), 40);
        assert_eq!(core::mem::size_of::<AvPlayerEventReplacement>(), 16);
        assert_eq!(core::mem::size_of::<AvPlayerInitData>(), 120);
        assert_eq!(core::mem::size_of::<AvPlayerDecoderInit>(), 32);
        assert_eq!(core::mem::size_of::<AvPlayerPostInitData>(), 132);
    }

    #[test]
    fn the_enums_match_the_header_values() {
        assert_eq!(AvPlayerSourceType::FileMp4 as u32, 1);
        assert_eq!(AvPlayerSourceType::Hls as u32, 8);
        assert_eq!(AvPlayerEvents::DrmError as u32, 0x40);
        assert_eq!(AvPlayerStreamType::TimedText as u32, 2);
        assert_eq!(AvPlayerAvSyncMode::None as u32, 1);
    }
}
