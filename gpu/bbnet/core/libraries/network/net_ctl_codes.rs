//! Network control codes, ported from
//! `legacy/gpu/bbnet/vendor/shadps4/core/libraries/network/net_ctl_codes.h`.

#![allow(non_upper_case_globals)]

pub const ORBIS_NET_CTL_ERROR_CALLBACK_MAX: i32 = 0x80412103u32 as i32;
pub const ORBIS_NET_CTL_ERROR_ID_NOT_FOUND: i32 = 0x80412104u32 as i32;
pub const ORBIS_NET_CTL_ERROR_INVALID_ID: i32 = 0x80412105u32 as i32;
pub const ORBIS_NET_CTL_ERROR_INVALID_ADDR: i32 = 0x80412107u32 as i32;
pub const ORBIS_NET_CTL_ERROR_INVALID_SIZE: i32 = 0x80412111u32 as i32;
pub const ORBIS_NET_CTL_ERROR_NOT_CONNECTED: i32 = 0x80412108u32 as i32;
pub const ORBIS_NET_CTL_ERROR_NOT_AVAIL: i32 = 0x80412109u32 as i32;
pub const ORBIS_NET_CTL_ERROR_NETWORK_DISABLED: i32 = 0x8041210Du32 as i32;
pub const ORBIS_NET_CTL_ERROR_DISCONNECT_REQ: i32 = 0x8041210Eu32 as i32;
pub const ORBIS_NET_CTL_ERROR_ETHERNET_PLUGOUT: i32 = 0x80412115u32 as i32;
pub const ORBIS_NET_CTL_ERROR_WIFI_DEAUTHED: i32 = 0x80412116u32 as i32;
pub const ORBIS_NET_CTL_ERROR_WIFI_BEACON_LOST: i32 = 0x80412117u32 as i32;
pub const ORBIS_NET_CTL_STATE_DISCONNECTED: i32 = 0;
pub const ORBIS_NET_CTL_STATE_CONNECTING: i32 = 1;
pub const ORBIS_NET_CTL_STATE_IPOBTAINING: i32 = 2;
pub const ORBIS_NET_CTL_STATE_IPOBTAINED: i32 = 3;
pub const ORBIS_NET_CTL_EVENT_TYPE_DISCONNECTED: i32 = 1;
pub const ORBIS_SCE_NET_CTL_EVENT_TYPE_DISCONNECT_REQ_FINISHED: i32 = 2;
pub const ORBIS_NET_CTL_EVENT_TYPE_IPOBTAINED: i32 = 3;
pub const ORBIS_NET_CTL_DEVICE_WIRED: i32 = 0;
pub const ORBIS_NET_CTL_DEVICE_WIRELESS: i32 = 1;
pub const ORBIS_NET_CTL_LINK_DISCONNECTED: i32 = 0;
pub const ORBIS_NET_CTL_LINK_CONNECTED: i32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_control_codes_match_the_header() {
        assert_eq!(ORBIS_NET_CTL_ERROR_CALLBACK_MAX, 0x80412103u32 as i32);
        assert_eq!(ORBIS_NET_CTL_ERROR_WIFI_BEACON_LOST, 0x80412117u32 as i32);
        assert_eq!(ORBIS_NET_CTL_STATE_IPOBTAINED, 3);
        assert_eq!(ORBIS_NET_CTL_EVENT_TYPE_IPOBTAINED, 3);
        assert_eq!(ORBIS_NET_CTL_DEVICE_WIRELESS, 1);
        assert_eq!(ORBIS_NET_CTL_LINK_CONNECTED, 1);
    }
}
