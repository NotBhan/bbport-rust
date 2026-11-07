//! Steam non-Steam shortcuts (`shortcuts.vdf`), ported from `bbport_steam.py`.
//!
//! `shortcuts.vdf` is Valve's binary KeyValues: a type byte (0 = map, 1 = string, 2 = int32,
//! 7 = uint64), a NUL-terminated key, then the value; 8 ends a map. The file holds one map,
//! `shortcuts`, whose entries are maps named `0`, `1`, ...

use std::path::{Path, PathBuf};
use thiserror::Error;

const MAP: u8 = 0;
const STRING: u8 = 1;
const INT32: u8 = 2;
const UINT64: u8 = 7;
const END: u8 = 8;

#[derive(Debug, Error)]
pub enum SteamError {
    #[error("unknown field type {0} in shortcuts.vdf")]
    UnknownField(u8),
    #[error("truncated shortcuts.vdf")]
    Truncated,
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VdfValue {
    Map(Vec<VdfField>),
    Text(String),
    Int(i32),
    UInt(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VdfField {
    pub kind: u8,
    pub key: String,
    pub value: VdfValue,
}

impl VdfField {
    fn text(key: &str, value: impl Into<String>) -> Self {
        Self {
            kind: STRING,
            key: key.to_string(),
            value: VdfValue::Text(value.into()),
        }
    }

    fn int(key: &str, value: i32) -> Self {
        Self {
            kind: INT32,
            key: key.to_string(),
            value: VdfValue::Int(value),
        }
    }

    fn map(key: &str, value: Vec<VdfField>) -> Self {
        Self {
            kind: MAP,
            key: key.to_string(),
            value: VdfValue::Map(value),
        }
    }
}

fn cstring(data: &[u8], at: usize) -> Result<(String, usize), SteamError> {
    let end = data[at..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|offset| at + offset)
        .ok_or(SteamError::Truncated)?;
    Ok((
        String::from_utf8_lossy(&data[at..end]).into_owned(),
        end + 1,
    ))
}

/// A map as a list of fields; nested maps are lists too. Returns `(items, next offset)`.
pub fn parse(data: &[u8], mut at: usize) -> Result<(Vec<VdfField>, usize), SteamError> {
    let mut items = Vec::new();
    while at < data.len() {
        let kind = data[at];
        at += 1;
        if kind == END {
            return Ok((items, at));
        }
        let (key, next) = cstring(data, at)?;
        at = next;
        let (value, next) = match kind {
            MAP => {
                let (nested, next) = parse(data, at)?;
                (VdfValue::Map(nested), next)
            }
            STRING => {
                let (text, next) = cstring(data, at)?;
                (VdfValue::Text(text), next)
            }
            INT32 => {
                let bytes = data.get(at..at + 4).ok_or(SteamError::Truncated)?;
                (
                    VdfValue::Int(i32::from_le_bytes(bytes.try_into().unwrap())),
                    at + 4,
                )
            }
            UINT64 => {
                let bytes = data.get(at..at + 8).ok_or(SteamError::Truncated)?;
                (
                    VdfValue::UInt(u64::from_le_bytes(bytes.try_into().unwrap())),
                    at + 8,
                )
            }
            other => return Err(SteamError::UnknownField(other)),
        };
        items.push(VdfField { kind, key, value });
        at = next;
    }
    Ok((items, at))
}

pub fn dump(items: &[VdfField]) -> Vec<u8> {
    let mut out = Vec::new();
    for item in items {
        out.push(item.kind);
        out.extend_from_slice(item.key.as_bytes());
        out.push(0);
        match item.kind {
            MAP => {
                if let VdfValue::Map(nested) = &item.value {
                    out.extend_from_slice(&dump(nested));
                    out.push(END);
                }
            }
            STRING => {
                if let VdfValue::Text(text) = &item.value {
                    out.extend_from_slice(text.as_bytes());
                    out.push(0);
                }
            }
            INT32 => {
                if let VdfValue::Int(value) = &item.value {
                    out.extend_from_slice(&value.to_le_bytes());
                }
            }
            UINT64 => {
                if let VdfValue::UInt(value) = &item.value {
                    out.extend_from_slice(&value.to_le_bytes());
                }
            }
            _ => {}
        }
    }
    out
}

/// The id Steam gives a non-Steam game: CRC32 of the quoted exe and the name, top bit set.
pub fn shortcut_id(exe: &str, name: &str) -> u32 {
    let digest = crc32fast::hash(format!("{exe}{name}").as_bytes());
    digest | 0x8000_0000
}

/// The shortcut entry Steam writes for a non-Steam game.
pub fn entry(name: &str, exe: &str, start_dir: &str, icon: &str, options: &str) -> Vec<VdfField> {
    let quoted = format!("\"{exe}\"");
    let appid = shortcut_id(&quoted, name);
    vec![
        VdfField::int("appid", appid as i32),
        VdfField::text("AppName", name),
        VdfField::text("Exe", quoted),
        VdfField::text("StartDir", format!("\"{start_dir}\"")),
        VdfField::text("icon", icon),
        VdfField::text("ShortcutPath", ""),
        VdfField::text("LaunchOptions", options),
        VdfField::int("IsHidden", 0),
        VdfField::int("AllowDesktopConfig", 1),
        VdfField::int("AllowOverlay", 1),
        VdfField::int("OpenVR", 0),
        VdfField::int("Devkit", 0),
        VdfField::text("DevkitGameID", ""),
        VdfField::int("DevkitOverrideAppID", 0),
        VdfField::int("LastPlayTime", 0),
        VdfField::text("FlatpakAppID", ""),
        VdfField::map("tags", Vec::new()),
    ]
}

/// Add the shortcut to one `shortcuts.vdf`, or update the entry that starts the same exe.
/// Returns `"added"` or `"updated"`. A copy of the old file is kept as `shortcuts.vdf.bak`.
pub fn add_shortcut(
    vdf_path: &Path,
    name: &str,
    exe: &str,
    start_dir: &str,
    icon: &str,
    options: &str,
) -> Result<&'static str, SteamError> {
    let data = if vdf_path.exists() {
        std::fs::read(vdf_path)?
    } else {
        Vec::new()
    };
    let mut root = if data.is_empty() {
        Vec::new()
    } else {
        parse(&data, 0)?.0
    };
    let shortcuts_index = root
        .iter()
        .position(|field| field.kind == MAP && field.key.to_lowercase() == "shortcuts");
    let mut shortcuts = shortcuts_index
        .and_then(|index| match &root[index].value {
            VdfValue::Map(items) => Some(items.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let new = entry(name, exe, start_dir, icon, options);
    let target = format!("\"{exe}\"").to_lowercase();
    let mut result = "added";
    let mut updated = false;
    for item in shortcuts.iter_mut() {
        if item.kind != MAP {
            continue;
        }
        let VdfValue::Map(fields) = &item.value else {
            continue;
        };
        let exe_value = fields
            .iter()
            .find(|field| field.key.to_lowercase() == "exe")
            .and_then(|field| match &field.value {
                VdfValue::Text(text) => Some(text.to_lowercase()),
                _ => None,
            })
            .unwrap_or_default();
        if exe_value != target {
            continue;
        }
        // Keep what Steam or the player added (play time, tags).
        let kept = ["lastplaytime", "tags"];
        let old: Vec<(String, VdfField)> = fields
            .iter()
            .map(|field| (field.key.to_lowercase(), field.clone()))
            .collect();
        let mut merged = Vec::with_capacity(new.len());
        for field in &new {
            let lower = field.key.to_lowercase();
            if kept.contains(&lower.as_str()) {
                if let Some((_, previous)) = old.iter().find(|(key, _)| *key == lower) {
                    merged.push(previous.clone());
                    continue;
                }
            }
            merged.push(field.clone());
        }
        item.value = VdfValue::Map(merged);
        updated = true;
        break;
    }
    if updated {
        result = "updated";
    } else {
        let key = shortcuts.len().to_string();
        shortcuts.push(VdfField::map(&key, new));
    }

    if let Some(index) = shortcuts_index {
        root[index].value = VdfValue::Map(shortcuts);
    } else {
        root.push(VdfField::map("shortcuts", shortcuts));
    }

    if !data.is_empty() {
        let backup = vdf_path.with_file_name(format!(
            "{}.bak",
            vdf_path
                .file_name()
                .map(|name| name.to_string_lossy())
                .unwrap_or_default()
        ));
        std::fs::write(&backup, &data)?;
    }
    if let Some(parent) = vdf_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut output = dump(&root);
    output.push(END);
    std::fs::write(vdf_path, output)?;
    Ok(result)
}

/// Steam's install folder from the registry (Windows); `None` elsewhere, like the Python
/// module failing to import `winreg`.
pub fn steam_folder() -> Option<PathBuf> {
    None
}

/// `userdata/<account>/config` folders of the accounts that used Steam on this PC.
pub fn user_configs(steam: &Path) -> Vec<PathBuf> {
    let users = steam.join("userdata");
    if !users.is_dir() {
        return Vec::new();
    }
    let mut accounts: Vec<PathBuf> = std::fs::read_dir(&users)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| {
                    entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false)
                        && entry.file_name().to_string_lossy() != "0"
                        && entry
                            .file_name()
                            .to_string_lossy()
                            .bytes()
                            .all(|byte| byte.is_ascii_digit())
                })
                .map(|entry| entry.path().join("config"))
                .collect()
        })
        .unwrap_or_default();
    accounts.sort();
    accounts
}

/// Whether Steam is running (Windows); always false elsewhere.
pub fn steam_running() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("bbport-rs-steam-{tag}-{stamp}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn vdf_round_trips() {
        let items = vec![VdfField::map(
            "shortcuts",
            vec![VdfField::map(
                "0",
                vec![
                    VdfField::text("AppName", "Bloodborne"),
                    VdfField::int("appid", 42),
                ],
            )],
        )];
        let mut data = dump(&items);
        data.push(END);
        let (parsed, next) = parse(&data, 0).unwrap();
        assert_eq!(next, data.len());
        assert_eq!(parsed, items);
    }

    #[test]
    fn unknown_field_type_is_rejected() {
        let error = parse(&[9, b'a', 0], 0).unwrap_err();
        assert!(matches!(error, SteamError::UnknownField(9)));
    }

    #[test]
    fn shortcut_id_sets_the_top_bit() {
        let id = shortcut_id("\"C:\\Games\\Play Bloodborne.exe\"", "Bloodborne");
        assert_ne!(id & 0x8000_0000, 0);
    }

    #[test]
    fn add_then_update_keeps_play_time_and_tags() {
        let dir = temp_dir("add");
        let vdf = dir.join("shortcuts.vdf");
        assert_eq!(
            add_shortcut(
                &vdf,
                "Bloodborne",
                r"C:\Games\bb\Play.exe",
                r"C:\Games\bb",
                "",
                ""
            )
            .unwrap(),
            "added"
        );
        // Simulate Steam writing play time, then update the icon.
        let data = std::fs::read(&vdf).unwrap();
        let (mut root, _) = parse(&data, 0).unwrap();
        if let VdfValue::Map(shortcuts) = &mut root[0].value {
            if let VdfValue::Map(fields) = &mut shortcuts[0].value {
                fields.push(VdfField::int("LastPlayTime", 1234));
            }
        }
        let mut written = dump(&root);
        written.push(END);
        std::fs::write(&vdf, written).unwrap();

        assert_eq!(
            add_shortcut(
                &vdf,
                "Bloodborne",
                r"C:\Games\bb\Play.exe",
                r"C:\Games\bb",
                "new.ico",
                ""
            )
            .unwrap(),
            "updated"
        );
        assert!(dir.join("shortcuts.vdf.bak").is_file());
        let (root, _) = parse(&std::fs::read(&vdf).unwrap(), 0).unwrap();
        let VdfValue::Map(shortcuts) = &root[0].value else {
            panic!("shortcuts map missing")
        };
        let VdfValue::Map(fields) = &shortcuts[0].value else {
            panic!("entry map missing")
        };
        assert!(fields.iter().any(|field| field.key == "LastPlayTime"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn user_configs_skips_zero_and_non_numeric_accounts() {
        let dir = temp_dir("configs");
        for name in ["0", "12345", "abc"] {
            std::fs::create_dir_all(dir.join("userdata").join(name)).unwrap();
        }
        let configs = user_configs(&dir);
        assert_eq!(configs.len(), 1);
        assert!(configs[0].ends_with("12345/config"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
