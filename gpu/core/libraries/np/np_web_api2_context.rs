//! NpWebApi2 context objects, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_web_api2/np_web_api2_context.h`.

use std::collections::HashMap;

use crate::gpu::core::libraries::np::np_web_api2::{
    OrbisNpWebApi2PushEventFilterParameter, OrbisNpWebApi2PushEventPushContextId,
};
use crate::gpu::core::libraries::np::np_web_api2_push_event::{
    PushEventCallback, PushEventFilter, PushEventHandle, PushEventPushContext,
    PushEventPushContextCallback,
};

/// `LibraryContextType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum LibraryContextType {
    Debug = 0,
    Normal = 1,
    Internal = 2,
    Presence = 3,
}

/// `LibraryContext`.
pub struct LibraryContext {
    pub id: i32,
    pub kind: LibraryContextType,
    pub http_ctx_id: i32,
    pub user_count: i32,
    pub pool_size: u64,
    pub deleting: bool,
    pub name: String,
    pub user_contexts: HashMap<i32, UserContext>,
    pub push_event_handles: HashMap<i32, PushEventHandle>,
    pub push_event_filters: HashMap<i32, PushEventFilter>,
}

impl LibraryContext {
    pub fn new(
        id: i32,
        kind: LibraryContextType,
        http_ctx_id: i32,
        pool_size: u64,
        name: &str,
    ) -> Self {
        Self {
            id,
            kind,
            http_ctx_id,
            user_count: 0,
            pool_size,
            deleting: false,
            name: name.to_string(),
            user_contexts: HashMap::new(),
            push_event_handles: HashMap::new(),
            push_event_filters: HashMap::new(),
        }
    }

    pub fn add_user(&mut self) {
        self.user_count += 1;
    }

    pub fn remove_user(&mut self) {
        self.user_count -= 1;
    }

    pub fn is_debug(&self) -> bool {
        self.kind == LibraryContextType::Debug
    }

    pub fn is_normal(&self) -> bool {
        self.kind == LibraryContextType::Normal
    }

    pub fn is_internal(&self) -> bool {
        self.kind == LibraryContextType::Internal
    }

    pub fn is_presence(&self) -> bool {
        self.kind == LibraryContextType::Presence
    }

    pub fn is_deleted(&self) -> bool {
        self.deleting
    }

    pub fn mark_for_deletion(&mut self) {
        self.deleting = true;
    }

    pub fn is_busy(&self) -> bool {
        self.user_count > 1
    }

    pub fn get_name(&self) -> Option<&str> {
        if self.name.is_empty() {
            None
        } else {
            Some(&self.name)
        }
    }

    pub fn set_handle_end_time(&mut self, handle_id: i32, process_time: u64) {
        if let Some(handle) = self.push_event_handles.get_mut(&handle_id) {
            handle.set_end_time(process_time);
        }
    }

    pub fn abort_all_push_event_handles(&mut self) {
        for handle in self.push_event_handles.values_mut() {
            handle.abort();
        }
    }

    pub fn has_busy_push_event_handles(&self) -> bool {
        self.push_event_handles
            .values()
            .any(PushEventHandle::is_busy)
    }
}

/// `UserContext`.
pub struct UserContext {
    pub id: i32,
    pub user_id: i32,
    pub user_count: i32,
    pub http_template_id: i32,
    pub deleting: bool,
    pub push_contexts_busy: bool,
    pub parent_ctx_id: i32,
    pub user_agent: String,
    pub requests: HashMap<i64, Request>,
    pub push_event_callbacks: HashMap<i32, PushEventCallback>,
    pub push_contexts: HashMap<i64, PushEventPushContext>,
    pub push_context_callbacks: HashMap<i32, PushEventPushContextCallback>,
}

impl UserContext {
    pub fn new(parent_ctx_id: i32, id: i32, user_id: i32) -> Self {
        Self {
            id,
            user_id,
            user_count: 0,
            http_template_id: 0,
            deleting: false,
            push_contexts_busy: false,
            parent_ctx_id,
            user_agent: String::new(),
            requests: HashMap::new(),
            push_event_callbacks: HashMap::new(),
            push_contexts: HashMap::new(),
            push_context_callbacks: HashMap::new(),
        }
    }

    pub fn add_user(&mut self) {
        self.user_count += 1;
    }

    pub fn remove_user(&mut self) {
        self.user_count -= 1;
    }

    pub fn is_deleted(&self) -> bool {
        self.deleting
    }

    pub fn mark_for_deletion(&mut self) {
        self.deleting = true;
    }

    pub fn is_busy(&self) -> bool {
        self.user_count > 1
    }

    pub fn remove_request(&mut self, request_id: i64) {
        self.requests.remove(&request_id);
    }
}

/// `Request`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpRequestHeader {
    pub field_name: [u8; 0],
    pub field_value: [u8; 0],
}

/// `Request`.
pub struct Request {
    pub id: i64,
    pub content_length: u64,
    pub http_response_headers: Option<String>,
    pub http_response_header_size: u64,
    pub user_count: i32,
    pub http_request_id: i32,
    pub timeout: u32,
    pub sent_data: i32,
    pub send_state: i32,
    pub multipart_supported: bool,
    pub sent: bool,
    pub aborted: bool,
    pub expired: bool,
    pub deleting: bool,
    pub end_time: u64,
    pub parent_ctx_id: i32,
    pub api_group: String,
    pub path: String,
    pub method: String,
    pub content_type: String,
    pub http_headers: Vec<(String, String)>,
}

impl Request {
    pub fn new(
        parent_ctx_id: i32,
        id: i64,
        api_group: &str,
        path: &str,
        method: &str,
        multipart: bool,
    ) -> Self {
        Self {
            id,
            content_length: 0,
            http_response_headers: None,
            http_response_header_size: 0,
            user_count: 0,
            http_request_id: 0,
            timeout: 0,
            sent_data: 0,
            send_state: 0,
            multipart_supported: multipart,
            sent: false,
            aborted: false,
            expired: false,
            deleting: false,
            end_time: 0,
            parent_ctx_id,
            api_group: api_group.to_string(),
            path: path.to_string(),
            method: method.to_string(),
            content_type: String::new(),
            http_headers: Vec::new(),
        }
    }

    pub fn add_user(&mut self) {
        self.user_count += 1;
    }

    pub fn remove_user(&mut self) {
        self.user_count -= 1;
    }

    pub fn is_multipart(&self) -> bool {
        self.multipart_supported
    }

    pub fn has_sent(&self) -> bool {
        self.sent
    }

    pub fn mark_sent(&mut self) {
        self.sent = true;
    }

    pub fn out_of_data(&self) -> bool {
        self.content_length == self.sent_data as u64
    }

    pub fn set_end_time(&mut self, process_time: u64) {
        if self.timeout != 0 && self.end_time == 0 {
            self.end_time = process_time + u64::from(self.timeout);
        }
    }

    pub fn clear_end_time(&mut self) {
        self.end_time = 0;
    }

    pub fn set_timeout(&mut self, timeout: u32) {
        self.timeout = timeout;
    }

    pub fn check_timeout(&mut self, process_time: u64) {
        if !self.expired && self.end_time != 0 && self.end_time < process_time {
            self.expired = true;
            self.abort();
        }
    }

    pub fn abort(&mut self) {
        self.aborted = true;
    }

    pub fn mark_for_deletion(&mut self) {
        self.deleting = true;
    }

    pub fn is_busy(&self) -> bool {
        self.user_count > 1
    }
}

/// The context API used by `np_web_api2.cpp`.
pub trait NpWebApi2ContextApi {
    fn create_push_event_handle(&mut self, lib_ctx_id: i32) -> i32;
    fn delete_push_event_handle(&mut self, lib_ctx_id: i32, handle_id: i32) -> i32;
    #[allow(clippy::too_many_arguments)]
    fn create_push_event_filter(
        &mut self,
        lib_ctx_id: i32,
        handle_id: i32,
        service_name: &str,
        label: u32,
        filter_param: Option<&OrbisNpWebApi2PushEventFilterParameter>,
        filter_param_num: u64,
        internal: bool,
    ) -> i32;
    fn create_user_context(&mut self, lib_ctx_id: i32, user_id: i32) -> i32;
    fn delete_user_context(&mut self, user_ctx_id: i32) -> i32;
    fn create_push_context(&mut self, user_ctx_id: i32) -> OrbisNpWebApi2PushEventPushContextId;
    fn delete_push_context(
        &mut self,
        user_ctx_id: i32,
        id: &OrbisNpWebApi2PushEventPushContextId,
    ) -> i32;
    fn process_push_events(&mut self);
    fn check_timeout(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_contexts_track_users_and_deletion() {
        let mut ctx = LibraryContext::new(1, LibraryContextType::Normal, 2, 4096, "ctx");
        assert!(ctx.is_normal());
        assert!(!ctx.is_debug());
        assert_eq!(ctx.get_name(), Some("ctx"));
        ctx.add_user();
        assert!(!ctx.is_busy());
        ctx.add_user();
        assert!(ctx.is_busy());
        ctx.mark_for_deletion();
        assert!(ctx.is_deleted());
    }

    #[test]
    fn requests_report_data_and_timeouts() {
        let mut request = Request::new(1, 9, "group", "/path", "GET", true);
        assert!(request.is_multipart());
        assert!(request.out_of_data());
        request.set_timeout(10);
        request.set_end_time(100);
        assert_eq!(request.end_time, 110);
        request.check_timeout(109);
        assert!(!request.expired);
        request.check_timeout(111);
        assert!(request.expired);
        assert!(request.aborted);
        assert_eq!(request.id, 9);
    }

    #[test]
    fn user_contexts_add_and_remove_requests() {
        let mut user = UserContext::new(1, 2, 3);
        user.add_user();
        assert!(!user.is_busy());
        user.requests
            .insert(7, Request::new(1, 7, "", "", "", false));
        user.remove_request(7);
        assert!(user.requests.is_empty());
    }
}

impl LibraryContext {
    /// `CreatePushEventHandle`.
    pub fn create_push_event_handle(&mut self) -> i32 {
        let id = self.push_event_handles.len() as i32 + 1;
        self.push_event_handles.insert(id, PushEventHandle::new(id));
        id
    }

    /// `DeletePushEventHandle`.
    pub fn delete_push_event_handle(&mut self, handle_id: i32) -> i32 {
        if self.push_event_handles.remove(&handle_id).is_some() {
            0
        } else {
            -1
        }
    }
}

impl UserContext {
    /// `CreateRequest`.
    pub fn create_request(
        &mut self,
        request_id: i64,
        api_group: &str,
        path: &str,
        method: &str,
        multipart: bool,
    ) -> &mut Request {
        self.requests.insert(
            request_id,
            Request::new(self.id, request_id, api_group, path, method, multipart),
        );
        self.requests
            .get_mut(&request_id)
            .expect("inserted request")
    }

    /// `GetRequest`.
    pub fn get_request(&mut self, request_id: i64) -> Option<&mut Request> {
        self.requests.get_mut(&request_id)
    }

    pub fn delete_request(&mut self, request_id: i64) -> bool {
        self.requests.remove(&request_id).is_some()
    }
}

#[cfg(test)]
mod context_cpp_tests {
    use super::*;

    #[test]
    fn contexts_allocate_handles_and_requests() {
        let mut library = LibraryContext::new(1, LibraryContextType::Normal, 2, 4096, "ctx");
        assert_eq!(library.create_push_event_handle(), 1);
        assert_eq!(library.delete_push_event_handle(1), 0);
        let mut user = UserContext::new(1, 2, 3);
        let request = user.create_request(7, "group", "/path", "GET", false);
        assert_eq!(request.id, 7);
        assert!(user.get_request(7).is_some());
        assert!(user.delete_request(7));
        assert!(user.get_request(7).is_none());
    }
}
