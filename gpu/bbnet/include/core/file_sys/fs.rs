//! The network file-handle table, ported from `bbnet/include/core/file_sys/fs.h`.

use std::any::Any;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

/// `FileType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FileType {
    Regular = 0,
    Directory = 1,
    Device = 2,
    Socket = 3,
    Epoll = 4,
    Resolver = 5,
    Equeue = 6,
}

impl FileType {
    fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Regular,
            1 => Self::Directory,
            2 => Self::Device,
            3 => Self::Socket,
            4 => Self::Epoll,
            5 => Self::Resolver,
            _ => Self::Equeue,
        }
    }
}

/// `Core::FileSys::File`. The network objects are opaque until the guest net layer is ported;
/// callers downcast them to the concrete socket/epoll/resolver type.
#[derive(Default)]
pub struct File {
    is_opened: AtomicBool,
    file_type: AtomicU8,
    guest_name: Mutex<String>,
    pub socket: Option<Arc<dyn Any + Send + Sync>>,
    pub epoll: Option<Arc<dyn Any + Send + Sync>>,
    pub resolver: Option<Arc<dyn Any + Send + Sync>>,
}

impl File {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_opened(&self) -> bool {
        self.is_opened.load(Ordering::Acquire)
    }

    pub fn set_opened(&self, opened: bool) {
        self.is_opened.store(opened, Ordering::Release);
    }

    pub fn file_type(&self) -> FileType {
        FileType::from_u8(self.file_type.load(Ordering::Acquire))
    }

    pub fn set_file_type(&self, file_type: FileType) {
        self.file_type.store(file_type as u8, Ordering::Release);
    }

    pub fn guest_name(&self) -> String {
        self.guest_name
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub fn set_guest_name(&self, name: impl Into<String>) {
        *self
            .guest_name
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = name.into();
    }
}

/// `Core::FileSys::HandleTable`.
#[derive(Default)]
pub struct HandleTable {
    files: Mutex<Vec<Option<Arc<File>>>>,
}

impl HandleTable {
    /// `HandleTable()`: ids 0-2 stay unused.
    pub fn new() -> Self {
        Self {
            files: Mutex::new(vec![None, None, None]),
        }
    }

    /// `CreateHandle`.
    pub fn create_handle(&self) -> i32 {
        let mut files = self.files.lock().unwrap_or_else(|error| error.into_inner());
        let file = Arc::new(File::new());
        for (index, slot) in files.iter_mut().enumerate().skip(3) {
            if slot.is_none() {
                *slot = Some(file);
                return index as i32;
            }
        }
        files.push(Some(file));
        (files.len() - 1) as i32
    }

    /// `DeleteHandle`.
    pub fn delete_handle(&self, handle: i32) {
        let mut files = self.files.lock().unwrap_or_else(|error| error.into_inner());
        if handle >= 3 && (handle as usize) < files.len() {
            files[handle as usize] = None;
        }
    }

    /// `GetFile`.
    pub fn get_file(&self, handle: i32) -> Option<Arc<File>> {
        let files = self.files.lock().unwrap_or_else(|error| error.into_inner());
        if handle >= 0 && (handle as usize) < files.len() {
            files[handle as usize].clone()
        } else {
            None
        }
    }

    fn typed(&self, handle: i32, file_type: FileType) -> Option<Arc<File>> {
        self.get_file(handle)
            .filter(|file| file.file_type() == file_type)
    }

    /// `GetSocket`.
    pub fn get_socket(&self, handle: i32) -> Option<Arc<File>> {
        self.typed(handle, FileType::Socket)
    }

    /// `GetEpoll`.
    pub fn get_epoll(&self, handle: i32) -> Option<Arc<File>> {
        self.typed(handle, FileType::Epoll)
    }

    /// `GetResolver`.
    pub fn get_resolver(&self, handle: i32) -> Option<Arc<File>> {
        self.typed(handle, FileType::Resolver)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_start_at_three_and_are_reused() {
        let table = HandleTable::new();
        assert!(table.get_file(0).is_none());
        let first = table.create_handle();
        let second = table.create_handle();
        assert_eq!(first, 3);
        assert_eq!(second, 4);
        table.delete_handle(first);
        assert!(table.get_file(first).is_none());
        assert_eq!(table.create_handle(), first);
        table.delete_handle(1);
        assert!(table.get_file(second).is_some());
    }

    #[test]
    fn typed_lookups_filter_by_file_type() {
        let table = HandleTable::new();
        let handle = table.create_handle();
        let file = table.get_file(handle).unwrap();
        assert_eq!(file.file_type(), FileType::Regular);
        assert!(table.get_socket(handle).is_none());
        file.set_file_type(FileType::Socket);
        assert!(table.get_socket(handle).is_some());
        assert!(table.get_epoll(handle).is_none());
        assert!(table.get_resolver(handle).is_none());

        file.set_guest_name("sceNetSocket");
        assert_eq!(file.guest_name(), "sceNetSocket");
        assert!(!file.is_opened());
        file.set_opened(true);
        assert!(file.is_opened());
    }
}
