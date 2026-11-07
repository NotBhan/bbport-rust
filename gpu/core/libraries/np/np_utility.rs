//! NP utility request types, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_utility/np_utility.h`.

pub const ORBIS_NP_LOOKUP_MAX_CTX_NUM: i32 = 32;
pub const ORBIS_NP_LOOKUP_MAX_REQUEST_NUM: i32 = 32;
pub const ORBIS_NP_LOOKUP_POLL_ASYNC_RET_FINISHED: i32 = 0;
pub const ORBIS_NP_LOOKUP_POLL_ASYNC_RET_RUNNING: i32 = 1;
pub const ORBIS_NP_LOOKUP_TIMEOUT_NO_EFFECT: u32 = 0;

pub type OrbisNpLookupTitleCtxId = i32;
pub type OrbisNpLookupRequestId = i32;

/// `OrbisNpLookupCreateAsyncRequestParameter`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpLookupCreateAsyncRequestParameter {
    pub size: u64,
    pub cpu_affinity_mask: u64,
    pub thread_priority: i32,
    pub padding: [u8; 4],
}

pub const ORBIS_NP_WORD_FILTER_MAX_CTX_NUM: i32 = 32;
pub const ORBIS_NP_WORD_FILTER_MAX_REQUEST_NUM: i32 = 32;
pub const ORBIS_NP_WORD_FILTER_TITLE_CTX_ID_OFFSET: i32 = 0x0100_0000;
pub const ORBIS_NP_WORD_FILTER_REQUEST_ID_OFFSET: i32 = 0x0200_0000;
pub const ORBIS_NP_WORD_FILTER_COMMENT_MAX_LENGTH: usize = 0x400;
pub const ORBIS_NP_WORD_FILTER_POLL_ASYNC_RET_FINISHED: i32 = 0;
pub const ORBIS_NP_WORD_FILTER_POLL_ASYNC_RET_RUNNING: i32 = 1;

pub type OrbisNpWordFilterTitleCtxId = i32;
pub type OrbisNpWordFilterRequestId = i32;

/// `OrbisNpWordFilterCreateAsyncRequestParameter`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpWordFilterCreateAsyncRequestParameter {
    pub size: u64,
    pub cpu_affinity_mask: u64,
    pub thread_priority: i32,
    pub padding: [u8; 4],
}

/// The utility entry points; the HLE loader implements them.
pub trait NpUtilityApi {
    fn initialize(&mut self) -> i32;
    fn terminate(&mut self) -> i32;
    fn lookup_create_request(&mut self, title_ctx_id: OrbisNpLookupTitleCtxId) -> i32;
    fn lookup_delete_request(&mut self, request_id: OrbisNpLookupRequestId) -> i32;
    fn word_filter_create_request(&mut self, title_ctx_id: OrbisNpWordFilterTitleCtxId) -> i32;
    fn word_filter_delete_request(&mut self, request_id: OrbisNpWordFilterRequestId) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_utility_request_types_match_the_header() {
        assert_eq!(
            core::mem::size_of::<OrbisNpLookupCreateAsyncRequestParameter>(),
            0x18
        );
        assert_eq!(
            core::mem::size_of::<OrbisNpWordFilterCreateAsyncRequestParameter>(),
            0x18
        );
        assert_eq!(ORBIS_NP_LOOKUP_MAX_CTX_NUM, 32);
        assert_eq!(ORBIS_NP_WORD_FILTER_TITLE_CTX_ID_OFFSET, 0x01000000);
        assert_eq!(ORBIS_NP_WORD_FILTER_COMMENT_MAX_LENGTH, 0x400);
        assert_eq!(ORBIS_NP_LOOKUP_POLL_ASYNC_RET_RUNNING, 1);
    }
}

/// NpUtility lookup/word-filter request state.
#[derive(Debug, Default)]
pub struct UtilityState {
    pub initialized: bool,
    pub next_title_ctx: i32,
    pub next_request: i32,
    pub title_ctxs: std::collections::HashSet<i32>,
    pub requests: std::collections::HashSet<i32>,
}

impl UtilityState {
    pub fn initialize(&mut self) -> i32 {
        self.initialized = true;
        self.next_title_ctx = 1;
        self.next_request = 1;
        0
    }

    pub fn terminate(&mut self) -> i32 {
        self.initialized = false;
        self.title_ctxs.clear();
        self.requests.clear();
        0
    }

    pub fn create_title_ctx(&mut self) -> Option<i32> {
        if !self.initialized {
            return None;
        }
        let id = self.next_title_ctx;
        self.next_title_ctx += 1;
        self.title_ctxs.insert(id);
        Some(id)
    }

    pub fn create_request(&mut self) -> Option<i32> {
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
    fn utility_contexts_and_requests_require_init() {
        let mut state = UtilityState::default();
        assert!(state.create_request().is_none());
        assert_eq!(state.initialize(), 0);
        assert_eq!(state.create_title_ctx(), Some(1));
        assert_eq!(state.create_request(), Some(1));
        assert_eq!(state.terminate(), 0);
    }
}
