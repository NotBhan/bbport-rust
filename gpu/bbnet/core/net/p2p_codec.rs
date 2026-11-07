//! The PS4 P2P wire format, ported from
//! `legacy/gpu/bbnet/vendor/shadps4/core/net/p2p_codec.h` and `p2p_codec.cpp`.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use crate::gpu::bbnet::core::net::host_net;
use crate::gpu::bbnet::core::net::p2p_crypto::{unprotect_payload, P2PKey};
use crate::gpu::bbnet::core::net::p2p_tcp::PseudoHeader;
use crate::gpu::bbnet::core::net::p2p_transport::{Codec, Decoded, Endpoint, Protection};

/// `Wire`.
pub mod wire {
    pub const MARKER: u8 = 0xFF;
    pub const FLAG_VALID: u8 = 0x80;
    pub const FLAG_BROADCAST: u8 = 0x40;
    pub const FLAG_DIGEST: u8 = 0x20;
    pub const KIND_DATAGRAM: u8 = 3;
    pub const KIND_STREAM: u8 = 7;
    pub const SIGNALING_VPORT: u16 = 0xFFFF;
}

/// `CommunicationId`.
pub type CommunicationId = [u8; 4];

/// `KeyEntry`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyEntry {
    pub key: P2PKey,
    pub local_port: u16,
    pub flags: u8,
    pub id: u8,
}

#[derive(Debug, Clone)]
struct Installed {
    entry: KeyEntry,
    references: i32,
}

#[derive(Debug, Default)]
struct KeyringState {
    peer_keys: BTreeMap<Endpoint, Installed>,
    wildcard_key: Option<P2PKey>,
    default_key: Option<P2PKey>,
    communication_id: Option<CommunicationId>,
}

/// `Keyring`.
#[derive(Debug, Default)]
pub struct Keyring {
    state: Mutex<KeyringState>,
}

impl Keyring {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, KeyringState> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    /// `AddPeerKey`.
    pub fn add_peer_key(&self, peer: &Endpoint, entry: &KeyEntry) {
        let mut state = self.lock();
        match state.peer_keys.get_mut(peer) {
            Some(installed) if installed.entry.key == entry.key => {
                installed.references += 1;
            }
            _ => {
                state.peer_keys.insert(
                    *peer,
                    Installed {
                        entry: entry.clone(),
                        references: 1,
                    },
                );
            }
        }
    }

    /// `ReleasePeerKey`.
    pub fn release_peer_key(&self, peer: &Endpoint, key: &P2PKey, id: u8) -> bool {
        let mut state = self.lock();
        let Some(installed) = state.peer_keys.get_mut(peer) else {
            return false;
        };
        if installed.entry.key != *key || installed.entry.id != id {
            return false;
        }
        installed.references -= 1;
        if installed.references == 0 {
            state.peer_keys.remove(peer);
        }
        true
    }

    /// `RemoveKeysWithId`.
    pub fn remove_keys_with_id(&self, id: u8) {
        self.lock()
            .peer_keys
            .retain(|_, installed| installed.entry.id != id);
    }

    /// `SetPeerKey`.
    pub fn set_peer_key(&self, peer: &Endpoint, key: &P2PKey) {
        self.lock().peer_keys.insert(
            *peer,
            Installed {
                entry: KeyEntry {
                    key: *key,
                    ..KeyEntry::default()
                },
                references: 1,
            },
        );
    }

    /// `RemovePeerKey`.
    pub fn remove_peer_key(&self, peer: &Endpoint) {
        self.lock().peer_keys.remove(peer);
    }

    /// `SetWildcardKey`.
    pub fn set_wildcard_key(&self, key: Option<P2PKey>) {
        self.lock().wildcard_key = key;
    }

    /// `GetWildcardKey`.
    pub fn wildcard_key(&self) -> Option<P2PKey> {
        self.lock().wildcard_key
    }

    /// `SetDefaultKey`.
    pub fn set_default_key(&self, key: Option<P2PKey>) {
        self.lock().default_key = key;
    }

    /// `GetDefaultKey`.
    pub fn default_key(&self) -> Option<P2PKey> {
        self.lock().default_key
    }

    /// `Find`.
    pub fn find(&self, peer: &Endpoint) -> Option<P2PKey> {
        let state = self.lock();
        if let Some(installed) = state.peer_keys.get(peer) {
            return Some(installed.entry.key);
        }
        state.wildcard_key.or(state.default_key)
    }

    /// `FindPeerEntry`.
    pub fn find_peer_entry(&self, peer: &Endpoint) -> Option<KeyEntry> {
        self.lock()
            .peer_keys
            .get(peer)
            .map(|installed| installed.entry.clone())
    }

    /// `SetCommunicationId`.
    pub fn set_communication_id(&self, id: Option<CommunicationId>) {
        self.lock().communication_id = id;
    }

    /// `GetCommunicationId`.
    pub fn communication_id(&self) -> Option<CommunicationId> {
        self.lock().communication_id
    }
}

/// `BroadcastObserver`.
pub type BroadcastObserver = Box<dyn Fn(&[u8], &Endpoint) + Send + Sync>;

/// `FramingCodec`.
pub struct FramingCodec {
    keyring: Arc<Keyring>,
    broadcast_observer: Option<BroadcastObserver>,
    reported_broadcasts: AtomicI32,
    pseudo_cache: Mutex<BTreeMap<Endpoint, PseudoHeader>>,
}

impl FramingCodec {
    /// `MaxReportedBroadcasts`.
    pub const MAX_REPORTED_BROADCASTS: i32 = 16;

    /// `FramingCodec(keyring, observer)`.
    pub fn new(keyring: Arc<Keyring>, broadcast_observer: Option<BroadcastObserver>) -> Self {
        Self {
            keyring,
            broadcast_observer,
            reported_broadcasts: AtomicI32::new(0),
            pseudo_cache: Mutex::new(BTreeMap::new()),
        }
    }

    fn protect(&self, data: &[u8], to: &Endpoint, protection: Protection) -> (Vec<u8>, u8) {
        if protection.crypto || protection.signature {
            if let Some(key) = self.keyring.find(to) {
                if let Some(output) = crate::gpu::bbnet::core::net::p2p_crypto::protect_payload(
                    data,
                    protection.crypto,
                    protection.signature,
                    &key,
                ) {
                    let mode = u8::from(protection.crypto) | (u8::from(protection.signature) << 1);
                    return (output, mode);
                }
            }
        }
        (data.to_vec(), 0)
    }
}

/// `FrameDatagram`.
pub fn frame_datagram(
    src_vport: u16,
    dst_vport: u16,
    kind: u8,
    communication_id: Option<CommunicationId>,
    body: &[u8],
) -> Vec<u8> {
    let mut packet = vec![
        wire::MARKER,
        wire::FLAG_VALID
            | kind
            | if communication_id.is_some() {
                wire::FLAG_DIGEST
            } else {
                0
            },
        (src_vport >> 8) as u8,
        src_vport as u8,
        (dst_vport >> 8) as u8,
        dst_vport as u8,
    ];
    if let Some(id) = communication_id {
        packet.extend_from_slice(&id);
    }
    packet.extend_from_slice(body);
    packet
}

impl Codec for FramingCodec {
    fn decode(&self, packet: &[u8], from: &Endpoint) -> Decoded {
        use crate::gpu::bbnet::core::net::p2p_transport::{CodecKind, Decoded};
        let mut out = Decoded::default();
        if packet.len() < 2 || packet[0] != wire::MARKER || packet[1] & wire::FLAG_VALID == 0 {
            return out;
        }
        let flags = packet[1];
        let kind = flags & 0x0F;
        if !(wire::KIND_DATAGRAM..=wire::KIND_STREAM + 3).contains(&kind) {
            return out;
        }
        let stream = kind >= wire::KIND_STREAM;

        let mut offset = 2;
        let mut src_vport = 0;
        let mut dst_vport = 0;
        if flags & wire::FLAG_BROADCAST != 0 {
            if let Some(observer) = &self.broadcast_observer {
                if self.reported_broadcasts.fetch_add(1, Ordering::Relaxed)
                    < Self::MAX_REPORTED_BROADCASTS
                {
                    observer(packet, from);
                }
            }
            if stream {
                return out;
            }
            offset = 4;
            if packet.len() < offset {
                return out;
            }
            dst_vport = u16::from_be_bytes([packet[2], packet[3]]);
            src_vport = dst_vport;
        } else if !stream {
            offset = 6;
            if packet.len() < offset {
                return out;
            }
            src_vport = u16::from_be_bytes([packet[2], packet[3]]);
            dst_vport = u16::from_be_bytes([packet[4], packet[5]]);
        }

        let mut communication_id: Option<CommunicationId> = None;
        if flags & wire::FLAG_DIGEST != 0 {
            if packet.len() < offset + 4 {
                return out;
            }
            let mut id = [0u8; 4];
            id.copy_from_slice(&packet[offset..offset + 4]);
            communication_id = Some(id);
            offset += 4;
        }

        let signaling = !stream && dst_vport == wire::SIGNALING_VPORT;
        if !signaling {
            if let Some(expected) = self.keyring.communication_id() {
                if communication_id != Some(expected) {
                    return out;
                }
            }
        }

        let mode = kind
            - if stream {
                wire::KIND_STREAM
            } else {
                wire::KIND_DATAGRAM
            };
        let body = &packet[offset..];
        if mode != 0 {
            let Some(key) = self.keyring.find(from) else {
                return out;
            };
            let Some(payload) = unprotect_payload(body, mode & 1 != 0, mode & 2 != 0, &key) else {
                return out;
            };
            out.payload = payload;
        } else {
            out.payload = body.to_vec();
        }

        out.kind = if stream {
            CodecKind::Stream
        } else if signaling {
            CodecKind::Signaling
        } else {
            CodecKind::Datagram
        };
        out.src_vport = src_vport;
        out.dst_vport = dst_vport;
        out
    }

    fn encode_stream(&self, tcp_segment: &[u8], to: &Endpoint, protection: Protection) -> Vec<u8> {
        let (body, mode) = self.protect(tcp_segment, to, protection);
        let communication_id = self.keyring.communication_id();
        let mut packet = vec![
            wire::MARKER,
            wire::FLAG_VALID
                | (wire::KIND_STREAM + mode)
                | if communication_id.is_some() {
                    wire::FLAG_DIGEST
                } else {
                    0
                },
        ];
        if let Some(id) = communication_id {
            packet.extend_from_slice(&id);
        }
        packet.extend_from_slice(&body);
        packet
    }

    fn encode_datagram(
        &self,
        src_vport: u16,
        dst_vport: u16,
        payload: &[u8],
        to: &Endpoint,
        protection: Protection,
    ) -> Vec<u8> {
        let (body, mode) = self.protect(payload, to, protection);
        frame_datagram(
            src_vport,
            dst_vport,
            wire::KIND_DATAGRAM + mode,
            self.keyring.communication_id(),
            &body,
        )
    }

    fn encode_signaling(&self, data: &[u8], _to: &Endpoint) -> Vec<u8> {
        frame_datagram(
            wire::SIGNALING_VPORT,
            wire::SIGNALING_VPORT,
            wire::KIND_DATAGRAM,
            None,
            data,
        )
    }

    fn stream_pseudo_header(&self, peer: &Endpoint) -> PseudoHeader {
        let mut cache = self
            .pseudo_cache
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(pseudo) = cache.get(peer) {
            return pseudo.clone();
        }
        let mut pseudo = PseudoHeader {
            family: peer.family(),
            ..PseudoHeader::default()
        };
        let (remote, size) = peer.address_bytes();
        pseudo.remote[..size].copy_from_slice(&remote[..size]);

        let mut error = host_net::Error::Ok;
        let probe = host_net::create_socket(peer.family(), libc::SOCK_DGRAM, 0, &mut error);
        if probe != host_net::INVALID_SOCKET {
            // SAFETY: `peer` holds a valid socket address for the call.
            let connected = unsafe {
                host_net::connect(probe, peer.sockaddr(), peer.length()) == host_net::Error::Ok
            };
            if connected {
                let mut local: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
                let mut local_len =
                    std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
                // SAFETY: `local` and `local_len` are valid for the call.
                let result = unsafe {
                    libc::getsockname(
                        probe,
                        &mut local as *mut libc::sockaddr_storage as *mut libc::sockaddr,
                        &mut local_len,
                    )
                };
                if result == 0 {
                    // SAFETY: getsockname filled a valid socket address.
                    let local_endpoint = unsafe {
                        Endpoint::from_sockaddr(
                            &local as *const libc::sockaddr_storage as *const libc::sockaddr,
                            local_len,
                        )
                    };
                    let (local_address, local_size) = local_endpoint.address_bytes();
                    pseudo.local[..local_size].copy_from_slice(&local_address[..local_size]);
                }
            }
            host_net::close_socket(probe);
        }
        if cache.len() >= 256 {
            cache.clear();
        }
        cache.insert(*peer, pseudo.clone());
        pseudo
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::bbnet::core::net::p2p_transport::CodecKind;

    fn peer() -> Endpoint {
        Endpoint::ipv4("127.0.0.2", 3658).unwrap()
    }

    #[test]
    fn the_keyring_tracks_references_wildcards_and_defaults() {
        let keyring = Keyring::new();
        let peer = peer();
        let key = P2PKey { value: [1; 16] };
        let entry = KeyEntry {
            key,
            id: 3,
            ..KeyEntry::default()
        };
        keyring.add_peer_key(&peer, &entry);
        keyring.add_peer_key(&peer, &entry);
        assert_eq!(keyring.find(&peer), Some(key));
        assert!(keyring.release_peer_key(&peer, &key, 3));
        assert_eq!(keyring.find(&peer), Some(key));
        assert!(keyring.release_peer_key(&peer, &key, 3));
        assert_eq!(keyring.find(&peer), None);

        keyring.add_peer_key(&peer, &entry);
        keyring.remove_keys_with_id(3);
        assert_eq!(keyring.find_peer_entry(&peer), None);

        let wildcard = P2PKey { value: [2; 16] };
        let default = P2PKey { value: [3; 16] };
        keyring.set_default_key(Some(default));
        assert_eq!(keyring.find(&peer), Some(default));
        keyring.set_wildcard_key(Some(wildcard));
        assert_eq!(keyring.find(&peer), Some(wildcard));
        assert_eq!(keyring.wildcard_key(), Some(wildcard));

        keyring.set_communication_id(Some([9, 8, 7, 6]));
        assert_eq!(keyring.communication_id(), Some([9, 8, 7, 6]));
    }

    #[test]
    fn datagrams_round_trip_plain_and_protected() {
        let keyring = Arc::new(Keyring::new());
        let codec = FramingCodec::new(keyring.clone(), None);

        let encoded = codec.encode_datagram(4000, 5000, b"hello", &peer(), Protection::default());
        let decoded = codec.decode(&encoded, &peer());
        assert_eq!(decoded.kind, CodecKind::Datagram);
        assert_eq!(decoded.src_vport, 4000);
        assert_eq!(decoded.dst_vport, 5000);
        assert_eq!(decoded.payload, b"hello");

        let key = P2PKey { value: [7; 16] };
        keyring.set_peer_key(&peer(), &key);
        let protection = Protection {
            crypto: true,
            signature: true,
        };
        let encoded = codec.encode_datagram(1, 2, b"secret", &peer(), protection);
        let decoded = codec.decode(&encoded, &peer());
        assert_eq!(decoded.payload, b"secret");
    }

    #[test]
    fn signaling_round_trips_without_a_digest() {
        let keyring = Arc::new(Keyring::new());
        let codec = FramingCodec::new(keyring, None);
        let encoded = codec.encode_signaling(b"signal", &peer());
        let decoded = codec.decode(&encoded, &peer());
        assert_eq!(decoded.kind, CodecKind::Signaling);
        assert_eq!(decoded.src_vport, wire::SIGNALING_VPORT);
        assert_eq!(decoded.dst_vport, wire::SIGNALING_VPORT);
        assert_eq!(decoded.payload, b"signal");
    }

    #[test]
    fn the_communication_id_gates_decoding() {
        let sender = Arc::new(Keyring::new());
        sender.set_communication_id(Some([1, 2, 3, 4]));
        let codec = FramingCodec::new(sender, None);
        let encoded = codec.encode_datagram(1, 2, b"x", &peer(), Protection::default());

        let receiver = Arc::new(Keyring::new());
        receiver.set_communication_id(Some([4, 3, 2, 1]));
        let other = FramingCodec::new(receiver, None);
        assert_eq!(other.decode(&encoded, &peer()).kind, CodecKind::Invalid);
    }

    #[test]
    fn stream_segments_and_the_pseudo_header_carry_the_peer_address() {
        let keyring = Arc::new(Keyring::new());
        let codec = FramingCodec::new(keyring, None);
        let encoded = codec.encode_stream(b"segment", &peer(), Protection::default());
        let decoded = codec.decode(&encoded, &peer());
        assert_eq!(decoded.kind, CodecKind::Stream);
        assert_eq!(decoded.payload, b"segment");

        let pseudo = codec.stream_pseudo_header(&peer());
        assert_eq!(pseudo.family, libc::AF_INET);
        assert_eq!(&pseudo.remote[..4], &[127, 0, 0, 2]);
        let cached = codec.stream_pseudo_header(&peer());
        assert_eq!(pseudo, cached);
    }

    #[test]
    fn malformed_packets_are_rejected() {
        let keyring = Arc::new(Keyring::new());
        let codec = FramingCodec::new(keyring, None);
        assert_eq!(codec.decode(&[], &peer()).kind, CodecKind::Invalid);
        assert_eq!(codec.decode(&[0xFF], &peer()).kind, CodecKind::Invalid);
        assert_eq!(
            codec.decode(&[0xFF, wire::FLAG_VALID | 1], &peer()).kind,
            CodecKind::Invalid
        );
        assert_eq!(
            codec
                .decode(
                    &[wire::MARKER, wire::FLAG_VALID | wire::KIND_DATAGRAM, 0, 1],
                    &peer()
                )
                .kind,
            CodecKind::Invalid
        );
    }
}
