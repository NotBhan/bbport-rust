//! Install the user's CUSA03173 base and 1.09 update with the bundled PKG extractor, ported
//! from `bbport_pkg_setup.py`.
//!
//! The extractor is ShadPs4Plus PKG Extractor 1.1 (GPL-2.0). Game data is staged beside the
//! chosen destination and moved into place only after both PKGs have been checked.

use crate::zipread::ZipArchive;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use thiserror::Error;

pub const TITLE_ID: &str = "CUSA03173";
pub const BASE_NAME: &str = "CUSA03173.pkg";
pub const UPDATE_NAME: &str = "CUSA03173 Update v1.09.pkg";
pub const GIB: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum PackageSetupError {
    #[error("{0}")]
    Message(String),
    #[error("Game installation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Game installation failed: {0}")]
    Zip(#[from] crate::zipread::ZipError),
}

fn message(text: impl Into<String>) -> PackageSetupError {
    PackageSetupError::Message(text.into())
}

/// The 720p/1080p 60fps update PKGs shipped beside the plain one in common dumps.
pub fn is_variant(name: &str) -> bool {
    let name = name.to_lowercase().replace(' ', "");
    ["720p", "1080p", "60fps"]
        .iter()
        .any(|word| name.contains(word))
}

/// `(base member name, update member name)` from the ZIP, following the plain-update rule.
pub fn package_members(archive: &ZipArchive) -> Result<(String, String), PackageSetupError> {
    let base_key = BASE_NAME.to_lowercase();
    let update_key = UPDATE_NAME.to_lowercase();
    let mut matches: Vec<(String, String)> = Vec::new();
    let mut others: Vec<String> = Vec::new();
    for entry in archive.entries() {
        if entry.is_dir {
            continue;
        }
        let name = Path::new(&entry.name)
            .file_name()
            .map(|value| value.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if name == base_key || name == update_key {
            if matches.iter().any(|(key, _)| *key == name) {
                return Err(message(format!("The ZIP has more than one {name}.")));
            }
            matches.push((name, entry.name.clone()));
        } else if name.ends_with(".pkg") && !is_variant(&name) {
            others.push(entry.name.clone());
        }
    }
    // A renamed plain update (a browser's "(1)", a store-style name) is fine when it is the only
    // other PKG; the extractor confirms it is an update before anything is extracted.
    if !matches.iter().any(|(key, _)| *key == update_key) && others.len() == 1 {
        matches.push((update_key.clone(), others[0].clone()));
    }
    let base = matches
        .iter()
        .find(|(key, _)| *key == base_key)
        .map(|(_, name)| name.clone());
    let update = matches
        .iter()
        .find(|(key, _)| *key == update_key)
        .map(|(_, name)| name.clone());
    match (base, update) {
        (Some(base), Some(update)) => Ok((base, update)),
        _ => Err(message(
            "The ZIP must contain CUSA03173.pkg and the plain 1.09 update PKG \
             (not the 720p or 1080p 60fps one).",
        )),
    }
}

fn check_type(extractor: &Path, package: &Path, expected: i32) -> Result<(), PackageSetupError> {
    let result = Command::new(extractor)
        .arg(package)
        .arg("--check-type")
        .status()
        .map_err(|error| {
            message(format!(
                "Could not inspect {}: {error}",
                package.file_name().unwrap_or_default().to_string_lossy()
            ))
        })?;
    if result.code() != Some(expected) {
        let kind = if expected == 101 {
            "base game"
        } else {
            "1.09 update"
        };
        return Err(message(format!(
            "{} is not a valid {kind} PKG. Check that you selected the right file.",
            package.file_name().unwrap_or_default().to_string_lossy()
        )));
    }
    Ok(())
}

fn is_type(extractor: &Path, package: &Path, expected: i32) -> bool {
    check_type(extractor, package, expected).is_ok()
}

fn extract(
    extractor: &Path,
    package: &Path,
    output: &Path,
    expected_folder: &Path,
    status: &dyn Fn(&str),
) -> Result<(), PackageSetupError> {
    let mut child = Command::new(extractor)
        .arg(package)
        .arg(output)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| message(format!("Could not start the PKG extractor: {error}")))?;
    let mut reported = -1i64;
    let mut output_tail: Vec<String> = Vec::new();
    // Drain stderr concurrently: a verbose extractor blocks forever once the pipe is full if
    // only stdout is read before wait().
    let stderr_tail = child.stderr.take().map(|stderr| {
        std::thread::spawn(move || {
            let mut tail = Vec::new();
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                tail.push(line.trim().to_string());
                if tail.len() > 8 {
                    tail.remove(0);
                }
            }
            tail
        })
    });
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            output_tail.push(line.trim().to_string());
            if output_tail.len() > 8 {
                output_tail.remove(0);
            }
            if let Some((number, total)) = parse_progress(&line) {
                let percent = number * 100 / total.max(1);
                if (percent / 10) as i64 > reported {
                    reported = (percent / 10) as i64;
                    status(&format!(
                        "Extracting {}: {percent}%",
                        expected_folder
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                    ));
                }
            }
        }
    }
    let result = child.wait()?;
    if let Some(thread) = stderr_tail {
        if let Ok(lines) = thread.join() {
            output_tail.extend(lines);
        }
    }
    if result.code() != Some(0) || !expected_folder.join("eboot.bin").is_file() {
        let detail = output_tail
            .iter()
            .rev()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" ");
        return Err(message(
            format!(
                "Could not extract {}. {detail}",
                package.file_name().unwrap_or_default().to_string_lossy()
            )
            .trim()
            .to_string(),
        ));
    }
    Ok(())
}

/// Parse `Extracting file 12 of 345` without a regex dependency.
fn parse_progress(line: &str) -> Option<(u64, u64)> {
    let marker = "Extracting file ";
    let rest = line.split_once(marker)?.1;
    let (number, rest) = rest.split_once(" of ")?;
    let number = number.trim().parse::<u64>().ok()?;
    let total = rest
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .parse::<u64>()
        .ok()?;
    Some((number, total))
}

/// Available bytes on the filesystem holding `path`, when `df` can tell us.
fn free_space(path: &Path) -> Option<u64> {
    let output = Command::new("df").arg("-Pk").arg(path).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().nth(1)?;
    let available = line.split_whitespace().nth(3)?;
    available.parse::<u64>().ok().map(|kb| kb * 1024)
}

struct Staging {
    path: PathBuf,
}

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn staging_dir(parent: &Path) -> Result<Staging, PackageSetupError> {
    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = parent.join(format!(".bbport-install-{}-{counter}", std::process::id()));
    fs::create_dir_all(&path)?;
    Ok(Staging { path })
}

fn copy_merge(source: &Path, target: &Path) -> Result<(), PackageSetupError> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let destination = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_merge(&entry.path(), &destination)?;
        } else {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

/// Install the game; returns the completed game folder. Never overwrites an existing
/// installation.
pub fn install_game(
    source: &Path,
    destination_parent: &Path,
    extractor: &Path,
    update: Option<&Path>,
    status: &dyn Fn(&str),
) -> Result<PathBuf, PackageSetupError> {
    let resolve = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let source = resolve(source);
    let destination_parent = resolve(destination_parent);
    let extractor = resolve(extractor);
    if !source.is_file() {
        return Err(message("Choose an existing game ZIP or base PKG."));
    }
    if !destination_parent.is_dir() {
        return Err(message("Choose an existing destination folder."));
    }
    if !extractor.is_file() {
        return Err(message(
            "The PKG extractor is missing from the launcher package.",
        ));
    }
    let target = destination_parent.join(TITLE_ID);
    if target.exists() {
        return Err(message(format!(
            "{} already exists. Choose another destination so your game files stay safe.",
            target.display()
        )));
    }
    let suffix = source
        .extension()
        .map(|extension| format!(".{}", extension.to_string_lossy().to_lowercase()))
        .unwrap_or_default();
    if suffix != ".zip" && suffix != ".pkg" {
        return Err(message("Choose cusa-03173.zip or CUSA03173.pkg."));
    }
    let update_path = update.map(resolve);
    if suffix == ".pkg" {
        match &update_path {
            Some(path) if path.is_file() => {}
            _ => return Err(message("Choose the plain 1.09 update PKG too.")),
        }
        if update_path.as_ref() == Some(&source) {
            return Err(message(format!(
                "{} was chosen in both steps. Choose the base game PKG first, then the plain \
                 1.09 update PKG.",
                source.file_name().unwrap_or_default().to_string_lossy()
            )));
        }
        for chosen in [&source, update_path.as_ref().expect("checked above")] {
            let name = chosen.file_name().unwrap_or_default().to_string_lossy();
            if is_variant(&name) {
                return Err(message(format!(
                    "{name} looks like the 720p or 1080p 60fps update. Choose the plain 1.09 \
                     update PKG instead."
                )));
            }
        }
    }

    let archive = if suffix == ".zip" {
        Some(ZipArchive::open(&source)?)
    } else {
        None
    };
    let (member_base, member_update) = match &archive {
        Some(archive) => {
            let (base, update) = package_members(archive)?;
            (Some(base), Some(update))
        }
        None => (None, None),
    };
    let package_bytes = match (&archive, &member_base, &member_update) {
        (Some(archive), Some(base), Some(update)) => {
            let base = archive
                .by_name(base)
                .ok_or_else(|| message("base PKG vanished"))?;
            let update = archive
                .by_name(update)
                .ok_or_else(|| message("update PKG vanished"))?;
            base.size + update.size
        }
        _ => {
            let update = update_path.as_ref().expect("a .pkg always has an update");
            fs::metadata(&source)?.len() + fs::metadata(update)?.len()
        }
    };
    // A ZIP install temporarily holds the PKGs and extracted data on the same drive.
    let needed =
        (package_bytes as f64 * if archive.is_some() { 2.2 } else { 1.2 }) as u64 + 5 * GIB;
    if let Some(free) = free_space(&destination_parent) {
        if free < needed {
            return Err(message(format!(
                "Not enough free space here. Allow about {} GB for setup.",
                needed / GIB + 1
            )));
        }
    }

    let staging = staging_dir(&destination_parent)?;
    let (base, patch) = match (&archive, member_base, member_update) {
        (Some(archive), Some(base_name), Some(update_name)) => {
            status("Unpacking the base PKG from the ZIP…");
            let base = staging.path.join(BASE_NAME);
            let entry = archive
                .by_name(&base_name)
                .ok_or_else(|| message("base PKG vanished"))?;
            let mut file = File::create(&base)?;
            archive.extract_to(entry, &mut file)?;
            file.flush()?;
            status("Unpacking the 1.09 update from the ZIP…");
            let patch = staging.path.join(UPDATE_NAME);
            let entry = archive
                .by_name(&update_name)
                .ok_or_else(|| message("update PKG vanished"))?;
            let mut file = File::create(&patch)?;
            archive.extract_to(entry, &mut file)?;
            file.flush()?;
            (base, patch)
        }
        _ => (
            source.clone(),
            update_path.clone().expect("a .pkg always has an update"),
        ),
    };

    status("Checking the base game and 1.09 update…");
    let mut base = base;
    let mut patch = patch;
    if check_type(&extractor, &base, 101).is_err() {
        // The two PKGs chosen the other way round: swap them.
        if archive.is_some() {
            return Err(check_type(&extractor, &base, 101).unwrap_err());
        }
        if !(is_type(&extractor, &base, 102) && is_type(&extractor, &patch, 101)) {
            return Err(message(format!(
                "{} is not a valid base game PKG. You chose {} as the base game and {} as the \
                 update; the base game is the large CUSA03173.pkg.",
                base.file_name().unwrap_or_default().to_string_lossy(),
                base.file_name().unwrap_or_default().to_string_lossy(),
                patch.file_name().unwrap_or_default().to_string_lossy()
            )));
        }
        std::mem::swap(&mut base, &mut patch);
    }
    if let Err(error) = check_type(&extractor, &patch, 102) {
        if archive.is_some() {
            return Err(error);
        }
        return Err(message(format!(
            "{error} You chose {} as the base game and {} as the update.",
            source.file_name().unwrap_or_default().to_string_lossy(),
            update_path
                .as_ref()
                .and_then(|path| path.file_name())
                .unwrap_or_default()
                .to_string_lossy()
        )));
    }

    let extracted = staging.path.join("extracted");
    fs::create_dir_all(&extracted)?;
    let game = extracted.join(TITLE_ID);
    status("Extracting the base game. This can take a while…");
    extract(&extractor, &base, &extracted, &game, status)?;
    let patch_folder = extracted.join(format!("{TITLE_ID}-patch"));
    status("Extracting the 1.09 update…");
    extract(&extractor, &patch, &extracted, &patch_folder, status)?;
    status("Applying the update to the game folder…");
    copy_merge(&patch_folder, &game)?;
    if !game.join("eboot.bin").is_file() {
        return Err(message("The completed game folder is missing eboot.bin."));
    }
    if target.exists() {
        return Err(message(format!(
            "{} appeared during setup. No files were replaced.",
            target.display()
        )));
    }
    fs::rename(&game, &target)?;
    status("Game files installed. Ready to play.");
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("bbport-rs-pkg-{tag}-{stamp}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for (name, contents) in entries {
            let offset = out.len() as u32;
            let crc = crc32fast::hash(contents);
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes());
            for _ in 0..4 {
                out.extend_from_slice(&0u16.to_le_bytes());
            }
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&(contents.len() as u32).to_le_bytes());
            out.extend_from_slice(&(contents.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(contents);

            central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            for _ in 0..4 {
                central.extend_from_slice(&0u16.to_le_bytes());
            }
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&(contents.len() as u32).to_le_bytes());
            central.extend_from_slice(&(contents.len() as u32).to_le_bytes());
            central.extend_from_slice(&(name.len() as u16).to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u32.to_le_bytes());
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(name.as_bytes());
        }
        let central_offset = out.len() as u32;
        let central_size = central.len() as u32;
        out.extend_from_slice(&central);
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&central_size.to_le_bytes());
        out.extend_from_slice(&central_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    #[test]
    fn variants_are_recognized_by_name() {
        assert!(is_variant("CUSA03173 Update v1.09 1080p 60fps.pkg"));
        assert!(!is_variant("CUSA03173 Update v1.09.pkg"));
    }

    #[test]
    fn package_members_accepts_a_renamed_plain_update() {
        let dir = temp_dir("members");
        let path = dir.join("game.zip");
        let zip = build_zip(&[
            ("dump/CUSA03173.pkg", b"base"),
            ("dump/CUSA03173 Update (1).pkg", b"update"),
        ]);
        fs::write(&path, zip).unwrap();
        let archive = ZipArchive::open(&path).unwrap();
        let (base, update) = package_members(&archive).unwrap();
        assert!(base.ends_with("CUSA03173.pkg"));
        assert!(update.ends_with("CUSA03173 Update (1).pkg"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn package_members_rejects_60fps_variants() {
        let dir = temp_dir("variants");
        let path = dir.join("game.zip");
        let zip = build_zip(&[
            ("CUSA03173.pkg", b"base"),
            ("CUSA03173 Update v1.09 60fps.pkg", b"update"),
        ]);
        fs::write(&path, zip).unwrap();
        let archive = ZipArchive::open(&path).unwrap();
        assert!(package_members(&archive).is_err());
        let _ = fs::remove_dir_all(dir);
    }

    /// A stand-in extractor: says which type each PKG is and creates the folders the real one
    /// would create.
    fn fake_extractor(dir: &Path) -> PathBuf {
        let path = dir.join("extractor.sh");
        fs::write(
            &path,
            "#!/bin/sh\n\
             if [ \"$2\" = \"--check-type\" ]; then\n\
               case \"$1\" in *Update*) exit 102;; *) exit 101;; esac\n\
             fi\n\
             mkdir -p \"$2/CUSA03173\" \"$2/CUSA03173-patch\"\n\
             : > \"$2/CUSA03173/eboot.bin\"\n\
             : > \"$2/CUSA03173-patch/eboot.bin\"\n\
             exit 0\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).unwrap();
        }
        path
    }

    #[cfg(unix)]
    #[test]
    fn a_noisy_extractor_does_not_block_on_stderr() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("noisy-extractor");
        let extractor = dir.join("noisy.sh");
        fs::write(
            &extractor,
            "#!/bin/sh\n\
             head -c 200000 /dev/zero >&2\n\
             mkdir -p \"$2/CUSA03173\"\n\
             : > \"$2/CUSA03173/eboot.bin\"\n\
             exit 0\n",
        )
        .unwrap();
        fs::set_permissions(&extractor, fs::Permissions::from_mode(0o755)).unwrap();
        let package = dir.join(BASE_NAME);
        fs::write(&package, b"base").unwrap();
        let output = dir.join("out");
        fs::create_dir_all(&output).unwrap();
        let game = output.join(TITLE_ID);
        extract(&extractor, &package, &output, &game, &|_| {}).unwrap();
        assert!(game.join("eboot.bin").is_file());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn installs_base_and_update_into_a_new_folder() {
        let dir = temp_dir("install");
        let destination = dir.join("dest");
        fs::create_dir_all(&destination).unwrap();
        let base = dir.join(BASE_NAME);
        let update = dir.join(UPDATE_NAME);
        fs::write(&base, b"base").unwrap();
        fs::write(&update, b"update").unwrap();
        let extractor = fake_extractor(&dir);
        let seen = std::sync::Mutex::new(Vec::new());
        let status = |text: &str| seen.lock().unwrap().push(text.to_string());
        let target = install_game(&base, &destination, &extractor, Some(&update), &status).unwrap();
        assert_eq!(target, destination.join(TITLE_ID));
        assert!(target.join("eboot.bin").is_file());
        let messages = seen.lock().unwrap();
        assert!(messages.iter().any(|text| text.contains("Ready to play")));
        // A second install refuses to overwrite.
        let error =
            install_game(&base, &destination, &extractor, Some(&update), &|_| {}).unwrap_err();
        assert!(error.to_string().contains("already exists"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn direct_pkg_install_requires_the_update() {
        let dir = temp_dir("pkg-only");
        let base = dir.join(BASE_NAME);
        fs::write(&base, b"base").unwrap();
        let extractor = fake_extractor(&dir);
        let error = install_game(&base, &dir, &extractor, None, &|_| {}).unwrap_err();
        assert!(error.to_string().contains("plain 1.09 update PKG too"));
        let _ = fs::remove_dir_all(dir);
    }
}
