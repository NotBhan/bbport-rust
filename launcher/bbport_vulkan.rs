//! Select Vulkan ICDs for the packaged port before starting GTK or the game, ported from
//! `bbport_vulkan.py`.
//!
//! Mesa comes with the package. NVIDIA's userspace driver must come from the host, matching
//! its kernel module. Only NVIDIA libraries are exposed, rather than the host's whole library
//! directory (which would also replace the package's libc/GTK/SDL).

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const MANIFEST_DIRS: [&str; 4] = [
    "/etc/vulkan/icd.d",
    "/usr/share/vulkan/icd.d",
    "/usr/local/share/vulkan/icd.d",
    "/run/opengl-driver/share/vulkan/icd.d",
];

pub const LIBRARY_DIRS: [&str; 7] = [
    "/usr/lib/x86_64-linux-gnu",
    "/lib/x86_64-linux-gnu",
    "/usr/lib64",
    "/lib64",
    "/usr/lib",
    "/lib",
    "/run/opengl-driver/lib",
];

/// Reject 32-bit ICDs in distributions that install both architectures.
pub fn elf64(path: &Path) -> bool {
    let Ok(data) = fs::read(path) else {
        return false;
    };
    let header = data.get(0..20).unwrap_or(&[]);
    header.get(0..6) == Some(b"\x7fELF\x02\x01") && header.get(18..20) == Some(&[0x3e, 0x00])
}

/// `(manifest JSON, driver path)` of the first usable host NVIDIA ICD, or `None`.
pub fn host_nvidia(
    manifest_dirs: &[PathBuf],
    library_dirs: &[PathBuf],
) -> Option<(Value, PathBuf)> {
    for directory in manifest_dirs {
        let mut manifests: Vec<PathBuf> = fs::read_dir(directory)
            .map(|entries| {
                entries
                    .flatten()
                    .map(|entry| entry.path())
                    .filter(|path| {
                        let name = path
                            .file_name()
                            .map(|value| value.to_string_lossy().to_lowercase())
                            .unwrap_or_default();
                        name.contains("nvidia") && name.ends_with(".json")
                    })
                    .collect()
            })
            .unwrap_or_default();
        manifests.sort();
        for manifest in manifests {
            let Ok(text) = fs::read_to_string(&manifest) else {
                continue;
            };
            let Ok(data) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            let Some(library) = data
                .get("ICD")
                .and_then(|icd| icd.get("library_path"))
                .and_then(Value::as_str)
            else {
                continue;
            };
            let library = PathBuf::from(library);
            let library_name = library
                .file_name()
                .map(|value| value.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if !library_name.contains("nvidia") {
                continue;
            }
            let candidates: Vec<PathBuf> = if library.is_absolute() {
                let mut candidates = vec![library.clone()];
                candidates.extend(library_dirs.iter().map(|dir| dir.join(&library_name)));
                candidates
            } else if library.components().count() > 1 {
                manifest
                    .parent()
                    .map(|parent| vec![parent.join(&library)])
                    .unwrap_or_default()
            } else {
                library_dirs.iter().map(|dir| dir.join(&library)).collect()
            };
            for candidate in candidates {
                if elf64(&candidate) {
                    let resolved = candidate.canonicalize().unwrap_or(candidate);
                    return Some((data, resolved));
                }
            }
        }
    }
    None
}

/// Configure `env` for the current machine; returns the human-readable status line.
pub fn configure(env: &mut BTreeMap<String, String>) -> io::Result<String> {
    let manifests: Vec<PathBuf> = MANIFEST_DIRS.iter().map(PathBuf::from).collect();
    let mut libraries: Vec<PathBuf> = LIBRARY_DIRS.iter().map(PathBuf::from).collect();
    configure_with(env, &manifests, &mut libraries)
}

pub fn configure_with(
    env: &mut BTreeMap<String, String>,
    manifest_dirs: &[PathBuf],
    library_dirs: &mut Vec<PathBuf>,
) -> io::Result<String> {
    // Both overrides belong to the user; VK_DRIVER_FILES takes precedence over
    // VK_ICD_FILENAMES. In particular don't replace an explicit Lavapipe setup.
    if env
        .get("VK_DRIVER_FILES")
        .map(|value| !value.is_empty())
        .unwrap_or(false)
        || env
            .get("VK_ICD_FILENAMES")
            .map(|value| !value.is_empty())
            .unwrap_or(false)
    {
        return Ok("Vulkan: using explicit driver override".into());
    }
    let mut bundled = env
        .get("BB_BUNDLED_VK_DRIVER_FILES")
        .cloned()
        .unwrap_or_default();
    if let Some(extra) = env
        .get("VK_ADD_DRIVER_FILES")
        .filter(|value| !value.is_empty())
    {
        bundled = if bundled.is_empty() {
            extra.clone()
        } else {
            format!("{extra}:{bundled}")
        };
    }
    if let Some(extra) = env
        .get("BB_NVIDIA_LIB_DIR")
        .filter(|value| !value.is_empty())
    {
        library_dirs.insert(0, PathBuf::from(extra));
    }

    let Some((data, driver)) = host_nvidia(manifest_dirs, library_dirs) else {
        if !bundled.is_empty() {
            env.insert("VK_DRIVER_FILES".into(), bundled);
        }
        return Ok("Vulkan: bundled AMD/Intel drivers; no accessible NVIDIA ICD found".into());
    };

    // The driver version/path and mtime give each installed driver its own small cache.
    let modified = fs::metadata(&driver)
        .and_then(|meta| meta.modified())
        .map(|time| {
            time.duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        })
        .unwrap_or(0);
    let mut hasher = Sha256::new();
    hasher.update(format!("{}:{}", driver.display(), modified).as_bytes());
    let key = hex(&hasher.finalize())[..16].to_string();

    let data_dir = env
        .get("BB_DATA_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env.get("XDG_DATA_HOME")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .or_else(|| dirs::home_dir().map(|home| home.join(".local/share")))
        })
        .unwrap_or_else(|| PathBuf::from(".local/share"))
        .join("bbport");
    let cache = data_dir.join("vulkan").join("nvidia").join(&key);
    let libraries = cache.join("lib");
    fs::create_dir_all(&libraries)?;

    // NVIDIA dlopens versioned shader/compiler libraries at runtime, so expose the driver's
    // siblings as well as its ICD. Never link generic host libs.
    let mut sources = vec![driver.clone()];
    if let Some(parent) = driver.parent() {
        for pattern in ["libnvidia-", "libGLX_nvidia.so", "libEGL_nvidia.so"] {
            let mut siblings: Vec<PathBuf> = fs::read_dir(parent)
                .map(|entries| {
                    entries
                        .flatten()
                        .map(|entry| entry.path())
                        .filter(|path| {
                            let name = path
                                .file_name()
                                .map(|value| value.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            name.starts_with(pattern)
                        })
                        .collect()
                })
                .unwrap_or_default();
            siblings.sort();
            sources.extend(siblings);
        }
    }
    for source in sources {
        if !elf64(&source) {
            continue;
        }
        let Some(name) = source.file_name() else {
            continue;
        };
        let link = libraries.join(name);
        let resolved = source.canonicalize().unwrap_or(source);
        #[cfg(unix)]
        {
            match std::os::unix::fs::symlink(&resolved, &link) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        #[cfg(not(unix))]
        {
            let _ = resolved;
            let _ = link;
        }
    }

    // Keep the manifest's API/architecture metadata, replace its library path.
    let mut data = data;
    if let Some(icd) = data.get_mut("ICD").and_then(Value::as_object_mut) {
        icd.insert(
            "library_path".into(),
            Value::String(
                libraries
                    .join(
                        driver
                            .file_name()
                            .map(|value| value.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                    )
                    .to_string_lossy()
                    .into_owned(),
            ),
        );
    }
    let manifest = cache.join("nvidia_icd.json");
    let temporary = cache.join(format!("nvidia_icd.{}.tmp", std::process::id()));
    fs::write(
        &temporary,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&data).unwrap_or_else(|_| json!({}).to_string())
        ),
    )?;
    fs::rename(&temporary, &manifest)?;

    env.insert(
        "VK_DRIVER_FILES".into(),
        if bundled.is_empty() {
            manifest.to_string_lossy().into_owned()
        } else {
            format!("{}:{bundled}", manifest.to_string_lossy())
        },
    );
    env.insert(
        "LD_LIBRARY_PATH".into(),
        match env.get("LD_LIBRARY_PATH").filter(|value| !value.is_empty()) {
            Some(existing) => format!("{}:{existing}", libraries.to_string_lossy()),
            None => libraries.to_string_lossy().into_owned(),
        },
    );
    Ok(format!(
        "Vulkan: host NVIDIA driver {}; bundled AMD/Intel also available",
        driver.display()
    ))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("bbport-rs-vulkan-{tag}-{stamp}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn fake_elf64(width: u8, machine: u16) -> Vec<u8> {
        // e_ident (16) + e_type (2) + e_machine (2), like a real ELF64 header.
        let mut data = vec![0u8; 20];
        data[0] = 0x7f;
        data[1] = b'E';
        data[2] = b'L';
        data[3] = b'F';
        data[4] = width;
        data[5] = 1;
        data[18..20].copy_from_slice(&machine.to_le_bytes());
        data
    }

    #[test]
    fn elf64_checks_class_and_machine() {
        let dir = temp_dir("elf");
        let good = dir.join("good.so");
        fs::write(&good, fake_elf64(2, 0x3e)).unwrap();
        assert!(elf64(&good));
        let thirty_two = dir.join("lib.so");
        fs::write(&thirty_two, fake_elf64(1, 0x3e)).unwrap();
        assert!(!elf64(&thirty_two));
        assert!(!elf64(&dir.join("missing")));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn host_nvidia_finds_an_absolute_library_in_the_library_dirs() {
        let dir = temp_dir("nvidia");
        let manifests = dir.join("icd.d");
        let libs = dir.join("lib");
        fs::create_dir_all(&manifests).unwrap();
        fs::create_dir_all(&libs).unwrap();
        fs::write(
            manifests.join("10-nvidia.json"),
            r#"{"ICD":{"library_path":"/host/lib/libnvidia-glcore.so.1"}}"#,
        )
        .unwrap();
        fs::write(libs.join("libnvidia-glcore.so.1"), fake_elf64(2, 0x3e)).unwrap();
        let found = host_nvidia(&[manifests], &[libs]).unwrap();
        assert!(found.1.ends_with("libnvidia-glcore.so.1"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn explicit_override_wins() {
        let dir = temp_dir("override");
        let mut env = BTreeMap::new();
        env.insert("VK_DRIVER_FILES".into(), "/my/driver.json".into());
        let mut libs: Vec<PathBuf> = Vec::new();
        let status = configure_with(&mut env, std::slice::from_ref(&dir), &mut libs).unwrap();
        assert!(status.contains("explicit driver override"));
        assert_eq!(
            env.get("VK_DRIVER_FILES").map(String::as_str),
            Some("/my/driver.json")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn no_nvidia_keeps_the_bundled_drivers() {
        let dir = temp_dir("bundled");
        let mut env = BTreeMap::new();
        env.insert(
            "BB_BUNDLED_VK_DRIVER_FILES".into(),
            "/app/share/vulkan/icd.d".into(),
        );
        let mut libs: Vec<PathBuf> = Vec::new();
        let status = configure_with(&mut env, std::slice::from_ref(&dir), &mut libs).unwrap();
        assert!(status.contains("no accessible NVIDIA ICD"));
        assert_eq!(
            env.get("VK_DRIVER_FILES").map(String::as_str),
            Some("/app/share/vulkan/icd.d")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn host_driver_gets_its_own_manifest_and_library_folder() {
        let dir = temp_dir("cache");
        let manifests = dir.join("icd.d");
        let libs = dir.join("lib");
        let data = dir.join("data");
        fs::create_dir_all(&manifests).unwrap();
        fs::create_dir_all(&libs).unwrap();
        fs::write(
            manifests.join("10-nvidia.json"),
            r#"{"ICD":{"library_path":"libnvidia-glcore.so.1","api_version":"1.3"}}"#,
        )
        .unwrap();
        fs::write(libs.join("libnvidia-glcore.so.1"), fake_elf64(2, 0x3e)).unwrap();
        let mut env = BTreeMap::new();
        env.insert("BB_DATA_DIR".into(), data.to_string_lossy().into_owned());
        let status = configure_with(
            &mut env,
            std::slice::from_ref(&manifests),
            &mut vec![libs.clone()],
        )
        .unwrap();
        assert!(status.contains("host NVIDIA driver"));
        let driver_files = env.get("VK_DRIVER_FILES").unwrap();
        assert!(driver_files.contains("vulkan/nvidia"));
        assert!(driver_files.ends_with("nvidia_icd.json"));
        assert!(Path::new(driver_files).is_file());
        assert!(env.get("LD_LIBRARY_PATH").unwrap().contains("/lib"));
        let _ = fs::remove_dir_all(dir);
    }
}
