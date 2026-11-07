//! A two-target log sink, ported from `legacy/gpu/shadps4/common/logging/log_file_sink.h`.
//!
//! The C++ builds on spdlog's `base_sink`; the port keeps the target selection and the size
//! limit and takes formatted lines instead of spdlog messages.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// `Common::Log::UNLIMITED_SIZE`.
pub const UNLIMITED_SIZE: u64 = 0;

/// The spdlog levels used by the sink's critical bypass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Level {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warning = 3,
    Error = 4,
    Critical = 5,
    Off = 6,
}

/// `Common::Log::LogFileSink`.
#[derive(Debug)]
pub struct LogFileSink {
    main_file: File,
    session_file: Option<File>,
    size_limit: u64,
    current_size: u64,
}

impl LogFileSink {
    /// `LogFileSink(main_filename, truncate, size_limit)`.
    pub fn new(main_filename: &Path, truncate: bool, size_limit: u64) -> io::Result<Self> {
        let main_file = open(main_filename, truncate)?;
        Ok(Self {
            main_file,
            session_file: None,
            size_limit,
            current_size: 0,
        })
    }

    /// Opens the session target. Until this is called, everything goes to the main file.
    pub fn open_session(&mut self, session_filename: &Path, truncate: bool) -> io::Result<()> {
        self.session_file = Some(open(session_filename, truncate)?);
        Ok(())
    }

    /// Whether a session target is configured.
    pub fn has_session(&self) -> bool {
        self.session_file.is_some()
    }

    /// Updates the session size limit, as `Switch` does with the emulator setting.
    pub fn set_size_limit(&mut self, size_limit: u64) {
        self.size_limit = size_limit;
    }

    /// `sink_it_`.
    pub fn sink(&mut self, level: Level, formatted: &[u8]) -> io::Result<()> {
        match &mut self.session_file {
            None => {
                self.main_file.write_all(formatted)?;
            }
            Some(session_file) => {
                if self.current_size < self.size_limit || level == Level::Critical {
                    session_file.write_all(formatted)?;
                    self.current_size += formatted.len() as u64;
                }
            }
        }
        Ok(())
    }

    /// `flush_`.
    pub fn flush(&mut self) -> io::Result<()> {
        self.main_file.flush()?;
        if let Some(session_file) = &mut self.session_file {
            session_file.flush()?;
        }
        Ok(())
    }

    /// The number of bytes written to the session target.
    pub fn current_size(&self) -> u64 {
        self.current_size
    }
}

fn open(path: &Path, truncate: bool) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(truncate)
        .open(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new() -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let path = std::env::temp_dir().join(format!(
                "bbport-log-sink-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn file(&self, name: &str) -> std::path::PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn without_a_session_everything_goes_to_the_main_file() {
        let dir = TempDir::new();
        let main = dir.file("main.log");
        let mut sink = LogFileSink::new(&main, true, UNLIMITED_SIZE).unwrap();
        assert!(!sink.has_session());
        sink.sink(Level::Info, b"first\n").unwrap();
        sink.flush().unwrap();
        assert_eq!(std::fs::read(&main).unwrap(), b"first\n");
    }

    #[test]
    fn a_session_target_enforces_the_size_limit_except_for_critical() {
        let dir = TempDir::new();
        let mut sink = LogFileSink::new(&dir.file("main.log"), true, 6).unwrap();
        sink.open_session(&dir.file("session.log"), true).unwrap();
        sink.sink(Level::Info, b"12345\n").unwrap();
        assert_eq!(sink.current_size(), 6);
        sink.sink(Level::Info, b"ignored\n").unwrap();
        assert_eq!(sink.current_size(), 6);
        sink.sink(Level::Critical, b"crit\n").unwrap();
        assert_eq!(sink.current_size(), 11);
        sink.flush().unwrap();
        assert_eq!(
            std::fs::read(dir.file("session.log")).unwrap(),
            b"12345\ncrit\n"
        );
        assert_eq!(std::fs::read(dir.file("main.log")).unwrap(), b"");
    }
}
