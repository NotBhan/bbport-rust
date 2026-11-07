//! P2P control surface, ported from
//! `bbnet/vendor/shadps4/core/libraries/net/net_p2p.h`.

/// The P2P transport controls used by the network and NP libraries.
pub trait P2PControl {
    fn configure_p2p(&mut self, udp_port: u16, advertised_addr: u32);
    fn start_p2p(&mut self, udp_port: u16) -> u16;
    fn stop_p2p(&mut self);
    fn ensure_transport(&mut self) -> bool;
    fn transport_ready(&self) -> bool;
    fn configured_port(&self) -> u16;
    fn bound_port(&self) -> u16;
    fn advertised_addr(&self) -> u32;
    fn advertised_port(&self) -> u16;
    fn signaling_send(&mut self, data: &[u8], dest_addr: u32, dest_port: u16) -> i32;
    fn signaling_recv(&mut self, buf: &mut [u8], from_addr: &mut u32, from_port: &mut u16) -> i32;
    fn control_send(&mut self, data: &[u8], dest_addr: u32, dest_port: u16) -> i32;
    fn control_recv(&mut self, buf: &mut [u8], from_addr: &mut u32, from_port: &mut u16) -> i32;
    fn matching_send(&mut self, data: &[u8], dest_addr: u32, dest_port: u16) -> i32;
    fn matching_recv(&mut self, buf: &mut [u8], from_addr: &mut u32, from_port: &mut u16) -> i32;
}

/// The signaling channel a packet belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalingChannel {
    Signaling = 0,
    Control = 1,
    Matching2 = 2,
}

/// `ClassifySignaling`; shadNet control and Matching2 packets start with `SHAD`.
pub fn classify_signaling(packet: &[u8]) -> SignalingChannel {
    if packet.len() < 5 || &packet[..4] != b"SHAD" {
        return SignalingChannel::Signaling;
    }
    match packet[4] {
        0x10 => SignalingChannel::Control,
        0x21 => SignalingChannel::Matching2,
        _ => SignalingChannel::Signaling,
    }
}

/// `P2PState` default configured port.
pub const DEFAULT_P2P_PORT: u16 = 3658;

/// The P2P state fields the signaling path reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct P2PState {
    pub configured_port: u16,
    pub keyring_present: bool,
}

impl Default for P2PState {
    fn default() -> Self {
        Self {
            configured_port: DEFAULT_P2P_PORT,
            keyring_present: true,
        }
    }
}

#[cfg(test)]
mod signaling_tests {
    use super::*;

    #[test]
    fn packets_are_classified_by_their_header() {
        assert_eq!(classify_signaling(b"SHAD\x10x"), SignalingChannel::Control);
        assert_eq!(
            classify_signaling(b"SHAD\x21x"),
            SignalingChannel::Matching2
        );
        assert_eq!(
            classify_signaling(b"SHAD\x99x"),
            SignalingChannel::Signaling
        );
        assert_eq!(classify_signaling(b"abc"), SignalingChannel::Signaling);
        assert_eq!(P2PState::default().configured_port, 3658);
        assert!(P2PState::default().keyring_present);
    }
}
