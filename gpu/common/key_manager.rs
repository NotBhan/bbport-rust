//! Trophy key storage, ported from `legacy/gpu/shadps4/common/key_manager.h` and
//! `key_manager.cpp`. Keys live in `keys.json` under the user directory, with byte vectors
//! stored as uppercase hex strings.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::gpu::common::path_util::{get_user_path, PathType};

/// `KeyManager::TrophyKeySet`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrophyKeySet {
    #[serde(rename = "ReleaseTrophyKey", with = "hex_serde", default)]
    pub release_trophy_key: Vec<u8>,
}

/// `KeyManager::AllKeys`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllKeys {
    #[serde(rename = "TrophyKeySet", default)]
    pub trophy_key_set: TrophyKeySet,
}

/// The errors the hex conversion can raise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HexError {
    OddLength,
    InvalidCharacter,
}

impl std::fmt::Display for HexError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OddLength => formatter.write_str("Invalid hex string length"),
            Self::InvalidCharacter => formatter.write_str("Invalid hex character"),
        }
    }
}

impl std::error::Error for HexError {}

fn hex_char_to_int(character: char) -> Result<u8, HexError> {
    match character {
        '0'..='9' => Ok(character as u8 - b'0'),
        'A'..='F' => Ok(character as u8 - b'A' + 10),
        'a'..='f' => Ok(character as u8 - b'a' + 10),
        _ => Err(HexError::InvalidCharacter),
    }
}

/// `KeyManager::HexStringToBytes`.
pub fn hex_string_to_bytes(hex_string: &str) -> Result<Vec<u8>, HexError> {
    if hex_string.is_empty() {
        return Ok(Vec::new());
    }
    if !hex_string.len().is_multiple_of(2) {
        return Err(HexError::OddLength);
    }
    let bytes = hex_string.as_bytes();
    let mut result = Vec::with_capacity(hex_string.len() / 2);
    for index in (0..bytes.len()).step_by(2) {
        let high = hex_char_to_int(bytes[index] as char)?;
        let low = hex_char_to_int(bytes[index + 1] as char)?;
        result.push((high << 4) | low);
    }
    Ok(result)
}

/// `KeyManager::BytesToHexString`.
pub fn bytes_to_hex_string(bytes: &[u8]) -> String {
    const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex.push(HEX_DIGITS[(byte >> 4) as usize] as char);
        hex.push(HEX_DIGITS[(byte & 0xF) as usize] as char);
    }
    hex
}

mod hex_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        super::bytes_to_hex_string(bytes).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let value = String::deserialize(deserializer)?;
        super::hex_string_to_bytes(&value).map_err(serde::de::Error::custom)
    }
}

static INSTANCE: Mutex<Option<Arc<Mutex<KeyManager>>>> = Mutex::new(None);

/// `KeyManager`.
#[derive(Debug, Clone, Default)]
pub struct KeyManager {
    keys: AllKeys,
}

impl KeyManager {
    /// `KeyManager()`.
    pub fn new() -> Self {
        Self {
            keys: AllKeys::default(),
        }
    }

    /// The `s_instance` singleton. Callers lock the returned manager to use it.
    pub fn get_instance() -> Arc<Mutex<KeyManager>> {
        let mut instance = INSTANCE.lock().unwrap_or_else(|error| error.into_inner());
        instance
            .get_or_insert_with(|| Arc::new(Mutex::new(KeyManager::new())))
            .clone()
    }

    /// `SetInstance`.
    pub fn set_instance(new_instance: Arc<Mutex<KeyManager>>) {
        let mut instance = INSTANCE.lock().unwrap_or_else(|error| error.into_inner());
        *instance = Some(new_instance);
    }

    /// `GetAllKeys`.
    pub fn get_all_keys(&self) -> &AllKeys {
        &self.keys
    }

    /// `SetAllKeys`.
    pub fn set_all_keys(&mut self, keys: AllKeys) {
        self.keys = keys;
    }

    /// `SetDefaultKeys`.
    pub fn set_default_keys(&mut self) {
        self.keys = AllKeys::default();
    }

    /// `HasKeys`.
    pub fn has_keys(&self) -> bool {
        !self.keys.trophy_key_set.release_trophy_key.is_empty()
    }

    /// `KeysToJson`.
    pub fn keys_to_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.keys).expect("the keys always serialize")
    }

    /// `JsonToKeys`: overlays the fields present in the document onto the defaults.
    pub fn json_to_keys(&mut self, value: &serde_json::Value) {
        let mut current = self.keys_to_json();
        merge_json(&mut current, value.clone());
        if let Ok(keys) = serde_json::from_value::<AllKeys>(current) {
            self.keys = keys;
        }
    }

    /// `TransferTrophyKey`.
    pub fn transfer_trophy_key(&mut self) -> bool {
        let path = get_user_path(PathType::UserDir).join("config.toml");
        self.transfer_trophy_key_from(&path)
    }

    /// The path-taking form of `TransferTrophyKey`, for tests and alternate configs.
    pub fn transfer_trophy_key_from(&mut self, path: &Path) -> bool {
        if !path.exists() {
            return false;
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            eprintln!("Got exception trying to load config file");
            return false;
        };
        let mut in_keys = false;
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_keys = line == "[Keys]";
                continue;
            }
            if !in_keys {
                continue;
            }
            let Some(value) = line.strip_prefix("TrophyKey") else {
                continue;
            };
            let Some(value) = value.trim_start().strip_prefix('=') else {
                continue;
            };
            let value = value
                .trim()
                .trim_end_matches(',')
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            if value.is_empty() {
                return false;
            }
            return match hex_string_to_bytes(value) {
                Ok(bytes) => {
                    self.keys.trophy_key_set.release_trophy_key = bytes;
                    true
                }
                Err(error) => {
                    eprintln!("Got exception trying to load config file. Exception: {error}");
                    false
                }
            };
        }
        false
    }

    /// `LoadFromFile`.
    pub fn load_from_file(&mut self) -> bool {
        let path = get_user_path(PathType::UserDir).join("keys.json");
        self.load_from_path(&path)
    }

    /// The path-taking form of `LoadFromFile`.
    pub fn load_from_path(&mut self, path: &Path) -> bool {
        if !path.exists() {
            self.set_default_keys();
            self.transfer_trophy_key();
            self.save_to_path(path);
            return true;
        }
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) => {
                eprintln!("Could not open key file: {} ({error})", path.display());
                return false;
            }
        };
        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(value) => {
                self.set_default_keys();
                self.json_to_keys(&value);
                if self.keys.trophy_key_set.release_trophy_key.is_empty()
                    && self.transfer_trophy_key()
                {
                    self.save_to_path(path);
                }
                true
            }
            Err(error) => {
                eprintln!("Error loading keys, using defaults: {error}");
                self.set_default_keys();
                false
            }
        }
    }

    /// `SaveToFile`.
    pub fn save_to_file(&mut self) -> bool {
        let path = get_user_path(PathType::UserDir).join("keys.json");
        self.save_to_path(&path)
    }

    /// The path-taking form of `SaveToFile`.
    pub fn save_to_path(&mut self, path: &Path) -> bool {
        let value = self.keys_to_json();
        let mut buffer = Vec::new();
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
        let mut serializer = serde_json::Serializer::with_formatter(&mut buffer, formatter);
        if let Err(error) = value.serialize(&mut serializer) {
            eprintln!("Error saving keys: {error}");
            return false;
        }
        if let Err(error) = std::fs::write(path, &buffer) {
            eprintln!(
                "Could not open key file for writing: {} ({error})",
                path.display()
            );
            return false;
        }
        true
    }

    /// The expected `keys.json` location.
    pub fn keys_path() -> PathBuf {
        get_user_path(PathType::UserDir).join("keys.json")
    }
}

fn merge_json(base: &mut serde_json::Value, overlay: serde_json::Value) {
    match (base, overlay) {
        (serde_json::Value::Object(base), serde_json::Value::Object(overlay)) => {
            for (key, value) in overlay {
                merge_json(base.entry(key).or_insert(serde_json::Value::Null), value);
            }
        }
        (base, overlay) => *base = overlay,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let path = std::env::temp_dir().join(format!(
                "bbport-keys-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn file(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn hex_conversion_is_uppercase_and_checked() {
        assert_eq!(bytes_to_hex_string(&[0x00, 0x1a, 0xFF]), "001AFF");
        assert_eq!(
            hex_string_to_bytes("001Aff").unwrap(),
            vec![0x00, 0x1a, 0xFF]
        );
        assert_eq!(hex_string_to_bytes(""), Ok(Vec::new()));
        assert_eq!(hex_string_to_bytes("0"), Err(HexError::OddLength));
        assert_eq!(hex_string_to_bytes("0g"), Err(HexError::InvalidCharacter));
    }

    #[test]
    fn saving_and_loading_round_trips_the_trophy_key() {
        let dir = TempDir::new();
        let path = dir.file("keys.json");
        let mut manager = KeyManager::new();
        assert!(!manager.has_keys());
        manager.keys.trophy_key_set.release_trophy_key = vec![0xde, 0xad, 0xbe, 0xef];
        assert!(manager.has_keys());
        assert!(manager.save_to_path(&path));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"ReleaseTrophyKey\": \"DEADBEEF\""));
        assert!(text.contains("    "), "the four-space indent is kept");

        let mut loaded = KeyManager::new();
        assert!(loaded.load_from_path(&path));
        assert_eq!(loaded.get_all_keys(), manager.get_all_keys());
    }

    #[test]
    fn a_missing_file_is_created_with_defaults() {
        let dir = TempDir::new();
        let path = dir.file("keys.json");
        let mut manager = KeyManager::new();
        assert!(manager.load_from_path(&path));
        assert!(path.exists());
        assert!(!manager.has_keys());
    }

    #[test]
    fn json_merging_only_replaces_present_fields() {
        let mut manager = KeyManager::new();
        manager.json_to_keys(&serde_json::json!({
            "TrophyKeySet": { "ReleaseTrophyKey": "AB01" }
        }));
        assert_eq!(
            manager.keys.trophy_key_set.release_trophy_key,
            vec![0xAB, 0x01]
        );
        manager.json_to_keys(&serde_json::json!({}));
        assert_eq!(
            manager.keys.trophy_key_set.release_trophy_key,
            vec![0xAB, 0x01]
        );
    }

    #[test]
    fn the_old_config_trophy_key_is_transferred() {
        let dir = TempDir::new();
        let config = dir.file("config.toml");
        std::fs::write(
            &config,
            "[General]\nTrophyKey = \"FF\"\n[Keys]\nTrophyKey = \"A1B2\"\n",
        )
        .unwrap();
        let mut manager = KeyManager::new();
        assert!(manager.transfer_trophy_key_from(&config));
        assert_eq!(
            manager.keys.trophy_key_set.release_trophy_key,
            vec![0xA1, 0xB2]
        );
        assert!(!manager.transfer_trophy_key_from(&dir.file("missing.toml")));
    }
}
