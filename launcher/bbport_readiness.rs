//! Non-destructive Game/Trophies/Saves checks, ported from `bbport_readiness.py`.

use crate::fsutil::{atomic_write, writable};
use crate::paths::expand;
use crate::sfo::{self, SfoValue};
use crate::Paths;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Ready,
    Warning,
    Error,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub label: String,
    pub level: Level,
    pub detail: String,
}

impl Check {
    fn new(label: &str, level: Level, detail: impl Into<String>) -> Self {
        Self {
            label: label.to_string(),
            level,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOutcome {
    Unchanged,
    Copied,
    Adopted,
}

#[derive(Debug, Error)]
pub enum ReadinessError {
    #[error("not a trophy keys file: {0}")]
    InvalidKey(String),
    #[error("trophy keys I/O: {0}")]
    Io(#[from] io::Error),
}

/// Bloodborne's release trophy key. Baking it in means a new install can decrypt the local
/// trophy pack without asking the player to import `keys.json` manually.
pub const BAKED_TROPHY_KEYS: &str = r#"{"TrophyKeySet":{"ReleaseTrophyKey":"21F41A6BAD8A1D3ECA7AD586C101B7A9","CUSA00207":"21F41A6BAD8A1D3ECA7AD586C101B7A9"}}"#;

/// A `ReleaseTrophyKey` is exactly 32 hexadecimal characters.
pub fn valid_trophy_key(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn strip_bom(data: &[u8]) -> &[u8] {
    data.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(data)
}

#[derive(Deserialize)]
struct KeysFile {
    #[serde(rename = "TrophyKeySet")]
    set: KeysSet,
}

#[derive(Deserialize)]
struct KeysSet {
    #[serde(rename = "ReleaseTrophyKey")]
    key: String,
}

fn key_from_bytes(data: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(strip_bom(data)).ok()?;
    let file: KeysFile = serde_json::from_str(text).ok()?;
    valid_trophy_key(&file.set.key).then_some(file.set.key)
}

/// Read a keys.json and return its validated key; `None` for any missing or invalid file.
pub fn read_trophy_key(path: &Path) -> Option<String> {
    let data = fs::read(path).ok()?;
    key_from_bytes(&data)
}

fn write_trophy_key(text: &[u8], destination: &Path) -> io::Result<PathBuf> {
    atomic_write(destination, text)?;
    Ok(destination.to_path_buf())
}

/// Install the baked key when neither the canonical nor the active saves folder has one, then
/// keep the two copies synchronized. Existing player keys are never overwritten.
pub fn ensure_baked_trophy_key(
    paths: &Paths,
    user_dir: &Path,
) -> Result<SyncOutcome, ReadinessError> {
    let canonical = paths.trophy_keys_path();
    let user_key = user_dir.join("keys.json");
    if read_trophy_key(&canonical).is_none() && read_trophy_key(&user_key).is_none() {
        write_trophy_key(BAKED_TROPHY_KEYS.as_bytes(), &canonical)?;
    }
    sync_trophy_keys(user_dir, &canonical)
}

/// Import a trophy keys file into the saves folder, and optionally the launcher's canonical
/// copy, without a UTF-8 BOM.
pub fn import_trophy_key(
    source: &Path,
    user_dir: &Path,
    canonical: Option<&Path>,
) -> Result<PathBuf, ReadinessError> {
    let data = fs::read(source)?;
    let text = std::str::from_utf8(strip_bom(&data))
        .map_err(|_| ReadinessError::InvalidKey("not valid UTF-8".into()))?
        .to_string();
    let file: KeysFile = serde_json::from_str(&text).map_err(|_| {
        ReadinessError::InvalidKey("TrophyKeySet.ReleaseTrophyKey is missing".into())
    })?;
    if !valid_trophy_key(&file.set.key) {
        return Err(ReadinessError::InvalidKey(
            "ReleaseTrophyKey must be 32 hexadecimal characters".into(),
        ));
    }
    let destination = user_dir.join("keys.json");
    write_trophy_key(text.as_bytes(), &destination)?;
    if let Some(canonical) = canonical {
        write_trophy_key(text.as_bytes(), canonical)?;
    }
    Ok(destination)
}

/// Keep the active saves folder and the launcher's canonical key in step.
pub fn sync_trophy_keys(user_dir: &Path, canonical: &Path) -> Result<SyncOutcome, ReadinessError> {
    let user_key = user_dir.join("keys.json");
    let canonical_valid = read_trophy_key(canonical).is_some();
    let user_valid = read_trophy_key(&user_key).is_some();
    if canonical_valid {
        if user_valid && fs::read(&user_key).ok() == fs::read(canonical).ok() {
            return Ok(SyncOutcome::Unchanged);
        }
        let text = fs::read(canonical)?;
        write_trophy_key(strip_bom(&text), &user_key)?;
        return Ok(SyncOutcome::Copied);
    }
    if user_valid {
        let text = fs::read(&user_key)?;
        write_trophy_key(strip_bom(&text), canonical)?;
        return Ok(SyncOutcome::Adopted);
    }
    Ok(SyncOutcome::Unchanged)
}

fn is_cusa_id(value: &str) -> bool {
    value.len() == 9
        && value.starts_with("CUSA")
        && value.as_bytes()[4..].iter().all(u8::is_ascii_digit)
}

/// The save title declared by the game, which can differ from `TITLE_ID`.
pub fn save_title_id(game: Option<&Path>) -> String {
    if let Some(game) = game {
        if let Ok(data) = fs::read(game.join("sce_sys").join("param.sfo")) {
            if let Ok(values) = sfo::parse_sfo(&data) {
                let declared = text_value(&values, "INSTALL_DIR_SAVEDATA")
                    .or_else(|| text_value(&values, "TITLE_ID"));
                if let Some(title) = declared.filter(|value| is_cusa_id(value)) {
                    return title;
                }
            }
        }
        if let Some(name) = game.file_name().and_then(|value| value.to_str()) {
            if is_cusa_id(name) {
                return name.to_string();
            }
        }
    }
    "CUSA03173".to_string()
}

fn text_value(values: &std::collections::HashMap<String, SfoValue>, key: &str) -> Option<String> {
    match values.get(key) {
        Some(SfoValue::Text(text)) => Some(text.clone()),
        _ => None,
    }
}

/// Return Game, Trophies and Saves checks without touching game or save data.
pub fn check_readiness(game_dir: Option<&Path>, user_dir: &Path) -> Vec<Check> {
    let game = game_dir.filter(|path| !path.as_os_str().is_empty());
    let mut result = Vec::new();

    let info = game
        .filter(|path| path.join("eboot.bin").is_file())
        .and_then(sfo::game_info);
    match (game, info) {
        (_, None) => result.push(Check::new(
            "Game files",
            Level::Error,
            "Choose the extracted CUSA03173 folder containing eboot.bin.",
        )),
        (Some(game), Some(info)) => {
            let missing: Vec<&str> = ["dvdroot_ps4", "sce_module", "sce_sys"]
                .into_iter()
                .filter(|name| !game.join(name).is_dir())
                .collect();
            if !missing.is_empty() {
                result.push(Check::new(
                    "Game files",
                    Level::Error,
                    format!("Missing: {}", missing.join(", ")),
                ));
            } else if info.version != "01.09" {
                result.push(Check::new(
                    "Game files",
                    Level::Warning,
                    format!(
                        "Version {} found; update 1.09 is recommended.",
                        info.version
                    ),
                ));
            } else {
                result.push(Check::new(
                    "Game files",
                    Level::Ready,
                    "Bloodborne 1.09 and required folders found.",
                ));
            }
        }
        (None, Some(_)) => unreachable!("info is only computed from a game folder"),
    }

    check_trophies(game, user_dir, &mut result);
    check_saves(game, user_dir, &mut result);
    result
}

fn check_trophies(game: Option<&Path>, user_dir: &Path, result: &mut Vec<Check>) {
    let pack = game.and_then(|game| {
        ["trophy00.trp", "TROPHY.TRP"]
            .into_iter()
            .map(|name| game.join("sce_sys").join("trophy").join(name))
            .find(|path| path.is_file())
    });
    let Some(pack) = pack else {
        result.push(Check::new(
            "Trophies",
            Level::Warning,
            "No trophy pack found; achievements can show IDs only.",
        ));
        return;
    };

    let valid_pack = fs::read(&pack)
        .map(|data| data.get(0..4) == Some(&[0xDC, 0xA2, 0x4D, 0x00]))
        .unwrap_or(false);
    if !valid_pack {
        result.push(Check::new(
            "Trophies",
            Level::Error,
            "Trophy pack cannot be read or has an invalid header.",
        ));
        return;
    }

    let has_key = read_trophy_key(&user_dir.join("keys.json")).is_some();
    let cached_complete =
        trophy_cache_complete(&user_dir.join("trophy_cache").join("metadata.json"), &pack);
    let (mut detail, level) = if cached_complete {
        (
            "Trophy names have been loaded by the game.".to_string(),
            Level::Ready,
        )
    } else if has_key {
        (
            "Pack and key format found; launch the game to verify decryption.".to_string(),
            Level::Warning,
        )
    } else {
        (
            "Pack found; launch the game once to build the trophy cache.".to_string(),
            Level::Warning,
        )
    };

    let history = user_dir.join("trophies.txt");
    if history.is_file() {
        match fs::read_to_string(&history) {
            Ok(text) => {
                let mut unlocked = HashSet::new();
                for line in text.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() != 2 {
                        continue;
                    }
                    if let (Ok(id), Ok(_slot)) = (parts[0].parse::<i64>(), parts[1].parse::<i64>())
                    {
                        if (0..128).contains(&id) {
                            unlocked.insert(id);
                        }
                    }
                }
                detail.push_str(&format!(" {} local unlocks recorded.", unlocked.len()));
            }
            Err(_) => detail.push_str(" Unlock history cannot be read."),
        }
    }
    result.push(Check::new("Trophies", level, detail));
}

fn trophy_cache_complete(cache_file: &Path, pack: &Path) -> bool {
    let Ok(text) = fs::read_to_string(cache_file) else {
        return false;
    };
    let Ok(cached) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    if cached.get("complete") != Some(&serde_json::Value::Bool(true)) {
        return false;
    }
    let entries_present = cached
        .get("entries")
        .map(|value| match value {
            serde_json::Value::Array(entries) => !entries.is_empty(),
            serde_json::Value::Object(map) => !map.is_empty(),
            _ => false,
        })
        .unwrap_or(false);
    if !entries_present {
        return false;
    }
    let source_size = cached
        .get("source_size")
        .and_then(|value| value.as_u64().or_else(|| value.as_f64().map(|v| v as u64)));
    match (source_size, pack.metadata()) {
        (Some(size), Ok(metadata)) => size == metadata.len(),
        _ => false,
    }
}

fn check_saves(game: Option<&Path>, user_dir: &Path, result: &mut Vec<Check>) {
    if !user_dir.exists() {
        result.push(Check::new(
            "Saves",
            Level::Warning,
            "Save folder will be created on first launch.",
        ));
        return;
    }
    if !user_dir.is_dir() {
        result.push(Check::new(
            "Saves",
            Level::Error,
            "Save path is a file, not a folder.",
        ));
        return;
    }
    if !writable(user_dir) {
        result.push(Check::new(
            "Saves",
            Level::Error,
            "Save folder is not writable.",
        ));
        return;
    }
    let save_id = save_title_id(game);
    let save_root = user_dir.join("savedata");
    let found_save = find_save_data(&save_root, &save_id);
    if found_save {
        result.push(Check::new(
            "Saves",
            Level::Ready,
            format!("Save folder is writable; {save_id} save data found."),
        ));
    } else {
        result.push(Check::new(
            "Saves",
            Level::Warning,
            format!("Save folder is writable; no {save_id} save yet."),
        ));
    }
}

/// Emulates `savedata/*/{save_id}/SPRJ*/userdata*` keeping at least one non-empty file.
fn find_save_data(save_root: &Path, save_id: &str) -> bool {
    let Ok(accounts) = fs::read_dir(save_root) else {
        return false;
    };
    for account in accounts.flatten() {
        let title_dir = account.path().join(save_id);
        let Ok(games) = fs::read_dir(&title_dir) else {
            continue;
        };
        for game in games.flatten() {
            let name = game.file_name();
            if !name.to_string_lossy().starts_with("SPRJ") {
                continue;
            }
            let Ok(files) = fs::read_dir(game.path()) else {
                continue;
            };
            for file in files.flatten() {
                let file_name = file.file_name();
                if !file_name.to_string_lossy().starts_with("userdata") {
                    continue;
                }
                if let Ok(metadata) = file.metadata() {
                    if metadata.is_file() && metadata.len() > 0 {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// The game's writable user folder, mirroring `saves_dir`: an explicit saves folder, else
/// `<game folder>/user`, else the data folder while no game folder is configured.
pub fn saves_dir(settings: &crate::Settings, data_dir: &Path) -> PathBuf {
    let configured = settings.get_str("user_dir");
    if !configured.is_empty() {
        return expand(&configured);
    }
    let game = settings.get_str("game_dir");
    if !game.is_empty() {
        return expand(&game).join("user");
    }
    data_dir.join("user")
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
        let dir = std::env::temp_dir().join(format!("bbport-rs-readiness-{tag}-{stamp}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn baked_key_is_installed_when_the_player_has_none() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("bbport-rs-baked-key-{stamp}"));
        std::fs::create_dir_all(root.join("config")).unwrap();
        let paths = Paths::with_config_dir(root.join("launcher"), root.join("config"));
        let user = root.join("user");
        ensure_baked_trophy_key(&paths, &user).unwrap();
        assert_eq!(
            read_trophy_key(&paths.trophy_keys_path()).as_deref(),
            Some("21F41A6BAD8A1D3ECA7AD586C101B7A9")
        );
        assert!(read_trophy_key(&user.join("keys.json")).is_some());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn trophy_keys_are_validated() {
        assert!(valid_trophy_key("0123456789abcdef0123456789abcdef"));
        assert!(valid_trophy_key("0123456789ABCDEF0123456789ABCDEF"));
        assert!(!valid_trophy_key("0123"));
        assert!(!valid_trophy_key("0123456789abcdef0123456789abcdeZ"));
    }

    #[test]
    fn missing_game_reports_the_choose_folder_error() {
        let dir = temp_dir("missing-game");
        let checks = check_readiness(Some(&dir.join("CUSA03173")), &dir.join("user"));
        assert_eq!(checks[0].level, Level::Error);
        assert!(checks[0]
            .detail
            .contains("Choose the extracted CUSA03173 folder"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn import_and_sync_trophy_keys_remove_the_bom() {
        let dir = temp_dir("keys");
        let source = dir.join("source.json");
        let mut body = b"\xEF\xBB\xBF".to_vec();
        body.extend_from_slice(
            br#"{"TrophyKeySet":{"ReleaseTrophyKey":"0123456789abcdef0123456789abcdef"}}"#,
        );
        std::fs::write(&source, body).unwrap();
        let user = dir.join("user");
        let canonical = dir.join("canonical.json");
        let written = import_trophy_key(&source, &user, Some(&canonical)).unwrap();
        assert_eq!(written, user.join("keys.json"));
        assert!(read_trophy_key(&written).is_some());
        assert!(read_trophy_key(&canonical).is_some());
        assert!(!std::fs::read(&written)
            .unwrap()
            .starts_with(&[0xEF, 0xBB, 0xBF]));
        assert_eq!(
            sync_trophy_keys(&user, &canonical).unwrap(),
            SyncOutcome::Unchanged
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn save_title_falls_back_to_the_folder_name() {
        let dir = temp_dir("save-title");
        let game = dir.join("CUSA12345");
        std::fs::create_dir_all(&game).unwrap();
        assert_eq!(save_title_id(Some(&game)), "CUSA12345");
        assert_eq!(save_title_id(Some(&dir.join("nope"))), "CUSA03173");
        let _ = std::fs::remove_dir_all(dir);
    }
}
