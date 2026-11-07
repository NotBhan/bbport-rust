//! GPU log level and writer, ported from `legacy/gpu/shim/common/logging/log.cpp`.

use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Level {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warning = 3,
    Error = 4,
    Critical = 5,
}

impl Level {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "trace" => Some(Self::Trace),
            "debug" => Some(Self::Debug),
            "info" => Some(Self::Info),
            "warning" => Some(Self::Warning),
            "error" => Some(Self::Error),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Trace => "Trace",
            Self::Debug => "Debug",
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Error => "Error",
            Self::Critical => "Critical",
        }
    }
}

fn threshold() -> Level {
    std::env::var("BB_GPU_LOG")
        .ok()
        .and_then(|value| Level::from_name(&value))
        .unwrap_or(Level::Warning)
}

pub fn should_log(level: Level) -> bool {
    level >= threshold()
}

pub fn write(log_class: &str, level: Level, file: &str, line: u32, function: &str, message: &str) {
    static MUTEX: Mutex<()> = Mutex::new(());
    let base = file.rsplit('/').next().unwrap_or(file);
    let _guard = MUTEX.lock().unwrap_or_else(|error| error.into_inner());
    eprintln!(
        "GPU [{log_class}] <{}> {base}:{line} {function}: {message}",
        level.name()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_order_and_names_match_the_cpp_table() {
        assert!(Level::Trace < Level::Warning);
        assert!(Level::Critical > Level::Error);
        assert_eq!(Level::from_name("warning"), Some(Level::Warning));
        assert_eq!(Level::from_name("nope"), None);
        assert_eq!(Level::Error.name(), "Error");
    }
}
