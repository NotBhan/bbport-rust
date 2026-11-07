//! The Hunter's Dream matchmaking backend, ported from `bbnet/thd/thd_matching2.cpp`.

use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThdRoom {
    pub room_id: u64,
    pub world_id: u32,
    pub lobby_id: u64,
    pub max_slots: u16,
    pub members: HashMap<u16, String>,
    pub next_member: u16,
}

/// `thd_matching2.cpp` state.
#[derive(Debug, Default)]
pub struct ThdMatching2State {
    pub next_room: u64,
    pub rooms: HashMap<u64, ThdRoom>,
    pub contexts: HashMap<u32, u64>,
}

impl ThdMatching2State {
    pub fn create_room(&mut self, world: u32, lobby: u64, max_slots: u16) -> u64 {
        self.next_room += 1;
        self.rooms.insert(
            self.next_room,
            ThdRoom {
                room_id: self.next_room,
                world_id: world,
                lobby_id: lobby,
                max_slots,
                ..ThdRoom::default()
            },
        );
        self.next_room
    }

    pub fn join_room(&mut self, room_id: u64, online_id: &str) -> Option<u16> {
        let room = self.rooms.get_mut(&room_id)?;
        if room.members.len() >= room.max_slots as usize {
            return None;
        }
        room.next_member += 1;
        room.members.insert(room.next_member, online_id.to_string());
        Some(room.next_member)
    }

    pub fn leave_room(&mut self, room_id: u64, member_id: u16) -> bool {
        self.rooms
            .get_mut(&room_id)
            .is_some_and(|room| room.members.remove(&member_id).is_some())
    }

    pub fn join_lobby(&mut self, ctx_id: u32, lobby_id: u64) {
        self.contexts.insert(ctx_id, lobby_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rooms_join_leave_and_lobbies_are_tracked() {
        let mut state = ThdMatching2State::default();
        let room = state.create_room(1, 2, 1);
        let member = state.join_room(room, "a").unwrap();
        assert!(state.join_room(room, "b").is_none());
        assert!(state.leave_room(room, member));
        assert!(!state.leave_room(room, member));
        state.join_lobby(1, 2);
        assert_eq!(state.contexts[&1], 2);
    }
}
