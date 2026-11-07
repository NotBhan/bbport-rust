//! The file table part the network library uses, ported from
//! `legacy/gpu/shim/core/file_sys/fs.h`. Only sockets, epolls and resolvers live here.

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum FileType {
    #[default]
    Regular,
    Directory,
    Device,
    Socket,
    Epoll,
    Resolver,
    Equeue,
}

#[derive(Debug, Clone, Default)]
pub struct File {
    pub is_opened: bool,
    pub file_type: FileType,
    pub guest_name: String,
    pub socket: Option<Arc<()>>,
    pub epoll: Option<Arc<()>>,
    pub resolver: Option<Arc<()>>,
}

#[derive(Default)]
pub struct HandleTable {
    files: Mutex<Vec<Option<File>>>,
}

impl HandleTable {
    pub fn new() -> Self {
        Self {
            files: Mutex::new(vec![None, None, None]),
        }
    }

    pub fn create_handle(&self) -> i32 {
        let mut files = self.files.lock().unwrap_or_else(|error| error.into_inner());
        for (index, slot) in files.iter_mut().enumerate().skip(3) {
            if slot.is_none() {
                *slot = Some(File::default());
                return index as i32;
            }
        }
        files.push(Some(File::default()));
        (files.len() - 1) as i32
    }

    pub fn delete_handle(&self, descriptor: i32) {
        let mut files = self.files.lock().unwrap_or_else(|error| error.into_inner());
        if descriptor >= 3 {
            if let Some(slot) = files.get_mut(descriptor as usize) {
                *slot = None;
            }
        }
    }

    pub fn get_file(&self, descriptor: i32) -> Option<File> {
        let files = self.files.lock().unwrap_or_else(|error| error.into_inner());
        descriptor
            .try_into()
            .ok()
            .and_then(|index: usize| files.get(index).and_then(Clone::clone))
    }

    pub fn get_typed(&self, descriptor: i32, file_type: FileType) -> Option<File> {
        self.get_file(descriptor)
            .filter(|file| file.file_type == file_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_start_at_three_and_can_be_reused() {
        let table = HandleTable::new();
        let first = table.create_handle();
        assert_eq!(first, 3);
        assert!(table.get_file(first).is_some());
        table.delete_handle(first);
        assert!(table.get_file(first).is_none());
        assert_eq!(table.create_handle(), 3);
        assert!(table.get_typed(first, FileType::Socket).is_none());
    }
}
