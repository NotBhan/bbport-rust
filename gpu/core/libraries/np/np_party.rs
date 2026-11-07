//! NpParty stubs, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_party.cpp`.

use crate::gpu::core::libraries::error_codes::ORBIS_OK;
use crate::gpu::core::libraries::np::np_error::ORBIS_NP_PARTY_ERROR_NOT_IN_PARTY;

/// `OrbisNpPartyState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum OrbisNpPartyState {
    NotInParty = 2,
    InPrivateParty = 3,
}

/// The NpParty entry points; every request is stubbed to its no-party answer.
pub trait NpPartyApi {
    fn check_callback(&mut self) -> i32;
    fn create(&mut self) -> i32;
    fn create_a(&mut self) -> i32;
    fn get_id(&mut self) -> i32;
    fn get_member_info(&mut self) -> i32;
    fn get_member_info_a(&mut self) -> i32;
    fn get_members(&mut self) -> i32;
    fn get_members_a(&mut self) -> i32;
    fn get_member_session_info(&mut self) -> i32;
    fn get_member_voice_info(&mut self) -> i32;
    fn get_state(&mut self) -> OrbisNpPartyState;
    fn get_state_as_user(&mut self) -> i32;
    fn get_state_as_user_a(&mut self) -> i32;
    fn get_voice_chat_priority(&mut self) -> i32;
    fn initialize(&mut self) -> i32;
    fn join(&mut self) -> i32;
    fn leave(&mut self) -> i32;
    fn register_handler(&mut self) -> i32;
    fn register_handler_a(&mut self) -> i32;
    fn register_private_handler(&mut self) -> i32;
    fn send_binary_message(&mut self) -> i32;
    fn set_voice_chat_priority(&mut self) -> i32;
    fn show_invitation_list(&mut self) -> i32;
    fn show_invitation_list_a(&mut self) -> i32;
    fn terminate(&mut self) -> i32;
    fn unregister_private_handler(&mut self) -> i32;
}

/// A stub implementation that follows the C++ return values.
#[derive(Debug, Default)]
pub struct NpParty;

impl NpPartyApi for NpParty {
    fn check_callback(&mut self) -> i32 {
        ORBIS_OK
    }
    fn create(&mut self) -> i32 {
        ORBIS_OK
    }
    fn create_a(&mut self) -> i32 {
        ORBIS_OK
    }
    fn get_id(&mut self) -> i32 {
        ORBIS_OK
    }
    fn get_member_info(&mut self) -> i32 {
        ORBIS_NP_PARTY_ERROR_NOT_IN_PARTY
    }
    fn get_member_info_a(&mut self) -> i32 {
        ORBIS_NP_PARTY_ERROR_NOT_IN_PARTY
    }
    fn get_members(&mut self) -> i32 {
        ORBIS_NP_PARTY_ERROR_NOT_IN_PARTY
    }
    fn get_members_a(&mut self) -> i32 {
        ORBIS_NP_PARTY_ERROR_NOT_IN_PARTY
    }
    fn get_member_session_info(&mut self) -> i32 {
        ORBIS_OK
    }
    fn get_member_voice_info(&mut self) -> i32 {
        ORBIS_OK
    }
    fn get_state(&mut self) -> OrbisNpPartyState {
        OrbisNpPartyState::NotInParty
    }
    fn get_state_as_user(&mut self) -> i32 {
        ORBIS_OK
    }
    fn get_state_as_user_a(&mut self) -> i32 {
        ORBIS_OK
    }
    fn get_voice_chat_priority(&mut self) -> i32 {
        ORBIS_OK
    }
    fn initialize(&mut self) -> i32 {
        ORBIS_OK
    }
    fn join(&mut self) -> i32 {
        ORBIS_OK
    }
    fn leave(&mut self) -> i32 {
        ORBIS_OK
    }
    fn register_handler(&mut self) -> i32 {
        ORBIS_OK
    }
    fn register_handler_a(&mut self) -> i32 {
        ORBIS_OK
    }
    fn register_private_handler(&mut self) -> i32 {
        ORBIS_OK
    }
    fn send_binary_message(&mut self) -> i32 {
        ORBIS_OK
    }
    fn set_voice_chat_priority(&mut self) -> i32 {
        ORBIS_OK
    }
    fn show_invitation_list(&mut self) -> i32 {
        ORBIS_OK
    }
    fn show_invitation_list_a(&mut self) -> i32 {
        ORBIS_OK
    }
    fn terminate(&mut self) -> i32 {
        ORBIS_OK
    }
    fn unregister_private_handler(&mut self) -> i32 {
        ORBIS_OK
    }
}

/// The `RegisterLib` export names.
pub const EXPORTS: [&str; 26] = [
    "3e4k2mzLkmc",
    "nOZRy-slBoA",
    "XQSUbbnpPBA",
    "DRA3ay-1DFQ",
    "F1P+-wpxQow",
    "v2RYVGrJDkM",
    "T2UOKf00ZN0",
    "TaNw7W25QJw",
    "4gOMfNYzllw",
    "EKi1jx59SP4",
    "aEzKdJzATZ0",
    "o7grRhiGHYI",
    "EjyAI+QNgFw",
    "-lc6XZnQXvM",
    "lhYCTQmBkds",
    "RXNCDw2GDEg",
    "J8jAi-tfJHc",
    "kA88gbv71ao",
    "+v4fVHMwFWc",
    "zo4G5WWYpKg",
    "U6VdUe-PNAY",
    "nazKyHygHhY",
    "-MFiL7hEnPE",
    "yARHEYLajs0",
    "oLYkibiHqRA",
    "zQ7gIvt11Pc",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn party_stubs_report_no_party() {
        let mut party = NpParty;
        assert_eq!(party.initialize(), ORBIS_OK);
        assert_eq!(party.get_state(), OrbisNpPartyState::NotInParty);
        assert_eq!(party.get_member_info(), ORBIS_NP_PARTY_ERROR_NOT_IN_PARTY);
        assert_eq!(party.get_members(), ORBIS_NP_PARTY_ERROR_NOT_IN_PARTY);
        assert_eq!(party.terminate(), ORBIS_OK);
        assert_eq!(EXPORTS.len(), 26);
    }
}
