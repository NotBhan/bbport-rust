//! C ABI of the GPU library, ported from `legacy/gpu/bbgpu.h`.
//!
//! These are the declarations the loader and the GPU library share. They are kept in Rust so
//! the boundary types and signatures are checked by the compiler; the implementations move
//! from `legacy/gpu` into the Rust modules behind them one at a time.

use std::ffi::{c_char, c_int};

pub type BbFrameCallback = Option<unsafe extern "C" fn()>;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BbGpuConfig {
    /// Window title.
    pub title: *const c_char,
    /// CUSA id, names the pipeline cache.
    pub serial: *const c_char,
    /// Pipeline cache/logs directory.
    pub user_dir: *const c_char,
    /// From the eboot's procparam.
    pub sdk_version: u32,
    /// `param.sfo` ATTRIBUTE.
    pub psf_attributes: u32,
    /// Initial window size.
    pub width: i32,
    pub height: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BbTrophySummary {
    pub num_trophies: i32,
    pub num_platinum: i32,
    pub num_gold: i32,
    pub num_silver: i32,
    pub num_bronze: i32,
    pub unlocked_trophies: i32,
    pub unlocked_platinum: i32,
    pub unlocked_gold: i32,
    pub unlocked_silver: i32,
    pub unlocked_bronze: i32,
    /// -1 without metadata.
    pub progress_percentage: i32,
    /// -1 without metadata.
    pub platinum_id: i32,
    /// Locked non-platinum trophies.
    pub base_locked: i32,
    pub title: [c_char; 128],
    pub description: [c_char; 1024],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BbAchievement {
    pub id: i32,
    /// A secret one: shown once earned.
    pub hidden: i32,
    /// Gamerscore, -1 when the grade is not known.
    pub score: i32,
    /// 0 unknown, 1 platinum, 2 gold, 3 silver, 4 bronze.
    pub grade: i32,
    /// Unix seconds, 0 while locked.
    pub unlocked_at: i64,
    /// UTF-8.
    pub name: [c_char; 128],
    pub description: [c_char; 256],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BbImeTextInput {
    pub initial_utf8: *const c_char,
    pub prompt_utf8: *const c_char,
}

extern "C" {
    /// Registers kernel event queues (needed with or without graphics).
    pub fn bbgpu_register_kernel();

    /// Co-op: the game folder and `param.sfo` identity for the network and PSN libraries.
    pub fn bbnet_configure(
        app0: *const c_char,
        serial: *const c_char,
        title: *const c_char,
        app_ver: *const c_char,
    );
    /// The game's code before the patches (The Hunter's Dream backend), then the online session.
    pub fn bbnet_set_image(image: *mut u8, size: u64);
    pub fn bbnet_initialize();

    /// Creates window, Vulkan device, presenter and GPU command processor.
    pub fn bbgpu_init(config: *const BbGpuConfig) -> c_int;
    /// Function for an imported NID (`NID#lib#mod`), or 0 when the GPU library does not
    /// provide it.
    pub fn bbgpu_resolve(scoped_nid: *const c_char) -> usize;
    /// Called first by the loader's SIGSEGV handler: 1 when a GPU page-tracking fault was
    /// handled.
    pub fn bbgpu_handle_fault(
        ucontext: *mut std::ffi::c_void,
        address: *mut std::ffi::c_void,
    ) -> c_int;
    /// `BB_WRITE_LOG=1`: prints the logged GPU-side writes to guest memory near the fault.
    pub fn bbgpu_dump_guest_writes(ucontext: *mut std::ffi::c_void);

    /// Keyboard text entry through the game window (IME dialog).
    pub fn bbgpu_text_input_begin(initial_utf8: *const c_char, prompt_utf8: *const c_char)
        -> c_int;
    pub fn bbgpu_text_input_poll(out_utf8: *mut c_char, size: u64) -> c_int;

    /// 1 while the in-game settings menu is open: the game's pad input is held neutral.
    pub fn bbgpu_overlay_captures_input() -> c_int;
    /// Hunter's Atlas: routes the pad buttons the quick menu/map use; returns the buttons left
    /// for the game.
    pub fn bbgpu_atlas_route_pad(buttons: u32, lx: u8, ly: u8, rx: u8, ry: u8) -> u32;

    /// Local trophy notifications. IDs are supplied by the game's trophy API, not PSN.
    pub fn bbgpu_trophy_configure(game_dir: *const c_char, user_dir: *const c_char, language: i32);
    pub fn bbgpu_trophy_load(timestamps: *const i64, count: u32);
    pub fn bbgpu_trophy_unlocked(id: i32, timestamp: i64);
    /// Summary for `sceNpTrophyGetGameInfo`.
    pub fn bbgpu_trophy_summary(out: *mut BbTrophySummary) -> c_int;
    pub fn bbgpu_trophy_is_unlocked(id: i32) -> c_int;

    /// Called at the start of every frame submission, on the submitting thread.
    pub fn bbgpu_set_frame_callback(callback: BbFrameCallback);

    /// `bbport.ini` settings by key for the in-game PC menu.
    pub fn bbgpu_setting_get(key: *const c_char, value: *mut c_int) -> c_int;
    pub fn bbgpu_setting_default(key: *const c_char, value: *mut c_int) -> c_int;
    pub fn bbgpu_setting_set(key: *const c_char, value: c_int);
    pub fn bbgpu_settings_save();

    /// Whether this GPU runs the upscaler (`BbSettings::Upscaler` index).
    pub fn bbgpu_upscaler_supported(upscaler: c_int) -> c_int;
    pub fn bbgpu_active_upscaler() -> c_int;
    pub fn bbgpu_upscaler_fallback_reason() -> *const c_char;

    /// Local achievements, read-only, for the in-game PC menu.
    pub fn bbgpu_achievement_count() -> c_int;
    pub fn bbgpu_achievement_get(index: c_int, out: *mut BbAchievement) -> c_int;
    /// Earned and total Gamerscore; 0 while the total is not the full 1000.
    pub fn bbgpu_achievement_score(earned: *mut c_int, total: *mut c_int) -> c_int;

    /// Mouse wheel notches for the pad read: bit 0 up, bit 1 down.
    pub fn bbgpu_mouse_wheel_take() -> u32;
    /// The mouse camera: whether the window holds the mouse and the motion since the last call.
    pub fn bbgpu_mouse_captured() -> c_int;
    pub fn bbgpu_mouse_motion_take(dx: *mut f32, dy: *mut f32) -> c_int;
    /// The menu pointer (`src/menu_pointer.c`).
    pub fn bbgpu_set_menu_pointer(active: c_int);
    /// Mouse on the menus' 1920x1080 stage; 0 when outside/unfocused.
    pub fn bbgpu_mouse_stage_position(x: *mut f32, y: *mut f32) -> c_int;

    /// Ends the game as closing its window does.
    pub fn bbgpu_request_quit();
    /// Number of symbols registered by the vendored libraries (diagnostics).
    pub fn bbgpu_symbol_count() -> u32;
}

impl Default for BbTrophySummary {
    fn default() -> Self {
        Self {
            num_trophies: 0,
            num_platinum: 0,
            num_gold: 0,
            num_silver: 0,
            num_bronze: 0,
            unlocked_trophies: 0,
            unlocked_platinum: 0,
            unlocked_gold: 0,
            unlocked_silver: 0,
            unlocked_bronze: 0,
            progress_percentage: 0,
            platinum_id: 0,
            base_locked: 0,
            title: [0; 128],
            description: [0; 1024],
        }
    }
}

impl Default for BbAchievement {
    fn default() -> Self {
        Self {
            id: 0,
            hidden: 0,
            score: 0,
            grade: 0,
            unlocked_at: 0,
            name: [0; 128],
            description: [0; 256],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    #[test]
    fn abi_struct_layouts_match_the_c_header() {
        assert_eq!(size_of::<BbGpuConfig>(), 40);
        assert_eq!(align_of::<BbGpuConfig>(), 8);
        assert_eq!(size_of::<BbTrophySummary>(), 1204);
        assert_eq!(align_of::<BbTrophySummary>(), 4);
        assert_eq!(size_of::<BbAchievement>(), 408);
        assert_eq!(align_of::<BbAchievement>(), 8);
        assert_eq!(size_of::<BbImeTextInput>(), 16);
    }
}
