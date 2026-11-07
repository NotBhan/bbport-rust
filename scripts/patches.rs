//! Third-party patch discovery (shadPS4/GoldHEN XML), ported from `scripts/patches.py`.
//!
//! The ELF patch compiler (`patch_fingerprint`, `compile_external`, SELF parsing) is a
//! separate milestone; this module covers the launcher-facing listing and selection.

use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Every retail release with update 1.09 (the same executable).
pub const BLOODBORNE_IDS: [&str; 9] = [
    "CUSA00900",
    "CUSA00207",
    "CUSA00208",
    "CUSA00299",
    "CUSA01363",
    "CUSA03179",
    "CUSA03173",
    "CUSA03014",
    "CUSA03023",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalPatch {
    /// `"<file name>/<patch name>"`, the launcher's selection key (`patches.json`).
    pub key: String,
    pub file: PathBuf,
    pub metadata: BTreeMap<String, String>,
}

impl ExternalPatch {
    fn enabled_by_default(&self) -> bool {
        self.metadata
            .get("isEnabled")
            .map(|value| value.to_lowercase() == "true")
            .unwrap_or(false)
    }
}

/// `[(key, file, metadata)]` of eboot patches for this version in `directory/*.xml`.
pub fn external_patches(
    directory: &Path,
    app_version: &str,
    exclude: Option<&Path>,
) -> Vec<ExternalPatch> {
    let mut found = Vec::new();
    if !directory.is_dir() {
        return found;
    }
    let exclude = exclude.and_then(|path| path.canonicalize().ok());
    let mut files: Vec<PathBuf> = std::fs::read_dir(directory)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_file()
                        && path
                            .extension()
                            .map(|extension| extension.to_string_lossy().to_lowercase() == "xml")
                            .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    for path in files {
        if exclude.is_some() && path.canonicalize().ok() == exclude {
            continue;
        }
        if path
            .file_name()
            .map(|name| name.to_string_lossy().to_lowercase() == "bbbridge-live-map.xml")
            .unwrap_or(false)
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let parsed = parse_patch_file(&text);
        let ids = match parsed {
            Ok(value) => value,
            Err(error) => {
                eprintln!("Patches: {}: {error}", path.display());
                continue;
            }
        };
        if !ids.ids.is_empty()
            && ids
                .ids
                .is_disjoint(&BLOODBORNE_IDS.iter().map(|id| id.to_string()).collect())
        {
            continue;
        }
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        for metadata in ids.metadata {
            let app_ver = metadata
                .get("AppVer")
                .map(String::as_str)
                .unwrap_or_default();
            let app_elf = metadata
                .get("AppElf")
                .map(String::as_str)
                .unwrap_or("eboot.bin");
            if app_ver != app_version || app_elf != "eboot.bin" {
                continue;
            }
            let name = metadata
                .get("Name")
                .cloned()
                .unwrap_or_else(|| "None".into());
            found.push(ExternalPatch {
                key: format!("{file_name}/{name}"),
                file: path.clone(),
                metadata,
            });
        }
    }
    found
}

#[derive(Default)]
struct ParsedPatchFile {
    ids: BTreeSet<String>,
    metadata: Vec<BTreeMap<String, String>>,
}

fn parse_patch_file(text: &str) -> Result<ParsedPatchFile, String> {
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);
    let mut parsed = ParsedPatchFile::default();
    let mut in_id = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) | Ok(Event::Empty(element)) => {
                match element.name().as_ref() {
                    b"Metadata" => {
                        let mut attributes = BTreeMap::new();
                        for attribute in element.attributes().flatten() {
                            let key = String::from_utf8_lossy(attribute.key.as_ref()).into_owned();
                            let value = attribute
                                .decode_and_unescape_value(reader.decoder())
                                .map(|value| value.into_owned())
                                .unwrap_or_default();
                            attributes.insert(key, value);
                        }
                        parsed.metadata.push(attributes);
                    }
                    b"ID" => in_id = true,
                    _ => {}
                }
            }
            Ok(Event::End(element)) => {
                if element.name().as_ref() == b"ID" {
                    in_id = false;
                }
            }
            Ok(Event::Text(value)) => {
                if in_id {
                    if let Ok(text) = value.decode() {
                        let text = text.trim();
                        if !text.is_empty() {
                            parsed.ids.insert(text.to_string());
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(error.to_string()),
            _ => {}
        }
    }
    Ok(parsed)
}

/// Selected external patches: `patches.json` `{"enabled": [...], "disabled": [...]}` overrides
/// each file's `isEnabled`.
pub fn external_selection(found: &[ExternalPatch], config: Option<&Path>) -> Vec<ExternalPatch> {
    let mut enabled = BTreeSet::new();
    let mut disabled = BTreeSet::new();
    if let Some(config) = config.filter(|path| path.is_file()) {
        if let Ok(text) = std::fs::read_to_string(config) {
            if let Ok(Value::Object(settings)) = serde_json::from_str::<Value>(&text) {
                let names = |key: &str| -> BTreeSet<String> {
                    settings
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
                enabled = names("enabled");
                disabled = names("disabled");
            }
        }
    }
    found
        .iter()
        .filter(|patch| {
            enabled.contains(&patch.key)
                || (!disabled.contains(&patch.key) && patch.enabled_by_default())
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("bbport-rs-patches-{tag}-{stamp}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const PATCH_XML: &str = r#"<?xml version="1.0"?>
<Patches>
  <Metadata AppVer="01.09" AppElf="eboot.bin" Name="Test Patch" isEnabled="true">
    <ID>CUSA03173</ID>
  </Metadata>
  <Metadata AppVer="01.09" AppElf="eboot.bin" Name="Off Patch" isEnabled="false">
    <ID>CUSA03173</ID>
  </Metadata>
</Patches>"#;

    #[test]
    fn finds_patches_for_the_version_and_game() {
        let dir = temp_dir("find");
        std::fs::write(dir.join("test.xml"), PATCH_XML).unwrap();
        let found = external_patches(&dir, "01.09", None);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].key, "test.xml/Test Patch");
        assert!(found[0].enabled_by_default());
        assert!(!found[1].enabled_by_default());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn skips_other_games_and_versions() {
        let dir = temp_dir("skip");
        std::fs::write(
            dir.join("other.xml"),
            r#"<Patches><Metadata AppVer="01.09" Name="Other"><ID>CUSA99999</ID></Metadata></Patches>"#,
        )
        .unwrap();
        std::fs::write(dir.join("bbbridge-live-map.xml"), PATCH_XML).unwrap();
        assert!(external_patches(&dir, "01.09", None).is_empty());
        std::fs::write(dir.join("version.xml"), PATCH_XML).unwrap();
        assert_eq!(external_patches(&dir, "01.00", None).len(), 0);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn config_overrides_is_enabled() {
        let dir = temp_dir("selection");
        std::fs::write(dir.join("test.xml"), PATCH_XML).unwrap();
        let found = external_patches(&dir, "01.09", None);
        let config = dir.join("patches.json");
        std::fs::write(
            &config,
            r#"{"enabled":["test.xml/Off Patch"],"disabled":["test.xml/Test Patch"]}"#,
        )
        .unwrap();
        let selected = external_selection(&found, Some(&config));
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].key, "test.xml/Off Patch");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn malformed_xml_is_skipped() {
        let dir = temp_dir("malformed");
        std::fs::write(dir.join("broken.xml"), "<Patches><Metadata").unwrap();
        assert!(external_patches(&dir, "01.09", None).is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
