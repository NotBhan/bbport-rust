//! UPnP port mapping, ported from
//! `bbnet/vendor/shadps4/core/libraries/net/net_upnp.h`.

use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

/// What discovery found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    pub external_ip_nbo: u32,
    pub external_port: u16,
    pub lan_addr: String,
}

/// The miniupnpc calls the backend provides.
pub trait UpnpDriver: Send {
    fn discover(&mut self) -> Option<Discovery>;
    fn add_mapping(&mut self, port: u16) -> bool;
    fn remove_mapping(&mut self, port: u16) -> bool;
}

#[derive(Debug, Default)]
struct State {
    done: bool,
    available: bool,
    external_ip_nbo: u32,
    external_port: u16,
}

/// `UPnPClient`.
#[derive(Debug, Default)]
pub struct UPnPClient {
    started: AtomicBool,
    p2p_features_enabled: AtomicBool,
    mapped_port: AtomicU16,
    state: Mutex<State>,
    ready: Condvar,
}

impl UPnPClient {
    /// The process-wide client.
    pub fn instance() -> &'static UPnPClient {
        static INSTANCE: UPnPClient = UPnPClient {
            started: AtomicBool::new(false),
            p2p_features_enabled: AtomicBool::new(true),
            mapped_port: AtomicU16::new(0),
            state: Mutex::new(State {
                done: false,
                available: false,
                external_ip_nbo: 0,
                external_port: 0,
            }),
            ready: Condvar::new(),
        };
        &INSTANCE
    }

    /// `Start`: discovers once, on a background thread.
    pub fn start(&'static self, mut driver: impl UpnpDriver + 'static) {
        if self.started.swap(true, Ordering::AcqRel) {
            return;
        }
        std::thread::spawn(move || {
            let discovery = driver.discover();
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            state.done = true;
            if let Some(discovery) = discovery {
                state.available = true;
                state.external_ip_nbo = discovery.external_ip_nbo;
                state.external_port = discovery.external_port;
            }
            drop(state);
            self.ready.notify_all();
        });
    }

    /// `SetP2PFeaturesEnabled`.
    pub fn set_p2p_features_enabled(&self, enabled: bool) {
        self.p2p_features_enabled.store(enabled, Ordering::Release);
    }

    /// `WaitReady`.
    pub fn wait_ready(&self, timeout_ms: i32) -> bool {
        let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        let (state, _) = if timeout_ms < 0 {
            (
                self.ready
                    .wait_while(state, |state| !state.done)
                    .unwrap_or_else(|error| error.into_inner()),
                (),
            )
        } else {
            (
                self.ready
                    .wait_timeout_while(state, Duration::from_millis(timeout_ms as u64), |state| {
                        !state.done
                    })
                    .map(|(guard, _)| guard)
                    .unwrap_or_else(|error| error.into_inner().0),
                (),
            )
        };
        state.available
    }

    /// `IsAvailable`.
    pub fn is_available(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .available
    }

    /// `GetExternalIp`.
    pub fn external_ip(&self) -> u32 {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .external_ip_nbo
    }

    /// `GetExternalIpString`.
    pub fn external_ip_string(&self) -> String {
        let ip = self.external_ip();
        if ip == 0 {
            return String::new();
        }
        let bytes = ip.to_ne_bytes();
        format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
    }

    /// `GetExternalPort`.
    pub fn external_port(&self) -> u16 {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .external_port
    }

    /// `AddMapping` / `RemoveMapping` go through a driver installed by the caller.
    pub fn add_mapping(&self, driver: &mut dyn UpnpDriver, port: u16) -> bool {
        driver.add_mapping(port)
    }

    pub fn remove_mapping(&self, driver: &mut dyn UpnpDriver, port: u16) -> bool {
        driver.remove_mapping(port)
    }

    /// `MapPortAsync` / `UnmapPort`: the mapped port is remembered for exit cleanup.
    pub fn map_port_async(&self, driver: &mut dyn UpnpDriver, port: u16) {
        self.mapped_port.store(port, Ordering::Release);
        driver.add_mapping(port);
    }

    pub fn unmap_port(&self, driver: &mut dyn UpnpDriver, port: u16) {
        if self.mapped_port.load(Ordering::Acquire) == port {
            driver.remove_mapping(port);
            self.mapped_port.store(0, Ordering::Release);
        }
    }
}

/// The external IP is stored in network byte order.
pub fn external_ip_nbo(octets: [u8; 4]) -> u32 {
    u32::from_ne_bytes(octets)
}

/// Parses an external IP string into network-order form.
pub fn external_ip_from_str(text: &str) -> Option<u32> {
    let address: std::net::Ipv4Addr = text.parse().ok()?;
    Some(u32::from_ne_bytes(address.octets()))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockDriver {
        discovery: Option<Discovery>,
        mapped: AtomicU16,
    }

    impl UpnpDriver for MockDriver {
        fn discover(&mut self) -> Option<Discovery> {
            self.discovery.clone()
        }

        fn add_mapping(&mut self, port: u16) -> bool {
            self.mapped.store(port, Ordering::Release);
            true
        }

        fn remove_mapping(&mut self, _port: u16) -> bool {
            self.mapped.store(0, Ordering::Release);
            true
        }
    }

    fn client() -> &'static UPnPClient {
        Box::leak(Box::new(UPnPClient::default()))
    }

    #[test]
    fn discovery_updates_the_external_address() {
        let client = client();
        client.start(MockDriver {
            discovery: Some(Discovery {
                external_ip_nbo: external_ip_nbo([203, 0, 113, 5]),
                external_port: 4000,
                lan_addr: "192.168.1.2".to_string(),
            }),
            mapped: AtomicU16::new(0),
        });
        assert!(client.wait_ready(2000));
        assert!(client.is_available());
        assert_eq!(client.external_ip_string(), "203.0.113.5");
        assert_eq!(client.external_port(), 4000);
        assert_eq!(
            external_ip_from_str("203.0.113.5"),
            Some(external_ip_nbo([203, 0, 113, 5]))
        );
    }

    #[test]
    fn mappings_are_forwarded_and_remembered() {
        let client = client();
        let mut driver = MockDriver {
            discovery: None,
            mapped: AtomicU16::new(0),
        };
        client.map_port_async(&mut driver, 3658);
        assert_eq!(driver.mapped.load(Ordering::Acquire), 3658);
        client.unmap_port(&mut driver, 1234);
        assert_eq!(driver.mapped.load(Ordering::Acquire), 3658);
        client.unmap_port(&mut driver, 3658);
        assert_eq!(driver.mapped.load(Ordering::Acquire), 0);
        client.set_p2p_features_enabled(false);
    }
}
