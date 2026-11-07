//! NP Web API types, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_web_api/np_web_api.h`.

use core::ffi::c_void;

use crate::gpu::bbnet::include::core::libraries::np::np_types::{
    OrbisNpAccountId, OrbisNpOnlineId, OrbisNpPeerAddressA, OrbisNpServiceLabel,
};

pub const ORBIS_NP_WEBAPI_DEFAULT_CONNECTION_NUM: u32 = 1;
pub const ORBIS_NP_WEBAPI_MAX_CONNECTION_NUM: u32 = 16;
pub const ORBIS_NP_WEBAPI_PUSH_EVENT_DATA_TYPE_LEN_MAX: usize = 64;
pub const ORBIS_NP_WEBAPI_EXTD_PUSH_EVENT_EXTD_DATA_KEY_LEN_MAX: usize = 32;

/// `OrbisNpWebApiPushEventDataType`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApiPushEventDataType {
    pub val: [u8; ORBIS_NP_WEBAPI_PUSH_EVENT_DATA_TYPE_LEN_MAX + 1],
}

/// `OrbisNpWebApiExtdPushEventExtdDataKey`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApiExtdPushEventExtdDataKey {
    pub val: [u8; ORBIS_NP_WEBAPI_EXTD_PUSH_EVENT_EXTD_DATA_KEY_LEN_MAX + 1],
}

/// `OrbisNpWebApiPushEventFilterParameter`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApiPushEventFilterParameter {
    pub data_type: OrbisNpWebApiPushEventDataType,
}

/// `OrbisNpWebApiServicePushEventFilterParameter`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApiServicePushEventFilterParameter {
    pub data_type: OrbisNpWebApiPushEventDataType,
}

/// `OrbisNpWebApiExtdPushEventFilterParameter`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApiExtdPushEventFilterParameter {
    pub data_type: OrbisNpWebApiPushEventDataType,
    pub extd_data_key: *mut OrbisNpWebApiExtdPushEventExtdDataKey,
    pub extd_data_key_num: u64,
}

/// `OrbisNpWebApiExtdPushEventExtdData`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApiExtdPushEventExtdData {
    pub extd_data_key: OrbisNpWebApiExtdPushEventExtdDataKey,
    pub data: *mut u8,
    pub data_len: u64,
}

/// `OrbisNpWebApiHttpHeader`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApiHttpHeader {
    pub name: *mut u8,
    pub value: *mut u8,
}

/// `OrbisNpWebApiMultipartPartParameter`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApiMultipartPartParameter {
    pub headers: *mut OrbisNpWebApiHttpHeader,
    pub header_num: u64,
    pub content_length: u64,
}

/// `OrbisNpWebApiHttpMethod`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum OrbisNpWebApiHttpMethod {
    Get = 0,
    Post = 1,
    Put = 2,
    Delete = 3,
    Patch = 4,
}

/// `OrbisNpWebApiContentParameter`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApiContentParameter {
    pub content_length: u64,
    pub content_type: *const u8,
    pub reserved: [u8; 16],
}

/// `OrbisNpWebApiResponseInformationOption`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApiResponseInformationOption {
    pub http_status: i32,
    pub error_object: *mut u8,
    pub error_object_size: u64,
    pub response_data_size: u64,
}

/// `OrbisNpWebApiMemoryPoolStats`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpWebApiMemoryPoolStats {
    pub pool_size: u64,
    pub max_inuse_size: u64,
    pub current_inuse_size: u64,
    pub reserved: i32,
}

/// `OrbisNpWebApiConnectionStats`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpWebApiConnectionStats {
    pub max: u32,
    pub used: u32,
    pub unused: u32,
    pub keep_alive: u32,
    pub reserved: u64,
}

/// `OrbisNpWebApiIntInitializeArgs`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApiIntInitializeArgs {
    pub lib_http_ctx_id: u32,
    pub reserved: [u8; 4],
    pub pool_size: u64,
    pub name: *const u8,
    pub struct_size: u64,
}

/// `OrbisNpWebApiIntCreateRequestExtraArgs`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApiIntCreateRequestExtraArgs {
    pub unk_0: *mut c_void,
    pub unk_1: *mut c_void,
    pub unk_2: *mut c_void,
}

pub type OrbisNpWebApiPushEventCallback = unsafe extern "C" fn();
pub type OrbisNpWebApiExtdPushEventCallback = unsafe extern "C" fn();
pub type OrbisNpWebApiExtdPushEventCallbackA = unsafe extern "C" fn(
    i32,
    i32,
    *const u8,
    OrbisNpServiceLabel,
    *const OrbisNpPeerAddressA,
    *const OrbisNpOnlineId,
    *const OrbisNpPeerAddressA,
    *const OrbisNpOnlineId,
    *const OrbisNpWebApiPushEventDataType,
    *const u8,
    u64,
    *const OrbisNpWebApiExtdPushEventExtdData,
    u64,
    *mut c_void,
);
pub type OrbisNpWebApiServicePushEventCallback = unsafe extern "C" fn();
pub type OrbisNpWebApiInternalServicePushEventCallback = unsafe extern "C" fn();
pub type OrbisNpWebApiInternalServicePushEventCallbackA = unsafe extern "C" fn();
pub type OrbisNpWebApiNotificationCallback = unsafe extern "C" fn();

/// `PushEventInput`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PushEventInput {
    pub target_user_id: i32,
    pub np_service_name: String,
    pub np_service_label: OrbisNpServiceLabel,
    pub data_type: String,
    pub data: String,
    pub from_online_id: OrbisNpOnlineId,
    pub has_from: bool,
    pub to_online_id: OrbisNpOnlineId,
    pub has_to: bool,
    pub from_account_id: OrbisNpAccountId,
    pub to_account_id: OrbisNpAccountId,
    pub extd_data: Vec<(String, String)>,
}

/// The Web API entry points; the HLE loader implements them.
pub trait NpWebApi {
    fn enqueue_push_event(&mut self, event: PushEventInput);
    fn initialize(&mut self, lib_http_ctx_id: i32, pool_size: u64) -> i32;
    fn terminate(&mut self, lib_ctx_id: i32) -> i32;
    fn create_context_a(&mut self, lib_ctx_id: i32, user_id: i32) -> i32;
    fn delete_context(&mut self, title_user_ctx_id: i32) -> i32;
    fn create_request(
        &mut self,
        title_user_ctx_id: i32,
        api_group: &str,
        path: &str,
        method: OrbisNpWebApiHttpMethod,
        content: Option<&OrbisNpWebApiContentParameter>,
    ) -> Result<i64, i32>;
    fn delete_request(&mut self, request_id: i64) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_web_api_types_match_the_header() {
        assert_eq!(core::mem::size_of::<OrbisNpWebApiPushEventDataType>(), 65);
        assert_eq!(
            core::mem::size_of::<OrbisNpWebApiExtdPushEventExtdDataKey>(),
            33
        );
        assert_eq!(
            core::mem::size_of::<OrbisNpWebApiPushEventFilterParameter>(),
            65
        );
        assert_eq!(
            core::mem::size_of::<OrbisNpWebApiExtdPushEventFilterParameter>(),
            88
        );
        assert_eq!(
            core::mem::size_of::<OrbisNpWebApiExtdPushEventExtdData>(),
            56
        );
        assert_eq!(core::mem::size_of::<OrbisNpWebApiHttpHeader>(), 16);
        assert_eq!(
            core::mem::size_of::<OrbisNpWebApiMultipartPartParameter>(),
            24
        );
        assert_eq!(core::mem::size_of::<OrbisNpWebApiContentParameter>(), 32);
        assert_eq!(
            core::mem::size_of::<OrbisNpWebApiResponseInformationOption>(),
            32
        );
        assert_eq!(core::mem::size_of::<OrbisNpWebApiMemoryPoolStats>(), 32);
        assert_eq!(core::mem::size_of::<OrbisNpWebApiConnectionStats>(), 24);
        assert_eq!(core::mem::size_of::<OrbisNpWebApiIntInitializeArgs>(), 32);
        assert_eq!(OrbisNpWebApiHttpMethod::Patch as i32, 4);
        assert_eq!(ORBIS_NP_WEBAPI_MAX_CONNECTION_NUM, 16);
    }
}

/// WebAPI context/request state.
#[derive(Debug, Default)]
pub struct WebApiState {
    pub initialized: bool,
    pub next_lib_ctx: i32,
    pub next_user_ctx: i32,
    pub next_request: i64,
    pub lib_contexts: std::collections::HashSet<i32>,
    pub user_contexts: std::collections::HashSet<i32>,
    pub requests: std::collections::HashSet<i64>,
}

impl WebApiState {
    pub fn initialize(&mut self) -> i32 {
        self.initialized = true;
        self.next_lib_ctx = 1;
        self.next_user_ctx = 1;
        self.next_request = 1;
        0
    }

    pub fn terminate(&mut self) -> i32 {
        self.initialized = false;
        self.lib_contexts.clear();
        self.user_contexts.clear();
        self.requests.clear();
        0
    }

    pub fn create_library_context(&mut self) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        let id = self.next_lib_ctx;
        self.next_lib_ctx += 1;
        self.lib_contexts.insert(id);
        Some(id)
    }

    pub fn create_user_context(&mut self) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        let id = self.next_user_ctx;
        self.next_user_ctx += 1;
        self.user_contexts.insert(id);
        Some(id)
    }

    pub fn create_request(&mut self) -> Option<i64> {
        if !self.initialized {
            return None;
        }
        let id = self.next_request;
        self.next_request += 1;
        self.requests.insert(id);
        Some(id)
    }

    pub fn delete_request(&mut self, id: i64) -> bool {
        self.requests.remove(&id)
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn webapi_contexts_and_requests_are_tracked() {
        let mut state = WebApiState::default();
        assert!(state.create_library_context().is_none());
        assert_eq!(state.initialize(), 0);
        assert_eq!(state.create_library_context(), Some(1));
        assert_eq!(state.create_user_context(), Some(1));
        assert_eq!(state.create_request(), Some(1));
        assert!(state.delete_request(1));
        assert!(!state.delete_request(1));
    }
}
