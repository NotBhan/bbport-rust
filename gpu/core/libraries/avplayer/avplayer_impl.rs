//! The AvPlayer facade, ported from
//! `shadps4/core/libraries/avplayer/avplayer_impl.h`.

use crate::gpu::core::libraries::avplayer::avplayer::*;

/// The `AvPlayer` methods exposed to the guest library.
pub trait AvPlayerApi {
    fn post_init(&mut self, data: &AvPlayerPostInitData) -> i32;
    fn add_source(&mut self, filename: &str) -> i32;
    fn add_source_ex(&mut self, path: &str, source_type: AvPlayerSourceType) -> i32;
    fn stream_count(&mut self) -> i32;
    fn stream_info(&mut self, stream_index: u32, info: &mut AvPlayerStreamInfo) -> i32;
    fn enable_stream(&mut self, stream_index: u32) -> i32;
    fn start(&mut self) -> i32;
    fn pause(&mut self) -> i32;
    fn resume(&mut self) -> i32;
    fn set_av_sync_mode(&mut self, sync_mode: AvPlayerAvSyncMode) -> i32;
    fn audio_data(&mut self, info: &mut AvPlayerFrameInfo) -> bool;
    fn video_data(&mut self, info: &mut AvPlayerFrameInfo) -> bool;
    fn video_data_ex(&mut self, info: &mut AvPlayerFrameInfoEx) -> bool;
    fn is_active(&mut self) -> bool;
    fn current_time(&mut self) -> u64;
    fn stop(&mut self) -> i32;
    fn set_looping(&mut self, looping: bool) -> bool;
}

/// The allocator/file callbacks `StubInitData` installs.
pub trait AvPlayerCallbacks {
    fn allocate(&mut self, alignment: u32, size: u32) -> *mut u8;
    fn deallocate(&mut self, pointer: *mut u8);
    fn allocate_texture(&mut self, alignment: u32, size: u32) -> *mut u8;
    fn deallocate_texture(&mut self, pointer: *mut u8);
    fn open_file(&mut self, filename: &str) -> i32;
    fn close_file(&mut self) -> i32;
    fn read_offset_file(&mut self, buffer: &mut [u8], position: u64, length: u32) -> i32;
    fn size_file(&mut self) -> u64;
}

/// `AvPlayer::StubInitData`; the file replacement is dropped when incomplete.
pub fn stub_init_data(
    data: &mut AvPlayerInitData,
    callbacks: &mut dyn AvPlayerCallbacks,
    file_callbacks_complete: bool,
) {
    let _ = callbacks;
    if !file_callbacks_complete {
        data.file_replacement = Default::default();
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn stub_init_data_requires_the_full_file_replacement() {
        struct Callbacks;
        impl AvPlayerCallbacks for Callbacks {
            fn allocate(&mut self, _: u32, _: u32) -> *mut u8 {
                std::ptr::null_mut()
            }
            fn deallocate(&mut self, _: *mut u8) {}
            fn allocate_texture(&mut self, _: u32, _: u32) -> *mut u8 {
                std::ptr::null_mut()
            }
            fn deallocate_texture(&mut self, _: *mut u8) {}
            fn open_file(&mut self, _: &str) -> i32 {
                0
            }
            fn close_file(&mut self) -> i32 {
                0
            }
            fn read_offset_file(&mut self, _: &mut [u8], _: u64, _: u32) -> i32 {
                0
            }
            fn size_file(&mut self) -> u64 {
                0
            }
        }
        let mut data = AvPlayerInitData::default();
        stub_init_data(&mut data, &mut Callbacks, false);
        assert!(data.file_replacement.open.is_none());
        stub_init_data(&mut data, &mut Callbacks, true);
    }
}
