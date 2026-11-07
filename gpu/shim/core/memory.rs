//! The MemoryManager surface used by the video core, ported from
//! `legacy/gpu/shim/core/memory.h`. The VMA-backed implementation lives in the pending
//! `runtime_memory`/`memory.cpp` port.

use super::address_space::AddressSpace;
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

pub struct MemoryManager {
    impl_: AddressSpace,
    rasterizer: *mut c_void,
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self {
            impl_: AddressSpace,
            rasterizer: std::ptr::null_mut(),
        }
    }
}

// The C++ `Core::Memory` singleton is process-global; the rasterizer pointer is only touched by
// the GPU thread, so the manager can live behind the shared mutex.
unsafe impl Send for MemoryManager {}

impl MemoryManager {
    pub fn set_rasterizer(&mut self, rasterizer: *mut c_void) {
        self.rasterizer = rasterizer;
    }

    pub fn address_space(&self) -> &AddressSpace {
        &self.impl_
    }

    /// The PS4's GPU can only handle 40-bit addresses.
    pub fn is_valid_gpu_mapping(&self, virtual_addr: u64, size: u64) -> bool {
        virtual_addr
            .checked_add(size)
            .map(|end| end < 0x100_0000_0000)
            .unwrap_or(false)
    }

    pub const fn is_valid_mapping(&self, _virtual_addr: u64, _size: u64) -> bool {
        true
    }

    pub const fn clamp_range_size(&self, _virtual_addr: u64, size: u64) -> u64 {
        size
    }

    pub fn copy_sparse_memory(&self, _source: u64, _dest: *mut u8, _size: u64) {}
    pub const fn try_write_backing(&self, _address: *mut c_void, _data: &[u8]) -> bool {
        false
    }
    pub fn invalidate_memory(&self, _address: u64, _size: u64) {}
    pub const fn rasterizer(&self) -> *mut c_void {
        self.rasterizer
    }
}

/// `Core::Memory` singleton.
pub fn memory() -> &'static Mutex<MemoryManager> {
    static MEMORY: OnceLock<Mutex<MemoryManager>> = OnceLock::new();
    MEMORY.get_or_init(|| Mutex::new(MemoryManager::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_mappings_are_limited_to_40_bits() {
        let manager = MemoryManager::default();
        assert!(manager.is_valid_gpu_mapping(0, 0x1000));
        assert!(manager.is_valid_gpu_mapping(0xFF_FFFF_E000, 0x1000));
        assert!(!manager.is_valid_gpu_mapping(0xFF_FFFF_F000, 0x1000));
        assert!(!manager.is_valid_gpu_mapping(u64::MAX, 1));
    }
}
