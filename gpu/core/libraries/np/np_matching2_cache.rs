//! NpMatching2 room/member cache, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_matching2/np_matching2_cache.h`.

use std::collections::HashMap;

use crate::gpu::bbnet::include::core::libraries::np::np_types::{
    OrbisNpAccountId, OrbisNpId, OrbisNpOnlineId,
};
use crate::gpu::core::libraries::np::np_types2::*;

/// `PeerInfo`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PeerInfo {
    pub addr: u32,
    pub port: u16,
    pub member_id: OrbisNpMatching2RoomMemberId,
    pub conn_id: i32,
    pub status: i32,
    pub online_id: OrbisNpOnlineId,
    pub sent_established: bool,
    pub active_since_us: i64,
    pub ping_us: u32,
}

/// `MemberBinCache`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemberBinCache {
    pub id: OrbisNpMatching2AttributeId,
    pub update_date: u64,
    pub data: Vec<u8>,
}

/// `MemberCache`.
#[derive(Debug, Clone, Default)]
pub struct MemberCache {
    pub np_id: OrbisNpId,
    pub account_id: OrbisNpAccountId,
    pub platform: u32,
    pub join_date: u64,
    pub member_id: OrbisNpMatching2RoomMemberId,
    pub team_id: OrbisNpMatching2TeamId,
    pub group_id: OrbisNpMatching2RoomGroupId,
    pub nat_type: OrbisNpMatching2NatType,
    pub flag_attr: OrbisNpMatching2Flags,
    pub addr: u32,
    pub port: u16,
    pub bins: HashMap<OrbisNpMatching2AttributeId, MemberBinCache>,
}

/// `RoomCache`.
#[derive(Debug, Clone)]
pub struct RoomCache {
    pub num_slots: u32,
    pub mask_password: u64,
    pub signaling_type: OrbisNpMatching2SignalingType,
    pub signaling_main_member: OrbisNpMatching2RoomMemberId,
    pub server_id: OrbisNpMatching2ServerId,
    pub world_id: OrbisNpMatching2WorldId,
    pub lobby_id: OrbisNpMatching2LobbyId,
    pub room_id: OrbisNpMatching2RoomId,
    pub max_slot: u16,
    pub public_slots: u16,
    pub private_slots: u16,
    pub open_public_slots: u16,
    pub open_private_slots: u16,
    pub passwd_slot_mask: u64,
    pub joined_slot_mask: u64,
    pub flags: OrbisNpMatching2Flags,
    pub bin_attrs_internal: Vec<OrbisNpMatching2RoomBinAttrInternal>,
    pub groups: HashMap<OrbisNpMatching2RoomGroupId, OrbisNpMatching2RoomGroup>,
    pub members: HashMap<OrbisNpMatching2RoomMemberId, MemberCache>,
    pub owner: bool,
}

impl Default for RoomCache {
    fn default() -> Self {
        Self {
            num_slots: 0,
            mask_password: 0,
            signaling_type: ORBIS_NP_MATCHING2_SIGNALING_TYPE_MESH,
            signaling_main_member: 0,
            server_id: 0,
            world_id: 0,
            lobby_id: 0,
            room_id: 0,
            max_slot: 0,
            public_slots: 0,
            private_slots: 0,
            open_public_slots: 0,
            open_private_slots: 0,
            passwd_slot_mask: 0,
            joined_slot_mask: 0,
            flags: 0,
            bin_attrs_internal: Vec::new(),
            groups: HashMap::new(),
            members: HashMap::new(),
            owner: false,
        }
    }
}

/// `Matching2ContextCache`.
#[derive(Debug, Default)]
pub struct Matching2ContextCache {
    pub peers: HashMap<OrbisNpMatching2RoomMemberId, PeerInfo>,
    pub rooms: HashMap<OrbisNpMatching2RoomId, RoomCache>,
}

impl Matching2ContextCache {
    pub fn reset(&mut self) {
        self.peers.clear();
        self.rooms.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_caches_default_to_mesh_and_reset() {
        let mut cache = Matching2ContextCache::default();
        cache.peers.insert(1, PeerInfo::default());
        cache.rooms.insert(7, RoomCache::default());
        assert_eq!(
            cache.rooms[&7].signaling_type,
            ORBIS_NP_MATCHING2_SIGNALING_TYPE_MESH
        );
        cache.reset();
        assert!(cache.peers.is_empty());
        assert!(cache.rooms.is_empty());
        let member = MemberCache::default();
        assert_eq!(member.port, 0);
        assert!(member.bins.is_empty());
    }
}
