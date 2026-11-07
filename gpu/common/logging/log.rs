//! The emulator's logger, ported from `legacy/gpu/shadps4/common/logging/log.h` and `log.cpp`.
//!
//! spdlog's sinks, async queue and duplicate filter are replaced by direct console output and
//! the ported `LogFileSink`; class filters, levels and the flush level keep their behaviour.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use super::classes;
use super::log_file_sink::{Level, LogFileSink, UNLIMITED_SIZE};
use crate::gpu::common::thread::get_current_thread_name;

/// `Common::Log::level_string_views`.
pub const LEVEL_NAMES: [&str; 7] = [
    "Trace", "Debug", "Info", "Warning", "Error", "Critical", "Off",
];

/// The spdlog level names, as accepted by the log filter and flush level settings.
pub fn level_from_str(name: &str) -> Option<Level> {
    match name {
        "trace" | "Trace" => Some(Level::Trace),
        "debug" | "Debug" => Some(Level::Debug),
        "info" | "Info" => Some(Level::Info),
        "warning" | "warn" | "Warning" => Some(Level::Warning),
        "error" | "err" | "Error" => Some(Level::Error),
        "critical" | "Critical" => Some(Level::Critical),
        "off" | "Off" => Some(Level::Off),
        _ => None,
    }
}

/// `to_string_view`.
pub fn level_to_string_view(level: Level) -> &'static str {
    LEVEL_NAMES[level as usize]
}

struct Logger {
    sink: Option<LogFileSink>,
    log_dir: PathBuf,
    levels: HashMap<String, Level>,
    default_level: Level,
    flush_level: Level,
}

impl Logger {
    fn new() -> Self {
        Self {
            sink: None,
            log_dir: PathBuf::new(),
            levels: HashMap::new(),
            default_level: Level::Info,
            flush_level: Level::Off,
        }
    }

    fn level_for(&self, class: &str) -> Level {
        self.levels
            .get(class)
            .copied()
            .unwrap_or(self.default_level)
    }

    fn should_log(&self, class: &str, level: Level) -> bool {
        let configured = self.level_for(class);
        configured != Level::Off && level >= configured
    }
}

fn logger() -> &'static Mutex<Logger> {
    static LOGGER: OnceLock<Mutex<Logger>> = OnceLock::new();
    LOGGER.get_or_init(|| Mutex::new(Logger::new()))
}

/// `Setup`: opens the main log file at `log_dir / filename`.
pub fn setup(filename: &str, log_dir: &Path, size_limit: u64) {
    let mut state = logger().lock().unwrap_or_else(|error| error.into_inner());
    state.log_dir = log_dir.to_path_buf();
    match LogFileSink::new(&log_dir.join(filename), false, size_limit) {
        Ok(sink) => state.sink = Some(sink),
        Err(error) => {
            eprintln!(
                "Failed to open the log file at {}: {error}",
                log_dir.join(filename).display()
            );
            state.sink = None;
        }
    }
    // Every class starts at trace until the filter is applied.
    state.default_level = Level::Trace;
    state.levels.clear();
}

/// `Switch`: re-applies the filter and opens the per-game session file.
pub fn switch(
    game_filename: &str,
    log_filter: &str,
    flush_level: &str,
    size_limit: u64,
    append: bool,
) {
    update_log_levels(log_filter, true);
    update_log_flush_level(flush_level);
    let mut state = logger().lock().unwrap_or_else(|error| error.into_inner());
    let path = state.log_dir.join(game_filename);
    if let Some(sink) = &mut state.sink {
        sink.set_size_limit(size_limit);
        if let Err(error) = sink.open_session(&path, !append) {
            eprintln!("Failed to open the session log {}: {error}", path.display());
        }
    }
}

/// `Shutdown`.
pub fn shutdown() {
    let mut state = logger().lock().unwrap_or_else(|error| error.into_inner());
    state.sink = None;
    state.default_level = Level::Off;
    state.levels.clear();
}

/// `Flush`.
pub fn flush() {
    let mut state = logger().lock().unwrap_or_else(|error| error.into_inner());
    if let Some(sink) = &mut state.sink {
        let _ = sink.flush();
    }
}

/// `UpdateSinks`: the port writes synchronously, so only the flush state is kept.
pub fn update_sinks() {
    let _ = logger();
}

/// `UpdateLogLevels`; an empty or disabled filter leaves every class off.
pub fn update_log_levels(log_filter: &str, enabled: bool) {
    let mut state = logger().lock().unwrap_or_else(|error| error.into_inner());
    state.default_level = Level::Info;
    state.levels.clear();

    if enabled {
        for class_level in log_filter.split_whitespace() {
            let mut parts = class_level.split(':');
            let (Some(class), Some(level), None) = (parts.next(), parts.next(), parts.next())
            else {
                eprintln!(r#"[Config] <Error> bad log filter provided"#);
                continue;
            };
            let Some(level) = level_from_str(level) else {
                eprintln!(r#"[Config] <Error> bad log filter provided"#);
                continue;
            };
            if class.starts_with('*') {
                state.default_level = level;
            } else {
                state.levels.insert(class.to_string(), level);
            }
        }
    } else {
        state.default_level = Level::Off;
    }
}

/// `UpdateLogFlushLevel`.
pub fn update_log_flush_level(log_flush_level: &str) {
    let mut state = logger().lock().unwrap_or_else(|error| error.into_inner());
    if !log_flush_level.is_empty() {
        if let Some(level) = level_from_str(log_flush_level) {
            state.flush_level = level;
        }
    }
}

/// Whether a message of `level` in `class` would be emitted.
pub fn should_log(class: &str, level: Level) -> bool {
    logger()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .should_log(class, level)
}

/// `LOG_GENERIC`.
pub fn log(class: &str, level: Level, file: &str, line: u32, function: &str, message: &str) {
    if level == Level::Off {
        return;
    }
    let mut state = logger().lock().unwrap_or_else(|error| error.into_inner());
    if !state.should_log(class, level) {
        return;
    }

    let thread_name = get_current_thread_name();
    let basename = Path::new(file)
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    let function = if function == "operator()" {
        "lambda"
    } else {
        function
    };
    let formatted = format!(
        "[{class}] <{}> ({thread_name}) {basename}:{line} {function}: {message}\n",
        level_to_string_view(level)
    );

    let color = match level {
        Level::Trace => "\x1b[1;30m",
        Level::Debug => "\x1b[0;36m",
        Level::Info => "\x1b[0;37m",
        Level::Warning => "\x1b[1;33m",
        Level::Error => "\x1b[1;31m",
        Level::Critical => "\x1b[1;35m",
        Level::Off => "",
    };
    print!("{color}{formatted}\x1b[0m");

    let should_flush = state.flush_level != Level::Off && level >= state.flush_level;
    if let Some(sink) = &mut state.sink {
        let _ = sink.sink(level, formatted.as_bytes());
        if should_flush {
            let _ = sink.flush();
        }
    }
}

/// The classes known to the logger, from `classes.h`.
pub fn all_loggers() -> &'static [&'static str] {
    classes::ALL_LOGGERS
}

/// `UNLIMITED_SIZE` re-export for the setup callers.
pub const DEFAULT_SIZE_LIMIT: u64 = UNLIMITED_SIZE;

/// `LOG_INFO`-style macros. `trace` is compiled out in release builds, as in the original.
#[macro_export]
macro_rules! log_trace {
    ($class:expr, $($arg:tt)*) => {
        if cfg!(debug_assertions) {
            $crate::gpu::common::logging::log::log(
                $class, $crate::gpu::common::logging::log_file_sink::Level::Trace,
                file!(), line!(), module_path!(), &format!($($arg)*))
        }
    };
}

#[macro_export]
macro_rules! log_debug {
    ($class:expr, $($arg:tt)*) => {
        $crate::gpu::common::logging::log::log(
            $class, $crate::gpu::common::logging::log_file_sink::Level::Debug,
            file!(), line!(), module_path!(), &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($class:expr, $($arg:tt)*) => {
        $crate::gpu::common::logging::log::log(
            $class, $crate::gpu::common::logging::log_file_sink::Level::Info,
            file!(), line!(), module_path!(), &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warning {
    ($class:expr, $($arg:tt)*) => {
        $crate::gpu::common::logging::log::log(
            $class, $crate::gpu::common::logging::log_file_sink::Level::Warning,
            file!(), line!(), module_path!(), &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($class:expr, $($arg:tt)*) => {
        $crate::gpu::common::logging::log::log(
            $class, $crate::gpu::common::logging::log_file_sink::Level::Error,
            file!(), line!(), module_path!(), &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_critical {
    ($class:expr, $($arg:tt)*) => {
        $crate::gpu::common::logging::log::log(
            $class, $crate::gpu::common::logging::log_file_sink::Level::Critical,
            file!(), line!(), module_path!(), &format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|error| error.into_inner())
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let path = std::env::temp_dir().join(format!(
                "bbport-log-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_level_names_and_parsing_match() {
        assert_eq!(level_to_string_view(Level::Trace), "Trace");
        assert_eq!(level_to_string_view(Level::Off), "Off");
        assert_eq!(level_from_str("warning"), Some(Level::Warning));
        assert_eq!(level_from_str("critical"), Some(Level::Critical));
        assert_eq!(level_from_str("nonsense"), None);
    }

    #[test]
    fn the_filter_sets_the_default_and_per_class_levels() {
        let _guard = test_lock();
        update_log_levels("*:warning Debug:trace", true);
        assert!(should_log("Debug", Level::Trace));
        assert!(!should_log("Render", Level::Info));
        assert!(should_log("Render", Level::Warning));
        assert!(should_log("anything", Level::Error));

        update_log_levels("*:info", false);
        assert!(!should_log("Render", Level::Critical));
    }

    #[test]
    fn bad_filter_entries_are_ignored() {
        let _guard = test_lock();
        update_log_levels("*:info broken Entry", true);
        assert!(should_log("Render", Level::Error));
        assert!(!should_log("Render", Level::Debug));
    }

    #[test]
    fn setup_writes_to_the_main_and_session_files() {
        let _guard = test_lock();
        let dir = TempDir::new();
        setup("shad.log", &dir.0, 1024 * 1024);
        log_info!("Render", "main message {}", 1);
        flush();
        let main = std::fs::read_to_string(dir.0.join("shad.log")).unwrap();
        assert!(main.contains("[Render] <Info>"));
        assert!(main.contains("main message 1"));

        update_log_levels("*:info", true);
        switch("game.log", "*:info", "", 1 << 20, false);
        log_warning!("Render", "session message");
        flush();
        let session = std::fs::read_to_string(dir.0.join("game.log")).unwrap();
        assert!(
            session.contains("[Render] <Warning>"),
            "session = {session:?}"
        );
        assert!(session.contains("session message"));
        let main_after = std::fs::read_to_string(dir.0.join("shad.log")).unwrap();
        assert_eq!(main_after, main, "the session file takes over after switch");

        shutdown();
        assert!(
            !should_log("Render", Level::Error),
            "shutdown disables the loggers"
        );
    }
}
