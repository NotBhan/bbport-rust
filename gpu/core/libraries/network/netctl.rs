//! `sceNetCtl` state, ported from
//! `bbnet/vendor/shadps4/core/libraries/network/netctl.cpp`.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NetCtlInfo {
    pub device: u32,
    pub ether_addr: [u8; 6],
    pub ip_address: [u8; 16],
    pub netmask: [u8; 16],
    pub default_route: [u8; 16],
    pub primary_dns: [u8; 16],
    pub secondary_dns: [u8; 16],
    pub http_proxy_config: u32,
    pub http_proxy_port: u16,
    pub http_proxy_auth: u32,
}

/// `NetCtl` state.
#[derive(Debug, Default)]
pub struct NetCtlState {
    pub initialized: bool,
    pub state: i32,
    pub info: NetCtlInfo,
    pub handlers: Vec<usize>,
}

pub const NET_CTL_STATE_DISCONNECTED: i32 = 0;
pub const NET_CTL_STATE_CONNECTED: i32 = 1;

impl NetCtlState {
    pub fn init(&mut self) -> i32 {
        self.initialized = true;
        self.state = NET_CTL_STATE_DISCONNECTED;
        0
    }

    pub fn term(&mut self) -> i32 {
        self.initialized = false;
        self.handlers.clear();
        0
    }

    pub fn get_state(&self) -> Option<i32> {
        self.initialized.then_some(self.state)
    }

    pub fn get_info(&self) -> Option<NetCtlInfo> {
        self.initialized.then_some(self.info)
    }

    pub fn register_handler(&mut self, handler: usize) -> i32 {
        if !self.initialized {
            return -1;
        }
        self.handlers.push(handler);
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netctl_reports_state_and_registers_callbacks() {
        let mut netctl = NetCtlState::default();
        assert!(netctl.get_state().is_none());
        assert_eq!(netctl.init(), 0);
        assert_eq!(netctl.get_state(), Some(NET_CTL_STATE_DISCONNECTED));
        assert!(netctl.get_info().is_some());
        assert_eq!(netctl.register_handler(1), 0);
        assert_eq!(netctl.term(), 0);
    }
}
