//! Signaling transport hooks, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_signaling/np_signaling_transport.h`.

use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

/// `TransportHooks`.
#[derive(Debug, Clone, Copy, Default)]
pub struct TransportHooks {
    pub signaling_send: Option<unsafe extern "C" fn(*const u8, u32, u32, u16) -> i32>,
    pub signaling_recv: Option<unsafe extern "C" fn(*mut u8, u32, *mut u32, *mut u16) -> i32>,
    pub control_send: Option<unsafe extern "C" fn(*const u8, u32, u32, u16) -> i32>,
    pub control_recv: Option<unsafe extern "C" fn(*mut u8, u32, *mut u32, *mut u16) -> i32>,
    pub transport_ready: Option<unsafe extern "C" fn() -> bool>,
    pub configured_port: Option<unsafe extern "C" fn() -> u16>,
    pub advertised_addr: Option<unsafe extern "C" fn() -> u32>,
    pub ensure_transport: Option<unsafe extern "C" fn() -> bool>,
}

fn hooks() -> &'static Mutex<TransportHooks> {
    static HOOKS: OnceLock<Mutex<TransportHooks>> = OnceLock::new();
    HOOKS.get_or_init(|| Mutex::new(TransportHooks::default()))
}

/// `SetTransportHooks`.
pub fn set_transport_hooks(new_hooks: TransportHooks) {
    *hooks().lock().unwrap_or_else(|error| error.into_inner()) = new_hooks;
}

/// `SignalingSendTo`.
pub fn signaling_send_to(data: &[u8], dest_addr: u32, dest_port: u16) -> i32 {
    let hooks = hooks().lock().unwrap_or_else(|error| error.into_inner());
    match hooks.signaling_send {
        // SAFETY: the caller installed a valid signaling callback.
        Some(send) => unsafe { send(data.as_ptr(), data.len() as u32, dest_addr, dest_port) },
        None => -1,
    }
}

/// `SignalingRecvFrom`.
pub fn signaling_recv_from(buf: &mut [u8], from_addr: &mut u32, from_port: &mut u16) -> i32 {
    let hooks = hooks().lock().unwrap_or_else(|error| error.into_inner());
    match hooks.signaling_recv {
        // SAFETY: the caller installed a valid signaling callback.
        Some(recv) => unsafe { recv(buf.as_mut_ptr(), buf.len() as u32, from_addr, from_port) },
        None => -1,
    }
}

/// `ControlSendTo`.
pub fn control_send_to(data: &[u8], dest_addr: u32, dest_port: u16) -> i32 {
    let hooks = hooks().lock().unwrap_or_else(|error| error.into_inner());
    match hooks.control_send {
        // SAFETY: the caller installed a valid control callback.
        Some(send) => unsafe { send(data.as_ptr(), data.len() as u32, dest_addr, dest_port) },
        None => -1,
    }
}

/// `ControlRecvFrom`.
pub fn control_recv_from(buf: &mut [u8], from_addr: &mut u32, from_port: &mut u16) -> i32 {
    let hooks = hooks().lock().unwrap_or_else(|error| error.into_inner());
    match hooks.control_recv {
        // SAFETY: the caller installed a valid control callback.
        Some(recv) => unsafe { recv(buf.as_mut_ptr(), buf.len() as u32, from_addr, from_port) },
        None => -1,
    }
}

/// `TransportIsReady`.
pub fn transport_is_ready() -> bool {
    let hooks = hooks().lock().unwrap_or_else(|error| error.into_inner());
    hooks
        .transport_ready
        .is_some_and(|ready| unsafe { ready() })
}

/// `ConfiguredPort`.
pub fn configured_port() -> u16 {
    let hooks = hooks().lock().unwrap_or_else(|error| error.into_inner());
    hooks.configured_port.map_or(0, |port| unsafe { port() })
}

/// `AdvertisedAddr`.
pub fn advertised_addr() -> u32 {
    let hooks = hooks().lock().unwrap_or_else(|error| error.into_inner());
    hooks.advertised_addr.map_or(0, |addr| unsafe { addr() })
}

/// `EnsureTransport`.
pub fn ensure_transport() -> bool {
    let hooks = hooks().lock().unwrap_or_else(|error| error.into_inner());
    hooks
        .ensure_transport
        .is_some_and(|ensure| unsafe { ensure() })
}

/// `PeerResolver`.
pub type PeerResolver = unsafe extern "C" fn(*const u8, usize, *mut u32, *mut u16) -> bool;

fn peer_resolver() -> &'static Mutex<Option<PeerResolver>> {
    static RESOLVER: OnceLock<Mutex<Option<PeerResolver>>> = OnceLock::new();
    RESOLVER.get_or_init(|| Mutex::new(None))
}

/// `SetPeerResolver`.
pub fn set_peer_resolver(resolver: PeerResolver) {
    *peer_resolver()
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(resolver);
}

fn matching2_enabled_flag() -> &'static AtomicBool {
    static ENABLED: AtomicBool = AtomicBool::new(false);
    &ENABLED
}

/// `SetMatching2Enabled`.
pub fn set_matching2_enabled(enabled: bool) {
    matching2_enabled_flag().store(enabled, Ordering::Release);
}

static MM_SERVER_ADDR: AtomicU32 = AtomicU32::new(0);
static MM_SERVER_UDP_PORT: AtomicU16 = AtomicU16::new(0);

/// `SetMmServerEndpoint`.
pub fn set_mm_server_endpoint(addr: u32, udp_port: u16) {
    MM_SERVER_ADDR.store(addr, Ordering::Release);
    MM_SERVER_UDP_PORT.store(udp_port, Ordering::Release);
}

/// `ResolvePeer`.
pub fn resolve_peer(online_id: &str, out_addr: &mut u32, out_port: &mut u16) -> bool {
    let resolver = *peer_resolver()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    match resolver {
        // SAFETY: the caller installed a valid resolver.
        Some(resolver) => unsafe {
            resolver(online_id.as_ptr(), online_id.len(), out_addr, out_port)
        },
        None => false,
    }
}

/// `Matching2Enabled`.
pub fn matching2_enabled() -> bool {
    matching2_enabled_flag().load(Ordering::Acquire)
}

/// `MmServerAddr`.
pub fn mm_server_addr() -> u32 {
    MM_SERVER_ADDR.load(Ordering::Acquire)
}

/// `MmServerUdpPort`.
pub fn mm_server_udp_port() -> u16 {
    MM_SERVER_UDP_PORT.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn send(data: *const u8, len: u32, _addr: u32, _port: u16) -> i32 {
        unsafe { *data.add(len as usize - 1) as i32 }
    }

    unsafe extern "C" fn recv(buf: *mut u8, _len: u32, addr: *mut u32, port: *mut u16) -> i32 {
        unsafe {
            *buf = 42;
            *addr = 7;
            *port = 8;
        }
        1
    }

    unsafe extern "C" fn ready() -> bool {
        true
    }

    unsafe extern "C" fn port() -> u16 {
        3658
    }

    unsafe extern "C" fn addr() -> u32 {
        0x01020304
    }

    unsafe extern "C" fn ensure() -> bool {
        true
    }

    unsafe extern "C" fn resolve(
        _online_id: *const u8,
        len: usize,
        addr: *mut u32,
        port: *mut u16,
    ) -> bool {
        unsafe {
            *addr = len as u32;
            *port = 1;
        }
        true
    }

    #[test]
    fn hooks_forward_and_report_their_state() {
        set_transport_hooks(TransportHooks {
            signaling_send: Some(send),
            signaling_recv: Some(recv),
            control_send: Some(send),
            control_recv: Some(recv),
            transport_ready: Some(ready),
            configured_port: Some(port),
            advertised_addr: Some(addr),
            ensure_transport: Some(ensure),
        });
        assert_eq!(signaling_send_to(&[9], 0, 0), 9);
        let mut buf = [0u8; 1];
        let (mut from_addr, mut from_port) = (0, 0);
        assert_eq!(
            signaling_recv_from(&mut buf, &mut from_addr, &mut from_port),
            1
        );
        assert_eq!((buf[0], from_addr, from_port), (42, 7, 8));
        assert_eq!(control_send_to(&[3], 0, 0), 3);
        assert!(transport_is_ready());
        assert_eq!(configured_port(), 3658);
        assert_eq!(advertised_addr(), 0x01020304);
        assert!(ensure_transport());

        set_peer_resolver(resolve);
        let (mut a, mut p) = (0, 0);
        assert!(resolve_peer("hunter", &mut a, &mut p));
        assert_eq!((a, p), (6, 1));
        set_matching2_enabled(true);
        assert!(matching2_enabled());
        set_mm_server_endpoint(1, 2);
        assert_eq!(mm_server_addr(), 1);
        assert_eq!(mm_server_udp_port(), 2);
    }
}
