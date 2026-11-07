//! The launcher's activity journal and playtime, ported from `bbport_activity.py`.

use crate::fsutil::atomic_write;
use serde_json::{json, Value};
use std::path::Path;

pub const EVENT_LIMIT: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub time: i64,
    pub kind: String,
    pub text: String,
}

impl Event {
    pub fn to_json(&self) -> Value {
        json!({"time": self.time, "kind": self.kind, "text": self.text})
    }

    fn from_json(value: &Value) -> Option<Self> {
        let time = value.get("time")?;
        let time = if let Some(number) = time.as_i64() {
            number
        } else {
            let number = time.as_f64()?;
            number as i64
        };
        Some(Self {
            time,
            kind: value.get("kind")?.as_str()?.to_string(),
            text: value.get("text")?.as_str()?.to_string(),
        })
    }
}

fn load_payload(path: &Path) -> Value {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Value::Object(Default::default());
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(value @ Value::Object(_)) => value,
        _ => Value::Object(Default::default()),
    }
}

pub fn load_events(path: &Path, limit: usize) -> Vec<Event> {
    let data = load_payload(path);
    let Some(raw) = data.get("events").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut events: Vec<Event> = raw.iter().filter_map(Event::from_json).collect();
    events.sort_by_key(|event| std::cmp::Reverse(event.time));
    events.truncate(limit);
    events
}

pub fn load_playtime(path: &Path) -> i64 {
    match load_payload(path).get("playtime_seconds") {
        Some(Value::Number(number)) => number.as_i64().unwrap_or(0).max(0),
        _ => 0,
    }
}

fn write_payload(path: &Path, events: &[Event], playtime: i64) -> std::io::Result<()> {
    let payload = json!({
        "events": events.iter().map(Event::to_json).collect::<Vec<_>>(),
        "playtime_seconds": playtime.max(0),
    });
    let text = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into()) + "\n";
    atomic_write(path, text.as_bytes())
}

/// Add a real session duration and persist it with the journal.
pub fn add_playtime(path: &Path, seconds: i64) -> std::io::Result<i64> {
    let total = load_playtime(path) + seconds.max(0);
    write_payload(path, &load_events(path, EVENT_LIMIT), total)?;
    Ok(total)
}

/// Prepend a real event and persist the capped journal atomically.
pub fn record_event(
    path: &Path,
    kind: &str,
    text: &str,
    timestamp: Option<i64>,
) -> std::io::Result<Event> {
    let event = Event {
        time: timestamp.unwrap_or_else(now_seconds),
        kind: kind.to_string(),
        text: text.to_string(),
    };
    let mut events = vec![event.clone()];
    events.extend(load_events(path, EVENT_LIMIT - 1));
    events.truncate(EVENT_LIMIT);
    write_payload(path, &events, load_playtime(path))?;
    Ok(event)
}

fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("bbport-rs-activity-{tag}-{stamp}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn events_are_newest_first_and_capped() {
        let dir = temp_dir("events");
        let path = dir.join("activity.json");
        for index in 0..25 {
            record_event(&path, "session", &format!("event {index}"), Some(index)).unwrap();
        }
        let events = load_events(&path, EVENT_LIMIT);
        assert_eq!(events.len(), EVENT_LIMIT);
        assert_eq!(events[0].time, 24);
        assert_eq!(events[19].time, 5);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn playtime_accumulates_and_ignores_bad_payloads() {
        let dir = temp_dir("playtime");
        let path = dir.join("activity.json");
        assert_eq!(add_playtime(&path, 60).unwrap(), 60);
        assert_eq!(add_playtime(&path, 30).unwrap(), 90);
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(load_playtime(&path), 0);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn malformed_events_are_skipped() {
        let dir = temp_dir("malformed");
        let path = dir.join("activity.json");
        std::fs::write(
            &path,
            r#"{"events":[{"time":1,"kind":"session","text":"ok"},{"time":"x"}],"playtime_seconds":5}"#,
        )
        .unwrap();
        let events = load_events(&path, EVENT_LIMIT);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].text, "ok");
        assert_eq!(load_playtime(&path), 5);
        let _ = std::fs::remove_dir_all(dir);
    }
}
