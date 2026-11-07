//! Buffer cache state, ported from
//! `shadps4/video_core/buffer_cache/buffer_cache.h`.

use crate::gpu::video_core::buffer_cache::buffer::{Buffer, MemoryType, StreamBuffer};
use crate::gpu::video_core::buffer_cache::fault_manager::FaultManager;

pub const ADDRESS_SPACE_BITS: u64 = 40;
pub const MAX_ARENA_PAGE_BITS: u64 = 32;
pub const MIN_ARENA_PAGE_BITS: u64 = 30;
pub const MAX_ARENA_PAGES: u64 = 1 << (ADDRESS_SPACE_BITS - MIN_ARENA_PAGE_BITS);
pub const MIN_BLOCK_SIZE: u64 = 16 * 1024;
pub const STREAM_THRESHOLD: u64 = 16 * 1024;

/// `BufferCache`.
#[derive(Debug)]
pub struct BufferCache {
    pub gds_buffer: Buffer,
    pub bda_pagetable_buffer: Option<Buffer>,
    pub fault_manager: FaultManager,
    pub stream_buffer: StreamBuffer,
    pub block_shift: u32,
    pub invalidations: u64,
    pub reads: u64,
}

impl BufferCache {
    pub fn new(block_shift: u32) -> Self {
        let mut fault_manager = FaultManager::new(0, 0, block_shift, 0);
        fault_manager.fault_buffer = Buffer::new(0, 64, MemoryType::HostCached);
        Self {
            gds_buffer: Buffer::new(0, 0x1000, MemoryType::DeviceLocal),
            bda_pagetable_buffer: None,
            fault_manager,
            stream_buffer: StreamBuffer::new(MemoryType::Stream, 1 << 20),
            block_shift,
            invalidations: 0,
            reads: 0,
        }
    }

    pub fn gds_buffer(&self) -> &Buffer {
        &self.gds_buffer
    }

    pub fn bda_page_table_buffer(&mut self) -> Option<&mut Buffer> {
        self.bda_pagetable_buffer.as_mut()
    }

    pub fn fault_buffer(&self) -> &Buffer {
        self.fault_manager.fault_buffer()
    }

    pub fn stream_buffer(&mut self) -> &mut StreamBuffer {
        &mut self.stream_buffer
    }

    pub fn sparse_page_shift(&self) -> u32 {
        self.block_shift
    }

    pub fn invalidate_memory(&mut self, _address: u64, _size: u64, _assume_locks: bool) {
        self.invalidations += 1;
    }

    pub fn extend_write_fault(&mut self, _address: u64) {}

    pub fn read_memory(&mut self, _address: u64, _size: u64, _is_write: bool, _assume_locks: bool) {
        self.reads += 1;
    }
}

pub trait BufferLookup {
    fn obtain_buffer(&mut self, address: u64, size: u32, written: bool) -> (u64, u64);
    fn obtain_buffer_for_image(&mut self, address: u64, size: u32) -> (u64, u64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_cache_exposes_its_streams_and_counters() {
        let mut cache = BufferCache::new(16);
        assert_eq!(cache.sparse_page_shift(), 16);
        assert_eq!(cache.gds_buffer().size_bytes(), 0x1000);
        assert!(cache.bda_page_table_buffer().is_none());
        cache.invalidate_memory(0x1000, 0x100, false);
        cache.read_memory(0x1000, 0x100, false, false);
        assert_eq!((cache.invalidations, cache.reads), (1, 1));
        assert_eq!(STREAM_THRESHOLD, 16 * 1024);
        assert_eq!(MIN_BLOCK_SIZE, 16 * 1024);
        assert_eq!(MAX_ARENA_PAGES, 1 << 10);
    }
}

impl BufferCache {
    /// `ObtainBuffer`; a host-backed allocation for a guest range.
    pub fn obtain_buffer(
        &mut self,
        address: u64,
        size: u32,
        written: bool,
    ) -> Option<(Buffer, u64)> {
        let _ = written;
        if size == 0 {
            return None;
        }
        Some((
            Buffer::new(address, size as usize, MemoryType::DeviceLocal),
            0,
        ))
    }

    /// `ObtainBufferForImage`.
    pub fn obtain_buffer_for_image(&mut self, address: u64, size: u32) -> Option<(Buffer, u64)> {
        self.obtain_buffer(address, size, false)
    }

    /// `SubmitPendingArenaBinds`/`SynchronizeDmaBuffers` have no host work in the standalone port.
    pub fn synchronize_dma_buffers(&mut self) {}

    pub fn submit_pending_arena_binds(&mut self) {}
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn buffer_cache_obtains_host_buffers() {
        let mut cache = BufferCache::new(12);
        assert!(cache.obtain_buffer(0x1000, 0, false).is_none());
        let (buffer, offset) = cache.obtain_buffer(0x1000, 0x40, true).unwrap();
        assert_eq!(offset, 0);
        assert_eq!(buffer.size_bytes(), 0x40);
        assert!(cache.obtain_buffer_for_image(0x1000, 0x40).is_some());
        cache.synchronize_dma_buffers();
    }
}
