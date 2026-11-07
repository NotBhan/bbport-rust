//! NpSignaling connection state, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_signaling/np_signaling_state.h`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::gpu::bbnet::include::core::libraries::np::np_types::{OrbisNpId, OrbisNpOnlineId};
use crate::gpu::core::libraries::np::np_types2::*;

pub const ACTIVATE_COOLDOWN_US: i64 = 6_000_000;
pub const ACTIVATE_BUDGET_MAX_US: i64 = 600_000_000;
pub const SIG_RETRY_MS: u32 = 500;
pub const SIG_PING_MS: u32 = 5_000;
pub const SIGNALING_MAGIC: [u8; 4] = *b"SHAD";

/// `StunPing`.
#[repr(C, packed(1))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StunPing {
    pub cmd: u8,
    pub online_id: [u8; 16],
    pub local_ip: u32,
}

/// `StunEcho`.
#[repr(C, packed(1))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StunEcho {
    pub ext_ip: u32,
    pub ext_port: u16,
}

/// `SignalingPacketType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SignalingPacketType {
    EchoPing = 0x05,
    EchoPong = 0x06,
    Handshake = 0x07,
    Control = 0x10,
}

/// `SignalingEchoPing`.
#[repr(C, packed(1))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalingEchoPing {
    pub magic: [u8; 4],
    pub kind: u8,
    pub conn_id: u32,
    pub send_ts_us: u64,
}

/// `SignalingEchoPong`.
#[repr(C, packed(1))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalingEchoPong {
    pub magic: [u8; 4],
    pub kind: u8,
    pub conn_id: u32,
    pub orig_ts_us: u64,
}

/// `SignalingHandshake`.
#[repr(C, packed(1))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalingHandshake {
    pub magic: [u8; 4],
    pub kind: u8,
    pub handshake_kind: u8,
    pub online_id_from: [u8; 16],
    pub online_id_to: [u8; 16],
    pub from_conn_id: u32,
    pub to_conn_id: u32,
    pub mapped_addr: u32,
    pub mapped_port: u16,
    pub reserved: u16,
    pub nonce: u64,
}

/// `SignalingControl`.
#[repr(C, packed(1))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalingControl {
    pub magic: [u8; 4],
    pub kind: u8,
    pub control_kind: u8,
    pub online_id_from: [u8; 16],
    pub online_id_to: [u8; 16],
    pub from_conn_id: u32,
    pub mapped_addr: u32,
    pub mapped_port: u16,
    pub reason: u16,
}

/// `ConnState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ConnState {
    Inactive = 0,
    SendingOffer = 5,
    WaitOffer = 6,
    ConnCheck = 7,
    SendingAccept = 8,
    WaitAccept = 9,
    Established = 10,
}

/// `HandshakeKind` / `ControlKind` / `ControlReason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HandshakeKind {
    Offer = 1,
    Accept = 2,
    Check = 3,
    CheckAck = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ControlKind {
    ActivationRequest = 1,
    ActivationAck = 2,
    Close = 3,
    Established = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ControlReason {
    Terminate = 0,
    Deactivate = 1,
}

/// `PeerTransport`.
#[derive(Debug, Clone, Copy)]
pub struct PeerTransport {
    pub transport_id: i32,
    pub addr: u32,
    pub port: u16,
    pub local_addr: u32,
    pub local_port: u16,
    pub npid: OrbisNpId,
    pub online_id: OrbisNpOnlineId,
    pub local_online_id: OrbisNpOnlineId,
    pub status: i32,
    pub state: ConnState,
    pub peer_activated: bool,
    pub peer_established: bool,
    pub transport_established_fired: bool,
    pub is_initiator: bool,
    pub probe_rtt_samples: [i32; 6],
    pub probe_sample_write_index: u32,
    pub last_echo_ping_us: i64,
    pub last_peer_rx_us: i64,
    pub last_handshake_send_us: i64,
}

impl Default for PeerTransport {
    fn default() -> Self {
        Self {
            transport_id: 0,
            addr: 0,
            port: 0,
            local_addr: 0,
            local_port: 0,
            npid: OrbisNpId::default(),
            online_id: OrbisNpOnlineId::default(),
            local_online_id: OrbisNpOnlineId::default(),
            status: ORBIS_NP_SIGNALING_CONN_STATUS_INACTIVE,
            state: ConnState::Inactive,
            peer_activated: false,
            peer_established: false,
            transport_established_fired: false,
            is_initiator: false,
            probe_rtt_samples: [-1; 6],
            probe_sample_write_index: 0,
            last_echo_ping_us: 0,
            last_peer_rx_us: 0,
            last_handshake_send_us: 0,
        }
    }
}

/// `ConnectionInfo`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ConnectionInfo {
    pub conn_id: OrbisNpSignalingConnectionId,
    pub ctx_id: OrbisNpSignalingContextId,
    pub transport_id: i32,
    pub status: i32,
    pub npid: OrbisNpId,
    pub online_id: OrbisNpOnlineId,
    pub sig1_established_event_fired: bool,
    pub locally_activated: bool,
    pub peer_activated: bool,
    pub peer_activated_fired: bool,
    pub mutual_fired: bool,
    pub dead_fired: bool,
}

/// `NpSignalingContext`.
#[derive(Debug, Default)]
pub struct NpSignalingContext {
    pub callback: usize,
    pub callback_arg: *mut core::ffi::c_void,
    pub active: bool,
    pub flags: u32,
    pub compiled_sdk_version: u32,
    pub account_id: u64,
    pub platform_type: u32,
    pub owner_npid: OrbisNpId,
    pub owner_online_id: OrbisNpOnlineId,
    pub bound_port: u16,
    pub ext_addr: AtomicU32,
    pub ext_port: AtomicU32,
    pub activate_budget_us: i64,
    pub activate_last_update_us: i64,
}

/// `CtxNpIdKey`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CtxNpIdKey {
    pub ctx_id: OrbisNpSignalingContextId,
    pub npid: OrbisNpId,
}

impl std::hash::Hash for CtxNpIdKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ctx_id.hash(state);
        // The C++ hashes the raw bytes of the NpId.
        // SAFETY: OrbisNpId is a plain struct of bytes.
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &self.npid as *const OrbisNpId as *const u8,
                core::mem::size_of::<OrbisNpId>(),
            )
        };
        state.write(bytes);
    }
}

/// `QueuedDispatch`.
#[derive(Debug, Clone, Copy)]
pub struct QueuedDispatch {
    pub fire_at_subsec_nanos: u64,
    pub delay_ms: u32,
    pub ctx_id: OrbisNpSignalingContextId,
    pub conn_id: OrbisNpSignalingConnectionId,
    pub event_type: u32,
    pub error_code: u32,
    pub callback: usize,
    pub callback_arg: *mut core::ffi::c_void,
}

/// `PeerNetInfoResult`.
#[derive(Debug, Clone, Copy, Default)]
pub struct PeerNetInfoResult {
    pub ctx_id: OrbisNpSignalingContextId,
    pub conn_id: OrbisNpSignalingConnectionId,
    pub local_ipv4: u32,
    pub external_ipv4: u32,
    pub nat_route_kind: u32,
}

/// `PendingActivation`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PendingActivation {
    pub conn_id: OrbisNpSignalingConnectionId,
    pub peer_online_id: String,
    pub start_handshake: bool,
}

/// `NpSignalingState`.
#[derive(Debug, Default)]
pub struct NpSignalingState {
    pub contexts: HashMap<i32, NpSignalingContext>,
    pub connections: HashMap<i32, ConnectionInfo>,
    pub peer_transports: HashMap<i32, PeerTransport>,
    pub npid_to_conn: HashMap<CtxNpIdKey, i32>,
    pub peer_netinfo_results: HashMap<i32, PeerNetInfoResult>,
    pub pending_activations: Vec<PendingActivation>,
    pub peer_netinfo_next_id: u32,
    pub peak_connection_count: u32,
    pub last_assigned_context_id: u32,
    pub connection_id_seed: u32,
    pub transport_id_seed: u32,
    pub initialized: AtomicBool,
    pub runtime_stop: AtomicBool,
}

impl NpSignalingState {
    pub fn new() -> Self {
        Self {
            peer_netinfo_next_id: 1,
            last_assigned_context_id: 0,
            connection_id_seed: 0,
            transport_id_seed: 0,
            ..Self::default()
        }
    }

    pub fn reset_sig1_data(&mut self) {
        self.contexts.clear();
        self.connections.clear();
        self.npid_to_conn.clear();
        self.peer_netinfo_results.clear();
        self.pending_activations.clear();
        self.peer_netinfo_next_id = 1;
        self.peak_connection_count = 0;
        self.last_assigned_context_id = 0;
        self.connection_id_seed = 0;
    }

    pub fn allocate_context_id(&mut self) -> OrbisNpSignalingContextId {
        self.last_assigned_context_id += 1;
        self.last_assigned_context_id as i32
    }

    pub fn allocate_connection_id(&mut self) -> OrbisNpSignalingConnectionId {
        self.connection_id_seed += 1;
        self.connection_id_seed as i32
    }

    pub fn allocate_transport_id(&mut self) -> i32 {
        self.transport_id_seed += 1;
        self.transport_id_seed as i32
    }

    pub fn remove_connection(&mut self, conn_id: OrbisNpSignalingConnectionId) {
        self.connections.remove(&conn_id);
        self.npid_to_conn.retain(|_, value| *value != conn_id);
    }

    pub fn remove_context_connections(&mut self, ctx_id: OrbisNpSignalingContextId) {
        self.connections
            .retain(|_, connection| connection.ctx_id != ctx_id);
        self.npid_to_conn.retain(|key, _| key.ctx_id != ctx_id);
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }
}

/// `OnlineIdToString`; the guest online id is a NUL-terminated 16-byte field.
pub fn online_id_to_string(online_id: &OrbisNpOnlineId) -> String {
    let length = online_id
        .data
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(online_id.data.len());
    String::from_utf8_lossy(&online_id.data[..length]).into_owned()
}

/// `OnlineIdEqualsString`.
pub fn online_id_equals_string(online_id: &OrbisNpOnlineId, value: &str) -> bool {
    online_id_to_string(online_id) == value
}

/// `NormalizeNpId`.
pub fn normalize_np_id(np_id: Option<&[u8]>) -> Option<OrbisNpId> {
    np_id.and_then(|bytes| {
        if bytes.len() != core::mem::size_of::<OrbisNpId>() {
            None
        } else {
            let mut id = OrbisNpId::default();
            // SAFETY: the source and destination have the same size and both are plain bytes.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    bytes.as_ptr(),
                    &mut id as *mut OrbisNpId as *mut u8,
                    bytes.len(),
                );
            }
            Some(id)
        }
    })
}

/// `MakeCtxNpIdKey`.
pub fn make_ctx_npid_key(ctx_id: OrbisNpSignalingContextId, npid: OrbisNpId) -> CtxNpIdKey {
    CtxNpIdKey { ctx_id, npid }
}

/// `ConnectionStateFromStatus`.
pub fn connection_state_from_status(status: i32) -> i32 {
    status
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signaling_packets_match_their_static_asserts() {
        assert_eq!(core::mem::size_of::<StunPing>(), 21);
        assert_eq!(core::mem::size_of::<StunEcho>(), 6);
        assert_eq!(core::mem::size_of::<SignalingEchoPing>(), 17);
        assert_eq!(core::mem::size_of::<SignalingEchoPong>(), 17);
        assert_eq!(core::mem::size_of::<SignalingHandshake>(), 0x3e);
        assert_eq!(core::mem::size_of::<SignalingControl>(), 0x32);
        assert_eq!(ConnState::Established as i32, 10);
        assert_eq!(HandshakeKind::CheckAck as u8, 4);
    }

    #[test]
    fn state_allocates_ids_and_removes_connections() {
        let mut state = NpSignalingState::new();
        assert_eq!(state.allocate_context_id(), 1);
        assert_eq!(state.allocate_connection_id(), 1);
        assert_eq!(state.allocate_transport_id(), 1);
        state.connections.insert(
            1,
            ConnectionInfo {
                conn_id: 1,
                ctx_id: 1,
                ..ConnectionInfo::default()
            },
        );
        state.connections.insert(
            2,
            ConnectionInfo {
                conn_id: 2,
                ctx_id: 2,
                ..ConnectionInfo::default()
            },
        );
        state.remove_connection(1);
        assert!(!state.connections.contains_key(&1));
        state.remove_context_connections(2);
        assert!(state.connections.is_empty());
        let mut online_id = OrbisNpOnlineId::default();
        online_id.data[..6].copy_from_slice(b"hunter");
        assert_eq!(online_id_to_string(&online_id), "hunter");
        assert!(online_id_equals_string(&online_id, "hunter"));
    }
}

impl NpSignalingState {
    /// `ConsumeActivationBudgetLocked`; a long budget is replenished over time.
    pub fn consume_activation_budget(&mut self, ctx_id: i32, now_us: i64) -> bool {
        let Some(context) = self.contexts.get_mut(&ctx_id) else {
            return false;
        };
        let elapsed = now_us.saturating_sub(context.activate_last_update_us);
        context.activate_budget_us =
            (context.activate_budget_us + elapsed).min(ACTIVATE_BUDGET_MAX_US);
        context.activate_last_update_us = now_us;
        if context.activate_budget_us < ACTIVATE_COOLDOWN_US {
            return false;
        }
        context.activate_budget_us -= ACTIVATE_COOLDOWN_US;
        true
    }

    /// `SnapshotConnectionStatisticsLocked`.
    pub fn snapshot_statistics(&self) -> (u32, u32, u32, u32) {
        let active = self.connections.len() as u32;
        let established = self
            .peer_transports
            .values()
            .filter(|peer| peer.state == ConnState::Established)
            .count() as u32;
        (
            self.peak_connection_count.max(active),
            active,
            0,
            established,
        )
    }

    /// `StagePeerNetInfoResultLocked`.
    pub fn stage_peer_net_info(&mut self, result: PeerNetInfoResult) -> OrbisNpSignalingRequestId {
        let id = self.peer_netinfo_next_id;
        self.peer_netinfo_next_id += 1;
        self.peer_netinfo_results.insert(id as i32, result);
        id
    }

    /// `TakePeerNetInfoResultLocked`.
    pub fn take_peer_net_info(&mut self, id: u32) -> Option<PeerNetInfoResult> {
        self.peer_netinfo_results.remove(&(id as i32))
    }

    /// `DispatchConnectionEvent`.
    pub fn dispatch_connection_event(
        &self,
        conn_id: i32,
        event_type: u32,
    ) -> Option<(usize, i32, u32)> {
        let connection = self.connections.get(&conn_id)?;
        let context = self.contexts.get(&connection.ctx_id)?;
        Some((context.callback, conn_id, event_type))
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn signaling_state_budgets_and_netinfo_results() {
        let mut state = NpSignalingState::new();
        let ctx = state.allocate_context_id();
        state.contexts.insert(ctx, NpSignalingContext::default());
        assert!(!state.consume_activation_budget(ctx, 0));
        assert!(state.consume_activation_budget(ctx, 7_000_000));
        assert_eq!(state.snapshot_statistics().1, 0);
        let id = state.stage_peer_net_info(PeerNetInfoResult::default());
        assert!(state.take_peer_net_info(id).is_some());
        assert!(state.take_peer_net_info(id).is_none());
    }
}
