//! Path layout of the port, mirroring `bbport_launcher.py` (`PORT_DIR`, `DATA_DIR`,
//! `CONFIG_DIR`) and honouring the same environment overrides.

use std::env;
use std::path::{Path, PathBuf};

/// Folder name under the XDG config root, as used by both launchers.
pub const APP_DIR_NAME: &str = "bbport-launcher";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub port_dir: PathBuf,
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub update_cache: PathBuf,
    pub packaged: bool,
}

impl Paths {
    /// Discover the port layout from the running executable or the current directory.
    pub fn discover() -> Self {
        Self::for_port_dir(find_port_dir())
    }

    /// Build the layout for an explicit port directory (used by tests and `--port-dir`).
    pub fn for_port_dir(port_dir: PathBuf) -> Self {
        Self::with_config_dir(port_dir, config_root().join(APP_DIR_NAME))
    }

    /// Build the layout with an explicit config directory (used by tests).
    pub fn with_config_dir(port_dir: PathBuf, config_dir: PathBuf) -> Self {
        let data_dir = non_empty_env("BB_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| port_dir.clone());
        let config_file = config_dir.join("settings.json");
        let update_cache = config_dir.join("update-check.json");
        Self {
            port_dir,
            data_dir,
            config_dir,
            config_file,
            update_cache,
            packaged: non_empty_env("BB_PREBUILT").is_some(),
        }
    }

    /// `BB_CONFIG` overrides the INI location; otherwise it lives in the data folder.
    pub fn ini_path(&self) -> PathBuf {
        non_empty_env("BB_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|| self.data_dir.join("bbport.ini"))
    }

    /// The launcher's canonical trophy key, kept beside the settings.
    pub fn trophy_keys_path(&self) -> PathBuf {
        self.config_dir.join("keys.json")
    }

    pub fn mods_dir(&self) -> PathBuf {
        self.data_dir.join("mods")
    }

    pub fn patches_dir(&self) -> PathBuf {
        self.data_dir.join("patches")
    }

    pub fn mods_config(&self) -> PathBuf {
        self.data_dir.join("mods.json")
    }

    pub fn patches_config(&self) -> PathBuf {
        self.data_dir.join("patches.json")
    }

    /// The default game folder for a source checkout: `<repo>/CUSA03173`, one level above the
    /// port directory. Packaged builds default to no folder, as in the Python launcher.
    pub fn default_game_dir(&self) -> String {
        if self.packaged {
            String::new()
        } else {
            self.port_dir
                .parent()
                .unwrap_or(&self.port_dir)
                .join("CUSA03173")
                .to_string_lossy()
                .into_owned()
        }
    }
}

/// `XDG_CONFIG_HOME` when set and non-empty, else `~/.config`.
pub fn config_root() -> PathBuf {
    non_empty_env("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

/// Find the port root: `BB_PORT_DIR`, then walk up from the executable and the working
/// directory until a `run.sh` is found.
pub fn find_port_dir() -> PathBuf {
    if let Some(dir) = non_empty_env("BB_PORT_DIR") {
        return PathBuf::from(dir);
    }
    let mut starts = Vec::new();
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            starts.push(parent.to_path_buf());
        }
    }
    if let Ok(cwd) = env::current_dir() {
        starts.push(cwd);
    }
    for start in &starts {
        let mut dir: Option<&Path> = Some(start);
        while let Some(candidate) = dir {
            if candidate.join("run.sh").is_file() {
                return candidate.to_path_buf();
            }
            dir = candidate.parent();
        }
    }
    starts
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from("."))
}

fn non_empty_env(name: &str) -> Option<String> {
    match env::var(name) {
        Ok(value) if !value.is_empty() => Some(value),
        _ => None,
    }
}

/// Expand a configured folder like `bbport_startup.expanded`: `~`, `$VAR` / `${VAR}` and, on
/// Windows, `%VAR%`. Unknown variables are left untouched, as in Python.
pub fn expand(value: &str) -> PathBuf {
    let with_home = expand_user(value);
    PathBuf::from(expand_vars(&with_home))
}

fn expand_user(value: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        if value == "~" {
            return home.to_string_lossy().into_owned();
        }
        if let Some(rest) = value.strip_prefix("~/") {
            return home.join(rest).to_string_lossy().into_owned();
        }
    }
    value.to_string()
}

fn expand_vars(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let chars: Vec<char> = value.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let current = chars[index];
        if current == '$' {
            if let Some((name, consumed)) = variable_name(&chars[index + 1..], true) {
                if let Ok(replacement) = env::var(&name) {
                    result.push_str(&replacement);
                    index += 1 + consumed;
                    continue;
                }
            }
        } else if current == '%' && cfg!(windows) {
            if let Some((name, consumed)) = variable_name(&chars[index + 1..], false) {
                if let Ok(replacement) = env::var(&name) {
                    result.push_str(&replacement);
                    index += 2 + consumed;
                    continue;
                }
            }
        }
        result.push(current);
        index += 1;
    }
    result
}

/// Parse the variable name after `$` or `%`; returns the name and how many characters it took
/// (including the closing brace or percent sign for the braced forms).
fn variable_name(chars: &[char], dollar: bool) -> Option<(String, usize)> {
    if chars.is_empty() {
        return None;
    }
    if dollar && chars[0] == '{' {
        let mut name = String::new();
        for (offset, ch) in chars[1..].iter().enumerate() {
            if *ch == '}' {
                return (!name.is_empty()).then_some((name, offset + 2));
            }
            if !(ch.is_ascii_alphanumeric() || *ch == '_') {
                return None;
            }
            name.push(*ch);
        }
        return None;
    }
    let terminator = if dollar { None } else { Some('%') };
    let mut name = String::new();
    for (offset, ch) in chars.iter().enumerate() {
        if Some(*ch) == terminator {
            return (!name.is_empty()).then_some((name, offset + 1));
        }
        if !(ch.is_ascii_alphanumeric() || *ch == '_') {
            break;
        }
        name.push(*ch);
    }
    if dollar && !name.is_empty() {
        Some((name.clone(), name.len()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_uses_the_app_folder() {
        let paths = Paths::for_port_dir(PathBuf::from("/tmp/bbport"));
        assert!(paths.config_dir.ends_with(APP_DIR_NAME));
        assert_eq!(paths.config_file, paths.config_dir.join("settings.json"));
    }

    #[test]
    fn default_game_dir_is_the_repo_sibling() {
        let paths = Paths::for_port_dir(PathBuf::from("/repo/linux_launcher"));
        assert_eq!(paths.default_game_dir(), "/repo/CUSA03173");
    }

    #[test]
    fn expand_replaces_home_and_environment_variables() {
        std::env::set_var("BBPORT_RS_TEST_DIR", "/tmp/game");
        assert_eq!(
            expand("$BBPORT_RS_TEST_DIR/user"),
            PathBuf::from("/tmp/game/user")
        );
        assert_eq!(expand("${BBPORT_RS_TEST_DIR}"), PathBuf::from("/tmp/game"));
        let home = dirs::home_dir().unwrap();
        assert_eq!(expand("~/saves"), home.join("saves"));
    }
}
