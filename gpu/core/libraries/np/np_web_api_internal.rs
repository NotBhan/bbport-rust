//! NpWebApi internal state, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_web_api/np_web_api_internal.h`.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::gpu::bbnet::include::core::libraries::np::np_types::OrbisNpServiceLabel;
use crate::gpu::core::libraries::np::np_web_api::*;

/// `OrbisNpWebApiContext`.
pub struct OrbisNpWebApiContext {
    pub kind: i32,
    pub user_count: i32,
    pub lib_ctx_id: i32,
    pub lib_http_ctx_id: i32,
    pub context_lock: Mutex<()>,
    pub user_contexts: HashMap<i32, *mut OrbisNpWebApiUserContext>,
    pub handles: HashMap<i32, *mut OrbisNpWebApiHandle>,
    pub timer_handles: HashMap<i32, *mut OrbisNpWebApiTimerHandle>,
    pub push_event_filters: HashMap<i32, *mut OrbisNpWebApiPushEventFilter>,
    pub service_push_event_filters: HashMap<i32, *mut OrbisNpWebApiServicePushEventFilter>,
    pub extended_push_event_filters: HashMap<i32, *mut OrbisNpWebApiExtendedPushEventFilter>,
    pub name: String,
    pub terminated: bool,
}

/// `OrbisNpWebApiUserContext`.
pub struct OrbisNpWebApiUserContext {
    pub parent_context: *mut OrbisNpWebApiContext,
    pub user_count: i32,
    pub user_ctx_id: i32,
    pub user_id: i32,
    pub requests: HashMap<i64, *mut OrbisNpWebApiRequest>,
    pub push_event_callbacks: HashMap<i32, *mut OrbisNpWebApiRegisteredPushEventCallback>,
    pub service_push_event_callbacks:
        HashMap<i32, *mut OrbisNpWebApiRegisteredServicePushEventCallback>,
    pub extended_push_event_callbacks:
        HashMap<i32, *mut OrbisNpWebApiRegisteredExtendedPushEventCallback>,
    pub deleted: bool,
    pub notification_callback: Option<OrbisNpWebApiNotificationCallback>,
    pub notification_user_args: *mut core::ffi::c_void,
}

/// A multipart body part.
#[derive(Debug, Clone, Default)]
pub struct MultipartPart {
    pub raw_headers: String,
    pub content_length: u64,
    pub data: String,
    pub sent_size: u64,
}

/// `OrbisNpWebApiRequest`.
pub struct OrbisNpWebApiRequest {
    pub parent_context: *mut OrbisNpWebApiContext,
    pub user_count: i32,
    pub request_id: i64,
    pub user_api_group: String,
    pub user_path: String,
    pub user_method: OrbisNpWebApiHttpMethod,
    pub user_content_length: u64,
    pub user_content_type: String,
    pub user_headers: Vec<(String, String)>,
    pub multipart: bool,
    pub multipart_content_type: String,
    pub multipart_boundary: String,
    pub multipart_parts: Vec<MultipartPart>,
    pub aborted: bool,
    pub sent: bool,
    pub request_timeout: u32,
    pub request_end_time: u64,
    pub timed_out: bool,
    pub request_state: u8,
    pub remaining_data: u64,
    pub read_offset: u32,
    pub data: [u8; 64],
    pub http_connection_id: i32,
    pub http_request_id: i32,
    pub http_template_id: i32,
}

/// `OrbisNpWebApiHandle`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpWebApiHandle {
    pub handle_id: i32,
    pub aborted: bool,
    pub deleted: bool,
    pub user_count: i32,
}

/// `OrbisNpWebApiTimerHandle`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpWebApiTimerHandle {
    pub handle_id: i32,
    pub handle_timeout: u32,
    pub handle_end_time: u64,
    pub timed_out: bool,
}

/// `OrbisNpWebApiPushEventFilter`.
pub struct OrbisNpWebApiPushEventFilter {
    pub filter_id: i32,
    pub filter_params: Vec<OrbisNpWebApiPushEventFilterParameter>,
    pub parent_context: *mut OrbisNpWebApiContext,
}

/// `OrbisNpWebApiServicePushEventFilter`.
pub struct OrbisNpWebApiServicePushEventFilter {
    pub filter_id: i32,
    pub internal: bool,
    pub filter_params: Vec<OrbisNpWebApiServicePushEventFilterParameter>,
    pub np_service_name: String,
    pub np_service_label: OrbisNpServiceLabel,
    pub parent_context: *mut OrbisNpWebApiContext,
}

/// `OrbisNpWebApiExtendedPushEventFilter`.
pub struct OrbisNpWebApiExtendedPushEventFilter {
    pub filter_id: i32,
    pub internal: bool,
    pub filter_params: Vec<OrbisNpWebApiExtdPushEventFilterParameter>,
    pub extd_data_keys: Vec<Vec<String>>,
    pub np_service_name: String,
    pub np_service_label: OrbisNpServiceLabel,
    pub parent_context: *mut OrbisNpWebApiContext,
}

/// `OrbisNpWebApiRegisteredPushEventCallback`.
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNpWebApiRegisteredPushEventCallback {
    pub callback_id: i32,
    pub filter_id: i32,
    pub callback: Option<OrbisNpWebApiPushEventCallback>,
    pub user_arg: *mut core::ffi::c_void,
}

/// `OrbisNpWebApiRegisteredServicePushEventCallback`.
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNpWebApiRegisteredServicePushEventCallback {
    pub callback_id: i32,
    pub filter_id: i32,
    pub callback: Option<OrbisNpWebApiServicePushEventCallback>,
    pub internal_callback: Option<OrbisNpWebApiInternalServicePushEventCallback>,
    pub internal_callback_a: Option<OrbisNpWebApiInternalServicePushEventCallbackA>,
    pub user_arg: *mut core::ffi::c_void,
}

/// `OrbisNpWebApiRegisteredExtendedPushEventCallback`.
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNpWebApiRegisteredExtendedPushEventCallback {
    pub callback_id: i32,
    pub filter_id: i32,
    pub callback: Option<OrbisNpWebApiExtdPushEventCallback>,
    pub callback_a: Option<OrbisNpWebApiExtdPushEventCallbackA>,
    pub user_arg: *mut core::ffi::c_void,
}

// The contexts hold raw guest pointers; all mutation is expected under their locks.
unsafe impl Send for OrbisNpWebApiContext {}
unsafe impl Sync for OrbisNpWebApiContext {}
unsafe impl Send for OrbisNpWebApiUserContext {}
unsafe impl Sync for OrbisNpWebApiUserContext {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_internal_state_defaults_are_empty() {
        let context = OrbisNpWebApiContext {
            kind: 0,
            user_count: 0,
            lib_ctx_id: 1,
            lib_http_ctx_id: 2,
            context_lock: Mutex::new(()),
            user_contexts: HashMap::new(),
            handles: HashMap::new(),
            timer_handles: HashMap::new(),
            push_event_filters: HashMap::new(),
            service_push_event_filters: HashMap::new(),
            extended_push_event_filters: HashMap::new(),
            name: "ctx".to_string(),
            terminated: false,
        };
        assert_eq!(context.lib_ctx_id, 1);
        assert!(context.user_contexts.is_empty());

        let handle = OrbisNpWebApiHandle {
            handle_id: 5,
            aborted: false,
            deleted: false,
            user_count: 1,
        };
        assert_eq!(handle.handle_id, 5);
        let timer = OrbisNpWebApiTimerHandle {
            handle_id: 6,
            handle_timeout: 7,
            handle_end_time: 8,
            timed_out: false,
        };
        assert_eq!(timer.handle_timeout, 7);
        let part = MultipartPart::default();
        assert_eq!(part.content_length, 0);
        assert_eq!(core::mem::size_of::<OrbisNpWebApiHandle>(), 12);
    }
}

/// The WebAPI internal request/context maps implemented by the cpp.
#[derive(Debug, Default)]
pub struct WebApiInternalState {
    pub next_lib_ctx: i32,
    pub next_user_ctx: i32,
    pub next_request: i64,
    pub lib_contexts: std::collections::HashMap<i32, String>,
    pub user_contexts: std::collections::HashMap<i32, i32>,
    pub requests: std::collections::HashMap<i64, (i32, String, String)>,
}

impl WebApiInternalState {
    pub fn create_library_context(&mut self, name: &str) -> i32 {
        self.next_lib_ctx += 1;
        self.lib_contexts
            .insert(self.next_lib_ctx, name.to_string());
        self.next_lib_ctx
    }

    pub fn create_user_context(&mut self, lib_ctx_id: i32, user_id: i32) -> i32 {
        if !self.lib_contexts.contains_key(&lib_ctx_id) {
            return -1;
        }
        self.next_user_ctx += 1;
        self.user_contexts.insert(self.next_user_ctx, user_id);
        self.next_user_ctx
    }

    pub fn create_request(&mut self, user_ctx_id: i32, path: &str, method: &str) -> Option<i64> {
        if !self.user_contexts.contains_key(&user_ctx_id) {
            return None;
        }
        self.next_request += 1;
        self.requests.insert(
            self.next_request,
            (user_ctx_id, path.to_string(), method.to_string()),
        );
        Some(self.next_request)
    }

    pub fn delete_request(&mut self, id: i64) -> bool {
        self.requests.remove(&id).is_some()
    }

    pub fn request_count(&self) -> usize {
        self.requests.len()
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn webapi_internal_contexts_and_requests() {
        let mut state = WebApiInternalState::default();
        let lib = state.create_library_context("ctx");
        assert_eq!(state.create_user_context(lib, 3), 1);
        let request = state.create_request(1, "/path", "GET").unwrap();
        assert_eq!(state.request_count(), 1);
        assert!(state.delete_request(request));
        assert_eq!(state.create_user_context(99, 3), -1);
    }
}
