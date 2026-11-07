//! NpWebApi2 internals, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_web_api2/np_web_api2_internal.h`.

use crate::gpu::bbnet::include::core::libraries::np::np_types::OrbisNpServiceLabel;
use crate::gpu::core::libraries::np::np_web_api2::*;

/// The internal API used by `np_web_api2.cpp`.
pub trait NpWebApi2Internal {
    fn create_library_context(
        &mut self,
        http_ctx_id: i32,
        kind: i32,
        pool_size: u64,
        name: &str,
    ) -> i32;
    fn memory_pool_stats(
        &mut self,
        lib_ctx_id: i32,
        stats: &mut OrbisNpWebApi2MemoryPoolStats,
    ) -> i32;
    fn create_push_event_handle(&mut self, lib_ctx_id: i32) -> i32;
    fn set_handle_timeout(&mut self, lib_ctx_id: i32, handle_id: i32, timeout: u32) -> i32;
    fn abort_push_event_handle(&mut self, lib_ctx_id: i32, handle_id: i32) -> i32;
    fn delete_push_event_handle(&mut self, lib_ctx_id: i32, handle_id: i32) -> i32;
    #[allow(clippy::too_many_arguments)]
    fn create_push_event_filter(
        &mut self,
        lib_ctx_id: i32,
        handle_id: i32,
        np_service_name: &str,
        np_service_label: OrbisNpServiceLabel,
        filter_param: Option<&OrbisNpWebApi2PushEventFilterParameter>,
        filter_param_num: u64,
        internal: bool,
    ) -> i32;
    fn delete_push_event_filter(&mut self, lib_ctx_id: i32, filter_id: i32) -> i32;
    fn terminate_library_context(&mut self, lib_ctx_id: i32) -> i32;
    fn create_user_context(&mut self, lib_ctx_id: i32, user_id: i32) -> i32;
    fn delete_user_context(&mut self, user_ctx_id: i32) -> i32;
    fn register_push_event_callback(
        &mut self,
        user_ctx_id: i32,
        filter_id: i32,
        callback: Option<OrbisNpWebApi2PushEventCallback>,
        user_arg: *mut core::ffi::c_void,
    ) -> i32;
    fn unregister_push_event_callback(&mut self, user_ctx_id: i32, callback_id: i32) -> i32;
    fn create_push_context(
        &mut self,
        user_ctx_id: i32,
        push_ctx_id: &mut OrbisNpWebApi2PushEventPushContextId,
    ) -> i32;
    fn start_push_context_callback(
        &mut self,
        user_ctx_id: i32,
        push_ctx_id: &OrbisNpWebApi2PushEventPushContextId,
    ) -> i32;
    fn delete_push_context(
        &mut self,
        user_ctx_id: i32,
        push_ctx_id: &OrbisNpWebApi2PushEventPushContextId,
    ) -> i32;
    fn register_push_context_callback(
        &mut self,
        user_ctx_id: i32,
        filter_id: i32,
        callback: Option<OrbisNpWebApi2PushEventPushContextCallback>,
        user_arg: *mut core::ffi::c_void,
    ) -> i32;
    fn unregister_push_context_callback(&mut self, user_ctx_id: i32, callback_id: i32) -> i32;
    fn process_push_events(&mut self);
    fn create_request(
        &mut self,
        user_ctx_id: i32,
        api_group: &str,
        path: &str,
        method: &str,
        content_parameter: Option<&OrbisNpWebApi2ContentParameter>,
        multipart: bool,
    ) -> Result<i64, i32>;
    fn add_http_request_header(&mut self, request_id: i64, name: &str, value: &str) -> i32;
    fn set_request_timeout(&mut self, request_id: i64, timeout: u32) -> i32;
    fn send_request(
        &mut self,
        request_id: i64,
        part_index: i32,
        data: &mut [u8],
        response: Option<&mut OrbisNpWebApi2ResponseInformationOption>,
    ) -> i32;
    fn http_response_header_data(
        &mut self,
        request_id: i64,
        field_name: &str,
        value: &mut [u8],
        value_size_out: &mut u64,
    ) -> i32;
    fn read_data(&mut self, request_id: i64, data: &mut [u8]) -> i32;
    fn abort_request(&mut self, request_id: i64) -> i32;
    fn delete_request(&mut self, request_id: i64) -> i32;
    fn check_timeout(&mut self);
}

/// `NpWebApi2Internal` reference state (contexts, handles and filters).
#[derive(Debug, Default)]
pub struct NpWebApi2InternalState {
    pub initialized: bool,
    pub next_lib_ctx: i32,
    pub next_user_ctx: i32,
    pub next_handle: i32,
    pub next_filter: i32,
    pub next_callback: i32,
    pub lib_contexts: std::collections::HashSet<i32>,
    pub user_contexts: std::collections::HashSet<i32>,
    pub handles: std::collections::HashSet<i32>,
    pub filters: std::collections::HashSet<i32>,
    pub callbacks: std::collections::HashSet<i32>,
}

impl NpWebApi2InternalState {
    pub fn create_library_context(
        &mut self,
        _http_ctx_id: i32,
        _kind: i32,
        _pool_size: u64,
        _name: &str,
    ) -> i32 {
        self.next_lib_ctx += 1;
        self.lib_contexts.insert(self.next_lib_ctx);
        self.next_lib_ctx
    }

    pub fn terminate_library_context(&mut self, id: i32) -> i32 {
        if self.lib_contexts.remove(&id) {
            0
        } else {
            -1
        }
    }

    pub fn create_user_context(&mut self, lib_ctx_id: i32, _user_id: i32) -> i32 {
        if !self.lib_contexts.contains(&lib_ctx_id) {
            return -1;
        }
        self.next_user_ctx += 1;
        self.user_contexts.insert(self.next_user_ctx);
        self.next_user_ctx
    }

    pub fn create_push_event_handle(&mut self, lib_ctx_id: i32) -> i32 {
        if !self.lib_contexts.contains(&lib_ctx_id) {
            return -1;
        }
        self.next_handle += 1;
        self.handles.insert(self.next_handle);
        self.next_handle
    }

    pub fn create_push_event_filter(&mut self, lib_ctx_id: i32) -> i32 {
        if !self.lib_contexts.contains(&lib_ctx_id) {
            return -1;
        }
        self.next_filter += 1;
        self.filters.insert(self.next_filter);
        self.next_filter
    }

    pub fn register_push_event_callback(&mut self, user_ctx_id: i32) -> i32 {
        if !self.user_contexts.contains(&user_ctx_id) {
            return -1;
        }
        self.next_callback += 1;
        self.callbacks.insert(self.next_callback);
        self.next_callback
    }

    pub fn process_push_events(&mut self) -> usize {
        self.callbacks.len()
    }
}

#[cfg(test)]
mod internal_cpp_tests {
    use super::*;

    #[test]
    fn internal_state_handles_contexts_handles_and_filters() {
        let mut state = NpWebApi2InternalState::default();
        let lib = state.create_library_context(1, 1, 4096, "ctx");
        assert_eq!(state.create_user_context(lib, 3), 1);
        assert_eq!(state.create_push_event_handle(lib), 1);
        assert_eq!(state.create_push_event_filter(lib), 1);
        assert_eq!(state.register_push_event_callback(1), 1);
        assert_eq!(state.process_push_events(), 1);
        assert_eq!(state.terminate_library_context(lib), 0);
        assert_eq!(state.create_user_context(lib, 3), -1);
    }
}
