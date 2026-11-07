//! The Hunter's Dream session/room backend, ported from `bbnet/thd/thd_session.cpp`.

use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThdSession {
    pub session_id: u64,
    pub owner: String,
    pub max_slots: u32,
    pub open_slots: u32,
    pub members: Vec<String>,
}

/// `thd_session.cpp` state.
#[derive(Debug, Default)]
pub struct ThdSessionState {
    pub next_id: u64,
    pub sessions: HashMap<u64, ThdSession>,
}

impl ThdSessionState {
    pub fn create(&mut self, owner: &str, max_slots: u32) -> u64 {
        self.next_id += 1;
        self.sessions.insert(
            self.next_id,
            ThdSession {
                session_id: self.next_id,
                owner: owner.to_string(),
                max_slots,
                open_slots: max_slots.saturating_sub(1),
                members: vec![owner.to_string()],
            },
        );
        self.next_id
    }

    pub fn join(&mut self, id: u64, member: &str) -> bool {
        let Some(session) = self.sessions.get_mut(&id) else {
            return false;
        };
        if session.open_slots == 0 || session.members.iter().any(|entry| entry == member) {
            return false;
        }
        session.members.push(member.to_string());
        session.open_slots -= 1;
        true
    }

    pub fn leave(&mut self, id: u64, member: &str) -> bool {
        let Some(session) = self.sessions.get_mut(&id) else {
            return false;
        };
        let before = session.members.len();
        session.members.retain(|entry| entry != member);
        if session.members.len() != before {
            session.open_slots += 1;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessions_enforce_slots_and_membership() {
        let mut state = ThdSessionState::default();
        let id = state.create("owner", 2);
        assert!(state.join(id, "a"));
        assert!(!state.join(id, "b"));
        assert!(state.leave(id, "a"));
        assert!(state.join(id, "b"));
        assert!(!state.leave(id, "missing"));
    }
}
