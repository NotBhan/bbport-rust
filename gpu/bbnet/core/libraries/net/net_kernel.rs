//! Kernel socket integration declarations, ported from
//! `bbnet/vendor/shadps4/core/libraries/net/net_kernel.h`.

use crate::gpu::bbnet::core::libraries::net::types::{OrbisNetId, OrbisNetSockaddr};

/// The libkernel/libScePosix socket entry points. The guest-net layer implements them.
pub trait KernelSocketApi {
    fn install_kernel_integration();
    fn kernel_close(&mut self, fd: i32) -> i32;
    fn kernel_read(&mut self, fd: i32, buf: &mut [u8]) -> i64;
    fn kernel_write(&mut self, fd: i32, buf: &[u8]) -> i64;

    fn sys_connect(&mut self, s: OrbisNetId, addr: &OrbisNetSockaddr, addr_len: u32) -> i32;
    fn sys_bind(&mut self, s: OrbisNetId, addr: &OrbisNetSockaddr, addr_len: u32) -> i32;
    fn sys_accept(&mut self, s: OrbisNetId, addr: &mut OrbisNetSockaddr, addr_len: &mut u32)
        -> i32;
    fn sys_listen(&mut self, s: OrbisNetId, backlog: i32) -> i32;
    fn sys_shutdown(&mut self, s: OrbisNetId, how: i32) -> i32;
    fn sys_socket(&mut self, family: i32, socket_type: i32, protocol: i32) -> i32;
    fn sys_socketclose(&mut self, s: OrbisNetId) -> i32;
    fn sys_send(&mut self, s: OrbisNetId, buf: &[u8], flags: i32) -> i32;
}

/// `sys_htons`.
pub fn sys_htons(value: u16) -> i32 {
    i32::from(value.to_be())
}

/// `sys_htonl`.
pub fn sys_htonl(value: u32) -> i32 {
    value.to_be() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_byte_order_helpers_swap_on_little_endian() {
        assert_eq!(sys_htons(0x1234), 0x3412);
        assert_eq!(sys_htonl(0x1234_5678) as u32, 0x7856_3412);
    }
}

/// `PosixError`; sets the guest errno and returns -1.
pub fn posix_error(orbis_errno: i32, set_errno: &mut dyn FnMut(i32)) -> i32 {
    set_errno(orbis_errno);
    -1
}

/// The system hooks `InstallKernelIntegration` installs, reduced to their decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelIntegration {
    pub connected_to_network: bool,
    pub upnp_enabled: bool,
    pub configured_p2p_port: i32,
}

impl KernelIntegration {
    /// `p2p_port`: invalid ports fall back to 3658.
    pub fn p2p_port(&self) -> u16 {
        if !(0..=0xffff).contains(&self.configured_p2p_port) {
            3658
        } else {
            self.configured_p2p_port as u16
        }
    }

    /// `p2p_started`: UPnP mapping is requested only when online and enabled.
    pub fn should_map_upnp(&self, _port: u16) -> bool {
        self.connected_to_network && self.upnp_enabled
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn p2p_port_falls_back_and_upnp_is_gated() {
        let integration = KernelIntegration {
            connected_to_network: true,
            upnp_enabled: false,
            configured_p2p_port: 70000,
        };
        assert_eq!(integration.p2p_port(), 3658);
        assert!(!integration.should_map_upnp(3658));
        assert_eq!(
            KernelIntegration {
                configured_p2p_port: 4242,
                ..integration
            }
            .p2p_port(),
            4242
        );
        let mut errno = 0;
        assert_eq!(posix_error(22, &mut |value| errno = value), -1);
        assert_eq!(errno, 22);
    }
}
