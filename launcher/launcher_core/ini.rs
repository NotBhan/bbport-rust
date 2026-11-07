//! `bbport.ini` handling: defaults for the keys the launcher edits, comment-preserving
//! in-place writes, and typed getters, ported from `bbport_launcher.py`.

use crate::fsutil::atomic_write;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::Path;

/// Game effects (patches applied at start): key, default.
pub const EFFECTS: &[(&str, bool)] = &[
    ("effect_chromatic_aberration", true),
    ("effect_dof", true),
    ("effect_motion_blur", true),
    ("effect_ssao", true),
    ("effect_game_aa", true),
    ("effect_dynamic_shadows", true),
    ("effect_ssr", false),
    ("skip_intro", false),
    ("debug_camera", false),
    ("debug_menu", false),
    ("facegen", true),
];

/// PC enhancements (`scripts/enhancements.py`), off is the original game.
pub const ENHANCEMENTS: &[(&str, bool)] = &[
    ("pc_mirror_appearance", false),
    ("pc_two_invaders", false),
    ("pc_rebirth", false),
    ("pc_enhanced_lamps", false),
    ("pc_boss_rematches", false),
];

/// Cheats and gameplay tweaks for 1.09, off by default.
pub const CHEATS_AND_TWEAKS: &[(&str, bool)] = &[
    ("cheat_no_death", false),
    ("cheat_stealth", false),
    ("cheat_silent", false),
    ("cheat_rally_no_decay", false),
    ("cheat_enemy_control", false),
    ("tweak_no_rally", false),
    ("tweak_camera_distance", false),
    ("tweak_no_camera_rotation", false),
    ("tweak_easy_run", false),
    ("tweak_ragdoll", false),
];

/// The bbport.ini keys the launcher edits, in the order the Python launcher writes them.
pub fn default_keys() -> Vec<(&'static str, String)> {
    let mut keys: Vec<(&'static str, String)> = vec![
        ("upscaler", "dlss".into()),
        ("preset", "2".into()),
        ("sharpen", "1".into()),
        ("sharpness", "0.50".into()),
        ("object_motion", "1".into()),
        ("show_fps", "1".into()),
        ("output_res", "1920x1080".into()),
        ("hud_position", "100".into()),
        ("model_lod", "0".into()),
        ("live_resolution", "0".into()),
    ];
    for (key, default) in EFFECTS.iter().chain(ENHANCEMENTS).chain(CHEATS_AND_TWEAKS) {
        keys.push((key, if *default { "1".into() } else { "0".into() }));
    }
    keys
}

#[derive(Debug, Clone)]
pub struct Ini {
    values: HashMap<String, String>,
    order: Vec<String>,
    lines: Vec<String>,
}

impl Ini {
    /// Defaults for a file that does not exist yet.
    pub fn defaults() -> Self {
        let mut ini = Self {
            values: HashMap::new(),
            order: Vec::new(),
            lines: Vec::new(),
        };
        for (key, value) in default_keys() {
            ini.insert(key, value);
        }
        ini
    }

    /// Read the INI, merging its keys over the defaults and keeping the raw lines for saving.
    pub fn load(path: &Path) -> Self {
        let mut ini = Self::defaults();
        let Ok(text) = std::fs::read_to_string(path) else {
            return ini;
        };
        ini.lines = text.lines().map(str::to_string).collect();
        let lines = ini.lines.clone();
        for line in &lines {
            if !line.contains('=') || line.trim_start().starts_with('#') {
                continue;
            }
            let (key, value) = line.split_once('=').expect("checked for '='");
            ini.insert(key.trim(), value.trim().to_string());
        }
        ini
    }

    fn insert(&mut self, key: &str, value: String) {
        if !self.values.contains_key(key) {
            self.order.push(key.to_string());
        }
        self.values.insert(key.to_string(), value);
    }

    /// Rewrite the edited keys in place, append the missing ones, keep comments and other keys.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let mut written: HashSet<&str> = HashSet::new();
        let mut out: Vec<String> = Vec::with_capacity(self.lines.len() + self.order.len());
        for line in &self.lines {
            if line.contains('=') && !line.trim_start().starts_with('#') {
                let key = line.split_once('=').expect("checked for '='").0.trim();
                if let Some(value) = self.values.get(key) {
                    out.push(format!("{key}={value}"));
                    written.insert(key);
                    continue;
                }
            }
            out.push(line.clone());
        }
        if self.lines.is_empty() {
            out.push("# bbport settings (in-game menu: Insert / L3+R3)".to_string());
        }
        for key in &self.order {
            if !written.contains(key.as_str()) {
                if let Some(value) = self.values.get(key) {
                    out.push(format!("{key}={value}"));
                }
            }
        }
        atomic_write(path, format!("{}\n", out.join("\n")).as_bytes())
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        self.insert(key, value.into());
    }

    /// `int(values.get(key, fallback))`, falling back on missing or malformed values.
    pub fn int(&self, key: &str, fallback: i64) -> i64 {
        self.get(key)
            .and_then(|value| value.trim().parse::<i64>().ok())
            .unwrap_or(fallback)
    }

    /// `float(values.get(key, fallback))`, rejecting non-finite values.
    pub fn float(&self, key: &str, fallback: f64) -> f64 {
        self.get(key)
            .and_then(|value| value.trim().parse::<f64>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(fallback)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.order.iter().map(String::as_str)
    }
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
        let dir = std::env::temp_dir().join(format!("bbport-rs-ini-{tag}-{stamp}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn defaults_include_effects_and_cheats() {
        let ini = Ini::defaults();
        assert_eq!(ini.get("upscaler"), Some("dlss"));
        assert_eq!(ini.get("effect_ssao"), Some("1"));
        assert_eq!(ini.get("effect_ssr"), Some("0"));
        assert_eq!(ini.get("cheat_no_death"), Some("0"));
    }

    #[test]
    fn typed_getters_fall_back_on_bad_values() {
        let mut ini = Ini::defaults();
        ini.set("preset", "not-a-number");
        ini.set("sharpness", "NaN");
        assert_eq!(ini.int("preset", 2), 2);
        assert_eq!(ini.float("sharpness", 0.5), 0.5);
        assert_eq!(ini.float("missing", 1.25), 1.25);
    }

    #[test]
    fn save_keeps_comments_and_appends_missing_keys() {
        let dir = temp_dir("save");
        let path = dir.join("bbport.ini");
        std::fs::write(&path, "# keep me\npreset=4\nother=value\n").unwrap();
        let mut ini = Ini::load(&path);
        ini.set("preset", "2");
        ini.save(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# keep me\npreset=2\nother=value\n"));
        assert!(text.contains("\nupscaler=dlss\n"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn empty_file_gets_the_default_comment() {
        let dir = temp_dir("empty");
        let path = dir.join("bbport.ini");
        let ini = Ini::defaults();
        ini.save(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# bbport settings (in-game menu: Insert / L3+R3)\n"));
        assert!(text.contains("\nshow_fps=1\n"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
