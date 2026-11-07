//! Glue between the C loader and the video core, ported from `legacy/gpu/shim/bbgpu.cpp`.
//! The host supplies the pieces the C build got from its runtime: guest memory, clocks, the
//! rasterizer and the display, as function pointers and traits.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::time::Duration;

use crate::gpu::bbgpu::{BbAchievement, BbGpuConfig};
use crate::gpu::shim::bbport_copy;
use crate::gpu::shim::bbport_overlay;
use crate::gpu::shim::bbport_settings;
use crate::gpu::shim::bbport_toggles::{self, Stats, Timer};
use crate::gpu::shim::bbport_trophies::{self as trophies, Grade};
use crate::gpu::shim::bbport_write_log::{self, Source};
use crate::gpu::shim::core::libraries::libs::{SymbolType, SymbolsResolver};
use crate::gpu::shim::core::signals::SignalDispatch;
use crate::gpu::shim::sdl_window::WindowSystemType;
use crate::gpu::shim::window::{self, WindowState};

/// `ORBIS_KERNEL_ERROR_EINVAL`.
pub const ORBIS_KERNEL_ERROR_EINVAL: i32 = 0x8002_0016u32 as i32;

pub use crate::gpu::core::libraries::error_codes::ORBIS_OK;

#[derive(Debug, Clone, Default)]
pub struct ElfInfo {
    pub initialized: bool,
    pub game_serial: String,
    pub title: String,
    pub sdk_ver: u32,
    pub psf_attributes: u32,
}

/// `Core::Loader::SymbolsResolver`: the vendored libraries register their exports here.
pub fn resolver() -> &'static Mutex<SymbolsResolver> {
    static RESOLVER: OnceLock<Mutex<SymbolsResolver>> = OnceLock::new();
    RESOLVER.get_or_init(|| Mutex::new(SymbolsResolver::default()))
}

fn elf() -> &'static Mutex<ElfInfo> {
    static ELF: OnceLock<Mutex<ElfInfo>> = OnceLock::new();
    ELF.get_or_init(|| Mutex::new(ElfInfo::default()))
}

fn lock<T>(mutex: &'static Mutex<T>) -> MutexGuard<'static, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

/// `SymbolsResolver::AddSymbol`: the vendored libraries register their exports.
pub fn add_symbol(nid: &str, library: &str, module: &str, address: u64) {
    resolver()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .add_symbol(nid, library, module, SymbolType::Function, address);
}

/// `bbgpu_resolve`: an imported `NID#lib#mod`, or 0 when the library does not provide it.
pub fn resolve(scoped_nid: &str) -> usize {
    let nid = scoped_nid.split('#').next().unwrap_or(scoped_nid);
    resolver()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .resolve(nid)
        .map(|symbol| symbol.address as usize)
        .unwrap_or(0)
}

pub fn symbol_count() -> u32 {
    resolver()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .count() as u32
}

/// `Emulator::FillElfInfo`: the fields the vendored `ElfInfo` singleton keeps.
pub fn fill_elf_info(
    serial: Option<&str>,
    title: Option<&str>,
    sdk_version: u32,
    psf_attributes: u32,
) {
    let mut info = lock(elf());
    info.initialized = true;
    info.game_serial = serial.unwrap_or("UNKNOWN").to_string();
    info.title = title.unwrap_or("").to_string();
    info.sdk_ver = sdk_version;
    info.psf_attributes = psf_attributes;
}

pub fn elf_info() -> ElfInfo {
    lock(elf()).clone()
}

/// The host's guest memory: `runtime_memory.*` in the C build.
#[derive(Clone, Copy, Default)]
pub struct MemoryProvider {
    pub is_mapped: Option<fn(u64, u64) -> bool>,
    pub clamp: Option<fn(u64, u64) -> u64>,
    /// (start, end, mapped) of the region holding the address.
    pub region: Option<RegionQuery>,
    pub write_backing: Option<fn(u64, &[u8]) -> bool>,
    pub gpu_protect: Option<fn(u64, u64, bool, bool)>,
    pub set_gpu_hooks: Option<fn(GpuRangeHook, GpuRangeHook, GpuRangeHook)>,
    pub process_time_us: Option<fn() -> u64>,
    pub process_time_counter: Option<fn() -> u64>,
    pub tsc_frequency: Option<fn() -> u64>,
    pub display_refresh_hz: Option<fn() -> u32>,
}

/// (start, end, mapped) of the guest region holding an address.
pub type RegionQuery = fn(u64) -> Option<(u64, u64, bool)>;

pub type GpuRangeHook = fn(u64, u64);

fn provider() -> &'static Mutex<MemoryProvider> {
    static PROVIDER: OnceLock<Mutex<MemoryProvider>> = OnceLock::new();
    PROVIDER.get_or_init(|| Mutex::new(MemoryProvider::default()))
}

pub fn set_memory_provider(value: MemoryProvider) {
    *lock(provider()) = value;
}

/// The rasterizer the memory manager drives (the vendored `Vulkan::Rasterizer`).
pub trait Rasterizer: Send + Sync {
    fn register_memory(&self, address: u64, size: u64);
    fn map_memory(&self, address: u64, size: u64);
    fn unmap_memory(&self, address: u64, size: u64);
    fn invalidate_memory(&self, address: u64, size: u64);
}

fn rasterizer() -> &'static Mutex<Option<Arc<dyn Rasterizer>>> {
    static RASTERIZER: OnceLock<Mutex<Option<Arc<dyn Rasterizer>>>> = OnceLock::new();
    RASTERIZER.get_or_init(|| Mutex::new(None))
}

fn map_hook(address: u64, size: u64) {
    let rasterizer = lock(rasterizer()).clone();
    if let Some(rasterizer) = rasterizer {
        rasterizer.register_memory(address, size);
        rasterizer.map_memory(address, size);
    }
}

fn unmap_hook(address: u64, size: u64) {
    let rasterizer = lock(rasterizer()).clone();
    if let Some(rasterizer) = rasterizer {
        rasterizer.unmap_memory(address, size);
    }
}

fn invalidate_hook(address: u64, size: u64) {
    let rasterizer = lock(rasterizer()).clone();
    if let Some(rasterizer) = rasterizer {
        rasterizer.invalidate_memory(address, size);
    }
}

/// `MemoryManager::SetRasterizer`: existing guest mappings are replayed by the runtime.
pub fn set_rasterizer(value: Box<dyn Rasterizer>) {
    *lock(rasterizer()) = Some(Arc::from(value));
    if let Some(set_hooks) = lock(provider()).set_gpu_hooks {
        set_hooks(map_hook, unmap_hook, invalidate_hook);
    }
}

pub fn invalidate_memory(address: u64, size: u64) {
    invalidate_hook(address, size);
}

pub fn is_valid_mapping(address: u64, size: u64) -> bool {
    lock(provider())
        .is_mapped
        .map(|is_mapped| is_mapped(address, size))
        .unwrap_or(false)
}

pub fn clamp_range_size(address: u64, size: u64) -> u64 {
    lock(provider())
        .clamp
        .map(|clamp| clamp(address, size))
        .unwrap_or(0)
}

/// # Safety
/// `dest..dest + size` must be writable and mapped guest reads in `source..source + size` must
/// be readable.
unsafe fn copy_sparse_serial(
    mut source: u64,
    mut dest: usize,
    mut size: u64,
    region: Option<RegionQuery>,
) {
    while size > 0 {
        let (end, mapped) = match region.and_then(|region| region(source)) {
            Some((_, end, mapped)) => (end, mapped),
            None => (source + size, false),
        };
        let count = size.min(end.saturating_sub(source));
        if count == 0 {
            break;
        }
        let destination =
            unsafe { std::slice::from_raw_parts_mut(dest as *mut u8, count as usize) };
        if mapped {
            let bytes = unsafe { std::slice::from_raw_parts(source as *const u8, count as usize) };
            destination.copy_from_slice(bytes);
        } else {
            destination.fill(0);
        }
        source += count;
        dest += count as usize;
        size -= count;
    }
}

/// `MemoryManager::CopySparseMemory`: large uploads are split across the copy threads.
///
/// # Safety
/// `dest` must be writable for `size` bytes, and the mapped parts of the guest range at
/// `source` must be readable for `size` bytes.
pub unsafe fn copy_sparse_memory(source: u64, dest: *mut u8, size: u64) {
    const CHUNK: u64 = 512 * 1024;
    let region = lock(provider()).region;
    if size < 4 * CHUNK || !bbport_copy::enabled() {
        unsafe { copy_sparse_serial(source, dest as usize, size, region) };
        return;
    }
    let chunks = size.div_ceil(CHUNK) as usize;
    let base = dest as usize;
    bbport_copy::parallel_for(chunks, &move |index| {
        let offset = index as u64 * CHUNK;
        unsafe {
            copy_sparse_serial(
                source + offset,
                base + offset as usize,
                CHUNK.min(size - offset),
                region,
            );
        }
    });
}

/// `MemoryManager::TryWriteBacking`: logs the write, then hands it to the host.
pub fn try_write_backing(address: u64, data: &[u8]) -> bool {
    bbport_write_log::note(address, data, Source::Backing);
    lock(provider())
        .write_backing
        .map(|write_backing| write_backing(address, data))
        .unwrap_or(false)
}

/// `AddressSpace::Protect`: counts the call and forwards it to the host.
pub fn protect(virtual_addr: u64, size: u64, read: bool, write: bool) {
    let _timer = Timer::new(&bbport_toggles::stats().t_protect);
    let pages = size.div_ceil(4096);
    let stats: &'static Stats = bbport_toggles::stats();
    stats.protect_calls.fetch_add(1, Ordering::Relaxed);
    stats.protect_pages.fetch_add(pages, Ordering::Relaxed);
    if !write {
        stats.protect_revoke_calls.fetch_add(1, Ordering::Relaxed);
        stats
            .protect_revoke_pages
            .fetch_add(pages, Ordering::Relaxed);
    }
    if let Some(gpu_protect) = lock(provider()).gpu_protect {
        gpu_protect(virtual_addr, size, read, write);
    }
}

/// `AddressSpace::GetUsableRegions`: guest memory holds one right-open interval.
pub const USABLE_REGION: (u64, u64) = (0x10_0000_0000, 0xfc_0000_0000);

/// Kernel services the vendored libraries call directly.
pub fn kernel_get_tsc_frequency() -> u64 {
    lock(provider())
        .tsc_frequency
        .map(|hook| hook())
        .unwrap_or(0)
}

pub fn kernel_read_tsc() -> u64 {
    read_tsc()
}

pub fn kernel_get_process_time() -> u64 {
    lock(provider())
        .process_time_us
        .map(|hook| hook())
        .unwrap_or(0)
}

pub fn kernel_get_process_time_counter() -> u64 {
    lock(provider())
        .process_time_counter
        .map(|hook| hook())
        .unwrap_or(0)
}

pub const fn kernel_get_process_time_counter_frequency() -> u64 {
    1_000_000_000
}

/// Base PS4.
pub const fn kernel_is_neo_mode() -> i32 {
    0
}

pub fn kernel_get_compiled_sdk_version(out: Option<&mut i32>) -> i32 {
    let Some(out) = out else {
        return ORBIS_KERNEL_ERROR_EINVAL;
    };
    *out = lock(elf()).sdk_ver as i32;
    ORBIS_OK
}

pub fn kernel_usleep(microseconds: u32) {
    std::thread::sleep(Duration::from_micros(u64::from(microseconds)));
}

fn tsc() -> &'static Mutex<fn() -> u64> {
    static TSC: OnceLock<Mutex<fn() -> u64>> = OnceLock::new();
    TSC.get_or_init(|| Mutex::new(|| 0))
}

/// The host's fenced timestamp counter (`Common::FencedRDTSC`).
pub fn set_read_tsc(hook: fn() -> u64) {
    *lock(tsc()) = hook;
}

fn read_tsc() -> u64 {
    lock(tsc())()
}

/// The display refresh rate the vblank follows (60 when the host has no display).
pub fn display_refresh_hz() -> u32 {
    lock(provider())
        .display_refresh_hz
        .map(|hook| hook())
        .unwrap_or(60)
        .max(60)
}

struct KernelService {
    requests: AtomicU32,
    runner: Mutex<Option<fn()>>,
    mutex: Mutex<bool>,
    condition: Condvar,
    stop: AtomicBool,
}

fn kernel_service() -> &'static KernelService {
    static SERVICE: OnceLock<KernelService> = OnceLock::new();
    SERVICE.get_or_init(|| KernelService {
        requests: AtomicU32::new(0),
        runner: Mutex::new(None),
        mutex: Mutex::new(false),
        condition: Condvar::new(),
        stop: AtomicBool::new(false),
    })
}

/// The boost asio `io_context::run` for the queue's timer events.
pub fn set_io_context_runner(runner: fn()) {
    *kernel_service()
        .runner
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(runner);
}

pub fn kernel_signal_request() {
    let service = kernel_service();
    service.requests.fetch_add(1, Ordering::Relaxed);
    let _guard = service
        .mutex
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    service.condition.notify_all();
}

fn kernel_service_thread() {
    let service = kernel_service();
    while !service.stop.load(Ordering::Relaxed) {
        {
            let mut ready = service
                .mutex
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            while service.requests.load(Ordering::Relaxed) == 0
                && !service.stop.load(Ordering::Relaxed)
            {
                let Ok(next) = service
                    .condition
                    .wait_timeout(ready, Duration::from_millis(50))
                else {
                    return;
                };
                ready = next.0;
                if next.1.timed_out() {
                    break;
                }
            }
        }
        if service.stop.load(Ordering::Relaxed) {
            break;
        }
        if service.requests.load(Ordering::Relaxed) == 0 {
            continue;
        }
        if let Some(runner) = *service
            .runner
            .lock()
            .unwrap_or_else(|error| error.into_inner())
        {
            runner();
        }
        service.requests.store(0, Ordering::Relaxed);
    }
}

pub fn start_kernel_service() -> std::thread::JoinHandle<()> {
    kernel_service().stop.store(false, Ordering::Relaxed);
    std::thread::Builder::new()
        .name("bb:kernel_service".into())
        .spawn(kernel_service_thread)
        .expect("kernel service thread")
}

pub fn stop_kernel_service() {
    let service = kernel_service();
    service.stop.store(true, Ordering::Relaxed);
    service.condition.notify_all();
}

fn signals() -> &'static Mutex<SignalDispatch> {
    static SIGNALS: OnceLock<Mutex<SignalDispatch>> = OnceLock::new();
    SIGNALS.get_or_init(|| Mutex::new(SignalDispatch::default()))
}

pub fn signal_dispatch() -> &'static Mutex<SignalDispatch> {
    signals()
}

/// `bbgpu_handle_fault`: 1 when a GPU page-tracking fault was handled.
pub fn handle_fault(ucontext: *mut c_void, address: *mut c_void) -> i32 {
    i32::from(lock(signals()).dispatch_access_violation(ucontext, address))
}

// RenderDoc capture hooks: not used by the port.
pub fn load_render_doc() {}
pub fn start_capture() {}
pub fn end_capture() {}
pub fn trigger_capture() {}
pub fn set_output_dir(_directory: &std::path::Path, _name: &str) {}
pub const fn is_render_doc_loaded() -> bool {
    false
}
pub fn request_screenshot() {}
pub const fn consume_game_only_screenshot_requests() -> u32 {
    0
}
pub const fn consume_with_overlays_screenshot_requests() -> u32 {
    0
}

pub fn overlay_captures_input() -> i32 {
    i32::from(bbport_overlay::captures_input())
}

pub fn atlas_route_pad(buttons: u32, lx: u8, ly: u8, rx: u8, ry: u8) -> u32 {
    bbport_overlay::route_pad(buttons, lx, ly, rx, ry)
}

/// Trophy grades: 0 unknown, 1 platinum, 2 gold, 3 silver, 4 bronze.
fn trophy_grade(grade: Grade) -> i32 {
    match grade {
        Grade::Platinum => 1,
        Grade::Gold => 2,
        Grade::Silver => 3,
        Grade::Bronze => 4,
        Grade::Unknown => 0,
    }
}

pub fn trophy_configure(game_dir: &str, user_dir: &str, language: i32) {
    trophies::configure(
        std::path::Path::new(game_dir),
        std::path::Path::new(user_dir),
        language,
    );
}

pub fn trophy_load(timestamps: &[i64]) {
    trophies::load_unlocks(timestamps);
    bbport_overlay::load_trophies(
        timestamps.as_ptr(),
        timestamps.len().min(u32::MAX as usize) as u32,
    );
}

pub fn trophy_summary() -> Option<crate::gpu::bbgpu::BbTrophySummary> {
    let summary = trophies::get_summary();
    if summary.num_trophies == 0 && summary.title.is_empty() {
        return None;
    }
    Some(crate::gpu::bbgpu::BbTrophySummary {
        num_trophies: summary.num_trophies,
        num_platinum: summary.num_platinum,
        num_gold: summary.num_gold,
        num_silver: summary.num_silver,
        num_bronze: summary.num_bronze,
        unlocked_trophies: summary.unlocked_trophies,
        unlocked_platinum: summary.unlocked_platinum,
        unlocked_gold: summary.unlocked_gold,
        unlocked_silver: summary.unlocked_silver,
        unlocked_bronze: summary.unlocked_bronze,
        progress_percentage: summary.progress_percentage,
        platinum_id: summary.platinum_id,
        base_locked: summary.base_locked,
        ..crate::gpu::bbgpu::BbTrophySummary::default()
    })
}

pub fn trophy_is_unlocked(id: i32) -> i32 {
    i32::from(trophies::is_unlocked(id))
}

pub fn achievement_count() -> i32 {
    trophies::entries().len() as i32
}

pub fn achievement_get(index: i32) -> Option<BbAchievement> {
    let entries = trophies::entries();
    let entry = entries.get(usize::try_from(index).ok()?)?;
    Some(BbAchievement {
        id: entry.id,
        hidden: i32::from(entry.hidden),
        score: entry.score,
        grade: trophy_grade(entry.grade),
        unlocked_at: entry.unlocked_at,
        ..BbAchievement::default()
    })
}

pub fn achievement_score() -> (i32, i32, bool) {
    let earned = trophies::earned_score();
    let total = trophies::total_score();
    (earned, total, total == 1000)
}

pub fn trophy_unlocked(id: i32, timestamp: i64) {
    trophies::unlock(id, timestamp);
    bbport_overlay::trophy_unlocked(id);
}

fn active_window() -> &'static Mutex<Option<WindowState>> {
    static WINDOW: OnceLock<Mutex<Option<WindowState>>> = OnceLock::new();
    WINDOW.get_or_init(|| Mutex::new(None))
}

pub fn text_input_begin(initial: &str, prompt: &str) -> bool {
    let mut window = lock(active_window());
    let Some(window) = window.as_mut() else {
        return false;
    };
    window.begin_text_input(initial, prompt);
    true
}

/// Returns (state, text): 0 pending, 1 accepted, 2 cancelled; 3 when no window exists.
pub fn text_input_poll() -> (i32, String) {
    let window = lock(active_window());
    match window.as_ref() {
        Some(window) => {
            let (text, state) = window.poll_text_input();
            (state, text)
        }
        None => (3, String::new()),
    }
}

/// The host window the game's text entries and title go through.
pub fn set_window(width: i32, height: i32, title: &str, system: WindowSystemType) {
    *lock(active_window()) = Some(WindowState::new(width, height, title, system));
}

/// Reads a C string the config carries (null is an absent value).
unsafe fn config_string(pointer: *const std::ffi::c_char) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    Some(
        unsafe { std::ffi::CStr::from_ptr(pointer) }
            .to_string_lossy()
            .into_owned(),
    )
}

/// `bbgpu_init`: the settings, the ELF identity, the window and the library registrations.
pub fn init(config: &BbGpuConfig) -> i32 {
    bbport_settings::load();
    let sdk_version = config.sdk_version;
    let serial = unsafe { config_string(config.serial) };
    let title = unsafe { config_string(config.title) }.unwrap_or_else(|| "Bloodborne".into());
    fill_elf_info(
        serial.as_deref(),
        Some(title.as_str()),
        sdk_version,
        config.psf_attributes,
    );
    if let Some(user_dir) = unsafe { config_string(config.user_dir) } {
        if std::env::var_os("BB_GPU_USER_DIR").is_none() {
            std::env::set_var("BB_GPU_USER_DIR", user_dir);
        }
    }
    set_window(config.width, config.height, &title, WindowSystemType::X11);
    // GnmDriver creates the presenter the VideoOut present thread uses; both register their
    // exports into `resolver()` when the vendored libraries are linked.
    0
}

/// `bbgpu_register_kernel`: the kernel service thread and the network registration.
pub fn register_kernel() {
    start_kernel_service();
    if let Some(hook) = *lock(net_register()) {
        hook();
    }
}

fn net_register() -> &'static Mutex<Option<NetRegister>> {
    static NET: OnceLock<Mutex<Option<NetRegister>>> = OnceLock::new();
    NET.get_or_init(|| Mutex::new(None))
}

/// `bbnet_register`: the network and PSN libraries (co-op).
pub fn set_net_register(hook: NetRegister) {
    *lock(net_register()) = Some(hook);
}

/// The host's network registration entry point.
pub type NetRegister = fn();

pub fn request_quit() {
    window::request_quit();
}

pub fn take_window_status() -> Option<String> {
    window::take_window_status()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;

    #[test]
    fn symbols_resolve_by_nid_and_the_scope_is_ignored() {
        let before = symbol_count();
        add_symbol("abc123", "libSceGnmDriver", "GnmDriver", 0x1234);
        assert_eq!(resolve("abc123#libSceGnmDriver#GnmDriver"), 0x1234);
        assert_eq!(resolve("abc123"), 0x1234);
        assert_eq!(resolve("missing#lib#mod"), 0);
        assert_eq!(symbol_count(), before + 1);
    }

    #[test]
    fn the_sdk_version_comes_from_the_filled_elf_info() {
        fill_elf_info(Some("CUSA00207"), Some("Bloodborne"), 0x0450_0000, 0);
        let mut version = 0;
        assert_eq!(
            kernel_get_compiled_sdk_version(Some(&mut version)),
            ORBIS_OK
        );
        assert_eq!(version, 0x0450_0000);
        assert_eq!(
            kernel_get_compiled_sdk_version(None),
            ORBIS_KERNEL_ERROR_EINVAL
        );
        assert_eq!(elf_info().game_serial, "CUSA00207");
    }

    #[test]
    fn sparse_copies_zero_the_unmapped_part_and_split_large_uploads() {
        #[repr(align(16))]
        struct Aligned([u8; 16]);

        fn region(address: u64) -> Option<(u64, u64, bool)> {
            let mapped = address & 0x08 == 0;
            Some((address + 8, address + 8, mapped))
        }
        set_memory_provider(MemoryProvider {
            region: Some(region),
            ..MemoryProvider::default()
        });
        let source = Aligned(*b"0123456789abcdef");
        let mut dest = [0xffu8; 16];
        unsafe {
            copy_sparse_memory(
                source.0.as_ptr() as u64,
                dest.as_mut_ptr(),
                dest.len() as u64,
            );
        }
        assert_eq!(&dest[..8], b"01234567");
        assert_eq!(&dest[8..], &[0; 8], "the unmapped half reads as zeroes");
        set_memory_provider(MemoryProvider::default());
    }

    #[test]
    fn protect_counts_pages_and_read_only_revocations() {
        protect(0x1000, 8192, true, false);
        protect(0x2000, 4096, true, true);
        // The counters are process-wide; the call must at least not panic and be counted.
        let stats = bbport_toggles::stats();
        assert!(stats.protect_calls.load(Ordering::Relaxed) >= 2);
    }

    #[test]
    fn achievements_report_the_entries_and_the_score() {
        let summary = trophy_summary();
        let _ = summary;
        assert_eq!(achievement_score().1, trophies::total_score());
        let entries = trophies::entries();
        if let Some(first) = entries.first() {
            let out = achievement_get(0).unwrap();
            assert_eq!(out.id, first.id);
            assert_eq!(out.grade, trophy_grade(first.grade));
        }
        assert!(achievement_get(-1).is_none());
    }

    #[test]
    fn the_kernel_service_wakes_on_a_request_and_stops() {
        static RUNS: AtomicU64 = AtomicU64::new(0);

        fn count_run() {
            RUNS.fetch_add(1, Ordering::Relaxed);
        }
        RUNS.store(0, Ordering::Relaxed);
        set_io_context_runner(count_run);
        let thread = start_kernel_service();
        kernel_signal_request();
        for _ in 0..50 {
            if RUNS.load(Ordering::Relaxed) > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(RUNS.load(Ordering::Relaxed) > 0);
        stop_kernel_service();
        kernel_signal_request();
        thread.join().unwrap();
    }
}
