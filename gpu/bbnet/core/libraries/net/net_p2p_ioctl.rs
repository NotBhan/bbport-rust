//! P2P key ioctls, ported from
//! `bbnet/vendor/shadps4/core/libraries/net/net_p2p_ioctl.h` and `net_p2p_ioctl.cpp`.

use crate::gpu::bbnet::core::libraries::net::error::*;
use crate::gpu::bbnet::core::libraries::net::log::format_sockaddr;
use crate::gpu::bbnet::core::libraries::net::types::*;
use crate::gpu::bbnet::core::net::p2p_codec::{KeyEntry, Keyring};
use crate::gpu::bbnet::core::net::p2p_crypto::derive_communication_id;
use crate::gpu::bbnet::core::net::p2p_transport::Endpoint;
use crate::gpu::common::logging::classes;

const ADDRESS_SIZE: usize = core::mem::size_of::<OrbisNetSockaddrIn>();
const RECORD_SIZE: usize = ADDRESS_SIZE + 20;
const KEY_OFFSET: usize = ADDRESS_SIZE;
const LOCAL_PORT_OFFSET: usize = RECORD_SIZE - 4;
const FLAGS_OFFSET: usize = RECORD_SIZE - 2;
const ID_OFFSET: usize = RECORD_SIZE - 1;
const DEFAULT_KEY_PORT: u16 = 3658;

fn read_entry(bytes: &[u8]) -> KeyEntry {
    let mut key = [0u8; 16];
    key.copy_from_slice(&bytes[KEY_OFFSET..KEY_OFFSET + 16]);
    KeyEntry {
        key: crate::gpu::bbnet::core::net::p2p_crypto::P2PKey { value: key },
        local_port: u16::from_ne_bytes([bytes[LOCAL_PORT_OFFSET], bytes[LOCAL_PORT_OFFSET + 1]]),
        flags: bytes[FLAGS_OFFSET],
        id: bytes[ID_OFFSET],
    }
}

fn read_peer(bytes: &[u8]) -> Option<Endpoint> {
    // SAFETY: the caller checked the record is at least `RECORD_SIZE` bytes.
    let inet: OrbisNetSockaddrIn =
        unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const OrbisNetSockaddrIn) };
    if inet.sin_family as u32 != ORBIS_NET_AF_INET {
        return None;
    }
    let mut host: libc::sockaddr_in = unsafe { core::mem::zeroed() };
    host.sin_family = libc::AF_INET as libc::sa_family_t;
    host.sin_port = inet.sin_port;
    host.sin_addr.s_addr = inet.sin_addr;
    // SAFETY: the host sockaddr is fully initialized and has the matching length.
    Some(unsafe {
        Endpoint::from_sockaddr(
            &host as *const libc::sockaddr_in as *const libc::sockaddr,
            core::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
        )
    })
}

fn write_key(bytes: &mut [u8], key: Option<crate::gpu::bbnet::core::net::p2p_crypto::P2PKey>) {
    match key {
        Some(key) => bytes[KEY_OFFSET..KEY_OFFSET + 16].copy_from_slice(&key.value),
        None => bytes[KEY_OFFSET..KEY_OFFSET + 16].fill(0),
    }
}

/// `P2PKeyIoctl`; `np_port` is in network byte order.
pub fn p2p_key_ioctl(keyring: &Keyring, np_port: u16, cmd: u64, data: Option<&mut [u8]>) -> i32 {
    let control = cmd as u32;
    if control & 0xff00 != 0x5000 {
        return ORBIS_NET_EINVAL;
    }
    if control == 0x2000_50c8 {
        crate::log_debug!(classes::LIB_NET, "P2P ioctl {:#x}: enable (no-op)", control);
        return 0;
    }
    if control & 0xff == 0xd9 {
        crate::log_debug!(
            classes::LIB_NET,
            "P2P ioctl {:#x}: communication ID cleared",
            control
        );
        keyring.set_communication_id(None);
        return 0;
    }
    let Some(bytes) = data else {
        return ORBIS_NET_EFAULT;
    };
    if control == 0x8024_50c8 || control == 0x8034_50c8 {
        if bytes.len() <= ID_OFFSET {
            return ORBIS_NET_EINVAL;
        }
        crate::log_debug!(
            classes::LIB_NET,
            "P2P ioctl {:#x}: keys with id {} removed",
            control,
            bytes[ID_OFFSET]
        );
        keyring.remove_keys_with_id(bytes[ID_OFFSET]);
        return 0;
    }
    let peer_text = || {
        // SAFETY: the length check guarantees a full sockaddr_in.
        let address = unsafe { &*(bytes.as_ptr() as *const OrbisNetSockaddr) };
        format_sockaddr(Some(address), ADDRESS_SIZE as u32)
    };

    match control & 0xff {
        0xc9 => {
            if bytes.len() < RECORD_SIZE {
                return ORBIS_NET_EINVAL;
            }
            let entry = read_entry(bytes);
            crate::log_debug!(
                classes::LIB_NET,
                "P2P ioctl {:#x}: default key set (id {})",
                control,
                entry.id
            );
            keyring.set_default_key(Some(entry.key));
            keyring.add_peer_key(
                &Endpoint::ipv4("127.0.0.1", DEFAULT_KEY_PORT).expect("a valid address"),
                &entry,
            );
            0
        }
        0xca => {
            if bytes.len() < RECORD_SIZE {
                return ORBIS_NET_EINVAL;
            }
            write_key(bytes, keyring.default_key());
            0
        }
        0xcb => {
            if bytes.len() < RECORD_SIZE {
                return ORBIS_NET_EINVAL;
            }
            let Some(peer) = read_peer(bytes) else {
                return ORBIS_NET_EINVAL;
            };
            let mut entry = read_entry(bytes);
            if entry.local_port == 0 {
                entry.local_port = np_port;
            }
            crate::log_debug!(
                classes::LIB_NET,
                "P2P ioctl {:#x}: key for peer {} added (id {}, flags {:#x})",
                control,
                peer_text(),
                entry.id,
                entry.flags
            );
            keyring.add_peer_key(&peer, &entry);
            0
        }
        0xcc => {
            if bytes.len() < RECORD_SIZE {
                return ORBIS_NET_EINVAL;
            }
            let Some(peer) = read_peer(bytes) else {
                return ORBIS_NET_EINVAL;
            };
            let entry = read_entry(bytes);
            let released = keyring.release_peer_key(&peer, &entry.key, entry.id);
            crate::log_debug!(
                classes::LIB_NET,
                "P2P ioctl {:#x}: key for peer {} (id {}) {}",
                control,
                peer_text(),
                entry.id,
                if released {
                    "released"
                } else {
                    "not installed"
                }
            );
            if released {
                0
            } else {
                ORBIS_NET_ENOENT
            }
        }
        0xcd => {
            if bytes.len() < RECORD_SIZE {
                return ORBIS_NET_EINVAL;
            }
            crate::log_debug!(
                classes::LIB_NET,
                "P2P ioctl {:#x}: wildcard key set",
                control
            );
            keyring.set_wildcard_key(Some(read_entry(bytes).key));
            0
        }
        0xce => {
            if bytes.len() < RECORD_SIZE {
                return ORBIS_NET_EINVAL;
            }
            write_key(bytes, keyring.wildcard_key());
            0
        }
        0xcf | 0xd0 => {
            if bytes.len() < 2 {
                return ORBIS_NET_EINVAL;
            }
            bytes[..2].copy_from_slice(&np_port.to_ne_bytes());
            0
        }
        0xd8 => {
            if bytes.len() < 16 {
                return ORBIS_NET_EINVAL;
            }
            crate::log_debug!(
                classes::LIB_NET,
                "P2P ioctl {:#x}: communication ID set",
                control
            );
            let mut value = [0u8; 16];
            value.copy_from_slice(&bytes[..16]);
            keyring.set_communication_id(Some(derive_communication_id(&value)));
            0
        }
        _ => {
            crate::log_warning!(
                classes::LIB_NET,
                "P2P ioctl {:#x}: unknown command",
                control
            );
            ORBIS_NET_EINVAL
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::bbnet::core::net::p2p_crypto::P2PKey;

    fn record() -> Vec<u8> {
        let mut bytes = vec![0u8; RECORD_SIZE];
        bytes[0] = ADDRESS_SIZE as u8;
        bytes[1] = ORBIS_NET_AF_INET as u8;
        bytes[2..4].copy_from_slice(&8080u16.to_be_bytes());
        bytes[4..8].copy_from_slice(&0x7f00_0001u32.to_be_bytes());
        bytes[KEY_OFFSET..KEY_OFFSET + 16].copy_from_slice(&[5u8; 16]);
        bytes[LOCAL_PORT_OFFSET..LOCAL_PORT_OFFSET + 2].copy_from_slice(&3658u16.to_ne_bytes());
        bytes[FLAGS_OFFSET] = 1;
        bytes[ID_OFFSET] = 7;
        bytes
    }

    #[test]
    fn default_and_wildcard_keys_round_trip() {
        let keyring = Keyring::new();
        let mut bytes = record();
        assert_eq!(p2p_key_ioctl(&keyring, 0, 0x200050c9, Some(&mut bytes)), 0);
        assert_eq!(keyring.default_key(), Some(P2PKey { value: [5; 16] }));
        assert!(keyring
            .find_peer_entry(&Endpoint::ipv4("127.0.0.1", DEFAULT_KEY_PORT).unwrap())
            .is_some());

        let mut bytes = vec![0u8; RECORD_SIZE];
        assert_eq!(p2p_key_ioctl(&keyring, 0, 0x200050ca, Some(&mut bytes)), 0);
        assert_eq!(&bytes[KEY_OFFSET..KEY_OFFSET + 16], &[5u8; 16]);

        let mut bytes = vec![0u8; RECORD_SIZE];
        assert_eq!(p2p_key_ioctl(&keyring, 0, 0x200050ce, Some(&mut bytes)), 0);
        assert_eq!(&bytes[KEY_OFFSET..KEY_OFFSET + 16], &[0u8; 16]);

        let mut bytes = record();
        bytes[KEY_OFFSET..KEY_OFFSET + 16].copy_from_slice(&[9u8; 16]);
        assert_eq!(p2p_key_ioctl(&keyring, 0, 0x200050cd, Some(&mut bytes)), 0);
        assert_eq!(keyring.wildcard_key(), Some(P2PKey { value: [9; 16] }));

        let mut bytes = vec![0u8; RECORD_SIZE];
        assert_eq!(p2p_key_ioctl(&keyring, 0, 0x200050ce, Some(&mut bytes)), 0);
        assert_eq!(&bytes[KEY_OFFSET..KEY_OFFSET + 16], &[9u8; 16]);
    }

    #[test]
    fn peer_keys_are_added_and_released() {
        let keyring = Keyring::new();
        let mut bytes = record();
        assert_eq!(
            p2p_key_ioctl(&keyring, 0x1234, 0x200050cb, Some(&mut bytes)),
            0
        );
        let peer = Endpoint::ipv4("127.0.0.1", 8080).unwrap();
        let entry = keyring.find_peer_entry(&peer).unwrap();
        assert_eq!(entry.local_port, 3658);
        assert_eq!(entry.id, 7);

        let mut release = record();
        release[LOCAL_PORT_OFFSET..LOCAL_PORT_OFFSET + 2].copy_from_slice(&3658u16.to_ne_bytes());
        assert_eq!(
            p2p_key_ioctl(&keyring, 0x1234, 0x200050cc, Some(&mut release)),
            0
        );
        assert!(keyring.find_peer_entry(&peer).is_none());
        assert_eq!(
            p2p_key_ioctl(&keyring, 0x1234, 0x200050cc, Some(&mut release)),
            ORBIS_NET_ENOENT
        );
    }

    #[test]
    fn removal_by_id_and_communication_ids_work() {
        let keyring = Keyring::new();
        let mut bytes = record();
        assert_eq!(p2p_key_ioctl(&keyring, 0, 0x200050cb, Some(&mut bytes)), 0);
        assert_eq!(p2p_key_ioctl(&keyring, 0, 0x802450c8, Some(&mut bytes)), 0);
        assert!(keyring
            .find_peer_entry(&Endpoint::ipv4("127.0.0.1", 8080).unwrap())
            .is_none());

        let mut id = [1u8; 16];
        assert_eq!(p2p_key_ioctl(&keyring, 0, 0x200050d8, Some(&mut id)), 0);
        assert_eq!(
            keyring.communication_id(),
            Some(derive_communication_id(&[1u8; 16]))
        );
        assert_eq!(p2p_key_ioctl(&keyring, 0, 0x200050d9, None), 0);
        assert_eq!(keyring.communication_id(), None);
    }

    #[test]
    fn invalid_commands_and_buffers_fail() {
        let keyring = Keyring::new();
        assert_eq!(p2p_key_ioctl(&keyring, 0, 0x1234, None), ORBIS_NET_EINVAL);
        assert_eq!(
            p2p_key_ioctl(&keyring, 0, 0x200050c9, None),
            ORBIS_NET_EFAULT
        );
        let mut short = [0u8; 4];
        assert_eq!(
            p2p_key_ioctl(&keyring, 0, 0x200050cb, Some(&mut short)),
            ORBIS_NET_EINVAL
        );
        let mut bytes = record();
        bytes[1] = 99;
        assert_eq!(
            p2p_key_ioctl(&keyring, 0, 0x200050cb, Some(&mut bytes)),
            ORBIS_NET_EINVAL
        );
        assert_eq!(
            p2p_key_ioctl(&keyring, 0, 0x200050ff, Some(&mut bytes)),
            ORBIS_NET_EINVAL
        );
    }

    #[test]
    fn the_port_commands_write_the_np_port() {
        let keyring = Keyring::new();
        let mut bytes = vec![0u8; RECORD_SIZE];
        assert_eq!(
            p2p_key_ioctl(&keyring, 0x1234, 0x200050cf, Some(&mut bytes)),
            0
        );
        assert_eq!(&bytes[..2], &0x1234u16.to_ne_bytes());
    }
}
