//! shadNet notification history/toasts, ported from
//! `legacy/gpu/shim/imgui/shadnet_notifications_layer.h`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    FriendRequest,
    FriendNew,
    FriendLost,
    Online,
    Info,
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub kind: Kind,
    pub text: String,
    pub time: String,
}

#[derive(Debug, Clone, Copy)]
pub struct KindDisplay {
    pub tag: &'static str,
    pub color: [f32; 4],
}

pub fn display_of(kind: Kind) -> KindDisplay {
    let (tag, color) = match kind {
        Kind::FriendRequest => ("REQUEST", [0.36, 0.65, 0.98, 1.0]),
        Kind::FriendNew => ("FRIEND", [0.45, 0.86, 0.55, 1.0]),
        Kind::FriendLost => ("OFFLINE", [0.85, 0.45, 0.45, 1.0]),
        Kind::Online => ("ONLINE", [0.98, 0.72, 0.25, 1.0]),
        Kind::Info => ("INFO", [0.80, 0.80, 0.80, 1.0]),
    };
    KindDisplay { tag, color }
}

fn history() -> &'static Mutex<Vec<HistoryEntry>> {
    static HISTORY: OnceLock<Mutex<Vec<HistoryEntry>>> = OnceLock::new();
    HISTORY.get_or_init(|| Mutex::new(Vec::new()))
}

fn unread() -> &'static AtomicUsize {
    static UNREAD: AtomicUsize = AtomicUsize::new(0);
    &UNREAD
}

pub fn push(kind: Kind, text: String) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let entry = HistoryEntry {
        kind,
        text,
        time: format!("{now}"),
    };
    if let Ok(mut entries) = history().lock() {
        entries.push(entry);
    }
    unread().fetch_add(1, Ordering::Relaxed);
}

pub fn get_history() -> Vec<HistoryEntry> {
    history()
        .lock()
        .map(|entries| entries.clone())
        .unwrap_or_default()
}

pub fn clear_history() {
    if let Ok(mut entries) = history().lock() {
        entries.clear();
    }
}

pub fn unread_count() -> usize {
    unread().load(Ordering::Relaxed)
}

pub fn mark_read() {
    unread().store(0, Ordering::Relaxed);
}

pub fn register() {}
pub fn unregister() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pushes_are_retained_and_counted_until_marked_read() {
        clear_history();
        mark_read();
        push(Kind::Info, "hello".into());
        push(Kind::Online, "world".into());
        assert_eq!(get_history().len(), 2);
        assert_eq!(unread_count(), 2);
        mark_read();
        assert_eq!(unread_count(), 0);
        assert_eq!(display_of(Kind::FriendNew).tag, "FRIEND");
        clear_history();
    }
}
