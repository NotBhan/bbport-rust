//! AvPlayer shared types, ported from
//! `shadps4/core/libraries/avplayer/avplayer_common.h` and `avplayer_common.cpp`.

use std::collections::VecDeque;
use std::sync::Mutex;

use crate::gpu::core::libraries::avplayer::avplayer::AvPlayerSourceType;

/// `AvState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvState {
    Unknown,
    Initial,
    AddingSource,
    Ready,
    Play,
    Stop,
    EndOfFile,
    Pause,
    C0x08,
    Jump,
    TrickMode,
    C0x0B,
    Buffering,
    Starting,
    Error,
}

/// `AvEventType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AvEventType {
    ChangeFlowState = 21,
    WarningId = 22,
    RevertState = 30,
    AddSource = 40,
    Error = 255,
}

/// `AvPlayerEventData`.
#[repr(C)]
#[derive(Clone, Copy)]
pub union AvPlayerEventData {
    pub num_frames: u32,
    pub state: AvState,
    pub error: i32,
    pub attempt: u32,
}

/// `AvPlayerEvent`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AvPlayerEvent {
    pub event: AvEventType,
    pub payload: AvPlayerEventData,
}

/// `AvPlayerQueue`.
#[derive(Debug)]
pub struct AvPlayerQueue<T> {
    queue: Mutex<VecDeque<T>>,
}

impl<T> Default for AvPlayerQueue<T> {
    fn default() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
        }
    }
}

impl<T> AvPlayerQueue<T> {
    /// `Size`.
    pub fn size(&self) -> usize {
        self.queue
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .len()
    }

    /// `Push`.
    pub fn push(&self, value: T) {
        self.queue
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push_back(value);
    }

    /// `Front`.
    pub fn front(&self) -> Option<T>
    where
        T: Clone,
    {
        self.queue
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .front()
            .cloned()
    }

    /// `Pop`.
    pub fn pop(&self) -> Option<T> {
        self.queue
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .pop_front()
    }

    /// `Clear`.
    pub fn clear(&self) {
        self.queue
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clear();
    }
}

fn iequals(left: &str, right: &str) -> bool {
    left.len() == right.len()
        && left
            .bytes()
            .zip(right.bytes())
            .all(|(a, b)| a.eq_ignore_ascii_case(&b))
}

/// `GetSourceType`.
pub fn get_source_type(path: &str) -> AvPlayerSourceType {
    if path.is_empty() {
        return AvPlayerSourceType::Unknown;
    }
    let mut name = path;
    if name.contains("://") {
        name = match name.find(['?', '#']) {
            Some(index) => &name[..index],
            None => name,
        };
        if name.is_empty() {
            return AvPlayerSourceType::Unknown;
        }
    }
    let Some(dot) = name.rfind('.') else {
        return AvPlayerSourceType::Unknown;
    };
    let extension = &name[dot..];
    let extension = extension.split('/').next().unwrap_or(extension);
    if [".mp4", ".m4v", ".m3d", ".m4a", ".mov"]
        .iter()
        .any(|candidate| iequals(extension, candidate))
    {
        return AvPlayerSourceType::FileMp4;
    }
    if iequals(extension, ".m3u8") {
        return AvPlayerSourceType::Hls;
    }
    AvPlayerSourceType::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_queue_pushes_pops_and_clears() {
        let queue = AvPlayerQueue::default();
        assert_eq!(queue.size(), 0);
        assert_eq!(queue.pop(), None);
        queue.push(1u32);
        queue.push(2u32);
        assert_eq!(queue.size(), 2);
        assert_eq!(queue.front(), Some(1));
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), Some(2));
        queue.push(3u32);
        queue.clear();
        assert_eq!(queue.size(), 0);
    }

    #[test]
    fn source_types_come_from_the_extension() {
        assert_eq!(get_source_type(""), AvPlayerSourceType::Unknown);
        assert_eq!(get_source_type("movie.mp4"), AvPlayerSourceType::FileMp4);
        assert_eq!(get_source_type("movie.M4V"), AvPlayerSourceType::FileMp4);
        assert_eq!(get_source_type("movie.m3d"), AvPlayerSourceType::FileMp4);
        assert_eq!(get_source_type("movie.m4a"), AvPlayerSourceType::FileMp4);
        assert_eq!(get_source_type("movie.mov"), AvPlayerSourceType::FileMp4);
        assert_eq!(get_source_type("stream.m3u8"), AvPlayerSourceType::Hls);
        assert_eq!(get_source_type("movie.mkv"), AvPlayerSourceType::Unknown);
        assert_eq!(
            get_source_type("https://example.com/path/movie.mp4?token=1#frag"),
            AvPlayerSourceType::FileMp4
        );
        assert_eq!(
            get_source_type("https://example.com/path/m3u8?token=1"),
            AvPlayerSourceType::Unknown
        );
        assert_eq!(
            get_source_type("https://example.com/path/file.m3u8/and/beyond"),
            AvPlayerSourceType::Hls
        );
    }

    #[test]
    fn the_event_types_match_the_header() {
        assert_eq!(AvEventType::ChangeFlowState as u32, 21);
        assert_eq!(AvEventType::Error as u32, 255);
        let event = AvPlayerEvent {
            event: AvEventType::AddSource,
            payload: AvPlayerEventData { attempt: 3 },
        };
        assert_eq!(unsafe { event.payload.attempt }, 3);
    }
}
