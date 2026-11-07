//! `BB_WRITE_LOG` guest-write ring, ported from `legacy/gpu/shim/bbport_write_log.h` and
//! `bbport_write_log.cpp`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum Source {
    #[default]
    Backing = 0,
    WriteData = 1,
    Fence = 2,
    FenceIntent = 3,
    WriteDataIntent = 4,
    EosIntent = 5,
    DmaData = 6,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Entry {
    pub address: u64,
    pub size: u64,
    pub first: u64,
    pub tsc: u64,
    pub source: u32,
    pub tid: u32,
}

const RING_SIZE: usize = 1 << 20;
const HITS_SIZE: usize = 256;
/// The suspicious qword: `0x0000005300000000`.
const PATTERN: u64 = 0x0000_0053_0000_0000;

fn ring() -> &'static Mutex<Vec<Entry>> {
    static RING: OnceLock<Mutex<Vec<Entry>>> = OnceLock::new();
    RING.get_or_init(|| Mutex::new(vec![Entry::default(); RING_SIZE]))
}

fn hits() -> &'static Mutex<Vec<Entry>> {
    static HITS: OnceLock<Mutex<Vec<Entry>>> = OnceLock::new();
    HITS.get_or_init(|| Mutex::new(Vec::with_capacity(HITS_SIZE)))
}

fn head() -> &'static AtomicU64 {
    static HEAD: AtomicU64 = AtomicU64::new(0);
    &HEAD
}

fn hits_head() -> &'static AtomicU64 {
    static HEAD: AtomicU64 = AtomicU64::new(0);
    &HEAD
}

/// `__rdtsc` stand-in on a monotonic clock (nanoseconds since the first record).
fn tsc() -> u64 {
    static EPOCH: OnceLock<std::time::Instant> = OnceLock::new();
    EPOCH
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_nanos() as u64
}

fn thread_id() -> u32 {
    thread_local! {
        static TID: u32 = {
            static NEXT: AtomicU64 = AtomicU64::new(1);
            NEXT.fetch_add(1, Ordering::Relaxed) as u32
        };
    }
    TID.with(|tid| *tid)
}

/// `BbWriteLog::Mode`: `BB_WRITE_LOG` (0 = off, 1 = on, 2 = intents too).
pub fn mode() -> i32 {
    static MODE: OnceLock<i32> = OnceLock::new();
    *MODE.get_or_init(|| {
        std::env::var("BB_WRITE_LOG")
            .ok()
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or(0)
    })
}

pub fn enabled() -> bool {
    mode() == 1
}

/// Records a write at the write itself (changes the timing of those writes).
pub fn note(address: u64, data: &[u8], source: Source) {
    if enabled() {
        record(address, data, source);
    }
}

/// Where the command thread decodes a fence/WriteData, off the write path.
pub fn note_intent(address: u64, data: &[u8], source: Source) {
    if mode() == 2 {
        record(address, data, source);
    }
}

/// The implementation behind `Note`/`NoteIntent`.
pub fn record(address: u64, data: &[u8], source: Source) {
    let size = data.len() as u64;
    let mut entry = Entry {
        address,
        size,
        first: 0,
        tsc: tsc(),
        source: source as u32,
        tid: thread_id(),
    };
    let first_bytes = &data[..data.len().min(8)];
    let mut first = [0u8; 8];
    first[..first_bytes.len()].copy_from_slice(first_bytes);
    entry.first = u64::from_ne_bytes(first);
    push(entry);
    // Only small writes are scanned: scanning downloads of megabytes delays them enough to hide
    // the race this log is for.
    if size > 64 {
        return;
    }
    let mut at = ((8 - (address & 7)) & 7) as usize;
    while at + 8 <= data.len() {
        let value = u64::from_ne_bytes(data[at..at + 8].try_into().unwrap_or([0; 8]));
        if value == PATTERN {
            let mut hit = entry;
            hit.address = address + at as u64;
            hit.first = value;
            push_hit(hit);
        }
        at += 8;
    }
}

fn push(entry: Entry) {
    let index = head().fetch_add(1, Ordering::Relaxed) as usize % RING_SIZE;
    if let Ok(mut ring) = ring().lock() {
        ring[index] = entry;
    }
}

fn push_hit(entry: Entry) {
    let index = hits_head().fetch_add(1, Ordering::Relaxed) as usize % HITS_SIZE;
    if let Ok(mut hits) = hits().lock() {
        if index < hits.len() {
            hits[index] = entry;
        } else {
            if hits.len() == HITS_SIZE {
                hits.remove(0);
            }
            hits.push(entry);
        }
    }
}

/// The pattern writes seen so far, oldest first (for the dump and tests).
pub fn hit_entries() -> Vec<Entry> {
    hits().lock().map(|hits| hits.clone()).unwrap_or_default()
}

/// The most recent ring entries, oldest first.
pub fn recent_entries(limit: usize) -> Vec<Entry> {
    let count = head().load(Ordering::Relaxed).min(RING_SIZE as u64) as usize;
    let start = count.saturating_sub(limit);
    ring()
        .lock()
        .map(|ring| (start..count).map(|index| ring[index]).collect())
        .unwrap_or_default()
}

/// The loader's dump entry point (`bbgpu_dump_guest_writes`). The C++ version also prints the
/// faulting registers from the signal context; the port prints the recorded writes.
///
/// # Safety
/// `_ucontext` is the loader's signal context pointer.
pub unsafe extern "C" fn bbgpu_dump_guest_writes(_ucontext: *mut std::ffi::c_void) {
    if mode() == 0 {
        return;
    }
    let sources = [
        "backing",
        "WriteData",
        "fence",
        "EOP (decoded)",
        "WriteData (decoded)",
        "EOS (decoded)",
        "DmaData",
    ];
    for entry in hit_entries() {
        let source = sources.get(entry.source as usize).unwrap_or(&"?");
        eprintln!(
            "Write log: pattern {source} {:#x} +{} first {:#x} tid {}",
            entry.address, entry.size, entry.first, entry.tid
        );
    }
    let count = head().load(Ordering::Relaxed);
    eprintln!("Write log: {count} writes logged");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_write_containing_the_pattern_is_recorded_as_a_hit() {
        // Byte 4 of the little-endian qword carries the 0x53.
        let data = [0u8, 0, 0, 0, 0x53, 0, 0, 0];
        let before = hit_entries().len();
        record(0x1000_0000, &data, Source::WriteData);
        assert!(hit_entries().len() > before);
        let hit = hit_entries().last().copied().unwrap();
        assert_eq!(hit.first, PATTERN);
        assert_eq!(hit.size, 8);
    }

    #[test]
    fn an_aligned_write_records_the_first_qword_and_recent_entries() {
        record(0x2000_0000, &[1, 2, 3, 4, 5, 6, 7, 8], Source::Fence);
        let recent = recent_entries(64);
        let entry = recent
            .iter()
            .find(|entry| entry.address == 0x2000_0000)
            .expect("the write is in the recent entries");
        assert_eq!(entry.first, u64::from_ne_bytes([1, 2, 3, 4, 5, 6, 7, 8]));
        assert_eq!(entry.source, Source::Fence as u32);
    }

    #[test]
    fn source_values_match_the_header() {
        assert_eq!(Source::Backing as u32, 0);
        assert_eq!(Source::EosIntent as u32, 5);
        assert_eq!(Source::DmaData as u32, 6);
    }
}
