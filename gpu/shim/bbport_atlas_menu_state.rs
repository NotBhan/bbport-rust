//! Atlas menu input state, ported from `legacy/gpu/shim/bbport_atlas_menu_state.h`.

pub const RIGHT: u32 = 0x20;
pub const UP: u32 = 0x10;
pub const DOWN: u32 = 0x40;
pub const LEFT: u32 = 0x80;
pub const CROSS: u32 = 0x4000;
pub const CIRCLE: u32 = 0x2000;

#[derive(Debug, Clone, Copy, Default)]
pub struct Result {
    pub buttons: u32,
    pub open: bool,
    pub close: bool,
    pub move_: bool,
}

#[derive(Debug, Clone, Default)]
pub struct BbAtlasMenuState {
    pub focused: bool,
    pub anchor: i32,
    pub previous: u32,
    pub blocked: u32,
    pub visible_since: u64,
    pub was_visible: bool,
}

impl BbAtlasMenuState {
    pub fn new() -> Self {
        Self {
            anchor: 2,
            ..Self::default()
        }
    }

    pub fn reset(&mut self) {
        self.focused = false;
        self.anchor = 2;
        self.blocked |= self.previous;
        self.was_visible = false;
        self.visible_since = 0;
    }

    pub fn settled(&mut self, visible: bool, now: u64) -> bool {
        if !visible {
            self.was_visible = false;
            self.visible_since = 0;
            return false;
        }
        if !self.was_visible {
            self.was_visible = true;
            self.visible_since = now;
        }
        now.saturating_sub(self.visible_since) >= 180
    }

    pub fn route(
        &mut self,
        visible: bool,
        index: i32,
        map_open: bool,
        buttons: u32,
        confirm: u32,
        now: u64,
    ) -> Result {
        self.blocked &= buttons;
        let rising = buttons & !self.previous;
        self.previous = buttons;
        let ready = self.settled(visible, now);
        let mut out = Result {
            buttons: buttons & !self.blocked,
            ..Result::default()
        };
        if map_open {
            self.blocked |= buttons;
            if rising & (if confirm == CROSS { CIRCLE } else { CROSS }) != 0 {
                out.close = true;
                self.blocked |= buttons;
            }
            out.buttons = 0;
            return out;
        }
        if !ready {
            self.focused = false;
            return out;
        }
        if self.focused && index != self.anchor {
            self.focused = false;
        }
        if !self.focused
            && ((index == 2 && rising & RIGHT != 0) || (index == 0 && rising & LEFT != 0))
        {
            self.focused = true;
            self.anchor = index;
            self.blocked |= if index == 2 { RIGHT } else { LEFT };
            out.move_ = true;
        } else if self.focused {
            if rising & confirm != 0 {
                out.open = true;
                self.blocked |= buttons;
            } else if rising & (LEFT | RIGHT) != 0 {
                let edge = rising & (LEFT | RIGHT);
                let reveal =
                    (self.anchor == 2 && edge == LEFT) || (self.anchor == 0 && edge == RIGHT);
                self.focused = false;
                if reveal {
                    self.blocked |= edge;
                    out.move_ = true;
                }
            } else if rising & (UP | DOWN | if confirm == CROSS { CIRCLE } else { CROSS }) != 0 {
                self.focused = false;
            }
        }
        out.buttons = buttons & !self.blocked;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settle_requires_180ms_and_route_moves_focus() {
        let mut state = BbAtlasMenuState::new();
        assert!(!state.settled(true, 0));
        assert!(!state.settled(true, 100));
        assert!(state.settled(true, 180));
        let result = state.route(true, 2, false, RIGHT, CROSS, 180);
        assert!(result.move_);
        assert!(state.focused);
        let result = state.route(true, 2, false, CROSS, CROSS, 181);
        assert!(result.open);
    }
}
