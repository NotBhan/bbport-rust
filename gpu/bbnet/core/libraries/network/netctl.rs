//! NetCtl info types, ported from
//! `bbnet/vendor/shadps4/core/libraries/network/netctl.h`.

pub const ORBIS_NET_ETHER_ADDR_LEN: i32 = 6;
pub const ORBIS_NET_CTL_SSID_LEN: i32 = 32 + 1;
pub const ORBIS_NET_CTL_HOSTNAME_LEN: i32 = 255 + 1;
pub const ORBIS_NET_CTL_AUTH_NAME_LEN: i32 = 127 + 1;
pub const ORBIS_NET_CTL_IPV4_ADDR_STR_LEN: i32 = 16;

/// `OrbisNetEtherAddr`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNetEtherAddr {
    pub data: [u8; ORBIS_NET_ETHER_ADDR_LEN as usize],
}

/// `OrbisNetCtlInfo`.
#[repr(C)]
#[derive(Clone, Copy)]
pub union OrbisNetCtlInfo {
    pub device: u32,
    pub ether_addr: OrbisNetEtherAddr,
    pub mtu: u32,
    pub link: u32,
    pub bssid: OrbisNetEtherAddr,
    pub ssid: [u8; ORBIS_NET_CTL_SSID_LEN as usize],
    pub wifi_security: u32,
    pub rssi_dbm: u8,
    pub rssi_percentage: u8,
    pub channel: u8,
    pub ip_config: u32,
    pub dhcp_hostname: [u8; ORBIS_NET_CTL_HOSTNAME_LEN as usize],
    pub pppoe_auth_name: [u8; ORBIS_NET_CTL_AUTH_NAME_LEN as usize],
    pub ip_address: [u8; ORBIS_NET_CTL_IPV4_ADDR_STR_LEN as usize],
    pub netmask: [u8; ORBIS_NET_CTL_IPV4_ADDR_STR_LEN as usize],
    pub default_route: [u8; ORBIS_NET_CTL_IPV4_ADDR_STR_LEN as usize],
    pub primary_dns: [u8; ORBIS_NET_CTL_IPV4_ADDR_STR_LEN as usize],
    pub secondary_dns: [u8; ORBIS_NET_CTL_IPV4_ADDR_STR_LEN as usize],
    pub http_proxy_config: u32,
    pub http_proxy_server: [u8; ORBIS_NET_CTL_HOSTNAME_LEN as usize],
    pub http_proxy_port: u16,
}

/// `OrbisNetCtlNatInfo`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNetCtlNatInfo {
    pub size: u32,
    pub stun_status: i32,
    pub nat_type: i32,
    pub mapped_addr: u32,
}

pub const ORBIS_NET_CTL_INFO_DEVICE: i32 = 1;
pub const ORBIS_NET_CTL_INFO_ETHER_ADDR: i32 = 2;
pub const ORBIS_NET_CTL_INFO_MTU: i32 = 3;
pub const ORBIS_NET_CTL_INFO_LINK: i32 = 4;
pub const ORBIS_NET_CTL_INFO_BSSID: i32 = 5;
pub const ORBIS_NET_CTL_INFO_SSID: i32 = 6;
pub const ORBIS_NET_CTL_INFO_WIFI_SECURITY: i32 = 7;
pub const ORBIS_NET_CTL_INFO_RSSI_DBM: i32 = 8;
pub const ORBIS_NET_CTL_INFO_RSSI_PERCENTAGE: i32 = 9;
pub const ORBIS_NET_CTL_INFO_CHANNEL: i32 = 10;
pub const ORBIS_NET_CTL_INFO_IP_CONFIG: i32 = 11;
pub const ORBIS_NET_CTL_INFO_DHCP_HOSTNAME: i32 = 12;
pub const ORBIS_NET_CTL_INFO_PPPOE_AUTH_NAME: i32 = 13;
pub const ORBIS_NET_CTL_INFO_IP_ADDRESS: i32 = 14;
pub const ORBIS_NET_CTL_INFO_NETMASK: i32 = 15;
pub const ORBIS_NET_CTL_INFO_DEFAULT_ROUTE: i32 = 16;
pub const ORBIS_NET_CTL_INFO_PRIMARY_DNS: i32 = 17;
pub const ORBIS_NET_CTL_INFO_SECONDARY_DNS: i32 = 18;
pub const ORBIS_NET_CTL_INFO_HTTP_PROXY_CONFIG: i32 = 19;
pub const ORBIS_NET_CTL_INFO_HTTP_PROXY_SERVER: i32 = 20;
pub const ORBIS_NET_CTL_INFO_HTTP_PROXY_PORT: i32 = 21;

/// The principal getter; the HLE layer implements it.
pub trait NetCtlApi {
    fn get_info(&mut self, code: i32, info: &mut OrbisNetCtlInfo) -> i32;
    fn get_nat_info(&mut self, nat_info: &mut OrbisNetCtlNatInfo) -> i32;
    fn get_state(&mut self, state: &mut i32) -> i32;
    fn get_result(&mut self, event_type: i32, error_code: &mut i32) -> i32;
    fn init(&mut self) -> i32;
    fn term(&mut self) -> i32;
    fn register_callback(
        &mut self,
        func: crate::gpu::bbnet::core::libraries::network::net_ctl_obj::OrbisNetCtlCallback,
        arg: *mut core::ffi::c_void,
        cid: &mut i32,
    ) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nat_info_layout_matches_the_static_assert() {
        assert_eq!(core::mem::size_of::<OrbisNetCtlNatInfo>(), 0x10);
        assert_eq!(core::mem::offset_of!(OrbisNetCtlNatInfo, mapped_addr), 0xc);
        assert_eq!(ORBIS_NET_CTL_INFO_HTTP_PROXY_PORT, 21);
        assert_eq!(ORBIS_NET_ETHER_ADDR_LEN, 6);
        assert_eq!(ORBIS_NET_CTL_SSID_LEN, 33);
        assert_eq!(core::mem::size_of::<OrbisNetEtherAddr>(), 6);
    }

    #[test]
    fn the_info_union_has_room_for_the_largest_member() {
        assert_eq!(
            core::mem::size_of::<OrbisNetCtlInfo>(),
            ORBIS_NET_CTL_HOSTNAME_LEN as usize
        );
        let info = OrbisNetCtlInfo { link: 1 };
        assert_eq!(unsafe { info.link }, 1);
    }
}
