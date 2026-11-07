//! AvPlayer source types, ported from
//! `shadps4/core/libraries/avplayer/avplayer_source.h`.

use std::sync::{Condvar, Mutex};
use std::time::Duration;

use crate::gpu::core::libraries::avplayer::avplayer::{
    AvPlayerFrameInfoEx, AvPlayerInitData, AvPlayerStreamInfo,
};

/// `GuestBuffer`; the allocator callbacks are replaced by an owned host buffer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuestBuffer {
    pub data: Vec<u8>,
    pub is_texture: bool,
}

impl GuestBuffer {
    pub fn new(size: u32, is_texture: bool) -> Self {
        Self {
            data: vec![0; size as usize],
            is_texture,
        }
    }

    pub fn buffer(&self) -> &[u8] {
        &self.data
    }

    pub fn size(&self) -> u64 {
        self.data.len() as u64
    }
}

/// `Frame`.
#[derive(Clone)]
pub struct Frame {
    pub buffer: GuestBuffer,
    pub info: AvPlayerFrameInfoEx,
}

/// `EventCV`; waits and notifies like the C++ condition variable.
#[derive(Debug, Default)]
pub struct EventCv {
    mutex: Mutex<()>,
    cv: Condvar,
}

impl EventCv {
    pub fn wait_until(&self, mut predicate: impl FnMut() -> bool) {
        let mut guard = self.mutex.lock().unwrap_or_else(|error| error.into_inner());
        while !predicate() {
            guard = self
                .cv
                .wait(guard)
                .unwrap_or_else(|error| error.into_inner());
        }
    }

    pub fn wait_for(&self, timeout: Duration, predicate: impl Fn() -> bool) -> bool {
        let guard = self.mutex.lock().unwrap_or_else(|error| error.into_inner());
        let (guard, result) = self
            .cv
            .wait_timeout_while(guard, timeout, |_| !predicate())
            .unwrap_or_else(|error| error.into_inner());
        drop(guard);
        result.timed_out() && predicate()
    }

    pub fn notify(&self) {
        self.cv.notify_all();
    }
}

/// `AvPlayerSource` state and surface.
#[derive(Default)]
pub struct AvPlayerSource {
    pub init_data: Option<AvPlayerInitData>,
    pub path: String,
    pub streams: Vec<AvPlayerStreamInfo>,
    pub duration: u64,
    pub looping: bool,
    pub paused: bool,
    pub eof: bool,
    pub active: bool,
    pub current_time: u64,
    pub audio_frames: Vec<Frame>,
    pub video_frames: Vec<Frame>,
    pub audio_cv: EventCv,
    pub video_cv: EventCv,
}

impl AvPlayerSource {
    pub fn new() -> Self {
        Self::default()
    }

    /// `Init`.
    pub fn init(&mut self, init_data: AvPlayerInitData, path: &str) -> bool {
        if path.is_empty() {
            return false;
        }
        self.init_data = Some(init_data);
        self.path = path.to_string();
        true
    }

    pub fn stream_count(&self) -> i32 {
        self.streams.len() as i32
    }

    pub fn stream_info(&self, index: u32, info: &mut AvPlayerStreamInfo) -> bool {
        match self.streams.get(index as usize) {
            Some(stream) => {
                *info = *stream;
                true
            }
            None => false,
        }
    }

    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    pub fn start(&mut self) -> bool {
        self.active = true;
        true
    }

    pub fn stop(&mut self) -> bool {
        self.active = false;
        true
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn has_frames(&self, num_frames: u32) -> Option<bool> {
        Some(self.video_frames.len() >= num_frames as usize)
    }

    pub fn get_audio_data(&mut self) -> Option<Frame> {
        self.audio_frames.pop()
    }

    pub fn get_video_data(&mut self) -> Option<Frame> {
        self.video_frames.pop()
    }

    pub fn current_time(&self) -> u64 {
        self.current_time
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn duration_millis(&self) -> u64 {
        self.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn guest_buffers_and_source_state_follow_the_header() {
        let buffer = GuestBuffer::new(16, true);
        assert_eq!(buffer.size(), 16);
        assert!(buffer.is_texture);
        assert!(buffer.buffer().iter().all(|byte| *byte == 0));
        let mut source = AvPlayerSource::new();
        assert!(source.init(AvPlayerInitData::default(), "movie.mp4"));
        assert!(source.start());
        assert!(source.is_active());
        source.pause();
        source.resume();
        source.set_looping(true);
        assert!(source.looping);
        assert_eq!(source.stream_count(), 0);
        assert!(!source.stream_info(0, &mut AvPlayerStreamInfo::default()));
        assert_eq!(source.has_frames(0), Some(true));
        assert!(source.stop());
        assert!(!source.is_active());
        let cv = Arc::new(EventCv::default());
        let waiter = {
            let cv = cv.clone();
            std::thread::spawn(move || cv.wait_until(|| true))
        };
        cv.notify();
        waiter.join().unwrap();
    }
}

/// The `avplayer_source.cpp` demux/decode state.
#[derive(Default)]
pub struct AvPlayerSourceRuntime {
    pub streams: Vec<AvPlayerStreamInfo>,
    pub duration_ms: u64,
    pub audio_queue: Vec<Frame>,
    pub video_queue: Vec<Frame>,
    pub eof: bool,
    pub paused: bool,
    pub active: bool,
}

impl AvPlayerSourceRuntime {
    pub fn find_streams(&mut self, count: usize) -> bool {
        self.streams = (0..count).map(|_| AvPlayerStreamInfo::default()).collect();
        !self.streams.is_empty()
    }

    pub fn stream_count(&self) -> i32 {
        self.streams.len() as i32
    }

    pub fn push_video(&mut self, frame: Frame) {
        self.video_queue.push(frame);
    }

    pub fn get_video(&mut self) -> Option<Frame> {
        if self.video_queue.is_empty() {
            self.eof = true;
            return None;
        }
        self.video_queue.pop()
    }

    pub fn has_frames(&self, frames: u32) -> Option<bool> {
        Some(self.video_queue.len() >= frames as usize)
    }

    pub fn duration_millis(&self) -> u64 {
        self.duration_ms
    }

    pub fn is_active(&self) -> bool {
        self.active && !self.eof
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn source_runtime_manages_streams_and_frames() {
        let mut runtime = AvPlayerSourceRuntime::default();
        assert!(runtime.find_streams(2));
        assert_eq!(runtime.stream_count(), 2);
        runtime.push_video(Frame {
            buffer: GuestBuffer::new(1, false),
            info: unsafe { std::mem::zeroed() },
        });
        assert_eq!(runtime.has_frames(1), Some(true));
        assert!(runtime.get_video().is_some());
        assert!(runtime.get_video().is_none());
        assert!(runtime.eof);
    }
}
