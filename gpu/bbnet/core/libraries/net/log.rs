//! Socket diagnostics, ported from
//! `legacy/gpu/bbnet/vendor/shadps4/core/libraries/net/net_log.h` and `net_log.cpp`.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use crate::gpu::bbnet::core::libraries::net::error::*;
use crate::gpu::bbnet::core::libraries::net::types::*;
use crate::gpu::common::logging::classes;

const ERRNO_NAMES: &[(i32, &str)] = &[
    (1, "EPERM"),
    (2, "ENOENT"),
    (3, "ESRCH"),
    (4, "EINTR"),
    (5, "EIO"),
    (6, "ENXIO"),
    (7, "E2BIG"),
    (8, "ENOEXEC"),
    (9, "EBADF"),
    (10, "ECHILD"),
    (11, "EDEADLK"),
    (12, "ENOMEM"),
    (13, "EACCES"),
    (14, "EFAULT"),
    (15, "ENOTBLK"),
    (16, "EBUSY"),
    (17, "EEXIST"),
    (18, "EXDEV"),
    (19, "ENODEV"),
    (20, "ENOTDIR"),
    (21, "EISDIR"),
    (22, "EINVAL"),
    (23, "ENFILE"),
    (24, "EMFILE"),
    (25, "ENOTTY"),
    (26, "ETXTBSY"),
    (27, "EFBIG"),
    (28, "ENOSPC"),
    (29, "ESPIPE"),
    (30, "EROFS"),
    (31, "EMLINK"),
    (32, "EPIPE"),
    (33, "EDOM"),
    (34, "ERANGE"),
    (35, "EWOULDBLOCK"),
    (36, "EINPROGRESS"),
    (37, "EALREADY"),
    (38, "ENOTSOCK"),
    (39, "EDESTADDRREQ"),
    (40, "EMSGSIZE"),
    (41, "EPROTOTYPE"),
    (42, "ENOPROTOOPT"),
    (43, "EPROTONOSUPPORT"),
    (44, "ESOCKTNOSUPPORT"),
    (45, "EOPNOTSUPP"),
    (46, "EPFNOSUPPORT"),
    (47, "EAFNOSUPPORT"),
    (48, "EADDRINUSE"),
    (49, "EADDRNOTAVAIL"),
    (50, "ENETDOWN"),
    (51, "ENETUNREACH"),
    (52, "ENETRESET"),
    (53, "ECONNABORTED"),
    (54, "ECONNRESET"),
    (55, "ENOBUFS"),
    (56, "EISCONN"),
    (57, "ENOTCONN"),
    (58, "ESHUTDOWN"),
    (59, "ETOOMANYREFS"),
    (60, "ETIMEDOUT"),
    (61, "ECONNREFUSED"),
    (62, "ELOOP"),
    (63, "ENAMETOOLONG"),
    (64, "EHOSTDOWN"),
    (65, "EHOSTUNREACH"),
    (66, "ENOTEMPTY"),
    (67, "EPROCLIM"),
    (68, "EUSERS"),
    (69, "EDQUOT"),
    (70, "ESTALE"),
    (71, "EREMOTE"),
    (72, "EBADRPC"),
    (73, "ERPCMISMATCH"),
    (74, "EPROGUNAVAIL"),
    (75, "EPROGMISMATCH"),
    (76, "EPROCUNAVAIL"),
    (77, "ENOLCK"),
    (78, "ENOSYS"),
    (79, "EFTYPE"),
    (80, "EAUTH"),
    (81, "ENEEDAUTH"),
    (82, "EIDRM"),
    (83, "ENOMS"),
    (84, "EOVERFLOW"),
    (85, "ECANCELED"),
    (92, "EPROTO"),
    (160, "EADHOC"),
    (161, "ERESERVED161"),
    (162, "ERESERVED162"),
    (163, "EINACTIVEDISABLED"),
    (164, "ENODATA"),
    (165, "EDESC"),
    (166, "EDESCTIMEDOUT"),
    (167, "ENETINTR"),
    (200, "ENOTINIT"),
    (201, "ENOLIBMEM"),
    (202, "ERESERVED202"),
    (203, "ECALLBACK"),
    (204, "EINTERNAL"),
    (205, "ERETURN"),
    (206, "ENOALLOCMEM"),
    (220, "RESOLVER_EINTERNAL"),
    (221, "RESOLVER_EBUSY"),
    (222, "RESOLVER_ENOSPACE"),
    (223, "RESOLVER_EPACKET"),
    (224, "RESOLVER_ERESERVED224"),
    (225, "RESOLVER_ENODNS"),
    (226, "RESOLVER_ETIMEDOUT"),
    (227, "RESOLVER_ENOSUPPORT"),
    (228, "RESOLVER_EFORMAT"),
    (229, "RESOLVER_ESERVERFAILURE"),
    (230, "RESOLVER_ENOHOST"),
    (231, "RESOLVER_ENOTIMPLEMENTED"),
    (232, "RESOLVER_ESERVERREFUSED"),
    (233, "RESOLVER_ENORECORD"),
    (234, "RESOLVER_EALIGNMENT"),
    (235, "RESOLVER_ENOTFOUND"),
    (236, "RESOLVER_ENOTINIT"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Routine,
    Network,
    Suspicious,
}

fn severity_of(error: i32) -> Severity {
    if error == ORBIS_NET_EWOULDBLOCK
        || error == ORBIS_NET_EINPROGRESS
        || error == ORBIS_NET_EALREADY
        || error == ORBIS_NET_EISCONN
        || error == ORBIS_NET_EINTR
        || error == ORBIS_NET_ENOTBLK
    {
        return Severity::Routine;
    }
    if error == ORBIS_NET_ECONNREFUSED
        || error == ORBIS_NET_ECONNRESET
        || error == ORBIS_NET_ECONNABORTED
        || error == ORBIS_NET_ENOTCONN
        || error == ORBIS_NET_EPIPE
        || error == ORBIS_NET_ETIMEDOUT
        || error == ORBIS_NET_EHOSTUNREACH
        || error == ORBIS_NET_EHOSTDOWN
        || error == ORBIS_NET_ENETUNREACH
        || error == ORBIS_NET_ENETDOWN
        || error == ORBIS_NET_ENETRESET
        || error == ORBIS_NET_EADDRINUSE
        || error == ORBIS_NET_EADDRNOTAVAIL
        || error == ORBIS_NET_ENOBUFS
        || error == ORBIS_NET_EACCES
        || error == ORBIS_NET_ESHUTDOWN
    {
        return Severity::Network;
    }
    if (0xdc..=0xec).contains(&error) {
        Severity::Network
    } else {
        Severity::Suspicious
    }
}

fn short_name(signature: &str) -> String {
    let head = signature.split('(').next().unwrap_or(signature);
    let start = head.rfind([':', ' ']);
    let mut head = match start {
        Some(index) => &head[index + 1..],
        None => head,
    };
    if let Some(stripped) = head.strip_suffix("Impl") {
        head = stripped;
    }
    head.to_string()
}

fn failure_counts() -> &'static Mutex<HashMap<(String, i32), u64>> {
    static COUNTS: OnceLock<Mutex<HashMap<(String, i32), u64>>> = OnceLock::new();
    COUNTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn count_failure(function: &str, error: i32) -> u64 {
    let mut counts = failure_counts()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let count = counts.entry((function.to_string(), error)).or_insert(0);
    *count += 1;
    *count
}

/// `ErrnoName`.
pub fn errno_name(orbis_errno: i32) -> String {
    ERRNO_NAMES
        .iter()
        .find_map(|(value, name)| (*value == orbis_errno).then_some((*name).to_string()))
        .unwrap_or_else(|| format!("E{orbis_errno}"))
}

/// `ErrorCodeName`.
pub fn error_code_name(code: i32) -> String {
    let value = code as u32;
    if (value & 0xffff_ff00) == ORBIS_NET_ERROR_BASE as u32 {
        errno_name((value & 0xff) as i32)
    } else {
        format!("{value:#x}")
    }
}

/// `LogFailure`; the C++ `std::source_location` is passed explicitly.
pub fn log_failure(orbis_errno: i32, file: &str, line: u32, function: &str) {
    let function = short_name(function);
    let severity = severity_of(orbis_errno);
    if severity == Severity::Routine {
        crate::log_trace!(
            classes::LIB_NET,
            "{} failed: {}",
            function,
            errno_name(orbis_errno)
        );
        return;
    }
    let count = count_failure(&function, orbis_errno);
    if count > 16 && !count.is_multiple_of(1000) {
        return;
    }
    let repeat = if count > 16 {
        format!(", {count} times so far")
    } else {
        String::new()
    };
    let file = file.rsplit(['/', '\\']).next().unwrap_or(file);
    match severity {
        Severity::Network => crate::log_debug!(
            classes::LIB_NET,
            "{} failed: {} ({}:{}{})",
            function,
            errno_name(orbis_errno),
            file,
            line,
            repeat
        ),
        _ => crate::log_warning!(
            classes::LIB_NET,
            "{} failed: {} ({}:{}{})",
            function,
            errno_name(orbis_errno),
            file,
            line,
            repeat
        ),
    }
}

#[derive(Debug, Default)]
struct Traffic {
    sent_bytes: u64,
    sent_calls: u64,
    received_bytes: u64,
    received_calls: u64,
    peers: HashSet<String>,
}

fn traffic() -> &'static Mutex<HashMap<i32, Traffic>> {
    static TRAFFIC: OnceLock<Mutex<HashMap<i32, Traffic>>> = OnceLock::new();
    TRAFFIC.get_or_init(|| Mutex::new(HashMap::new()))
}

/// `CountTraffic`.
pub fn count_traffic(socket: OrbisNetId, sent: bool, bytes: i64) {
    let mut traffic = traffic().lock().unwrap_or_else(|error| error.into_inner());
    let entry = traffic.entry(socket).or_default();
    if sent {
        entry.sent_bytes = entry.sent_bytes.wrapping_add(bytes as u64);
        entry.sent_calls += 1;
    } else {
        entry.received_bytes = entry.received_bytes.wrapping_add(bytes as u64);
        entry.received_calls += 1;
    }
}

/// `NoteP2PPeer`.
pub fn note_p2p_peer(socket: OrbisNetId, sent: bool, addr: Option<&OrbisNetSockaddr>, len: u32) {
    let Some(addr) = addr else {
        return;
    };
    if addr.sa_family as u32 != ORBIS_NET_AF_INET
        || len < core::mem::size_of::<OrbisNetSockaddrIn>() as u32
    {
        return;
    }
    // SAFETY: the length check above guarantees a full sockaddr_in is readable.
    let sockaddr_in: OrbisNetSockaddrIn =
        unsafe { core::ptr::read_unaligned(addr as *const OrbisNetSockaddr as *const _) };
    if sockaddr_in.sin_vport == 0 {
        return;
    }
    const MAX_PEERS: usize = 16;
    let text = format_sockaddr(Some(addr), len);
    {
        let mut traffic = traffic().lock().unwrap_or_else(|error| error.into_inner());
        let peers = &mut traffic.entry(socket).or_default().peers;
        let key = format!("{}{}", if sent { "> " } else { "< " }, text);
        if peers.len() >= MAX_PEERS || !peers.insert(key) {
            return;
        }
    }
    crate::log_info!(
        classes::LIB_NET,
        "socket {}: first P2P packet {} {}",
        socket,
        if sent { "to" } else { "from" },
        text
    );
}

/// `LogSocketClosed`.
pub fn log_socket_closed(socket: OrbisNetId) {
    let entry = {
        let mut traffic = traffic().lock().unwrap_or_else(|error| error.into_inner());
        traffic.remove(&socket)
    };
    let entry = entry.unwrap_or_default();
    crate::log_debug!(
        classes::LIB_NET,
        "socket {} closed: sent {} bytes in {} calls, received {} bytes in {} calls",
        socket,
        entry.sent_bytes,
        entry.sent_calls,
        entry.received_bytes,
        entry.received_calls
    );
}

/// `FormatSockaddr`.
pub fn format_sockaddr(addr: Option<&OrbisNetSockaddr>, len: u32) -> String {
    let Some(addr) = addr else {
        return "null".to_string();
    };
    if len < 2 {
        return format!("(length {len})");
    }
    if addr.sa_family as u32 == ORBIS_NET_AF_INET
        && len >= core::mem::size_of::<OrbisNetSockaddrIn>() as u32
    {
        // SAFETY: the length check guarantees a full sockaddr_in is readable.
        let sockaddr_in: OrbisNetSockaddrIn =
            unsafe { core::ptr::read_unaligned(addr as *const OrbisNetSockaddr as *const _) };
        let ip = u32::from_be(sockaddr_in.sin_addr);
        let mut text = format!(
            "{}.{}.{}.{}:{}",
            ip >> 24,
            (ip >> 16) & 0xff,
            (ip >> 8) & 0xff,
            ip & 0xff,
            u16::from_be(sockaddr_in.sin_port)
        );
        if sockaddr_in.sin_vport != 0 {
            text.push_str(&format!(" vport {}", u16::from_be(sockaddr_in.sin_vport)));
        }
        return text;
    }
    if addr.sa_family as u32 == ORBIS_NET_AF_UNIX {
        // SAFETY: `len` bounds the bytes after sa_family; the scan stops at a NUL or the end.
        let path = unsafe {
            let bytes = core::slice::from_raw_parts(
                (addr as *const OrbisNetSockaddr).cast::<u8>().add(2),
                (len - 2) as usize,
            );
            let end = bytes
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(bytes.len());
            String::from_utf8_lossy(&bytes[..end]).into_owned()
        };
        return format!("unix:{path}");
    }
    format!("family {} (length {})", addr.sa_family, len)
}

/// `SocketTypeName`.
pub fn socket_type_name(socket_type: i32) -> &'static str {
    match socket_type as u32 {
        ORBIS_NET_SOCK_STREAM => "STREAM",
        ORBIS_NET_SOCK_DGRAM => "DGRAM",
        ORBIS_NET_SOCK_RAW => "RAW",
        ORBIS_NET_SOCK_DGRAM_P2P => "DGRAM_P2P",
        ORBIS_NET_SOCK_STREAM_P2P => "STREAM_P2P",
        _ => "unknown",
    }
}

/// `OptionName`.
pub fn option_name(level: i32, name: i32) -> String {
    let option = match level as u32 {
        ORBIS_NET_SOL_SOCKET => match name as u32 {
            ORBIS_NET_SO_REUSEADDR => Some("SO_REUSEADDR"),
            ORBIS_NET_SO_KEEPALIVE => Some("SO_KEEPALIVE"),
            ORBIS_NET_SO_BROADCAST => Some("SO_BROADCAST"),
            ORBIS_NET_SO_LINGER => Some("SO_LINGER"),
            ORBIS_NET_SO_REUSEPORT => Some("SO_REUSEPORT"),
            ORBIS_NET_SO_ONESBCAST => Some("SO_ONESBCAST"),
            ORBIS_NET_SO_USECRYPTO => Some("SO_USECRYPTO"),
            ORBIS_NET_SO_USESIGNATURE => Some("SO_USESIGNATURE"),
            ORBIS_NET_SO_SNDBUF => Some("SO_SNDBUF"),
            ORBIS_NET_SO_RCVBUF => Some("SO_RCVBUF"),
            ORBIS_NET_SO_SNDLOWAT => Some("SO_SNDLOWAT"),
            ORBIS_NET_SO_RCVLOWAT => Some("SO_RCVLOWAT"),
            ORBIS_NET_SO_ERROR => Some("SO_ERROR"),
            ORBIS_NET_SO_TYPE => Some("SO_TYPE"),
            ORBIS_NET_SO_SNDTIMEO => Some("SO_SNDTIMEO"),
            ORBIS_NET_SO_RCVTIMEO => Some("SO_RCVTIMEO"),
            ORBIS_NET_SO_ERROR_EX => Some("SO_ERROR_EX"),
            ORBIS_NET_SO_ACCEPTTIMEO => Some("SO_ACCEPTTIMEO"),
            ORBIS_NET_SO_CONNECTTIMEO => Some("SO_CONNECTTIMEO"),
            ORBIS_NET_SO_NBIO => Some("SO_NBIO"),
            ORBIS_NET_SO_POLICY => Some("SO_POLICY"),
            ORBIS_NET_SO_NAME => Some("SO_NAME"),
            ORBIS_NET_SO_PRIORITY => Some("SO_PRIORITY"),
            0x1005 => Some("SO_SNDTIMEO (BSD)"),
            0x1006 => Some("SO_RCVTIMEO (BSD)"),
            _ => None,
        },
        ORBIS_NET_IPPROTO_IP => match name as u32 {
            ORBIS_NET_IP_HDRINCL => Some("IP_HDRINCL"),
            ORBIS_NET_IP_TOS => Some("IP_TOS"),
            ORBIS_NET_IP_TTL => Some("IP_TTL"),
            ORBIS_NET_IP_MULTICAST_IF => Some("IP_MULTICAST_IF"),
            ORBIS_NET_IP_MULTICAST_TTL => Some("IP_MULTICAST_TTL"),
            ORBIS_NET_IP_MULTICAST_LOOP => Some("IP_MULTICAST_LOOP"),
            ORBIS_NET_IP_ADD_MEMBERSHIP => Some("IP_ADD_MEMBERSHIP"),
            ORBIS_NET_IP_DROP_MEMBERSHIP => Some("IP_DROP_MEMBERSHIP"),
            ORBIS_NET_IP_TTLCHK => Some("IP_TTLCHK"),
            ORBIS_NET_IP_MAXTTL => Some("IP_MAXTTL"),
            ORBIS_NET_IP_DONTFRAG => Some("IP_DONTFRAG"),
            _ => None,
        },
        ORBIS_NET_IPPROTO_TCP => match name as u32 {
            ORBIS_NET_TCP_NODELAY => Some("TCP_NODELAY"),
            ORBIS_NET_TCP_MAXSEG => Some("TCP_MAXSEG"),
            ORBIS_NET_TCP_MSS_TO_ADVERTISE => Some("TCP_MSS_TO_ADVERTISE"),
            ORBIS_NET_TCP_KEEPLISTEN => Some("TCP_KEEPLISTEN"),
            _ => None,
        },
        ORBIS_NET_IPPROTO_UDP => {
            (name as u32 == ORBIS_NET_UDP_SND_ON_SUSPEND).then_some("UDP_SND_ON_SUSPEND")
        }
        _ => None,
    };
    let level_name = match level as u32 {
        ORBIS_NET_SOL_SOCKET => Some("SOL_SOCKET"),
        ORBIS_NET_IPPROTO_IP => Some("IPPROTO_IP"),
        ORBIS_NET_IPPROTO_TCP => Some("IPPROTO_TCP"),
        ORBIS_NET_IPPROTO_UDP => Some("IPPROTO_UDP"),
        _ => None,
    };
    match (level_name, option) {
        (None, _) => format!("level {level:#x}/option {name:#x}"),
        (Some(level_name), None) => format!("{level_name}/option {name:#x}"),
        (Some(level_name), Some(option)) => format!("{level_name}/{option}"),
    }
}

/// `EpollEventsName`.
pub fn epoll_events_name(events: u32) -> String {
    const BITS: [(u32, &str); 5] = [
        (ORBIS_NET_EPOLLIN, "IN"),
        (ORBIS_NET_EPOLLOUT, "OUT"),
        (ORBIS_NET_EPOLLERR, "ERR"),
        (ORBIS_NET_EPOLLHUP, "HUP"),
        (ORBIS_NET_EPOLLDESCID, "DESCID"),
    ];
    let mut out = String::new();
    let mut rest = events;
    for (bit, name) in BITS {
        if events & bit != 0 {
            if !out.is_empty() {
                out.push('|');
            }
            out.push_str(name);
            rest &= !bit;
        }
    }
    if rest != 0 {
        out.push_str(&format!(
            "{}{rest:#x}",
            if out.is_empty() { "" } else { "|" }
        ));
    }
    if out.is_empty() {
        "0".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn errno_and_error_code_names_follow_the_tables() {
        assert_eq!(errno_name(ORBIS_NET_EAGAIN), "EWOULDBLOCK");
        assert_eq!(errno_name(ORBIS_NET_RESOLVER_ENOTINIT), "RESOLVER_ENOTINIT");
        assert_eq!(errno_name(999), "E999");
        assert_eq!(error_code_name(0x80410123u32 as i32), "EWOULDBLOCK");
        assert_eq!(error_code_name(0x1234), "0x1234");
    }

    #[test]
    fn socket_type_and_option_names_match() {
        assert_eq!(socket_type_name(ORBIS_NET_SOCK_STREAM as i32), "STREAM");
        assert_eq!(
            socket_type_name(ORBIS_NET_SOCK_STREAM_P2P as i32),
            "STREAM_P2P"
        );
        assert_eq!(socket_type_name(99), "unknown");
        assert_eq!(
            option_name(ORBIS_NET_SOL_SOCKET as i32, ORBIS_NET_SO_REUSEADDR as i32),
            "SOL_SOCKET/SO_REUSEADDR"
        );
        assert_eq!(
            option_name(ORBIS_NET_SOL_SOCKET as i32, 0x1005),
            "SOL_SOCKET/SO_SNDTIMEO (BSD)"
        );
        assert_eq!(
            option_name(ORBIS_NET_IPPROTO_TCP as i32, ORBIS_NET_TCP_NODELAY as i32),
            "IPPROTO_TCP/TCP_NODELAY"
        );
        assert_eq!(
            option_name(
                ORBIS_NET_IPPROTO_UDP as i32,
                ORBIS_NET_UDP_SND_ON_SUSPEND as i32
            ),
            "IPPROTO_UDP/UDP_SND_ON_SUSPEND"
        );
        assert_eq!(option_name(100, 7), "level 0x64/option 0x7");
        assert_eq!(
            option_name(ORBIS_NET_SOL_SOCKET as i32, 0x9999),
            "SOL_SOCKET/option 0x9999"
        );
    }

    #[test]
    fn epoll_event_names_join_the_bits() {
        assert_eq!(epoll_events_name(0), "0");
        assert_eq!(
            epoll_events_name(ORBIS_NET_EPOLLIN | ORBIS_NET_EPOLLOUT),
            "IN|OUT"
        );
        assert_eq!(epoll_events_name(ORBIS_NET_EPOLLDESCID), "DESCID");
        assert_eq!(epoll_events_name(0x4), "0x4");
        assert_eq!(epoll_events_name(ORBIS_NET_EPOLLIN | 0x4), "IN|0x4");
    }

    #[test]
    fn sockaddr_formatting_matches_the_original() {
        assert_eq!(format_sockaddr(None, 0), "null");

        let mut addr = OrbisNetSockaddr {
            sa_len: size_of::<OrbisNetSockaddrIn>() as u8,
            sa_family: ORBIS_NET_AF_INET as u8,
            sa_data: [0; 14],
        };
        assert_eq!(format_sockaddr(Some(&addr), 1), "(length 1)");
        // SAFETY: the struct is large enough for the sockaddr_in fields written here.
        unsafe {
            let inet = &mut *(&mut addr as *mut OrbisNetSockaddr as *mut OrbisNetSockaddrIn);
            inet.sin_port = 8080u16.to_be();
            inet.sin_addr = 0x7f000001u32.to_be();
            inet.sin_vport = 3658u16.to_be();
        }
        assert_eq!(
            format_sockaddr(Some(&addr), size_of::<OrbisNetSockaddrIn>() as u32),
            "127.0.0.1:8080 vport 3658"
        );

        let mut unix = OrbisNetSockaddr {
            sa_len: 8,
            sa_family: ORBIS_NET_AF_UNIX as u8,
            sa_data: [0; 14],
        };
        unix.sa_data[..5].copy_from_slice(b"/tmp\0");
        assert_eq!(format_sockaddr(Some(&unix), 8), "unix:/tmp");

        let mut other = OrbisNetSockaddr {
            sa_len: 4,
            sa_family: 99,
            sa_data: [0; 14],
        };
        other.sa_family = 99;
        assert_eq!(format_sockaddr(Some(&other), 4), "family 99 (length 4)");
    }

    #[test]
    fn traffic_accounting_is_reset_when_the_socket_closes() {
        count_traffic(5, true, 100);
        count_traffic(5, false, 40);
        count_traffic(5, true, 1);
        log_socket_closed(5);
        assert!(traffic().lock().unwrap().get(&5).is_none());
        log_socket_closed(5);
    }

    #[test]
    fn short_names_strip_signatures_and_impl_suffixes() {
        assert_eq!(
            short_name("Libraries::Net::sceNetSocket(int, int, int)"),
            "sceNetSocket"
        );
        assert_eq!(short_name("sceNetBindImpl"), "sceNetBind");
        assert_eq!(short_name("bind"), "bind");
    }
}
