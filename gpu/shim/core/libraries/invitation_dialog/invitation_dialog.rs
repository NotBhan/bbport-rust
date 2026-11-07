//! The session invitation event types from `legacy/gpu/shim/core/libraries/invitation_dialog.h`.
//! The dialog itself is not ported; co-op uses bells, not invitations.

pub const ORBIS_NP_SESSION_ID_MAX_SIZE: usize = 45;
pub const ORBIS_NP_INVITATION_ID_SIZE: usize = 60;
pub const ORBIS_NP_SESSION_INVITATION_EVENT_FLAG_INVITATION: i32 = 0x01;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpSessionId {
    pub data: [std::ffi::c_char; ORBIS_NP_SESSION_ID_MAX_SIZE],
    pub term: std::ffi::c_char,
    pub padding: [std::ffi::c_char; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpInvitationId {
    pub data: [std::ffi::c_char; ORBIS_NP_INVITATION_ID_SIZE],
    pub term: std::ffi::c_char,
}

/// `Libraries::Np::OrbisNpOnlineId` (`np_types.h`): 16 bytes of id + 2 bytes of term.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpOnlineId {
    pub data: [std::ffi::c_char; 16],
    pub term: std::ffi::c_char,
    pub padding: [std::ffi::c_char; 1],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpSessionInvitationEventParam {
    pub session_id: OrbisNpSessionId,
    pub invitation_id: OrbisNpInvitationId,
    pub flag: i32,
    pub padding: [std::ffi::c_char; 4],
    pub online_id: OrbisNpOnlineId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn event_param_matches_the_c_static_assert() {
        assert_eq!(size_of::<OrbisNpSessionInvitationEventParam>(), 0x8c);
        assert_eq!(size_of::<OrbisNpSessionId>(), 48);
        assert_eq!(size_of::<OrbisNpInvitationId>(), 61);
    }
}
