//! User settings changed at run time, ported from `legacy/gpu/shim/bbport_settings.h`.
//! The file (`bbport.ini`, `BB_CONFIG` overrides the path) holds the values; environment
//! variables override it at start.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Upscaler {
    Off = 0,
    Fsr3 = 1,
    Fsr4 = 2,
    Fsr411 = 3,
    Taa = 4,
    Dlss = 5,
}

pub const UPSCALER_COUNT: i32 = 6;

/// FSR 4 v07 or FSR 4.1.1: the same inputs, settings and placement in the frame.
pub fn is_fsr4(upscaler: i32) -> bool {
    upscaler == Upscaler::Fsr4 as i32 || upscaler == Upscaler::Fsr411 as i32
}

/// The dispatch path can run before `OnFrameStart` after the menu selects FSR 4.
pub fn fsr4_implementation_needed_for_record(
    active_upscaler: i32,
    supported: bool,
    failed: bool,
    have_implementation: bool,
) -> bool {
    is_fsr4(active_upscaler) && supported && !failed && !have_implementation
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Preset {
    NativeAa = 0,
    Quality = 1,
    Balanced = 2,
    Performance = 3,
    UltraPerformance = 4,
}

pub const PRESET_COUNT: i32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum DebugView {
    None = 0,
    Reactive = 1,
    Motion = 2,
}

pub const DEBUG_VIEW_COUNT: i32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum FrameGen {
    Off = 0,
    Fsr3 = 1,
    Dlss = 2,
}

pub const FRAME_GEN_COUNT: i32 = 3;

/// Game effects switched by the community patches at start.
#[derive(Debug, Clone, Copy)]
pub struct Effect {
    pub key: &'static str,
    pub label: &'static str,
    pub default_on: bool,
}

pub const EFFECTS: &[Effect] = &[
    Effect {
        key: "effect_chromatic_aberration",
        label: "Chromatic aberration",
        default_on: true,
    },
    Effect {
        key: "effect_dof",
        label: "Depth of field (DoF)",
        default_on: true,
    },
    Effect {
        key: "effect_motion_blur",
        label: "Motion blur",
        default_on: true,
    },
    Effect {
        key: "effect_ssao",
        label: "Ambient occlusion (SSAO)",
        default_on: true,
    },
    Effect {
        key: "effect_game_aa",
        label: "Game anti-aliasing",
        default_on: true,
    },
    Effect {
        key: "effect_dynamic_shadows",
        label: "Dynamic shadows",
        default_on: true,
    },
    Effect {
        key: "effect_ssr",
        label: "Screen-space reflections (SSR)",
        default_on: false,
    },
    Effect {
        key: "skip_intro",
        label: "Skip intro logos and movie",
        default_on: false,
    },
    Effect {
        key: "debug_camera",
        label: "Free camera (Cross + L3)",
        default_on: false,
    },
    Effect {
        key: "debug_menu",
        label: "Debug menu (requires font files)",
        default_on: false,
    },
    Effect {
        key: "cheat_no_death",
        label: "Cheat: never die (minimum 1 HP)",
        default_on: false,
    },
    Effect {
        key: "cheat_stealth",
        label: "Cheat: enemies cannot see you",
        default_on: false,
    },
    Effect {
        key: "cheat_silent",
        label: "Cheat: enemies cannot hear you",
        default_on: false,
    },
    Effect {
        key: "cheat_rally_no_decay",
        label: "Cheat: Rally never decays",
        default_on: false,
    },
    Effect {
        key: "cheat_enemy_control",
        label: "Cheat: control targeted enemy (R3 / L3)",
        default_on: false,
    },
    Effect {
        key: "tweak_no_rally",
        label: "Disable Rally (health recovery)",
        default_on: false,
    },
    Effect {
        key: "tweak_camera_distance",
        label: "Increase camera distance",
        default_on: false,
    },
    Effect {
        key: "tweak_no_camera_rotation",
        label: "Disable automatic camera rotation",
        default_on: false,
    },
    Effect {
        key: "tweak_easy_run",
        label: "Run with less stick tilt",
        default_on: false,
    },
    Effect {
        key: "tweak_ragdoll",
        label: "Dark Souls-style ragdoll physics",
        default_on: false,
    },
    Effect {
        key: "pc_mirror_appearance",
        label: "Hunter's Dream mirror: change appearance",
        default_on: false,
    },
    Effect {
        key: "pc_two_invaders",
        label: "Two invaders in the late areas (every player needs it)",
        default_on: false,
    },
    Effect {
        key: "pc_rebirth",
        label: "Rebirth in the Nightmare at the Altar of Despair",
        default_on: false,
    },
    Effect {
        key: "pc_enhanced_lamps",
        label: "Lamps: Level Up and Storage at every lit lamp",
        default_on: false,
    },
];

pub const EFFECT_COUNT: usize = EFFECTS.len();
pub const ENHANCEMENT_COUNT: usize = 4;
pub const FIRST_ENHANCEMENT: usize = EFFECT_COUNT - ENHANCEMENT_COUNT;

/// Live output resolutions: the upscaler's output and the UI host targets.
pub const OUTPUT_WIDTHS: [i32; 13] = [
    1280, 1920, 2560, 3840, 1280, 2560, 3440, 5120, 5120, 1080, 3840, 3840, 1920,
];
pub const OUTPUT_HEIGHTS: [i32; 13] = [
    720, 1080, 1440, 2160, 800, 1080, 1440, 1440, 2160, 1920, 1600, 1080, 1080,
];
pub const OUTPUT_COUNT: usize = OUTPUT_WIDTHS.len();
pub const OUTPUT_CUSTOM: usize = OUTPUT_COUNT - 1;
pub const OUTPUT_DEFAULT: usize = 1;

/// The ini path: `BB_CONFIG` or `bbport.ini`.
pub fn config_path() -> PathBuf {
    std::env::var("BB_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("bbport.ini"))
}

pub struct Values {
    pub upscaler: AtomicI32,
    pub active_upscaler: AtomicI32,
    pub upscaler_fallback_reason: OnceLock<String>,
    pub preset: AtomicI32,
    pub sharpen: AtomicBool,
    pub sharpness: AtomicU32,
    pub jitter: AtomicBool,
    pub reactive: AtomicBool,
    pub object_motion: AtomicBool,
    pub object_motion_explicit: AtomicBool,
    pub object_motion_supported: AtomicBool,
    pub object_motion_opt_in_allowed: AtomicBool,
    pub object_motion_problem: OnceLock<String>,
    pub reactive_scale: AtomicU32,
    pub reactive_threshold: AtomicU32,
    pub reactive_max: AtomicU32,
    pub debug_view: AtomicI32,
    pub show_fps: AtomicBool,
    pub fullscreen: AtomicBool,
    pub fsr4_auto_exposure: AtomicBool,
    pub fsr4_invert_jitter: AtomicBool,
    pub mouse_camera: AtomicBool,
    pub mouse_sensitivity: AtomicI32,
    pub mouse_invert_y: AtomicBool,
    pub mouse_menu: AtomicBool,
    pub active_render_width: AtomicI32,
    pub active_render_height: AtomicI32,
    pub effects: Vec<AtomicBool>,
    pub model_lod: AtomicI32,
    pub output_res: AtomicI32,
    pub live_resolution: AtomicI32,
    pub fsr4_problem: OnceLock<String>,
    pub fsr4_supported: AtomicBool,
    pub fsr411_supported: AtomicBool,
    pub dlss_supported: AtomicBool,
    pub dlss_problem: OnceLock<String>,
    pub frame_gen: AtomicI32,
    pub frame_gen_multiplier: AtomicI32,
    pub frame_gen_fit_display: AtomicBool,
    pub dlssg_max_multiplier: AtomicI32,
    pub frame_gen_problem: OnceLock<String>,
    pub frame_gen_now: AtomicI32,
    pub frame_gen_display_hz: AtomicI32,
    pub game_fps: AtomicU32,
    pub frame_gen_flip_limit: AtomicI32,
    pub startup_preset: AtomicI32,
    pub startup_upscaler: AtomicI32,
    pub startup_object_motion: AtomicBool,
    pub startup_effects: Vec<AtomicBool>,
    pub startup_model_lod: AtomicI32,
    pub startup_output_res: AtomicI32,
    pub startup_live_resolution: AtomicI32,
}

impl Default for Values {
    fn default() -> Self {
        let effects = EFFECTS
            .iter()
            .map(|effect| AtomicBool::new(effect.default_on))
            .collect();
        let startup_effects = EFFECTS.iter().map(|_| AtomicBool::new(false)).collect();
        Self {
            upscaler: AtomicI32::new(Upscaler::Fsr3 as i32),
            active_upscaler: AtomicI32::new(Upscaler::Fsr3 as i32),
            upscaler_fallback_reason: OnceLock::new(),
            preset: AtomicI32::new(Preset::NativeAa as i32),
            sharpen: AtomicBool::new(true),
            sharpness: AtomicU32::new(0.3f32.to_bits()),
            jitter: AtomicBool::new(true),
            reactive: AtomicBool::new(false),
            object_motion: AtomicBool::new(true),
            object_motion_explicit: AtomicBool::new(false),
            object_motion_supported: AtomicBool::new(true),
            object_motion_opt_in_allowed: AtomicBool::new(false),
            object_motion_problem: OnceLock::new(),
            reactive_scale: AtomicU32::new(1.0f32.to_bits()),
            reactive_threshold: AtomicU32::new(0.2f32.to_bits()),
            reactive_max: AtomicU32::new(0.9f32.to_bits()),
            debug_view: AtomicI32::new(DebugView::None as i32),
            show_fps: AtomicBool::new(false),
            fullscreen: AtomicBool::new(false),
            fsr4_auto_exposure: AtomicBool::new(true),
            fsr4_invert_jitter: AtomicBool::new(false),
            mouse_camera: AtomicBool::new(true),
            mouse_sensitivity: AtomicI32::new(10),
            mouse_invert_y: AtomicBool::new(false),
            mouse_menu: AtomicBool::new(true),
            active_render_width: AtomicI32::new(1920),
            active_render_height: AtomicI32::new(1080),
            effects,
            model_lod: AtomicI32::new(0),
            output_res: AtomicI32::new(OUTPUT_DEFAULT as i32),
            live_resolution: AtomicI32::new(0),
            fsr4_problem: OnceLock::new(),
            fsr4_supported: AtomicBool::new(false),
            fsr411_supported: AtomicBool::new(false),
            dlss_supported: AtomicBool::new(false),
            dlss_problem: OnceLock::new(),
            frame_gen: AtomicI32::new(FrameGen::Off as i32),
            frame_gen_multiplier: AtomicI32::new(2),
            frame_gen_fit_display: AtomicBool::new(false),
            dlssg_max_multiplier: AtomicI32::new(1),
            frame_gen_problem: OnceLock::new(),
            frame_gen_now: AtomicI32::new(0),
            frame_gen_display_hz: AtomicI32::new(0),
            game_fps: AtomicU32::new(0.0f32.to_bits()),
            frame_gen_flip_limit: AtomicI32::new(0),
            startup_preset: AtomicI32::new(Preset::NativeAa as i32),
            startup_upscaler: AtomicI32::new(Upscaler::Fsr3 as i32),
            startup_object_motion: AtomicBool::new(true),
            startup_effects,
            startup_model_lod: AtomicI32::new(0),
            startup_output_res: AtomicI32::new(OUTPUT_DEFAULT as i32),
            startup_live_resolution: AtomicI32::new(0),
        }
    }
}

impl Values {
    pub fn sharpness(&self) -> f32 {
        f32::from_bits(self.sharpness.load(Ordering::Relaxed))
    }
    pub fn set_sharpness(&self, value: f32) {
        self.sharpness.store(value.to_bits(), Ordering::Relaxed);
    }
    pub fn reactive_scale(&self) -> f32 {
        f32::from_bits(self.reactive_scale.load(Ordering::Relaxed))
    }
    pub fn set_reactive_scale(&self, value: f32) {
        self.reactive_scale
            .store(value.to_bits(), Ordering::Relaxed);
    }
    pub fn reactive_threshold(&self) -> f32 {
        f32::from_bits(self.reactive_threshold.load(Ordering::Relaxed))
    }
    pub fn set_reactive_threshold(&self, value: f32) {
        self.reactive_threshold
            .store(value.to_bits(), Ordering::Relaxed);
    }
    pub fn reactive_max(&self) -> f32 {
        f32::from_bits(self.reactive_max.load(Ordering::Relaxed))
    }
    pub fn set_reactive_max(&self, value: f32) {
        self.reactive_max.store(value.to_bits(), Ordering::Relaxed);
    }
    pub fn game_fps(&self) -> f32 {
        f32::from_bits(self.game_fps.load(Ordering::Relaxed))
    }
    pub fn set_game_fps(&self, value: f32) {
        self.game_fps.store(value.to_bits(), Ordering::Relaxed);
    }
}

pub fn get() -> &'static Values {
    static VALUES: OnceLock<Values> = OnceLock::new();
    VALUES.get_or_init(Values::default)
}

fn env_bool(key: &str) -> Option<bool> {
    std::env::var(key).ok().map(|value| value == "1")
}

fn env_i32(key: &str) -> Option<i32> {
    std::env::var(key)
        .ok()
        .and_then(|value| value.trim().parse().ok())
}

fn env_f32(key: &str) -> Option<f32> {
    std::env::var(key)
        .ok()
        .and_then(|value| value.trim().parse().ok())
}

/// Reads the file, then the environment overrides. Called once at start.
pub fn load() {
    let values = get();
    for line in read_ini().lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "upscaler" => set_if_parsed(&values.upscaler, value),
            "preset" => set_if_parsed(&values.preset, value),
            "sharpen" => set_bool(&values.sharpen, value),
            "sharpness" => {
                if let Ok(parsed) = value.parse() {
                    values.set_sharpness(parsed);
                }
            }
            "jitter" => set_bool(&values.jitter, value),
            "reactive" => set_bool(&values.reactive, value),
            "object_motion" => {
                set_bool(&values.object_motion, value);
                values.object_motion_explicit.store(true, Ordering::Relaxed);
            }
            "debug_view" => set_if_parsed(&values.debug_view, value),
            "show_fps" => set_bool(&values.show_fps, value),
            "fullscreen" => set_bool(&values.fullscreen, value),
            "fsr4_auto_exposure" => set_bool(&values.fsr4_auto_exposure, value),
            "fsr4_invert_jitter" => set_bool(&values.fsr4_invert_jitter, value),
            "mouse_camera" => set_bool(&values.mouse_camera, value),
            "mouse_sensitivity" => set_if_parsed(&values.mouse_sensitivity, value),
            "mouse_invert_y" => set_bool(&values.mouse_invert_y, value),
            "mouse_menu" => set_bool(&values.mouse_menu, value),
            "model_lod" => set_if_parsed(&values.model_lod, value),
            "output_res" => set_if_parsed(&values.output_res, value),
            "live_resolution" => set_if_parsed(&values.live_resolution, value),
            "frame_gen" => set_if_parsed(&values.frame_gen, value),
            "frame_gen_multiplier" => set_if_parsed(&values.frame_gen_multiplier, value),
            "frame_gen_fit_display" => set_bool(&values.frame_gen_fit_display, value),
            other => {
                if let Some(index) = EFFECTS.iter().position(|effect| effect.key == other) {
                    set_bool(&values.effects[index], value);
                }
            }
        }
    }
    if let Some(value) = env_i32("BB_UPSCALER") {
        values.upscaler.store(value, Ordering::Relaxed);
        values.active_upscaler.store(value, Ordering::Relaxed);
    }
    if let Some(value) = env_i32("BB_PRESET") {
        values.preset.store(value, Ordering::Relaxed);
    }
    if let Some(value) = env_bool("BB_SHARPEN") {
        values.sharpen.store(value, Ordering::Relaxed);
    }
    if let Some(value) = env_f32("BB_SHARPNESS") {
        values.set_sharpness(value);
    }
    if let Some(value) = env_bool("BB_JITTER") {
        values.jitter.store(value, Ordering::Relaxed);
    }
    if let Some(value) = env_bool("BB_REACTIVE") {
        values.reactive.store(value, Ordering::Relaxed);
    }
    if let Some(value) = env_bool("BB_OBJECT_MOTION") {
        values.object_motion.store(value, Ordering::Relaxed);
        values.object_motion_explicit.store(true, Ordering::Relaxed);
    }
    if let Some(value) = env_bool("BB_SHOW_FPS") {
        values.show_fps.store(value, Ordering::Relaxed);
    }
    if let Some(value) = env_bool("BB_FULLSCREEN") {
        values.fullscreen.store(value, Ordering::Relaxed);
    }
    if let Some(value) = env_bool("BB_MOUSE_CAMERA") {
        values.mouse_camera.store(value, Ordering::Relaxed);
    }
    if let Some(value) = env_i32("BB_MOUSE_SENSITIVITY") {
        values.mouse_sensitivity.store(value, Ordering::Relaxed);
    }
    if let Some(value) = env_bool("BB_MOUSE_INVERT_Y") {
        values.mouse_invert_y.store(value, Ordering::Relaxed);
    }
    if let Some(value) = env_bool("BB_MOUSE_MENU") {
        values.mouse_menu.store(value, Ordering::Relaxed);
    }
    if let Some(value) = env_i32("BB_MODEL_LOD") {
        values.model_lod.store(value, Ordering::Relaxed);
    }
    if let Some(value) = env_i32("BB_OUTPUT_RES") {
        values.output_res.store(value, Ordering::Relaxed);
    }
    if let Some(value) = env_i32("BB_LIVE_RESOLUTION") {
        values.live_resolution.store(value, Ordering::Relaxed);
    }
    values
        .startup_preset
        .store(values.preset.load(Ordering::Relaxed), Ordering::Relaxed);
    values
        .startup_upscaler
        .store(values.upscaler.load(Ordering::Relaxed), Ordering::Relaxed);
    values.startup_object_motion.store(
        values.object_motion.load(Ordering::Relaxed),
        Ordering::Relaxed,
    );
    values
        .startup_model_lod
        .store(values.model_lod.load(Ordering::Relaxed), Ordering::Relaxed);
    values
        .startup_output_res
        .store(values.output_res.load(Ordering::Relaxed), Ordering::Relaxed);
    values.startup_live_resolution.store(
        values.live_resolution.load(Ordering::Relaxed),
        Ordering::Relaxed,
    );
    for (index, _effect) in EFFECTS.iter().enumerate() {
        values.startup_effects[index].store(
            values.effects[index].load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
    }
}

fn read_ini() -> String {
    std::fs::read_to_string(config_path()).unwrap_or_default()
}

fn set_if_parsed(target: &AtomicI32, value: &str) {
    if let Ok(parsed) = value.parse() {
        target.store(parsed, Ordering::Relaxed);
    }
}

fn set_bool(target: &AtomicBool, value: &str) {
    if let Ok(parsed) = value.parse::<i32>() {
        target.store(parsed != 0, Ordering::Relaxed);
    }
}

/// Writes the file (menu changes).
pub fn save() {
    let values = get();
    let mut out = String::new();
    out.push_str(&format!(
        "upscaler={}\n",
        values.upscaler.load(Ordering::Relaxed)
    ));
    out.push_str(&format!(
        "preset={}\n",
        values.preset.load(Ordering::Relaxed)
    ));
    out.push_str(&format!(
        "sharpen={}\n",
        i32::from(values.sharpen.load(Ordering::Relaxed))
    ));
    out.push_str(&format!("sharpness={}\n", values.sharpness()));
    out.push_str(&format!(
        "jitter={}\n",
        i32::from(values.jitter.load(Ordering::Relaxed))
    ));
    out.push_str(&format!(
        "reactive={}\n",
        i32::from(values.reactive.load(Ordering::Relaxed))
    ));
    out.push_str(&format!(
        "object_motion={}\n",
        i32::from(values.object_motion.load(Ordering::Relaxed))
    ));
    out.push_str(&format!(
        "debug_view={}\n",
        values.debug_view.load(Ordering::Relaxed)
    ));
    out.push_str(&format!(
        "show_fps={}\n",
        i32::from(values.show_fps.load(Ordering::Relaxed))
    ));
    out.push_str(&format!(
        "fullscreen={}\n",
        i32::from(values.fullscreen.load(Ordering::Relaxed))
    ));
    out.push_str(&format!(
        "mouse_camera={}\n",
        i32::from(values.mouse_camera.load(Ordering::Relaxed))
    ));
    out.push_str(&format!(
        "mouse_sensitivity={}\n",
        values.mouse_sensitivity.load(Ordering::Relaxed)
    ));
    out.push_str(&format!(
        "mouse_invert_y={}\n",
        i32::from(values.mouse_invert_y.load(Ordering::Relaxed))
    ));
    out.push_str(&format!(
        "mouse_menu={}\n",
        i32::from(values.mouse_menu.load(Ordering::Relaxed))
    ));
    out.push_str(&format!(
        "model_lod={}\n",
        values.model_lod.load(Ordering::Relaxed)
    ));
    out.push_str(&format!(
        "output_res={}\n",
        values.output_res.load(Ordering::Relaxed)
    ));
    out.push_str(&format!(
        "live_resolution={}\n",
        values.live_resolution.load(Ordering::Relaxed)
    ));
    out.push_str(&format!(
        "frame_gen={}\n",
        values.frame_gen.load(Ordering::Relaxed)
    ));
    out.push_str(&format!(
        "frame_gen_multiplier={}\n",
        values.frame_gen_multiplier.load(Ordering::Relaxed)
    ));
    for (index, effect) in EFFECTS.iter().enumerate() {
        out.push_str(&format!(
            "{}={}\n",
            effect.key,
            i32::from(values.effects[index].load(Ordering::Relaxed))
        ));
    }
    let _ = std::fs::write(config_path(), out);
}

/// Checks the loaded choice before the first frame; unsupported FSR 4 uses FSR 3.1.
pub fn configure_upscaler_support(fsr4: bool, fsr411: bool) {
    let values = get();
    values.fsr4_supported.store(fsr4, Ordering::Relaxed);
    values.fsr411_supported.store(fsr411, Ordering::Relaxed);
    let selected = values.upscaler.load(Ordering::Relaxed);
    let supported = if selected == Upscaler::Fsr411 as i32 {
        fsr411
    } else {
        fsr4
    };
    if is_fsr4(selected) && !supported {
        values
            .active_upscaler
            .store(Upscaler::Fsr3 as i32, Ordering::Relaxed);
        let _ = values
            .upscaler_fallback_reason
            .set(values.fsr4_problem.get().cloned().unwrap_or_default());
    } else {
        values.active_upscaler.store(selected, Ordering::Relaxed);
    }
}

/// The renderer's object-motion availability: clears the setting where it is refused.
pub fn configure_object_motion_support(
    supported: bool,
    problem: Option<&str>,
    opt_in_allowed: bool,
) {
    let values = get();
    values
        .object_motion_supported
        .store(supported, Ordering::Relaxed);
    values
        .object_motion_opt_in_allowed
        .store(opt_in_allowed, Ordering::Relaxed);
    if let Some(problem) = problem {
        let _ = values.object_motion_problem.set(problem.to_string());
    }
    if !supported && !(opt_in_allowed && values.object_motion_explicit.load(Ordering::Relaxed)) {
        values.object_motion.store(false, Ordering::Relaxed);
    }
}

/// After device creation: DLSS availability; a DLSS setting falls back without it.
pub fn configure_dlss_support(available: bool, problem: Option<&str>) {
    let values = get();
    values.dlss_supported.store(available, Ordering::Relaxed);
    if let Some(problem) = problem {
        let _ = values.dlss_problem.set(problem.to_string());
    }
    if !available && values.upscaler.load(Ordering::Relaxed) == Upscaler::Dlss as i32 {
        values
            .active_upscaler
            .store(Upscaler::Fsr3 as i32, Ordering::Relaxed);
    }
    if !available && values.frame_gen.load(Ordering::Relaxed) == FrameGen::Dlss as i32 {
        values
            .frame_gen
            .store(FrameGen::Off as i32, Ordering::Relaxed);
    }
}

/// Startup-patched scene dimensions cannot change until run.sh prepares a new image.
pub fn fixed_render_session() -> bool {
    let values = get();
    values.live_resolution.load(Ordering::Relaxed) != 0
        || values.output_res.load(Ordering::Relaxed) != OUTPUT_DEFAULT as i32
}

pub fn render_preset() -> i32 {
    get().preset.load(Ordering::Relaxed)
}

pub fn resolution_needs_restart() -> bool {
    let values = get();
    values.output_res.load(Ordering::Relaxed) != values.startup_output_res.load(Ordering::Relaxed)
        || values.live_resolution.load(Ordering::Relaxed)
            != values.startup_live_resolution.load(Ordering::Relaxed)
}

/// Camera and HUD aspect changes require new guest patches on restart.
pub fn aspect_needs_restart() -> bool {
    resolution_needs_restart()
}

/// Render resolution divisor of a preset (1.0 native, 1.5 quality, ...).
pub fn preset_scale(preset: i32) -> f32 {
    match preset {
        x if x == Preset::Quality as i32 => 1.5,
        x if x == Preset::Balanced as i32 => 1.7,
        x if x == Preset::Performance as i32 => 2.0,
        x if x == Preset::UltraPerformance as i32 => 3.0,
        _ => 1.0,
    }
}

pub fn preset_name(preset: i32) -> &'static str {
    match preset {
        x if x == Preset::Quality as i32 => "Quality",
        x if x == Preset::Balanced as i32 => "Balanced",
        x if x == Preset::Performance as i32 => "Performance",
        x if x == Preset::UltraPerformance as i32 => "Ultra Performance",
        _ => "Native AA",
    }
}

pub fn upscaler_name(upscaler: i32) -> &'static str {
    match upscaler {
        x if x == Upscaler::Fsr3 as i32 => "FSR 3.1",
        x if x == Upscaler::Fsr4 as i32 => "FSR 4",
        x if x == Upscaler::Fsr411 as i32 => "FSR 4.1.1",
        x if x == Upscaler::Taa as i32 => "TAA",
        x if x == Upscaler::Dlss as i32 => "DLSS",
        _ => "Off",
    }
}

pub fn frame_gen_name(frame_gen: i32) -> &'static str {
    match frame_gen {
        x if x == FrameGen::Fsr3 as i32 => "FSR 3.1",
        x if x == FrameGen::Dlss as i32 => "DLSS",
        _ => "Off",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_helpers_match_the_menu() {
        assert_eq!(preset_scale(Preset::NativeAa as i32), 1.0);
        assert_eq!(preset_scale(Preset::Quality as i32), 1.5);
        assert_eq!(
            preset_name(Preset::UltraPerformance as i32),
            "Ultra Performance"
        );
        assert_eq!(upscaler_name(Upscaler::Fsr411 as i32), "FSR 4.1.1");
        assert_eq!(frame_gen_name(FrameGen::Dlss as i32), "DLSS");
        assert!(is_fsr4(Upscaler::Fsr4 as i32));
        assert!(!is_fsr4(Upscaler::Fsr3 as i32));
        assert!(fsr4_implementation_needed_for_record(
            Upscaler::Fsr4 as i32,
            true,
            false,
            false
        ));
        assert!(!fsr4_implementation_needed_for_record(
            Upscaler::Fsr4 as i32,
            true,
            false,
            true
        ));
    }

    #[test]
    fn the_effect_table_keeps_the_defaults_and_enhancements_last() {
        assert_eq!(EFFECT_COUNT, 24);
        assert_eq!(ENHANCEMENT_COUNT, 4);
        assert!(EFFECTS[0].default_on);
        assert_eq!(EFFECTS[FIRST_ENHANCEMENT].key, "pc_mirror_appearance");
        assert_eq!(OUTPUT_WIDTHS[OUTPUT_DEFAULT], 1920);
        assert_eq!(OUTPUT_HEIGHTS[OUTPUT_CUSTOM], 1080);
    }

    #[test]
    fn unsupported_fsr4_falls_back_to_fsr3() {
        let values = get();
        values
            .upscaler
            .store(Upscaler::Fsr4 as i32, Ordering::Relaxed);
        configure_upscaler_support(false, false);
        assert_eq!(
            values.active_upscaler.load(Ordering::Relaxed),
            Upscaler::Fsr3 as i32
        );
        configure_upscaler_support(true, true);
        assert_eq!(
            values.active_upscaler.load(Ordering::Relaxed),
            Upscaler::Fsr4 as i32
        );
        values
            .upscaler
            .store(Upscaler::Fsr3 as i32, Ordering::Relaxed);
    }
}
