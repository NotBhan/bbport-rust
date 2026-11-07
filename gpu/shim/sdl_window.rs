//! The game window, ported from `legacy/gpu/shim/sdl_window.h`. SDL is not linked in the Rust
//! port yet; this keeps the window state and the event/text-entry surface.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum WindowSystemType {
    #[default]
    Headless,
    Windows,
    X11,
    Wayland,
    Metal,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WindowSystemInfo {
    pub display_connection: *mut std::ffi::c_void,
    pub render_surface: *mut std::ffi::c_void,
    pub render_surface_scale: f32,
    pub kind: WindowSystemType,
}

impl Default for WindowSystemInfo {
    fn default() -> Self {
        Self {
            display_connection: std::ptr::null_mut(),
            render_surface: std::ptr::null_mut(),
            render_surface_scale: 1.0,
            kind: WindowSystemType::Headless,
        }
    }
}

pub struct WindowSdl {
    width: AtomicI32,
    height: AtomicI32,
    is_open: AtomicBool,
    on_hdr_display: AtomicBool,
    text: Mutex<String>,
    window_info: WindowSystemInfo,
}

impl WindowSdl {
    pub fn new(width: i32, height: i32, _title: &str) -> Self {
        Self {
            width: AtomicI32::new(width),
            height: AtomicI32::new(height),
            is_open: AtomicBool::new(true),
            on_hdr_display: AtomicBool::new(false),
            text: Mutex::new(String::new()),
            window_info: WindowSystemInfo::default(),
        }
    }

    pub fn width(&self) -> i32 {
        self.width.load(Ordering::Relaxed)
    }
    pub fn height(&self) -> i32 {
        self.height.load(Ordering::Relaxed)
    }
    pub const fn sdl_window(&self) -> *mut std::ffi::c_void {
        std::ptr::null_mut()
    }
    pub const fn window_info(&self) -> WindowSystemInfo {
        self.window_info
    }
    pub fn is_open(&self) -> bool {
        self.is_open.load(Ordering::Relaxed)
    }
    pub fn is_on_hdr_display(&self) -> bool {
        self.on_hdr_display.load(Ordering::Relaxed)
    }
    pub fn poll_events(&self) -> bool {
        self.is_open.load(Ordering::Relaxed)
    }
    pub fn begin_text_input(&self, initial: &str, _prompt: &str) {
        if let Ok(mut text) = self.text.lock() {
            *text = initial.to_string();
        }
    }
    pub fn poll_text_input(&self, out: &mut String) -> i32 {
        if let Ok(text) = self.text.lock() {
            *out = text.clone();
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_state_is_reported() {
        let window = WindowSdl::new(1280, 720, "Bloodborne");
        assert_eq!(window.width(), 1280);
        assert_eq!(window.height(), 720);
        assert!(window.is_open());
        assert!(window.poll_events());
        assert_eq!(window.window_info().kind, WindowSystemType::Headless);
    }
}
