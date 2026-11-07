//! FSR 4 assets, bundled keyboard prompts and control-scheme handling, ported from
//! `bbport_assets.py`.

use crate::i18n::tr;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const FSR4_COMMIT: &str = "ae8d628fae208813172446d1e49ed94150b04658";
pub const FSR4_BASE: &str =
    "https://raw.githubusercontent.com/FireBurn/Q2RTX/ae8d628fae208813172446d1e49ed94150b04658/baseq2/fsr4_shaders";

pub const KEYBOARD_PROMPTS: &str = "Keyboard Prompts (PCBButtons)";

/// The game's default keyboard layout the launcher writes with `apply_control_scheme`.
pub fn keyboard_prompt_layout() -> BTreeMap<&'static str, &'static str> {
    [
        ("key_interact", "E"),
        ("key_roll", "Left Shift"),
        ("key_blood_vial", "R"),
        ("key_use_item", "Q"),
        ("key_transform", "F"),
        ("key_attack", "Mouse1"),
        ("key_firearm", "Mouse2"),
        ("key_strong_attack", "none"),
        ("key_strong_modifier", "Left Ctrl"),
        ("key_l3", "Space"),
        ("key_lock_on", "Mouse3"),
        ("key_gestures", "G"),
        ("key_personal_effects", "H"),
        ("key_menu", "Escape"),
    ]
    .into_iter()
    .collect()
}

/// The FSR 4 v07 asset set of `tools/fetch_fsr4_assets.sh` (1080 and 2160 tiers).
pub fn fsr4_files() -> Vec<String> {
    let mut files = vec![
        "LICENSE-FSR4-v07.txt".to_string(),
        "rcas.spv".to_string(),
        "spd_auto_exposure.spv".to_string(),
    ];
    for model in [
        "native",
        "quality",
        "balanced",
        "performance",
        "ultraperf",
        "drs",
    ] {
        files.push(format!("fsr4_model_v07_i8_{model}_initializers.bin"));
        files.push(format!("fsr4_model_v07_i8_{model}_pre_weights.bin"));
        files.push(format!("fsr4_model_v07_i8_{model}_shader_manifest.json"));
        for tier in ["1080", "2160"] {
            files.push(format!("fsr4_model_v07_i8_{model}_{tier}_pre.spv"));
            files.push(format!("fsr4_model_v07_i8_{model}_{tier}_post.spv"));
            for pass in 1..=12 {
                files.push(format!("fsr4_model_v07_i8_{model}_{tier}_pass{pass}.spv"));
            }
        }
    }
    files
}

/// Validates an FSR 4 asset file for existence, non-emptiness, and format integrity.
pub fn validate_fsr4_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    let size = metadata.len();
    if size == 0 {
        return false;
    }
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy())
        .unwrap_or_default();
    let name = name.as_ref();
    if name.ends_with(".spv") {
        if size < 20 || size % 4 != 0 {
            return false;
        }
        let Ok(data) = std::fs::read(path) else {
            return false;
        };
        let magic = data.get(0..4);
        return magic == Some(&[0x03, 0x02, 0x23, 0x07])
            || magic == Some(&[0x07, 0x23, 0x02, 0x03]);
    }
    if name.ends_with("_initializers.bin") {
        return size == 89216;
    }
    if name.ends_with("_pre_weights.bin") {
        return size == 1024;
    }
    if name.ends_with("_shader_manifest.json") {
        return std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .map(|value| value.is_object() && !value.as_object().unwrap().is_empty())
            .unwrap_or(false);
    }
    if name == "LICENSE-FSR4-v07.txt" {
        let Ok(content) = std::fs::read_to_string(path) else {
            return false;
        };
        return ["FidelityFX", "AMD", "MIT"]
            .iter()
            .any(|marker| content.contains(marker));
    }
    true
}

/// Returns missing or invalid FSR 4 assets.
pub fn fsr4_missing(folder: Option<&Path>, port_dir: Option<&Path>) -> Vec<String> {
    let dest = folder.map(PathBuf::from).unwrap_or_else(|| {
        port_dir
            .unwrap_or_else(|| Path::new("."))
            .join("fsr4_shaders")
    });
    fsr4_files()
        .into_iter()
        .filter(|name| !validate_fsr4_file(&dest.join(name)))
        .collect()
}

/// Checks whether the active Vulkan GPU supports FSR 4 INT8 features.
pub fn fsr4_gpu_supported(port_dir: &Path) -> Option<bool> {
    let mut exe = port_dir.join("bin").join("bb-gpu-capabilities");
    if !exe.is_file() {
        exe = port_dir.join("out").join("bb-gpu-capabilities");
    }
    if !exe.is_file() {
        return None;
    }
    let output = std::process::Command::new(&exe)
        .arg("--fsr4")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    match lines.last() {
        Some(&"1") => Some(true),
        Some(&"0") => Some(false),
        _ => Some(output.status.success()),
    }
}

/// FSR 3.1.1 shader files: `None` when present, else a translated problem description.
pub fn fsr411_problem(directory: &Path, output: &str, preset: i64) -> Option<String> {
    let (width, height) = output.split_once('x')?;
    let (width, height) = (width.parse::<u64>().ok()?, height.parse::<u64>().ok()?);
    let model = format!(
        "{}{}",
        if width > 1920 || height > 1080 {
            "t2160"
        } else {
            "t1080"
        },
        if preset == 4 { "_m1" } else { "_m0" }
    );
    let mut passes = vec![
        "spd".to_string(),
        "prepass".to_string(),
        "pass0_post".to_string(),
    ];
    for index in 1..=12 {
        passes.push(format!("pass{index}"));
        passes.push(format!("pass{index}_post"));
    }
    passes.push("postpass".into());
    passes.push("rcas".into());
    for name in passes {
        let path = directory.join(&model).join(format!("{name}.spv"));
        let ok = path
            .metadata()
            .map(|meta| meta.is_file() && meta.len() >= 20 && meta.len() % 4 == 0)
            .unwrap_or(false);
        if !ok {
            return Some(
                tr("Нет или повреждён файл {}").replace("{}", &format!("{model}/{name}.spv")),
            );
        }
    }
    let path = directory.join(&model).join("initializer.bin");
    let ok = path
        .metadata()
        .map(|meta| meta.is_file() && meta.len() == 131072)
        .unwrap_or(false);
    if !ok {
        return Some(
            tr("Нет или повреждён файл {}").replace("{}", &format!("{model}/initializer.bin")),
        );
    }
    None
}

pub fn is_xbox_prompt(name: &str) -> bool {
    let lowered = name.to_lowercase();
    lowered.contains("xbox") && lowered.contains("prompt")
}

pub fn is_keyboard_prompt(name: &str) -> bool {
    let lowered = name.to_lowercase();
    name == KEYBOARD_PROMPTS || (lowered.contains("keyboard") && lowered.contains("prompt"))
}

pub fn is_prompt_mod(name: &str) -> bool {
    is_xbox_prompt(name) || is_keyboard_prompt(name)
}

/// Normalizes any control scheme string into `pc`, `xbox`, or `playstation`.
pub fn canonical_control_scheme(value: Option<&str>, default: &str) -> String {
    let Some(value) = value else {
        return default.to_string();
    };
    let lowered = value.trim().to_lowercase();
    let pc = [
        "pc",
        "keyboard",
        "kb",
        "kbm",
        "mouse",
        "keyboard & mouse",
        "keyboard and mouse",
        "pc — keyboard & mouse",
        "pc — keyboard and mouse",
        "пк — клавиатура и мышь",
    ];
    let xbox = ["xbox", "xinput", "xbox controller", "геймпад xbox"];
    let playstation = [
        "playstation",
        "ps",
        "ps4",
        "ps5",
        "dualshock",
        "dualsense",
        "playstation controller",
        "геймпад playstation",
    ];
    if pc.contains(&lowered.as_str()) {
        "pc".into()
    } else if xbox.contains(&lowered.as_str()) {
        "xbox".into()
    } else if playstation.contains(&lowered.as_str()) {
        "playstation".into()
    } else {
        default.to_string()
    }
}

/// The keyboard prompt pack the port ships, or `None` in a copy without it.
pub fn bundled_keyboard_prompts(port_dir: &Path) -> Option<PathBuf> {
    for folder in [
        port_dir.join("bundled"),
        port_dir.join("packaging").join("windows").join("bundled"),
        port_dir.join("mods"),
    ] {
        let candidate = folder.join(KEYBOARD_PROMPTS);
        if candidate.join("menu").join("common.tpf.dcx").is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Copy the bundled keyboard prompts into the mods folder (once). Returns the mod folder.
pub fn install_keyboard_prompts(mods_dir: &Path, port_dir: &Path) -> Result<PathBuf, String> {
    let source = bundled_keyboard_prompts(port_dir)
        .ok_or_else(|| tr("Клавиатурные подсказки отсутствуют в дистрибутиве"))?;
    let target = mods_dir.join(KEYBOARD_PROMPTS);
    if !target.join("menu").join("common.tpf.dcx").is_file() {
        copy_dir(&source, &target)?;
    }
    Ok(target)
}

fn copy_dir(source: &Path, target: &Path) -> Result<(), String> {
    std::fs::create_dir_all(target).map_err(|error| error.to_string())?;
    let entries = std::fs::read_dir(source).map_err(|error| error.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        let destination = target.join(entry.file_name());
        if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            copy_dir(&path, &destination)?;
        } else {
            std::fs::copy(&path, &destination).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

/// Detects the active scheme from `mods.json`, then the fallback, then the mods folder.
pub fn detect_control_scheme(
    mods_dir: &Path,
    config_file: Option<&Path>,
    fallback: Option<&str>,
) -> String {
    if let Some(config) = config_file {
        if let Ok(text) = std::fs::read_to_string(config) {
            if let Ok(profile) = serde_json::from_str::<Value>(&text) {
                let names = |key: &str| -> Vec<String> {
                    profile
                        .get(key)
                        .and_then(Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(Value::as_str)
                                .map(str::to_string)
                                .collect()
                        })
                        .unwrap_or_default()
                };
                let enabled = names("enabled");
                let disabled = names("disabled");
                if enabled.iter().any(|name| is_keyboard_prompt(name)) {
                    return "pc".into();
                }
                if enabled.iter().any(|name| is_xbox_prompt(name)) {
                    return "xbox".into();
                }
                if disabled.iter().any(|name| is_prompt_mod(name))
                    && !enabled.iter().any(|name| is_prompt_mod(name))
                {
                    return "playstation".into();
                }
            }
        }
    }
    if let Some(fallback) = fallback {
        if !fallback.is_empty() {
            let canonical = canonical_control_scheme(Some(fallback), "");
            if !canonical.is_empty() {
                return canonical;
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(mods_dir) {
        if entries.flatten().any(|entry| {
            entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false)
                && is_xbox_prompt(&entry.file_name().to_string_lossy())
        }) {
            return "xbox".into();
        }
    }
    "playstation".into()
}

/// Applies a control scheme to `mods.json` and updates prompt mod activation. Returns the
/// normalized scheme name. The keyboard layout is returned through `keyboard_layout` when the
/// caller wants to write it into `bbport.ini`.
pub fn apply_control_scheme(
    scheme: &str,
    mods_dir: &Path,
    config_file: &Path,
    port_dir: &Path,
) -> Result<String, String> {
    let scheme = canonical_control_scheme(Some(scheme), "xbox");
    if let Some(parent) = config_file.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut profile: Value = std::fs::read_to_string(config_file)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| Value::Object(Default::default()));

    let mut enabled = string_set(&profile, "enabled");
    let mut disabled = string_set(&profile, "disabled");
    let mut available = list_dirs(mods_dir);
    let mut xbox_mods: Vec<String> = available
        .iter()
        .filter(|name| is_xbox_prompt(name))
        .cloned()
        .collect();
    let mut kb_mods: Vec<String> = available
        .iter()
        .filter(|name| is_keyboard_prompt(name))
        .cloned()
        .collect();

    if scheme == "pc" {
        if kb_mods.is_empty() && !available.iter().any(|name| name == KEYBOARD_PROMPTS) {
            install_keyboard_prompts(mods_dir, port_dir)?;
            available = list_dirs(mods_dir);
            kb_mods = available
                .iter()
                .filter(|name| is_keyboard_prompt(name))
                .cloned()
                .collect();
        }
        for name in &kb_mods {
            enabled.insert(name.clone());
            disabled.remove(name);
        }
        if available.iter().any(|name| name == KEYBOARD_PROMPTS) {
            enabled.insert(KEYBOARD_PROMPTS.to_string());
            disabled.remove(KEYBOARD_PROMPTS);
        }
        for name in &xbox_mods {
            disabled.insert(name.clone());
            enabled.remove(name);
        }
    } else if scheme == "xbox" {
        for name in &kb_mods {
            disabled.insert(name.clone());
            enabled.remove(name);
        }
        if enabled.remove(KEYBOARD_PROMPTS) {
            disabled.insert(KEYBOARD_PROMPTS.to_string());
        }
        if let Some(first) = xbox_mods.first().cloned() {
            enabled.insert(first.clone());
            disabled.remove(&first);
            for extra in xbox_mods.drain(1..) {
                disabled.insert(extra.clone());
                enabled.remove(&extra);
            }
        }
    } else {
        let mut names = kb_mods;
        names.append(&mut xbox_mods);
        names.push(KEYBOARD_PROMPTS.to_string());
        for name in names {
            if available.contains(&name) || enabled.contains(&name) || disabled.contains(&name) {
                disabled.insert(name.clone());
                enabled.remove(&name);
            }
        }
    }

    let profile_object = profile.as_object_mut().expect("checked object");
    profile_object.insert(
        "enabled".into(),
        Value::Array(
            enabled
                .iter()
                .map(|name| Value::String(name.clone()))
                .collect(),
        ),
    );
    profile_object.insert(
        "disabled".into(),
        Value::Array(
            disabled
                .iter()
                .map(|name| Value::String(name.clone()))
                .collect(),
        ),
    );
    if let Some(order) = profile_object.get("order").and_then(Value::as_array) {
        let kept: Vec<Value> = order
            .iter()
            .filter(|value| {
                value.as_str().is_some_and(|name| {
                    available.contains(&name.to_string())
                        || enabled.contains(name)
                        || disabled.contains(name)
                })
            })
            .cloned()
            .collect();
        profile_object.insert("order".into(), Value::Array(kept));
    }
    let text = serde_json::to_string_pretty(&profile).unwrap_or_else(|_| "{}".into()) + "\n";
    crate::fsutil::atomic_write(config_file, text.as_bytes()).map_err(|error| error.to_string())?;
    Ok(scheme)
}

fn string_set(profile: &Value, key: &str) -> std::collections::BTreeSet<String> {
    profile
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn list_dirs(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("bbport-rs-assets-{tag}-{stamp}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn fsr4_file_list_matches_the_fetch_script() {
        let files = fsr4_files();
        assert!(files.contains(&"rcas.spv".to_string()));
        assert!(files.contains(&"fsr4_model_v07_i8_native_1080_pass12.spv".to_string()));
        assert!(files.contains(&"fsr4_model_v07_i8_drs_2160_pre.spv".to_string()));
        // 3 shared + 6 models * (3 + 2 tiers * 14)
        assert_eq!(files.len(), 3 + 6 * (3 + 2 * 14));
    }

    #[test]
    fn spv_validation_checks_size_alignment_and_magic() {
        let dir = temp_dir("spv");
        let good = dir.join("good.spv");
        std::fs::write(
            &good,
            [
                0x03, 0x02, 0x23, 0x07, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        )
        .unwrap();
        assert!(validate_fsr4_file(&good));
        let bad = dir.join("bad.spv");
        std::fs::write(&bad, [0x00; 20]).unwrap();
        assert!(!validate_fsr4_file(&bad));
        let empty = dir.join("empty.spv");
        std::fs::write(&empty, []).unwrap();
        assert!(!validate_fsr4_file(&empty));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_assets_are_listed() {
        let dir = temp_dir("missing");
        let missing = fsr4_missing(Some(&dir), None);
        assert_eq!(missing.len(), fsr4_files().len());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn keyboard_scheme_installs_and_enables_prompts() {
        let dir = temp_dir("scheme");
        let port = dir.join("port");
        let mods = port.join("mods");
        let config = dir.join("mods.json");
        let bundle = port.join("bundled").join(KEYBOARD_PROMPTS).join("menu");
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::write(bundle.join("common.tpf.dcx"), b"x").unwrap();
        std::fs::create_dir_all(mods.join("Xbox Prompts Pack")).unwrap();

        let applied = apply_control_scheme("pc", &mods, &config, &port).unwrap();
        assert_eq!(applied, "pc");
        let profile: Value =
            serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
        let enabled: Vec<&str> = profile["enabled"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert!(enabled.contains(&KEYBOARD_PROMPTS));
        assert!(mods
            .join(KEYBOARD_PROMPTS)
            .join("menu")
            .join("common.tpf.dcx")
            .is_file());

        assert_eq!(
            apply_control_scheme("xbox", &mods, &config, &port).unwrap(),
            "xbox"
        );
        let profile: Value =
            serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
        assert!(profile["disabled"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == KEYBOARD_PROMPTS));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn detect_scheme_prefers_config_then_mods_folder() {
        let dir = temp_dir("detect");
        let mods = dir.join("mods");
        std::fs::create_dir_all(mods.join(KEYBOARD_PROMPTS)).unwrap();
        assert_eq!(detect_control_scheme(&mods, None, None), "playstation");
        std::fs::create_dir_all(mods.join(WORD_XBOX)).unwrap();
        assert_eq!(detect_control_scheme(&mods, None, None), "xbox");

        let config = dir.join("mods.json");
        std::fs::write(&config, r#"{"enabled":["Keyboard Prompts (PCBButtons)"]}"#).unwrap();
        assert_eq!(detect_control_scheme(&mods, Some(&config), None), "pc");
        let _ = std::fs::remove_dir_all(dir);
    }

    const WORD_XBOX: &str = "Xbox Prompts Pack";
}
