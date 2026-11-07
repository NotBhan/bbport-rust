//! Local trophy metadata, ported from `legacy/gpu/shim/bbport_trophies.h` and
//! `bbport_trophies.cpp`: the TRP pack, the metadata cache and the unlock table.
//!
//! The pack's entry decryption needs `common/aes` (still pending); a decryptor can be plugged
//! in through [`set_trp_decryptor`], and until then only the icons and IDs are available.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use serde_json::Value;

const TARGET_SCORE: i32 = 1000;
const SCORE_STEP: i32 = 5;
const BRONZE_WEIGHT: i32 = 1;
const SILVER_WEIGHT: i32 = 3;
const GOLD_WEIGHT: i32 = 6;
const PLATINUM_WEIGHT: i32 = 12;
const MAX_ENTRY_BYTES: u64 = 2 * 1024 * 1024;
const CACHE_VERSION: i64 = 2;
pub const UNLOCK_COUNT: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum Grade {
    #[default]
    Unknown = 0,
    Bronze = 1,
    Silver = 2,
    Gold = 3,
    Platinum = 4,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Entry {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub grade: Grade,
    pub hidden: bool,
    /// -1 means the grade is not known yet.
    pub score: i32,
    pub icon: PathBuf,
    pub unlocked_at: i64,
}

#[derive(Debug, Clone, Default)]
pub struct Summary {
    pub num_trophies: i32,
    pub num_platinum: i32,
    pub num_gold: i32,
    pub num_silver: i32,
    pub num_bronze: i32,
    pub unlocked_trophies: i32,
    pub unlocked_platinum: i32,
    pub unlocked_gold: i32,
    pub unlocked_silver: i32,
    pub unlocked_bronze: i32,
    pub progress_percentage: i32,
    pub platinum_id: i32,
    pub base_locked: i32,
    pub title: String,
    pub description: String,
}

struct TrophyState {
    entries: Vec<Entry>,
    complete_metadata: bool,
    game_title: String,
    game_description: String,
    unlocks: [i64; UNLOCK_COUNT],
}

impl Default for TrophyState {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            complete_metadata: false,
            game_title: String::new(),
            game_description: String::new(),
            unlocks: [0; UNLOCK_COUNT],
        }
    }
}

fn state() -> &'static Mutex<TrophyState> {
    static STATE: OnceLock<Mutex<TrophyState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(TrophyState::default()))
}

fn lock() -> MutexGuard<'static, TrophyState> {
    state().lock().unwrap_or_else(|error| error.into_inner())
}

fn weight(grade: Grade) -> i32 {
    match grade {
        Grade::Bronze => BRONZE_WEIGHT,
        Grade::Silver => SILVER_WEIGHT,
        Grade::Gold => GOLD_WEIGHT,
        Grade::Platinum => PLATINUM_WEIGHT,
        Grade::Unknown => 0,
    }
}

fn fixed_score(grade: Grade) -> i32 {
    match grade {
        Grade::Bronze => 10,
        Grade::Silver => 25,
        Grade::Gold => 50,
        Grade::Platinum => 100,
        Grade::Unknown => -1,
    }
}

fn grade_code(grade: Grade) -> &'static str {
    match grade {
        Grade::Bronze => "B",
        Grade::Silver => "S",
        Grade::Gold => "G",
        Grade::Platinum => "P",
        Grade::Unknown => "?",
    }
}

fn parse_grade(value: &str) -> Grade {
    match value {
        "B" => Grade::Bronze,
        "S" => Grade::Silver,
        "G" => Grade::Gold,
        "P" => Grade::Platinum,
        _ => Grade::Unknown,
    }
}

/// `AssignScores`: proportional scores to 1000 with the platinum taking the remainder; fixed
/// scores while any grade is unknown or no platinum exists.
pub fn assign_scores(found: &mut [Entry]) {
    let mut weight_sum = 0;
    let mut platinum_index: Option<usize> = None;
    let mut all_known = !found.is_empty();
    for (index, entry) in found.iter().enumerate() {
        let entry_weight = weight(entry.grade);
        if entry_weight == 0 {
            all_known = false;
        }
        weight_sum += entry_weight;
        if entry.grade == Grade::Platinum {
            platinum_index = Some(index);
        }
    }
    let Some(platinum_index) = platinum_index else {
        for entry in found {
            entry.score = fixed_score(entry.grade);
        }
        return;
    };
    if !all_known || weight_sum == 0 {
        for entry in found {
            entry.score = fixed_score(entry.grade);
        }
        return;
    }
    let mut total = 0;
    for entry in found.iter_mut() {
        let share =
            f64::from(TARGET_SCORE) * f64::from(weight(entry.grade)) / f64::from(weight_sum);
        entry.score = ((share / f64::from(SCORE_STEP) + 0.5) as i32) * SCORE_STEP;
        total += entry.score;
    }
    found[platinum_index].score += TARGET_SCORE - total;
}

#[derive(Debug, Clone, PartialEq)]
struct TrpEntry {
    name: String,
    offset: u64,
    size: u64,
    flag: u32,
}

fn be32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn be64(bytes: &[u8]) -> u64 {
    (u64::from(be32(bytes)) << 32) | u64::from(be32(&bytes[4..]))
}

fn read_directory(file: &mut File, file_size: u64) -> Vec<TrpEntry> {
    let mut header = [0u8; 96];
    if file.seek(SeekFrom::Start(0)).is_err() || file.read_exact(&mut header).is_err() {
        return Vec::new();
    }
    if be32(&header) != 0xDCA2_4D00 {
        return Vec::new();
    }
    let count = be32(&header[16..]);
    let stride = be32(&header[20..]);
    if count == 0
        || count > 512
        || !(64..=4096).contains(&stride)
        || u64::from(count) * u64::from(stride) > file_size.saturating_sub(96.min(file_size))
    {
        return Vec::new();
    }
    let mut result = Vec::new();
    for index in 0..count {
        let mut record = [0u8; 64];
        let position = 96 + u64::from(index) * u64::from(stride);
        if file.seek(SeekFrom::Start(position)).is_err() || file.read_exact(&mut record).is_err() {
            return Vec::new();
        }
        let Some(end) = record[..32].iter().position(|byte| *byte == 0) else {
            continue;
        };
        let entry = TrpEntry {
            name: String::from_utf8_lossy(&record[..end]).into_owned(),
            offset: be64(&record[32..]),
            size: be64(&record[40..]),
            flag: be32(&record[48..]),
        };
        if entry.size > MAX_ENTRY_BYTES
            || entry.offset > file_size
            || entry.size > file_size - entry.offset
        {
            continue;
        }
        result.push(entry);
    }
    result
}

fn read_entry(file: &mut File, entry: &TrpEntry) -> Vec<u8> {
    let mut bytes = vec![0u8; entry.size as usize];
    if file.seek(SeekFrom::Start(entry.offset)).is_err() || file.read_exact(&mut bytes).is_err() {
        return Vec::new();
    }
    bytes
}

/// `npbind.dat`'s `NPWRddddd_dd` communication id.
fn np_comm_id(game_dir: &Path) -> String {
    let Ok(data) = std::fs::read(game_dir.join("sce_sys").join("npbind.dat")) else {
        return String::new();
    };
    let text = String::from_utf8_lossy(&data);
    let bytes = text.as_bytes();
    for start in 0..bytes.len().saturating_sub(12) {
        if &bytes[start..start + 4] != b"NPWR" {
            continue;
        }
        let candidate = &bytes[start..start + 12];
        if candidate[4..9].iter().all(u8::is_ascii_digit)
            && candidate[9] == b'_'
            && candidate[10..12].iter().all(u8::is_ascii_digit)
        {
            return String::from_utf8_lossy(candidate).into_owned();
        }
    }
    String::new()
}

/// `keys.json`'s `TrophyKeySet.ReleaseTrophyKey`.
fn read_key(user_dir: &Path) -> Option<[u8; 16]> {
    let mut data = std::fs::read(user_dir.join("keys.json")).ok()?;
    if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
        data.drain(0..3);
    }
    let root: Value = serde_json::from_slice(&data).ok()?;
    let hex = root
        .get("TrophyKeySet")?
        .get("ReleaseTrophyKey")?
        .as_str()?;
    if hex.len() != 32 {
        return None;
    }
    let mut key = [0u8; 16];
    for (index, byte) in key.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some(key)
}

/// The pack's entry decryptor (`common/aes`'s CBC over the np communication id).
pub type TrpDecryptor = fn(&[u8], &[u8; 16], &str) -> Option<String>;

fn decryptor() -> &'static Mutex<Option<TrpDecryptor>> {
    static DECRYPTOR: OnceLock<Mutex<Option<TrpDecryptor>>> = OnceLock::new();
    DECRYPTOR.get_or_init(|| Mutex::new(None))
}

pub fn set_trp_decryptor(value: Option<TrpDecryptor>) {
    if let Ok(mut slot) = decryptor().lock() {
        *slot = value;
    }
}

fn xml_tag<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(&xml[start..end])
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("{name}=\"");
    let start = tag.find(&marker)? + marker.len();
    let end = tag[start..].find('"')? + start;
    Some(&tag[start..end])
}

fn parse_xml(
    xml: &str,
    found: &mut Vec<Entry>,
    title: &mut String,
    description: &mut String,
) -> bool {
    if let Some(value) = xml_tag(xml, "title-name") {
        *title = value.to_string();
    }
    if let Some(value) = xml_tag(xml, "title-detail") {
        *description = value.to_string();
    }
    let mut parsed = 0;
    let mut rest = xml;
    while let Some(start) = rest.find("<trophy ") {
        let body = &rest[start..];
        let Some(tag_end) = body.find('>') else {
            break;
        };
        let tag = &body[..tag_end];
        let element_end = body.find("</trophy>").unwrap_or(body.len());
        let element = &body[tag_end..element_end];
        rest = &body[element_end..];
        let Some(id) = attribute(tag, "id").and_then(|id| id.parse::<i32>().ok()) else {
            continue;
        };
        if !(0..UNLOCK_COUNT as i32).contains(&id) {
            continue;
        }
        let position = found
            .iter()
            .position(|entry| entry.id == id)
            .unwrap_or_else(|| {
                found.push(Entry::default());
                found.len() - 1
            });
        let entry = &mut found[position];
        entry.id = id;
        entry.name = xml_tag(element, "name").unwrap_or("").to_string();
        if entry.name.is_empty() {
            entry.name = format!("Trophy #{id}");
        }
        entry.description = xml_tag(element, "detail").unwrap_or("").to_string();
        entry.grade = parse_grade(attribute(tag, "ttype").unwrap_or(""));
        let hidden = attribute(tag, "hidden").unwrap_or("false");
        entry.hidden = hidden == "true" || hidden == "1";
        parsed += 1;
    }
    parsed > 0
}

fn trp_path(game_dir: &Path) -> Option<PathBuf> {
    let directory = game_dir.join("sce_sys").join("trophy");
    ["trophy00.trp", "TROPHY.TRP"]
        .iter()
        .map(|name| directory.join(name))
        .find(|path| path.is_file())
}

fn chosen_xml_name(language: i32, directory: &[TrpEntry]) -> Option<String> {
    let candidates = [
        format!("TROP_{language:02}.ESFM"),
        "TROP_01.ESFM".to_string(),
        "TROP.ESFM".to_string(),
    ];
    candidates.into_iter().find(|candidate| {
        directory
            .iter()
            .any(|entry| entry.name == *candidate && entry.flag == 3)
    })
}

fn write_file(path: &Path, data: &[u8]) -> bool {
    let Ok(mut file) = File::create(path) else {
        return false;
    };
    file.write_all(data).is_ok()
}

fn icon_id(name: &str) -> Option<i32> {
    let lower = name.to_ascii_lowercase();
    if lower.len() != 11 || !lower.starts_with("trop") || !lower.ends_with(".png") {
        return None;
    }
    lower[4..7].parse().ok()
}

fn attach_unlocks(state: &mut TrophyState) {
    for entry in &mut state.entries {
        if (0..UNLOCK_COUNT as i32).contains(&entry.id) {
            entry.unlocked_at = state.unlocks[entry.id as usize];
        }
    }
}

fn extract(
    file: &mut File,
    directory: &[TrpEntry],
    cache_dir: &Path,
    language: i32,
    key: Option<[u8; 16]>,
    np_comm_id: &str,
) -> (Vec<Entry>, bool) {
    let mut found = Vec::new();
    let _ = std::fs::create_dir_all(cache_dir.join("icons"));
    for item in directory {
        if item.flag != 0 {
            continue;
        }
        let Some(id) = icon_id(&item.name) else {
            continue;
        };
        if !(0..UNLOCK_COUNT as i32).contains(&id) {
            continue;
        }
        let icon = cache_dir.join("icons").join(&item.name);
        let bytes = read_entry(file, item);
        if bytes.len() < 8 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") || !write_file(&icon, &bytes)
        {
            continue;
        }
        found.push(Entry {
            id,
            name: format!("Trophy #{id}"),
            icon,
            ..Entry::default()
        });
    }
    let mut complete = false;
    let decryptor = decryptor().lock().map(|slot| *slot).ok().flatten();
    if let (Some(key), Some(decryptor), true) = (key, decryptor, !np_comm_id.is_empty()) {
        if let Some(selected) = chosen_xml_name(language, directory) {
            if let Some(item) = directory
                .iter()
                .find(|item| item.name == selected && item.flag == 3)
            {
                let raw = read_entry(file, item);
                let mut title = String::new();
                let mut description = String::new();
                if let Some(xml) = decryptor(&raw, &key, np_comm_id) {
                    complete = parse_xml(&xml, &mut found, &mut title, &mut description);
                }
            }
        }
    }
    found.sort_by_key(|entry| entry.id);
    assign_scores(&mut found);
    (found, complete)
}

fn cache_entry(root: &Value, cache_dir: &Path) -> Option<Entry> {
    let id = root.get("id")?.as_i64()? as i32;
    if !(0..UNLOCK_COUNT as i32).contains(&id) {
        return None;
    }
    let icon_name = root
        .get("icon")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("TROP{id:03}.PNG"));
    let icon = cache_dir.join("icons").join(icon_name);
    Some(Entry {
        id,
        name: root
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("Trophy #{id}")),
        description: root
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        grade: parse_grade(root.get("grade").and_then(Value::as_str).unwrap_or("?")),
        hidden: root.get("hidden").and_then(Value::as_bool).unwrap_or(false),
        score: root.get("score").and_then(Value::as_i64).unwrap_or(-1) as i32,
        icon: if icon.is_file() { icon } else { PathBuf::new() },
        unlocked_at: 0,
    })
}

/// `Configure`: reads the pack (or its cache) and fills the entries.
pub fn configure(game_dir: &Path, user_dir: &Path, language: i32) {
    let mut state = lock();
    state.entries.clear();
    state.complete_metadata = false;
    state.game_title.clear();
    state.game_description.clear();
    let Some(trp) = trp_path(game_dir) else {
        eprintln!(
            "Trophies: no trophy pack under {}/sce_sys/trophy",
            game_dir.display()
        );
        return;
    };
    let Ok(metadata) = std::fs::metadata(&trp) else {
        return;
    };
    let size = metadata.len();
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    let cache_dir = user_dir.join("trophy_cache");
    let cache_file = cache_dir.join("metadata.json");
    let key = read_key(user_dir);
    if key.is_none() {
        eprintln!(
            "Trophies: no trophy key in {}/keys.json; showing IDs only",
            user_dir.display()
        );
    }
    if let Ok(cached) = std::fs::read(&cache_file) {
        if cached.len() <= 512 * 1024 {
            if let Ok(root) = serde_json::from_slice::<Value>(&cached) {
                let matches = root.get("version").and_then(Value::as_i64) == Some(CACHE_VERSION)
                    && root.get("source_size").and_then(Value::as_u64) == Some(size)
                    && root.get("source_modified").and_then(Value::as_i64) == Some(modified)
                    && root.get("language").and_then(Value::as_i64) == Some(i64::from(language))
                    && (root.get("complete").and_then(Value::as_bool) == Some(true)
                        || key.is_none());
                if matches {
                    state.game_title = root
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    state.game_description = root
                        .get("description")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    for item in root
                        .get("entries")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default()
                    {
                        if let Some(entry) = cache_entry(&item, &cache_dir) {
                            state.entries.push(entry);
                        }
                    }
                    state.complete_metadata = root
                        .get("complete")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    attach_unlocks(&mut state);
                    return;
                }
            }
        }
    }
    let Ok(mut file) = File::open(&trp) else {
        return;
    };
    let directory = read_directory(&mut file, size);
    if directory.is_empty() {
        return;
    }
    let (entries, complete) = extract(
        &mut file,
        &directory,
        &cache_dir,
        language,
        key,
        &np_comm_id(game_dir),
    );
    state.entries = entries;
    state.complete_metadata = complete;
    attach_unlocks(&mut state);
    if !complete && key.is_some() {
        eprintln!(
            "Trophies: could not decrypt the trophy XML; check npbind.dat and the \
             ReleaseTrophyKey in keys.json"
        );
    }
    let _ = std::fs::create_dir_all(&cache_dir);
    let entries: Vec<Value> = state
        .entries
        .iter()
        .map(|entry| {
            serde_json::json!({
                "id": entry.id,
                "name": entry.name,
                "description": entry.description,
                "grade": grade_code(entry.grade),
                "hidden": entry.hidden,
                "score": entry.score,
                "icon": entry.icon.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default(),
            })
        })
        .collect();
    let root = serde_json::json!({
        "version": CACHE_VERSION,
        "source_size": size,
        "source_modified": modified,
        "language": language,
        "complete": state.complete_metadata,
        "title": state.game_title,
        "description": state.game_description,
        "entries": entries,
    });
    if let Ok(text) = serde_json::to_string_pretty(&root) {
        let _ = write_file(&cache_file, text.as_bytes());
    }
}

pub fn load_unlocks(timestamps: &[i64]) {
    let mut state = lock();
    state.unlocks = [0; UNLOCK_COUNT];
    for (index, timestamp) in timestamps.iter().take(UNLOCK_COUNT).enumerate() {
        state.unlocks[index] = *timestamp;
    }
    attach_unlocks(&mut state);
}

pub fn unlock(id: i32, timestamp: i64) {
    if !(0..UNLOCK_COUNT as i32).contains(&id) {
        return;
    }
    let mut state = lock();
    state.unlocks[id as usize] = timestamp;
    attach_unlocks(&mut state);
}

pub fn entries() -> Vec<Entry> {
    lock().entries.clone()
}

pub fn find(id: i32) -> Entry {
    let state = lock();
    if let Some(entry) = state.entries.iter().find(|entry| entry.id == id) {
        return entry.clone();
    }
    Entry {
        id,
        name: format!("Trophy #{id}"),
        unlocked_at: if (0..UNLOCK_COUNT as i32).contains(&id) {
            state.unlocks[id as usize]
        } else {
            0
        },
        ..Entry::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ScoreTotals {
    pub earned: i32,
    pub total: i32,
}

pub fn has_metadata() -> bool {
    lock().complete_metadata
}

pub fn earned_score() -> i32 {
    lock()
        .entries
        .iter()
        .filter(|entry| entry.unlocked_at != 0 && entry.score > 0)
        .map(|entry| entry.score)
        .sum()
}

pub fn total_score() -> i32 {
    let state = lock();
    if !state.complete_metadata {
        return -1;
    }
    state
        .entries
        .iter()
        .filter(|entry| entry.score > 0)
        .map(|entry| entry.score)
        .sum()
}

pub fn get_summary() -> Summary {
    let state = lock();
    let mut summary = Summary {
        title: state.game_title.clone(),
        description: state.game_description.clone(),
        num_trophies: state.entries.len() as i32,
        ..Summary::default()
    };
    for entry in &state.entries {
        match entry.grade {
            Grade::Platinum => {
                summary.num_platinum += 1;
                summary.platinum_id = entry.id;
            }
            Grade::Gold => summary.num_gold += 1,
            Grade::Silver => summary.num_silver += 1,
            Grade::Bronze => summary.num_bronze += 1,
            Grade::Unknown => {}
        }
        if entry.unlocked_at == 0 {
            if entry.grade != Grade::Platinum {
                summary.base_locked += 1;
            }
            continue;
        }
        summary.unlocked_trophies += 1;
        match entry.grade {
            Grade::Platinum => summary.unlocked_platinum += 1,
            Grade::Gold => summary.unlocked_gold += 1,
            Grade::Silver => summary.unlocked_silver += 1,
            Grade::Bronze => summary.unlocked_bronze += 1,
            Grade::Unknown => {}
        }
    }
    if summary.num_trophies > 0 {
        summary.progress_percentage = summary.unlocked_trophies * 100 / summary.num_trophies;
    }
    summary
}

pub fn is_unlocked(id: i32) -> bool {
    let state = lock();
    (0..UNLOCK_COUNT as i32).contains(&id) && state.unlocks[id as usize] != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The module keeps one process-wide state: the tests that read it run one at a time.
    fn serial() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    fn temp_dir(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "bbport_trophies_{name}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn write_pack(game_dir: &Path) {
        let directory = game_dir.join("sce_sys").join("trophy");
        std::fs::create_dir_all(&directory).unwrap();
        let mut pack = vec![0u8; 96];
        pack[0..4].copy_from_slice(&0xDCA2_4D00u32.to_be_bytes());
        pack[16..20].copy_from_slice(&2u32.to_be_bytes());
        pack[20..24].copy_from_slice(&64u32.to_be_bytes());
        let mut record = [0u8; 64];
        record[..11].copy_from_slice(b"TROP001.PNG");
        record[32..40].copy_from_slice(&224u64.to_be_bytes());
        record[40..48].copy_from_slice(&16u64.to_be_bytes());
        pack.extend_from_slice(&record);
        let mut empty = [0u8; 64];
        empty[..11].copy_from_slice(b"TROP002.PNG");
        empty[32..40].copy_from_slice(&224u64.to_be_bytes());
        empty[40..48].copy_from_slice(&16u64.to_be_bytes());
        pack.extend_from_slice(&empty);
        pack.extend_from_slice(b"\x89PNG\r\n\x1a\nicon-bytes!!");
        std::fs::write(directory.join("trophy00.trp"), &pack).unwrap();
    }

    #[test]
    fn the_pack_directory_yields_icons_and_ids() {
        let _serial = serial();
        let game_dir = temp_dir("game");
        let user_dir = temp_dir("user");
        write_pack(&game_dir);
        configure(&game_dir, &user_dir, 1);
        let first = entries();
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].id, 1);
        assert!(first[0].icon.is_file());
        assert!(!has_metadata());
        // The cache makes a second read identical.
        configure(&game_dir, &user_dir, 1);
        assert_eq!(entries(), first);
        std::fs::remove_dir_all(&game_dir).ok();
        std::fs::remove_dir_all(&user_dir).ok();
    }

    #[test]
    fn scores_sum_to_a_thousand_with_a_platinum() {
        let mut found = vec![
            Entry {
                id: 1,
                grade: Grade::Bronze,
                ..Entry::default()
            },
            Entry {
                id: 2,
                grade: Grade::Silver,
                ..Entry::default()
            },
            Entry {
                id: 3,
                grade: Grade::Gold,
                ..Entry::default()
            },
            Entry {
                id: 4,
                grade: Grade::Platinum,
                ..Entry::default()
            },
        ];
        assign_scores(&mut found);
        let total: i32 = found.iter().map(|entry| entry.score).sum();
        assert_eq!(total, TARGET_SCORE);
        assert!(found[3].score > found[2].score);
        let mut unknown = vec![Entry {
            grade: Grade::Unknown,
            ..Entry::default()
        }];
        assign_scores(&mut unknown);
        assert_eq!(unknown[0].score, -1);
    }

    #[test]
    fn unlocks_fill_the_summary_and_the_scores() {
        let _serial = serial();
        let mut state = lock();
        state.entries = vec![
            Entry {
                id: 1,
                grade: Grade::Bronze,
                score: 10,
                ..Entry::default()
            },
            Entry {
                id: 2,
                grade: Grade::Platinum,
                score: 975,
                ..Entry::default()
            },
            Entry {
                id: 3,
                grade: Grade::Bronze,
                score: 15,
                ..Entry::default()
            },
        ];
        state.complete_metadata = true;
        state.unlocks = [0; UNLOCK_COUNT];
        state.unlocks[1] = 1_700_000_000;
        attach_unlocks(&mut state);
        drop(state);
        let summary = get_summary();
        assert_eq!(summary.num_trophies, 3);
        assert_eq!(
            summary.base_locked, 1,
            "the locked bronze, not the platinum"
        );
        assert_eq!(summary.unlocked_trophies, 1);
        assert_eq!(summary.platinum_id, 2);
        assert_eq!(summary.progress_percentage, 33);
        assert_eq!(earned_score(), 10);
        assert_eq!(total_score(), 1000);
        assert!(is_unlocked(1));
        assert!(!is_unlocked(2));
        unlock(2, 1_700_000_001);
        assert_eq!(earned_score(), 985);
        assert_eq!(get_summary().unlocked_platinum, 1);
    }

    #[test]
    fn the_xml_parser_reads_attributes_and_fields() {
        let xml = "<trophyconf><title-name>Bloodborne</title-name><title-detail>Fear the \
                   blood</title-detail><trophy id=\"4\" ttype=\"G\" hidden=\"true\"><name>Father \
                   Gascoigne</name><detail>Defeat the hunter</detail></trophy></trophyconf>";
        let mut found = vec![
            Entry {
                id: 4,
                name: "Trophy #4".into(),
                ..Entry::default()
            },
            Entry {
                id: 5,
                grade: Grade::Bronze,
                ..Entry::default()
            },
        ];
        assign_scores(&mut found);
        let mut title = String::new();
        let mut description = String::new();
        assert!(parse_xml(xml, &mut found, &mut title, &mut description));
        assert_eq!(title, "Bloodborne");
        assert_eq!(description, "Fear the blood");
        assert_eq!(found[0].name, "Father Gascoigne");
        assert_eq!(found[0].grade, Grade::Gold);
        assert!(found[0].hidden);
        assert_eq!(
            found[1].name, "",
            "an entry the XML does not mention is untouched"
        );
    }
}
