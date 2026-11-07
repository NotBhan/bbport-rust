//! Host socket layer, ported from
//! `legacy/gpu/bbnet/vendor/shadps4/core/net/host_net.h` and `host_net.cpp`.
//!
//! Linux only, matching the port's target: sockets are non-blocking with CLOEXEC, the wake
//! handle is an `eventfd`, and the epoll wrapper drives the guest epoll.

use std::time::{Duration, Instant};

/// `NativeSocket`.
pub type NativeSocket = i32;
/// `InvalidSocket`.
pub const INVALID_SOCKET: NativeSocket = -1;

/// `Clock`.
pub type Clock = Instant;
/// `Deadline`: `None` means no deadline.
pub type Deadline = Option<Instant>;

/// `DeadlineFromSocketTimeout`.
pub fn deadline_from_socket_timeout(timeout: Duration) -> Deadline {
    if timeout.is_zero() {
        None
    } else {
        Some(Clock::now() + timeout)
    }
}

fn timeout_ms(deadline: &Deadline) -> i32 {
    let Some(deadline) = deadline else {
        return -1;
    };
    let now = Clock::now();
    if *deadline <= now {
        return 0;
    }
    let left = *deadline - now;
    let ms = left.as_millis().min(i32::MAX as u128) as i64;
    // Round up so we do not wake early.
    let ms = if !left.subsec_nanos().is_multiple_of(1_000_000) || left.as_millis() == 0 {
        ms.max(1)
    } else {
        ms
    };
    ms as i32
}

fn expired(deadline: &Deadline) -> bool {
    deadline.is_some_and(|deadline| Clock::now() >= deadline)
}

/// `Error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Error {
    Ok = 0,
    Perm = 1,
    NoEnt = 2,
    Intr = 4,
    Io = 5,
    BadF = 9,
    NoMem = 12,
    Acces = 13,
    Fault = 14,
    NotBlk = 15,
    Busy = 16,
    Exist = 17,
    NoDev = 19,
    Inval = 22,
    NFile = 23,
    MFile = 24,
    NoSpc = 28,
    Pipe = 32,
    WouldBlock = 35,
    InProgress = 36,
    Already = 37,
    NotSock = 38,
    DestAddrReq = 39,
    MsgSize = 40,
    ProtoType = 41,
    NoProtoOpt = 42,
    ProtoNoSupport = 43,
    SocktNoSupport = 44,
    OpNotSupp = 45,
    PfNoSupport = 46,
    AfNoSupport = 47,
    AddrInUse = 48,
    AddrNotAvail = 49,
    NetDown = 50,
    NetUnreach = 51,
    NetReset = 52,
    ConnAborted = 53,
    ConnReset = 54,
    NoBufs = 55,
    IsConn = 56,
    NotConn = 57,
    TooManyRefs = 59,
    TimedOut = 60,
    ConnRefused = 61,
    Loop = 62,
    NameTooLong = 63,
    HostDown = 64,
    HostUnreach = 65,
    NotEmpty = 66,
    Canceled = 85,
    Internal = 204,
}

/// `TranslateNative`.
pub fn translate_native(code: i32) -> Error {
    if code == 0 {
        return Error::Ok;
    }
    match code {
        libc::EPERM => Error::Perm,
        libc::ENOENT => Error::NoEnt,
        libc::EINTR => Error::Intr,
        libc::EIO => Error::Io,
        libc::EBADF => Error::BadF,
        libc::ENOMEM => Error::NoMem,
        libc::EACCES => Error::Acces,
        libc::EFAULT => Error::Fault,
        libc::EBUSY => Error::Busy,
        libc::EEXIST => Error::Exist,
        libc::ENODEV => Error::NoDev,
        libc::EINVAL => Error::Inval,
        libc::ENFILE => Error::NFile,
        libc::EMFILE => Error::MFile,
        libc::ENOSPC => Error::NoSpc,
        libc::EPIPE => Error::Pipe,
        libc::EAGAIN => Error::WouldBlock,
        libc::EINPROGRESS => Error::InProgress,
        libc::EALREADY => Error::Already,
        libc::ENOTSOCK => Error::NotSock,
        libc::EDESTADDRREQ => Error::DestAddrReq,
        libc::EMSGSIZE => Error::MsgSize,
        libc::EPROTOTYPE => Error::ProtoType,
        libc::ENOPROTOOPT => Error::NoProtoOpt,
        libc::EPROTONOSUPPORT => Error::ProtoNoSupport,
        libc::ESOCKTNOSUPPORT => Error::SocktNoSupport,
        libc::EOPNOTSUPP => Error::OpNotSupp,
        libc::EPFNOSUPPORT => Error::PfNoSupport,
        libc::EAFNOSUPPORT => Error::AfNoSupport,
        libc::EADDRINUSE => Error::AddrInUse,
        libc::EADDRNOTAVAIL => Error::AddrNotAvail,
        libc::ENETDOWN => Error::NetDown,
        libc::ENETUNREACH => Error::NetUnreach,
        libc::ENETRESET => Error::NetReset,
        libc::ECONNABORTED => Error::ConnAborted,
        libc::ECONNRESET => Error::ConnReset,
        libc::ENOBUFS => Error::NoBufs,
        libc::EISCONN => Error::IsConn,
        libc::ENOTCONN => Error::NotConn,
        libc::ESHUTDOWN => Error::Pipe,
        libc::ETOOMANYREFS => Error::TooManyRefs,
        libc::ETIMEDOUT => Error::TimedOut,
        libc::ECONNREFUSED => Error::ConnRefused,
        libc::ELOOP => Error::Loop,
        libc::ENAMETOOLONG => Error::NameTooLong,
        libc::EHOSTDOWN => Error::HostDown,
        libc::EHOSTUNREACH => Error::HostUnreach,
        libc::ENOTEMPTY => Error::NotEmpty,
        libc::ECANCELED => Error::Canceled,
        _ => Error::Internal,
    }
}

/// `LastError`.
pub fn last_error() -> Error {
    translate_native(std::io::Error::last_os_error().raw_os_error().unwrap_or(0))
}

/// `Initialize`; no Winsock equivalent is needed.
pub fn initialize() -> bool {
    true
}

/// `Shutdown`.
pub fn shutdown() {}

/// `IoResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoResult {
    pub value: i64,
    pub error: Error,
}

fn set_non_blocking(socket: NativeSocket) -> bool {
    // SAFETY: fcntl only inspects and updates flags of the given descriptor.
    unsafe {
        let flags = libc::fcntl(socket, libc::F_GETFL, 0);
        flags >= 0 && libc::fcntl(socket, libc::F_SETFL, flags | libc::O_NONBLOCK) == 0
    }
}

fn set_close_on_exec(socket: NativeSocket) -> bool {
    // SAFETY: fcntl only inspects and updates flags of the given descriptor.
    unsafe {
        let flags = libc::fcntl(socket, libc::F_GETFD, 0);
        flags >= 0 && libc::fcntl(socket, libc::F_SETFD, flags | libc::FD_CLOEXEC) == 0
    }
}

/// `ConfigureSocket`.
pub fn configure_socket(socket: NativeSocket, _socket_type: i32) -> bool {
    if !set_non_blocking(socket) {
        return false;
    }
    set_close_on_exec(socket);
    true
}

/// `CreateSocket`.
pub fn create_socket(
    family: i32,
    socket_type: i32,
    protocol: i32,
    error: &mut Error,
) -> NativeSocket {
    // SAFETY: a plain socket creation call.
    let socket = unsafe {
        libc::socket(
            family,
            socket_type | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
            protocol,
        )
    };
    if socket == INVALID_SOCKET {
        *error = last_error();
        return INVALID_SOCKET;
    }
    if !configure_socket(socket, socket_type) {
        *error = last_error();
        close_socket(socket);
        return INVALID_SOCKET;
    }
    *error = Error::Ok;
    socket
}

/// `CreateSocketPair`.
pub fn create_socket_pair(
    family: i32,
    socket_type: i32,
    protocol: i32,
    out: &mut [NativeSocket; 2],
) -> Error {
    out[0] = INVALID_SOCKET;
    out[1] = INVALID_SOCKET;
    let mut fds = [0i32; 2];
    // SAFETY: `socketpair` writes two descriptors into the array.
    if unsafe { libc::socketpair(family, socket_type, protocol, fds.as_mut_ptr()) } != 0 {
        return last_error();
    }
    if !configure_socket(fds[0], socket_type) || !configure_socket(fds[1], socket_type) {
        let error = last_error();
        close_socket(fds[0]);
        close_socket(fds[1]);
        return error;
    }
    out[0] = fds[0];
    out[1] = fds[1];
    Error::Ok
}

/// `CloseSocket`.
pub fn close_socket(socket: NativeSocket) {
    if socket == INVALID_SOCKET {
        return;
    }
    // SAFETY: closing a descriptor we own; no retry on EINTR, as in the original.
    unsafe {
        libc::close(socket);
    }
}

/// `PrepareBind`.
pub fn prepare_bind(socket: NativeSocket, reuse_addr: bool, reuse_port: bool) -> Error {
    let on = i32::from(reuse_addr);
    // SAFETY: the option value points at a live `int` for the duration of the call.
    if unsafe {
        libc::setsockopt(
            socket,
            libc::SOL_SOCKET,
            libc::SO_REUSEADDR,
            &on as *const i32 as *const libc::c_void,
            std::mem::size_of::<i32>() as libc::socklen_t,
        )
    } != 0
    {
        return last_error();
    }
    let on = i32::from(reuse_port);
    // SAFETY: as above.
    if unsafe {
        libc::setsockopt(
            socket,
            libc::SOL_SOCKET,
            libc::SO_REUSEPORT,
            &on as *const i32 as *const libc::c_void,
            std::mem::size_of::<i32>() as libc::socklen_t,
        )
    } != 0
    {
        return last_error();
    }
    Error::Ok
}

/// `PrepareClose`.
pub fn prepare_close(socket: NativeSocket, guest_blocking: bool) {
    let mut value: libc::linger = unsafe { std::mem::zeroed() };
    let mut length = std::mem::size_of::<libc::linger>() as libc::socklen_t;
    // SAFETY: `value` and `length` are valid for the call.
    let result = unsafe {
        libc::getsockopt(
            socket,
            libc::SOL_SOCKET,
            libc::SO_LINGER,
            &mut value as *mut libc::linger as *mut libc::c_void,
            &mut length,
        )
    };
    if result != 0 || value.l_onoff == 0 || value.l_linger <= 0 {
        return;
    }
    if guest_blocking {
        // SAFETY: fcntl only updates the descriptor's flags.
        unsafe {
            let flags = libc::fcntl(socket, libc::F_GETFL, 0);
            if flags >= 0 {
                libc::fcntl(socket, libc::F_SETFL, flags & !libc::O_NONBLOCK);
            }
        }
        return;
    }
    value.l_onoff = 0;
    // SAFETY: the linger value is valid for the call.
    unsafe {
        libc::setsockopt(
            socket,
            libc::SOL_SOCKET,
            libc::SO_LINGER,
            &value as *const libc::linger as *const libc::c_void,
            std::mem::size_of::<libc::linger>() as libc::socklen_t,
        );
    }
}

/// `Bind`.
///
/// # Safety
/// `addr` must point at a valid socket address of `len` bytes.
pub unsafe fn bind(
    socket: NativeSocket,
    addr: *const libc::sockaddr,
    len: libc::socklen_t,
) -> Error {
    if libc::bind(socket, addr, len) == 0 {
        Error::Ok
    } else {
        last_error()
    }
}

/// `Listen`.
pub fn listen(socket: NativeSocket, backlog: i32) -> Error {
    // SAFETY: the descriptor is owned by the caller.
    if unsafe { libc::listen(socket, backlog) } == 0 {
        Error::Ok
    } else {
        last_error()
    }
}

/// `ShutdownSocket`.
pub fn shutdown_socket(socket: NativeSocket, how: i32) -> Error {
    // SAFETY: the descriptor is owned by the caller.
    if unsafe { libc::shutdown(socket, how) } == 0 {
        Error::Ok
    } else {
        last_error()
    }
}

/// `Connect`.
///
/// # Safety
/// `addr` must point at a valid socket address of `len` bytes.
pub unsafe fn connect(
    socket: NativeSocket,
    addr: *const libc::sockaddr,
    len: libc::socklen_t,
) -> Error {
    if libc::connect(socket, addr, len) == 0 {
        return Error::Ok;
    }
    last_error()
}

/// `PendingSocketError`.
pub fn pending_socket_error(socket: NativeSocket) -> Error {
    let mut value = 0i32;
    let mut length = std::mem::size_of::<i32>() as libc::socklen_t;
    // SAFETY: the option value points at a live `int`.
    if unsafe {
        libc::getsockopt(
            socket,
            libc::SOL_SOCKET,
            libc::SO_ERROR,
            &mut value as *mut i32 as *mut libc::c_void,
            &mut length,
        )
    } != 0
    {
        return last_error();
    }
    translate_native(value)
}

/// `BytesReadable`.
pub fn bytes_readable(socket: NativeSocket) -> IoResult {
    let mut value = 0i32;
    // SAFETY: FIONREAD writes an `int`.
    if unsafe { libc::ioctl(socket, libc::FIONREAD, &mut value) } != 0 {
        return IoResult {
            value: 0,
            error: last_error(),
        };
    }
    IoResult {
        value: value as i64,
        error: Error::Ok,
    }
}

/// `Accept`.
///
/// # Safety
/// `addr` and `len` must be null or point at writable socket-address storage.
pub unsafe fn accept(
    socket: NativeSocket,
    addr: *mut libc::sockaddr,
    len: *mut libc::socklen_t,
    error: &mut Error,
) -> NativeSocket {
    loop {
        let client = libc::accept4(socket, addr, len, libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC);
        if client == INVALID_SOCKET {
            let last = last_error();
            if last == Error::Intr {
                continue;
            }
            *error = last;
            return INVALID_SOCKET;
        }
        if !configure_socket(client, libc::SOCK_STREAM) {
            *error = last_error();
            close_socket(client);
            return INVALID_SOCKET;
        }
        *error = Error::Ok;
        return client;
    }
}

/// `SendTo`.
///
/// # Safety
/// `buf` must be readable for `buf.len()` bytes; `addr` must be null or a valid address.
pub unsafe fn send_to(
    socket: NativeSocket,
    buf: &[u8],
    flags: i32,
    addr: Option<*const libc::sockaddr>,
    addr_len: libc::socklen_t,
) -> IoResult {
    let flags = flags | libc::MSG_NOSIGNAL;
    let result = loop {
        let result = match addr {
            Some(addr) => libc::sendto(
                socket,
                buf.as_ptr() as *const libc::c_void,
                buf.len(),
                flags,
                addr,
                addr_len,
            ),
            None => libc::send(
                socket,
                buf.as_ptr() as *const libc::c_void,
                buf.len(),
                flags,
            ),
        };
        if result >= 0 || std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
            break result;
        }
    };
    if result < 0 {
        IoResult {
            value: -1,
            error: last_error(),
        }
    } else {
        IoResult {
            value: result as i64,
            error: Error::Ok,
        }
    }
}

/// `RecvFrom`.
///
/// # Safety
/// `buf` must be writable for `buf.len()` bytes; `addr`/`addr_len` must be null or valid.
pub unsafe fn recv_from(
    socket: NativeSocket,
    buf: &mut [u8],
    flags: i32,
    addr: Option<*mut libc::sockaddr>,
    addr_len: Option<*mut libc::socklen_t>,
) -> IoResult {
    let result = loop {
        let result = match (addr, addr_len) {
            (Some(addr), Some(addr_len)) => libc::recvfrom(
                socket,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
                flags,
                addr,
                addr_len,
            ),
            _ => libc::recv(
                socket,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
                flags,
            ),
        };
        if result >= 0 || std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
            break result;
        }
    };
    if result < 0 {
        IoResult {
            value: -1,
            error: last_error(),
        }
    } else {
        IoResult {
            value: result as i64,
            error: Error::Ok,
        }
    }
}

/// `WakeHandle`.
#[derive(Debug)]
pub struct WakeHandle {
    read: NativeSocket,
    write: NativeSocket,
}

impl WakeHandle {
    /// `WakeHandle()`.
    pub fn new() -> Self {
        // SAFETY: eventfd returns a new descriptor or -1.
        let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };
        Self {
            read: fd,
            write: fd,
        }
    }

    /// `Valid`.
    pub fn valid(&self) -> bool {
        self.read != INVALID_SOCKET
    }

    /// `Signal`.
    pub fn signal(&self) {
        if !self.valid() {
            return;
        }
        let value = 1u64;
        // SAFETY: writing eight bytes to the eventfd, as the counter expects.
        unsafe {
            libc::write(
                self.write,
                &value as *const u64 as *const libc::c_void,
                std::mem::size_of::<u64>(),
            );
        }
    }

    /// `Drain`.
    pub fn drain(&self) {
        if !self.valid() {
            return;
        }
        let mut buffer = [0u8; 64];
        loop {
            // SAFETY: the buffer is valid for the given length.
            let read = unsafe {
                libc::read(
                    self.read,
                    buffer.as_mut_ptr() as *mut libc::c_void,
                    buffer.len(),
                )
            };
            if read <= 0 {
                return;
            }
        }
    }

    /// `PollHandle`.
    pub fn poll_handle(&self) -> NativeSocket {
        self.read
    }
}

impl Default for WakeHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WakeHandle {
    fn drop(&mut self) {
        close_socket(self.read);
    }
}

/// `Interest`.
pub mod interest {
    pub const READABLE: u32 = 1 << 0;
    pub const WRITABLE: u32 = 1 << 1;
}

/// `WaitResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitResult {
    Ready,
    TimedOut,
    Woken,
    Failed,
}

/// `WaitOne`.
pub fn wait_one(
    socket: NativeSocket,
    interest: u32,
    wake: &WakeHandle,
    deadline: Deadline,
) -> WaitResult {
    let mut fds = [libc::pollfd {
        fd: socket,
        events: 0,
        revents: 0,
    }; 2];
    fds[0].events = if interest & interest::READABLE != 0 {
        libc::POLLIN
    } else {
        0
    } | if interest & interest::WRITABLE != 0 {
        libc::POLLOUT
    } else {
        0
    };
    fds[1].fd = wake.poll_handle();
    fds[1].events = libc::POLLIN;

    loop {
        fds[0].revents = 0;
        fds[1].revents = 0;
        // SAFETY: the pollfd array is valid for the call.
        let result = unsafe { libc::poll(fds.as_mut_ptr(), 2, timeout_ms(&deadline)) };
        if result < 0 {
            if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            return WaitResult::Failed;
        }
        if fds[1].revents != 0 {
            return WaitResult::Woken;
        }
        if fds[0].revents != 0 {
            return WaitResult::Ready;
        }
        if expired(&deadline) {
            return WaitResult::TimedOut;
        }
    }
}

/// `EventBits`.
pub mod event_bits {
    pub const EV_IN: u32 = 1 << 0;
    pub const EV_OUT: u32 = 1 << 1;
    pub const EV_ERR: u32 = 1 << 2;
    pub const EV_HUP: u32 = 1 << 3;
    pub const EV_DESC_ID: u32 = 1 << 4;
}

/// `EpollFlags`.
pub const EPOLL_ONESHOT: u32 = 1 << 0;

/// `ReservedTag`.
pub const RESERVED_TAG: u64 = u64::MAX;

/// `EpollEvent`.
#[derive(Debug, Clone, Copy, Default)]
pub struct EpollEvent {
    pub tag: u64,
    pub events: u32,
}

/// `EpollWaitResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpollWaitResult {
    pub count: i32,
    pub woken: bool,
    pub error: Error,
}

fn to_epoll(events: u32, flags: u32) -> u32 {
    let mut epoll = libc::EPOLLRDHUP as u32;
    if events & event_bits::EV_IN != 0 {
        epoll |= libc::EPOLLIN as u32;
    }
    if events & event_bits::EV_OUT != 0 {
        epoll |= libc::EPOLLOUT as u32;
    }
    if flags & EPOLL_ONESHOT != 0 {
        epoll |= libc::EPOLLONESHOT as u32;
    }
    epoll
}

fn from_epoll(events: u32) -> u32 {
    let mut out = 0;
    if events & libc::EPOLLIN as u32 != 0 {
        out |= event_bits::EV_IN;
    }
    if events & libc::EPOLLOUT as u32 != 0 {
        out |= event_bits::EV_OUT;
    }
    if events & libc::EPOLLERR as u32 != 0 {
        out |= event_bits::EV_ERR;
    }
    if events & (libc::EPOLLHUP as u32 | libc::EPOLLRDHUP as u32) != 0 {
        out |= event_bits::EV_HUP;
    }
    out
}

/// `HostEpoll`.
#[derive(Debug)]
pub struct HostEpoll {
    handle: i32,
    wake: WakeHandle,
}

impl HostEpoll {
    /// `HostEpoll()`.
    pub fn new() -> Self {
        // SAFETY: epoll_create1 returns a descriptor or -1.
        let handle = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        let wake = WakeHandle::new();
        let epoll = Self { handle, wake };
        if epoll.valid() && epoll.wake.valid() {
            let mut event = libc::epoll_event {
                events: libc::EPOLLIN as u32,
                u64: RESERVED_TAG,
            };
            // SAFETY: the event is valid for the call.
            unsafe {
                libc::epoll_ctl(
                    epoll.handle,
                    libc::EPOLL_CTL_ADD,
                    epoll.wake.poll_handle(),
                    &mut event,
                );
            }
        }
        epoll
    }

    /// `Valid`.
    pub fn valid(&self) -> bool {
        self.handle >= 0
    }

    fn control(&self, op: i32, socket: NativeSocket, tag: u64, events: u32, flags: u32) -> Error {
        let mut event = libc::epoll_event {
            events: to_epoll(events, flags),
            u64: tag,
        };
        // SAFETY: the event is valid for the call.
        if unsafe { libc::epoll_ctl(self.handle, op, socket, &mut event) } == 0 {
            Error::Ok
        } else {
            last_error()
        }
    }

    /// `Add`.
    pub fn add(&self, socket: NativeSocket, tag: u64, events: u32, flags: u32) -> Error {
        self.control(libc::EPOLL_CTL_ADD, socket, tag, events, flags)
    }

    /// `Modify`.
    pub fn modify(&self, socket: NativeSocket, tag: u64, events: u32, flags: u32) -> Error {
        self.control(libc::EPOLL_CTL_MOD, socket, tag, events, flags)
    }

    /// `Remove`.
    pub fn remove(&self, socket: NativeSocket) -> Error {
        let mut event = libc::epoll_event { events: 0, u64: 0 };
        // SAFETY: the event is valid for the call.
        if unsafe { libc::epoll_ctl(self.handle, libc::EPOLL_CTL_DEL, socket, &mut event) } == 0 {
            Error::Ok
        } else {
            last_error()
        }
    }

    /// `Wait`.
    pub fn wait(&self, out: &mut [EpollEvent], deadline: Deadline) -> EpollWaitResult {
        let capacity = out.len().min(64);
        if capacity == 0 {
            return EpollWaitResult {
                count: 0,
                woken: false,
                error: Error::Inval,
            };
        }
        let mut buffer = vec![libc::epoll_event { events: 0, u64: 0 }; capacity];
        loop {
            // SAFETY: the buffer is valid for `capacity` entries.
            let count = unsafe {
                libc::epoll_wait(
                    self.handle,
                    buffer.as_mut_ptr(),
                    capacity as i32,
                    timeout_ms(&deadline),
                )
            };
            if count < 0 {
                if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                return EpollWaitResult {
                    count: 0,
                    woken: false,
                    error: last_error(),
                };
            }
            let mut result = EpollWaitResult {
                count: 0,
                woken: false,
                error: Error::Ok,
            };
            for event in buffer.iter().take(count as usize) {
                if event.u64 == RESERVED_TAG {
                    result.woken = true;
                    continue;
                }
                out[result.count as usize] = EpollEvent {
                    tag: event.u64,
                    events: from_epoll(event.events),
                };
                result.count += 1;
            }
            if count > 0 || expired(&deadline) {
                return result;
            }
        }
    }

    /// `Wake`.
    pub fn wake(&self) {
        self.wake.signal();
    }

    /// `DrainWake`.
    pub fn drain_wake(&self) {
        self.wake.drain();
    }
}

impl Default for HostEpoll {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for HostEpoll {
    fn drop(&mut self) {
        if self.handle >= 0 {
            // SAFETY: closing the epoll descriptor we own.
            unsafe {
                libc::close(self.handle);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_errors_map_to_the_orbis_enum() {
        assert_eq!(translate_native(0), Error::Ok);
        assert_eq!(translate_native(libc::EAGAIN), Error::WouldBlock);
        assert_eq!(translate_native(libc::EINPROGRESS), Error::InProgress);
        assert_eq!(translate_native(libc::ECONNREFUSED), Error::ConnRefused);
        assert_eq!(translate_native(-12345), Error::Internal);
    }

    #[test]
    fn a_socket_pair_transfers_bytes() {
        let mut pair = [INVALID_SOCKET; 2];
        assert_eq!(
            create_socket_pair(libc::AF_UNIX, libc::SOCK_STREAM, 0, &mut pair),
            Error::Ok
        );
        let sent = unsafe { send_to(pair[0], b"ping", 0, None, 0) };
        assert_eq!(
            sent,
            IoResult {
                value: 4,
                error: Error::Ok
            }
        );
        let mut buffer = [0u8; 8];
        let received = unsafe { recv_from(pair[1], &mut buffer, 0, None, None) };
        assert_eq!(
            received,
            IoResult {
                value: 4,
                error: Error::Ok
            }
        );
        assert_eq!(&buffer[..4], b"ping");
        close_socket(pair[0]);
        close_socket(pair[1]);
    }

    #[test]
    fn the_wake_handle_interrupts_a_wait() {
        let mut pair = [INVALID_SOCKET; 2];
        assert_eq!(
            create_socket_pair(libc::AF_UNIX, libc::SOCK_STREAM, 0, &mut pair),
            Error::Ok
        );
        let wake = WakeHandle::new();
        assert!(wake.valid());
        wake.signal();
        assert_eq!(
            wait_one(
                pair[0],
                interest::READABLE,
                &wake,
                Some(Clock::now() + Duration::from_secs(1))
            ),
            WaitResult::Woken
        );
        wake.drain();
        assert_eq!(
            wait_one(
                pair[0],
                interest::READABLE,
                &wake,
                Some(Clock::now() + Duration::from_millis(1))
            ),
            WaitResult::TimedOut
        );
        close_socket(pair[0]);
        close_socket(pair[1]);
    }

    #[test]
    fn epoll_reports_readable_sockets_and_the_wake_tag() {
        let epoll = HostEpoll::new();
        assert!(epoll.valid());
        let mut pair = [INVALID_SOCKET; 2];
        assert_eq!(
            create_socket_pair(libc::AF_UNIX, libc::SOCK_STREAM, 0, &mut pair),
            Error::Ok
        );
        assert_eq!(epoll.add(pair[1], 7, event_bits::EV_IN, 0), Error::Ok);
        let sent = unsafe { send_to(pair[0], b"x", 0, None, 0) };
        assert_eq!(sent.error, Error::Ok);

        let mut events = [EpollEvent::default(); 8];
        let result = epoll.wait(&mut events, Some(Clock::now() + Duration::from_secs(1)));
        assert_eq!(result.error, Error::Ok);
        assert!(!result.woken);
        assert!(result.count >= 1);
        assert_eq!(events[0].tag, 7);
        assert_ne!(events[0].events & event_bits::EV_IN, 0);

        epoll.wake();
        let result = epoll.wait(&mut events, Some(Clock::now() + Duration::from_secs(1)));
        assert!(result.woken);
        epoll.drain_wake();

        assert_eq!(epoll.remove(pair[1]), Error::Ok);
        close_socket(pair[0]);
        close_socket(pair[1]);
    }

    #[test]
    fn socket_options_and_sizes_work() {
        let mut error = Error::Ok;
        let socket = create_socket(libc::AF_INET, libc::SOCK_DGRAM, 0, &mut error);
        assert_ne!(socket, INVALID_SOCKET);
        assert_eq!(error, Error::Ok);
        assert_eq!(prepare_bind(socket, true, true), Error::Ok);
        assert_eq!(bytes_readable(socket).error, Error::Ok);
        prepare_close(socket, false);
        close_socket(socket);
        assert_eq!(close_socket(INVALID_SOCKET), ());
    }

    #[test]
    fn deadlines_respect_the_bsd_zero_convention() {
        assert_eq!(deadline_from_socket_timeout(Duration::ZERO), None);
        assert!(deadline_from_socket_timeout(Duration::from_micros(5)).is_some());
    }
}
