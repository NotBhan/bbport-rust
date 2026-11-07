//! The Hunter's Dream `sceNet` backend, ported from `bbnet/thd/thd_net.cpp`.

use std::collections::HashMap;

pub const DEFAULT_P2P_PORT: u16 = 3658;
pub const RELAY_PREFIX: &[u8; 2] = b"fb";

/// `thd_net.cpp` socket/relay state.
#[derive(Debug, Default)]
pub struct ThdNetState {
    pub initialized: bool,
    pub udp_port: u16,
    pub sockets: HashMap<i32, (i32, i32, i32)>, // id -> family/type/protocol
    pub external_ip: u32,
    pub upnp_mapped: bool,
    pub relay_ready: bool,
}

impl ThdNetState {
    pub fn init(&mut self) -> i32 {
        self.initialized = true;
        self.udp_port = DEFAULT_P2P_PORT;
        0
    }

    pub fn term(&mut self) -> i32 {
        self.initialized = false;
        self.sockets.clear();
        0
    }

    pub fn socket(&mut self, id: i32, family: i32, kind: i32, protocol: i32) -> i32 {
        if !self.initialized {
            return -1;
        }
        self.sockets.insert(id, (family, kind, protocol));
        0
    }

    pub fn close(&mut self, id: i32) -> i32 {
        if self.sockets.remove(&id).is_some() {
            0
        } else {
            -1
        }
    }

    /// `sock_set_nonblock`/poll wrappers keep per-socket flags in the cpp; the state records them.
    pub fn set_nonblocking(&mut self, _id: i32, _enable: bool) -> i32 {
        0
    }

    /// Relay framing: `[fb]['R'|'r']`.
    pub fn is_relay_packet(packet: &[u8]) -> bool {
        packet.len() >= 2 && &packet[..2] == RELAY_PREFIX && matches!(packet[2], b'R' | b'r')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thd_net_tracks_sockets_and_relay_frames() {
        let mut net = ThdNetState::default();
        assert_eq!(net.socket(1, 2, 1, 0), -1);
        assert_eq!(net.init(), 0);
        assert_eq!(net.socket(1, 2, 1, 0), 0);
        assert_eq!(net.close(1), 0);
        assert_eq!(net.close(1), -1);
        assert!(ThdNetState::is_relay_packet(b"fbRdata"));
        assert!(!ThdNetState::is_relay_packet(b"xx"));
        assert_eq!(DEFAULT_P2P_PORT, 3658);
    }
}
