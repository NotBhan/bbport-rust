//! Host network information, ported from
//! `legacy/gpu/bbnet/vendor/shadps4/core/libraries/net/net_util.h` and `net_util.cpp`.

use std::net::{IpAddr, TcpStream, ToSocketAddrs};
use std::sync::Mutex;
use std::time::Duration;

use crate::gpu::bbnet::core::libraries::net::error::ORBIS_NET_ERETURN;
use crate::gpu::bbnet::core::libraries::net::types::OrbisNetInAddr;
use crate::gpu::common::logging::classes;

/// `ORBIS_OK`.
pub const ORBIS_OK: i32 = 0;

#[derive(Debug, Default)]
struct State {
    ether_address: [u8; 6],
    default_gateway: String,
    netmask: String,
    ip: String,
    external_ip: u32,
    nat_type: u32,
}

/// `NetUtil::NetUtilInternal`.
#[derive(Debug, Default)]
pub struct NetUtilInternal {
    state: Mutex<State>,
}

impl NetUtilInternal {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    /// `GetEthernetAddr`.
    pub fn ethernet_addr(&self) -> [u8; 6] {
        self.lock().ether_address
    }

    /// `GetDefaultGateway`.
    pub fn default_gateway(&self) -> String {
        self.lock().default_gateway.clone()
    }

    /// `GetNetmask`.
    pub fn netmask(&self) -> String {
        self.lock().netmask.clone()
    }

    /// `GetIp`.
    pub fn ip(&self) -> String {
        self.lock().ip.clone()
    }

    /// `GetExternalIp`.
    pub fn external_ip(&self) -> u32 {
        self.lock().external_ip
    }

    /// `SetExternalIp`.
    pub fn set_external_ip(&self, address: u32) {
        self.lock().external_ip = address;
    }

    /// `GetNatType`.
    pub fn nat_type(&self) -> u32 {
        self.lock().nat_type
    }

    /// `RetrieveEthernetAddr`: the first non-loopback interface's hardware address.
    pub fn retrieve_ethernet_addr(&self) -> bool {
        let mut state = self.lock();
        // SAFETY: `getifaddrs` fills a linked list that is released below with `freeifaddrs`.
        unsafe {
            let mut interfaces: *mut libc::ifaddrs = std::ptr::null_mut();
            if libc::getifaddrs(&mut interfaces) != 0 {
                return false;
            }
            let mut current = interfaces;
            let mut found = false;
            while !current.is_null() {
                let interface = &*current;
                if !interface.ifa_addr.is_null()
                    && (*interface.ifa_addr).sa_family as i32 == libc::AF_PACKET
                    && !std::ffi::CStr::from_ptr(interface.ifa_name)
                        .to_bytes()
                        .starts_with(b"lo")
                {
                    let link = &*(interface.ifa_addr as *const libc::sockaddr_ll);
                    if link.sll_halen >= 6 {
                        state.ether_address.copy_from_slice(&link.sll_addr[..6]);
                        found = true;
                        break;
                    }
                }
                current = interface.ifa_next;
            }
            libc::freeifaddrs(interfaces);
            found
        }
    }

    /// `RetrieveDefaultGateway`, from `/proc/net/route` on Linux.
    pub fn retrieve_default_gateway(&self) -> bool {
        let mut state = self.lock();
        let contents = match std::fs::read_to_string("/proc/net/route") {
            Ok(contents) => contents,
            Err(_) => return false,
        };
        match parse_default_gateway(&contents) {
            Some(gateway) => {
                state.default_gateway = gateway;
                true
            }
            None => false,
        }
    }

    /// `RetrieveNetmask`: the last IPv4 interface's netmask, as in the original loop.
    pub fn retrieve_netmask(&self) -> bool {
        let mut state = self.lock();
        // SAFETY: the list is released with `freeifaddrs` before returning.
        unsafe {
            let mut interfaces: *mut libc::ifaddrs = std::ptr::null_mut();
            if libc::getifaddrs(&mut interfaces) != 0 {
                return false;
            }
            let mut current = interfaces;
            let mut success = false;
            while !current.is_null() {
                let interface = &*current;
                if !interface.ifa_addr.is_null()
                    && (*interface.ifa_addr).sa_family as i32 == libc::AF_INET
                    && !interface.ifa_netmask.is_null()
                {
                    let netmask = &*(interface.ifa_netmask as *const libc::sockaddr_in);
                    state.netmask = format_network_u32(netmask.sin_addr.s_addr);
                    success = true;
                }
                current = interface.ifa_next;
            }
            libc::freeifaddrs(interfaces);
            success
        }
    }

    /// `RetrieveIp`: the local address of a route to 1.1.1.1:80.
    pub fn retrieve_ip(&self) -> bool {
        let mut state = self.lock();
        match TcpStream::connect_timeout(
            &"1.1.1.1:80".parse().expect("a valid address"),
            Duration::from_secs(1),
        ) {
            Ok(stream) => match stream.local_addr() {
                Ok(address) => {
                    state.ip = address.ip().to_string();
                    true
                }
                Err(_) => false,
            },
            Err(_) => false,
        }
    }

    /// `ResolveHostname`.
    pub fn resolve_hostname(&self, hostname: &str, address: &mut OrbisNetInAddr) -> i32 {
        let result = (hostname, 0u16).to_socket_addrs();
        match result {
            Ok(mut addresses) => {
                let resolved = addresses.find_map(|entry| match entry.ip() {
                    IpAddr::V4(ip) => Some(ip),
                    IpAddr::V6(_) => None,
                });
                match resolved {
                    Some(ip) => {
                        crate::log_debug!(
                            classes::LIB_NET,
                            "resolved address for {}: {}",
                            hostname,
                            ip
                        );
                        address.inaddr_addr = u32::from_ne_bytes(ip.octets());
                        ORBIS_OK
                    }
                    None => {
                        crate::log_error!(
                            classes::LIB_NET,
                            "address resolution for {} failed: no IPv4 address",
                            hostname
                        );
                        ORBIS_NET_ERETURN
                    }
                }
            }
            Err(error) => {
                crate::log_error!(
                    classes::LIB_NET,
                    "address resolution for {} failed: {}",
                    hostname,
                    error
                );
                ORBIS_NET_ERETURN
            }
        }
    }
}

/// The gateway parsing half of `RetrieveDefaultGateway`, split out for tests.
fn parse_default_gateway(contents: &str) -> Option<String> {
    for line in contents.lines().skip(1) {
        let mut fields = line.split_whitespace();
        let _interface = fields.next()?;
        let destination = fields.next()?;
        let gateway = fields.next()?;
        if destination == "00000000" {
            let value = u32::from_str_radix(gateway, 16).ok()?;
            return Some(format_network_u32(value));
        }
    }
    None
}

/// Formats a `sockaddr_in` `s_addr` (network bytes in memory) as a dotted address.
fn format_network_u32(value: u32) -> String {
    let bytes = value.to_ne_bytes();
    format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_proc_route_gateway_is_formatted_in_memory_order() {
        let table = "\
Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
eth0\t00000000\t0102A8C0\t0003\t0\t0\t0\t00000000\t0\t0\t0
eth0\t0002A8C0\t00000000\t0001\t0\t0\t0\t00FFFFFF\t0\t0\t0
";
        assert_eq!(parse_default_gateway(table).as_deref(), Some("192.168.2.1"));
        assert_eq!(parse_default_gateway("Iface\tDestination\n"), None);
        assert_eq!(format_network_u32(0x00FF_FFFF), "255.255.255.0");
    }

    #[test]
    fn external_ip_and_nat_type_round_trip() {
        let util = NetUtilInternal::new();
        assert_eq!(util.external_ip(), 0);
        util.set_external_ip(0x0102_0304);
        assert_eq!(util.external_ip(), 0x0102_0304);
        assert_eq!(util.nat_type(), 0);
    }

    #[test]
    fn localhost_resolves_to_an_ipv4_address() {
        let util = NetUtilInternal::new();
        let mut address = OrbisNetInAddr::default();
        assert_eq!(util.resolve_hostname("localhost", &mut address), ORBIS_OK);
        let bytes = address.inaddr_addr.to_ne_bytes();
        assert_eq!(bytes, [127, 0, 0, 1]);
    }

    #[test]
    fn retrieval_helpers_do_not_panic() {
        let util = NetUtilInternal::new();
        let _ = util.retrieve_ethernet_addr();
        let _ = util.retrieve_netmask();
        let _ = util.retrieve_default_gateway();
        let _ = util.ethernet_addr();
        let _ = util.netmask();
        let _ = util.default_gateway();
    }
}
