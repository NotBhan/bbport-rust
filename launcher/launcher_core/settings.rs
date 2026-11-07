//! `settings.json` handling: defaults, type validation and atomic persistence, ported from
//! `bbport_launcher.py`.

use crate::fsutil::atomic_write;
use crate::paths::Paths;
use serde_json::{Map, Number, Value};
use std::io;
use std::path::PathBuf;
use thiserror::Error;

pub const UI_LANGUAGES: &[&str] = &["en", "fr", "ru", "zh", "ko", "ja"];

pub const BOOL_SETTINGS: &[&str] = &[
    "mods_enabled",
    "fullscreen",
    "hdr",
    "mangohud",
    "precompile_shaders",
    "discord_rpc",
    "frame_stats",
    "gpu_profile",
    "vk_validation",
    "reduce_motion",
    "check_updates",
];

pub const STRING_SETTINGS: &[&str] = &[
    "ui_language",
    "control_scheme",
    "game_dir",
    "user_dir",
    "mods_dir",
    "patches_dir",
    "language",
    "player_name",
    "dlc_dir",
    "present_mode",
    "fps_mode",
    "draw_pipe",
    "readbacks",
    "extra_env",
];

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("unknown setting: {0}")]
    UnknownKey(String),
    #[error("invalid value for {key}: {value}")]
    InvalidValue { key: String, value: String },
    #[error("cannot write settings: {0}")]
    Io(#[from] io::Error),
}

/// The launcher's defaults for the current port layout.
pub fn defaults(paths: &Paths) -> Map<String, Value> {
    let mut values = Map::new();
    values.insert("ui_language".into(), Value::String("en".into()));
    values.insert("control_scheme".into(), Value::String("xbox".into()));
    values.insert("game_dir".into(), Value::String(paths.default_game_dir()));
    values.insert("user_dir".into(), Value::String(String::new()));
    values.insert("mods_dir".into(), Value::String(String::new()));
    values.insert("mods_enabled".into(), Value::Bool(true));
    values.insert("patches_dir".into(), Value::String(String::new()));
    values.insert("language".into(), Value::String("1".into()));
    values.insert("player_name".into(), Value::String(String::new()));
    values.insert("dlc_dir".into(), Value::String(String::new()));
    values.insert("fullscreen".into(), Value::Bool(false));
    values.insert("hdr".into(), Value::Bool(false));
    values.insert("present_mode".into(), Value::String("Mailbox".into()));
    values.insert("fps_mode".into(), Value::String("uncap".into()));
    values.insert("fps_limit".into(), Value::Number(Number::from(0)));
    values.insert("frames_ahead".into(), Value::Number(Number::from(1)));
    values.insert("draw_pipe".into(), Value::String(String::new()));
    values.insert("readbacks".into(), Value::String(String::new()));
    values.insert("mangohud".into(), Value::Bool(false));
    values.insert("precompile_shaders".into(), Value::Bool(true));
    values.insert("discord_rpc".into(), Value::Bool(true));
    values.insert("frame_stats".into(), Value::Bool(false));
    values.insert("gpu_profile".into(), Value::Bool(false));
    values.insert("vk_validation".into(), Value::Bool(false));
    values.insert("extra_env".into(), Value::String(String::new()));
    values.insert("reduce_motion".into(), Value::Bool(false));
    values.insert("check_updates".into(), Value::Bool(true));
    values
}

/// The value as the launcher uses it, or `None` when invalid for `key` (the default is kept).
pub fn setting_value(key: &str, value: &Value) -> Option<Value> {
    if BOOL_SETTINGS.contains(&key) {
        return value.is_boolean().then(|| value.clone());
    }
    if key == "ui_language" {
        return match value.as_str() {
            Some(language) if UI_LANGUAGES.contains(&language) => Some(value.clone()),
            _ => Some(Value::String("en".into())),
        };
    }
    if STRING_SETTINGS.contains(&key) {
        return value.is_string().then(|| value.clone());
    }
    if key == "fps_limit" {
        return number_clamped(value, 0.0, 480.0);
    }
    if key == "frames_ahead" {
        return number_clamped(value, 0.0, 8.0);
    }
    Some(value.clone())
}

fn number_clamped(value: &Value, low: f64, high: f64) -> Option<Value> {
    if value.is_boolean() {
        return None;
    }
    let number = value.as_f64()?;
    if !number.is_finite() {
        return None;
    }
    Some(Value::Number(
        Number::from(number.max(low).min(high) as i64),
    ))
}

/// A settings bag: known keys are validated like the Python launcher, unknown keys are kept.
#[derive(Debug, Clone)]
pub struct Settings {
    values: Map<String, Value>,
}

impl Settings {
    /// Defaults for a fresh installation.
    pub fn new(paths: &Paths) -> Self {
        Self {
            values: defaults(paths),
        }
    }

    /// Read the settings file, keeping defaults for missing or invalid entries.
    pub fn load(paths: &Paths) -> Self {
        let mut settings = Self::new(paths);
        let Ok(text) = std::fs::read_to_string(&paths.config_file) else {
            return settings;
        };
        let Ok(Value::Object(data)) = serde_json::from_str::<Value>(&text) else {
            return settings;
        };
        let known = defaults(paths);
        for (key, value) in data {
            if !known.contains_key(&key) {
                settings.values.insert(key, value);
                continue;
            }
            if let Some(valid) = setting_value(&key, &value) {
                settings.values.insert(key, valid);
            }
        }
        settings
    }

    /// Write the settings file atomically, like the Python launcher.
    pub fn save(&self, paths: &Paths) -> Result<(), SettingsError> {
        let text = serde_json::to_string_pretty(&self.values).map_err(|error| {
            SettingsError::Io(io::Error::new(io::ErrorKind::InvalidData, error))
        })?;
        atomic_write(&paths.config_file, text.as_bytes())?;
        Ok(())
    }

    pub fn values(&self) -> &Map<String, Value> {
        &self.values
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    /// Set a known key after validating it like `load` would.
    pub fn set(&mut self, key: &str, value: Value) -> Result<(), SettingsError> {
        if !defaults_contains(key) {
            return Err(SettingsError::UnknownKey(key.to_string()));
        }
        match setting_value(key, &value) {
            Some(valid) => {
                self.values.insert(key.to_string(), valid);
                Ok(())
            }
            None => Err(SettingsError::InvalidValue {
                key: key.to_string(),
                value: value.to_string(),
            }),
        }
    }

    /// Store a key without validating it, used for the launcher's own state keys
    /// (`setupCompleted`, `setupVersion`, ...) that live outside `DEFAULTS`.
    pub fn set_raw(&mut self, key: &str, value: Value) {
        self.values.insert(key.to_string(), value);
    }

    pub fn get_str(&self, key: &str) -> String {
        self.values
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    }

    pub fn get_bool(&self, key: &str) -> bool {
        self.values
            .get(key)
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    pub fn get_i64(&self, key: &str) -> i64 {
        match self.values.get(key) {
            Some(Value::Number(number)) => number.as_i64().unwrap_or(0),
            Some(Value::String(text)) => text.trim().parse().unwrap_or(0),
            _ => 0,
        }
    }

    /// Python truthiness over any JSON value, used by the start-up keys that live outside
    /// `DEFAULTS` (`setupCompleted`, `setupSkipped`, `launchDirectly`).
    pub fn is_truthy(&self, key: &str) -> bool {
        match self.values.get(key) {
            None | Some(Value::Null) => false,
            Some(Value::Bool(value)) => *value,
            Some(Value::Number(number)) => number.as_f64().map(|v| v != 0.0).unwrap_or(false),
            Some(Value::String(text)) => !text.is_empty(),
            Some(Value::Array(items)) => !items.is_empty(),
            Some(Value::Object(map)) => !map.is_empty(),
        }
    }

    /// The configured game folder, with `~` and variables expanded.
    pub fn game_dir(&self) -> PathBuf {
        crate::paths::expand(&self.get_str("game_dir"))
    }
}

fn defaults_contains(key: &str) -> bool {
    const KEYS: &[&str] = &[
        "ui_language",
        "control_scheme",
        "game_dir",
        "user_dir",
        "mods_dir",
        "mods_enabled",
        "patches_dir",
        "language",
        "player_name",
        "dlc_dir",
        "fullscreen",
        "hdr",
        "present_mode",
        "fps_mode",
        "fps_limit",
        "frames_ahead",
        "draw_pipe",
        "readbacks",
        "mangohud",
        "precompile_shaders",
        "discord_rpc",
        "frame_stats",
        "gpu_profile",
        "vk_validation",
        "extra_env",
        "reduce_motion",
        "check_updates",
    ];
    KEYS.contains(&key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn paths(tag: &str) -> (Paths, PathBuf) {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("bbport-rs-settings-{tag}-{stamp}"));
        std::fs::create_dir_all(root.join("launcher")).unwrap();
        let port = root.join("linux_launcher");
        std::fs::create_dir_all(&port).unwrap();
        (Paths::with_config_dir(port, root.join("config")), root)
    }

    #[test]
    fn defaults_match_the_python_launcher() {
        let (paths, _root) = paths("defaults");
        let settings = Settings::new(&paths);
        assert_eq!(settings.get_str("present_mode"), "Mailbox");
        assert_eq!(settings.get_str("fps_mode"), "uncap");
        assert_eq!(settings.get_i64("frames_ahead"), 1);
        assert!(settings.get_bool("precompile_shaders"));
        assert!(settings.get_bool("check_updates"));
    }

    #[test]
    fn every_default_key_is_settable() {
        let (paths, root) = paths("keys");
        let defaults = defaults(&paths);
        let mut settings = Settings::new(&paths);
        for (key, value) in &defaults {
            assert!(
                settings.set(key, value.clone()).is_ok(),
                "default key {key} is not accepted by set()"
            );
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn load_keeps_defaults_for_invalid_values_and_unknown_keys() {
        let (paths, root) = paths("load");
        std::fs::create_dir_all(&paths.config_dir).unwrap();
        std::fs::write(
            &paths.config_file,
            r#"{"fullscreen": "yes", "ui_language": "de", "fps_limit": 900, "custom_key": 7}"#,
        )
        .unwrap();
        let settings = Settings::load(&paths);
        assert!(!settings.get_bool("fullscreen"));
        assert_eq!(settings.get_str("ui_language"), "en");
        assert_eq!(settings.get_i64("fps_limit"), 480);
        assert_eq!(
            settings.get("custom_key"),
            Some(&Value::Number(Number::from(7)))
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn save_and_load_round_trip() {
        let (paths, root) = paths("round-trip");
        let mut settings = Settings::new(&paths);
        settings
            .set("player_name", Value::String("Hunter".into()))
            .unwrap();
        settings
            .set("frames_ahead", Value::Number(Number::from(0)))
            .unwrap();
        settings.save(&paths).unwrap();
        let loaded = Settings::load(&paths);
        assert_eq!(loaded.get_str("player_name"), "Hunter");
        assert_eq!(loaded.get_i64("frames_ahead"), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn unknown_settings_are_rejected_by_set() {
        let (paths, root) = paths("unknown");
        let mut settings = Settings::new(&paths);
        assert!(matches!(
            settings.set("nonsense", Value::Bool(true)),
            Err(SettingsError::UnknownKey(_))
        ));
        let _ = std::fs::remove_dir_all(root);
    }
}
