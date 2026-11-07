//! User paths, ported from `legacy/gpu/shadps4/common/path_util.h` and its `.cpp`. The C++
//! fills a static table once, at first use; the port builds the same tree on the same first
//! use, through a `OnceLock`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

/// `Common::FS::PathType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(usize)]
pub enum PathType {
    /// Where shadPS4 stores its data.
    UserDir,
    /// Where log files are stored.
    LogDir,
    /// Where screenshots are stored.
    ScreenshotsDir,
    /// Where shaders are stored.
    ShaderDir,
    /// Where game temp data is stored.
    TempDataDir,
    /// Where game data is stored.
    GameDataDir,
    /// Where system modules are stored.
    SysModuleDir,
    /// Where downloads/temp files are stored.
    DownloadDir,
    /// Where rdoc captures are stored.
    CapturesDir,
    /// Where cheats are stored.
    CheatsDir,
    /// Where patches are stored.
    PatchesDir,
    /// Where game metadata (e.g. trophies and menu backgrounds) is stored.
    MetaDataDir,
    /// Where custom files for trophies are stored.
    CustomTrophy,
    /// Where custom files for different games are stored.
    CustomConfigs,
    /// Where pipeline and shader cache is stored.
    CacheDir,
    /// Where dumped system fonts are stored.
    FontsDir,
    /// Where general trophy metadata is stored.
    TrophyDir,
    /// PS4 home directory.
    HomeDir,
    /// Where custom modules are stored.
    CustomModulesDir,
}

impl PathType {
    /// Every path kind, in enum order.
    pub const ALL: [PathType; 19] = [
        PathType::UserDir,
        PathType::LogDir,
        PathType::ScreenshotsDir,
        PathType::ShaderDir,
        PathType::TempDataDir,
        PathType::GameDataDir,
        PathType::SysModuleDir,
        PathType::DownloadDir,
        PathType::CapturesDir,
        PathType::CheatsDir,
        PathType::PatchesDir,
        PathType::MetaDataDir,
        PathType::CustomTrophy,
        PathType::CustomConfigs,
        PathType::CacheDir,
        PathType::FontsDir,
        PathType::TrophyDir,
        PathType::HomeDir,
        PathType::CustomModulesDir,
    ];

    pub const fn index(self) -> usize {
        self as usize
    }

    /// The subdirectory this kind maps to under the user directory, `None` for the root.
    pub const fn subdirectory(self) -> Option<&'static str> {
        match self {
            PathType::UserDir => None,
            PathType::LogDir => Some(LOG_DIR),
            PathType::ScreenshotsDir => Some(SCREENSHOTS_DIR),
            PathType::ShaderDir => Some(SHADER_DIR),
            PathType::TempDataDir => Some(TEMPDATA_DIR),
            PathType::GameDataDir => Some(GAMEDATA_DIR),
            PathType::SysModuleDir => Some(SYSMODULES_DIR),
            PathType::DownloadDir => Some(DOWNLOAD_DIR),
            PathType::CapturesDir => Some(CAPTURES_DIR),
            PathType::CheatsDir => Some(CHEATS_DIR),
            PathType::PatchesDir => Some(PATCHES_DIR),
            PathType::MetaDataDir => Some(METADATA_DIR),
            PathType::CustomTrophy => Some(CUSTOM_TROPHY),
            PathType::CustomConfigs => Some(CUSTOM_CONFIGS),
            PathType::CacheDir => Some(CACHE_DIR),
            PathType::FontsDir => Some(FONTS_DIR),
            PathType::TrophyDir => Some(TROPHY_DIR),
            PathType::HomeDir => Some(HOME_DIR),
            PathType::CustomModulesDir => Some(CUSTOM_MODULES_DIR),
        }
    }
}

/// `PORTABLE_DIR`.
pub const PORTABLE_DIR: &str = "user";

// Sub-directories contained within a user data directory
pub const LOG_DIR: &str = "log";
pub const SCREENSHOTS_DIR: &str = "screenshots";
pub const SHADER_DIR: &str = "shader";
pub const GAMEDATA_DIR: &str = "data";
pub const TEMPDATA_DIR: &str = "temp";
pub const SYSMODULES_DIR: &str = "sys_modules";
pub const DOWNLOAD_DIR: &str = "download";
pub const CAPTURES_DIR: &str = "captures";
pub const CHEATS_DIR: &str = "cheats";
pub const PATCHES_DIR: &str = "patches";
pub const METADATA_DIR: &str = "game_data";
pub const CUSTOM_TROPHY: &str = "custom_trophy";
pub const CUSTOM_CONFIGS: &str = "custom_configs";
pub const CACHE_DIR: &str = "cache";
pub const FONTS_DIR: &str = "fonts";
pub const TROPHY_DIR: &str = "trophy";
pub const HOME_DIR: &str = "home";
pub const CUSTOM_MODULES_DIR: &str = "custom_modules";

/// `LOG_FILE`.
pub const LOG_FILE: &str = "shad_log.txt";

/// The bbport addition: where the port keeps its user data.
pub const USER_DIR_ENV: &str = "BB_GPU_USER_DIR";

/// `MAX_PATH`: UTF-16 code units on Windows, UTF-8 code units elsewhere.
#[cfg(windows)]
const MAX_PATH: usize = 260;
#[cfg(not(windows))]
const MAX_PATH: usize = 1024;

/// The `Notice.txt` the C++ drops into the custom trophy directory.
const CUSTOM_TROPHY_NOTICE: &str =
    "++++++++++++++++++++++++++++++++\n+ Custom Trophy Images / Sound \
+\n++++++++++++++++++++++++++++++++\n\nYou can add custom images to the trophies.\n*We recommend \
a square resolution image, for example 200x200, 500x500, the same size as the height and \
width.\nIn this folder ('user\\custom_trophy'), add the files with the following \
names:\n\nbronze.png\nsilver.png\ngold.png\nplatinum.png\n\nYou can add a custom sound for trophy \
notifications.\n*By default, no audio is played unless it is in this folder and you are using the \
QT version.\nIn this folder ('user\\custom_trophy'), add the files with the following \
names:\n\ntrophy.mp3";

/// The table the C++'s `UserPaths` lambda produces, filled on first use.
fn user_paths() -> &'static RwLock<Vec<PathBuf>> {
    static PATHS: OnceLock<RwLock<Vec<PathBuf>>> = OnceLock::new();
    PATHS.get_or_init(|| RwLock::new(create_paths_under(&user_dir())))
}

/// The starting point: `BB_GPU_USER_DIR` when set, else the portable directory beside the
/// working directory.
fn user_dir() -> PathBuf {
    match std::env::var_os(USER_DIR_ENV) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => match std::env::current_dir() {
            Ok(cwd) => cwd.join(PORTABLE_DIR),
            Err(_) => PathBuf::from(PORTABLE_DIR),
        },
    }
}

/// The standard location for the platform, used when the portable directory is missing.
pub fn platform_user_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join("Library")
            .join("Application Support")
            .join("shadPS4")
    }
    #[cfg(target_os = "linux")]
    {
        match std::env::var_os("XDG_DATA_HOME") {
            Some(xdg) if !xdg.is_empty() => PathBuf::from(xdg).join("shadPS4"),
            _ => std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default()
                .join(".local")
                .join("share")
                .join("shadPS4"),
        }
    }
    #[cfg(windows)]
    {
        // bbport: never shadPS4's %APPDATA% folder (an installed shadPS4 owns it); this runs
        // before main, so the portable folder is created here (run.py sets BB_GPU_USER_DIR).
        PathBuf::from(PORTABLE_DIR)
    }
}

/// The whole table: the user directory, one entry per `PathType`, and the trophy notice.
pub fn create_paths_under(user_dir: &Path) -> Vec<PathBuf> {
    let mut root = user_dir.to_path_buf();
    if !root.exists() {
        // If it doesn't exist, use the standard path for the platform instead.
        // NOTE: On Windows the portable directory is created instead.
        #[cfg(windows)]
        {
            let _ = fs::create_dir_all(&root);
        }
        #[cfg(not(windows))]
        {
            root = platform_user_dir();
        }
    }

    let mut paths = vec![PathBuf::new(); PathType::ALL.len()];
    for path_type in PathType::ALL {
        let path = match path_type.subdirectory() {
            Some(subdirectory) => root.join(subdirectory),
            None => root.clone(),
        };
        // The C++ ignores a failure here; a caller that needs the directory will report it.
        let _ = fs::create_dir(&path);
        paths[path_type.index()] = path;
    }

    let notice_path = paths[PathType::CustomTrophy.index()].join("Notice.txt");
    if let Ok(mut notice) = fs::File::create(notice_path) {
        use std::io::Write;
        let _ = notice.write_all(CUSTOM_TROPHY_NOTICE.as_bytes());
    }

    paths
}

/// `Common::FS::ValidatePath`: non-empty and within the platform's path limit.
pub fn validate_path(path: &Path) -> bool {
    if path.as_os_str().is_empty() {
        return false;
    }
    #[cfg(windows)]
    let length = path.to_string_lossy().encode_utf16().count();
    #[cfg(not(windows))]
    let length = path.to_string_lossy().len();
    length < MAX_PATH
}

/// `Common::FS::PathToUTF8String`.
pub fn path_to_utf8_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// `Common::FS::GetUserPath`. The C++ hands out a reference into its static table; the port
/// returns a copy because the table sits behind a lock.
pub fn get_user_path(path_type: PathType) -> PathBuf {
    let paths = user_paths()
        .read()
        .expect("the path table is never poisoned");
    paths.get(path_type.index()).cloned().unwrap_or_default()
}

/// `Common::FS::GetUserPathString`.
pub fn get_user_path_string(path_type: PathType) -> String {
    path_to_utf8_string(&get_user_path(path_type))
}

/// `Common::FS::SetUserPath`: only a directory is accepted.
pub fn set_user_path(path_type: PathType, new_path: &Path) -> bool {
    if !new_path.is_dir() {
        return false;
    }
    let mut paths = user_paths()
        .write()
        .expect("the path table is never poisoned");
    paths[path_type.index()] = new_path.to_path_buf();
    true
}

/// `FindGameByID`'s `boot_path_for`: a game directory holds both `sce_sys/param.sfo` and
/// `eboot.bin`, and the port has no packed `.zar` support.
fn boot_path_for(root: &Path) -> Option<PathBuf> {
    if root.is_dir() {
        if !root.join("sce_sys").join("param.sfo").exists() {
            return None;
        }
        let eboot_path = root.join("eboot.bin");
        if eboot_path.exists() {
            return Some(eboot_path);
        }
        return None;
    }
    // bbport: packed (ZArchive) game images are not supported by the port.
    None
}

/// `Common::FS::FindGameByID`: the `eboot.bin` of an unpacked game, or nothing.
pub fn find_game_by_id(dir: &Path, game_id: &str, max_depth: i32) -> Option<PathBuf> {
    if max_depth < 0 {
        return None;
    }

    // Check if this is the game we're looking for.
    if dir.file_name().and_then(|name| name.to_str()) == Some(game_id) {
        if let Some(found) = boot_path_for(dir) {
            return Some(found);
        }
    }

    if let Some(found) = boot_path_for(&dir.join(game_id)) {
        return Some(found);
    }
    if let Some(found) = boot_path_for(&dir.join(format!("{game_id}.zar"))) {
        return Some(found);
    }

    // Recursively search subdirectories.
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return None,
    };
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        if let Some(found) = find_game_by_id(&entry.path(), game_id, max_depth - 1) {
            return Some(found);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unique scratch directory under the system temp directory, removed by `TempTree`.
    struct TempTree(PathBuf);

    impl TempTree {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "bbport_path_util_{}_{}_{name}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("the clock is after the epoch")
                    .as_nanos()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("the scratch directory is writable");
            TempTree(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn every_path_kind_maps_to_its_directory() {
        let tree = TempTree::new("table");
        let paths = create_paths_under(tree.path());
        assert_eq!(paths.len(), PathType::ALL.len());
        assert_eq!(paths[PathType::UserDir.index()], tree.path());
        for path_type in PathType::ALL {
            let path = &paths[path_type.index()];
            assert!(path.is_dir(), "{path_type:?} was not created");
            match path_type.subdirectory() {
                Some(subdirectory) => assert_eq!(path, &tree.path().join(subdirectory)),
                None => assert_eq!(path, &tree.path().to_path_buf()),
            }
        }
        assert_eq!(PathType::LogDir.subdirectory(), Some("log"));
        assert_eq!(PathType::MetaDataDir.subdirectory(), Some("game_data"));
        assert_eq!(
            PathType::CustomModulesDir.subdirectory(),
            Some("custom_modules")
        );
        assert_eq!(LOG_FILE, "shad_log.txt");
    }

    #[test]
    fn the_custom_trophy_notice_is_written() {
        let tree = TempTree::new("notice");
        let paths = create_paths_under(tree.path());
        let notice = paths[PathType::CustomTrophy.index()].join("Notice.txt");
        let contents = fs::read_to_string(&notice).expect("Notice.txt was written");
        assert!(contents.starts_with("++++++++++++++++++++++++++++++++"));
        assert!(contents.ends_with("trophy.mp3"));
        assert_eq!(contents.matches("bronze.png").count(), 1);
    }

    #[test]
    fn a_path_is_valid_when_it_is_not_empty_and_not_too_long() {
        assert!(!validate_path(Path::new("")));
        assert!(validate_path(Path::new("/games/Bloodborne")));
        let long = "a".repeat(MAX_PATH - 1);
        assert!(validate_path(Path::new(&long)));
        let too_long = "a".repeat(MAX_PATH);
        assert!(!validate_path(Path::new(&too_long)));
        assert_eq!(MAX_PATH, if cfg!(windows) { 260 } else { 1024 });
        assert_eq!(path_to_utf8_string(Path::new("/tmp/x")), "/tmp/x");
    }

    /// A game directory with the two files `boot_path_for` looks for.
    fn make_game(root: &Path) {
        fs::create_dir_all(root.join("sce_sys")).expect("the tree is writable");
        fs::write(root.join("sce_sys").join("param.sfo"), b"param").expect("writable");
        fs::write(root.join("eboot.bin"), b"eboot").expect("writable");
    }

    #[test]
    fn the_game_search_finds_the_eboot_and_respects_the_depth() {
        let tree = TempTree::new("search");
        make_game(&tree.path().join("Bloodborne").join("CUSA00900"));
        make_game(
            &tree
                .path()
                .join("Bloodborne")
                .join("other")
                .join("CUSA00207"),
        );
        assert_eq!(
            find_game_by_id(tree.path(), "CUSA00900", 4),
            Some(
                tree.path()
                    .join("Bloodborne")
                    .join("CUSA00900")
                    .join("eboot.bin")
            )
        );
        assert_eq!(
            find_game_by_id(tree.path(), "CUSA00900", 0),
            None,
            "the game sits two levels down"
        );
        assert_eq!(
            find_game_by_id(&tree.path().join("Bloodborne"), "CUSA00207", 2),
            Some(
                tree.path()
                    .join("Bloodborne")
                    .join("other")
                    .join("CUSA00207")
                    .join("eboot.bin")
            )
        );
        assert_eq!(find_game_by_id(tree.path(), "CUSA40400", 8), None);
        assert_eq!(find_game_by_id(tree.path(), "CUSA00900", -1), None);
        assert_eq!(
            find_game_by_id(&tree.path().join("missing"), "CUSA00900", 8),
            None
        );

        // A directory that names the game but has no eboot is not a match.
        let bare = tree.path().join("Bare").join("CUSA10000");
        fs::create_dir_all(bare.join("sce_sys")).expect("writable");
        fs::write(bare.join("sce_sys").join("param.sfo"), b"param").expect("writable");
        assert_eq!(
            find_game_by_id(tree.path(), "CUSA10000", 8),
            None,
            "the game has a param.sfo but no eboot"
        );
        fs::write(bare.join("eboot.bin"), b"eboot").expect("writable");
        assert_eq!(
            find_game_by_id(tree.path(), "CUSA10000", 8),
            Some(bare.join("eboot.bin"))
        );
    }
}
