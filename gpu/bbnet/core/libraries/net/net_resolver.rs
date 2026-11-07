//! Net resolver contexts, ported from
//! `bbnet/vendor/shadps4/core/libraries/net/net_resolver.{h,cpp}`.

use std::collections::HashMap;
use std::net::{Ipv4Addr, ToSocketAddrs};
use std::sync::{Mutex, OnceLock};

use crate::gpu::bbnet::core::libraries::net::types::{OrbisNetInAddr, OrbisNetResolverInfo};

pub const ORBIS_NET_OK: i32 = 0;
pub const ORBIS_NET_ERROR_INVALID_ARGUMENT: i32 = 0x8041_0116u32 as i32;
pub const ORBIS_NET_ERROR_RESOLVER_ENOENT: i32 = 0x8041_0118u32 as i32;
pub const ORBIS_NET_ERROR_RESOLVER_EINTR: i32 = 0x8041_0119u32 as i32;

#[derive(Debug, Clone)]
pub struct ResolverContext {
    pub id: i32,
    pub name: String,
    pub last_error: i32,
}

fn resolvers() -> &'static Mutex<HashMap<i32, ResolverContext>> {
    static RESOLVERS: OnceLock<Mutex<HashMap<i32, ResolverContext>>> = OnceLock::new();
    RESOLVERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_id() -> &'static Mutex<i32> {
    static NEXT: OnceLock<Mutex<i32>> = OnceLock::new();
    NEXT.get_or_init(|| Mutex::new(1))
}

type OnlineCheck = Box<dyn Fn() -> bool + Send + Sync>;

fn online_check() -> &'static Mutex<Option<OnlineCheck>> {
    static CHECK: OnceLock<Mutex<Option<OnlineCheck>>> = OnceLock::new();
    CHECK.get_or_init(|| Mutex::new(None))
}

/// `SetResolverOnlineCheck`.
pub fn set_resolver_online_check(check: Box<dyn Fn() -> bool + Send + Sync>) {
    *online_check()
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(check);
}

fn is_online() -> bool {
    online_check()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .as_ref()
        .is_some_and(|check| check())
}

/// `CreateResolver`.
pub fn create_resolver(name: String) -> i32 {
    if !is_online() {
        return ORBIS_NET_ERROR_RESOLVER_ENOENT;
    }
    let mut next = next_id().lock().unwrap_or_else(|error| error.into_inner());
    let id = *next;
    *next += 1;
    drop(next);
    resolvers()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .insert(
            id,
            ResolverContext {
                id,
                name,
                last_error: 0,
            },
        );
    id
}

/// `DestroyResolver`.
pub fn destroy_resolver(id: i32) -> i32 {
    resolvers()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(&id)
        .map_or(ORBIS_NET_ERROR_INVALID_ARGUMENT, |_| ORBIS_NET_OK)
}

/// `AbortResolver`.
pub fn abort_resolver(id: i32, _flags: u32) -> i32 {
    if resolvers()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .contains_key(&id)
    {
        ORBIS_NET_OK
    } else {
        ORBIS_NET_ERROR_INVALID_ARGUMENT
    }
}

fn set_error(id: i32, error: i32) {
    if let Some(context) = resolvers()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .get_mut(&id)
    {
        context.last_error = error;
    }
}

/// `ResolverStartNtoa`; resolves `hostname` to IPv4 and fills the resolver info array.
pub fn resolver_start_ntoa(
    id: i32,
    hostname: &str,
    addr: Option<&mut OrbisNetInAddr>,
    info: Option<&mut OrbisNetResolverInfo>,
    _async: bool,
    disable_ipaddress: bool,
) -> i32 {
    if !resolvers()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .contains_key(&id)
    {
        return ORBIS_NET_ERROR_INVALID_ARGUMENT;
    }
    let resolved: Vec<Ipv4Addr> = match (hostname, 0u16).to_socket_addrs() {
        Ok(addrs) => addrs
            .filter_map(|addr| match addr {
                std::net::SocketAddr::V4(v4) => Some(*v4.ip()),
                std::net::SocketAddr::V6(_) => None,
            })
            .collect(),
        Err(_) => {
            set_error(id, ORBIS_NET_ERROR_RESOLVER_ENOENT);
            return ORBIS_NET_ERROR_RESOLVER_ENOENT;
        }
    };
    let Some(first) = resolved.first().copied() else {
        set_error(id, ORBIS_NET_ERROR_RESOLVER_ENOENT);
        return ORBIS_NET_ERROR_RESOLVER_ENOENT;
    };
    if !disable_ipaddress {
        if let Some(addr) = addr {
            addr.inaddr_addr = u32::from_ne_bytes(first.octets());
        }
    }
    if let Some(info) = info {
        info.records = resolved.len().min(info.addrs.len()) as u32;
        info.recordsv4 = info.records;
        for (index, ip) in resolved.iter().take(info.addrs.len()).enumerate() {
            info.addrs[index].u.addr.inaddr_addr = u32::from_ne_bytes(ip.octets());
            info.addrs[index].af = 2;
        }
    }
    ORBIS_NET_OK
}

/// `ResolverStartAton`; formats an IPv4 address.
pub fn resolver_start_aton(
    id: i32,
    addr: Option<&OrbisNetInAddr>,
    hostname: &mut [u8],
    _async: bool,
) -> i32 {
    if !resolvers()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .contains_key(&id)
    {
        return ORBIS_NET_ERROR_INVALID_ARGUMENT;
    }
    let Some(addr) = addr else {
        return ORBIS_NET_ERROR_INVALID_ARGUMENT;
    };
    let ip = Ipv4Addr::from(addr.inaddr_addr.to_ne_bytes());
    let text = ip.to_string();
    if hostname.len() <= text.len() {
        return ORBIS_NET_ERROR_INVALID_ARGUMENT;
    }
    hostname[..text.len()].copy_from_slice(text.as_bytes());
    hostname[text.len()] = 0;
    ORBIS_NET_OK
}

/// `ResolverGetError`.
pub fn resolver_get_error(id: i32, status: &mut i32) -> i32 {
    let contexts = resolvers()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let Some(context) = contexts.get(&id) else {
        return ORBIS_NET_ERROR_INVALID_ARGUMENT;
    };
    *status = context.last_error;
    ORBIS_NET_OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::bbnet::core::libraries::net::types::{OrbisNetAddrUnion, OrbisNetResolverAddr};

    #[test]
    fn resolvers_resolve_aton_and_ntoa() {
        set_resolver_online_check(Box::new(|| true));
        let id = create_resolver("test".to_string());
        assert!(id > 0);
        let mut addr = OrbisNetInAddr { inaddr_addr: 0 };
        let mut info = OrbisNetResolverInfo {
            addrs: [OrbisNetResolverAddr {
                u: OrbisNetAddrUnion {
                    addr: OrbisNetInAddr { inaddr_addr: 0 },
                },
                af: 0,
                pad: [0; 3],
            }; 10],
            records: 0,
            recordsv4: 0,
            pad: [0; 14],
        };
        assert_eq!(
            resolver_start_ntoa(
                id,
                "127.0.0.1",
                Some(&mut addr),
                Some(&mut info),
                false,
                false
            ),
            ORBIS_NET_OK
        );
        assert_eq!(
            Ipv4Addr::from(addr.inaddr_addr.to_ne_bytes()),
            Ipv4Addr::LOCALHOST
        );
        assert_eq!(info.records, 1);
        let mut name = [0u8; 32];
        assert_eq!(
            resolver_start_aton(id, Some(&addr), &mut name, false),
            ORBIS_NET_OK
        );
        assert_eq!(&name[..9], b"127.0.0.1");
        let mut status = 0;
        assert_eq!(resolver_get_error(id, &mut status), ORBIS_NET_OK);
        assert_eq!(status, 0);
        assert_eq!(destroy_resolver(id), ORBIS_NET_OK);
        assert_eq!(destroy_resolver(id), ORBIS_NET_ERROR_INVALID_ARGUMENT);
    }
}
