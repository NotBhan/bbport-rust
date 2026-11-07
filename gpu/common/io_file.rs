//! File access, ported from `legacy/gpu/shadps4/common/io_file.h` and `io_file.cpp`.
//!
//! The C++ wraps `std::FILE*`; the port wraps `std::fs::File`. Share flags are Windows-only
//! and kept for API parity, while everything else (access modes, sizes, seek/tell, mapping)
//! follows the original.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

/// `Common::FS::FileAccessMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FileAccessMode {
    Read = 1 << 0,
    Write = 1 << 1,
    ReadWrite = 1 << 0 | 1 << 1,
    Append = 1 << 2,
    ReadAppend = 1 << 0 | 1 << 2,
    Create = 1 << 3,
}

/// `Common::FS::FileType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileType {
    #[default]
    BinaryFile,
    TextFile,
}

/// `Common::FS::FileShareFlag`; only meaningful on Windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileShareFlag {
    ShareNone,
    #[default]
    ShareReadOnly,
    ShareWriteOnly,
    ShareReadWrite,
}

/// `Common::FS::SeekOrigin`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum SeekOrigin {
    #[default]
    SetOrigin,
    CurrentPosition,
    End,
}

fn access_mode_to_str(mode: FileAccessMode) -> &'static str {
    match mode {
        FileAccessMode::Read => "rb",
        FileAccessMode::Append => "ab",
        FileAccessMode::Write | FileAccessMode::ReadWrite => "r+b",
        FileAccessMode::ReadAppend => "a+b",
        FileAccessMode::Create => "wb",
    }
}

fn open_options(mode: FileAccessMode) -> OpenOptions {
    let mut options = OpenOptions::new();
    match access_mode_to_str(mode) {
        "rb" => {
            options.read(true);
        }
        "ab" => {
            options.create(true).append(true);
        }
        "r+b" => {
            options.read(true).write(true);
        }
        "a+b" => {
            options.create(true).read(true).append(true);
        }
        "wb" => {
            options.create(true).write(true).truncate(true);
        }
        _ => unreachable!(),
    }
    options
}

/// `Common::FS::IOFile`.
#[derive(Debug)]
pub struct IoFile {
    file: Option<File>,
    file_path: PathBuf,
    file_access_mode: FileAccessMode,
    file_type: FileType,
    file_mapping: usize,
}

impl Default for IoFile {
    fn default() -> Self {
        Self {
            file: None,
            file_path: PathBuf::new(),
            file_access_mode: FileAccessMode::Read,
            file_type: FileType::BinaryFile,
            file_mapping: 0,
        }
    }
}

impl IoFile {
    /// `IOFile()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// The path constructor; opening failures leave the file closed, as in the original.
    pub fn open_at(
        path: impl AsRef<Path>,
        mode: FileAccessMode,
        file_type: FileType,
        _flag: FileShareFlag,
    ) -> Self {
        let mut file = Self::default();
        let _ = file.open(path, mode, file_type, _flag);
        file
    }

    /// `GetPath`.
    pub fn path(&self) -> &Path {
        &self.file_path
    }

    /// `GetAccessMode`.
    pub fn access_mode(&self) -> FileAccessMode {
        self.file_access_mode
    }

    /// `GetType`.
    pub fn file_type(&self) -> FileType {
        self.file_type
    }

    /// `IsOpen`.
    pub fn is_open(&self) -> bool {
        self.file.is_some()
    }

    /// `IsWriteOnly`.
    pub fn is_write_only(&self) -> bool {
        matches!(
            self.file_access_mode,
            FileAccessMode::Append | FileAccessMode::Write
        )
    }

    /// `Open`.
    pub fn open(
        &mut self,
        path: impl AsRef<Path>,
        mode: FileAccessMode,
        file_type: FileType,
        _flag: FileShareFlag,
    ) -> std::io::Result<()> {
        self.close();
        self.file_path = path.as_ref().to_path_buf();
        self.file_access_mode = mode;
        self.file_type = file_type;
        match open_options(mode).open(&self.file_path) {
            Ok(file) => {
                self.file = Some(file);
                Ok(())
            }
            Err(error) => {
                eprintln!(
                    "Failed to open the file at path={}, error_message={}",
                    self.file_path.display(),
                    error
                );
                Err(error)
            }
        }
    }

    /// `Close`.
    pub fn close(&mut self) {
        self.file = None;
        self.file_mapping = 0;
    }

    /// `Unlink`.
    pub fn unlink(&mut self) {
        if !self.is_open() {
            return;
        }
        if let Err(error) = std::fs::remove_file(&self.file_path) {
            eprintln!(
                "Failed to unlink the file at path={}, ec_message={}",
                self.file_path.display(),
                error
            );
        }
    }

    /// `GetFileMapping`; on POSIX this is the file descriptor.
    pub fn get_file_mapping(&mut self) -> usize {
        if self.file_mapping != 0 {
            return self.file_mapping;
        }
        if let Some(file) = &self.file {
            self.file_mapping = file.as_raw_fd() as usize;
        }
        self.file_mapping
    }

    /// `ReadSpan` / `ReadRaw`.
    pub fn read(&mut self, data: &mut [u8]) -> usize {
        match &mut self.file {
            None => 0,
            Some(file) => match file.read(data) {
                Ok(read) => read,
                Err(error) => {
                    eprintln!("Failed to read file, error = {}", error);
                    0
                }
            },
        }
    }

    /// `WriteSpan` / `WriteRaw`.
    pub fn write(&mut self, data: &[u8]) -> usize {
        match &mut self.file {
            None => 0,
            Some(file) => match file.write_all(data) {
                Ok(()) => {
                    let _ = file.flush();
                    data.len()
                }
                Err(error) => {
                    eprintln!("Failed to write to file, error = {}", error);
                    0
                }
            },
        }
    }

    /// `ReadString`.
    pub fn read_string(&mut self, length: usize) -> String {
        let mut buffer = vec![0u8; length];
        let read = self.read(&mut buffer);
        String::from_utf8_lossy(&buffer[..read]).into_owned()
    }

    /// `WriteString`.
    pub fn write_string(&mut self, string: &str) -> usize {
        self.write(string.as_bytes())
    }

    /// `Flush`.
    pub fn flush(&mut self) -> bool {
        match &mut self.file {
            None => false,
            Some(file) => match file.flush() {
                Ok(()) => true,
                Err(error) => {
                    eprintln!(
                        "Failed to flush the file at path={}, ec_message={}",
                        self.file_path.display(),
                        error
                    );
                    false
                }
            },
        }
    }

    /// `Commit`.
    pub fn commit(&mut self) -> bool {
        match &mut self.file {
            None => false,
            Some(file) => {
                let flushed = file.flush().is_ok();
                let synced = file.sync_all().is_ok();
                if !(flushed && synced) {
                    eprintln!(
                        "Failed to commit the file at path={}",
                        self.file_path.display()
                    );
                }
                flushed && synced
            }
        }
    }

    /// `SetSize`.
    pub fn set_size(&mut self, size: u64) -> bool {
        match &mut self.file {
            None => false,
            Some(file) => match file.set_len(size) {
                Ok(()) => true,
                Err(error) => {
                    eprintln!(
                        "Failed to resize the file at path={}, size={}, ec_message={}",
                        self.file_path.display(),
                        size,
                        error
                    );
                    false
                }
            },
        }
    }

    /// `GetSize`.
    pub fn get_size(&mut self) -> u64 {
        if !self.is_open() {
            return 0;
        }
        self.flush();
        match std::fs::metadata(&self.file_path) {
            Ok(metadata) => metadata.len(),
            Err(error) => {
                eprintln!(
                    "Failed to retrieve the file size of path={}, ec_message={}",
                    self.file_path.display(),
                    error
                );
                0
            }
        }
    }

    /// `Seek`.
    pub fn seek(&mut self, offset: i64, origin: SeekOrigin) -> bool {
        let position = match origin {
            SeekOrigin::SetOrigin => SeekFrom::Start(offset.max(0) as u64),
            SeekOrigin::CurrentPosition => SeekFrom::Current(offset),
            SeekOrigin::End => SeekFrom::End(offset),
        };
        match &mut self.file {
            None => false,
            Some(file) => match file.seek(position) {
                Ok(_) => true,
                Err(error) => {
                    eprintln!(
                        "Failed to seek the file at path={}, offset={}, origin={}, ec_message={}",
                        self.file_path.display(),
                        offset,
                        origin as u32,
                        error
                    );
                    false
                }
            },
        }
    }

    /// `Tell`.
    pub fn tell(&mut self) -> i64 {
        match &mut self.file {
            None => 0,
            Some(file) => match file.stream_position() {
                Ok(position) => position as i64,
                Err(_) => 0,
            },
        }
    }

    /// `IOFile::WriteBytes`.
    pub fn write_bytes(path: impl AsRef<Path>, data: &[u8]) -> usize {
        let mut file = Self::open_at(
            path,
            FileAccessMode::Create,
            FileType::BinaryFile,
            FileShareFlag::ShareReadOnly,
        );
        file.write(data)
    }
}

/// `Common::FS::GetDirectorySize`.
pub fn get_directory_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    let mut total = 0;
    let mut stack = vec![path.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                if let Ok(metadata) = entry.metadata() {
                    total += metadata.len();
                }
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let path = std::env::temp_dir().join(format!(
                "bbport-io-file-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn file(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn create(dir: &TempDir, name: &str, data: &[u8]) -> IoFile {
        let mut file = IoFile::open_at(
            dir.file(name),
            FileAccessMode::Create,
            FileType::BinaryFile,
            FileShareFlag::ShareReadOnly,
        );
        assert!(file.is_open());
        assert_eq!(file.write(data), data.len());
        file
    }

    #[test]
    fn create_write_and_read_round_trip() {
        let dir = TempDir::new();
        let mut file = create(&dir, "a.bin", b"hello");
        assert_eq!(file.get_size(), 5);
        assert!(file.flush());
        file.close();
        assert!(!file.is_open());

        let mut file = IoFile::open_at(
            dir.file("a.bin"),
            FileAccessMode::Read,
            FileType::BinaryFile,
            FileShareFlag::ShareReadOnly,
        );
        assert!(file.is_open());
        assert!(file.seek(0, SeekOrigin::SetOrigin));
        assert_eq!(file.tell(), 0);
        assert_eq!(file.read_string(5), "hello");
        assert_eq!(file.tell(), 5);
        file.close();
        assert!(!file.is_open());
    }

    #[test]
    fn append_adds_to_the_end() {
        let dir = TempDir::new();
        create(&dir, "append.bin", b"one");
        let mut file = IoFile::open_at(
            dir.file("append.bin"),
            FileAccessMode::Append,
            FileType::BinaryFile,
            FileShareFlag::ShareReadOnly,
        );
        assert_eq!(file.write(b"-two"), 4);
        assert_eq!(file.get_size(), 7);
        file.close();
        assert_eq!(std::fs::read(dir.file("append.bin")).unwrap(), b"one-two");
    }

    #[test]
    fn resizing_and_unlinking_work() {
        let dir = TempDir::new();
        let mut file = create(&dir, "size.bin", b"hello");
        assert!(file.set_size(2));
        assert_eq!(file.get_size(), 2);
        file.unlink();
        assert!(!dir.file("size.bin").exists());
    }

    #[test]
    fn the_mapping_is_the_descriptor() {
        let dir = TempDir::new();
        let mut file = create(&dir, "map.bin", b"x");
        assert_ne!(file.get_file_mapping(), 0);
    }

    #[test]
    fn the_directory_size_sums_regular_files() {
        let dir = TempDir::new();
        create(&dir, "one.bin", b"1234");
        std::fs::create_dir_all(dir.file("nested")).unwrap();
        IoFile::write_bytes(dir.file("nested/two.bin"), b"123456");
        assert_eq!(get_directory_size(&dir.0), 10);
        assert_eq!(get_directory_size(&dir.file("missing")), 0);
    }
}
