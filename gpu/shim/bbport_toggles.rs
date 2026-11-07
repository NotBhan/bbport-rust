//! Toggleable optimizations and the frame statistics, ported from
//! `legacy/gpu/shim/bbport_toggles.h`.

use std::sync::atomic::{AtomicI32, AtomicI64, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

pub const REGION_CACHE: u64 = 1;
pub const FETCH_SHADER_CACHE: u64 = 2;
pub const PAGE_TRACKING_EARLY_EXIT: u64 = 4;
pub const PENDING_POLL_LIMIT: u64 = 8;
pub const THREADED_RECORDING: u64 = 16;
pub const IMAGE_DESC_CACHE: u64 = 32;
pub const LOCK_FREE_UPLOAD_CHECK: u64 = 64;
pub const FIND_IMAGE_CACHE: u64 = 128;
pub const DEFERRED_UPLOADS: u64 = 256;
pub const ACCESS_MEMO: u64 = 512;
pub const TEXTURE_BINDING_MEMO: u64 = 1024;
pub const COARSE_READ_TRACKING: u64 = 2048;
pub const PREPARED_RESOURCES: u64 = 4096;
pub const DRAW_PREPARATION: u64 = 8192;
pub const DEFERRED_STREAM_COPIES: u64 = 16384;
pub const HOT_PAGES: u64 = 32768;
pub const FAULT_WINDOW: u64 = 65536;
pub const PARALLEL_COPIES: u64 = 131072;
pub const ASYNC_FENCES: u64 = 262144;
pub const POOL_SMALL_COPIES: u64 = 524288;
pub const RECORD_PREFETCH: u64 = 1 << 32;
pub const TEXTURE_VIEW_MEMO: u64 = 1 << 33;
pub const TEXTURE_BIND_HELPER: u64 = 1 << 34;
pub const EARLY_DRAW_INPUTS: u64 = 1 << 35;
pub const CONSTANT_RING: u64 = 1 << 36;
pub const DRAW_PIPELINE: u64 = 1 << 37;
pub const PIPELINED_TASKS: u64 = 1 << 38;
pub const PENDING_FENCE_WAITS: u64 = 1 << 39;
pub const PIPELINED_DISPATCH: u64 = 1 << 40;
pub const RECORDER_FENCES: u64 = 1 << 41;
pub const PIPELINED_MEMORY_WRITES: u64 = 1 << 42;
pub const MULTI_COPY_SHADER: u64 = 1 << 43;
pub const RENDER_STATE_MEMO: u64 = 1 << 44;
pub const TEXTURE_SET_MEMO: u64 = 1 << 45;
pub const PIPELINED_INDIRECT_DRAWS: u64 = 1 << 46;
pub const SCENE_ATTACHMENTS_ONLY: u64 = 1 << 47;
pub const SAMPLE_SCENE_PROXIES: u64 = 1 << 48;
pub const ORDERED_GUEST_WRITES: u64 = 1 << 49;
pub const SCENE_HALF_RES: u64 = 1 << 50;
pub const UPDATE_IMAGE_FAST_PATH: u64 = 1 << 30;
pub const TAA_TONEMAP_BLEND: u64 = 1 << 51;
pub const TAA_CLIP: u64 = 1 << 52;
pub const TAA_VARIANCE: u64 = 1 << 53;
pub const TAA_FILTER: u64 = 1 << 54;
pub const TAA_KEEP_NEARER_HISTORY: u64 = 1 << 55;
pub const SCENE_MIP_BIAS: u64 = 1 << 57;

fn runtime_disabled() -> &'static AtomicU64 {
    static DISABLED: AtomicU64 = AtomicU64::new(0);
    &DISABLED
}

/// `runtime_disabled_optimizations` (written by the runtime memory code).
pub fn set_runtime_disabled(mask: u64) {
    runtime_disabled().store(mask, Ordering::Relaxed);
}

pub fn runtime_disabled_mask() -> u64 {
    runtime_disabled().load(Ordering::Relaxed)
}

/// Off whatever the toggle file says. Precise readbacks (`BB_READBACKS=2`) read-protect
/// GPU-written pages and deadlock against the recording/copy threads (bbport#2).
pub fn forced_off() -> u64 {
    static MASK: OnceLock<u64> = OnceLock::new();
    *MASK.get_or_init(|| {
        match std::env::var("BB_READBACKS")
            .ok()
            .and_then(|value| value.trim().parse::<i64>().ok())
        {
            Some(2) => THREADED_RECORDING | PARALLEL_COPIES | POOL_SMALL_COPIES,
            _ => 0,
        }
    })
}

pub fn disabled(bit: u64) -> bool {
    (runtime_disabled_mask() | forced_off()) & bit != 0
}

pub struct Stats {
    pub tracker_faults: AtomicU64,
    pub hot_pages: AtomicI64,
    pub images_registered: AtomicU64,
    pub image_upload_bytes: AtomicU64,
    pub buffer_upload_bytes: AtomicU64,
    pub gpu_thread_clock: AtomicI32,
    pub draws: AtomicU64,
    pub dispatches: AtomicU64,
    pub submissions: AtomicU64,
    pub gpu_frames: AtomicU64,
    pub t_resident: AtomicU64,
    pub t_protect: AtomicU64,
    pub t_image_create: AtomicU64,
    pub t_refresh: AtomicU64,
    pub t_staging: AtomicU64,
    pub t_host_wait: AtomicU64,
    pub t_copy: AtomicU64,
    pub copy_bytes: AtomicU64,
    pub t_read_faults: AtomicU64,
    pub read_faults: AtomicU64,
    pub t_write_faults: AtomicU64,
    pub t_copy_cpu: AtomicU64,
    pub copy_sys_us: AtomicU64,
    pub copy_minflt: AtomicU64,
    pub gpu_idle_ns: AtomicU64,
    pub reduced_draws: AtomicU64,
    pub scene_draws: AtomicU64,
    pub sync_recording_ns: AtomicU64,
    pub host_copies_wait_ns: AtomicU64,
    pub tick_wait_ns: AtomicU64,
    pub copy_threads_wait_ns: AtomicU64,
    pub host_copy_waits: AtomicU64,
    pub gpu_sys_us: AtomicU64,
    pub gpu_user_us: AtomicU64,
    pub gpu_invol_switches: AtomicU64,
    pub gpu_vol_switches: AtomicU64,
    pub gpu_minor_faults: AtomicU64,
    pub gpu_signal_faults: AtomicU64,
    pub protect_calls: AtomicU64,
    pub protect_pages: AtomicU64,
    pub protect_revoke_calls: AtomicU64,
    pub protect_revoke_pages: AtomicU64,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            tracker_faults: AtomicU64::new(0),
            hot_pages: AtomicI64::new(0),
            images_registered: AtomicU64::new(0),
            image_upload_bytes: AtomicU64::new(0),
            buffer_upload_bytes: AtomicU64::new(0),
            gpu_thread_clock: AtomicI32::new(-1),
            draws: AtomicU64::new(0),
            dispatches: AtomicU64::new(0),
            submissions: AtomicU64::new(0),
            gpu_frames: AtomicU64::new(0),
            t_resident: AtomicU64::new(0),
            t_protect: AtomicU64::new(0),
            t_image_create: AtomicU64::new(0),
            t_refresh: AtomicU64::new(0),
            t_staging: AtomicU64::new(0),
            t_host_wait: AtomicU64::new(0),
            t_copy: AtomicU64::new(0),
            copy_bytes: AtomicU64::new(0),
            t_read_faults: AtomicU64::new(0),
            read_faults: AtomicU64::new(0),
            t_write_faults: AtomicU64::new(0),
            t_copy_cpu: AtomicU64::new(0),
            copy_sys_us: AtomicU64::new(0),
            copy_minflt: AtomicU64::new(0),
            gpu_idle_ns: AtomicU64::new(0),
            reduced_draws: AtomicU64::new(0),
            scene_draws: AtomicU64::new(0),
            sync_recording_ns: AtomicU64::new(0),
            host_copies_wait_ns: AtomicU64::new(0),
            tick_wait_ns: AtomicU64::new(0),
            copy_threads_wait_ns: AtomicU64::new(0),
            host_copy_waits: AtomicU64::new(0),
            gpu_sys_us: AtomicU64::new(0),
            gpu_user_us: AtomicU64::new(0),
            gpu_invol_switches: AtomicU64::new(0),
            gpu_vol_switches: AtomicU64::new(0),
            gpu_minor_faults: AtomicU64::new(0),
            gpu_signal_faults: AtomicU64::new(0),
            protect_calls: AtomicU64::new(0),
            protect_pages: AtomicU64::new(0),
            protect_revoke_calls: AtomicU64::new(0),
            protect_revoke_pages: AtomicU64::new(0),
        }
    }
}

pub fn stats() -> &'static Stats {
    static STATS: OnceLock<Stats> = OnceLock::new();
    STATS.get_or_init(Stats::default)
}

/// Diagnostics are collected only with `BB_FRAME_STATS=1`.
pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("BB_FRAME_STATS")
            .map(|value| value.starts_with('1'))
            .unwrap_or(false)
    })
}

/// Adds its lifetime to `total` on drop when `BB_FRAME_STATS=1`.
pub struct Timer {
    total: &'static AtomicU64,
    start: Option<Instant>,
}

impl Timer {
    pub fn new(total: &'static AtomicU64) -> Self {
        Self {
            total,
            start: enabled().then(Instant::now),
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            self.total
                .fetch_add(start.elapsed().as_nanos() as u64, Ordering::Relaxed);
        }
    }
}

/// `BbStats::WaitTimer`: always measures, used for scheduler blocking time.
pub struct WaitTimer {
    total: &'static AtomicU64,
    start: Instant,
}

impl WaitTimer {
    pub fn new(total: &'static AtomicU64) -> Self {
        Self {
            total,
            start: Instant::now(),
        }
    }
}

impl Drop for WaitTimer {
    fn drop(&mut self) {
        self.total
            .fetch_add(self.start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_combines_the_runtime_mask_and_the_bit() {
        set_runtime_disabled(0);
        assert!(!disabled(REGION_CACHE));
        set_runtime_disabled(REGION_CACHE);
        assert!(disabled(REGION_CACHE));
        assert!(!disabled(IMAGE_DESC_CACHE));
        set_runtime_disabled(0);
        // Precise readbacks force the threaded paths off regardless of the toggle file.
        if std::env::var("BB_READBACKS").as_deref() == Ok("2") {
            assert!(disabled(THREADED_RECORDING));
        }
    }

    #[test]
    fn wait_timer_always_accumulates() {
        let total: &'static AtomicU64 = Box::leak(Box::new(AtomicU64::new(0)));
        {
            let _timer = WaitTimer::new(total);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(total.load(Ordering::Relaxed) > 0);
    }
}
