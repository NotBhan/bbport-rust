//! Session invitation event types, ported from
//! `bbnet/include/core/libraries/invitation_dialog/invitation_dialog.h`.

use crate::gpu::bbnet::include::core::libraries::np::np_types::{
    OrbisNpAccountId, OrbisNpOnlineId,
};

pub const ORBIS_NP_SESSION_ID_MAX_SIZE: i32 = 45;
pub const ORBIS_NP_INVITATION_ID_SIZE: i32 = 60;

/// `OrbisNpSessionId`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbisNpSessionId {
    pub data: [u8; ORBIS_NP_SESSION_ID_MAX_SIZE as usize],
    pub term: u8,
    pub padding: [u8; 2],
}

/// `OrbisNpInvitationId`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbisNpInvitationId {
    pub data: [u8; ORBIS_NP_INVITATION_ID_SIZE as usize],
    pub term: u8,
}

/// `OrbisNpSessionInvitationEventFlag`.
pub type OrbisNpSessionInvitationEventFlag = i32;
pub const ORBIS_NP_SESSION_INVITATION_EVENT_FLAG_INVITATION: i32 = 0x01;

/// `OrbisNpSessionInvitationEventParam`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbisNpSessionInvitationEventParam {
    pub session_id: OrbisNpSessionId,
    pub invitation_id: OrbisNpInvitationId,
    pub flag: OrbisNpSessionInvitationEventFlag,
    pub padding: [u8; 4],
    pub online_id: OrbisNpOnlineId,
    pub user_id: i32,
    pub referral_online_id: OrbisNpOnlineId,
    pub referral_account_id: OrbisNpAccountId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_invitation_parameter_offsets_match_the_static_asserts() {
        assert_eq!(
            core::mem::offset_of!(OrbisNpSessionInvitationEventParam, user_id),
            140
        );
        assert_eq!(
            core::mem::offset_of!(OrbisNpSessionInvitationEventParam, referral_online_id),
            144
        );
        assert_eq!(
            core::mem::offset_of!(OrbisNpSessionInvitationEventParam, referral_account_id),
            168
        );
        assert_eq!(
            core::mem::size_of::<OrbisNpSessionInvitationEventParam>(),
            176
        );
        assert_eq!(
            core::mem::size_of::<OrbisNpSessionInvitationEventParam>() % 8,
            0
        );
    }
}
