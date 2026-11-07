//! Matching2 matchmaking client, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_matching2/np_matching2_mm.h`.

use crate::gpu::core::libraries::np::np_types2::{
    OrbisNpMatching2ContextId, OrbisNpMatching2RequestId,
};

/// `MmCommand`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum MmCommand {
    ContextStart = 100,
    CreateRoom = 101,
    JoinRoom = 102,
    LeaveRoom = 103,
    SearchRoom = 104,
    RequestSignalingInfos = 105,
    ContextStop = 106,
    SetUserInfo = 107,
    SetRoomDataInternal = 108,
    SetRoomDataExternal = 109,
    KickoutRoomMember = 110,
    GetWorldInfoList = 111,
    GetRoomDataExternalList = 112,
    GetUserInfoList = 113,
    GetRoomMemberDataExternalList = 114,
    SendRoomMessage = 115,
    SetRoomMemberDataInternal = 116,
}

/// The matchmaking callbacks used by the MM client.
pub trait MmApi {
    fn is_client_running(&self) -> bool;
    fn context_start(&mut self, ctx_id: OrbisNpMatching2ContextId);
    fn context_stop(&mut self, ctx_id: OrbisNpMatching2ContextId) -> i32;
    fn submit_request(
        &mut self,
        ctx_id: OrbisNpMatching2ContextId,
        req_id: OrbisNpMatching2RequestId,
        command: MmCommand,
        payload: &[u8],
    ) -> i32;
    fn mm_server_addr(&self) -> u32;
    fn mm_server_udp_port(&self) -> u16;
    fn request_signaling_infos(&mut self, online_id: &str) -> Option<(u32, u16)>;
}

pub const SERVER_ADDR: u32 = 0;
pub const SERVER_UDP_PORT: u16 = 0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mm_commands_match_the_protocol() {
        assert_eq!(MmCommand::ContextStart as u16, 100);
        assert_eq!(MmCommand::SetRoomMemberDataInternal as u16, 116);
        assert_eq!(SERVER_ADDR, 0);
        assert_eq!(SERVER_UDP_PORT, 0);
    }
}

/// The matchmaking shadNet client state.
#[derive(Debug, Default)]
pub struct MmClientState {
    pub running: bool,
    pub server_host: String,
    pub tcp_port: u16,
    pub pending: std::collections::HashMap<u64, MmCommand>,
    pub next_packet: u64,
}

impl MmClientState {
    pub fn set_client(&mut self, host: &str, port: u16) {
        self.server_host = host.to_string();
        self.tcp_port = port;
        self.running = true;
        self.next_packet = 1;
    }

    pub fn clear_client(&mut self) {
        self.running = false;
        self.server_host.clear();
        self.pending.clear();
    }

    pub fn submit(
        &mut self,
        ctx_id: OrbisNpMatching2ContextId,
        req_id: OrbisNpMatching2RequestId,
        command: MmCommand,
        payload: &[u8],
    ) -> i32 {
        if !self.running || payload.is_empty() {
            return -1;
        }
        let _ = (ctx_id, req_id);
        let packet = self.next_packet;
        self.next_packet += 1;
        self.pending.insert(packet, command);
        packet as i32
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn mm_client_tracks_running_and_pending_requests() {
        let mut client = MmClientState::default();
        assert_eq!(client.submit(1, 1, MmCommand::ContextStart, b"x"), -1);
        client.set_client("host", 31313);
        assert!(client.running);
        assert_eq!(client.submit(1, 1, MmCommand::CreateRoom, b"payload"), 1);
        assert_eq!(client.pending.get(&1), Some(&MmCommand::CreateRoom));
        client.clear_client();
        assert!(!client.running);
    }
}
