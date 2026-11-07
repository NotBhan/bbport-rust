//! NP authentication parameters, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_auth.h`.

use crate::gpu::bbnet::include::core::libraries::np::np_types::{
    OrbisNpClientId, OrbisNpClientSecret, OrbisNpOnlineId,
};
use crate::gpu::core::libraries::system::userservice::OrbisUserServiceUserId;

pub const ORBIS_NP_AUTH_REQUEST_LIMIT: i32 = 0x10;
pub const ORBIS_NP_AUTH_REQUEST_ID_OFFSET: i32 = 0x1000_0000;

/// `OrbisNpAuthCreateAsyncRequestParameter`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpAuthCreateAsyncRequestParameter {
    pub size: u64,
    pub cpu_affinity_mask: u64,
    pub thread_priority: i32,
    pub padding: [u8; 4],
}

/// `OrbisNpAuthGetAuthorizationCodeParameter`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNpAuthGetAuthorizationCodeParameter {
    pub size: u64,
    pub online_id: *const OrbisNpOnlineId,
    pub client_id: *const OrbisNpClientId,
    pub scope: *const i8,
}

/// `OrbisNpAuthGetAuthorizationCodeParameterA`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNpAuthGetAuthorizationCodeParameterA {
    pub size: u64,
    pub user_id: OrbisUserServiceUserId,
    pub padding: [u8; 4],
    pub client_id: *const OrbisNpClientId,
    pub scope: *const i8,
}

/// `OrbisNpAuthGetIdTokenParameter`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNpAuthGetIdTokenParameter {
    pub size: u64,
    pub online_id: *const OrbisNpOnlineId,
    pub client_id: *const OrbisNpClientId,
    pub client_secret: *const OrbisNpClientSecret,
    pub scope: *const i8,
}

/// `OrbisNpAuthGetIdTokenParameterA`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNpAuthGetIdTokenParameterA {
    pub size: u64,
    pub user_id: OrbisUserServiceUserId,
    pub padding: [u8; 4],
    pub client_id: *const OrbisNpClientId,
    pub client_secret: *const OrbisNpClientSecret,
    pub scope: *const i8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_auth_parameters_match_the_guest_layouts() {
        assert_eq!(
            core::mem::size_of::<OrbisNpAuthCreateAsyncRequestParameter>(),
            24
        );
        assert_eq!(
            core::mem::size_of::<OrbisNpAuthGetAuthorizationCodeParameter>(),
            32
        );
        assert_eq!(
            core::mem::size_of::<OrbisNpAuthGetAuthorizationCodeParameterA>(),
            32
        );
        assert_eq!(core::mem::size_of::<OrbisNpAuthGetIdTokenParameter>(), 40);
        assert_eq!(core::mem::size_of::<OrbisNpAuthGetIdTokenParameterA>(), 40);
        assert_eq!(ORBIS_NP_AUTH_REQUEST_LIMIT, 0x10);
    }
}

/// `NpAuth` request/context state.
#[derive(Debug, Default)]
pub struct NpAuthState {
    pub initialized: bool,
    pub next_context_id: i32,
    pub next_request_id: i32,
    pub contexts: std::collections::HashMap<i32, i32>,
    pub requests: std::collections::HashMap<i32, i32>,
}

impl NpAuthState {
    pub fn initialize(&mut self) -> i32 {
        self.initialized = true;
        self.next_context_id = 1;
        self.next_request_id = 1;
        0
    }

    pub fn terminate(&mut self) -> i32 {
        self.initialized = false;
        self.contexts.clear();
        self.requests.clear();
        0
    }

    pub fn create_context(&mut self) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        let id = self.next_context_id;
        self.next_context_id += 1;
        self.contexts.insert(id, id);
        Some(id)
    }

    pub fn create_request(&mut self) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        let id = self.next_request_id;
        self.next_request_id += 1;
        self.requests.insert(id, 0);
        Some(id)
    }

    pub fn delete_request(&mut self, id: i32) -> bool {
        self.requests.remove(&id).is_some()
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn auth_contexts_and_requests_are_allocated_after_init() {
        let mut auth = NpAuthState::default();
        assert!(auth.create_context().is_none());
        assert_eq!(auth.initialize(), 0);
        assert_eq!(auth.create_context(), Some(1));
        assert_eq!(auth.create_request(), Some(1));
        assert!(auth.delete_request(1));
        assert_eq!(auth.terminate(), 0);
        assert!(auth.create_request().is_none());
    }
}
