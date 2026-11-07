//! GPU buffer memory, ported from `shadps4/video_core/buffer_cache/buffer.h`.

pub const READ_FLAGS: u32 = 0x1 | 0x10 | 0x20 | 0x40 | 0x80 | 0x20000;
pub const ALL_FLAGS: u32 = READ_FLAGS | 0x2 | 0x8;

/// `MemoryType`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MemoryType {
    #[default]
    DeviceLocal = 0,
    HostUncached = 1,
    HostCached = 2,
    Stream = 3,
    Sparse = 4,
}

/// `Buffer`; the Vulkan allocation is replaced by host-owned backing in the standalone port.
#[derive(Debug, Clone)]
pub struct Buffer {
    pub cpu_addr: u64,
    pub size_bytes: usize,
    pub mapped_data: Vec<u8>,
    pub is_coherent: bool,
    pub mem_type: MemoryType,
    pub handle: u64,
    pub bda_addr: u64,
}

impl Buffer {
    pub fn new(cpu_addr: u64, size_bytes: usize, mem_type: MemoryType) -> Self {
        Self {
            cpu_addr,
            size_bytes,
            mapped_data: vec![0; size_bytes],
            is_coherent: true,
            mem_type,
            handle: 0,
            bda_addr: 0,
        }
    }

    pub fn is_in_bounds(&self, addr: u64, size: u64) -> bool {
        addr >= self.cpu_addr && addr.saturating_add(size) <= self.cpu_addr + self.size_bytes as u64
    }

    pub fn cpu_addr(&self) -> u64 {
        self.cpu_addr
    }

    pub fn offset(&self, other_cpu_addr: u64) -> u64 {
        other_cpu_addr.wrapping_sub(self.cpu_addr)
    }

    pub fn size_bytes(&self) -> usize {
        self.size_bytes
    }

    pub fn handle(&self) -> u64 {
        self.handle
    }

    pub fn buffer_device_address(&self) -> u64 {
        assert_ne!(self.bda_addr, 0, "Can't get BDA from a non BDA buffer");
        self.bda_addr
    }

    /// Makes host writes visible to the device; host memory is coherent here.
    pub fn flush(&mut self, _offset: u64, _size: u64) {}

    /// Makes device writes visible to the host.
    pub fn invalidate(&mut self, _offset: u64, _size: u64) {}
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Watch {
    tick: u64,
    upper_bound: u64,
}

/// `StreamBuffer`.
#[derive(Debug)]
#[allow(dead_code)]
pub struct StreamBuffer {
    pub buffer: Buffer,
    non_coherent_atom_size: u64,
    offset: u64,
    mapped_size: u64,
    last_tick: u64,
    current_watches: Vec<Watch>,
    current_watch_cursor: usize,
    previous_watches: Vec<Watch>,
    wait_cursor: usize,
    wait_bound: u64,
}

impl StreamBuffer {
    pub fn new(mem_type: MemoryType, size_bytes: u64) -> Self {
        Self {
            buffer: Buffer::new(0, size_bytes as usize, mem_type),
            non_coherent_atom_size: 64,
            offset: 0,
            mapped_size: 0,
            last_tick: 0,
            current_watches: Vec::new(),
            current_watch_cursor: 0,
            previous_watches: Vec::new(),
            wait_cursor: 0,
            wait_bound: 0,
        }
    }

    /// `Map`; returns the offset of the reserved region.
    pub fn map(&mut self, size: u64, alignment: u64, allow_wait: bool) -> Option<u64> {
        let alignment = alignment.max(1);
        let size = (size + self.non_coherent_atom_size - 1) & !(self.non_coherent_atom_size - 1);
        let aligned = (self.offset + alignment - 1) & !(alignment - 1);
        if aligned + size > self.buffer.size_bytes as u64 {
            if !allow_wait || !self.wait_pending_operations(aligned + size, allow_wait) {
                return None;
            }
            self.offset = 0;
            self.current_watches.clear();
            self.current_watch_cursor = 0;
        }
        self.offset = aligned;
        self.mapped_size = size;
        Some(aligned)
    }

    /// `Commit`.
    pub fn commit(&mut self, tick: u64) {
        self.last_tick = tick;
        self.current_watches.push(Watch {
            tick,
            upper_bound: self.offset + self.mapped_size,
        });
        self.offset += self.mapped_size;
    }

    /// `Reserve`.
    pub fn reserve(
        &mut self,
        size: u64,
        alignment: u64,
        allow_wait: bool,
        tick: u64,
    ) -> Option<u64> {
        let offset = self.map(size, alignment, allow_wait)?;
        self.commit(tick);
        Some(offset)
    }

    pub fn last_tick(&self) -> u64 {
        self.last_tick
    }

    fn wait_pending_operations(&mut self, upper_bound: u64, _allow_wait: bool) -> bool {
        self.wait_bound = upper_bound;
        self.wait_cursor = self.current_watch_cursor;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffers_report_bounds_offsets_and_addresses() {
        let mut buffer = Buffer::new(0x1000, 0x100, MemoryType::DeviceLocal);
        assert!(buffer.is_in_bounds(0x1000, 0x100));
        assert!(!buffer.is_in_bounds(0x10ff, 2));
        assert_eq!(buffer.offset(0x1010), 0x10);
        assert_eq!(buffer.size_bytes(), 0x100);
        buffer.bda_addr = 0x55;
        assert_eq!(buffer.buffer_device_address(), 0x55);
        assert_eq!(ALL_FLAGS & READ_FLAGS, READ_FLAGS);
        assert_eq!(core::mem::size_of::<MemoryType>(), 1);
    }

    #[test]
    fn stream_buffers_align_reserve_and_wrap() {
        let mut stream = StreamBuffer::new(MemoryType::Stream, 256);
        let first = stream.reserve(64, 64, true, 7).expect("first reservation");
        assert_eq!(first, 0);
        let second = stream.reserve(64, 64, true, 8).expect("second reservation");
        assert_eq!(second, 64);
        assert_eq!(stream.last_tick(), 8);
        assert!(stream.reserve(4096, 64, false, 9).is_none());
    }
}
