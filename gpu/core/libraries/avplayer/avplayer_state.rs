//! The AvPlayer state machine surface, ported from
//! `shadps4/core/libraries/avplayer/avplayer_state.h`.

use crate::gpu::core::libraries::avplayer::avplayer::*;

/// `AvPlayerStateCallback`.
pub trait AvPlayerStateCallback {
    fn get_sync_mode(&mut self) -> AvPlayerAvSyncMode;
    fn on_warning(&mut self, id: u32);
    fn on_error(&mut self);
    fn on_eof(&mut self);
}

/// The per-player state settings.
#[derive(Debug, Clone, Copy)]
pub struct AvPlayerStateConfig {
    pub auto_start: bool,
    pub default_language: [u8; 4],
    pub sync_mode: AvPlayerAvSyncMode,
    pub thread_priority: u32,
    pub thread_affinity: u32,
}

impl Default for AvPlayerStateConfig {
    fn default() -> Self {
        Self {
            auto_start: false,
            default_language: [b'e', b'n', b'g', 0],
            sync_mode: AvPlayerAvSyncMode::Default,
            thread_priority: 0,
            thread_affinity: 0,
        }
    }
}

/// The `AvPlayerState` methods; the HLE layer drives them.
pub trait AvPlayerStateApi {
    fn post_init(&mut self, post_init_data: &AvPlayerPostInitData);
    fn add_source(&mut self, filename: &str, source_type: AvPlayerSourceType) -> bool;
    fn stream_count(&mut self) -> i32;
    fn stream_info(&mut self, stream_index: u32, info: &mut AvPlayerStreamInfo) -> bool;
    fn enable_stream(&mut self, stream_index: u32) -> bool;
    fn start(&mut self) -> bool;
    fn stop(&mut self) -> bool;
    fn pause(&mut self) -> bool;
    fn resume(&mut self) -> bool;
    fn set_av_sync_mode(&mut self, sync_mode: AvPlayerAvSyncMode);
    fn audio_data(&mut self, info: &mut AvPlayerFrameInfo) -> bool;
    fn video_data(&mut self, info: &mut AvPlayerFrameInfo) -> bool;
    fn video_data_ex(&mut self, info: &mut AvPlayerFrameInfoEx) -> bool;
    fn is_active(&mut self) -> bool;
    fn current_time(&mut self) -> u64;
    fn set_looping(&mut self, looping: bool) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Callback;

    impl AvPlayerStateCallback for Callback {
        fn get_sync_mode(&mut self) -> AvPlayerAvSyncMode {
            AvPlayerAvSyncMode::None
        }

        fn on_warning(&mut self, _id: u32) {}

        fn on_error(&mut self) {}

        fn on_eof(&mut self) {}
    }

    #[test]
    fn the_default_state_config_matches_the_header_defaults() {
        let config = AvPlayerStateConfig::default();
        assert!(!config.auto_start);
        assert_eq!(&config.default_language[..3], b"eng");
        assert_eq!(config.sync_mode, AvPlayerAvSyncMode::Default);
        let mut callback = Callback;
        assert_eq!(callback.get_sync_mode(), AvPlayerAvSyncMode::None);
        callback.on_warning(1);
        callback.on_eof();
    }
}

/// The `avplayer_state.cpp` implementation state.
#[derive(Debug, Default)]
pub struct AvPlayerStateImpl {
    pub config: Option<AvPlayerStateConfig>,
    pub sources: Vec<(String, AvPlayerSourceType)>,
    pub active: bool,
    pub paused: bool,
    pub looping: bool,
    pub current_time: u64,
    pub sync_mode: AvPlayerAvSyncMode,
}

impl AvPlayerStateImpl {
    pub fn post_init(&mut self, data: &AvPlayerPostInitData) {
        let _ = data;
        self.config = Some(AvPlayerStateConfig::default());
    }

    pub fn add_source(&mut self, filename: &str, source_type: AvPlayerSourceType) -> bool {
        if filename.is_empty() {
            return false;
        }
        self.sources.push((filename.to_string(), source_type));
        true
    }

    pub fn stream_count(&self) -> i32 {
        self.sources.len() as i32
    }

    pub fn start(&mut self) -> bool {
        if self.sources.is_empty() {
            return false;
        }
        self.active = true;
        true
    }

    pub fn stop(&mut self) -> bool {
        self.active = false;
        self.paused = false;
        true
    }

    pub fn pause(&mut self) -> bool {
        if !self.active {
            return false;
        }
        self.paused = true;
        true
    }

    pub fn resume(&mut self) -> bool {
        if !self.active {
            return false;
        }
        self.paused = false;
        true
    }

    pub fn set_looping(&mut self, looping: bool) -> bool {
        self.looping = looping;
        true
    }

    pub fn set_av_sync_mode(&mut self, mode: AvPlayerAvSyncMode) {
        self.sync_mode = mode;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn current_time(&self) -> u64 {
        self.current_time
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn avplayer_state_starts_and_pauses_sources() {
        let mut state = AvPlayerStateImpl::default();
        assert!(!state.start());
        assert!(state.add_source("movie.mp4", AvPlayerSourceType::Unknown));
        assert_eq!(state.stream_count(), 1);
        assert!(state.start());
        assert!(state.pause());
        assert!(state.resume());
        assert!(state.set_looping(true));
        assert!(state.stop());
        assert!(!state.is_active());
    }
}
