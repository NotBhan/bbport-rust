//! SDL window host for the Vulkan swapchain, ported from `legacy/gpu/shim/window.cpp`.
//! SDL itself is not linked: the window thread feeds decoded [`Event`]s into [`WindowState`],
//! and the C ABI entry points read the same host state.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use crate::gpu::shim::bbport_settings;
use crate::gpu::shim::sdl_window::{WindowSystemInfo, WindowSystemType};
use crate::gpu::shim::stage_point;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    TextInput(String),
    KeyBackspace,
    KeyEnter,
    KeyEscape,
    GamepadSouth,
    GamepadEast,
    MouseButtonDown,
    MouseMotion { xrel: f32, yrel: f32 },
    MouseWheel { y: f32, flipped: bool },
    WindowResized { width: i32, height: i32 },
    WindowPixelSizeChanged { width: i32, height: i32 },
    FocusLost,
    HdrStateChanged(bool),
    Quit,
    OverlayHandled,
}

/// The window info holds the host's display and surface handles, which SDL owns on its thread.
unsafe impl Send for WindowState {}

pub struct WindowState {
    pub width: i32,
    pub height: i32,
    pub base_title: String,
    pub is_open: bool,
    pub fullscreen_applied: bool,
    pub window_info: WindowSystemInfo,
    pub on_hdr_display: bool,
    text: String,
    text_prompt: String,
    text_state: i32,
    text_requested: bool,
    text_active: bool,
    mouse_armed: bool,
    mouse_captured: bool,
    menu_pointer_seen: bool,
    rearm_after_menu: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Tick {
    pub fullscreen: bool,
    pub mouse_camera: bool,
    pub releases_mouse: bool,
    pub focused: bool,
    pub menu_pointer: bool,
}

impl Default for Tick {
    fn default() -> Self {
        Self {
            fullscreen: false,
            mouse_camera: true,
            releases_mouse: false,
            focused: true,
            menu_pointer: false,
        }
    }
}

impl WindowState {
    /// The window is created on the video driver's thread; only the state is built here.
    pub fn new(width: i32, height: i32, title: &str, system: WindowSystemType) -> Self {
        let fullscreen_applied = bbport_settings::get().fullscreen.load(Ordering::Relaxed);
        Self {
            width,
            height,
            base_title: title.to_string(),
            is_open: true,
            fullscreen_applied,
            window_info: WindowSystemInfo {
                kind: system,
                ..WindowSystemInfo::default()
            },
            on_hdr_display: false,
            text: String::new(),
            text_prompt: String::new(),
            text_state: 0,
            text_requested: false,
            text_active: false,
            mouse_armed: false,
            mouse_captured: false,
            menu_pointer_seen: false,
            rearm_after_menu: false,
        }
    }

    pub fn begin_text_input(&mut self, initial: &str, prompt: &str) {
        self.text = initial.to_string();
        self.text_prompt = prompt.to_string();
        self.text_state = 0;
        self.text_requested = true;
    }

    pub fn poll_text_input(&self) -> (String, i32) {
        (self.text.clone(), self.text_state)
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub const fn text_active(&self) -> bool {
        self.text_active
    }

    pub const fn mouse_captured(&self) -> bool {
        self.mouse_captured
    }

    /// The title: the base one, the status, or the text dialog with its caret.
    pub fn title(&self, status: Option<&str>) -> String {
        if self.text_active {
            format!(
                "{} \u{2014} {}: {}_  (Enter = OK, Esc = cancel)",
                self.base_title, self.text_prompt, self.text
            )
        } else {
            match status {
                Some(status) if !status.is_empty() => {
                    format!("{} \u{2014} {status}", self.base_title)
                }
                _ => self.base_title.clone(),
            }
        }
    }

    /// `WindowSDL::PollEvents`: the per-frame part (text entry, fullscreen, mouse capture).
    pub fn tick(&mut self, tick: Tick) {
        if self.text_requested {
            self.text_requested = false;
            self.text_active = true;
        }
        self.fullscreen_applied = tick.fullscreen;
        if tick.menu_pointer && !self.menu_pointer_seen {
            self.menu_pointer_seen = true;
            self.rearm_after_menu = self.mouse_captured;
        } else if !tick.menu_pointer && self.menu_pointer_seen {
            self.menu_pointer_seen = false;
            self.mouse_armed = self.mouse_armed || self.rearm_after_menu;
            self.rearm_after_menu = false;
        }
        let can_capture = bbport_settings::get().mouse_camera.load(Ordering::Relaxed)
            && !self.text_active
            && !tick.releases_mouse
            && tick.focused
            && !tick.menu_pointer;
        if !can_capture && !tick.menu_pointer {
            self.mouse_armed = false;
        }
        let wanted = can_capture && self.mouse_armed;
        if wanted != self.mouse_captured {
            self.mouse_captured = wanted;
            set_mouse_captured(wanted);
        }
        set_window_focus(tick.focused);
    }

    /// Returns true when the event belonged to the text dialog.
    pub fn apply_event(&mut self, event: &Event, can_capture: bool) -> bool {
        if self.text_active {
            match event {
                Event::TextInput(text) => {
                    self.text.push_str(text);
                    return true;
                }
                Event::KeyBackspace => {
                    if !self.text.is_empty() {
                        let mut cut = self.text.len() - 1;
                        let bytes = self.text.as_bytes();
                        while cut > 0 && (bytes[cut] & 0xC0) == 0x80 {
                            cut -= 1;
                        }
                        self.text.truncate(cut);
                    }
                    return true;
                }
                Event::KeyEnter | Event::KeyEscape => {
                    self.text_state = if matches!(event, Event::KeyEscape) {
                        2
                    } else {
                        1
                    };
                    self.text_active = false;
                    return true;
                }
                Event::GamepadSouth | Event::GamepadEast => {
                    self.text_state = if matches!(event, Event::GamepadSouth) {
                        1
                    } else {
                        2
                    };
                    self.text_active = false;
                    return true;
                }
                _ => {}
            }
        }
        match event {
            Event::WindowResized { width, height }
            | Event::WindowPixelSizeChanged { width, height } => {
                self.width = *width;
                self.height = *height;
            }
            Event::MouseButtonDown => self.mouse_armed = can_capture,
            Event::MouseMotion { xrel, yrel } => {
                if self.mouse_captured {
                    note_mouse_motion(*xrel, *yrel);
                }
            }
            Event::MouseWheel { y, flipped } => {
                note_mouse_wheel(if *flipped { -y } else { *y });
            }
            Event::FocusLost => self.mouse_armed = false,
            Event::HdrStateChanged(on) => self.on_hdr_display = *on,
            Event::Quit => self.is_open = false,
            _ => {}
        }
        false
    }
}

fn window_points() -> &'static Mutex<(i32, i32)> {
    static POINTS: OnceLock<Mutex<(i32, i32)>> = OnceLock::new();
    POINTS.get_or_init(|| Mutex::new((0, 0)))
}

fn wheel_state() -> &'static Mutex<(i32, i32, bool, bool)> {
    static WHEEL: OnceLock<Mutex<(i32, i32, bool, bool)>> = OnceLock::new();
    WHEEL.get_or_init(|| Mutex::new((0, 0, false, false)))
}

fn lock<T>(mutex: &'static Mutex<T>) -> MutexGuard<'static, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

static WINDOW_FOCUSED: AtomicBool = AtomicBool::new(false);
static MENU_POINTER: AtomicBool = AtomicBool::new(false);
static MOUSE_CAPTURED: AtomicBool = AtomicBool::new(false);
static MOTION_X: AtomicI64 = AtomicI64::new(0);
static MOTION_Y: AtomicI64 = AtomicI64::new(0);
static MOTION_TAKEN_MS: AtomicI64 = AtomicI64::new(0);
static FRAME_W: AtomicI32 = AtomicI32::new(0);
static FRAME_H: AtomicI32 = AtomicI32::new(0);
static QUIT_REQUESTED: AtomicBool = AtomicBool::new(false);

fn set_window_focus(focused: bool) {
    WINDOW_FOCUSED.store(focused, Ordering::Relaxed);
}

pub fn set_window_points(width: i32, height: i32) {
    *lock(window_points()) = (width, height);
}

/// Mouse wheel notches since the pad last took one: each is a short press of WheelUp/WheelDown.
pub fn note_mouse_wheel(y: f32) {
    let mut wheel = lock(wheel_state());
    if y > 0.0 {
        wheel.0 += 1;
    }
    if y < 0.0 {
        wheel.1 += 1;
    }
}

/// One notch a read, each direction: bit 0 up, bit 1 down, held for one read.
pub fn mouse_wheel_take() -> u32 {
    let mut wheel = lock(wheel_state());
    let mut bits = 0;
    if wheel.2 {
        wheel.2 = false;
    } else if wheel.0 > 0 {
        wheel.0 -= 1;
        wheel.2 = true;
        bits |= 1;
    }
    if wheel.3 {
        wheel.3 = false;
    } else if wheel.1 > 0 {
        wheel.1 -= 1;
        wheel.3 = true;
        bits |= 2;
    }
    bits
}

/// Mouse motion in 1/16 counts for the mouse camera.
pub fn note_mouse_motion(x: f32, y: f32) {
    MOTION_X.fetch_add((x * 16.0).round() as i64, Ordering::Relaxed);
    MOTION_Y.fetch_add((y * 16.0).round() as i64, Ordering::Relaxed);
}

pub fn set_mouse_captured(captured: bool) {
    MOUSE_CAPTURED.store(captured, Ordering::Relaxed);
    MOTION_X.store(0, Ordering::Relaxed);
    MOTION_Y.store(0, Ordering::Relaxed);
}

pub fn set_menu_pointer(active: bool) {
    MENU_POINTER.store(active, Ordering::Relaxed);
}

pub fn mouse_captured() -> bool {
    MOUSE_CAPTURED.load(Ordering::Relaxed)
}

/// The presented frame's size (the presenter letterboxes it into the window).
pub fn note_presented_frame(width: i32, height: i32) {
    FRAME_W.store(width, Ordering::Relaxed);
    FRAME_H.store(height, Ordering::Relaxed);
}

/// Mouse on the menus' 1920x1080 stage; `None` when outside, unfocused or captured.
pub fn mouse_stage_position(mouse_x: f32, mouse_y: f32) -> Option<(f32, f32)> {
    let (window_w, window_h) = *lock(window_points());
    let ww = window_w as f32;
    let wh = window_h as f32;
    let fw = FRAME_W.load(Ordering::Relaxed) as f32;
    let fh = FRAME_H.load(Ordering::Relaxed) as f32;
    if !WINDOW_FOCUSED.load(Ordering::Relaxed)
        || ww <= 0.0
        || wh <= 0.0
        || fw <= 0.0
        || fh <= 0.0
        || MOUSE_CAPTURED.load(Ordering::Relaxed)
    {
        return None;
    }
    stage_point::point(mouse_x, mouse_y, ww, wh, fw, fh)
}

/// Motion since the last read; motion that waited over 250 ms is dropped.
pub fn mouse_motion_take(now_ms: i64) -> (f32, f32) {
    let last = MOTION_TAKEN_MS.swap(now_ms, Ordering::Relaxed);
    let x = MOTION_X.swap(0, Ordering::Relaxed);
    let y = MOTION_Y.swap(0, Ordering::Relaxed);
    if now_ms - last > 250 {
        return (0.0, 0.0);
    }
    (x as f32 / 16.0, y as f32 / 16.0)
}

fn status() -> &'static Mutex<(String, bool)> {
    static STATUS: OnceLock<Mutex<(String, bool)>> = OnceLock::new();
    STATUS.get_or_init(|| Mutex::new((String::new(), false)))
}

/// The window title's status (any thread; empty: the plain title).
pub fn set_window_status(text: &str) {
    let mut status = lock(status());
    if status.0 != text {
        status.0 = text.to_string();
        status.1 = true;
    }
}

pub fn take_window_status() -> Option<String> {
    let mut status = lock(status());
    if !status.1 {
        return None;
    }
    status.1 = false;
    Some(status.0.clone())
}

/// The in-game PC menu's Quit Game: the same as closing the window.
pub fn request_quit() {
    QUIT_REQUESTED.store(true, Ordering::Relaxed);
}

pub fn take_quit_request() -> bool {
    QUIT_REQUESTED.swap(false, Ordering::Relaxed)
}

/// The overlay's text entry mirror: the window thread pushes its title state to the overlay.
pub fn set_text_entry_for_overlay(active: bool, prompt: &str, text: &str) {
    crate::gpu::shim::bbport_overlay::set_text_entry(active, prompt, text);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window() -> WindowState {
        let mut window = WindowState::new(1280, 720, "Bloodborne", WindowSystemType::X11);
        window.tick(Tick::default());
        window
    }

    #[test]
    fn text_input_appends_drops_a_code_point_and_commits() {
        let mut window = window();
        window.begin_text_input("", "Name");
        window.tick(Tick::default());
        assert!(window.text_active());
        window.apply_event(&Event::TextInput("Vile".into()), true);
        window.apply_event(&Event::TextInput("\u{00e9}".into()), true);
        window.apply_event(&Event::KeyBackspace, true);
        assert_eq!(window.text(), "Vile");
        window.apply_event(&Event::KeyEnter, true);
        let (text, state) = window.poll_text_input();
        assert_eq!(text, "Vile");
        assert_eq!(state, 1);
        assert!(!window.text_active());
        window.begin_text_input("", "Name");
        window.tick(Tick::default());
        window.apply_event(&Event::KeyEscape, true);
        assert_eq!(window.poll_text_input().1, 2);
        window.begin_text_input("", "Name");
        window.tick(Tick::default());
        window.apply_event(&Event::GamepadEast, true);
        assert_eq!(window.poll_text_input().1, 2);
    }

    #[test]
    fn the_mouse_camera_needs_a_click_and_gives_the_mouse_back_for_a_menu() {
        let mut window = window();
        assert!(!window.mouse_captured());
        window.apply_event(&Event::MouseButtonDown, true);
        window.tick(Tick::default());
        assert!(window.mouse_captured());
        window.tick(Tick {
            menu_pointer: true,
            ..Tick::default()
        });
        assert!(!window.mouse_captured());
        window.tick(Tick::default());
        assert!(
            window.mouse_captured(),
            "the menu closing re-arms the camera"
        );
        window.tick(Tick {
            focused: false,
            ..Tick::default()
        });
        assert!(!window.mouse_captured());
    }

    #[test]
    fn wheel_notches_are_one_press_each_and_motion_goes_stale() {
        let _ = mouse_wheel_take();
        note_mouse_wheel(1.0);
        assert_eq!(mouse_wheel_take(), 1);
        assert_eq!(mouse_wheel_take(), 0);
        note_mouse_wheel(-1.0);
        assert_eq!(mouse_wheel_take(), 2);

        set_mouse_captured(true);
        let _ = mouse_motion_take(1_000);
        note_mouse_motion(4.0, -8.0);
        assert_eq!(mouse_motion_take(1_100), (4.0, -8.0));
        note_mouse_motion(1.0, 1.0);
        assert_eq!(
            mouse_motion_take(1_500),
            (0.0, 0.0),
            "stale motion is dropped"
        );
        set_mouse_captured(false);
    }
}
