//! The map controller chord state machine, ported from
//! `legacy/gpu/shim/bbport_map_controller.h` and `bbport_map_controller.cpp`.

use std::sync::atomic::{AtomicBool, Ordering};

pub const HOLD_DURATION_MS: u64 = 650;

const LEFT_SHOULDER: u8 = 1 << 0;
const RIGHT_SHOULDER: u8 = 1 << 1;
const START: u8 = 1 << 2;
const ALL_BUTTONS: u8 = LEFT_SHOULDER | RIGHT_SHOULDER | START;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamepadButton {
    LeftShoulder,
    RightShoulder,
    Start,
    Other(u8),
}

/// One SDL gamepad event, reduced to the fields the chord uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerEvent {
    WindowFocusLost,
    GamepadRemoved {
        which: u32,
    },
    ButtonDown {
        which: u32,
        button: GamepadButton,
        /// SDL event timestamp (nanoseconds), on the same clock as `tick`'s milliseconds.
        timestamp_ns: u64,
    },
    ButtonUp {
        which: u32,
        button: GamepadButton,
        timestamp_ns: u64,
    },
}

const fn chord_bit(button: GamepadButton) -> u8 {
    match button {
        GamepadButton::LeftShoulder => LEFT_SHOULDER,
        GamepadButton::RightShoulder => RIGHT_SHOULDER,
        GamepadButton::Start => START,
        GamepadButton::Other(_) => 0,
    }
}

pub struct BbMapControllerChord {
    controller: u32,
    held: u8,
    started_ms: u64,
    started: bool,
    enabled: bool,
    fired: bool,
    capture_latched: bool,
    blocked_until_release: bool,
    capture: AtomicBool,
}

impl Default for BbMapControllerChord {
    fn default() -> Self {
        Self {
            controller: 0,
            held: 0,
            started_ms: 0,
            started: false,
            enabled: true,
            fired: false,
            capture_latched: false,
            blocked_until_release: false,
            capture: AtomicBool::new(false),
        }
    }
}

impl BbMapControllerChord {
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn is_complete(&self) -> bool {
        self.held == ALL_BUTTONS
    }

    fn refresh_capture(&self) {
        self.capture.store(
            self.enabled && (self.is_complete() || self.capture_latched),
            Ordering::Release,
        );
    }

    /// `BbMapControllerChord::HandleEvent`: `true` when the overlay should consume the event.
    pub fn handle_event(&mut self, event: ControllerEvent) -> bool {
        match event {
            ControllerEvent::WindowFocusLost => {
                self.reset();
                false
            }
            ControllerEvent::GamepadRemoved { which } => {
                if self.controller != 0 && which == self.controller {
                    self.reset();
                }
                false
            }
            ControllerEvent::ButtonDown {
                which,
                button,
                timestamp_ns,
            } => {
                let bit = chord_bit(button);
                if bit == 0 || !self.enabled {
                    return false;
                }
                // The first chord button claims the sequence; another pad cannot disturb it.
                if self.controller == 0 {
                    self.controller = which;
                }
                if which != self.controller {
                    return false;
                }
                let was_complete = self.is_complete();
                self.held |= bit;
                if self.is_complete() {
                    self.capture_latched = true;
                }
                if !self.blocked_until_release && !self.started && self.is_complete() {
                    // SDL event timestamps and SDL_GetTicks share the monotonic clock.
                    self.started_ms = timestamp_ns / 1_000_000;
                    self.started = true;
                }
                self.refresh_capture();
                was_complete || self.is_complete() || self.capture_latched
            }
            ControllerEvent::ButtonUp { which, button, .. } => {
                if self.controller != 0 && which != self.controller {
                    return false;
                }
                let bit = chord_bit(button);
                let was_complete = self.is_complete() || self.capture_latched;
                self.held &= !bit;
                if !self.is_complete() && !self.fired && was_complete {
                    // A short/aborted attempt cannot re-arm until every chord button is released.
                    self.blocked_until_release = true;
                    self.started = false;
                    self.started_ms = 0;
                }
                if self.held == 0 {
                    self.controller = 0;
                    self.started = false;
                    self.started_ms = 0;
                    self.fired = false;
                    self.capture_latched = false;
                    self.blocked_until_release = false;
                }
                self.refresh_capture();
                was_complete
            }
        }
    }

    /// `Tick`: `true` exactly once when the complete chord has been held long enough.
    pub fn tick(&mut self, now_ms: u64) -> bool {
        if !self.enabled || !self.started || self.fired || !self.is_complete() {
            return false;
        }
        if now_ms < self.started_ms || now_ms - self.started_ms < HOLD_DURATION_MS {
            return false;
        }
        self.fired = true;
        true
    }

    pub fn reset(&mut self) {
        self.held = 0;
        self.controller = 0;
        self.started_ms = 0;
        self.started = false;
        self.fired = false;
        self.capture_latched = false;
        self.blocked_until_release = false;
        self.capture.store(false, Ordering::Release);
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled {
            self.refresh_capture();
        } else {
            self.reset();
        }
    }

    pub const fn is_arming(&self) -> bool {
        self.enabled && self.started && self.is_complete() && !self.fired
    }

    pub fn wants_capture(&self) -> bool {
        self.capture.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOWN: fn(GamepadButton, u64) -> ControllerEvent =
        |button, ns| ControllerEvent::ButtonDown {
            which: 1,
            button,
            timestamp_ns: ns,
        };
    const UP: fn(GamepadButton) -> ControllerEvent = |button| ControllerEvent::ButtonUp {
        which: 1,
        button,
        timestamp_ns: 0,
    };

    #[test]
    fn a_held_chord_fires_once_after_the_hold_duration() {
        let mut chord = BbMapControllerChord::new();
        assert!(!chord.handle_event(DOWN(GamepadButton::LeftShoulder, 0)));
        assert!(!chord.handle_event(DOWN(GamepadButton::RightShoulder, 0)));
        assert!(chord.handle_event(DOWN(GamepadButton::Start, 5_000_000_000)));
        assert!(chord.is_arming());
        assert!(chord.wants_capture());
        // The chord went down at 5_000 ms (the event timestamp converted to ms).
        assert!(!chord.tick(5_000));
        assert!(chord.tick(5_000 + HOLD_DURATION_MS));
        assert!(!chord.tick(6_000));
        assert!(!chord.is_arming());
    }

    #[test]
    fn an_aborted_chord_blocks_rearming_until_every_button_is_released() {
        let mut chord = BbMapControllerChord::new();
        chord.handle_event(DOWN(GamepadButton::LeftShoulder, 0));
        chord.handle_event(DOWN(GamepadButton::RightShoulder, 0));
        chord.handle_event(DOWN(GamepadButton::Start, 0));
        chord.handle_event(UP(GamepadButton::Start));
        assert!(!chord.is_arming());
        chord.handle_event(DOWN(GamepadButton::Start, 1_000_000_000));
        chord.handle_event(UP(GamepadButton::Start));
        assert!(!chord.is_arming());
        chord.handle_event(UP(GamepadButton::LeftShoulder));
        chord.handle_event(UP(GamepadButton::RightShoulder));
        assert_eq!(chord.held, 0);
        chord.handle_event(DOWN(GamepadButton::LeftShoulder, 2_000_000_000));
        chord.handle_event(DOWN(GamepadButton::RightShoulder, 2_000_000_000));
        chord.handle_event(DOWN(GamepadButton::Start, 2_000_000_000));
        assert!(chord.is_arming());
    }

    #[test]
    fn another_controller_and_focus_loss_do_not_complete_the_chord() {
        let mut chord = BbMapControllerChord::new();
        chord.handle_event(DOWN(GamepadButton::LeftShoulder, 0));
        let other = ControllerEvent::ButtonDown {
            which: 2,
            button: GamepadButton::RightShoulder,
            timestamp_ns: 0,
        };
        assert!(!chord.handle_event(other));
        chord.handle_event(DOWN(GamepadButton::RightShoulder, 0));
        chord.handle_event(DOWN(GamepadButton::Start, 0));
        assert!(chord.is_complete());
        chord.handle_event(ControllerEvent::WindowFocusLost);
        assert!(!chord.is_complete());
        assert!(!chord.wants_capture());
    }

    #[test]
    fn a_removed_controller_resets_only_its_own_chord() {
        let mut chord = BbMapControllerChord::new();
        chord.handle_event(DOWN(GamepadButton::LeftShoulder, 0));
        chord.handle_event(ControllerEvent::GamepadRemoved { which: 2 });
        assert!(!chord.is_complete());
        assert!(chord.held != 0);
        chord.handle_event(ControllerEvent::GamepadRemoved { which: 1 });
        assert_eq!(chord.held, 0);
    }
}
