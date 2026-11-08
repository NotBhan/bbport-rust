//! Read-only trophy report for the launcher, ported from `bbport_trophies.py`.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

pub const GRADES: [&str; 4] = ["B", "S", "G", "P"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrophyEntry {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub grade: String,
    pub hidden: bool,
    pub icon: Option<PathBuf>,
    pub earned: bool,
    pub unlocked_at: Option<i64>,
    pub name_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrophyReport {
    pub entries: Vec<TrophyEntry>,
    pub set_title: String,
    pub set_description: String,
    pub metadata_available: bool,
    pub metadata_complete: bool,
    pub reason: Option<String>,
    pub earned: i64,
    pub total: Option<i64>,
    pub completion: Option<i64>,
    pub grade_counts: BTreeMap<String, i64>,
    pub local_unlocks: usize,
    pub orphan_unlocks: Vec<i64>,
    pub stale: bool,
}

fn is_placeholder_name(name: &str) -> bool {
    name.strip_prefix("Trophy #")
        .map(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()))
        .unwrap_or(false)
}

fn read_json(path: &Path) -> (Option<Value>, Option<&'static str>) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return (None, Some("missing"));
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(value @ Value::Object(_)) => (Some(value), None),
        _ => (None, Some("malformed")),
    }
}

/// Strictly parse local history; malformed lines are ignored, the last duplicate wins.
fn read_unlocks(path: &Path) -> HashMap<i64, i64> {
    let mut unlocks = HashMap::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return unlocks;
    };
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 2
            || !parts[0].bytes().all(|byte| byte.is_ascii_digit())
            || !parts[1].bytes().all(|byte| byte.is_ascii_digit())
        {
            continue;
        }
        let (Ok(trophy_id), Ok(timestamp)) = (parts[0].parse::<i64>(), parts[1].parse::<i64>())
        else {
            continue;
        };
        if (0..128).contains(&trophy_id) && timestamp > 0 {
            unlocks.insert(trophy_id, timestamp);
        }
    }
    unlocks
}

fn safe_icon(icons: &Path, name: Option<&Value>, trophy_id: i64) -> Option<PathBuf> {
    let name = name
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("TROP{trophy_id:03}.PNG"));
    let candidate_name = Path::new(&name).file_name()?.to_string_lossy().into_owned();
    if candidate_name.is_empty() || candidate_name == "." || candidate_name == ".." {
        return None;
    }
    let candidate = icons.join(candidate_name);
    candidate.is_file().then_some(candidate)
}

fn pack_size(game_dir: Option<&Path>) -> Option<u64> {
    let game_dir = game_dir?;
    let folder = game_dir.join("sce_sys").join("trophy");
    for name in ["trophy00.trp", "TROPHY.TRP"] {
        let candidate = folder.join(name);
        if let Ok(metadata) = candidate.metadata() {
            if metadata.is_file() {
                return Some(metadata.len());
            }
        }
    }
    None
}

/// Load the local trophy cache without modifying it or the game files.
pub fn load_trophy_report(user_dir: &Path, game_dir: Option<&Path>) -> TrophyReport {
    let user = crate::paths::expand(&user_dir.to_string_lossy());
    let game = game_dir.map(|path| crate::paths::expand(&path.to_string_lossy()));
    let cache = user.join("trophy_cache").join("metadata.json");
    let icons = user.join("trophy_cache").join("icons");
    let unlocks = read_unlocks(&user.join("trophies.txt"));

    let (data, problem) = read_json(&cache);
    let Some(data) = data else {
        let reason = if problem == Some("missing") {
            "No trophy cache yet. Launch the game once (the trophy key is already installed)."
        } else {
            "Trophy metadata is malformed."
        };
        return TrophyReport {
            entries: Vec::new(),
            set_title: "Bloodborne".into(),
            set_description: String::new(),
            metadata_available: false,
            metadata_complete: false,
            reason: Some(reason.into()),
            earned: 0,
            total: None,
            completion: None,
            grade_counts: BTreeMap::new(),
            local_unlocks: unlocks.len(),
            orphan_unlocks: sorted_keys(&unlocks),
            stale: false,
        };
    };

    let version = data.get("version").cloned().unwrap_or(Value::Null);
    let complete = data.get("complete") == Some(&Value::Bool(true));
    let raw_entries = data
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    let mut all_names_available = !raw_entries.is_empty();

    for raw in &raw_entries {
        let Some(raw) = raw.as_object() else {
            continue;
        };
        let Some(trophy_id) = raw.get("id").and_then(json_int) else {
            continue;
        };
        if !(0..128).contains(&trophy_id) || !seen.insert(trophy_id) {
            continue;
        }
        let name = raw
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let description = raw
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let mut grade = raw
            .get("grade")
            .map(|value| match value {
                Value::String(text) => text.clone(),
                Value::Null => "?".to_string(),
                other => other.to_string(),
            })
            .unwrap_or_else(|| "?".to_string())
            .to_uppercase();
        if !GRADES.contains(&grade.as_str()) {
            grade = "?".into();
        }
        let name_available =
            !name.is_empty() && !is_placeholder_name(&name) && GRADES.contains(&grade.as_str());
        all_names_available = all_names_available && name_available;
        let icon = safe_icon(&icons, raw.get("icon"), trophy_id);
        entries.push(TrophyEntry {
            id: trophy_id,
            name,
            description,
            grade,
            hidden: raw.get("hidden") == Some(&Value::Bool(true)),
            icon,
            earned: unlocks.contains_key(&trophy_id),
            unlocked_at: unlocks.get(&trophy_id).copied(),
            name_available,
        });
    }

    let metadata_complete = complete && version.as_u64() == Some(2) && !entries.is_empty();
    let metadata_available = metadata_complete && all_names_available;
    let reason = if metadata_available {
        None
    } else if version.as_u64() != Some(2) {
        Some("Trophy metadata uses an older format.".into())
    } else if !complete {
        Some("The game has not decrypted the trophy pack yet.".into())
    } else {
        Some("Trophy metadata is incomplete.".into())
    };

    let entry_ids: BTreeSet<i64> = entries.iter().map(|entry| entry.id).collect();
    let orphan_unlocks: Vec<i64> = unlocks
        .keys()
        .copied()
        .filter(|id| !entry_ids.contains(id))
        .collect();
    let total = metadata_available.then_some(entries.len() as i64);
    let earned = if metadata_available {
        entries.iter().filter(|entry| entry.earned).count() as i64
    } else {
        0
    };
    let completion = total
        .filter(|value| *value > 0)
        .map(|value| earned * 100 / value);
    let mut grade_counts: BTreeMap<String, i64> =
        GRADES.iter().map(|grade| (grade.to_string(), 0)).collect();
    if metadata_available {
        for entry in &entries {
            *grade_counts.entry(entry.grade.clone()).or_insert(0) += 1;
        }
    }

    let stale = data
        .get("source_size")
        .and_then(Value::as_u64)
        .zip(pack_size(game.as_deref()))
        .map(|(cached, actual)| cached != actual)
        .unwrap_or(false);

    let title = data
        .get("title")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or("Bloodborne")
        .to_string();
    let description = data
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    TrophyReport {
        entries,
        set_title: title,
        set_description: description,
        metadata_available,
        metadata_complete,
        reason,
        earned,
        total,
        completion,
        grade_counts,
        local_unlocks: unlocks.len(),
        orphan_unlocks,
        stale,
    }
}

fn sorted_keys(unlocks: &HashMap<i64, i64>) -> Vec<i64> {
    let mut keys: Vec<i64> = unlocks.keys().copied().collect();
    keys.sort_unstable();
    keys
}

/// Python's `int(value)` for JSON scalars: numbers truncate, numeric strings parse.
fn json_int(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number
            .as_i64()
            .or_else(|| number.as_f64().map(|v| v as i64)),
        Value::String(text) => text
            .trim()
            .parse::<i64>()
            .ok()
            .or_else(|| text.trim().parse::<f64>().ok().map(|v| v as i64)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("bbport-rs-trophies-{tag}-{stamp}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn missing_cache_reports_the_import_hint() {
        let dir = temp_dir("missing");
        let report = load_trophy_report(&dir, None);
        assert!(!report.metadata_available);
        assert!(report.reason.unwrap().contains("Launch the game once"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_complete_v2_cache_becomes_available() {
        let dir = temp_dir("complete");
        let cache_dir = dir.join("trophy_cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(
            cache_dir.join("metadata.json"),
            r#"{"version":2,"complete":true,"source_size":100,
                "entries":[{"id":1,"name":"Hunter","description":"","grade":"B","icon":"x.png"},
                           {"id":2,"name":"Beast","description":"","grade":"P"}]}"#,
        )
        .unwrap();
        std::fs::write(dir.join("trophies.txt"), "1 1700000000\n2 1700000100\n").unwrap();
        let report = load_trophy_report(&dir, None);
        assert!(report.metadata_available);
        assert_eq!(report.total, Some(2));
        assert_eq!(report.earned, 2);
        assert_eq!(report.completion, Some(100));
        assert_eq!(report.grade_counts.get("P"), Some(&1));
        assert_eq!(report.local_unlocks, 2);
        assert!(report.reason.is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn placeholders_and_old_formats_are_unavailable() {
        let dir = temp_dir("placeholder");
        let cache_dir = dir.join("trophy_cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(
            cache_dir.join("metadata.json"),
            r#"{"version":1,"complete":true,"entries":[{"id":1,"name":"Trophy #1","grade":"B"}]}"#,
        )
        .unwrap();
        let report = load_trophy_report(&dir, None);
        assert!(!report.metadata_available);
        assert!(report.reason.unwrap().contains("older format"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn orphan_unlocks_and_bad_history_lines_are_reported() {
        let dir = temp_dir("orphans");
        let cache_dir = dir.join("trophy_cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(
            cache_dir.join("metadata.json"),
            r#"{"version":2,"complete":true,"entries":[{"id":1,"name":"Hunter","grade":"B"}]}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("trophies.txt"),
            "1 1700000000\n7 1700000001\nbad line\n",
        )
        .unwrap();
        let report = load_trophy_report(&dir, None);
        assert_eq!(report.local_unlocks, 2);
        assert_eq!(report.orphan_unlocks, vec![7]);
        let _ = std::fs::remove_dir_all(dir);
    }
}
