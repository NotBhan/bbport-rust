//! `sceAvPlayer*` entry points, ported from
//! `shadps4/core/libraries/avplayer/avplayer.cpp`.

use crate::gpu::core::libraries::avplayer::avplayer::AvPlayerSourceDetails;
use crate::gpu::core::libraries::avplayer::avplayer_error::ORBIS_AVPLAYER_ERROR_INVALID_PARAMS;
use crate::gpu::core::libraries::avplayer::avplayer_impl::AvPlayerApi;
use crate::gpu::core::libraries::error_codes::ORBIS_OK;

/// `sceAvPlayerAddSource`.
pub fn add_source(handle: Option<&mut dyn AvPlayerApi>, filename: &str) -> i32 {
    match handle {
        Some(player) => player.add_source(filename),
        None => ORBIS_AVPLAYER_ERROR_INVALID_PARAMS,
    }
}

/// `sceAvPlayerAddSourceEx`.
///
/// # Safety
/// `details.uri.name` must point to `details.uri.length` readable bytes.
pub unsafe fn add_source_ex(
    handle: Option<&mut dyn AvPlayerApi>,
    source_details: Option<&AvPlayerSourceDetails>,
) -> i32 {
    const MAX_URI_LENGTH: u32 = 64 * 1024;
    let (Some(player), Some(details)) = (handle, source_details) else {
        return ORBIS_AVPLAYER_ERROR_INVALID_PARAMS;
    };
    if details.uri.name.is_null() || details.uri.length == 0 {
        return ORBIS_AVPLAYER_ERROR_INVALID_PARAMS;
    }
    if details.uri.length > MAX_URI_LENGTH {
        return ORBIS_AVPLAYER_ERROR_INVALID_PARAMS;
    }
    let name = unsafe {
        std::slice::from_raw_parts(details.uri.name as *const u8, details.uri.length as usize)
    };
    player.add_source_ex(
        std::str::from_utf8(name).unwrap_or_default(),
        details.source_type,
    )
}

/// `sceAvPlayerChangeStream`.
pub fn change_stream() -> i32 {
    ORBIS_OK
}

/// `sceAvPlayerClose`.
pub fn close(handle: Option<&mut dyn AvPlayerApi>) -> i32 {
    match handle {
        Some(player) => {
            player.stop();
            ORBIS_OK
        }
        None => ORBIS_AVPLAYER_ERROR_INVALID_PARAMS,
    }
}

/// `sceAvPlayerCurrentTime`.
pub fn current_time(handle: Option<&mut dyn AvPlayerApi>) -> u64 {
    match handle {
        Some(player) => player.current_time(),
        None => ORBIS_AVPLAYER_ERROR_INVALID_PARAMS as u32 as u64,
    }
}

/// `sceAvPlayerEnableStream`.
pub fn enable_stream(handle: Option<&mut dyn AvPlayerApi>, stream_id: u32) -> i32 {
    match handle {
        Some(player) => player.enable_stream(stream_id),
        None => ORBIS_AVPLAYER_ERROR_INVALID_PARAMS,
    }
}

/// `sceAvPlayerGetStreamInfo`.
pub fn get_stream_info(
    handle: Option<&mut dyn AvPlayerApi>,
    stream_id: u32,
    info: Option<&mut crate::gpu::core::libraries::avplayer::avplayer::AvPlayerStreamInfo>,
) -> i32 {
    let (Some(player), Some(info)) = (handle, info) else {
        return ORBIS_AVPLAYER_ERROR_INVALID_PARAMS;
    };
    player.stream_info(stream_id, info)
}

/// `sceAvPlayerInit`; A null `AvPlayerInitData` fails.
pub fn init(data_present: bool) -> Result<(), i32> {
    if data_present {
        Ok(())
    } else {
        Err(ORBIS_AVPLAYER_ERROR_INVALID_PARAMS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::core::libraries::avplayer::avplayer::{
        AvPlayerAvSyncMode, AvPlayerFrameInfo, AvPlayerFrameInfoEx, AvPlayerPostInitData,
        AvPlayerSourceDetails, AvPlayerSourceType, AvPlayerStreamInfo, AvPlayerUri,
    };

    struct Player;

    impl AvPlayerApi for Player {
        fn post_init(&mut self, _: &AvPlayerPostInitData) -> i32 {
            ORBIS_OK
        }
        fn add_source(&mut self, _: &str) -> i32 {
            ORBIS_OK
        }
        fn add_source_ex(&mut self, _: &str, _: AvPlayerSourceType) -> i32 {
            ORBIS_OK
        }
        fn stream_count(&mut self) -> i32 {
            0
        }
        fn stream_info(&mut self, _: u32, _: &mut AvPlayerStreamInfo) -> i32 {
            0
        }
        fn enable_stream(&mut self, _: u32) -> i32 {
            ORBIS_OK
        }
        fn start(&mut self) -> i32 {
            ORBIS_OK
        }
        fn pause(&mut self) -> i32 {
            ORBIS_OK
        }
        fn resume(&mut self) -> i32 {
            ORBIS_OK
        }
        fn set_av_sync_mode(&mut self, _: AvPlayerAvSyncMode) -> i32 {
            ORBIS_OK
        }
        fn audio_data(&mut self, _: &mut AvPlayerFrameInfo) -> bool {
            false
        }
        fn video_data(&mut self, _: &mut AvPlayerFrameInfo) -> bool {
            false
        }
        fn video_data_ex(&mut self, _: &mut AvPlayerFrameInfoEx) -> bool {
            false
        }
        fn is_active(&mut self) -> bool {
            false
        }
        fn current_time(&mut self) -> u64 {
            0
        }
        fn stop(&mut self) -> i32 {
            ORBIS_OK
        }
        fn set_looping(&mut self, _: bool) -> bool {
            true
        }
    }

    #[test]
    fn source_ex_rejects_invalid_guest_strings_before_reading_them() {
        let mut details = AvPlayerSourceDetails {
            uri: AvPlayerUri {
                name: std::ptr::null(),
                length: 0,
            },
            reserved1: [0; 64],
            source_type: AvPlayerSourceType::FileMp4,
            reserved2: [0; 44],
        };
        unsafe {
            assert_eq!(
                add_source_ex(Some(&mut Player), Some(&details)),
                ORBIS_AVPLAYER_ERROR_INVALID_PARAMS
            );
            details.uri.length = 64 * 1024 + 1;
            assert_eq!(
                add_source_ex(Some(&mut Player), Some(&details)),
                ORBIS_AVPLAYER_ERROR_INVALID_PARAMS
            );
        }
    }

    #[test]
    fn null_handles_are_rejected() {
        assert_eq!(add_source(None, "a"), ORBIS_AVPLAYER_ERROR_INVALID_PARAMS);
        assert_eq!(close(None), ORBIS_AVPLAYER_ERROR_INVALID_PARAMS);
        assert_eq!(enable_stream(None, 0), ORBIS_AVPLAYER_ERROR_INVALID_PARAMS);
        assert_eq!(init(false), Err(ORBIS_AVPLAYER_ERROR_INVALID_PARAMS));
        assert_eq!(init(true), Ok(()));
        assert_eq!(add_source(Some(&mut Player), "a"), ORBIS_OK);
        assert_eq!(change_stream(), ORBIS_OK);
    }
}
