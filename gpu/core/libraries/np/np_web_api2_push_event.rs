//! NpWebApi2 push-event objects, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_web_api2/np_web_api2_push_event.h`.

use crate::gpu::bbnet::include::core::libraries::np::np_types::OrbisNpServiceLabel;
use crate::gpu::core::libraries::np::np_web_api2::*;

/// `UserContext`; an opaque parent used by push contexts.
pub struct UserContext;

/// `PushEventHandle`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PushEventHandle {
    id: i32,
    timeout: u32,
    user_count: i32,
    aborted: bool,
    deleting: bool,
    timed_out: bool,
    end_time: u64,
}

impl PushEventHandle {
    pub fn new(id: i32) -> Self {
        Self {
            id,
            ..Self::default()
        }
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn set_timeout(&mut self, timeout: u32) {
        self.timeout = timeout;
    }

    pub fn add_user(&mut self) {
        self.user_count += 1;
    }

    pub fn remove_user(&mut self) {
        self.user_count -= 1;
    }

    pub fn abort(&mut self) {
        self.aborted = true;
    }

    pub fn is_aborted(&self) -> bool {
        self.aborted
    }

    pub fn mark_for_deletion(&mut self) {
        self.deleting = true;
    }

    pub fn is_deleted(&self) -> bool {
        self.deleting
    }

    /// `SetEndTime`; `process_time` is the emulator's process clock in microseconds.
    pub fn set_end_time(&mut self, process_time: u64) {
        if self.timeout != 0 {
            self.end_time = process_time + u64::from(self.timeout);
        }
    }

    pub fn clear_end_time(&mut self) {
        self.end_time = 0;
    }

    pub fn end_time(&self) -> u64 {
        self.end_time
    }

    pub fn is_timed_out(&self) -> bool {
        self.timed_out
    }

    pub fn check_timeout(&mut self, time: u64) {
        if !self.timed_out && self.end_time != 0 && self.end_time < time {
            self.timed_out = true;
            self.abort();
        }
    }

    pub fn is_busy(&self) -> bool {
        self.user_count > 1
    }
}

/// `PushEventFilter`.
#[derive(Debug, Clone, Default)]
pub struct PushEventFilter {
    id: i32,
    np_service_label: OrbisNpServiceLabel,
    internal: bool,
    filter_params: Vec<OrbisNpWebApi2PushEventFilterParameter>,
    data_key_copy_storage: Vec<Vec<OrbisNpWebApi2PushEventExtdDataKey>>,
    np_service_name: String,
}

impl PushEventFilter {
    /// The header's constructor.
    pub fn new(
        id: i32,
        service_name: Option<&str>,
        service_label: OrbisNpServiceLabel,
        is_internal: bool,
    ) -> Self {
        Self {
            id,
            np_service_label: service_label,
            internal: is_internal,
            filter_params: Vec::new(),
            data_key_copy_storage: Vec::new(),
            np_service_name: service_name.unwrap_or("").to_string(),
        }
    }

    /// `Initialize`; copies the filter parameters and their ext-data keys.
    pub fn initialize(
        &mut self,
        handle: &PushEventHandle,
        filter_param: Option<&OrbisNpWebApi2PushEventFilterParameter>,
        filter_param_num: u64,
    ) -> i32 {
        if handle.is_deleted() {
            return crate::gpu::core::libraries::np::np_error::ORBIS_NP_WEBAPI2_ERROR_HANDLE_NOT_FOUND;
        }
        if handle.is_timed_out() {
            return crate::gpu::core::libraries::np::np_error::ORBIS_NP_WEBAPI2_ERROR_TIMEOUT;
        }
        if handle.is_aborted() {
            return crate::gpu::core::libraries::np::np_error::ORBIS_NP_WEBAPI2_ERROR_ABORTED;
        }
        self.filter_params.clear();
        self.data_key_copy_storage.clear();
        if filter_param_num == 0 {
            return 0;
        }
        let Some(first) = filter_param else {
            return -1;
        };
        // SAFETY: the caller provides `filter_param_num` consecutive entries.
        let params = unsafe { core::slice::from_raw_parts(first, filter_param_num as usize) };
        for param in params {
            self.filter_params.push(*param);
            let keys = if param.extd_data_key_num == 0 || param.extd_data_key.is_null() {
                Vec::new()
            } else {
                // SAFETY: the caller provides the key array.
                unsafe {
                    core::slice::from_raw_parts(
                        param.extd_data_key,
                        param.extd_data_key_num as usize,
                    )
                    .to_vec()
                }
            };
            self.data_key_copy_storage.push(keys);
        }
        0
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn np_service_name(&self) -> &str {
        &self.np_service_name
    }

    pub fn np_service_label(&self) -> OrbisNpServiceLabel {
        self.np_service_label
    }

    pub fn internal(&self) -> bool {
        self.internal
    }

    pub fn filter_params(&self) -> &[OrbisNpWebApi2PushEventFilterParameter] {
        &self.filter_params
    }
}

/// `PushEventCallback`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PushEventCallback {
    pub id: i32,
    pub filter_id: i32,
    pub callback: Option<OrbisNpWebApi2PushEventCallback>,
    pub user_arg: *mut core::ffi::c_void,
    pub is_busy: bool,
}

/// `PushEventPushContext`.
#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
pub struct PushEventPushContext {
    state: i32,
    timeout: u32,
    parent_user_ctx: *mut UserContext,
    end_time: u64,
    start_time: u64,
    id: OrbisNpWebApi2PushEventPushContextId,
    started: bool,
}

impl PushEventPushContext {
    pub fn new(timeout: u32, parent_user_ctx: *mut UserContext) -> Self {
        Self {
            timeout,
            parent_user_ctx,
            ..Self::default()
        }
    }

    /// `Initialize`; the id is a small incrementing index (the real library uses a mostly
    /// random string).
    pub fn initialize(&mut self) {
        let id = next_push_context_id();
        self.id.uuid = [0; 37];
        let bytes = id.to_le_bytes();
        self.id.uuid[..bytes.len()].copy_from_slice(&bytes);
    }

    pub fn id(&self) -> &OrbisNpWebApi2PushEventPushContextId {
        &self.id
    }

    /// `GetFakeId`.
    pub fn fake_id(&self) -> i64 {
        let mut bytes = [0u8; 8];
        let count = bytes.len().min(self.id.uuid.len());
        bytes[..count].copy_from_slice(&self.id.uuid[..count]);
        i64::from_le_bytes(bytes)
    }

    pub fn start(&mut self) {
        self.started = true;
    }

    pub fn set_state(&mut self, state: i32) {
        self.state = state;
    }

    pub fn callback_running(&self) -> bool {
        self.state == 2
    }

    pub fn started(&self) -> bool {
        self.started
    }

    pub fn parent_user_context(&self) -> *mut UserContext {
        self.parent_user_ctx
    }
}

fn next_push_context_id() -> i64 {
    use std::sync::atomic::{AtomicI64, Ordering};
    static NEXT: AtomicI64 = AtomicI64::new(0);
    let mut id = NEXT.fetch_add(1, Ordering::AcqRel) + 1;
    if id < 0 {
        id = 1;
        NEXT.store(1, Ordering::Release);
    }
    id
}

/// `PushEventPushContextCallback`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PushEventPushContextCallback {
    pub id: i32,
    pub filter_id: i32,
    pub callback: Option<OrbisNpWebApi2PushEventPushContextCallback>,
    pub user_arg: *mut core::ffi::c_void,
    pub is_busy: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_track_users_timeouts_and_deletion() {
        let mut handle = PushEventHandle::new(3);
        assert_eq!(handle.id(), 3);
        handle.add_user();
        assert!(!handle.is_busy());
        handle.add_user();
        assert!(handle.is_busy());
        handle.remove_user();
        handle.set_timeout(10);
        handle.set_end_time(100);
        assert_eq!(handle.end_time(), 110);
        handle.check_timeout(109);
        assert!(!handle.is_timed_out());
        handle.check_timeout(111);
        assert!(handle.is_timed_out());
        assert!(handle.is_aborted());
        handle.clear_end_time();
        assert_eq!(handle.end_time(), 0);
        handle.abort();
        assert!(handle.is_aborted());
        handle.mark_for_deletion();
        assert!(handle.is_deleted());
    }

    #[test]
    fn filters_copy_their_parameter_and_keys() {
        let keys = [
            OrbisNpWebApi2PushEventExtdDataKey { val: [b'a'; 33] },
            OrbisNpWebApi2PushEventExtdDataKey { val: [b'b'; 33] },
        ];
        let param = OrbisNpWebApi2PushEventFilterParameter {
            data_type: OrbisNpWebApi2PushEventDataType { val: [b'x'; 65] },
            extd_data_key: keys.as_ptr() as *mut _,
            extd_data_key_num: keys.len() as u64,
        };
        let mut filter = PushEventFilter::new(7, Some("np:service"), 9, true);
        let handle = PushEventHandle::new(1);
        assert_eq!(filter.initialize(&handle, Some(&param), 1), 0);
        assert_eq!(filter.id(), 7);
        assert_eq!(filter.np_service_name(), "np:service");
        assert_eq!(filter.np_service_label(), 9);
        assert!(filter.internal());
        assert_eq!(filter.filter_params().len(), 1);
    }

    #[test]
    fn push_contexts_initialize_and_report_state() {
        let mut context = PushEventPushContext::new(5, std::ptr::null_mut());
        context.initialize();
        assert_eq!(context.id().uuid[8], 0);
        assert_eq!(context.id().uuid[0], 1);
        context.start();
        assert!(context.started());
        context.set_state(2);
        assert!(context.callback_running());
        assert!(context.parent_user_context().is_null());
        assert_ne!(context.fake_id(), 0);
    }
}
