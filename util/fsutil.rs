//! Small filesystem helpers shared by the ported modules.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::process;

/// Write `contents` through a temporary file in the target directory, then rename it over
/// `path`. Mirrors the Python launcher's `atomic_write_text` (which also fsyncs the file and
/// the directory entry).
pub fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());

    let mut attempt = 0u32;
    let temp = loop {
        attempt += 1;
        let candidate = parent.join(format!(".{}.{}.{}.tmp", name, process::id(), attempt));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                let written = file.write_all(contents).and_then(|()| file.sync_all());
                if let Err(error) = written {
                    let _ = fs::remove_file(&candidate);
                    return Err(error);
                }
                break candidate;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists && attempt < 1000 => {
                continue
            }
            Err(error) => return Err(error),
        }
    };

    if let Err(error) = replace_file(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    if let Ok(directory) = File::open(parent) {
        let _ = directory.sync_all();
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_file(temp: &Path, path: &Path) -> io::Result<()> {
    fs::rename(temp, path)
}

/// Windows `rename` refuses to replace an existing file. Keep the old file under a unique
/// backup name, rename the new file into place, and restore the old file if the second rename
/// fails.
#[cfg(windows)]
fn replace_file(temp: &Path, path: &Path) -> io::Result<()> {
    match fs::rename(temp, path) {
        Ok(()) => Ok(()),
        Err(_) if path.exists() => {
            let backup = temp.with_extension("old");
            fs::rename(path, &backup)?;
            match fs::rename(temp, path) {
                Ok(()) => {
                    let _ = fs::remove_file(&backup);
                    Ok(())
                }
                Err(rename_error) => {
                    let _ = fs::rename(&backup, path);
                    Err(rename_error)
                }
            }
        }
        Err(error) => Err(error),
    }
}

/// Check that `folder` can be written to by creating and removing a temporary file.
pub fn writable(folder: &Path) -> bool {
    if !folder.is_dir() {
        return false;
    }
    let probe = folder.join(format!(".bbport-check-{}", process::id()));
    match OpenOptions::new().write(true).create_new(true).open(&probe) {
        Ok(file) => {
            drop(file);
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("bbport-rs-{tag}-{}-{stamp}", process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn atomic_write_replaces_and_leaves_no_temp_file() {
        let dir = temp_dir("atomic");
        let path = dir.join("settings.json");
        atomic_write(&path, b"first").unwrap();
        atomic_write(&path, b"second").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "second");
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn writable_reports_missing_folder_as_false() {
        let dir = temp_dir("writable");
        assert!(writable(&dir));
        assert!(!writable(&dir.join("missing")));
        let _ = fs::remove_dir_all(&dir);
    }
}
