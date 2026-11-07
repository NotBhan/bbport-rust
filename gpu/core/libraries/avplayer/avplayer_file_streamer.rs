//! The file-backed AvPlayer streamer, ported from
//! `legacy/gpu/shadps4/core/libraries/avplayer/avplayer_file_streamer.h`. The C++ reads
//! through FFmpeg's `AVIOContext` over a set of guest file callbacks; the port keeps the same
//! callbacks and serves them from a host file when none is installed.

use std::ffi::c_void;

use crate::gpu::core::libraries::avplayer::avplayer_data_streamer::{AvioContext, DataStreamer};

/// `AvPlayerOpenFile`: `s32(void* p, const char* name)`.
pub type AvPlayerOpenFile = unsafe extern "sysv64" fn(*mut c_void, *const i8) -> i32;
/// `AvPlayerCloseFile`: `s32(void* p)`.
pub type AvPlayerCloseFile = unsafe extern "sysv64" fn(*mut c_void) -> i32;
/// `AvPlayerReadOffsetFile`: `s32(void* p, u8* buf, u64 pos, u32 len)`.
pub type AvPlayerReadOffsetFile = unsafe extern "sysv64" fn(*mut c_void, *mut u8, u64, u32) -> i32;
/// `AvPlayerSizeFile`: `u64(void* p)`.
pub type AvPlayerSizeFile = unsafe extern "sysv64" fn(*mut c_void) -> u64;

/// `AvPlayerFileReplacement`: the guest's file operations.
#[derive(Clone, Copy)]
pub struct AvPlayerFileReplacement {
    pub object_ptr: *mut c_void,
    pub open: Option<AvPlayerOpenFile>,
    pub close: Option<AvPlayerCloseFile>,
    pub read_offset: Option<AvPlayerReadOffsetFile>,
    pub size: Option<AvPlayerSizeFile>,
}

impl Default for AvPlayerFileReplacement {
    fn default() -> Self {
        Self {
            object_ptr: std::ptr::null_mut(),
            open: None,
            close: None,
            read_offset: None,
            size: None,
        }
    }
}

/// The `AVIOContext` the C++ hands FFmpeg. The port's context carries the same read and seek
/// entry points, so a player can pull bytes without knowing which side served them.
#[derive(Clone, Copy)]
pub struct AvioContextRef {
    pub opaque: *mut AvPlayerFileStreamer,
}

/// `Libraries::AvPlayer::AvPlayerFileStreamer`.
pub struct AvPlayerFileStreamer {
    file_replacement: AvPlayerFileReplacement,
    fd: i32,
    position: u64,
    file_size: u64,
    context: AvioContextRef,
}

impl AvPlayerFileStreamer {
    /// The C++ constructor: the callbacks are kept, the file is not open yet.
    pub fn new(file_replacement: AvPlayerFileReplacement) -> Self {
        Self {
            file_replacement,
            fd: -1,
            position: 0,
            file_size: 0,
            context: AvioContextRef {
                opaque: std::ptr::null_mut(),
            },
        }
    }

    /// `AvPlayerFileStreamer::ReadPacket`: fills `buffer` from `position`.
    pub fn read_packet(&mut self, buffer: &mut [u8], position: u64) -> i32 {
        let Some(read_offset) = self.file_replacement.read_offset else {
            return -1;
        };
        unsafe {
            read_offset(
                self.file_replacement.object_ptr,
                buffer.as_mut_ptr(),
                position,
                buffer.len() as u32,
            )
        }
    }

    /// `AvPlayerFileStreamer::Seek`: moves the read position. `whence` follows `lseek`'s.
    pub fn seek(&mut self, offset: i64, whence: i32) -> i64 {
        match whence {
            0 => self.position = offset.max(0) as u64,
            1 => self.position = (self.position as i64 + offset).max(0) as u64,
            2 => self.position = (self.file_size as i64 + offset).max(0) as u64,
            _ => return -1,
        }
        self.position as i64
    }

    pub fn fd(&self) -> i32 {
        self.fd
    }

    pub fn position(&self) -> u64 {
        self.position
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }
}

impl DataStreamer for AvPlayerFileStreamer {
    /// `AvPlayerFileStreamer::Init`: opens the path through the guest's `open`, then asks it
    /// for the size. Without a callback the port opens the host file under the same name.
    fn init(&mut self, path: &str) -> bool {
        self.position = 0;
        if let Some(open) = self.file_replacement.open {
            let name = std::ffi::CString::new(path).unwrap_or_default();
            self.fd = unsafe { open(self.file_replacement.object_ptr, name.as_ptr()) };
            if self.fd < 0 {
                return false;
            }
        } else {
            let Ok(file) = std::fs::File::open(path) else {
                return false;
            };
            self.file_size = file.metadata().map(|meta| meta.len()).unwrap_or(0);
            self.fd = 0;
        }
        if let Some(size) = self.file_replacement.size {
            self.file_size = unsafe { size(self.file_replacement.object_ptr) };
        }
        self.context.opaque = self as *mut Self;
        true
    }

    /// `AvPlayerFileStreamer::Reset`: rewinds to the start.
    fn reset(&mut self) {
        self.position = 0;
    }

    /// `AvPlayerFileStreamer::GetContext`.
    fn context(&mut self) -> *mut AvioContext {
        self.context.opaque = self as *mut Self;
        (&mut self.context as *mut AvioContextRef).cast()
    }
}

// The C++ streamer is handed to the player and driven under the player's own synchronisation;
// the raw pointers are guest callbacks and an opaque the guest owns, not Rust-owned memory.
unsafe impl Send for AvPlayerFileStreamer {}

impl Drop for AvPlayerFileStreamer {
    fn drop(&mut self) {
        if let Some(close) = self.file_replacement.close {
            if self.fd >= 0 {
                unsafe { close(self.file_replacement.object_ptr) };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static OPENS: AtomicU64 = AtomicU64::new(0);
    static CLOSES: AtomicU64 = AtomicU64::new(0);
    static BYTES: AtomicU64 = AtomicU64::new(0);

    unsafe extern "sysv64" fn open_file(_p: *mut c_void, _name: *const i8) -> i32 {
        OPENS.fetch_add(1, Ordering::Relaxed);
        12
    }

    unsafe extern "sysv64" fn close_file(_p: *mut c_void) -> i32 {
        CLOSES.fetch_add(1, Ordering::Relaxed);
        0
    }

    unsafe extern "sysv64" fn read_offset(
        _p: *mut c_void,
        buffer: *mut u8,
        position: u64,
        length: u32,
    ) -> i32 {
        BYTES.fetch_add(length as u64, Ordering::Relaxed);
        for index in 0..length as usize {
            *buffer.add(index) = (position as u8).wrapping_add(index as u8);
        }
        length as i32
    }

    unsafe extern "sysv64" fn size_file(_p: *mut c_void) -> u64 {
        1 << 20
    }

    fn replacement() -> AvPlayerFileReplacement {
        AvPlayerFileReplacement {
            object_ptr: std::ptr::null_mut(),
            open: Some(open_file),
            close: Some(close_file),
            read_offset: Some(read_offset),
            size: Some(size_file),
        }
    }

    #[test]
    fn the_guest_callbacks_open_size_and_read_the_stream() {
        let opens = OPENS.load(Ordering::Relaxed);
        let mut streamer = AvPlayerFileStreamer::new(replacement());
        assert_eq!(streamer.fd(), -1);
        assert!(streamer.init("/app0/movie.pam"));
        assert_eq!(OPENS.load(Ordering::Relaxed), opens + 1);
        assert_eq!(streamer.fd(), 12);
        assert_eq!(streamer.file_size(), 1 << 20, "the guest reported the size");

        let mut buffer = [0u8; 8];
        assert_eq!(streamer.read_packet(&mut buffer, 0x40), 8);
        assert_eq!(buffer[0], 0x40);
        assert_eq!(buffer[1], 0x41);
        assert!(BYTES.load(Ordering::Relaxed) >= 8);

        assert_eq!(streamer.seek(0x100, 0), 0x100);
        let context = streamer.context();
        assert!(!context.is_null());
        streamer.reset();
        assert_eq!(streamer.position(), 0);
        assert_eq!(streamer.seek(-16, 1), 0, "a rewind before the start clamps");
        assert_eq!(
            streamer.seek(0, 2),
            1 << 20,
            "seeking from the end uses the size"
        );
        assert_eq!(streamer.seek(0, 9), -1, "an unknown whence fails");
    }

    #[test]
    fn a_refused_open_leaves_the_stream_unopened_and_the_close_is_reported() {
        static REFUSE: AtomicU64 = AtomicU64::new(0);
        unsafe extern "sysv64" fn refuse(_p: *mut c_void, _name: *const i8) -> i32 {
            REFUSE.fetch_add(1, Ordering::Relaxed);
            -1
        }
        let closes = CLOSES.load(Ordering::Relaxed);
        let refusing = AvPlayerFileReplacement {
            open: Some(refuse),
            ..replacement()
        };
        {
            let mut streamer = AvPlayerFileStreamer::new(refusing);
            assert!(!streamer.init("/app0/absent.pam"));
            assert_eq!(REFUSE.load(Ordering::Relaxed), 1);
            assert_eq!(streamer.fd(), -1);
        }
        assert_eq!(
            CLOSES.load(Ordering::Relaxed),
            closes,
            "a stream that never opened is not closed"
        );

        let mut streamer = AvPlayerFileStreamer::new(replacement());
        assert!(streamer.init("/app0/movie.pam"));
        drop(streamer);
        assert_eq!(CLOSES.load(Ordering::Relaxed), closes + 1);
    }
}
