//! Staging buffer pool, ported from
//! `shadps4/video_core/renderer_vulkan/vk_staging_buffer_pool.h`.

use crate::gpu::video_core::buffer_cache::buffer::{Buffer, MemoryType};

pub const NUM_TYPES: usize = 3;

/// `StagingBufferRef`.
#[derive(Debug, Clone, Copy, Default)]
pub struct StagingBufferRef {
    pub buffer: usize,
    pub offset: u64,
    pub size: u64,
    pub mapped: u64,
    pub kind: MemoryType,
    pub deferred: bool,
}

impl StagingBufferRef {
    pub fn flush(&self, buffers: &mut [Buffer]) {
        buffers[self.buffer].flush(self.offset, self.size);
    }

    pub fn invalidate(&self, buffers: &mut [Buffer]) {
        buffers[self.buffer].invalidate(self.offset, self.size);
    }
}

#[derive(Debug, Default)]
struct Ring {
    blocks: Vec<(usize, u64)>,
    current: usize,
}

#[derive(Debug, Default)]
#[allow(dead_code)]
struct LargeBuffer {
    buffer: usize,
    tick: u64,
    last_used_frame: u64,
    held: bool,
}

/// `StagingBufferPool`.
#[derive(Debug, Default)]
pub struct StagingBufferPool {
    pub buffers: Vec<Buffer>,
    rings: Vec<Ring>,
    large_caches: Vec<Vec<LargeBuffer>>,
    deferred: Vec<StagingBufferRef>,
    pub frame: u64,
    pub keep_blocks: u64,
}

impl StagingBufferPool {
    pub fn new() -> Self {
        Self {
            rings: (0..NUM_TYPES).map(|_| Ring::default()).collect(),
            large_caches: (0..NUM_TYPES).map(|_| Vec::new()).collect(),
            ..Self::default()
        }
    }

    /// `Request`.
    pub fn request(
        &mut self,
        size: u64,
        kind: MemoryType,
        alignment: u64,
        deferred: bool,
    ) -> StagingBufferRef {
        let alignment = alignment.max(1);
        let index = kind as usize % NUM_TYPES;
        if let Some(&(buffer, block_offset)) =
            self.rings[index].blocks.get(self.rings[index].current)
        {
            let buffer_ref = &mut self.buffers[buffer];
            let offset = (block_offset + alignment - 1) & !(alignment - 1);
            if offset + size <= buffer_ref.size_bytes as u64 {
                return StagingBufferRef {
                    buffer,
                    offset,
                    size,
                    mapped: offset,
                    kind,
                    deferred,
                };
            }
        }
        let buffer = self.buffers.len();
        self.buffers
            .push(Buffer::new(0, size.max(4096) as usize, kind));
        self.rings[index].blocks.push((buffer, 0));
        self.rings[index].current = self.rings[index].blocks.len() - 1;
        StagingBufferRef {
            buffer,
            offset: 0,
            size,
            mapped: 0,
            kind,
            deferred,
        }
    }

    /// `FreeDeferred`.
    pub fn free_deferred(&mut self, reference: StagingBufferRef) {
        if reference.deferred {
            self.deferred.push(reference);
        }
    }

    /// `TickFrame`.
    pub fn tick_frame(&mut self) {
        self.frame += 1;
        self.deferred.clear();
        for cache in &mut self.large_caches {
            cache.retain(|entry| entry.held);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_allocate_blocks_and_deferred_frees_are_tracked() {
        let mut pool = StagingBufferPool::new();
        let first = pool.request(128, MemoryType::HostCached, 0, false);
        assert_eq!(first.offset, 0);
        assert_eq!(first.size, 128);
        let second = pool.request(64, MemoryType::HostCached, 64, false);
        assert_eq!(second.offset, 0);
        pool.free_deferred(StagingBufferRef {
            deferred: true,
            ..first
        });
        assert_eq!(pool.deferred.len(), 1);
        pool.tick_frame();
        assert!(pool.deferred.is_empty());
        assert_eq!(pool.frame, 1);
        first.flush(&mut pool.buffers);
        assert_eq!(NUM_TYPES, 3);
    }
}
