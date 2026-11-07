//! Network types, ported from
//! `legacy/gpu/bbnet/vendor/shadps4/core/libraries/net/net_types.h`.

#![allow(non_upper_case_globals)]

pub type OrbisNetId = i32;

// `OrbisNetFamily`
pub const ORBIS_NET_AF_UNIX: u32 = 1;
pub const ORBIS_NET_AF_INET: u32 = 2;
pub const ORBIS_NET_AF_INET6: u32 = 28;

// `OrbisNetSocketType`
pub const ORBIS_NET_SOCK_STREAM: u32 = 1;
pub const ORBIS_NET_SOCK_DGRAM: u32 = 2;
pub const ORBIS_NET_SOCK_RAW: u32 = 3;
pub const ORBIS_NET_SOCK_DGRAM_P2P: u32 = 6;
pub const ORBIS_NET_SOCK_STREAM_P2P: u32 = 10;

// `OrbisNetProtocol`
pub const ORBIS_NET_IPPROTO_IP: u32 = 0;
pub const ORBIS_NET_IPPROTO_ICMP: u32 = 1;
pub const ORBIS_NET_IPPROTO_IGMP: u32 = 2;
pub const ORBIS_NET_IPPROTO_TCP: u32 = 6;
pub const ORBIS_NET_IPPROTO_UDP: u32 = 17;
pub const ORBIS_NET_IPPROTO_IPV6: u32 = 41;
pub const ORBIS_NET_SOL_SOCKET: u32 = 0xFFFF;

// `OrbisNetSocketIpOption`
pub const ORBIS_NET_IP_HDRINCL: u32 = 2;
pub const ORBIS_NET_IP_TOS: u32 = 3;
pub const ORBIS_NET_IP_TTL: u32 = 4;
pub const ORBIS_NET_IP_MULTICAST_IF: u32 = 9;
pub const ORBIS_NET_IP_MULTICAST_TTL: u32 = 10;
pub const ORBIS_NET_IP_MULTICAST_LOOP: u32 = 11;
pub const ORBIS_NET_IP_ADD_MEMBERSHIP: u32 = 12;
pub const ORBIS_NET_IP_DROP_MEMBERSHIP: u32 = 13;
pub const ORBIS_NET_IP_TTLCHK: u32 = 30;
pub const ORBIS_NET_IP_MAXTTL: u32 = 31;
pub const ORBIS_NET_IP_DONTFRAG: u32 = 67;

// `OrbisNetSocketTcpOption`
pub const ORBIS_NET_TCP_NODELAY: u32 = 1;
pub const ORBIS_NET_TCP_MAXSEG: u32 = 2;
pub const ORBIS_NET_TCP_MSS_TO_ADVERTISE: u32 = 3;
pub const ORBIS_NET_TCP_KEEPLISTEN: u32 = 128;

// `OrbisNetSocketUdpOption`
pub const ORBIS_NET_UDP_SND_ON_SUSPEND: u32 = 128;

// `OrbisNetSocketSoOption`
pub const ORBIS_NET_SO_REUSEADDR: u32 = 0x00000004;
pub const ORBIS_NET_SO_KEEPALIVE: u32 = 0x00000008;
pub const ORBIS_NET_SO_BROADCAST: u32 = 0x00000020;
pub const ORBIS_NET_SO_LINGER: u32 = 0x00000080;
pub const ORBIS_NET_SO_REUSEPORT: u32 = 0x00000200;
pub const ORBIS_NET_SO_ONESBCAST: u32 = 0x00010000;
pub const ORBIS_NET_SO_USECRYPTO: u32 = 0x00020000;
pub const ORBIS_NET_SO_USESIGNATURE: u32 = 0x00040000;
pub const ORBIS_NET_SO_SNDBUF: u32 = 0x1001;
pub const ORBIS_NET_SO_RCVBUF: u32 = 0x1002;
pub const ORBIS_NET_SO_SNDLOWAT: u32 = 0x1003;
pub const ORBIS_NET_SO_RCVLOWAT: u32 = 0x1004;
pub const ORBIS_NET_SO_ERROR: u32 = 0x1007;
pub const ORBIS_NET_SO_TYPE: u32 = 0x1008;
pub const ORBIS_NET_SO_SNDTIMEO: u32 = 0x1105;
pub const ORBIS_NET_SO_RCVTIMEO: u32 = 0x1106;
pub const ORBIS_NET_SO_ERROR_EX: u32 = 0x1107;
pub const ORBIS_NET_SO_ACCEPTTIMEO: u32 = 0x1108;
pub const ORBIS_NET_SO_CONNECTTIMEO: u32 = 0x1109;
pub const ORBIS_NET_SO_NBIO: u32 = 0x1200;
pub const ORBIS_NET_SO_POLICY: u32 = 0x1201;
pub const ORBIS_NET_SO_NAME: u32 = 0x1202;
pub const ORBIS_NET_SO_PRIORITY: u32 = 0x1203;

// `OrbisNetFlags`
pub const ORBIS_NET_MSG_PEEK: u32 = 0x00000002;
pub const ORBIS_NET_MSG_WAITALL: u32 = 0x00000040;
pub const ORBIS_NET_MSG_DONTWAIT: u32 = 0x00000080;
pub const ORBIS_NET_MSG_USECRYPTO: u32 = 0x00100000;
pub const ORBIS_NET_MSG_USESIGNATURE: u32 = 0x00200000;
pub const ORBIS_NET_MSG_PEEKLEN: u32 = 0x00400000 | ORBIS_NET_MSG_PEEK;

// `OrbisNetShutdown`
pub const ORBIS_NET_SHUT_RD: i32 = 0;
pub const ORBIS_NET_SHUT_WR: i32 = 1;
pub const ORBIS_NET_SHUT_RDWR: i32 = 2;

// `OrbisNetEpollFlag`
pub const ORBIS_NET_EPOLL_CTL_ADD: u32 = 1;
pub const ORBIS_NET_EPOLL_CTL_MOD: u32 = 2;
pub const ORBIS_NET_EPOLL_CTL_DEL: u32 = 3;

// `OrbisNetEpollEvents`
pub const ORBIS_NET_EPOLLIN: u32 = 0x1;
pub const ORBIS_NET_EPOLLOUT: u32 = 0x2;
pub const ORBIS_NET_EPOLLERR: u32 = 0x8;
pub const ORBIS_NET_EPOLLHUP: u32 = 0x10;
pub const ORBIS_NET_EPOLLDESCID: u32 = 0x10000;

// `OrbisNetResolverFlag`
pub const ORBIS_NET_RESOLVER_ASYNC: u32 = 0x1;
pub const ORBIS_NET_RESOLVER_START_NTOA_DISABLE_IPADDRESS: u32 = 0x10000;

// `OrbisNetSockInfoState`
pub const ORBIS_NET_SOCKINFO_STATE_UNKNOWN: i32 = 0;
pub const ORBIS_NET_SOCKINFO_STATE_CLOSED: i32 = 1;
pub const ORBIS_NET_SOCKINFO_STATE_OPENED: i32 = 2;
pub const ORBIS_NET_SOCKINFO_STATE_LISTEN: i32 = 3;
pub const ORBIS_NET_SOCKINFO_STATE_SYN_SENT: i32 = 4;
pub const ORBIS_NET_SOCKINFO_STATE_SYN_RECEIVED: i32 = 5;
pub const ORBIS_NET_SOCKINFO_STATE_ESTABLISHED: i32 = 6;
pub const ORBIS_NET_SOCKINFO_STATE_FIN_WAIT_1: i32 = 7;
pub const ORBIS_NET_SOCKINFO_STATE_FIN_WAIT_2: i32 = 8;
pub const ORBIS_NET_SOCKINFO_STATE_CLOSE_WAIT: i32 = 9;
pub const ORBIS_NET_SOCKINFO_STATE_CLOSING: i32 = 10;
pub const ORBIS_NET_SOCKINFO_STATE_LAST_ACK: i32 = 11;
pub const ORBIS_NET_SOCKINFO_STATE_TIME_WAIT: i32 = 12;

// `OrbisNetSockInfoFlags`
pub const ORBIS_NET_SOCKINFO_F_SELF: i32 = 0x00000001;
pub const ORBIS_NET_SOCKINFO_F_KERNEL: i32 = 0x00000002;
pub const ORBIS_NET_SOCKINFO_F_OTHERS: i32 = 0x00000004;
pub const ORBIS_NET_SOCKINFO_F_RECV_WAIT: i32 = 0x00010000;
pub const ORBIS_NET_SOCKINFO_F_SEND_WAIT: i32 = 0x00020000;
pub const ORBIS_NET_SOCKINFO_F_RECV_EWAIT: i32 = 0x00040000;
pub const ORBIS_NET_SOCKINFO_F_SEND_EWAIT: i32 = 0x00080000;
pub const ORBIS_NET_SOCKINFO_F_NONBLOCK: i32 = 0x00200000;
pub const ORBIS_NET_SOCKINFO_F_ALL: i32 = 0x006F0007;

pub const ORBIS_NET_SOCK_POLICY_NUM_MIN: i32 = 0;
pub const ORBIS_NET_SOCK_POLICY_NUM_MAX: i32 = 15;
pub const ORBIS_NET_SOCK_POLICY_NA: i32 = -1;
pub const ORBIS_NET_SOCK_PRIORITY_NUM_USER_MIN: i32 = 8;
pub const ORBIS_NET_SOCK_PRIORITY_NUM_USER_MAX: i32 = 23;
pub const ORBIS_NET_SOCKET_ABORT_FLAG_RCV_PRESERVATION: u32 = 0x1;
pub const ORBIS_NET_SOCKET_ABORT_FLAG_SND_PRESERVATION: u32 = 0x2;
pub const ORBIS_NET_SOCKET_ABORT_FLAG_SND_PRESERVATION_AGAIN: u32 = 0x4;
pub const ORBIS_NET_ADHOC_PORT: u16 = 3658;
pub const ORBIS_NET_INADDR_ANY: u32 = 0x00000000;
pub const ORBIS_NET_INADDR_LOOPBACK: u32 = 0x7f000001;
pub const ORBIS_NET_INADDR_BROADCAST: u32 = 0xffffffff;
pub const ORBIS_NET_INADDR_UNSPEC_GROUP: u32 = 0xe0000000;
pub const ORBIS_NET_INADDR_AUTOIP: u32 = 0xa9fe0000;
pub const ORBIS_NET_IN_CLASSD_NET: u32 = 0xf0000000;
pub const ORBIS_NET_IN_AUTOIP_NET: u32 = 0xffff0000;
pub const ORBIS_NET_INET_ADDRSTRLEN: i32 = 16;
pub const ORBIS_NET_ETHER_ADDRSTRLEN: i32 = 18;
pub const ORBIS_NET_UDP_SND_ON_SUSPEND_DATALEN_MAX: u32 = 512;
pub const ORBIS_NET_RESOLVER_ABORT_FLAG_NTOA_PRESERVATION: u32 = 0x1;
pub const ORBIS_NET_RESOLVER_ABORT_FLAG_ATON_PRESERVATION: u32 = 0x2;
pub const ORBIS_NET_RESOLVER_HOSTNAME_LEN_MAX: i32 = 255;
pub const ORBIS_NET_DEBUG_NAME_LEN_MAX: i32 = 31;

/// `OrbisNetSockaddr`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNetSockaddr {
    pub sa_len: u8,
    pub sa_family: u8,
    pub sa_data: [u8; 14],
}

/// `OrbisNetSockaddrIn`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNetSockaddrIn {
    pub sin_len: u8,
    pub sin_family: u8,
    pub sin_port: u16,
    pub sin_addr: u32,
    pub sin_vport: u16,
    pub sin_zero: [u8; 6],
}

/// `OrbisNetSockaddrIn6`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNetSockaddrIn6 {
    pub sin6_len: u8,
    pub sin6_family: u8,
    pub sin6_port: u16,
    pub sin6_flowinfo: u32,
    pub sin6_addr: [u8; 16],
    pub sin6_scope_id: u32,
}

/// `OrbisNetLinger`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNetLinger {
    pub l_onoff: i32,
    pub l_linger: i32,
}

/// `OrbisNetEpollData`.
#[repr(C)]
#[derive(Clone, Copy)]
pub union OrbisNetEpollData {
    pub ptr: *mut core::ffi::c_void,
    pub data_u32: u32,
    pub fd: i32,
    pub data_u64: u64,
}

/// `OrbisNetEpollEvent`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OrbisNetEpollEvent {
    pub events: u32,
    pub pad: u32,
    pub ident: u64,
    pub data: OrbisNetEpollData,
}

/// `OrbisNetInAddr`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNetInAddr {
    pub inaddr_addr: u32,
}

/// `OrbisNetUdpSndOnSuspend`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNetUdpSndOnSuspend {
    pub onoff: i32,
    pub addr: *mut OrbisNetSockaddr,
    pub addrlen: u32,
    pub data: *mut core::ffi::c_void,
    pub datalen: u32,
}

/// `OrbisNetMemoryPoolStats`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNetMemoryPoolStats {
    pub pool_size: u64,
    pub max_inuse_size: u64,
    pub current_inuse_size: u64,
    pub reserved: i32,
}

/// `OrbisNetStatisticsInfo`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNetStatisticsInfo {
    pub kernel_mem_free_size: i32,
    pub kernel_mem_free_min: i32,
    pub packet_count: i32,
    pub packet_qos_count: i32,
    pub libnet_mem_free_size: i32,
    pub libnet_mem_free_min: i32,
}

/// `OrbisNetSockaddrUn`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNetSockaddrUn {
    pub sun_len: u8,
    pub sun_family: u8,
    pub sun_path: [u8; 104],
}

/// `OrbisNetIovec`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNetIovec {
    pub iov_base: *mut core::ffi::c_void,
    pub iov_len: u64,
}

/// `OrbisNetMsghdr`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNetMsghdr {
    pub msg_name: *mut core::ffi::c_void,
    pub msg_namelen: u32,
    pub msg_iov: *mut OrbisNetIovec,
    pub msg_iovlen: i32,
    pub msg_control: *mut core::ffi::c_void,
    pub msg_controllen: u32,
    pub msg_flags: i32,
}

/// `OrbisNetAddrUnion`.
#[repr(C)]
#[derive(Clone, Copy)]
pub union OrbisNetAddrUnion {
    pub addr: OrbisNetInAddr,
    pub addr6: [u8; 16],
}

/// `OrbisNetResolverAddr`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OrbisNetResolverAddr {
    pub u: OrbisNetAddrUnion,
    pub af: u32,
    pub pad: [u32; 3],
}

/// `OrbisNetResolverInfo`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OrbisNetResolverInfo {
    pub addrs: [OrbisNetResolverAddr; 10],
    pub records: u32,
    pub recordsv4: u32,
    pub pad: [u32; 14],
}

/// `OrbisNetSockInfo`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNetSockInfo {
    pub name: [u8; ORBIS_NET_DEBUG_NAME_LEN_MAX as usize + 1],
    pub pid: i32,
    pub s: OrbisNetId,
    pub socket_type: i8,
    pub policy: i8,
    pub priority: i8,
    pub reserved8: i8,
    pub recv_queue_length: i32,
    pub send_queue_length: i32,
    pub local_adr: OrbisNetInAddr,
    pub remote_adr: OrbisNetInAddr,
    pub local_port: u16,
    pub remote_port: u16,
    pub local_vport: u16,
    pub remote_vport: u16,
    pub state: i32,
    pub flags: i32,
    pub tx_bps: i32,
    pub rx_bps: i32,
    pub max_tx_bps: i32,
    pub max_rx_bps: i32,
    pub tx_vbps: i32,
    pub rx_vbps: i32,
    pub recv_buffer_size: i32,
    pub send_buffer_size: i32,
    pub reserved6: [i32; 8],
    pub tx_drops: i32,
    pub rx_drops: i32,
    pub tx_wait: i32,
    pub reserved: [i32; 2],
}

/// `OrbisNetEtherAddr`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNetEtherAddr {
    pub data: [u8; 6],
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn the_socket_address_sizes_match_the_static_asserts() {
        assert_eq!(size_of::<OrbisNetSockaddr>(), 16);
        assert_eq!(size_of::<OrbisNetSockaddrIn>(), 16);
        assert_eq!(size_of::<OrbisNetSockaddrIn6>(), 28);
        assert_eq!(size_of::<OrbisNetEpollEvent>(), 24);
        assert_eq!(size_of::<OrbisNetSockaddrUn>(), 106);
        assert_eq!(size_of::<OrbisNetMsghdr>(), 48);
        assert_eq!(size_of::<OrbisNetResolverInfo>(), 384);
        assert_eq!(size_of::<OrbisNetSockInfo>(), 160);
        assert_eq!(size_of::<OrbisNetUdpSndOnSuspend>(), 40);
        assert_eq!(size_of::<OrbisNetMemoryPoolStats>(), 32);
        assert_eq!(size_of::<OrbisNetStatisticsInfo>(), 24);
    }

    #[test]
    fn the_option_values_match_the_header() {
        assert_eq!(ORBIS_NET_SO_REUSEADDR, 0x00000004);
        assert_eq!(ORBIS_NET_SO_SNDBUF, 0x1001);
        assert_eq!(ORBIS_NET_MSG_PEEKLEN, 0x00400002);
        assert_eq!(ORBIS_NET_EPOLLDESCID, 0x10000);
        assert_eq!(ORBIS_NET_SOCKINFO_F_ALL, 0x006F0007);
        assert_eq!(ORBIS_NET_ADHOC_PORT, 3658);
    }
}
