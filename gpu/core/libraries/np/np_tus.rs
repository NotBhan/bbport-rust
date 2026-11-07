//! NpTus/NpTss types, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_tus/np_tus.h`.

use crate::gpu::bbnet::include::core::libraries::np::np_types::{
    OrbisNpAccountId, OrbisNpId, OrbisNpOnlineId, OrbisNpServiceLabel,
};
use crate::gpu::bbnet::include::core::libraries::rtc::rtc::OrbisRtcTick;

/// `OrbisNpTusOpeType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum OrbisNpTusOpeType {
    Equal = 1,
    NotEqual = 2,
    GreaterThan = 3,
    GreaterOrEqual = 4,
    LessThan = 5,
    LessOrEqual = 6,
}

/// `OrbisNpTusDataStatusSortType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum OrbisNpTusDataStatusSortType {
    DescendingDate = 1,
    AscendingDate = 2,
}

/// `OrbisNpTusVariableSortType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum OrbisNpTusVariableSortType {
    DescendingDate = 1,
    AscendingDate = 2,
    DescendingValue = 3,
    AscendingValue = 4,
}

/// `OrbisNpTusVirtualUserId`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpTusVirtualUserId {
    pub data: [u8; 16],
    pub term: u8,
    pub dummy: [u8; 3],
}

/// `OrbisNpTssStatus`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(i32)]
pub enum OrbisNpTssStatus {
    #[default]
    Ok = 0,
    Partial = 1,
    NotModified = 2,
}

/// `OrbisNpTssDataStatus`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpTssDataStatus {
    pub modified: OrbisRtcTick,
    pub status: OrbisNpTssStatus,
    pub content_length: u64,
}

/// `OrbisNpTssIfType`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(i32)]
pub enum OrbisNpTssIfType {
    #[default]
    IfModifiedSince = 0,
    IfRange = 1,
}

/// `OrbisNpTssIfModifiedSinceParam`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpTssIfModifiedSinceParam {
    pub if_type: OrbisNpTssIfType,
    pub padding: [u8; 4],
    pub last_modified: OrbisRtcTick,
}

/// `OrbisNpTssGetDataOptParam`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNpTssGetDataOptParam {
    pub size: u64,
    pub offset: *mut u64,
    pub last_byte: *mut u64,
    pub if_param: *mut OrbisNpTssIfModifiedSinceParam,
}

/// `OrbisNpTusGetFriendsVariableOptParam`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNpTusGetFriendsVariableOptParam {
    pub size: u64,
    pub start_serial_rank: *mut u32,
    pub hits: *mut u32,
}

/// `OrbisNpTusGetFriendsDataStatusOptParam`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNpTusGetFriendsDataStatusOptParam {
    pub size: u64,
    pub start_serial_rank: *mut u32,
    pub hits: *mut u32,
}

/// `OrbisNpTusDataInfo`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbisNpTusDataInfo {
    pub size: u64,
    pub data: [u8; 384],
}

impl Default for OrbisNpTusDataInfo {
    fn default() -> Self {
        Self {
            size: 0,
            data: [0; 384],
        }
    }
}

/// `OrbisNpTusDataStatus`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpTusDataStatus {
    pub owner_id: OrbisNpId,
    pub has_data: i32,
    pub last_changed_date: OrbisRtcTick,
    pub last_changed_author_id: OrbisNpId,
    pub pad: [u8; 4],
    pub data: *mut core::ffi::c_void,
    pub data_size: u64,
    pub info: OrbisNpTusDataInfo,
}

/// `OrbisNpTusDataStatusA`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpTusDataStatusA {
    pub owner_id: OrbisNpOnlineId,
    pub reserved1: [u8; 16],
    pub has_data: i32,
    pub last_changed_date: OrbisRtcTick,
    pub last_changed_author_id: OrbisNpOnlineId,
    pub reserved2: [u8; 20],
    pub data: *mut core::ffi::c_void,
    pub data_size: u64,
    pub info: OrbisNpTusDataInfo,
    pub owner_account_id: OrbisNpAccountId,
    pub last_changed_author_account_id: OrbisNpAccountId,
    pub reserved: [u8; 16],
}

/// `OrbisNpTusVariable`.
#[repr(C, packed(4))]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpTusVariable {
    pub owner_id: OrbisNpId,
    pub has_data: i32,
    pub pad: [u8; 4],
    pub last_changed_date: OrbisRtcTick,
    pub last_changed_author_id: OrbisNpId,
    pub variable: i64,
    pub old_variable: i64,
    pub owner_account_id: OrbisNpAccountId,
    pub last_changed_author_account_id: OrbisNpAccountId,
}

/// `OrbisNpTusVariableA`.
#[repr(C, packed(4))]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpTusVariableA {
    pub owner_id: OrbisNpOnlineId,
    pub reserved1: [u8; 16],
    pub has_data: i32,
    pub last_changed_date: OrbisRtcTick,
    pub pad: [u8; 4],
    pub last_changed_author_id: OrbisNpOnlineId,
    pub reserved2: [u8; 16],
    pub variable: i64,
    pub old_variable: i64,
    pub owner_account_id: OrbisNpAccountId,
    pub last_changed_author_account_id: OrbisNpAccountId,
}

/// `OrbisNpTusVariableForCrossSave`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpTusVariableForCrossSave {
    pub owner_id: OrbisNpId,
    pub has_data: i32,
    pub last_changed_date: OrbisRtcTick,
    pub pad: [u8; 4],
    pub last_changed_author_id: OrbisNpId,
    pub variable: i64,
    pub old_variable: i64,
    pub owner_account_id: OrbisNpAccountId,
    pub last_changed_author_account_id: OrbisNpAccountId,
}

/// `OrbisNpTusDataStatusForCrossSave`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpTusDataStatusForCrossSave {
    pub owner_id: OrbisNpId,
    pub has_data: i32,
    pub last_changed_date: OrbisRtcTick,
    pub last_changed_author_id: OrbisNpId,
    pub pad: [u8; 4],
    pub data: *mut core::ffi::c_void,
    pub data_size: u64,
    pub info: OrbisNpTusDataInfo,
    pub owner_account_id: OrbisNpAccountId,
    pub last_changed_author_account_id: OrbisNpAccountId,
    pub reserved: [u8; 16],
}

/// The Tus/Tss entry points; the HLE loader implements them.
pub trait NpTusApi {
    fn create_np_title_ctx(&mut self, service_label: OrbisNpServiceLabel, np_id: &OrbisNpId)
        -> i32;
    fn delete_np_title_ctx(&mut self, ctx_id: i32) -> i32;
    fn create_request(&mut self, lib_ctx_id: i32) -> i32;
    fn delete_request(&mut self, request_id: i32) -> i32;
    fn abort_request(&mut self, request_id: i32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tus_structs_match_the_guest_layouts() {
        assert_eq!(core::mem::size_of::<OrbisNpTusVirtualUserId>(), 20);
        assert_eq!(core::mem::size_of::<OrbisNpTssDataStatus>(), 24);
        assert_eq!(core::mem::size_of::<OrbisNpTssIfModifiedSinceParam>(), 16);
        assert_eq!(core::mem::size_of::<OrbisNpTssGetDataOptParam>(), 32);
        assert_eq!(
            core::mem::size_of::<OrbisNpTusGetFriendsVariableOptParam>(),
            24
        );
        assert_eq!(core::mem::size_of::<OrbisNpTusDataInfo>(), 392);
        assert_eq!(core::mem::size_of::<OrbisNpTusDataStatus>(), 496);
        assert_eq!(core::mem::size_of::<OrbisNpTusDataStatusA>(), 528);
        assert_eq!(core::mem::size_of::<OrbisNpTusVariable>(), 120);
        assert_eq!(core::mem::size_of::<OrbisNpTusVariableA>(), 120);
        assert_eq!(core::mem::size_of::<OrbisNpTusVariableForCrossSave>(), 120);
        assert_eq!(
            core::mem::size_of::<OrbisNpTusDataStatusForCrossSave>(),
            528
        );
        assert_eq!(OrbisNpTusOpeType::GreaterOrEqual as i32, 4);
        assert_eq!(OrbisNpTssStatus::NotModified as i32, 2);
    }
}

/// TUS/TSS title-context and request state.
#[derive(Debug, Default)]
pub struct TusState {
    pub next_title_ctx: i32,
    pub next_request: i32,
    pub title_ctxs: std::collections::HashSet<i32>,
    pub requests: std::collections::HashSet<i32>,
}

impl TusState {
    pub fn create_title_ctx(&mut self) -> i32 {
        self.next_title_ctx += 1;
        self.title_ctxs.insert(self.next_title_ctx);
        self.next_title_ctx
    }

    pub fn create_request(&mut self) -> i32 {
        self.next_request += 1;
        self.requests.insert(self.next_request);
        self.next_request
    }

    pub fn delete_request(&mut self, id: i32) -> i32 {
        if self.requests.remove(&id) {
            0
        } else {
            -1
        }
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn tus_requests_are_allocated_and_deleted() {
        let mut state = TusState::default();
        let ctx = state.create_title_ctx();
        let request = state.create_request();
        assert!(state.title_ctxs.contains(&ctx));
        assert_eq!(state.delete_request(request), 0);
        assert_eq!(state.delete_request(request), -1);
    }
}
