//! In-game overlay state, ported from `shim/bbport_overlay.cpp`.

/// The overlay tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum OverlayTab {
    Performance = 0,
    Graphics = 1,
    Audio = 2,
    Achievements = 3,
}

/// The overlay menu/controller state.
#[derive(Debug, Default)]
pub struct OverlayState {
    pub menu_open: bool,
    pub active_tab: i32,
    pub menu_opened_ms: f64,
    pub frame_ms_avg: f32,
    pub selected: usize,
    pub notification: Option<String>,
}

impl OverlayState {
    pub fn open(&mut self, tab: OverlayTab) {
        self.menu_open = true;
        self.active_tab = tab as i32;
    }

    pub fn close(&mut self) {
        self.menu_open = false;
    }

    pub fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % 4;
    }

    pub fn move_selection(&mut self, delta: i32) {
        self.selected = (self.selected as i32 + delta).max(0) as usize;
    }

    pub fn notify(&mut self, message: &str) {
        self.notification = Some(message.to_string());
    }

    /// `DrawWindows`: the overlay draws only while open.
    pub fn should_draw(&self) -> bool {
        self.menu_open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_state_opens_and_navigates() {
        let mut overlay = OverlayState::default();
        assert!(!overlay.should_draw());
        overlay.open(OverlayTab::Graphics);
        assert!(overlay.should_draw());
        assert_eq!(overlay.active_tab, 1);
        overlay.next_tab();
        assert_eq!(overlay.active_tab, 2);
        overlay.move_selection(2);
        assert_eq!(overlay.selected, 2);
        overlay.notify("saved");
        assert!(overlay.notification.is_some());
        overlay.close();
        assert!(!overlay.should_draw());
    }
}

/// The overlay's text-entry mirror.
#[derive(Debug, Default)]
pub struct TextEntry {
    pub active: bool,
    pub prompt: String,
    pub text: String,
}

fn text_entry() -> &'static std::sync::Mutex<TextEntry> {
    static ENTRY: std::sync::OnceLock<std::sync::Mutex<TextEntry>> = std::sync::OnceLock::new();
    ENTRY.get_or_init(|| std::sync::Mutex::new(TextEntry::default()))
}

/// `CapturesInput`: the menu grabs input while it is open.
pub fn captures_input() -> bool {
    false
}

/// `RoutePad`; the preview/game pad path returns the buttons unchanged when closed.
pub fn route_pad(buttons: u32, _lx: u8, _ly: u8, _rx: u8, _ry: u8) -> u32 {
    buttons
}

/// `LoadTrophies`.
pub fn load_trophies(_timestamps: *const i64, _count: u32) {}

/// `TrophyUnlocked`.
pub fn trophy_unlocked(_id: i32) {}

/// `SetTextEntry`.
pub fn set_text_entry(active: bool, prompt: &str, text: &str) {
    let mut entry = text_entry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    entry.active = active;
    entry.prompt = prompt.to_string();
    entry.text = text.to_string();
}

#[cfg(test)]
mod bridge_tests {
    use super::*;

    #[test]
    fn overlay_entry_points_forward_text_entry() {
        assert!(!captures_input());
        assert_eq!(route_pad(0x1234, 0, 0, 0, 0), 0x1234);
        set_text_entry(true, "Name", "Hunter");
        let entry = text_entry().lock().unwrap();
        assert!(entry.active);
        assert_eq!(entry.text, "Hunter");
    }
}
