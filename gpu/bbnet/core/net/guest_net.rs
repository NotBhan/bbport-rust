//! Guest network object API, ported from `bbnet/vendor/shadps4/core/net/guest_net.h`.

use std::sync::Arc;

use crate::gpu::bbnet::core::net::host_net::Error;

/// `NetResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetResult {
    pub value: i64,
    pub error: Error,
}

impl NetResult {
    pub fn ok(value: i64) -> Self {
        Self {
            value,
            error: Error::Ok,
        }
    }

    pub fn fail(error: Error) -> Self {
        Self { value: -1, error }
    }

    pub fn is_ok(&self) -> bool {
        self.error == Error::Ok
    }
}

pub const MAX_SOCKETS: usize = 128;
pub const SOCKET_ABORT_PRESERVE_RECV: u32 = 0x1;
pub const SOCKET_ABORT_PRESERVE_SEND: u32 = 0x2;
pub const SOCKET_ABORT_PRESERVE_SEND_AGAIN: u32 = 0x4;
pub const EPOLL_ABORT_PRESERVE: u32 = 0x1;

/// `IdAllocator`.
#[derive(Default)]
pub struct IdAllocator {
    pub allocate: Option<Box<dyn Fn() -> i32 + Send + Sync>>,
    pub release: Option<Box<dyn Fn(i32) + Send + Sync>>,
}

/// `NetObjectKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetObjectKind {
    Socket,
    Epoll,
    External,
}

/// `SocketInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketInfo {
    pub kind: i32,
    pub p2p: bool,
    pub nonblocking: bool,
    pub reuse_addr: bool,
    pub rcv_timeout_us: i64,
    pub snd_timeout_us: i64,
    pub connect_timeout_us: i64,
    pub accept_timeout_us: i64,
    pub connected: bool,
    pub listening: bool,
    pub pending_error: Error,
}

/// `SocketAttributes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketAttributes {
    pub name: [u8; 32],
    pub snd_buf: i32,
    pub rcv_buf: i32,
    pub policy: i32,
    pub priority: i32,
    pub stored_options: std::collections::HashMap<(i32, i32), i32>,
    pub p2p_tcp_port_bound: bool,
    pub pending_memberships: Vec<[u8; 8]>,
}

impl Default for SocketAttributes {
    fn default() -> Self {
        Self {
            name: [0; 32],
            snd_buf: 0,
            rcv_buf: 0,
            policy: 0,
            priority: 16,
            stored_options: std::collections::HashMap::new(),
            p2p_tcp_port_bound: false,
            pending_memberships: Vec::new(),
        }
    }
}

/// `P2PSendProtection`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct P2PSendProtection {
    pub crypto: bool,
    pub signature: bool,
}

/// `EpollOp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollOp {
    Add,
    Modify,
    Delete,
}

/// `GuestEpollEvent`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GuestEpollEvent {
    pub events: u32,
    pub ident: i32,
    pub data: u64,
}

/// `SelectEntry`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SelectEntry {
    pub id: i32,
    pub interest: u32,
    pub ready: u32,
}

/// The transport surface used for P2P sockets.
pub trait P2PTransport: Send + Sync {
    fn family(&self) -> i32;
}

/// The guest network API; host sockets implement it.
pub trait GuestNetApi {
    fn object_kind(&self, id: i32) -> Option<NetObjectKind>;
    fn close_object(&mut self, id: i32) -> NetResult;
    fn reserve_external_id(&mut self) -> NetResult;
    fn signal_external(&mut self, id: i32, hangup: bool) -> NetResult;
    fn release_external_id(&mut self, id: i32) -> NetResult;
    fn list_sockets(&self) -> Vec<i32>;
    fn to_orbis_return(&self, result: NetResult) -> i32;
    fn socket_create(&mut self, family: i32, kind: i32, protocol: i32) -> NetResult;
    fn socket_create_pair(
        &mut self,
        family: i32,
        kind: i32,
        protocol: i32,
    ) -> Result<[i32; 2], NetResult>;
    fn socket_bind(
        &mut self,
        id: i32,
        addr: *const libc::sockaddr,
        len: libc::socklen_t,
    ) -> NetResult;
    fn socket_listen(&mut self, id: i32, backlog: i32) -> NetResult;
    fn socket_connect(
        &mut self,
        id: i32,
        addr: *const libc::sockaddr,
        len: libc::socklen_t,
    ) -> NetResult;
    fn socket_accept(
        &mut self,
        id: i32,
        addr: *mut libc::sockaddr,
        len: *mut libc::socklen_t,
    ) -> NetResult;
    #[allow(clippy::too_many_arguments)]
    fn socket_send_to(
        &mut self,
        id: i32,
        buf: *const u8,
        len: usize,
        host_flags: i32,
        dontwait: bool,
        addr: *const libc::sockaddr,
        addr_len: libc::socklen_t,
    ) -> NetResult;
    #[allow(clippy::too_many_arguments)]
    fn socket_recv_from(
        &mut self,
        id: i32,
        buf: *mut u8,
        len: usize,
        host_flags: i32,
        dontwait: bool,
        addr: *mut libc::sockaddr,
        addr_len: *mut libc::socklen_t,
    ) -> NetResult;
    fn socket_shutdown(&mut self, id: i32, how: i32) -> NetResult;
    fn socket_get_info(&self, id: i32) -> Option<SocketInfo>;
    fn socket_get_attributes(&self, id: i32) -> Option<SocketAttributes>;
    fn socket_update_attributes(
        &mut self,
        id: i32,
        update: &mut dyn FnMut(&mut SocketAttributes),
    ) -> NetResult;
    fn socket_set_non_blocking(&mut self, id: i32, enable: bool) -> NetResult;
    fn socket_bytes_readable(&mut self, id: i32) -> NetResult;
    fn socket_abort(&mut self, id: i32, flags: u32) -> NetResult;
    fn epoll_create(&mut self) -> NetResult;
    fn epoll_control(
        &mut self,
        eid: i32,
        op: EpollOp,
        socket_id: i32,
        events: u32,
        flags: u32,
        data: u64,
    ) -> NetResult;
    fn epoll_wait(&mut self, eid: i32, out: &mut [GuestEpollEvent], timeout_us: i64) -> NetResult;
    fn epoll_abort(&mut self, eid: i32, flags: u32) -> NetResult;
    fn epoll_destroy(&mut self, eid: i32) -> NetResult;
    fn socket_select(&mut self, entries: &mut [SelectEntry], timeout_us: i64) -> NetResult;
}

/// Process-wide allocator and transport hooks.
#[derive(Default)]
pub struct GuestNetState {
    pub allocator: IdAllocator,
    pub p2p_transports: std::collections::HashMap<i32, Arc<dyn P2PTransport>>,
}

impl GuestNetState {
    pub fn set_id_allocator(&mut self, allocator: IdAllocator) {
        self.allocator = allocator;
    }

    pub fn set_p2p_transport(&mut self, family: i32, transport: Arc<dyn P2PTransport>) {
        self.p2p_transports.insert(family, transport);
    }

    pub fn to_orbis_return(result: NetResult) -> i32 {
        if result.is_ok() {
            result.value as i32
        } else {
            -(result.error as i32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn net_results_and_attributes_default_as_expected() {
        let ok = NetResult::ok(7);
        assert!(ok.is_ok());
        assert_eq!(ok.value, 7);
        let failed = NetResult::fail(Error::Inval);
        assert!(!failed.is_ok());
        assert_eq!(failed.value, -1);
        assert_eq!(GuestNetState::to_orbis_return(ok), 7);
        assert_eq!(GuestNetState::to_orbis_return(failed), -22);
        let attributes = SocketAttributes::default();
        assert_eq!(attributes.priority, 16);
        assert!(attributes.name.iter().all(|&byte| byte == 0));
        assert_eq!(MAX_SOCKETS, 128);
        assert_eq!(SelectEntry::default().id, 0);
        assert_eq!(SOCKET_ABORT_PRESERVE_SEND_AGAIN, 4);
    }
}

/// The guest-net object table implemented by `guest_net.cpp`.
#[derive(Debug, Default)]
pub struct GuestNetObjects {
    pub next_id: i32,
    pub objects: std::collections::HashMap<i32, NetObjectKind>,
    pub sockets: std::collections::HashMap<i32, SocketInfo>,
    pub epolls: std::collections::HashSet<i32>,
}

impl GuestNetObjects {
    pub fn allocate(&mut self, kind: NetObjectKind) -> i32 {
        self.next_id += 1;
        let id = self.next_id;
        self.objects.insert(id, kind);
        if kind == NetObjectKind::Socket {
            self.sockets.insert(
                id,
                SocketInfo {
                    kind: 0,
                    p2p: false,
                    nonblocking: false,
                    reuse_addr: false,
                    rcv_timeout_us: 0,
                    snd_timeout_us: 0,
                    connect_timeout_us: 0,
                    accept_timeout_us: 0,
                    connected: false,
                    listening: false,
                    pending_error: Error::Ok,
                },
            );
        } else if kind == NetObjectKind::Epoll {
            self.epolls.insert(id);
        }
        id
    }

    pub fn object_kind(&self, id: i32) -> Option<NetObjectKind> {
        self.objects.get(&id).copied()
    }

    pub fn close(&mut self, id: i32) -> NetResult {
        if self.objects.remove(&id).is_none() {
            return NetResult::fail(Error::BadF);
        }
        self.sockets.remove(&id);
        self.epolls.remove(&id);
        NetResult::ok(0)
    }

    pub fn list_sockets(&self) -> Vec<i32> {
        self.sockets.keys().copied().collect()
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn guest_objects_allocate_and_close() {
        let mut objects = GuestNetObjects::default();
        let epoll = objects.allocate(NetObjectKind::Epoll);
        assert_eq!(objects.object_kind(epoll), Some(NetObjectKind::Epoll));
        assert_eq!(objects.close(epoll), NetResult::ok(0));
        assert_eq!(objects.close(epoll), NetResult::fail(Error::BadF));
        let socket = objects.allocate(NetObjectKind::Socket);
        assert_eq!(objects.list_sockets(), vec![socket]);
    }
}
