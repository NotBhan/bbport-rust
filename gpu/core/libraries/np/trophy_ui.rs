//! Trophy notification UI state, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/trophy_ui.{h,cpp}`.

use std::collections::VecDeque;

/// `TrophyInfo`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrophyInfo {
    pub trophy_icon_path: String,
    pub trophy_name: String,
    pub trophy_type: String,
}

/// The queue the game pushes trophy unlocks into.
#[derive(Debug, Default)]
pub struct TrophyUiQueue {
    pub pending: VecDeque<TrophyInfo>,
    pub current: Option<TrophyInfo>,
    pub finished: bool,
}

impl TrophyUiQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// `AddTrophyToQueue`.
    pub fn add(&mut self, path: &str, name: &str, rarity: &str) {
        self.pending.push_back(TrophyInfo {
            trophy_icon_path: path.to_string(),
            trophy_name: name.to_string(),
            trophy_type: rarity.to_string(),
        });
    }

    /// Starts the next queued notification.
    pub fn advance(&mut self) -> bool {
        self.current = self.pending.pop_front();
        self.finished = false;
        self.current.is_some()
    }

    /// `Finish`.
    pub fn finish(&mut self) {
        self.finished = true;
    }

    pub fn is_active(&self) -> bool {
        self.current.is_some() && !self.finished
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trophy_notifications_are_queued_and_finished() {
        let mut queue = TrophyUiQueue::new();
        queue.add("icon.png", "Hunter", "bronze");
        queue.add("gold.png", "Legend", "gold");
        assert!(queue.advance());
        assert_eq!(queue.current.as_ref().unwrap().trophy_name, "Hunter");
        assert!(queue.is_active());
        queue.finish();
        assert!(!queue.is_active());
        assert!(queue.advance());
        assert_eq!(queue.current.as_ref().unwrap().trophy_type, "gold");
        assert!(!queue.advance());
    }
}
