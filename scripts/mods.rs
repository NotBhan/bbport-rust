//! Merge loose-file mods through links, preserving the original game and mod files, ported
//! from `scripts/mods.py`. The Linux port uses symlinks; the Windows hard-link/junction paths
//! are kept behind `cfg(windows)` fallbacks.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Top-level folders of the game's `dvdroot_ps4`: a mod made of these is a dvdroot_ps4 itself.
pub const GAME_FOLDERS: [&str; 20] = [
    "action", "chr", "event", "facegen", "font", "map", "menu", "movie", "msg", "mtd", "obj",
    "other", "param", "paramdef", "parts", "remo", "script", "sfx", "shader", "sound",
];

/// Files that come with a mod without being part of it: notes, previews, links, licences.
const EXTRA_SUFFIXES: [&str; 17] = [
    "", ".txt", ".md", ".ini", ".json", ".pdf", ".rtf", ".nfo", ".url", ".htm", ".html", ".jpg",
    ".jpeg", ".png", ".webp", ".gif", ".bmp",
];

const ARCHIVE_SUFFIXES: [&str; 3] = [".zip", ".7z", ".rar"];

pub const SEEDED: &str = ".bbport-bundled.json";

#[derive(Debug, Error)]
pub enum ModsError {
    #[error("{0}")]
    Message(String),
    #[error("{0}")]
    Io(#[from] io::Error),
}

/// `name` inside `folder`, matching an existing entry case-insensitively (mods made on Windows
/// often differ from the game's lower-case names).
pub fn child(folder: &Path, name: &str) -> PathBuf {
    if let Ok(entries) = fs::read_dir(folder) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().to_lowercase() == name.to_lowercase() {
                return entry.path();
            }
        }
    }
    folder.join(name)
}

/// `(root, prefix)`: where a mod's files are and the game path they go to, or `None`.
pub fn content_root(folder: &Path, depth: usize) -> Option<(PathBuf, String)> {
    if !folder.is_dir() {
        return None;
    }
    // Every retail release (`patches.BLOODBORNE_IDS`).
    for wrapper in [
        "",
        "app0",
        "CUSA03173",
        "CUSA00900",
        "CUSA00207",
        "CUSA00208",
        "CUSA00299",
        "CUSA01363",
        "CUSA03179",
        "CUSA03014",
        "CUSA03023",
    ] {
        let base = if wrapper.is_empty() {
            folder.to_path_buf()
        } else {
            child(folder, wrapper)
        };
        let dvdroot = child(&base, "dvdroot_ps4");
        if dvdroot.is_dir() {
            return Some((base, String::new()));
        }
    }
    let entries: Vec<PathBuf> = fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    !path
                        .file_name()
                        .map(|name| name.to_string_lossy().starts_with('.'))
                        .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();
    let folders: Vec<&PathBuf> = entries.iter().filter(|path| path.is_dir()).collect();
    if folders.iter().any(|folder| {
        folder
            .file_name()
            .map(|name| GAME_FOLDERS.contains(&name.to_string_lossy().to_lowercase().as_str()))
            .unwrap_or(false)
    }) {
        return Some((folder.to_path_buf(), "dvdroot_ps4".into()));
    }
    let plain_files = entries.iter().any(|path| {
        path.is_file()
            && !EXTRA_SUFFIXES.contains(
                &path
                    .extension()
                    .map(|extension| format!(".{}", extension.to_string_lossy().to_lowercase()))
                    .unwrap_or_default()
                    .as_str(),
            )
    });
    if depth < 3 && folders.len() == 1 && !plain_files {
        return content_root(folders[0], depth + 1);
    }
    None
}

/// Why `folder` is not a mod (`content_root` is `None`), for the launcher's list.
pub fn problem(folder: &Path) -> String {
    let mut files = Vec::new();
    collect_files_limited(folder, &mut files, 2000);
    if files.is_empty() {
        return "empty".into();
    }
    if files.iter().any(|path| {
        ARCHIVE_SUFFIXES.contains(
            &path
                .extension()
                .map(|extension| format!(".{}", extension.to_string_lossy().to_lowercase()))
                .unwrap_or_default()
                .as_str(),
        )
    }) {
        return "still an archive: extract it here".into();
    }
    let folders: Vec<PathBuf> = fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_dir()
                        && !path
                            .file_name()
                            .map(|name| name.to_string_lossy().starts_with('.'))
                            .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();
    if folders.len() > 1 {
        return "several folders: put each mod in its own folder in Mods (each with dvdroot_ps4, \
                or folders like chr, parts, map)"
            .into();
    }
    "no game files: expected dvdroot_ps4, or folders like chr, parts, map".into()
}

fn collect_files_limited(folder: &Path, out: &mut Vec<PathBuf>, limit: usize) {
    if out.len() >= limit {
        return;
    }
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= limit {
            return;
        }
        let path = entry.path();
        if path.is_file() {
            out.push(path);
        } else if path.is_dir() {
            collect_files_limited(&path, out, limit);
        }
    }
}

/// `(name, problem)` of the folders of `root` that are not mods.
pub fn unrecognized(root: &Path) -> Vec<(String, String)> {
    if !root.is_dir() {
        return Vec::new();
    }
    let mut found: Vec<(String, String)> = fs::read_dir(root)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_dir()
                        && !path
                            .file_name()
                            .map(|name| name.to_string_lossy().starts_with('.'))
                            .unwrap_or(false)
                        && content_root(path, 0).is_none()
                })
                .map(|path| {
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    let reason = problem(&path);
                    (name, reason)
                })
                .collect()
        })
        .unwrap_or_default();
    found.sort_by(|left, right| {
        (left.0.to_lowercase(), left.0.clone()).cmp(&(right.0.to_lowercase(), right.0.clone()))
    });
    found
}

/// Named mods: folders of `root` with an accepted layout (`content_root`).
pub fn discover(root: &Path) -> Vec<String> {
    if !root.is_dir() {
        return Vec::new();
    }
    let mut names: Vec<String> = fs::read_dir(root)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_dir() && content_root(path, 0).is_some())
                .map(|path| {
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort_by(|left, right| {
        (left.to_lowercase(), left.clone()).cmp(&(right.to_lowercase(), right.clone()))
    });
    names
}

/// Selected mods: `order` first, then new folders, minus `disabled`.
pub fn selected(root: &Path, config: Option<&Path>) -> Result<Vec<String>, ModsError> {
    let available = discover(root);
    let Some(config) = config.filter(|path| path.is_file()) else {
        return Ok(available);
    };
    let text = fs::read_to_string(config)?;
    let Ok(settings) = serde_json::from_str::<Value>(&text) else {
        return Ok(available);
    };
    let Some(object) = settings.as_object() else {
        return Ok(available);
    };
    let string_list = |key: &str| -> Result<Vec<String>, ModsError> {
        match object.get(key) {
            None => Ok(Vec::new()),
            Some(Value::Array(items)) => items
                .iter()
                .map(|item| {
                    item.as_str().map(str::to_string).ok_or_else(|| {
                        ModsError::Message(format!("{key} must be a list of folder names"))
                    })
                })
                .collect(),
            Some(_) => Err(ModsError::Message(format!(
                "{key} must be a list of folder names"
            ))),
        }
    };
    let disabled: BTreeSet<String> = string_list("disabled")?.into_iter().collect();
    let order = string_list("order")?;
    let mut result: Vec<String> = Vec::new();
    for name in order.iter().chain(available.iter()) {
        if available.contains(name) && !disabled.contains(name) && !result.contains(name) {
            result.push(name.clone());
        }
    }
    Ok(result)
}

/// `(game path, source)` of every file a mod replaces or adds under `dvdroot_ps4`.
pub fn mod_files(folder: &Path) -> Result<Vec<(PathBuf, PathBuf)>, ModsError> {
    let Some((root, prefix)) = content_root(folder, 0) else {
        return Err(ModsError::Message(format!(
            "{}: expected dvdroot_ps4 (or chr/, parts/, ...) inside the mod folder",
            folder.display()
        )));
    };
    let root = root.canonicalize()?;
    let mut walker = Vec::new();
    let mut found = Vec::new();
    walk(&root, &mut walker, &mut found)?;
    let mut files = Vec::new();
    for source in found {
        if !prefix.is_empty() {
            let first = source
                .strip_prefix(&root)
                .ok()
                .and_then(|relative| relative.components().next())
                .map(|component| component.as_os_str().to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if !GAME_FOLDERS.contains(&first.as_str()) {
                continue; // notes and folders beside the game's (Screenshots, Optional)
            }
        }
        let relative = source.strip_prefix(&root).map_err(|_| {
            ModsError::Message(format!(
                "{} is not under {}",
                source.display(),
                root.display()
            ))
        })?;
        let relative = if prefix.is_empty() {
            relative.to_path_buf()
        } else {
            Path::new(&prefix).join(relative)
        };
        let first = relative
            .components()
            .next()
            .map(|component| component.as_os_str().to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if first != "dvdroot_ps4" {
            if ["eboot.bin", "sce_module", "sce_sys"].contains(&first.as_str()) {
                return Err(ModsError::Message(format!(
                    "Executable/system replacement is unsupported: {}",
                    source.display()
                )));
            }
            continue;
        }
        if !source.is_file() {
            return Err(ModsError::Message(format!(
                "Not a regular mod file: {}",
                source.display()
            )));
        }
        files.push((relative, source));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn walk(
    directory: &Path,
    directories: &mut Vec<PathBuf>,
    files: &mut Vec<PathBuf>,
) -> Result<(), ModsError> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };
    let mut names: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    names.sort();
    for path in names {
        let symlink = path
            .symlink_metadata()
            .map(|meta| meta.file_type().is_symlink())
            .unwrap_or(false);
        if symlink {
            return Err(ModsError::Message(format!(
                "Mod symlinks are unsupported: {}",
                path.display()
            )));
        }
        if path.is_dir() {
            directories.push(path.clone());
            walk(&path, directories, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

pub fn is_link(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
}

pub fn link_target(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

pub fn remove_link(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::IsADirectory => fs::remove_dir(path),
        Err(error) => Err(error),
    }
}

pub fn make_link(link: &Path, target: &Path) -> io::Result<()> {
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent)?;
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(not(unix))]
    {
        if target.is_dir() {
            std::os::windows::fs::symlink_dir(target, link)
        } else {
            match std::os::windows::fs::symlink_file(target, link) {
                Ok(()) => Ok(()),
                Err(_) => match fs::hard_link(target, link) {
                    Ok(()) => Ok(()),
                    Err(_) => fs::copy(target, link).map(|_| ()),
                },
            }
        }
    }
}

/// Materialize one directory level; never write through a directory link.
pub fn expand(directory: &Path) -> Result<(), ModsError> {
    if is_link(directory) {
        let target = link_target(directory);
        if !target.is_dir() {
            return Err(ModsError::Message(format!(
                "File/directory conflict at {}",
                directory.file_name().unwrap_or_default().to_string_lossy()
            )));
        }
        remove_link(directory)?;
        fs::create_dir_all(directory)?;
        for entry in fs::read_dir(&target)? {
            let entry = entry?;
            make_link(&directory.join(entry.file_name()), &entry.path())?;
        }
        Ok(())
    } else if directory.exists() && !directory.is_dir() {
        Err(ModsError::Message(format!(
            "File/directory conflict at {}",
            directory.file_name().unwrap_or_default().to_string_lossy()
        )))
    } else {
        fs::create_dir_all(directory)?;
        Ok(())
    }
}

/// Remove an overlay tree whose links are deleted, not followed.
pub fn remove_overlay(overlay: &Path) -> io::Result<()> {
    if !overlay.exists() {
        return Ok(());
    }
    fs::remove_dir_all(overlay)
}

/// Build the merged overlay for `mods` (name, folder), returning the overlay folder.
pub fn build_overlay(
    game: &Path,
    out: &Path,
    mods: &[(String, PathBuf)],
) -> Result<PathBuf, ModsError> {
    let game = game.canonicalize()?;
    let mut replacements: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut owners: BTreeMap<String, String> = BTreeMap::new();
    for (name, root) in mods {
        let mut count = 0;
        for (relative, source) in mod_files(root)? {
            let key = relative.to_string_lossy().replace('\\', "/").to_lowercase();
            if let Some(previous) = owners.get(&key) {
                eprintln!("Mods: {}: {previous} -> {name}", relative.display());
            }
            owners.insert(key, name.clone());
            replacements.push((relative, source));
            count += 1;
        }
        eprintln!("Mods: {name}: {count} files");
    }
    if replacements.is_empty() {
        return Ok(game);
    }
    let out = out.canonicalize().unwrap_or_else(|_| out.to_path_buf());
    fs::create_dir_all(&out)?;
    let mut places = vec![out];
    if let Ok(temp) = std::env::temp_dir().canonicalize() {
        if !places.contains(&temp) {
            places.push(temp);
        }
    }
    for (index, place) in places.iter().enumerate() {
        match overlay_at(&game, place, &replacements) {
            Ok(overlay) => return Ok(overlay),
            Err(error) if index + 1 < places.len() => {
                eprintln!(
                    "Mods: no overlay in {} ({error}); trying {}",
                    place.display(),
                    places[index + 1].display()
                );
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("places always has at least one entry")
}

fn overlay_at(
    game: &Path,
    out: &Path,
    replacements: &[(PathBuf, PathBuf)],
) -> Result<PathBuf, ModsError> {
    let overlay = unique_dir(out)?;
    let result = (|| -> Result<(), ModsError> {
        for entry in fs::read_dir(game)? {
            let entry = entry?;
            make_link(&overlay.join(entry.file_name()), &entry.path())?;
        }
        let (mut replaced, mut added) = (0u32, 0u32);
        for (relative, source) in replacements {
            let mut parent = overlay.clone();
            if let Some(parts) = relative.parent() {
                for part in parts.components() {
                    parent = child(&parent, &part.as_os_str().to_string_lossy());
                    expand(&parent)?;
                }
            }
            let destination = match relative.file_name() {
                Some(name) => child(&parent, &name.to_string_lossy()),
                None => continue,
            };
            if is_link(&destination) {
                if link_target(&destination).starts_with(game) {
                    replaced += 1;
                }
                remove_link(&destination)?;
            } else if destination.exists() {
                return Err(ModsError::Message(format!(
                    "File/directory conflict: {}",
                    relative.display()
                )));
            } else {
                added += 1;
            }
            make_link(&destination, source)?;
        }
        eprintln!("Mods: {replaced} game files replaced, {added} added");
        Ok(())
    })();
    match result {
        Ok(()) => Ok(overlay),
        Err(error) => {
            let _ = remove_overlay(&overlay);
            Err(error)
        }
    }
}

fn unique_dir(base: &Path) -> Result<PathBuf, ModsError> {
    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let candidate = base.join(format!("mod-game-{}-{counter}", std::process::id()));
    fs::create_dir_all(&candidate)?;
    Ok(candidate)
}

/// Copies mods the port ships into the player's mods folder once; those in `off` arrive
/// switched off in `config`. The seeded names are remembered in `.bbport-bundled.json`.
pub fn seed_bundled(
    mods_dir: &Path,
    bundled: &[PathBuf],
    off: &[PathBuf],
    config: Option<&Path>,
) -> Result<(), ModsError> {
    let off_names: BTreeSet<String> = off
        .iter()
        .map(|folder| {
            folder
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    let record = mods_dir.join(SEEDED);
    let mut seeded: BTreeSet<String> = fs::read_to_string(&record)
        .ok()
        .and_then(|text| serde_json::from_str::<Vec<String>>(&text).ok())
        .map(|names| names.into_iter().collect())
        .unwrap_or_default();
    let mut changed = false;
    for source in bundled.iter().chain(off.iter()) {
        let name = source
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if seeded.contains(&name) || !source.is_dir() {
            continue;
        }
        let target = child(mods_dir, &name);
        if !target.exists() {
            if off_names.contains(&name) {
                if let Some(config) = config {
                    disable_in_config(config, &name)?;
                }
            }
            copy_tree(source, &target)?;
            eprintln!("Mods: added the bundled {name}");
        }
        seeded.insert(name);
        changed = true;
    }
    if changed {
        fs::create_dir_all(mods_dir)?;
        let names: Vec<String> = seeded.into_iter().collect();
        fs::write(
            &record,
            serde_json::to_string(&names).unwrap_or_else(|_| "[]".into()),
        )?;
    }
    Ok(())
}

fn disable_in_config(config: &Path, name: &str) -> Result<(), ModsError> {
    let mut settings: Value = fs::read_to_string(config)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| Value::Object(Default::default()));
    let object = settings.as_object_mut().expect("checked object");
    let disabled: Vec<String> = object
        .get("disabled")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let enabled: Vec<String> = object
        .get("enabled")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if !disabled.iter().any(|value| value == name) && !enabled.iter().any(|value| value == name) {
        let mut names: BTreeSet<String> = disabled.into_iter().collect();
        names.insert(name.to_string());
        object.insert(
            "disabled".into(),
            Value::Array(names.into_iter().map(Value::String).collect()),
        );
        if let Some(parent) = config.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            config,
            serde_json::to_string_pretty(&settings).unwrap_or_else(|_| "{}".into()),
        )?;
    }
    Ok(())
}

fn copy_tree(source: &Path, target: &Path) -> Result<(), ModsError> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let destination = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &destination)?;
        } else {
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("bbport-rs-mods-{tag}-{stamp}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn content_root_accepts_the_known_layouts() {
        let dir = temp_dir("layout");
        let mod_a = dir.join("Mod A");
        fs::create_dir_all(mod_a.join("dvdroot_ps4").join("chr")).unwrap();
        assert_eq!(content_root(&mod_a, 0).unwrap().1, "");

        let mod_b = dir.join("Mod B");
        fs::create_dir_all(mod_b.join("chr")).unwrap();
        assert_eq!(content_root(&mod_b, 0).unwrap().1, "dvdroot_ps4");

        let wrapped = dir.join("Wrapper").join("Mod C");
        fs::create_dir_all(wrapped.join("dvdroot_ps4")).unwrap();
        assert!(content_root(&dir.join("Wrapper"), 0).is_some());

        let empty = dir.join("Empty");
        fs::create_dir_all(&empty).unwrap();
        assert!(content_root(&empty, 0).is_none());
        assert_eq!(problem(&empty), "empty");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn discover_and_unrecognized_sort_names() {
        let dir = temp_dir("discover");
        fs::create_dir_all(dir.join("b mod").join("dvdroot_ps4")).unwrap();
        fs::create_dir_all(dir.join("A mod").join("chr")).unwrap();
        fs::create_dir_all(dir.join("notes")).unwrap();
        fs::write(dir.join("notes").join("readme.txt"), b"x").unwrap();
        assert_eq!(
            discover(&dir),
            vec!["A mod".to_string(), "b mod".to_string()]
        );
        let bad = unrecognized(&dir);
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].0, "notes");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn mod_files_rejects_executable_replacements_and_symlinks() {
        let dir = temp_dir("modfiles");
        let good = dir.join("good");
        fs::create_dir_all(good.join("dvdroot_ps4").join("chr")).unwrap();
        fs::write(good.join("dvdroot_ps4").join("chr").join("a.chr"), b"x").unwrap();
        let files = mod_files(&good).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].0.ends_with("dvdroot_ps4/chr/a.chr"));

        let bad = dir.join("bad");
        fs::create_dir_all(bad.join("dvdroot_ps4")).unwrap();
        fs::write(bad.join("eboot.bin"), b"x").unwrap();
        let error = mod_files(&bad).unwrap_err();
        assert!(error.to_string().contains("Executable/system replacement"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn build_overlay_replaces_and_adds_files() {
        let dir = temp_dir("overlay");
        let game = dir.join("game");
        let dvdroot = game.join("dvdroot_ps4");
        fs::create_dir_all(dvdroot.join("chr")).unwrap();
        fs::write(dvdroot.join("chr").join("a.chr"), b"original").unwrap();
        let mod_folder = dir.join("mod");
        fs::create_dir_all(mod_folder.join("dvdroot_ps4").join("chr")).unwrap();
        fs::write(
            mod_folder.join("dvdroot_ps4").join("chr").join("a.chr"),
            b"modded",
        )
        .unwrap();
        fs::write(mod_folder.join("dvdroot_ps4").join("new.dat"), b"new").unwrap();

        let overlay =
            build_overlay(&game, &dir.join("out"), &[("mod".into(), mod_folder)]).unwrap();
        assert_eq!(
            fs::read(overlay.join("dvdroot_ps4").join("chr").join("a.chr")).unwrap(),
            b"modded"
        );
        assert!(overlay.join("dvdroot_ps4").join("new.dat").is_file());
        // The game itself is untouched.
        assert_eq!(
            fs::read(dvdroot.join("chr").join("a.chr")).unwrap(),
            b"original"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn selected_uses_order_and_disabled() {
        let dir = temp_dir("selected");
        fs::create_dir_all(dir.join("one").join("dvdroot_ps4")).unwrap();
        fs::create_dir_all(dir.join("two").join("dvdroot_ps4")).unwrap();
        let config = dir.join("mods.json");
        fs::write(&config, r#"{"order":["two"],"disabled":["one"]}"#).unwrap();
        assert_eq!(
            selected(&dir, Some(&config)).unwrap(),
            vec!["two".to_string()]
        );
        fs::write(&config, r#"{"disabled":"nope"}"#).unwrap();
        assert!(selected(&dir, Some(&config)).is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn seed_bundled_copies_once_and_disables_off_mods() {
        let dir = temp_dir("seed");
        let bundled = dir.join("bundled").join("Prompts");
        fs::create_dir_all(bundled.join("menu")).unwrap();
        fs::write(bundled.join("menu").join("common.tpf.dcx"), b"x").unwrap();
        let mods = dir.join("mods");
        let config = dir.join("mods.json");
        seed_bundled(&mods, &[], std::slice::from_ref(&bundled), Some(&config)).unwrap();
        assert!(mods
            .join("Prompts")
            .join("menu")
            .join("common.tpf.dcx")
            .is_file());
        let settings: Value = serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
        assert!(settings["disabled"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "Prompts"));
        // A deleted bundled mod stays deleted.
        fs::remove_dir_all(mods.join("Prompts")).unwrap();
        seed_bundled(&mods, &[], &[bundled], Some(&config)).unwrap();
        assert!(!mods.join("Prompts").exists());
        let _ = fs::remove_dir_all(dir);
    }
}
