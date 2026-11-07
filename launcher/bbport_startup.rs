//! Start-up routing, ported from `bbport_startup.py`: first-run setup, direct launch and the
//! overrides that open the launcher.

use crate::fsutil;
use crate::paths::expand;
use crate::readiness::saves_dir;
use crate::Settings;
use std::path::{Path, PathBuf};

/// Bump when the wizard changes enough that everyone should see it again.
pub const SETUP_VERSION: i64 = 1;

pub const PROBLEM_GAME: &str = "game";
pub const PROBLEM_SAVES: &str = "saves";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteMode {
    Direct,
    Launcher,
}

impl RouteMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Launcher => "launcher",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub mode: RouteMode,
    pub page: String,
    pub step: i64,
    pub problem: String,
}

impl Route {
    fn launcher(page: &str, step: i64, problem: &str) -> Self {
        Self {
            mode: RouteMode::Launcher,
            page: page.to_string(),
            step,
            problem: problem.to_string(),
        }
    }

    fn direct() -> Self {
        Self {
            mode: RouteMode::Direct,
            page: "Home".into(),
            step: 0,
            problem: String::new(),
        }
    }
}

pub fn game_ok(settings: &Settings) -> bool {
    let configured = settings.get_str("game_dir");
    let folder = if configured.is_empty() {
        PathBuf::from(".")
    } else {
        expand(&configured)
    };
    folder.join("eboot.bin").is_file()
}

/// A folder the game can write saves to; a missing one counts when its nearest existing parent
/// is writable, because the launcher creates it on first launch.
pub fn writable(folder: &Path) -> bool {
    let mut candidate = folder.to_path_buf();
    while !candidate.exists() {
        match candidate.parent() {
            Some(parent) if parent != candidate => candidate = parent.to_path_buf(),
            _ => return false,
        }
    }
    fsutil::writable(&candidate)
}

/// An explicit saves folder, else `<game folder>/user`, else the data folder while no game
/// folder is configured.
pub fn saves_folder(settings: &Settings, data_dir: &Path) -> PathBuf {
    saves_dir(settings, data_dir)
}

pub fn setup_pending(settings: &Settings) -> bool {
    let done = settings.is_truthy("setupCompleted") || settings.is_truthy("setupSkipped");
    let current = settings.get_i64("setupVersion") >= SETUP_VERSION;
    !done || !current
}

/// `{'mode': 'direct' | 'launcher', 'page', 'step' (wizard), 'problem'}`.
pub fn choose_route(
    settings: &Settings,
    args: &[String],
    keys_held: bool,
    game_valid: bool,
    saves_valid: bool,
) -> Route {
    let has = |flag: &str| args.iter().any(|arg| arg == flag);
    if has("--setup") || setup_pending(settings) {
        return Route::launcher("Setup", 0, "");
    }
    if !game_valid {
        return Route::launcher("Game", 1, PROBLEM_GAME);
    }
    if !saves_valid {
        return Route::launcher("Game", 2, PROBLEM_SAVES);
    }
    if keys_held || has("--launcher") || !settings.is_truthy_default_true("launchDirectly") {
        return Route::launcher("Home", 0, "");
    }
    Route::direct()
}

pub fn route(settings: &Settings, args: &[String], data_dir: &Path, keys_held: bool) -> Route {
    choose_route(
        settings,
        args,
        keys_held,
        game_ok(settings),
        writable(&saves_folder(settings, data_dir)),
    )
}

/// `launchDirectly` defaults to true, unlike the other truthiness checks.
trait DefaultTrue {
    fn is_truthy_default_true(&self, key: &str) -> bool;
}

impl DefaultTrue for Settings {
    fn is_truthy_default_true(&self, key: &str) -> bool {
        match self.get(key) {
            None => true,
            Some(_) => self.is_truthy(key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("bbport-rs-startup-{tag}-{stamp}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn settings_for(dir: &Path, game: Option<&Path>) -> crate::Settings {
        let paths = crate::Paths::with_config_dir(dir.to_path_buf(), dir.join("config"));
        let mut settings = crate::Settings::new(&paths);
        settings
            .set(
                "game_dir",
                Value::String(
                    game.map(|path| path.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                ),
            )
            .unwrap();
        settings.set_raw("setupCompleted", Value::Bool(true));
        settings.set_raw("setupVersion", Value::Number(SETUP_VERSION.into()));
        settings
    }

    #[test]
    fn setup_is_pending_until_completed_and_current() {
        let dir = temp_dir("setup");
        let paths = crate::Paths::with_config_dir(dir.clone(), dir.join("config"));
        let mut settings = crate::Settings::new(&paths);
        assert!(setup_pending(&settings));
        settings.set_raw("setupCompleted", Value::Bool(true));
        settings.set_raw("setupVersion", Value::Number(SETUP_VERSION.into()));
        assert!(!setup_pending(&settings));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_game_routes_to_the_game_page() {
        let dir = temp_dir("route-game");
        let settings = settings_for(&dir, None);
        let route = choose_route(&settings, &[], false, false, true);
        assert_eq!(route.mode, RouteMode::Launcher);
        assert_eq!(route.page, "Game");
        assert_eq!(route.problem, PROBLEM_GAME);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_valid_game_with_writable_saves_launches_directly() {
        let dir = temp_dir("route-direct");
        let game = dir.join("CUSA03173");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("eboot.bin"), b"x").unwrap();
        let settings = settings_for(&dir, Some(&game));
        let route = choose_route(&settings, &[], false, true, true);
        assert_eq!(route.mode, RouteMode::Direct);
        assert_eq!(route.page, "Home");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn explicit_launcher_flag_forces_the_launcher() {
        let dir = temp_dir("route-launcher");
        let settings = settings_for(&dir, None);
        let args = vec!["--launcher".to_string()];
        let route = choose_route(&settings, &args, false, true, true);
        assert_eq!(route.mode, RouteMode::Launcher);
        assert_eq!(route.page, "Home");
        let _ = std::fs::remove_dir_all(dir);
    }
}
