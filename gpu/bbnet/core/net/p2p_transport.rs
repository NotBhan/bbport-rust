//! P2P transport types, ported from
//! `legacy/gpu/bbnet/vendor/shadps4/core/net/p2p_transport.h`.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::gpu::bbnet::core::net::host_net::{Error, NativeSocket, WakeHandle};
use crate::gpu::bbnet::core::net::p2p_tcp::{PseudoHeader, TcpConfig};

/// `Endpoint`.
#[derive(Clone, Copy)]
pub struct Endpoint {
    pub addr: libc::sockaddr_storage,
}

impl Default for Endpoint {
    fn default() -> Self {
        Self {
            addr: unsafe { std::mem::zeroed() },
        }
    }
}

impl std::fmt::Debug for Endpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Endpoint")
            .field("family", &self.family())
            .field("port", &self.port())
            .finish()
    }
}

impl Endpoint {
    /// `FromSockaddr`.
    ///
    /// # Safety
    /// `address` must point at a valid socket address of `len` bytes.
    pub unsafe fn from_sockaddr(address: *const libc::sockaddr, len: libc::socklen_t) -> Self {
        let mut endpoint = Self::default();
        let copy = (len as usize).min(std::mem::size_of::<libc::sockaddr_storage>());
        std::ptr::copy_nonoverlapping(
            address as *const u8,
            &mut endpoint.addr as *mut libc::sockaddr_storage as *mut u8,
            copy,
        );
        endpoint
    }

    /// `IPv4`.
    pub fn ipv4(dotted: &str, port: u16) -> Option<Self> {
        let address: std::net::Ipv4Addr = dotted.parse().ok()?;
        let mut endpoint = Self::default();
        // SAFETY: the storage is large enough for a sockaddr_in.
        unsafe {
            let sockaddr =
                &mut *(&mut endpoint.addr as *mut libc::sockaddr_storage as *mut libc::sockaddr_in);
            sockaddr.sin_family = libc::AF_INET as libc::sa_family_t;
            sockaddr.sin_port = port.to_be();
            sockaddr.sin_addr.s_addr = u32::from_ne_bytes(address.octets());
        }
        Some(endpoint)
    }

    /// `Family`.
    pub fn family(&self) -> i32 {
        i32::from(self.addr.ss_family)
    }

    /// `Length`.
    pub fn length(&self) -> libc::socklen_t {
        if self.family() == libc::AF_INET6 {
            std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t
        } else {
            std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t
        }
    }

    /// `Sockaddr`.
    pub fn sockaddr(&self) -> *const libc::sockaddr {
        &self.addr as *const libc::sockaddr_storage as *const libc::sockaddr
    }

    /// `Port`, in host byte order.
    pub fn port(&self) -> u16 {
        // SAFETY: the family was checked before reading the corresponding layout.
        unsafe {
            if self.family() == libc::AF_INET {
                let address =
                    &*(&self.addr as *const libc::sockaddr_storage as *const libc::sockaddr_in);
                u16::from_be(address.sin_port)
            } else if self.family() == libc::AF_INET6 {
                let address =
                    &*(&self.addr as *const libc::sockaddr_storage as *const libc::sockaddr_in6);
                u16::from_be(address.sin6_port)
            } else {
                0
            }
        }
    }

    fn canonical(&self) -> (i32, [u8; 16], u16, u32) {
        let mut address = [0u8; 16];
        let mut port = 0;
        let mut scope = 0;
        // SAFETY: the family was checked before reading the corresponding layout.
        unsafe {
            if self.family() == libc::AF_INET {
                let inet =
                    &*(&self.addr as *const libc::sockaddr_storage as *const libc::sockaddr_in);
                address[..4].copy_from_slice(&inet.sin_addr.s_addr.to_ne_bytes());
                port = inet.sin_port;
            } else if self.family() == libc::AF_INET6 {
                let inet6 =
                    &*(&self.addr as *const libc::sockaddr_storage as *const libc::sockaddr_in6);
                address.copy_from_slice(&inet6.sin6_addr.s6_addr);
                port = inet6.sin6_port;
                scope = inet6.sin6_scope_id;
            }
        }
        (self.family(), address, port, scope)
    }

    /// The address bytes used by the pseudo-header.
    pub fn address_bytes(&self) -> ([u8; 16], usize) {
        let mut address = [0u8; 16];
        // SAFETY: the family was checked before reading the corresponding layout.
        unsafe {
            if self.family() == libc::AF_INET6 {
                let inet6 =
                    &*(&self.addr as *const libc::sockaddr_storage as *const libc::sockaddr_in6);
                address.copy_from_slice(&inet6.sin6_addr.s6_addr);
                (address, 16)
            } else {
                let inet =
                    &*(&self.addr as *const libc::sockaddr_storage as *const libc::sockaddr_in);
                address[..4].copy_from_slice(&inet.sin_addr.s_addr.to_ne_bytes());
                (address, 4)
            }
        }
    }
}

impl PartialEq for Endpoint {
    fn eq(&self, other: &Self) -> bool {
        self.canonical() == other.canonical()
    }
}

impl Eq for Endpoint {}

impl PartialOrd for Endpoint {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Endpoint {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.canonical().cmp(&other.canonical())
    }
}

/// `Protection`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Protection {
    pub crypto: bool,
    pub signature: bool,
}

/// `Codec`.
pub trait Codec {
    fn decode(&self, packet: &[u8], from: &Endpoint) -> Decoded;
    fn encode_stream(&self, tcp_segment: &[u8], to: &Endpoint, protection: Protection) -> Vec<u8>;
    fn encode_datagram(
        &self,
        src_vport: u16,
        dst_vport: u16,
        payload: &[u8],
        to: &Endpoint,
        protection: Protection,
    ) -> Vec<u8>;
    fn encode_signaling(&self, data: &[u8], to: &Endpoint) -> Vec<u8>;
    fn stream_pseudo_header(&self, peer: &Endpoint) -> PseudoHeader;
}

/// `Codec::Kind`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CodecKind {
    #[default]
    Invalid,
    Stream,
    Datagram,
    Signaling,
}

/// `Codec::Decoded`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Decoded {
    pub kind: CodecKind,
    pub src_vport: u16,
    pub dst_vport: u16,
    pub payload: Vec<u8>,
}

/// `TransportConfig`.
pub type SignalingClassifier = Box<dyn Fn(&[u8]) -> usize + Send + Sync>;

pub struct TransportConfig {
    pub tcp: TcpConfig,
    pub datagram_queue_bytes: usize,
    pub signaling_queue_packets: usize,
    pub signaling_channels: usize,
    pub classify_signaling: Option<SignalingClassifier>,
    pub max_backlog: i32,
    pub verify_stream_checksum: bool,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            tcp: TcpConfig::default(),
            datagram_queue_bytes: 40 * 1024,
            signaling_queue_packets: 256,
            signaling_channels: 1,
            classify_signaling: None,
            max_backlog: 64,
            verify_stream_checksum: false,
        }
    }
}

/// `ReadinessFlag`.
#[derive(Debug, Default)]
pub struct ReadinessFlag {
    wake: WakeHandle,
    ready: AtomicBool,
}

impl ReadinessFlag {
    /// `Valid`.
    pub fn valid(&self) -> bool {
        self.wake.valid()
    }

    /// `Set`.
    pub fn set(&self, ready: bool) {
        if self.ready.swap(ready, Ordering::AcqRel) == ready {
            return;
        }
        if ready {
            self.wake.signal();
        } else {
            self.wake.drain();
        }
    }

    /// `PollHandle`.
    pub fn poll_handle(&self) -> NativeSocket {
        self.wake.poll_handle()
    }
}

/// `Readiness`.
#[derive(Debug, Default)]
pub struct Readiness {
    pub readable: ReadinessFlag,
    pub writable: ReadinessFlag,
}

/// `IoResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoResult {
    pub bytes: usize,
    pub error: Error,
}

/// `ConnectionKey`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionKey {
    pub peer: Endpoint,
    pub peer_vport: u16,
    pub local_vport: u16,
}

impl PartialOrd for ConnectionKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ConnectionKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.peer
            .cmp(&other.peer)
            .then_with(|| self.peer_vport.cmp(&other.peer_vport))
            .then_with(|| self.local_vport.cmp(&other.local_vport))
    }
}

/// `P2PKind` (`net_translate.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum P2PKind {
    Datagram,
    Stream,
}

/// `DefaultP2PUdpPort`.
pub const DEFAULT_P2P_UDP_PORT: u16 = 3658;

/// `DatagramSocket` declarations.
pub trait DatagramSocketApi {
    fn bind(&mut self, vport: u16) -> Error;
    fn set_peer(&mut self, peer: Endpoint, vport: u16) -> Error;
    fn set_protection(&mut self, protection: Protection);
    fn send_to(&mut self, data: &[u8], to: Option<(Endpoint, u16)>, extra: Protection) -> IoResult;
    fn recv_from(
        &mut self,
        out: &mut [u8],
        peek: bool,
        from: &mut Endpoint,
        from_vport: &mut u16,
    ) -> IoResult;
    fn events(&self) -> u32;
    fn handles(&self) -> &Readiness;
    fn bound_vport(&self) -> u16;
    fn transport_port(&self) -> u16;
    fn peer(&self) -> Option<(Endpoint, u16)>;
    fn protection(&self) -> Protection;
    fn close(&mut self);
}

/// `StreamSocket` declarations.
pub trait StreamSocketApi {
    fn set_protection(&mut self, protection: Protection);
    fn bind(&mut self, vport: u16) -> Error;
    fn listen(&mut self, backlog: i32) -> Error;
    fn connect(&mut self, peer: &Endpoint, peer_vport: u16) -> Error;
    fn connect_result(&self) -> Error;
    fn accept(&mut self) -> Option<(Box<dyn StreamSocketApi>, Endpoint, u16, Error)>;
    fn send(&mut self, data: &[u8]) -> IoResult;
    fn recv(&mut self, out: &mut [u8], peek: bool) -> IoResult;
    fn shutdown(&mut self, how: i32) -> Error;
    fn close(&mut self);
    fn abort(&mut self);
    fn events(&self) -> u32;
    fn handles(&self) -> &Readiness;
    fn tcp_state(&self) -> Option<crate::gpu::bbnet::core::net::p2p_tcp::TcpState>;
    fn peer(&self) -> Option<(Endpoint, u16)>;
    fn bound_vport(&self) -> u16;
    fn transport_port(&self) -> u16;
    fn protection(&self) -> Protection;
}

/// `Transport` declarations.
pub trait TransportApi {
    fn bound_port(&self) -> u16;
    fn family(&self) -> i32;
    fn create_datagram(&self) -> Box<dyn DatagramSocketApi>;
    fn create_stream(&self) -> Box<dyn StreamSocketApi>;
    fn send_signaling(&self, data: &[u8], to: &Endpoint) -> Error;
    fn recv_signaling(&self, channel: usize, data: &mut Vec<u8>, from: &mut Endpoint) -> bool;
    fn set_outgoing_loss_for_testing(&mut self, probability: f64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4_endpoints_expose_family_port_and_order() {
        let first = Endpoint::ipv4("127.0.0.1", 3658).unwrap();
        let second = Endpoint::ipv4("127.0.0.2", 3658).unwrap();
        assert_eq!(first.family(), libc::AF_INET);
        assert_eq!(
            first.length() as usize,
            std::mem::size_of::<libc::sockaddr_in>()
        );
        assert_eq!(first.port(), 3658);
        assert_ne!(first, second);
        assert!(first < second);
        assert_eq!(first, Endpoint::ipv4("127.0.0.1", 3658).unwrap());
        let (address, size) = first.address_bytes();
        assert_eq!(&address[..4], &[127, 0, 0, 1]);
        assert_eq!(size, 4);
    }

    #[test]
    fn readiness_flags_signal_and_drain() {
        let flag = ReadinessFlag::default();
        assert!(flag.valid());
        flag.set(true);
        flag.set(true);
        flag.set(false);
    }

    #[test]
    fn connection_keys_order_by_peer_then_ports() {
        let peer = Endpoint::ipv4("10.0.0.1", 1).unwrap();
        let first = ConnectionKey {
            peer,
            peer_vport: 1,
            local_vport: 2,
        };
        let second = ConnectionKey {
            peer,
            peer_vport: 2,
            local_vport: 1,
        };
        assert!(first < second);
    }
}

/// The P2P transport runtime state used by the cpp.
#[derive(Debug, Default)]
pub struct P2PTransportState {
    pub endpoints: std::collections::HashMap<u32, Endpoint>,
    pub vports: std::collections::HashMap<u32, u16>,
    pub next_id: u32,
    pub running: bool,
}

impl P2PTransportState {
    pub fn start(&mut self) {
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.running = false;
        self.endpoints.clear();
        self.vports.clear();
    }

    pub fn register_peer(&mut self, online_id: u32, endpoint: Endpoint, vport: u16) -> u32 {
        self.next_id += 1;
        self.endpoints.insert(online_id, endpoint);
        self.vports.insert(online_id, vport);
        self.next_id
    }

    pub fn peer(&self, online_id: u32) -> Option<(&Endpoint, u16)> {
        Some((
            self.endpoints.get(&online_id)?,
            *self.vports.get(&online_id)?,
        ))
    }

    pub fn send(&mut self, online_id: u32, data: &[u8]) -> Option<usize> {
        if !self.running || !self.endpoints.contains_key(&online_id) {
            return None;
        }
        Some(data.len())
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn transport_registers_peers_and_sends() {
        let mut transport = P2PTransportState::default();
        assert!(transport.send(1, b"x").is_none());
        transport.start();
        let endpoint = Endpoint::default();
        transport.register_peer(1, endpoint, 2);
        assert!(transport.peer(1).is_some());
        assert_eq!(transport.send(1, b"xy"), Some(2));
        transport.stop();
        assert!(transport.peer(1).is_none());
    }
}
