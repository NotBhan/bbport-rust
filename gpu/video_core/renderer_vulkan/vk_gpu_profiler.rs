//! GPU pass profiler, ported from
//! `shadps4/video_core/renderer_vulkan/vk_gpu_profiler.h`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub const NUM_SLICES: usize = 4;
pub const SLICE_QUERIES: u32 = 4096;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Total {
    pub ms: f64,
    pub segments: u64,
}

/// `GpuProfiler`; timestamps are labels the renderer stamps, kept per frame slice.
#[derive(Debug, Default)]
pub struct GpuProfiler {
    pub keys: [Vec<u64>; NUM_SLICES],
    pub used: [u32; NUM_SLICES],
    pub pending: [bool; NUM_SLICES],
    pub described: HashMap<u64, String>,
    pub totals: HashMap<u64, Total>,
    pub frames: u64,
    pub current: u64,
    pub slice: usize,
    pub scheduler_id: usize,
}

fn profiler() -> &'static Mutex<Option<GpuProfiler>> {
    static PROFILER: OnceLock<Mutex<Option<GpuProfiler>>> = OnceLock::new();
    PROFILER.get_or_init(|| Mutex::new(None))
}

impl GpuProfiler {
    /// `Get`; the profiler exists only while `BB_GPU_PROFILE=1`.
    pub fn get() -> bool {
        std::env::var("BB_GPU_PROFILE")
            .map(|value| value == "1")
            .unwrap_or(false)
    }

    /// `Init`; installs the process-wide profiler.
    pub fn init(scheduler_id: usize) {
        *profiler().lock().unwrap_or_else(|error| error.into_inner()) = Some(Self {
            scheduler_id,
            ..Self::default()
        });
    }

    pub fn mark(key: u64, label: &str) {
        let mut guard = profiler().lock().unwrap_or_else(|error| error.into_inner());
        if let Some(profiler) = guard.as_mut() {
            profiler
                .described
                .entry(key)
                .or_insert_with(|| label.to_string());
            profiler.keys[profiler.slice].push(key);
            profiler.used[profiler.slice] += 1;
            profiler.current = key;
        }
    }

    pub fn current() -> u64 {
        profiler()
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .map_or(0, |profiler| profiler.current)
    }

    pub fn resume(key: u64) {
        Self::mark(key, "");
    }

    pub fn records(scheduler_id: usize) -> bool {
        profiler()
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .is_some_and(|profiler| profiler.scheduler_id == scheduler_id)
    }

    /// `BeginFrame`; advances the slice and accounts the previous frame's segments.
    pub fn begin_frame() {
        let mut guard = profiler().lock().unwrap_or_else(|error| error.into_inner());
        if let Some(profiler) = guard.as_mut() {
            let slice = profiler.slice;
            for key in &profiler.keys[slice] {
                let total = profiler.totals.entry(*key).or_default();
                total.segments += 1;
            }
            profiler.frames += 1;
            profiler.keys[slice].clear();
            profiler.used[slice] = 0;
            profiler.pending[slice] = false;
            profiler.slice = (slice + 1) % NUM_SLICES;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_are_labelled_once_and_frames_accumulate() {
        GpuProfiler::init(1);
        assert!(GpuProfiler::records(1));
        assert!(!GpuProfiler::records(2));
        GpuProfiler::mark(7, "pass");
        GpuProfiler::mark(7, "ignored");
        assert_eq!(GpuProfiler::current(), 7);
        GpuProfiler::begin_frame();
        let guard = profiler().lock().unwrap();
        let profiler = guard.as_ref().unwrap();
        assert_eq!(profiler.described.get(&7).map(String::as_str), Some("pass"));
        assert_eq!(profiler.totals.get(&7).unwrap().segments, 2);
        assert_eq!(profiler.frames, 1);
        assert_eq!(NUM_SLICES, 4);
        assert_eq!(SLICE_QUERIES, 4096);
    }
}
