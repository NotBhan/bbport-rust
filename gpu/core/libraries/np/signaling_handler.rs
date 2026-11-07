//! Signaling handler hooks, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/signaling_handler.h`.

use crate::gpu::core::libraries::np::np_types2::*;

/// `Start`/`Stop` and context references.
pub trait SignalingHandlerRuntime {
    fn start(&mut self);
    fn stop(&mut self);
    fn add_context_ref(&mut self);
    fn release_context_ref(&mut self);
}

/// The matching2-facing signaling operations.
pub trait SignalingHandlerApi {
    fn activate_sig1(
        &mut self,
        ctx_id: OrbisNpSignalingContextId,
        peer_npid: &[u8],
        peer_online_id: &[u8],
    ) -> Result<OrbisNpSignalingConnectionId, i32>;
    fn find_matching2_member_np_id(&self, account_id: u64) -> Option<[u8; 36]>;
    fn deactivate_sig1(&mut self, conn_id: OrbisNpSignalingConnectionId);
    fn terminate_sig1(&mut self, conn_id: OrbisNpSignalingConnectionId);
    fn start_matching2(&mut self, ctx_id: OrbisNpMatching2ContextId);
    fn stop_matching2(&mut self, ctx_id: OrbisNpMatching2ContextId, error_code: i32);
    fn stop_matching2_room(
        &mut self,
        ctx_id: OrbisNpMatching2ContextId,
        room_id: OrbisNpMatching2RoomId,
        error_code: i32,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signaling_context_ids_are_signed() {
        let id: OrbisNpSignalingContextId = -1;
        assert_eq!(id, -1);
        assert_eq!(ORBIS_NP_SIGNALING_EVENT_DEAD, 0);
    }
}

/// The signaling-handler state used by the cpp.
#[derive(Debug, Default)]
pub struct SignalingHandlerState {
    pub started: bool,
    pub context_refs: u32,
    pub contexts: std::collections::HashMap<OrbisNpSignalingContextId, bool>,
    pub connections:
        std::collections::HashMap<OrbisNpSignalingConnectionId, OrbisNpSignalingContextId>,
    pub next_connection: OrbisNpSignalingConnectionId,
}

impl SignalingHandlerState {
    pub fn start(&mut self) {
        self.started = true;
    }

    pub fn stop(&mut self) {
        self.started = false;
        self.contexts.clear();
        self.connections.clear();
    }

    pub fn add_context_ref(&mut self) {
        self.context_refs += 1;
    }

    pub fn release_context_ref(&mut self) {
        self.context_refs = self.context_refs.saturating_sub(1);
    }

    pub fn activate(
        &mut self,
        ctx_id: OrbisNpSignalingContextId,
    ) -> Result<OrbisNpSignalingConnectionId, i32> {
        if !self.started {
            return Err(-1);
        }
        self.next_connection += 1;
        self.connections.insert(self.next_connection, ctx_id);
        Ok(self.next_connection)
    }

    pub fn deactivate(&mut self, conn_id: OrbisNpSignalingConnectionId) -> bool {
        self.connections.remove(&conn_id).is_some()
    }

    pub fn start_matching2(&mut self, ctx_id: OrbisNpMatching2ContextId) {
        self.contexts.insert(ctx_id as i32, true);
    }

    pub fn stop_matching2(&mut self, ctx_id: OrbisNpMatching2ContextId) {
        self.contexts.remove(&(ctx_id as i32));
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn signaling_handler_tracks_contexts_and_connections() {
        let mut handler = SignalingHandlerState::default();
        assert!(handler.activate(1).is_err());
        handler.start();
        handler.add_context_ref();
        handler.start_matching2(2);
        let conn = handler.activate(1).unwrap();
        assert!(handler.deactivate(conn));
        assert!(!handler.deactivate(conn));
        handler.stop_matching2(2);
        handler.release_context_ref();
        assert_eq!(handler.context_refs, 0);
        handler.stop();
        assert!(!handler.started);
    }
}
