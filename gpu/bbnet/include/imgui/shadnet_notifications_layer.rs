//! shadNet notifications, ported from `bbnet/include/imgui/shadnet_notifications_layer.h`.

use std::sync::{Mutex, OnceLock};

/// `Kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    FriendRequest,
    FriendNew,
    FriendLost,
    Online,
    Info,
    Warning,
}

/// `HistoryEntry`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub kind: Kind,
    pub text: String,
    pub time: String,
}

/// `KindDisplay`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KindDisplay {
    pub tag: &'static str,
    pub color: [f32; 4],
}

/// `DisplayOf`; the colors are the ones from `bbnet_glue.cpp`.
pub fn display_of(kind: Kind) -> KindDisplay {
    match kind {
        Kind::FriendRequest => KindDisplay {
            tag: "Friend request",
            color: [0.9, 0.8, 0.3, 1.0],
        },
        Kind::FriendNew => KindDisplay {
            tag: "Friend",
            color: [0.4, 0.9, 0.4, 1.0],
        },
        Kind::FriendLost => KindDisplay {
            tag: "Friend",
            color: [0.9, 0.4, 0.4, 1.0],
        },
        Kind::Online => KindDisplay {
            tag: "Online",
            color: [0.4, 0.8, 0.9, 1.0],
        },
        Kind::Warning => KindDisplay {
            tag: "Warning",
            color: [0.95, 0.6, 0.2, 1.0],
        },
        Kind::Info => KindDisplay {
            tag: "Info",
            color: [0.8, 0.8, 0.8, 1.0],
        },
    }
}

struct State {
    history: Vec<HistoryEntry>,
    unread: usize,
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(State {
            history: Vec::new(),
            unread: 0,
        })
    })
}

fn local_time() -> String {
    // SAFETY: `localtime_r` fills a caller-owned `tm` from a `time_t`.
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut broken_down: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&now, &mut broken_down).is_null() {
            return String::new();
        }
        format!(
            "{:02}:{:02}:{:02}",
            broken_down.tm_hour, broken_down.tm_min, broken_down.tm_sec
        )
    }
}

/// `Push`.
pub fn push(kind: Kind, text: String) {
    let time = local_time();
    let mut state = state().lock().unwrap_or_else(|error| error.into_inner());
    state.history.push(HistoryEntry { kind, text, time });
    state.unread += 1;
}

/// `GetHistory`.
pub fn history() -> Vec<HistoryEntry> {
    state()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .history
        .clone()
}

/// `ClearHistory`.
pub fn clear_history() {
    let mut state = state().lock().unwrap_or_else(|error| error.into_inner());
    state.history.clear();
    state.unread = 0;
}

/// `UnreadCount`.
pub fn unread_count() -> usize {
    state()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .unread
}

/// `MarkRead`.
pub fn mark_read() {
    state()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .unread = 0;
}

/// `Register`.
pub fn register() {}

/// `Unregister`.
pub fn unregister() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_display_styles_match_the_glue() {
        assert_eq!(display_of(Kind::FriendRequest).tag, "Friend request");
        assert_eq!(display_of(Kind::Warning).color, [0.95, 0.6, 0.2, 1.0]);
        assert_eq!(display_of(Kind::Info).tag, "Info");
    }

    #[test]
    fn history_and_unread_counts_track_pushes() {
        clear_history();
        push(Kind::Online, "a".into());
        push(Kind::Warning, "b".into());
        assert_eq!(unread_count(), 2);
        let entries = history();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].kind, Kind::Online);
        assert_eq!(entries[1].text, "b");
        assert_eq!(entries[0].time.len(), 8, "HH:MM:SS");
        mark_read();
        assert_eq!(unread_count(), 0);
        clear_history();
        assert!(history().is_empty());
        assert_eq!(unread_count(), 0);
    }
}
