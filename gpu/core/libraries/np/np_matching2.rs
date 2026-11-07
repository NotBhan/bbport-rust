//! NpMatching2 API surface, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_matching2/np_matching2.h`.

use crate::gpu::core::libraries::np::np_types2::*;

/// The matching2 entry points; the HLE loader implements them.
pub trait NpMatching2Api {
    fn initialize(&mut self, param: Option<&OrbisNpMatching2InitializeParameter>) -> i32;
    fn terminate(&mut self) -> i32;
    fn create_context(
        &mut self,
        param: Option<&OrbisNpMatching2CreateContextParameter>,
    ) -> Result<OrbisNpMatching2ContextId, i32>;
    fn create_context_a(
        &mut self,
        param: Option<&OrbisNpMatching2CreateContextParameterA>,
    ) -> Result<OrbisNpMatching2ContextId, i32>;
    fn destroy_context(&mut self, ctx_id: OrbisNpMatching2ContextId) -> i32;
    fn context_start(&mut self, ctx_id: OrbisNpMatching2ContextId, timeout: u64) -> i32;
    fn context_stop(&mut self, ctx_id: OrbisNpMatching2ContextId) -> i32;
    fn abort_context_start(&mut self, ctx_id: OrbisNpMatching2ContextId) -> i32;
    fn get_server_id(
        &mut self,
        ctx_id: OrbisNpMatching2ContextId,
    ) -> Result<OrbisNpMatching2ServerId, i32>;
    fn register_context_callback(
        &mut self,
        callback: OrbisNpMatching2ContextCallback,
        userdata: *mut core::ffi::c_void,
    ) -> i32;
    fn register_room_event_callback(
        &mut self,
        ctx_id: OrbisNpMatching2ContextId,
        callback: OrbisNpMatching2RoomEventCallback,
        userdata: *mut core::ffi::c_void,
    ) -> i32;
    fn register_lobby_event_callback(
        &mut self,
        ctx_id: OrbisNpMatching2ContextId,
        callback: OrbisNpMatching2LobbyEventCallback,
        userdata: *mut core::ffi::c_void,
    ) -> i32;
    fn register_signaling_callback(
        &mut self,
        ctx_id: OrbisNpMatching2ContextId,
        callback: OrbisNpMatching2SignalingCallback,
        userdata: *mut core::ffi::c_void,
    ) -> i32;
    fn set_default_request_opt_param(
        &mut self,
        ctx_id: OrbisNpMatching2ContextId,
        param: Option<&OrbisNpMatching2RequestOptParam>,
    ) -> i32;
    fn get_memory_info(&mut self) -> i32;
    fn get_ssl_memory_info(&mut self) -> i32;
}

/// `RegisterLib` entry points (declared set).
pub const ENTRY_POINTS: [&str; 12] = [
    "sceNpMatching2Initialize",
    "sceNpMatching2Terminate",
    "sceNpMatching2CreateContext",
    "sceNpMatching2CreateContextA",
    "sceNpMatching2DestroyContext",
    "sceNpMatching2ContextStart",
    "sceNpMatching2ContextStop",
    "sceNpMatching2AbortContextStart",
    "sceNpMatching2GetServerId",
    "sceNpMatching2RegisterContextCallback",
    "sceNpMatching2GetMemoryInfo",
    "sceNpMatching2GetSslMemoryInfo",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_matching2_api_uses_the_guest_ids() {
        struct Api;
        impl NpMatching2Api for Api {
            fn initialize(&mut self, _: Option<&OrbisNpMatching2InitializeParameter>) -> i32 {
                0
            }
            fn terminate(&mut self) -> i32 {
                0
            }
            fn create_context(
                &mut self,
                _: Option<&OrbisNpMatching2CreateContextParameter>,
            ) -> Result<OrbisNpMatching2ContextId, i32> {
                Ok(1)
            }
            fn create_context_a(
                &mut self,
                _: Option<&OrbisNpMatching2CreateContextParameterA>,
            ) -> Result<OrbisNpMatching2ContextId, i32> {
                Ok(2)
            }
            fn destroy_context(&mut self, _: OrbisNpMatching2ContextId) -> i32 {
                0
            }
            fn context_start(&mut self, _: OrbisNpMatching2ContextId, _: u64) -> i32 {
                0
            }
            fn context_stop(&mut self, _: OrbisNpMatching2ContextId) -> i32 {
                0
            }
            fn abort_context_start(&mut self, _: OrbisNpMatching2ContextId) -> i32 {
                0
            }
            fn get_server_id(
                &mut self,
                _: OrbisNpMatching2ContextId,
            ) -> Result<OrbisNpMatching2ServerId, i32> {
                Ok(1)
            }
            fn register_context_callback(
                &mut self,
                _: OrbisNpMatching2ContextCallback,
                _: *mut core::ffi::c_void,
            ) -> i32 {
                0
            }
            fn register_room_event_callback(
                &mut self,
                _: OrbisNpMatching2ContextId,
                _: OrbisNpMatching2RoomEventCallback,
                _: *mut core::ffi::c_void,
            ) -> i32 {
                0
            }
            fn register_lobby_event_callback(
                &mut self,
                _: OrbisNpMatching2ContextId,
                _: OrbisNpMatching2LobbyEventCallback,
                _: *mut core::ffi::c_void,
            ) -> i32 {
                0
            }
            fn register_signaling_callback(
                &mut self,
                _: OrbisNpMatching2ContextId,
                _: OrbisNpMatching2SignalingCallback,
                _: *mut core::ffi::c_void,
            ) -> i32 {
                0
            }
            fn set_default_request_opt_param(
                &mut self,
                _: OrbisNpMatching2ContextId,
                _: Option<&OrbisNpMatching2RequestOptParam>,
            ) -> i32 {
                0
            }
            fn get_memory_info(&mut self) -> i32 {
                0
            }
            fn get_ssl_memory_info(&mut self) -> i32 {
                0
            }
        }
        let mut api = Api;
        assert_eq!(api.create_context(None), Ok(1));
        assert_eq!(api.get_server_id(1), Ok(1));
        assert_eq!(ENTRY_POINTS.len(), 12);
    }
}

/// The matching2 initialization/context manager used by the cpp.
#[derive(Debug, Default)]
pub struct Matching2State {
    pub initialized: bool,
    pub manager: crate::gpu::core::libraries::np::np_matching2_internal::ContextManager,
    pub next_request: OrbisNpMatching2RequestId,
}

impl Matching2State {
    pub fn initialize(&mut self) -> i32 {
        self.initialized = true;
        self.next_request = 1;
        0
    }

    pub fn terminate(&mut self) -> i32 {
        self.initialized = false;
        self.manager.contexts.clear();
        0
    }

    pub fn create_context(&mut self) -> Option<OrbisNpMatching2ContextId> {
        if !self.initialized {
            return None;
        }
        self.manager.create_context()
    }

    pub fn allocate_request(&mut self) -> OrbisNpMatching2RequestId {
        let id = self.next_request;
        self.next_request = self.next_request.wrapping_add(1).max(1);
        id
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn matching2_initialization_gates_context_creation() {
        let mut state = Matching2State::default();
        assert!(state.create_context().is_none());
        assert_eq!(state.initialize(), 0);
        assert!(state.create_context().is_some());
        assert_eq!(state.allocate_request(), 1);
        assert_eq!(state.terminate(), 0);
    }
}
