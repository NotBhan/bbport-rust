//! The environment `run.sh` receives, ported from the Linux launcher's `game_environment`
//! (`bbport_launcher.py`) including the control-scheme/prompt logic from `bbport_assets.py`.

use crate::assets::{canonical_control_scheme, is_keyboard_prompt};
use crate::paths::{expand, Paths};
use crate::readiness::saves_dir;
use crate::Settings;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The patches folder: configured, else `<data>/patches`.
pub fn patches_dir(settings: &Settings, data_dir: &Path) -> PathBuf {
    let configured = settings.get_str("patches_dir");
    if configured.is_empty() {
        data_dir.join("patches")
    } else {
        expand(&configured)
    }
}

/// Whether the keyboard prompt mod is active: `control_scheme` is PC, or `mods.json` enables
/// a keyboard prompt mod.
pub fn keyboard_prompts_enabled(settings: &Settings, data_dir: &Path) -> bool {
    if canonical_control_scheme(Some(&settings.get_str("control_scheme")), "xbox") == "pc" {
        return true;
    }
    if !settings.get_bool("mods_enabled") {
        return false;
    }
    let config = data_dir.join("mods.json");
    let Ok(text) = std::fs::read_to_string(&config) else {
        return false;
    };
    let Ok(profile) = serde_json::from_str::<Value>(&text) else {
        return false;
    };
    profile
        .get("enabled")
        .and_then(Value::as_array)
        .map(|enabled| {
            enabled
                .iter()
                .filter_map(Value::as_str)
                .any(is_keyboard_prompt)
        })
        .unwrap_or(false)
}

/// The environment for `run.sh` from the launcher settings.
pub fn game_environment(settings: &Settings, paths: &Paths) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    env.insert(
        "BB_GAME_DIR".into(),
        expand(&settings.get_str("game_dir"))
            .to_string_lossy()
            .into_owned(),
    );
    env.insert(
        "BB_USER_DIR".into(),
        saves_dir(settings, &paths.data_dir)
            .to_string_lossy()
            .into_owned(),
    );

    let player_name = settings.get_str("player_name");
    let player_name = player_name.trim();
    if !player_name.is_empty() {
        env.insert("BB_USER_NAME".into(), player_name.to_string());
    }
    let dlc_dir = settings.get_str("dlc_dir");
    if !dlc_dir.trim().is_empty() {
        env.insert(
            "BB_DLC_DIR".into(),
            expand(&dlc_dir).to_string_lossy().into_owned(),
        );
    }

    let mods_dir = settings.get_str("mods_dir");
    let mods_dir = if mods_dir.is_empty() {
        paths.mods_dir()
    } else {
        expand(&mods_dir)
    };
    env.insert(
        "BB_MODS_DIR".into(),
        mods_dir.to_string_lossy().into_owned(),
    );
    env.insert(
        "BB_MODS_CONFIG".into(),
        paths.mods_config().to_string_lossy().into_owned(),
    );
    env.insert(
        "BB_MODS_ENABLED".into(),
        if settings.get_bool("mods_enabled") {
            "1"
        } else {
            "0"
        }
        .into(),
    );

    let keyboard = keyboard_prompts_enabled(settings, &paths.data_dir);
    env.insert(
        "BB_BUTTON_PROMPTS".into(),
        if keyboard {
            "Keyboard".into()
        } else {
            String::new()
        },
    );
    env.insert(
        "BB_KEY_PROMPT_IMAGES".into(),
        if keyboard { "1" } else { "0" }.into(),
    );

    env.insert(
        "BB_PATCHES_DIR".into(),
        patches_dir(settings, &paths.data_dir)
            .to_string_lossy()
            .into_owned(),
    );
    env.insert(
        "BB_PATCHES_CONFIG".into(),
        paths.patches_config().to_string_lossy().into_owned(),
    );
    env.insert("BB_LANGUAGE".into(), settings.get_str("language"));
    env.insert(
        "BB_FULLSCREEN".into(),
        if settings.get_bool("fullscreen") {
            "1"
        } else {
            "0"
        }
        .into(),
    );
    env.insert("BB_PRESENT_MODE".into(), settings.get_str("present_mode"));
    if settings.get_bool("hdr") {
        env.insert("BB_HDR".into(), "1".into());
    }
    env.insert("BB_FPS".into(), settings.get_str("fps_mode"));

    let fps_limit = settings.get_i64("fps_limit");
    if fps_limit > 0 {
        env.insert("BB_FPS_LIMIT".into(), fps_limit.to_string());
    }
    // 1 is the game's default; only export a different overlap bound (0 = unlimited).
    let frames_ahead = settings.get_i64("frames_ahead");
    if frames_ahead != 1 {
        env.insert("BB_FRAMES_AHEAD".into(), frames_ahead.to_string());
    }
    for (key, name) in [("draw_pipe", "BB_DRAW_PIPE"), ("readbacks", "BB_READBACKS")] {
        let value = settings.get_str(key);
        if !value.is_empty() {
            env.insert(name.into(), value);
        }
    }
    if settings.get_bool("mangohud") {
        env.insert("MANGOHUD".into(), "1".into());
    }
    for (key, name) in [
        ("frame_stats", "BB_FRAME_STATS"),
        ("gpu_profile", "BB_GPU_PROFILE"),
        ("vk_validation", "BB_VK_VALIDATION"),
    ] {
        if settings.get_bool(key) {
            env.insert(name.into(), "1".into());
        }
    }
    env.insert(
        "BB_SHADER_PREBUILD".into(),
        if settings.get_bool("precompile_shaders") {
            "1"
        } else {
            "0"
        }
        .into(),
    );
    env.insert(
        "BB_DISCORD_RPC".into(),
        if settings.get_bool("discord_rpc") {
            "1"
        } else {
            "0"
        }
        .into(),
    );

    for item in settings.get_str("extra_env").split_whitespace() {
        if let Some((key, value)) = item.split_once('=') {
            env.insert(key.to_string(), value.to_string());
        }
    }
    env
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::canonical_control_scheme;
    use std::path::PathBuf;

    fn paths() -> Paths {
        Paths::for_port_dir(PathBuf::from("/repo/linux_launcher"))
    }

    #[test]
    fn control_scheme_normalizes_like_python() {
        assert_eq!(
            canonical_control_scheme(Some("PC — Keyboard & Mouse"), "xbox"),
            "pc"
        );
        assert_eq!(canonical_control_scheme(Some("XInput"), "xbox"), "xbox");
        assert_eq!(
            canonical_control_scheme(Some("DualSense"), "xbox"),
            "playstation"
        );
        assert_eq!(canonical_control_scheme(Some("strange"), "xbox"), "xbox");
        assert_eq!(canonical_control_scheme(None, "xbox"), "xbox");
    }

    #[test]
    fn keyboard_scheme_sets_the_prompt_variables() {
        let paths = paths();
        let mut settings = Settings::new(&paths);
        settings
            .set("control_scheme", Value::String("pc".into()))
            .unwrap();
        let env = game_environment(&settings, &paths);
        assert_eq!(
            env.get("BB_BUTTON_PROMPTS").map(String::as_str),
            Some("Keyboard")
        );
        assert_eq!(
            env.get("BB_KEY_PROMPT_IMAGES").map(String::as_str),
            Some("1")
        );
    }

    #[test]
    fn frames_ahead_is_only_exported_when_it_differs_from_one() {
        let paths = paths();
        let mut settings = Settings::new(&paths);
        assert!(!game_environment(&settings, &paths).contains_key("BB_FRAMES_AHEAD"));
        settings
            .set("frames_ahead", Value::Number(0.into()))
            .unwrap();
        assert_eq!(
            game_environment(&settings, &paths)
                .get("BB_FRAMES_AHEAD")
                .map(String::as_str),
            Some("0")
        );
    }

    #[test]
    fn extra_env_entries_are_split_on_the_first_equals() {
        let paths = paths();
        let mut settings = Settings::new(&paths);
        settings
            .set("extra_env", Value::String("A=1 B=two=three".into()))
            .unwrap();
        let env = game_environment(&settings, &paths);
        assert_eq!(env.get("A").map(String::as_str), Some("1"));
        assert_eq!(env.get("B").map(String::as_str), Some("two=three"));
    }
}
