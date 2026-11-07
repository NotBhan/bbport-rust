//! NpSignaling API surface, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_signaling/np_signaling.h`.

use crate::gpu::core::libraries::np::np_types2::*;

/// The signaling entry points; the HLE loader implements them.
pub trait NpSignalingApi {
    fn initialize(
        &mut self,
        memory_size: i64,
        thread_priority: i32,
        cpu_affinity_mask: i32,
        thread_stack_size: i64,
    ) -> i32;
    fn terminate(&mut self) -> i32;
    fn create_context(
        &mut self,
        np_id: Option<&[u8]>,
        callback: usize,
        callback_arg: *mut core::ffi::c_void,
    ) -> Result<OrbisNpSignalingContextId, i32>;
    fn create_context_a(
        &mut self,
        user_id: i32,
        callback: usize,
        callback_arg: *mut core::ffi::c_void,
    ) -> Result<OrbisNpSignalingContextId, i32>;
    fn delete_context(&mut self, ctx_id: OrbisNpSignalingContextId) -> i32;
    fn activate_connection(
        &mut self,
        ctx_id: OrbisNpSignalingContextId,
        peer_np_id: Option<&[u8]>,
    ) -> Result<OrbisNpSignalingConnectionId, i32>;
    fn activate_connection_a(
        &mut self,
        ctx_id: OrbisNpSignalingContextId,
        peer: Option<&OrbisNpSignalingAccountPlatformPair>,
    ) -> Result<OrbisNpSignalingConnectionId, i32>;
    fn deactivate_connection(
        &mut self,
        ctx_id: OrbisNpSignalingContextId,
        conn_id: OrbisNpSignalingConnectionId,
    ) -> i32;
    fn terminate_connection(
        &mut self,
        ctx_id: OrbisNpSignalingContextId,
        conn_id: OrbisNpSignalingConnectionId,
    ) -> i32;
    fn get_connection_status(
        &mut self,
        ctx_id: OrbisNpSignalingContextId,
        conn_id: OrbisNpSignalingConnectionId,
    ) -> Option<(u32, u32, u16)>;
    fn set_context_option(
        &mut self,
        ctx_id: OrbisNpSignalingContextId,
        option_id: i32,
        value: i32,
    ) -> i32;
    fn get_context_option(
        &mut self,
        ctx_id: OrbisNpSignalingContextId,
        option_id: i32,
    ) -> Option<i32>;
    fn get_local_net_info(
        &mut self,
        ctx_id: OrbisNpSignalingContextId,
    ) -> Option<OrbisNpSignalingNetInfo>;
    fn get_peer_net_info(
        &mut self,
        ctx_id: OrbisNpSignalingContextId,
        peer_np_id: Option<&[u8]>,
    ) -> Option<OrbisNpSignalingRequestId>;
    fn get_memory_info(&mut self) -> OrbisNpSignalingMemoryInfo;
    fn get_connection_statistics(&mut self) -> OrbisNpSignalingConnectionStatistics;
}

/// The helper calls used by NpHandler.
pub trait NpSignalingPeers {
    fn active_connection_id_for_peer(&self, online_id: &str) -> i32;
    fn connection_status_for_peer(&self, online_id: &str) -> Option<i32>;
    fn peer_address(&self, online_id: &str) -> Option<(u32, u16)>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_signaling_types_keep_their_layouts() {
        assert_eq!(
            core::mem::size_of::<OrbisNpSignalingConnectionStatistics>(),
            16
        );
        assert_eq!(
            core::mem::size_of::<OrbisNpSignalingAccountPlatformPair>(),
            16
        );
        assert_eq!(ORBIS_NP_SIGNALING_EVENT_ESTABLISHED, 1);
    }
}

/// The signaling contexts/connections allocated by `np_signaling.cpp`.
#[derive(Debug, Default)]
pub struct NpSignalingStateFacade {
    pub initialized: bool,
    pub next_context: OrbisNpSignalingContextId,
    pub next_connection: OrbisNpSignalingConnectionId,
    pub contexts: std::collections::HashMap<OrbisNpSignalingContextId, bool>,
    pub connections:
        std::collections::HashMap<OrbisNpSignalingConnectionId, OrbisNpSignalingContextId>,
}

impl NpSignalingStateFacade {
    pub fn initialize(&mut self) -> i32 {
        self.initialized = true;
        0
    }

    pub fn terminate(&mut self) -> i32 {
        self.initialized = false;
        self.contexts.clear();
        self.connections.clear();
        0
    }

    pub fn create_context(&mut self) -> Option<OrbisNpSignalingContextId> {
        if !self.initialized {
            return None;
        }
        self.next_context += 1;
        self.contexts.insert(self.next_context, true);
        Some(self.next_context)
    }

    pub fn delete_context(&mut self, ctx_id: OrbisNpSignalingContextId) -> i32 {
        if self.contexts.remove(&ctx_id).is_some() {
            0
        } else {
            -1
        }
    }

    pub fn activate_connection(
        &mut self,
        ctx_id: OrbisNpSignalingContextId,
    ) -> Option<OrbisNpSignalingConnectionId> {
        if !self.contexts.contains_key(&ctx_id) {
            return None;
        }
        self.next_connection += 1;
        self.connections.insert(self.next_connection, ctx_id);
        Some(self.next_connection)
    }

    pub fn connection_status(
        &self,
        conn_id: OrbisNpSignalingConnectionId,
    ) -> Option<(u32, u32, u16)> {
        self.connections.contains_key(&conn_id).then_some((
            ORBIS_NP_SIGNALING_CONN_STATUS_INACTIVE as u32,
            0,
            0,
        ))
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn signaling_contexts_and_connections_are_tracked() {
        let mut state = NpSignalingStateFacade::default();
        assert!(state.create_context().is_none());
        assert_eq!(state.initialize(), 0);
        let ctx = state.create_context().unwrap();
        let conn = state.activate_connection(ctx).unwrap();
        assert!(state.connection_status(conn).is_some());
        assert_eq!(state.delete_context(ctx), 0);
        assert_eq!(state.delete_context(ctx), -1);
    }
}
