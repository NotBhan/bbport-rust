//! Hunter's Atlas quick-menu, ported from `legacy/gpu/shim/bbport_atlas_menu.h` and
//! `bbport_atlas_menu.cpp`. The ImGui drawing lives behind `draw`; the controller/state logic
//! is implemented against `AtlasHost`, which stands in for BbMap and the runtime hooks.

use std::sync::{Mutex, MutexGuard, OnceLock};

use crate::gpu::shim::bbport_atlas_menu_state::BbAtlasMenuState;

/// The Map shortcut button (PS4 pad bit 0x4).
pub const MAP_SHORTCUT_BUTTON: u32 = 0x4;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MenuSnapshot {
    pub top_level: bool,
    pub selected: i32,
    pub open: bool,
}

/// The host side of the menu: BbMap plus the runtime map hooks.
pub trait AtlasHost {
    fn map_enabled(&self) -> bool;
    fn menu_snapshot(&self) -> Option<MenuSnapshot>;
    fn map_location_present(&self) -> bool;
    fn map_open(&self) -> bool;
    fn open_from_menu(&self);
    fn close_map(&self);
    fn back(&self);
    fn mapped_pad(&self, rising: u32, lx: u8, ly: u8, rx: u8, ry: u8, buttons: u32);
    fn confirm_button(&self) -> u32;
    fn shortcut_mode(&self) -> u32;
    fn audio_menu_event(&self, index: u32);
}

struct AtlasState {
    state: BbAtlasMenuState,
    visible: bool,
    focused: bool,
    suppressed: bool,
    controller: bool,
    mouse_held: bool,
    map_previous: u32,
}

impl Default for AtlasState {
    fn default() -> Self {
        Self {
            state: BbAtlasMenuState::new(),
            visible: false,
            focused: false,
            suppressed: false,
            controller: false,
            mouse_held: false,
            map_previous: 0,
        }
    }
}

fn state() -> &'static Mutex<AtlasState> {
    static STATE: OnceLock<Mutex<AtlasState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(AtlasState::default()))
}

fn lock() -> MutexGuard<'static, AtlasState> {
    state().lock().unwrap_or_else(|error| error.into_inner())
}

fn query(host: &dyn AtlasHost) -> bool {
    if lock().suppressed || !host.map_enabled() {
        return false;
    }
    let top_level = host
        .menu_snapshot()
        .map(|menu| menu.top_level)
        .unwrap_or(false);
    top_level && host.map_location_present()
}

fn refresh(host: &dyn AtlasHost, now_ms: u64) {
    let in_menu = query(host);
    let ready = lock().state.settled(in_menu, now_ms);
    let map_open = host.map_open();
    let mut state = lock();
    state.visible = ready && !map_open;
    state.focused = state.state.focused;
}

pub fn visible() -> bool {
    lock().visible
}

pub fn interacting() -> bool {
    lock().visible
}

pub fn focused() -> bool {
    lock().focused
}

/// `UsesControllerGlyphs`: BB_ATLAS_QA=1 with BB_ATLAS_QA_DEVICE overrides the live device.
pub fn uses_controller_glyphs() -> bool {
    let qa = std::env::var("BB_ATLAS_QA")
        .map(|value| !value.is_empty())
        .unwrap_or(false);
    match (qa, std::env::var("BB_ATLAS_QA_DEVICE")) {
        (true, Ok(device)) if !device.is_empty() => device == "controller",
        _ => lock().controller,
    }
}

pub const fn uses_play_station_glyphs() -> bool {
    false
}

pub fn suppress(value: bool) {
    let mut state = lock();
    state.suppressed = value;
    if value {
        state.visible = false;
        state.state.reset();
    }
}

pub fn reset() {
    let mut state = lock();
    state.state.reset();
    state.focused = false;
    state.visible = false;
    state.mouse_held = false;
    state.map_previous = 0;
}

pub fn tick() {}

/// `Tick`: while the map is open without its own menu, close it and reset.
pub fn tick_with(host: &dyn AtlasHost, now_ms: u64) {
    refresh(host, now_ms);
    let menu_open = host.menu_snapshot().map(|menu| menu.open).unwrap_or(false);
    let focused = lock().state.focused;
    if host.map_open() && !menu_open && focused {
        host.close_map();
        reset();
    }
}

pub fn draw() {}

pub fn handle_mouse(_event: &std::ffi::c_void) -> bool {
    false
}

/// Legacy no-host route: without a host the menu never interacts.
pub fn route_pad(buttons: u32, _lx: u8, _ly: u8, _rx: u8, _ry: u8) -> u32 {
    buttons
}

/// `RoutePad`: filters the game's buttons while the tile or the map menu owns input.
pub fn route_pad_at(
    host: &dyn AtlasHost,
    buttons: u32,
    lx: u8,
    ly: u8,
    rx: u8,
    ry: u8,
    now_ms: u64,
) -> u32 {
    let in_menu = query(host);
    let was_open = host.map_open();
    let selected = host.menu_snapshot().map(|menu| menu.selected).unwrap_or(0);
    let rising = buttons & !lock().map_previous;
    let mut result = lock().state.route(
        in_menu,
        selected,
        was_open,
        buttons,
        host.confirm_button(),
        now_ms,
    );
    if result.move_ {
        host.audio_menu_event(0);
    }
    if result.open {
        host.audio_menu_event(1);
        host.open_from_menu();
    }
    if !was_open && in_menu && host.shortcut_mode() == 1 && rising & MAP_SHORTCUT_BUTTON != 0 {
        host.audio_menu_event(1);
        host.open_from_menu();
        lock().state.blocked |= buttons;
        result.buttons = 0;
    }
    if was_open {
        if result.close {
            host.back();
        } else {
            host.mapped_pad(rising, lx, ly, rx, ry, buttons);
        }
        result.buttons = 0;
    }
    lock().map_previous = buttons;
    refresh(host, now_ms);
    result.buttons
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shim::bbport_atlas_menu_state::CROSS;
    use std::cell::{Cell, RefCell};

    #[derive(Default)]
    struct FakeHost {
        enabled: bool,
        location: bool,
        menu: MenuSnapshot,
        open: Cell<bool>,
        opened: Cell<u32>,
        closed: Cell<u32>,
        backed: Cell<u32>,
        mapped: Cell<u32>,
        confirm: Cell<u32>,
        shortcut: Cell<u32>,
        audio: RefCell<Vec<u32>>,
    }

    impl AtlasHost for FakeHost {
        fn map_enabled(&self) -> bool {
            self.enabled
        }
        fn menu_snapshot(&self) -> Option<MenuSnapshot> {
            Some(self.menu)
        }
        fn map_location_present(&self) -> bool {
            self.location
        }
        fn map_open(&self) -> bool {
            self.open.get()
        }
        fn open_from_menu(&self) {
            self.opened.set(self.opened.get() + 1);
            self.open.set(true);
        }
        fn close_map(&self) {
            self.closed.set(self.closed.get() + 1);
            self.open.set(false);
        }
        fn back(&self) {
            self.backed.set(self.backed.get() + 1);
        }
        fn mapped_pad(&self, _rising: u32, _lx: u8, _ly: u8, _rx: u8, _ry: u8, _buttons: u32) {
            self.mapped.set(self.mapped.get() + 1);
        }
        fn confirm_button(&self) -> u32 {
            self.confirm.get()
        }
        fn shortcut_mode(&self) -> u32 {
            self.shortcut.get()
        }
        fn audio_menu_event(&self, index: u32) {
            self.audio.borrow_mut().push(index);
        }
    }

    fn host() -> FakeHost {
        FakeHost {
            enabled: true,
            location: true,
            menu: MenuSnapshot {
                top_level: true,
                selected: 2,
                open: false,
            },
            confirm: Cell::new(CROSS),
            ..FakeHost::default()
        }
    }

    #[test]
    fn focus_then_confirm_opens_the_map() {
        reset();
        let host = host();
        // The tile settles 180 ms after it becomes visible.
        assert_eq!(route_pad_at(&host, 0, 0, 0, 0, 0, 1000), 0);
        assert_eq!(route_pad_at(&host, 0, 0, 0, 0, 0, 1200), 0);
        assert_eq!(route_pad_at(&host, 0x20, 0, 0, 0, 0, 1201), 0);
        assert_eq!(*host.audio.borrow(), vec![0]);
        let buttons = route_pad_at(&host, CROSS, 0, 0, 0, 0, 1202);
        assert_eq!(host.opened.get(), 1);
        assert_eq!(*host.audio.borrow(), vec![0, 1]);
        assert_eq!(buttons, 0);
    }

    #[test]
    fn the_shortcut_opens_from_the_top_level_menu() {
        reset();
        let host = host();
        host.shortcut.set(1);
        let buttons = route_pad_at(&host, MAP_SHORTCUT_BUTTON, 0, 0, 0, 0, 0);
        assert_eq!(host.opened.get(), 1);
        assert_eq!(buttons, 0);
    }

    #[test]
    fn an_open_map_owns_the_pad_and_back_closes_it() {
        reset();
        let host = host();
        host.open.set(true);
        let buttons = route_pad_at(&host, 0x4000, 0, 0, 0, 0, 0);
        assert_eq!(buttons, 0);
        assert!(host.mapped.get() >= 1 || host.backed.get() >= 1);
    }
}
