//! NP manager types, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_manager.h`.

use crate::gpu::bbnet::include::core::libraries::np::np_types::OrbisNpId;
use crate::gpu::core::libraries::system::userservice::OrbisUserServiceUserId;

pub const ORBIS_NP_MANAGER_REQUEST_LIMIT: i32 = 0x20;
pub const ORBIS_NP_MANAGER_REQUEST_ID_OFFSET: i32 = 0x2000_0000;

/// `OrbisNpState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OrbisNpState {
    NotSignedUp = 0,
    SignedOut = 1,
    SignedIn = 2,
}

/// `OrbisNpReachabilityState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OrbisNpReachabilityState {
    Unavailable = 0,
    Available = 1,
    Reachable = 2,
}

pub type OrbisNpStateCallback = unsafe extern "C" fn(
    OrbisUserServiceUserId,
    OrbisNpState,
    *mut OrbisNpId,
    *mut core::ffi::c_void,
);
pub type OrbisNpStateCallbackA =
    unsafe extern "C" fn(OrbisUserServiceUserId, OrbisNpState, *mut core::ffi::c_void);
pub type OrbisNpStateCallbackForNpToolkit =
    unsafe extern "C" fn(OrbisUserServiceUserId, OrbisNpState, *mut core::ffi::c_void);
pub type OrbisNpReachabilityStateCallback =
    unsafe extern "C" fn(OrbisUserServiceUserId, OrbisNpReachabilityState, *mut core::ffi::c_void);

/// `OrbisNpGamePresenseStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OrbisNpGamePresenseStatus {
    Offline = 0,
    Online = 1,
}

/// `OrbisNpCountryCode`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpCountryCode {
    pub country_code: [u8; 2],
    pub end: u8,
    pub pad: u8,
}

/// `OrbisNpAgeRestriction`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpAgeRestriction {
    pub country_code: OrbisNpCountryCode,
    pub age: i8,
    pub padding: [u8; 3],
}

/// `OrbisNpContentRestriction`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNpContentRestriction {
    pub size: u64,
    pub default_age_restriction: i8,
    pub padding: [u8; 3],
    pub age_restriction_count: i32,
    pub age_restriction: *const OrbisNpAgeRestriction,
}

/// `OrbisNpDate`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

/// `OrbisNpLanguageCode`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpLanguageCode {
    pub code: [u8; 6],
    pub padding: [u8; 10],
}

/// `OrbisNpParentalControlInfo`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpParentalControlInfo {
    pub content_restriction: bool,
    pub chat_restriction: bool,
    pub user_generated_content_restriction: bool,
}

/// `OrbisNpCheckPlusParameter`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpCheckPlusParameter {
    pub size: u64,
    pub user_id: OrbisUserServiceUserId,
    pub padding: [u8; 4],
    pub features: u64,
    pub reserved: [u8; 32],
}

/// `OrbisNpCheckPlusResult`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpCheckPlusResult {
    pub authorized: bool,
    pub reserved: [u8; 32],
}

/// `OrbisNpCreateAsyncRequestParameter`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpCreateAsyncRequestParameter {
    pub size: u64,
    pub cpu_affinity_mask: u64,
    pub thread_priority: i32,
    pub padding: [u8; 4],
}

/// The registerable callback type.
pub type NpCallback = Box<dyn Fn() + Send>;

/// The manager entry points; the HLE loader implements them.
pub trait NpManagerApi {
    fn register_callback(&mut self, key: String, callback: NpCallback);
    fn deregister_callback(&mut self, key: &str);
    fn get_np_id(&mut self, user_id: OrbisUserServiceUserId, np_id: &mut OrbisNpId) -> i32;
    fn get_account_id(&mut self, user_id: OrbisUserServiceUserId, account_id: &mut u64) -> i32;
    fn get_online_id(
        &mut self,
        user_id: OrbisUserServiceUserId,
        online_id: &mut crate::gpu::bbnet::include::core::libraries::np::np_types::OrbisNpOnlineId,
    ) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manager_types_match_the_header() {
        assert_eq!(ORBIS_NP_MANAGER_REQUEST_LIMIT, 0x20);
        assert_eq!(OrbisNpState::SignedIn as u32, 2);
        assert_eq!(OrbisNpReachabilityState::Reachable as u32, 2);
        assert_eq!(OrbisNpGamePresenseStatus::Online as u32, 1);
        assert_eq!(core::mem::size_of::<OrbisNpCountryCode>(), 4);
        assert_eq!(core::mem::size_of::<OrbisNpAgeRestriction>(), 8);
        assert_eq!(core::mem::size_of::<OrbisNpContentRestriction>(), 24);
        assert_eq!(core::mem::size_of::<OrbisNpDate>(), 4);
        assert_eq!(core::mem::size_of::<OrbisNpLanguageCode>(), 16);
        assert_eq!(core::mem::size_of::<OrbisNpParentalControlInfo>(), 3);
        assert_eq!(core::mem::size_of::<OrbisNpCheckPlusParameter>(), 56);
        assert_eq!(core::mem::size_of::<OrbisNpCheckPlusResult>(), 33);
        assert_eq!(
            core::mem::size_of::<OrbisNpCreateAsyncRequestParameter>(),
            24
        );
    }
}

/// `NpManager` context state.
#[derive(Debug, Default)]
pub struct NpManagerState {
    pub initialized: bool,
    pub next_title_ctx_id: i32,
    pub title_ctxs: std::collections::HashMap<i32, u64>,
    pub account_ids: std::collections::HashMap<i32, u64>,
}

impl NpManagerState {
    pub fn initialize(&mut self) -> i32 {
        self.initialized = true;
        self.next_title_ctx_id = 1;
        0
    }

    pub fn terminate(&mut self) -> i32 {
        self.initialized = false;
        self.title_ctxs.clear();
        self.account_ids.clear();
        0
    }

    pub fn create_title_ctx(&mut self, account_id: u64) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        let id = self.next_title_ctx_id;
        self.next_title_ctx_id += 1;
        self.title_ctxs.insert(id, account_id);
        Some(id)
    }

    pub fn delete_title_ctx(&mut self, id: i32) -> bool {
        self.title_ctxs.remove(&id).is_some()
    }

    pub fn get_account_id(&self, user_id: i32) -> u64 {
        self.account_ids.get(&user_id).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn manager_allocates_and_deletes_title_contexts() {
        let mut manager = NpManagerState::default();
        assert!(manager.create_title_ctx(42).is_none());
        assert_eq!(manager.initialize(), 0);
        assert_eq!(manager.create_title_ctx(42), Some(1));
        assert!(manager.delete_title_ctx(1));
        assert!(!manager.delete_title_ctx(1));
        assert_eq!(manager.terminate(), 0);
    }
}
