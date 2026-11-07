//! NpScore types, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_score/np_score.h`.

use crate::gpu::bbnet::include::core::libraries::np::np_types::{
    OrbisNpAccountId, OrbisNpId, OrbisNpOnlineId,
};
use crate::gpu::bbnet::include::core::libraries::rtc::rtc::OrbisRtcTick;

pub type OrbisNpScoreTitleCtxId = i32;
pub type OrbisNpScoreRequestId = i32;
pub type OrbisNpScoreBoardId = u32;
pub type OrbisNpScorePcId = i32;
pub type OrbisNpScoreRankNumber = u32;
pub type OrbisNpScoreValue = i64;

pub const ORBIS_NP_SCORE_COMMENT_MAXLEN: usize = 63;
pub const ORBIS_NP_SCORE_GAMEINFO_MAXSIZE: usize = 189;
pub const ORBIS_NP_SCORE_CENSOR_COMMENT_MAXLEN: i32 = 255;
pub const ORBIS_NP_SCORE_SANITIZE_COMMENT_MAXLEN: i32 = 255;
pub const ORBIS_NP_SCORE_NORMAL_UPDATE: i32 = 0;
pub const ORBIS_NP_SCORE_FORCE_UPDATE: i32 = 1;
pub const ORBIS_NP_SCORE_DESCENDING_ORDER: i32 = 0;
pub const ORBIS_NP_SCORE_ASCENDING_ORDER: i32 = 1;
pub const ORBIS_NP_SCORE_MAX_RANGE_NUM_PER_REQUEST: i32 = 100;
pub const ORBIS_NP_SCORE_MAX_ID_NUM_PER_REQUEST: i32 = 101;
pub const ORBIS_NP_SCORE_MAX_SELECTED_FRIENDS_NUM: i32 = 100;
pub const ORBIS_NP_SCORE_MAX_RANGE_NUM_PER_TRANS: i32 = 100;
pub const ORBIS_NP_SCORE_MAX_NPID_NUM_PER_TRANS: i32 = 101;
pub const ORBIS_NP_SCORE_MAX_NPID_NUM_PER_REQUEST: i32 = 101;
pub const ORBIS_NP_SCORE_MAX_CTX_NUM: i32 = 32;
pub const ORBIS_NP_SCORE_BINDMODE_ALL_FORBIDDEN: i32 = 0x0000;
pub const ORBIS_NP_SCORE_BINDMODE_RDONLY: i32 = 0x0001;
pub const ORBIS_NP_SCORE_BINDMODE_WRONLY: i32 = 0x0002;
pub const ORBIS_NP_SCORE_BINDMODE_RDWR: i32 =
    ORBIS_NP_SCORE_BINDMODE_RDONLY | ORBIS_NP_SCORE_BINDMODE_WRONLY;
pub const ORBIS_NP_SCORE_BINDMODE_DEFAULT: i32 = ORBIS_NP_SCORE_BINDMODE_RDWR;

/// `OrbisNpScoreBoardInfo`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpScoreBoardInfo {
    pub rank_limit: u32,
    pub update_mode: u32,
    pub sort_mode: u32,
    pub upload_num_limit: OrbisNpScoreRankNumber,
    pub upload_size_limit: u64,
}

/// `OrbisNpScoreComment`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpScoreComment {
    pub utf8_comment: [u8; ORBIS_NP_SCORE_COMMENT_MAXLEN + 1],
}

/// `OrbisNpScoreGameInfo`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpScoreGameInfo {
    pub info_size: u64,
    pub data: [u8; ORBIS_NP_SCORE_GAMEINFO_MAXSIZE],
    pub pad2: [u8; 3],
}

/// `OrbisNpScoreGetFriendRankingOptParam`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNpScoreGetFriendRankingOptParam {
    pub size: u64,
    pub start_serial_rank: *mut u32,
    pub hits: *mut u32,
}

/// `OrbisNpScoreAccountIdPcId`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpScoreAccountIdPcId {
    pub account_id: OrbisNpAccountId,
    pub pc_id: OrbisNpScorePcId,
    pub pad: [u8; 4],
}

/// `OrbisNpScoreNpIdPcId`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpScoreNpIdPcId {
    pub np_id: OrbisNpId,
    pub pc_id: OrbisNpScorePcId,
    pub pad: [u8; 4],
}

/// `OrbisNpScoreRankData`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbisNpScoreRankData {
    pub np_id: OrbisNpId,
    pub reserved: [u8; 49],
    pub pad0: [u8; 3],
    pub pc_id: OrbisNpScorePcId,
    pub serial_rank: OrbisNpScoreRankNumber,
    pub rank: OrbisNpScoreRankNumber,
    pub highest_rank: OrbisNpScoreRankNumber,
    pub has_game_data: i32,
    pub pad1: [u8; 4],
    pub score_value: OrbisNpScoreValue,
    pub record_date: OrbisRtcTick,
}

/// `OrbisNpScoreRankDataA`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbisNpScoreRankDataA {
    pub online_id: OrbisNpOnlineId,
    pub reserved0: [u8; 16],
    pub reserved: [u8; 49],
    pub pad0: [u8; 3],
    pub pc_id: OrbisNpScorePcId,
    pub serial_rank: OrbisNpScoreRankNumber,
    pub rank: OrbisNpScoreRankNumber,
    pub highest_rank: OrbisNpScoreRankNumber,
    pub has_game_data: i32,
    pub pad1: [u8; 4],
    pub score_value: OrbisNpScoreValue,
    pub record_date: OrbisRtcTick,
    pub account_id: OrbisNpAccountId,
    pub pad2: [u8; 8],
}

/// `OrbisNpScorePlayerRankData`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbisNpScorePlayerRankData {
    pub has_data: i32,
    pub pad0: [u8; 4],
    pub rank_data: OrbisNpScoreRankData,
}

/// `OrbisNpScorePlayerRankDataA`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbisNpScorePlayerRankDataA {
    pub has_data: i32,
    pub pad0: [u8; 4],
    pub rank_data: OrbisNpScoreRankDataA,
}

/// `OrbisNpScoreRankDataForCrossSave`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbisNpScoreRankDataForCrossSave {
    pub np_id: OrbisNpId,
    pub reserved: [u8; 49],
    pub pad0: [u8; 3],
    pub pc_id: OrbisNpScorePcId,
    pub serial_rank: OrbisNpScoreRankNumber,
    pub rank: OrbisNpScoreRankNumber,
    pub highest_rank: OrbisNpScoreRankNumber,
    pub has_game_data: i32,
    pub pad1: [u8; 4],
    pub score_value: OrbisNpScoreValue,
    pub record_date: OrbisRtcTick,
    pub account_id: OrbisNpAccountId,
    pub pad2: [u8; 8],
}

/// `OrbisNpScorePlayerRankDataForCrossSave`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbisNpScorePlayerRankDataForCrossSave {
    pub has_data: i32,
    pub pad0: [u8; 4],
    pub rank_data: OrbisNpScoreRankDataForCrossSave,
}

/// The score entry points; the HLE loader implements them.
pub trait NpScoreApi {
    fn create_np_title_ctx(&mut self, service_label: u32, self_np_id: &OrbisNpId) -> i32;
    fn delete_np_title_ctx(&mut self, title_ctx_id: i32) -> i32;
    fn create_request(&mut self, title_ctx_id: i32) -> i32;
    fn delete_request(&mut self, request_id: i32) -> i32;
    fn abort_request(&mut self, request_id: i32) -> i32;
    fn poll_async(&mut self, request_id: i32, result: &mut i32) -> i32;
    fn wait_async(&mut self, request_id: i32, result: &mut i32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_score_structs_match_the_static_asserts() {
        assert_eq!(core::mem::size_of::<OrbisNpScoreBoardInfo>(), 24);
        assert_eq!(core::mem::size_of::<OrbisNpScoreComment>(), 64);
        assert_eq!(core::mem::size_of::<OrbisNpScoreGameInfo>(), 200);
        assert_eq!(core::mem::size_of::<OrbisNpScoreRankData>(), 0x80);
        assert_eq!(core::mem::size_of::<OrbisNpScoreRankDataA>(), 0x90);
        assert_eq!(core::mem::size_of::<OrbisNpScorePlayerRankData>(), 0x88);
        assert_eq!(core::mem::size_of::<OrbisNpScorePlayerRankDataA>(), 0x98);
        assert_eq!(ORBIS_NP_SCORE_BINDMODE_DEFAULT, 3);
        assert_eq!(ORBIS_NP_SCORE_MAX_CTX_NUM, 32);
    }
}

/// Score context/request state.
#[derive(Debug, Default)]
pub struct ScoreState {
    pub next_title_ctx: i32,
    pub next_request: i32,
    pub title_ctxs: std::collections::HashSet<i32>,
    pub requests: std::collections::HashSet<i32>,
}

impl ScoreState {
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
    fn score_requests_are_allocated_and_deleted() {
        let mut state = ScoreState::default();
        let ctx = state.create_title_ctx();
        let request = state.create_request();
        assert!(state.title_ctxs.contains(&ctx));
        assert_eq!(state.delete_request(request), 0);
        assert_eq!(state.delete_request(request), -1);
    }
}
