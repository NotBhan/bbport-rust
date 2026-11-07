//! Guest/host address and option translation, ported from
//! `bbnet/vendor/shadps4/core/libraries/net/net_translate.h` and `.cpp`.

use crate::gpu::bbnet::core::libraries::net::error::*;
use crate::gpu::bbnet::core::libraries::net::types::*;
use crate::gpu::bbnet::core::net::host_net;
use crate::gpu::bbnet::core::net::p2p_transport::{Endpoint, P2PKind, DEFAULT_P2P_UDP_PORT};

/// `HostSocketKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostSocketKind {
    pub family: i32,
    pub socket_type: i32,
    pub protocol: i32,
    pub p2p: bool,
}

/// `ToHostFamily`.
pub fn to_host_family(orbis_family: i32) -> Option<i32> {
    match orbis_family as u32 {
        ORBIS_NET_AF_UNIX => Some(libc::AF_UNIX),
        ORBIS_NET_AF_INET => Some(libc::AF_INET),
        ORBIS_NET_AF_INET6 => Some(libc::AF_INET6),
        _ => None,
    }
}

/// `ToOrbisFamily`.
pub fn to_orbis_family(host_family: i32) -> i32 {
    match host_family {
        libc::AF_UNIX => ORBIS_NET_AF_UNIX as i32,
        libc::AF_INET => ORBIS_NET_AF_INET as i32,
        libc::AF_INET6 => ORBIS_NET_AF_INET6 as i32,
        _ => 0,
    }
}

/// `ToHostSocketKind`; the error is the guest errno.
pub fn to_host_socket_kind(
    family: i32,
    socket_type: i32,
    protocol: i32,
) -> Result<HostSocketKind, i32> {
    if family as u32 == ORBIS_NET_AF_UNIX {
        if socket_type != ORBIS_NET_SOCK_STREAM as i32 && socket_type != ORBIS_NET_SOCK_DGRAM as i32
        {
            return Err(ORBIS_NET_EPROTONOSUPPORT);
        }
        return Ok(HostSocketKind {
            family: libc::AF_UNIX,
            socket_type: if socket_type == ORBIS_NET_SOCK_STREAM as i32 {
                libc::SOCK_STREAM
            } else {
                libc::SOCK_DGRAM
            },
            protocol: 0,
            p2p: false,
        });
    }
    if family as u32 != ORBIS_NET_AF_INET {
        return Err(ORBIS_NET_EPROTONOSUPPORT);
    }
    let host_family = to_host_family(family).expect("the family was checked");
    let mut kind = HostSocketKind {
        family: host_family,
        socket_type: 0,
        protocol,
        p2p: false,
    };
    match socket_type as u32 {
        ORBIS_NET_SOCK_STREAM => kind.socket_type = libc::SOCK_STREAM,
        ORBIS_NET_SOCK_DGRAM => kind.socket_type = libc::SOCK_DGRAM,
        ORBIS_NET_SOCK_RAW => kind.socket_type = libc::SOCK_RAW,
        ORBIS_NET_SOCK_DGRAM_P2P => {
            kind.socket_type = libc::SOCK_DGRAM;
            kind.p2p = true;
        }
        ORBIS_NET_SOCK_STREAM_P2P => {
            kind.socket_type = libc::SOCK_STREAM;
            kind.p2p = true;
        }
        _ => return Err(ORBIS_NET_EPROTONOSUPPORT),
    }
    if socket_type as u32 != ORBIS_NET_SOCK_RAW && protocol != 0 {
        let natural = if kind.p2p {
            -1
        } else if kind.socket_type == libc::SOCK_STREAM {
            ORBIS_NET_IPPROTO_TCP as i32
        } else {
            ORBIS_NET_IPPROTO_UDP as i32
        };
        if protocol != natural {
            return Err(ORBIS_NET_EPROTOTYPE);
        }
    }
    Ok(kind)
}

/// `ToOrbisSocketType`.
pub fn to_orbis_socket_type(host_type: i32, p2p: bool) -> i32 {
    if host_type == libc::SOCK_STREAM {
        return if p2p {
            ORBIS_NET_SOCK_STREAM_P2P as i32
        } else {
            ORBIS_NET_SOCK_STREAM as i32
        };
    }
    if host_type == libc::SOCK_DGRAM {
        return if p2p {
            ORBIS_NET_SOCK_DGRAM_P2P as i32
        } else {
            ORBIS_NET_SOCK_DGRAM as i32
        };
    }
    ORBIS_NET_SOCK_RAW as i32
}

fn read_orbis_in(addr: &OrbisNetSockaddr) -> OrbisNetSockaddrIn {
    // SAFETY: the caller guarantees the length covers a sockaddr_in.
    unsafe { core::ptr::read_unaligned(addr as *const OrbisNetSockaddr as *const _) }
}

fn read_orbis_in6(addr: &OrbisNetSockaddr) -> OrbisNetSockaddrIn6 {
    // SAFETY: the caller guarantees the length covers a sockaddr_in6.
    unsafe { core::ptr::read_unaligned(addr as *const OrbisNetSockaddr as *const _) }
}

/// `ToHostSockaddr`; returns the host storage and its length.
pub fn to_host_sockaddr(
    addr: &OrbisNetSockaddr,
    addrlen: u32,
) -> Result<(libc::sockaddr_storage, libc::socklen_t), i32> {
    if addrlen < 2 {
        return Err(ORBIS_NET_EINVAL);
    }
    let mut out: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
    match addr.sa_family as u32 {
        ORBIS_NET_AF_INET => {
            if addrlen < core::mem::size_of::<OrbisNetSockaddrIn>() as u32 {
                return Err(ORBIS_NET_EINVAL);
            }
            let inet = read_orbis_in(addr);
            // SAFETY: the storage is large enough for a sockaddr_in.
            let host = unsafe {
                &mut *(&mut out as *mut libc::sockaddr_storage as *mut libc::sockaddr_in)
            };
            host.sin_family = libc::AF_INET as libc::sa_family_t;
            host.sin_port = inet.sin_port;
            host.sin_addr.s_addr = inet.sin_addr;
            Ok((
                out,
                core::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
            ))
        }
        ORBIS_NET_AF_INET6 => {
            if addrlen < core::mem::size_of::<OrbisNetSockaddrIn6>() as u32 {
                return Err(ORBIS_NET_EINVAL);
            }
            let inet6 = read_orbis_in6(addr);
            // SAFETY: the storage is large enough for a sockaddr_in6.
            let host = unsafe {
                &mut *(&mut out as *mut libc::sockaddr_storage as *mut libc::sockaddr_in6)
            };
            host.sin6_family = libc::AF_INET6 as libc::sa_family_t;
            host.sin6_port = inet6.sin6_port;
            host.sin6_flowinfo = inet6.sin6_flowinfo;
            host.sin6_addr.s6_addr = inet6.sin6_addr;
            host.sin6_scope_id = inet6.sin6_scope_id;
            Ok((
                out,
                core::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t,
            ))
        }
        ORBIS_NET_AF_UNIX => {
            let size = addrlen.min(core::mem::size_of::<OrbisNetSockaddrUn>() as u32);
            // SAFETY: `size` bounds the bytes read after the family field.
            let path = unsafe {
                let bytes = core::slice::from_raw_parts(
                    (addr as *const OrbisNetSockaddr).cast::<u8>().add(2),
                    (size - 2) as usize,
                );
                let end = bytes
                    .iter()
                    .position(|byte| *byte == 0)
                    .unwrap_or(bytes.len());
                &bytes[..end]
            };
            // SAFETY: the storage is large enough for a sockaddr_un.
            let host = unsafe {
                &mut *(&mut out as *mut libc::sockaddr_storage as *mut libc::sockaddr_un)
            };
            if path.len() >= host.sun_path.len() {
                return Err(ORBIS_NET_ENAMETOOLONG);
            }
            host.sun_family = libc::AF_UNIX as libc::sa_family_t;
            for (slot, byte) in host.sun_path.iter_mut().zip(path.iter()) {
                *slot = *byte as i8;
            }
            Ok((
                out,
                (core::mem::size_of::<libc::sa_family_t>() + path.len() + 1) as libc::socklen_t,
            ))
        }
        _ => Err(ORBIS_NET_EAFNOSUPPORT),
    }
}

/// `CopyOut`: writes at most `out.len()` bytes, returns the required length.
pub fn copy_sockaddr(full: &[u8], out: &mut [u8]) -> u32 {
    let count = out.len().min(full.len());
    out[..count].copy_from_slice(&full[..count]);
    full.len() as u32
}

/// `ToOrbisSockaddr`; returns the number of bytes the guest would need.
pub fn to_orbis_sockaddr(
    addr: &libc::sockaddr_storage,
    len: libc::socklen_t,
    out: &mut [u8],
) -> u32 {
    match i32::from(addr.ss_family) {
        libc::AF_INET if len as usize >= core::mem::size_of::<libc::sockaddr_in>() => {
            // SAFETY: the family and length guarantee a sockaddr_in.
            let host =
                unsafe { &*(addr as *const libc::sockaddr_storage as *const libc::sockaddr_in) };
            let inet = OrbisNetSockaddrIn {
                sin_len: core::mem::size_of::<OrbisNetSockaddrIn>() as u8,
                sin_family: ORBIS_NET_AF_INET as u8,
                sin_port: host.sin_port,
                sin_addr: host.sin_addr.s_addr,
                sin_vport: 0,
                sin_zero: [0; 6],
            };
            // SAFETY: the struct is plain bytes.
            let bytes = unsafe {
                core::slice::from_raw_parts(
                    &inet as *const OrbisNetSockaddrIn as *const u8,
                    core::mem::size_of::<OrbisNetSockaddrIn>(),
                )
            };
            copy_sockaddr(bytes, out)
        }
        libc::AF_INET6 if len as usize >= core::mem::size_of::<libc::sockaddr_in6>() => {
            // SAFETY: the family and length guarantee a sockaddr_in6.
            let host =
                unsafe { &*(addr as *const libc::sockaddr_storage as *const libc::sockaddr_in6) };
            let inet6 = OrbisNetSockaddrIn6 {
                sin6_len: core::mem::size_of::<OrbisNetSockaddrIn6>() as u8,
                sin6_family: ORBIS_NET_AF_INET6 as u8,
                sin6_port: host.sin6_port,
                sin6_flowinfo: host.sin6_flowinfo,
                sin6_addr: host.sin6_addr.s6_addr,
                sin6_scope_id: host.sin6_scope_id,
            };
            // SAFETY: the struct is plain bytes.
            let bytes = unsafe {
                core::slice::from_raw_parts(
                    &inet6 as *const OrbisNetSockaddrIn6 as *const u8,
                    core::mem::size_of::<OrbisNetSockaddrIn6>(),
                )
            };
            copy_sockaddr(bytes, out)
        }
        libc::AF_UNIX => {
            // SAFETY: the family guarantees a sockaddr_un.
            let host =
                unsafe { &*(addr as *const libc::sockaddr_storage as *const libc::sockaddr_un) };
            let mut un = OrbisNetSockaddrUn {
                sun_len: core::mem::size_of::<OrbisNetSockaddrUn>() as u8,
                sun_family: ORBIS_NET_AF_UNIX as u8,
                sun_path: [0; 104],
            };
            let max = (len as usize).saturating_sub(core::mem::size_of::<libc::sa_family_t>());
            let path_len = host
                .sun_path
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(host.sun_path.len())
                .min(max)
                .min(103);
            for (slot, byte) in un.sun_path.iter_mut().zip(host.sun_path[..path_len].iter()) {
                *slot = *byte as u8;
            }
            // SAFETY: the struct is plain bytes.
            let bytes = unsafe {
                core::slice::from_raw_parts(
                    &un as *const OrbisNetSockaddrUn as *const u8,
                    core::mem::size_of::<OrbisNetSockaddrUn>(),
                )
            };
            copy_sockaddr(bytes, out)
        }
        _ => 0,
    }
}

/// `ToP2PAddress`.
pub fn to_p2p_address(
    addr: &OrbisNetSockaddr,
    addrlen: u32,
    kind: P2PKind,
) -> Result<(Endpoint, u16), i32> {
    if addrlen != core::mem::size_of::<OrbisNetSockaddrIn>() as u32 {
        return Err(ORBIS_NET_EINVAL);
    }
    if addr.sa_family as u32 != ORBIS_NET_AF_INET {
        return Err(ORBIS_NET_EAFNOSUPPORT);
    }
    let inet = read_orbis_in(addr);
    let mut host: libc::sockaddr_in = unsafe { std::mem::zeroed() };
    host.sin_family = libc::AF_INET as libc::sa_family_t;
    host.sin_addr.s_addr = inet.sin_addr;
    let port = if kind == P2PKind::Datagram {
        host.sin_port = inet.sin_port;
        u16::from_be(inet.sin_vport)
    } else {
        host.sin_port = if inet.sin_vport != 0 {
            inet.sin_vport
        } else {
            DEFAULT_P2P_UDP_PORT.to_be()
        };
        u16::from_be(inet.sin_port)
    };
    // SAFETY: the host sockaddr is fully initialized.
    let endpoint = unsafe {
        Endpoint::from_sockaddr(
            &host as *const libc::sockaddr_in as *const libc::sockaddr,
            core::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
        )
    };
    Ok((endpoint, port))
}

/// `ToOrbisP2PAddress`; returns the number of bytes the guest would need.
pub fn to_orbis_p2p_address(endpoint: &Endpoint, port: u16, kind: P2PKind, out: &mut [u8]) -> u32 {
    let mut inet = OrbisNetSockaddrIn {
        sin_len: core::mem::size_of::<OrbisNetSockaddrIn>() as u8,
        sin_family: ORBIS_NET_AF_INET as u8,
        ..OrbisNetSockaddrIn::default()
    };
    let mut udp_port = 0u16;
    if endpoint.family() == libc::AF_INET {
        // SAFETY: the family guarantees a sockaddr_in.
        let host = unsafe {
            &*(&endpoint.addr as *const libc::sockaddr_storage as *const libc::sockaddr_in)
        };
        udp_port = host.sin_port;
        inet.sin_addr = host.sin_addr.s_addr;
    }
    if kind == P2PKind::Datagram {
        inet.sin_port = udp_port;
        inet.sin_vport = port.to_be();
    } else {
        inet.sin_port = port.to_be();
        inet.sin_vport = udp_port;
    }
    // SAFETY: the struct is plain bytes.
    let bytes = unsafe {
        core::slice::from_raw_parts(
            &inet as *const OrbisNetSockaddrIn as *const u8,
            core::mem::size_of::<OrbisNetSockaddrIn>(),
        )
    };
    copy_sockaddr(bytes, out)
}

/// `HostMsgFlags`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HostMsgFlags {
    pub host: i32,
    pub dontwait: bool,
    pub peek: bool,
    pub peeklen: bool,
    pub waitall: bool,
    pub crypto: bool,
    pub signature: bool,
    pub unsupported: i32,
}

/// `ToHostMsgFlags`.
pub fn to_host_msg_flags(orbis_flags: i32) -> HostMsgFlags {
    const NO_SIGNAL: i32 = 0x20000;
    let mut out = HostMsgFlags::default();
    let mut rest = orbis_flags;
    let peeklen = ORBIS_NET_MSG_PEEKLEN as i32;
    if rest & peeklen == peeklen {
        out.peeklen = true;
        rest &= !(peeklen & !(ORBIS_NET_MSG_PEEK as i32));
    }
    if rest & ORBIS_NET_MSG_PEEK as i32 != 0 {
        out.host |= libc::MSG_PEEK;
        out.peek = true;
        rest &= !(ORBIS_NET_MSG_PEEK as i32);
    }
    if rest & ORBIS_NET_MSG_WAITALL as i32 != 0 {
        out.host |= libc::MSG_WAITALL;
        out.waitall = true;
        rest &= !(ORBIS_NET_MSG_WAITALL as i32);
    }
    if rest & ORBIS_NET_MSG_DONTWAIT as i32 != 0 {
        out.dontwait = true;
        rest &= !(ORBIS_NET_MSG_DONTWAIT as i32);
    }
    if rest & ORBIS_NET_MSG_USECRYPTO as i32 != 0 {
        out.crypto = true;
        rest &= !(ORBIS_NET_MSG_USECRYPTO as i32);
    }
    if rest & ORBIS_NET_MSG_USESIGNATURE as i32 != 0 {
        out.signature = true;
        rest &= !(ORBIS_NET_MSG_USESIGNATURE as i32);
    }
    rest &= !NO_SIGNAL;
    out.unsupported = rest;
    out
}

/// `OptionValue`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionValue {
    Int,
    Linger,
    Raw,
}

/// `HostOption`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostOption {
    pub level: i32,
    pub name: i32,
    pub value: OptionValue,
}

/// `ToHostOption`.
pub fn to_host_option(level: i32, name: i32) -> Option<HostOption> {
    match level as u32 {
        ORBIS_NET_SOL_SOCKET => match name as u32 {
            ORBIS_NET_SO_KEEPALIVE => Some(HostOption {
                level: libc::SOL_SOCKET,
                name: libc::SO_KEEPALIVE,
                value: OptionValue::Int,
            }),
            ORBIS_NET_SO_BROADCAST => Some(HostOption {
                level: libc::SOL_SOCKET,
                name: libc::SO_BROADCAST,
                value: OptionValue::Int,
            }),
            ORBIS_NET_SO_SNDBUF => Some(HostOption {
                level: libc::SOL_SOCKET,
                name: libc::SO_SNDBUF,
                value: OptionValue::Int,
            }),
            ORBIS_NET_SO_RCVBUF => Some(HostOption {
                level: libc::SOL_SOCKET,
                name: libc::SO_RCVBUF,
                value: OptionValue::Int,
            }),
            ORBIS_NET_SO_SNDLOWAT => Some(HostOption {
                level: libc::SOL_SOCKET,
                name: libc::SO_SNDLOWAT,
                value: OptionValue::Int,
            }),
            ORBIS_NET_SO_RCVLOWAT => Some(HostOption {
                level: libc::SOL_SOCKET,
                name: libc::SO_RCVLOWAT,
                value: OptionValue::Int,
            }),
            ORBIS_NET_SO_LINGER => Some(HostOption {
                level: libc::SOL_SOCKET,
                name: libc::SO_LINGER,
                value: OptionValue::Linger,
            }),
            _ => None,
        },
        ORBIS_NET_IPPROTO_IP => match name as u32 {
            ORBIS_NET_IP_HDRINCL => Some(HostOption {
                level: libc::IPPROTO_IP,
                name: libc::IP_HDRINCL,
                value: OptionValue::Int,
            }),
            ORBIS_NET_IP_TOS => Some(HostOption {
                level: libc::IPPROTO_IP,
                name: libc::IP_TOS,
                value: OptionValue::Int,
            }),
            ORBIS_NET_IP_TTL => Some(HostOption {
                level: libc::IPPROTO_IP,
                name: libc::IP_TTL,
                value: OptionValue::Int,
            }),
            ORBIS_NET_IP_MULTICAST_IF => Some(HostOption {
                level: libc::IPPROTO_IP,
                name: libc::IP_MULTICAST_IF,
                value: OptionValue::Raw,
            }),
            ORBIS_NET_IP_MULTICAST_TTL => Some(HostOption {
                level: libc::IPPROTO_IP,
                name: libc::IP_MULTICAST_TTL,
                value: OptionValue::Int,
            }),
            ORBIS_NET_IP_MULTICAST_LOOP => Some(HostOption {
                level: libc::IPPROTO_IP,
                name: libc::IP_MULTICAST_LOOP,
                value: OptionValue::Int,
            }),
            ORBIS_NET_IP_ADD_MEMBERSHIP => Some(HostOption {
                level: libc::IPPROTO_IP,
                name: libc::IP_ADD_MEMBERSHIP,
                value: OptionValue::Raw,
            }),
            ORBIS_NET_IP_DROP_MEMBERSHIP => Some(HostOption {
                level: libc::IPPROTO_IP,
                name: libc::IP_DROP_MEMBERSHIP,
                value: OptionValue::Raw,
            }),
            _ => None,
        },
        ORBIS_NET_IPPROTO_TCP => match name as u32 {
            ORBIS_NET_TCP_NODELAY => Some(HostOption {
                level: libc::IPPROTO_TCP,
                name: libc::TCP_NODELAY,
                value: OptionValue::Int,
            }),
            ORBIS_NET_TCP_MAXSEG => Some(HostOption {
                level: libc::IPPROTO_TCP,
                name: libc::TCP_MAXSEG,
                value: OptionValue::Int,
            }),
            _ => None,
        },
        _ => None,
    }
}

/// `IsReservedPort`.
pub fn is_reserved_port(port: u16) -> bool {
    (1..=1023).contains(&port)
        || port == 5353
        || (8540..=8579).contains(&port)
        || (9293..=9310).contains(&port)
        || port >= 40000
}

/// `IsReservedP2PVport`.
pub fn is_reserved_p2p_vport(vport: u16) -> bool {
    vport == 5353 || vport >= 32768
}

/// `DefaultSendBuffer`.
pub fn default_send_buffer(host_type: i32) -> i32 {
    if host_type == libc::SOCK_STREAM {
        32768
    } else {
        9216
    }
}

/// `DefaultReceiveBuffer`.
pub fn default_receive_buffer(host_type: i32) -> i32 {
    if host_type == libc::SOCK_STREAM {
        65536
    } else {
        40 * 1024
    }
}

/// `ToHostEpollEvents`.
pub fn to_host_epoll_events(orbis_events: u32) -> u32 {
    let mut out = 0;
    if orbis_events & ORBIS_NET_EPOLLIN != 0 {
        out |= host_net::event_bits::EV_IN;
    }
    if orbis_events & ORBIS_NET_EPOLLOUT != 0 {
        out |= host_net::event_bits::EV_OUT;
    }
    if orbis_events & ORBIS_NET_EPOLLERR != 0 {
        out |= host_net::event_bits::EV_ERR;
    }
    if orbis_events & ORBIS_NET_EPOLLHUP != 0 {
        out |= host_net::event_bits::EV_HUP;
    }
    out
}

/// `ToOrbisEpollEvents`.
pub fn to_orbis_epoll_events(host_events: u32) -> u32 {
    let mut out = 0;
    if host_events & host_net::event_bits::EV_IN != 0 {
        out |= ORBIS_NET_EPOLLIN;
    }
    if host_events & host_net::event_bits::EV_OUT != 0 {
        out |= ORBIS_NET_EPOLLOUT;
    }
    if host_events & host_net::event_bits::EV_ERR != 0 {
        out |= ORBIS_NET_EPOLLERR;
    }
    if host_events & host_net::event_bits::EV_HUP != 0 {
        out |= ORBIS_NET_EPOLLHUP;
    }
    if host_events & host_net::event_bits::EV_DESC_ID != 0 {
        out |= ORBIS_NET_EPOLLDESCID;
    }
    out
}

/// `ToOrbisErrno`.
pub fn to_orbis_errno(error: host_net::Error) -> i32 {
    if error == host_net::Error::NotSock {
        return ORBIS_NET_EBADF;
    }
    error as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families_and_socket_kinds_translate_both_ways() {
        assert_eq!(
            to_host_family(ORBIS_NET_AF_INET as i32),
            Some(libc::AF_INET)
        );
        assert_eq!(
            to_host_family(ORBIS_NET_AF_UNIX as i32),
            Some(libc::AF_UNIX)
        );
        assert_eq!(to_host_family(99), None);
        assert_eq!(to_orbis_family(libc::AF_INET6), ORBIS_NET_AF_INET6 as i32);
        assert_eq!(to_orbis_family(99), 0);

        let stream =
            to_host_socket_kind(ORBIS_NET_AF_INET as i32, ORBIS_NET_SOCK_STREAM as i32, 0).unwrap();
        assert_eq!(stream.socket_type, libc::SOCK_STREAM);
        assert_eq!(
            to_orbis_socket_type(stream.socket_type, false),
            ORBIS_NET_SOCK_STREAM as i32
        );
        let p2p = to_host_socket_kind(
            ORBIS_NET_AF_INET as i32,
            ORBIS_NET_SOCK_STREAM_P2P as i32,
            0,
        )
        .unwrap();
        assert!(p2p.p2p);
        assert_eq!(
            to_orbis_socket_type(p2p.socket_type, true),
            ORBIS_NET_SOCK_STREAM_P2P as i32
        );
        assert_eq!(
            to_host_socket_kind(ORBIS_NET_AF_INET as i32, ORBIS_NET_SOCK_DGRAM as i32, 6),
            Err(ORBIS_NET_EPROTOTYPE)
        );
        assert_eq!(
            to_host_socket_kind(ORBIS_NET_AF_UNIX as i32, ORBIS_NET_SOCK_RAW as i32, 0),
            Err(ORBIS_NET_EPROTONOSUPPORT)
        );
    }

    #[test]
    fn ipv4_addresses_round_trip() {
        let guest = OrbisNetSockaddrIn {
            sin_len: 16,
            sin_family: ORBIS_NET_AF_INET as u8,
            sin_port: 8080u16.to_be(),
            sin_addr: 0x7f00_0001u32.to_be(),
            sin_vport: 0,
            sin_zero: [0; 6],
        };
        let bytes = unsafe {
            core::slice::from_raw_parts(
                &guest as *const OrbisNetSockaddrIn as *const OrbisNetSockaddr,
                1,
            )
        };
        let addr = &bytes[0];
        let (host, len) = to_host_sockaddr(addr, 16).unwrap();
        assert_eq!(len as usize, core::mem::size_of::<libc::sockaddr_in>());
        let mut out = [0u8; 16];
        let needed = to_orbis_sockaddr(&host, len, &mut out);
        assert_eq!(needed, 16);
        let converted = unsafe { &*(out.as_ptr() as *const OrbisNetSockaddrIn) };
        assert_eq!(converted.sin_family as u32, ORBIS_NET_AF_INET);
        assert_eq!(converted.sin_port, 8080u16.to_be());
        assert_eq!(converted.sin_addr, 0x7f00_0001u32.to_be());
    }

    #[test]
    fn p2p_addresses_swap_ports_by_kind() {
        let guest = OrbisNetSockaddrIn {
            sin_len: 16,
            sin_family: ORBIS_NET_AF_INET as u8,
            sin_port: 8080u16.to_be(),
            sin_addr: 0x7f00_0001u32.to_be(),
            sin_vport: 3658u16.to_be(),
            sin_zero: [0; 6],
        };
        let addr = unsafe { &*(&guest as *const OrbisNetSockaddrIn as *const OrbisNetSockaddr) };
        let (endpoint, port) = to_p2p_address(addr, 16, P2PKind::Datagram).unwrap();
        assert_eq!(port, 3658);
        assert_eq!(endpoint.port(), 8080);
        let (endpoint, port) = to_p2p_address(addr, 16, P2PKind::Stream).unwrap();
        assert_eq!(port, 8080);
        assert_eq!(endpoint.port(), 3658);

        let mut out = [0u8; 16];
        let needed = to_orbis_p2p_address(&endpoint, port, P2PKind::Stream, &mut out);
        assert_eq!(needed, 16);
        let converted = unsafe { &*(out.as_ptr() as *const OrbisNetSockaddrIn) };
        assert_eq!(converted.sin_port, 8080u16.to_be());
        assert_eq!(converted.sin_vport, 3658u16.to_be());
    }

    #[test]
    fn message_flags_and_options_match() {
        let flags = to_host_msg_flags(ORBIS_NET_MSG_PEEKLEN as i32 | ORBIS_NET_MSG_DONTWAIT as i32);
        assert!(flags.peeklen && flags.peek && flags.dontwait);
        assert_eq!(flags.host & libc::MSG_PEEK, libc::MSG_PEEK);
        let flags = to_host_msg_flags(ORBIS_NET_MSG_USECRYPTO as i32 | 0x1);
        assert!(flags.crypto);
        assert_eq!(flags.unsupported, 0x1);

        let option =
            to_host_option(ORBIS_NET_SOL_SOCKET as i32, ORBIS_NET_SO_LINGER as i32).unwrap();
        assert_eq!(option.value, OptionValue::Linger);
        assert_eq!(
            to_host_option(ORBIS_NET_IPPROTO_TCP as i32, ORBIS_NET_TCP_NODELAY as i32)
                .unwrap()
                .name,
            libc::TCP_NODELAY
        );
        assert!(to_host_option(1, 2).is_none());
    }

    #[test]
    fn reserved_ports_buffers_and_events_match() {
        assert!(is_reserved_port(80));
        assert!(!is_reserved_port(2048));
        assert!(is_reserved_port(5353));
        assert!(is_reserved_port(40000));
        assert!(is_reserved_p2p_vport(32768));
        assert!(!is_reserved_p2p_vport(1000));
        assert_eq!(default_send_buffer(libc::SOCK_STREAM), 32768);
        assert_eq!(default_receive_buffer(libc::SOCK_DGRAM), 40960);
        let host = to_host_epoll_events(ORBIS_NET_EPOLLIN | ORBIS_NET_EPOLLHUP);
        assert_eq!(
            host,
            host_net::event_bits::EV_IN | host_net::event_bits::EV_HUP
        );
        assert_eq!(
            to_orbis_epoll_events(host | host_net::event_bits::EV_DESC_ID),
            ORBIS_NET_EPOLLIN | ORBIS_NET_EPOLLHUP | ORBIS_NET_EPOLLDESCID
        );
        assert_eq!(to_orbis_errno(host_net::Error::NotSock), ORBIS_NET_EBADF);
        assert_eq!(
            to_orbis_errno(host_net::Error::ConnRefused),
            ORBIS_NET_ECONNREFUSED
        );
    }
}
