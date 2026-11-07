//! NP Web API 2 types, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_web_api2/np_web_api2.h`.

use crate::gpu::bbnet::include::core::libraries::np::np_types::{
    OrbisNpOnlineId, OrbisNpPeerAddressA, OrbisNpServiceLabel,
};

/// `OrbisNpWebApi2IntInitializeArgs`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApi2IntInitializeArgs {
    pub lib_http_ctx_id: i32,
    pub reserved: i32,
    pub pool_size: u64,
    pub name: *const u8,
    pub struct_size: u64,
}

/// `OrbisNpWebApi2IntInitialize2Args`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApi2IntInitialize2Args {
    pub lib_http_ctx_id: i32,
    pub reserved: i32,
    pub pool_size: u64,
    pub name: *const u8,
    pub push_config_group: u32,
    pub reserved2: i32,
    pub struct_size: u64,
}

/// `OrbisNpWebApi2ContentParameter`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApi2ContentParameter {
    pub content_length: u64,
    pub content_type: *const u8,
    pub reserved: [u8; 16],
}

/// `OrbisNpWebApi2MemoryPoolStats`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpWebApi2MemoryPoolStats {
    pub pool_size: u64,
    pub max_inuse_size: u64,
    pub current_inuse_size: u64,
    pub reserved: i32,
}

/// `OrbisNpWebApi2ResponseInformationOption`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApi2ResponseInformationOption {
    pub http_status: i32,
    pub reserved: i32,
    pub error_object: *mut u8,
    pub error_object_size: u64,
    pub response_data_size: u64,
}

/// `OrbisNpWebApi2PushEventDataType`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApi2PushEventDataType {
    pub val: [u8; 65],
}

/// `OrbisNpWebApi2PushEventExtdDataKey`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApi2PushEventExtdDataKey {
    pub val: [u8; 33],
}

/// `OrbisNpWebApi2PushEventFilterParameter`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApi2PushEventFilterParameter {
    pub data_type: OrbisNpWebApi2PushEventDataType,
    pub extd_data_key: *mut OrbisNpWebApi2PushEventExtdDataKey,
    pub extd_data_key_num: u64,
}

/// `OrbisNpWebApi2PushEventExtdData`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApi2PushEventExtdData {
    pub extd_data_key: OrbisNpWebApi2PushEventExtdDataKey,
    pub data: *mut u8,
    pub data_len: u64,
}

pub type OrbisNpWebApi2PushEventCallback = unsafe extern "C" fn(
    i32,
    i32,
    *const u8,
    OrbisNpServiceLabel,
    *const OrbisNpPeerAddressA,
    *const OrbisNpOnlineId,
    *const OrbisNpPeerAddressA,
    *const OrbisNpOnlineId,
    *const OrbisNpWebApi2PushEventDataType,
    *const u8,
    u64,
    *const OrbisNpWebApi2PushEventExtdData,
    u64,
    *mut core::ffi::c_void,
);

/// `OrbisNpWebApi2PushEventPushContextCallbackType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum OrbisNpWebApi2PushEventPushContextCallbackType {
    Unknown = -1,
    Received = 0,
    Dropped = 1,
}

/// `OrbisNpWebApi2PushEventPushContextId`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpWebApi2PushEventPushContextId {
    pub uuid: [u8; 37],
}

impl Default for OrbisNpWebApi2PushEventPushContextId {
    fn default() -> Self {
        Self { uuid: [0; 37] }
    }
}

pub type OrbisNpWebApi2PushEventPushContextCallback = unsafe extern "C" fn(
    i32,
    i32,
    *const OrbisNpWebApi2PushEventPushContextId,
    OrbisNpWebApi2PushEventPushContextCallbackType,
    *const u8,
    OrbisNpServiceLabel,
    *const OrbisNpPeerAddressA,
    *const OrbisNpOnlineId,
    *const OrbisNpPeerAddressA,
    *const OrbisNpOnlineId,
    *const OrbisNpWebApi2PushEventDataType,
    *const u8,
    u64,
    *const OrbisNpWebApi2PushEventExtdData,
    u64,
    *mut core::ffi::c_void,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_web_api2_types_match_the_header() {
        assert_eq!(core::mem::size_of::<OrbisNpWebApi2IntInitializeArgs>(), 32);
        assert_eq!(core::mem::size_of::<OrbisNpWebApi2IntInitialize2Args>(), 40);
        assert_eq!(core::mem::size_of::<OrbisNpWebApi2ContentParameter>(), 32);
        assert_eq!(core::mem::size_of::<OrbisNpWebApi2MemoryPoolStats>(), 32);
        assert_eq!(
            core::mem::size_of::<OrbisNpWebApi2ResponseInformationOption>(),
            32
        );
        assert_eq!(core::mem::size_of::<OrbisNpWebApi2PushEventDataType>(), 65);
        assert_eq!(
            core::mem::size_of::<OrbisNpWebApi2PushEventExtdDataKey>(),
            33
        );
        assert_eq!(
            core::mem::size_of::<OrbisNpWebApi2PushEventFilterParameter>(),
            88
        );
        assert_eq!(core::mem::size_of::<OrbisNpWebApi2PushEventExtdData>(), 56);
        assert_eq!(
            core::mem::size_of::<OrbisNpWebApi2PushEventPushContextId>(),
            37
        );
        assert_eq!(
            OrbisNpWebApi2PushEventPushContextCallbackType::Dropped as i32,
            1
        );
    }
}

/// NpWebApi2 library/user context and push-handle state.
#[derive(Debug, Default)]
pub struct WebApi2State {
    pub initialized: bool,
    pub next_lib_ctx: i32,
    pub next_user_ctx: i32,
    pub next_handle: i32,
    pub next_request: i64,
    pub lib_contexts: std::collections::HashSet<i32>,
    pub user_contexts: std::collections::HashSet<i32>,
    pub handles: std::collections::HashSet<i32>,
    pub requests: std::collections::HashSet<i64>,
}

impl WebApi2State {
    pub fn initialize(&mut self) -> i32 {
        self.initialized = true;
        self.next_lib_ctx = 1;
        self.next_user_ctx = 1;
        self.next_handle = 1;
        self.next_request = 1;
        0
    }

    pub fn terminate(&mut self) -> i32 {
        self.initialized = false;
        self.lib_contexts.clear();
        self.user_contexts.clear();
        self.handles.clear();
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

    pub fn create_push_handle(&mut self) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        let id = self.next_handle;
        self.next_handle += 1;
        self.handles.insert(id);
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
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn webapi2_contexts_and_handles_are_tracked() {
        let mut state = WebApi2State::default();
        assert!(state.create_library_context().is_none());
        assert_eq!(state.initialize(), 0);
        assert_eq!(state.create_library_context(), Some(1));
        assert_eq!(state.create_user_context(), Some(1));
        assert_eq!(state.create_push_handle(), Some(1));
        assert_eq!(state.create_request(), Some(1));
        assert_eq!(state.terminate(), 0);
    }
}
