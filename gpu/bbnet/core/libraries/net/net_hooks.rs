//! Kernel network library hooks, ported from
//! `bbnet/vendor/shadps4/core/libraries/net/net.h`.

use std::sync::{Arc, Mutex, OnceLock};

use crate::gpu::bbnet::core::libraries::net::types::{OrbisNetEpollEvent, OrbisNetSockaddr};

/// `SystemHooks`; the frontend installs the system facts the network library reports.
#[derive(Clone, Default)]
pub struct SystemHooks {
    pub is_online: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
    pub mac_address: Option<Arc<dyn Fn() -> Option<[u8; 6]> + Send + Sync>>,
    pub p2p_port: Option<Arc<dyn Fn() -> u16 + Send + Sync>>,
    pub p2p_started: Option<Arc<dyn Fn(u16) + Send + Sync>>,
    pub p2p_stopped: Option<Arc<dyn Fn(u16) + Send + Sync>>,
    pub public_addr: Option<Arc<dyn Fn() -> u32 + Send + Sync>>,
}

fn hooks() -> &'static Mutex<SystemHooks> {
    static HOOKS: OnceLock<Mutex<SystemHooks>> = OnceLock::new();
    HOOKS.get_or_init(|| Mutex::new(SystemHooks::default()))
}

/// `SetSystemHooks`.
pub fn set_system_hooks(new_hooks: SystemHooks) {
    *hooks().lock().unwrap_or_else(|error| error.into_inner()) = new_hooks;
}

/// `GetSystemHooks`.
pub fn get_system_hooks() -> SystemHooks {
    hooks()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

type KernelErrnoHook = Mutex<Option<fn(i32)>>;

fn kernel_errno_hook() -> &'static KernelErrnoHook {
    static HOOK: OnceLock<KernelErrnoHook> = OnceLock::new();
    HOOK.get_or_init(|| Mutex::new(None))
}

/// `SetKernelErrnoHook`.
pub fn set_kernel_errno_hook(hook: Option<fn(i32)>) {
    *kernel_errno_hook()
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = hook;
}

/// Reports a guest errno through the installed kernel hook.
pub fn report_kernel_errno(orbis_errno: i32) {
    let hook = *kernel_errno_hook()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Some(hook) = hook {
        hook(orbis_errno);
    }
}

/// The kernel network entry points; the HLE loader implements them.
pub trait NetApi {
    fn socket(&mut self, name: &str, family: i32, kind: i32, protocol: i32) -> i32;
    fn bind(&mut self, socket: i32, addr: &OrbisNetSockaddr, addr_len: u32) -> i32;
    fn connect(&mut self, socket: i32, addr: &OrbisNetSockaddr, addr_len: u32) -> i32;
    fn listen(&mut self, socket: i32, backlog: i32) -> i32;
    fn accept(&mut self, socket: i32, addr: &mut OrbisNetSockaddr, addr_len: &mut u32) -> i32;
    fn send(&mut self, socket: i32, data: &[u8], flags: i32) -> i32;
    fn recv(&mut self, socket: i32, data: &mut [u8], flags: i32) -> i32;
    fn shutdown(&mut self, socket: i32, how: i32) -> i32;
    fn socket_close(&mut self, socket: i32) -> i32;
    fn epoll_create(&mut self, name: &str, flags: i32) -> i32;
    fn epoll_control(&mut self, eid: i32, op: i32, id: i32, event: &mut OrbisNetEpollEvent) -> i32;
    fn epoll_wait(&mut self, eid: i32, events: &mut [OrbisNetEpollEvent], timeout: i32) -> i32;
    fn epoll_destroy(&mut self, eid: i32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_hooks_round_trip() {
        let mac = Arc::new(|| Some([1, 2, 3, 4, 5, 6]));
        set_system_hooks(SystemHooks {
            is_online: Some(Arc::new(|| true)),
            mac_address: Some(mac),
            p2p_port: Some(Arc::new(|| 3658)),
            p2p_started: None,
            p2p_stopped: None,
            public_addr: Some(Arc::new(|| 0x01020304)),
        });
        let hooks = get_system_hooks();
        assert!(hooks.is_online.as_ref().unwrap()());
        assert_eq!(
            hooks.mac_address.as_ref().unwrap()(),
            Some([1, 2, 3, 4, 5, 6])
        );
        assert_eq!(hooks.p2p_port.as_ref().unwrap()(), 3658);
        assert_eq!(hooks.public_addr.as_ref().unwrap()(), 0x01020304);
        assert!(hooks.p2p_started.is_none());
    }

    #[test]
    fn the_errno_hook_receives_reports() {
        static SEEN: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
        fn hook(errno: i32) {
            SEEN.store(errno, std::sync::atomic::Ordering::SeqCst);
        }
        set_kernel_errno_hook(Some(hook));
        report_kernel_errno(22);
        assert_eq!(SEEN.load(std::sync::atomic::Ordering::SeqCst), 22);
        set_kernel_errno_hook(None);
    }
}
