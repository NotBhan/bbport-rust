//! The AvPlayer data streamer interface, ported from
//! `legacy/gpu/shadps4/core/libraries/avplayer/avplayer_data_streamer.h`.

use std::ffi::c_void;

/// The FFmpeg `AVIOContext` the player reads through.
pub type AvioContext = c_void;

/// `Libraries::AvPlayer::IDataStreamer`: a byte source the player pulls from.
pub trait DataStreamer: Send {
    /// `Init`: opens `path`; false leaves the streamer unused.
    fn init(&mut self, path: &str) -> bool;
    /// `Reset`: rewinds the source.
    fn reset(&mut self);
    /// `GetContext`: the read callback context handed to FFmpeg.
    fn context(&mut self) -> *mut AvioContext;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Bytes {
        path: String,
        position: usize,
        context: u8,
    }

    impl DataStreamer for Bytes {
        fn init(&mut self, path: &str) -> bool {
            if path.is_empty() {
                return false;
            }
            self.path = path.to_string();
            true
        }

        fn reset(&mut self) {
            self.position = 0;
        }

        fn context(&mut self) -> *mut AvioContext {
            (&mut self.context as *mut u8).cast()
        }
    }

    #[test]
    fn a_streamer_opens_rewinds_and_hands_out_its_context() {
        let mut streamer = Bytes {
            path: String::new(),
            position: 12,
            context: 7,
        };
        assert!(!streamer.init(""));
        assert!(streamer.init("/app0/movie.pam"));
        assert_eq!(streamer.path, "/app0/movie.pam");
        streamer.reset();
        assert_eq!(streamer.position, 0);
        let context = streamer.context();
        assert!(!context.is_null());
        assert_eq!(unsafe { *(context as *const u8) }, 7);
    }
}
