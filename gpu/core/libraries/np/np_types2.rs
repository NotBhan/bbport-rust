//! Np matching/signaling core types, ported from the matching and signaling sections of
//! `bbnet/vendor/shadps4/core/libraries/np/np_types2.h`.

use crate::gpu::bbnet::include::core::libraries::np::np_types::{
    OrbisNpAccountId, OrbisNpId, OrbisNpOnlineId, OrbisNpServiceLabel,
};

pub type OrbisNpMatching2AttributeId = u16;
pub type OrbisNpMatching2ContextId = u16;
pub type OrbisNpMatching2Flags = u32;
pub type OrbisNpMatching2LobbyId = u64;
pub type OrbisNpMatching2NatType = u8;
pub type OrbisNpMatching2RequestId = u32;
pub type OrbisNpMatching2RoomGroupId = u8;
pub type OrbisNpMatching2RoomId = u64;
pub type OrbisNpMatching2RoomMemberId = u16;
pub type OrbisNpMatching2ServerId = u16;
pub type OrbisNpMatching2TeamId = u8;
pub type OrbisNpMatching2WorldId = u32;
pub type OrbisNpMatching2CastType = u8;
pub type OrbisNpMatching2SignalingType = u8;

pub const ORBIS_NP_MATCHING2_SIGNALING_TYPE_MESH: u8 = 0;
pub const ORBIS_NP_MATCHING2_SIGNALING_TYPE_STAR: u8 = 1;

/// `OrbisNpMatching2Event`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u16)]
pub enum OrbisNpMatching2Event {
    #[default]
    ContextStart = 0,
    ContextStop = 1,
    ContextError = 2,
    RequestError = 3,
    RoomEvent = 10,
    RoomMessage = 11,
    LobbyEvent = 20,
    LobbyMessage = 21,
    SignalingEvent = 30,
}

/// `OrbisNpMatching2EventCause`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum OrbisNpMatching2EventCause {
    #[default]
    None = 0,
}

/// `OrbisNpMatching2RoomGroup`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpMatching2RoomGroup {
    pub group_id: OrbisNpMatching2RoomGroupId,
    pub reserve: u8,
    pub group_label: [u8; 8],
    pub num_slots: u32,
    pub num_members: u32,
}

/// `OrbisNpMatching2RoomBinAttrInternal`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpMatching2RoomBinAttrInternal {
    pub id: OrbisNpMatching2AttributeId,
    pub update_date: u64,
    pub update_member_id: OrbisNpMatching2RoomMemberId,
    pub data: *mut u8,
    pub size: u32,
}

/// `OrbisNpMatching2RoomMemberBinAttrInternal`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpMatching2RoomMemberBinAttrInternal {
    pub update_date: u64,
    pub data: *mut u8,
    pub size: u32,
}

/// `OrbisNpMatching2BinAttr`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpMatching2BinAttr {
    pub id: OrbisNpMatching2AttributeId,
    pub data: *mut u8,
    pub size: u64,
}

/// `OrbisNpMatching2IntAttr`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpMatching2IntAttr {
    pub id: OrbisNpMatching2AttributeId,
    pub value: u32,
}

/// `OrbisNpMatching2RoomGroupInfo`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpMatching2RoomGroupInfo {
    pub group_id: OrbisNpMatching2RoomGroupId,
    pub reserve: u8,
    pub group_label: [u8; 8],
}

/// `OrbisNpMatching2World`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpMatching2World {
    pub world_id: OrbisNpMatching2WorldId,
    pub num_lobbies: u32,
    pub max_lobby_id: OrbisNpMatching2LobbyId,
}

/// `OrbisNpMatching2UserInfo`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpMatching2UserInfo {
    pub np_id: OrbisNpId,
    pub online_id: OrbisNpOnlineId,
}

/// `OrbisNpMatching2UserInfoA`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpMatching2UserInfoA {
    pub account_id: OrbisNpAccountId,
    pub online_id: OrbisNpOnlineId,
}

/// `OrbisNpMatching2CreateContextParameter`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpMatching2CreateContextParameter {
    pub np_id: OrbisNpId,
    pub service_label: OrbisNpServiceLabel,
}

/// `OrbisNpMatching2CreateContextParameterA`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpMatching2CreateContextParameterA {
    pub account_id: OrbisNpAccountId,
    pub service_label: OrbisNpServiceLabel,
    pub padding: [u8; 4],
}

/// `OrbisNpMatching2RequestOptParam`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpMatching2RequestOptParam {
    pub callback: Option<usize>,
    pub user_arg: *mut core::ffi::c_void,
    pub timeout: u32,
    pub padding: u32,
}

/// `OrbisNpMatching2InitializeParameter`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpMatching2InitializeParameter {
    pub pool_size: u64,
    pub thread_priority: i32,
    pub cpu_affinity_mask: i32,
}

pub type OrbisNpMatching2ContextCallback = usize;
pub type OrbisNpMatching2LobbyEventCallback = usize;
pub type OrbisNpMatching2RoomEventCallback = usize;
pub type OrbisNpMatching2SignalingCallback = usize;
pub type OrbisNpMatching2RequestCallback = usize;

/// Signaling aliases and layouts from the same header.
pub type OrbisNpSignalingContextId = i32;
pub type OrbisNpSignalingConnectionId = i32;
pub type OrbisNpSignalingRequestId = u32;
pub type OrbisNpSignalingHandler = usize;

pub const ORBIS_NP_SIGNALING_EVENT_DEAD: i32 = 0;
pub const ORBIS_NP_SIGNALING_EVENT_ESTABLISHED: i32 = 1;
pub const ORBIS_NP_SIGNALING_EVENT_NETINFO_ERROR: i32 = 2;
pub const ORBIS_NP_SIGNALING_EVENT_NETINFO_RESULT: i32 = 3;
pub const ORBIS_NP_SIGNALING_EVENT_PEER_ACTIVATED: i32 = 10;
pub const ORBIS_NP_SIGNALING_EVENT_PEER_DEACTIVATED: i32 = 11;
pub const ORBIS_NP_SIGNALING_EVENT_MUTUAL_ACTIVATED: i32 = 12;
pub const ORBIS_NP_SIGNALING_CONN_STATUS_INACTIVE: i32 = 0;

/// `OrbisNpSignalingNetInfo`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpSignalingNetInfo {
    pub size: u64,
    pub local_addr: u32,
    pub mapped_addr: u32,
    pub nat_status: i32,
    pub pad: u32,
}

/// `OrbisNpSignalingAccountPlatformPair`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpSignalingAccountPlatformPair {
    pub account_id: u64,
    pub platform_type: u32,
    pub pad: u32,
}

/// `OrbisNpSignalingMemoryInfo`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpSignalingMemoryInfo {
    pub current_in_use: u64,
    pub peak_in_use: u64,
    pub max_system_size: u64,
}

/// `OrbisNpSignalingConnectionStatistics`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpSignalingConnectionStatistics {
    pub peak_connection_count: u32,
    pub active_connection_count: u32,
    pub transient_connection_count: u32,
    pub established_connection_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_core_aliases_and_signaling_layouts_match_the_header() {
        assert_eq!(core::mem::size_of::<OrbisNpSignalingNetInfo>(), 0x18);
        assert_eq!(
            core::mem::size_of::<OrbisNpSignalingAccountPlatformPair>(),
            0x10
        );
        assert_eq!(core::mem::size_of::<OrbisNpSignalingMemoryInfo>(), 0x18);
        assert_eq!(
            core::mem::size_of::<OrbisNpSignalingConnectionStatistics>(),
            0x10
        );
        assert_eq!(core::mem::size_of::<OrbisNpMatching2ContextId>(), 2);
        assert_eq!(core::mem::size_of::<OrbisNpMatching2RoomId>(), 8);
        assert_eq!(ORBIS_NP_MATCHING2_SIGNALING_TYPE_MESH, 0);
        assert_eq!(ORBIS_NP_SIGNALING_EVENT_MUTUAL_ACTIVATED, 12);
    }
}
