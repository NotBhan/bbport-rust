//! Startup reachability probe, ported from `bbnet/shadnet/server_probe.{h,cpp}`.

use std::io::Read;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::gpu::bbnet::shadnet::client::{
    PacketType, SHAD_HEADER_SIZE, SHAD_MAX_PACKET_SIZE, SHAD_PROTOCOL_VERSION,
};

pub const SHAD_PROBE_TIMEOUT_MS: u32 = 3000;

/// `ProbeResult`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProbeResult {
    Ok,
    #[default]
    Unreachable,
    VersionMismatch,
    ProtocolError,
}

/// `ProbeInfo`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProbeInfo {
    pub result: ProbeResult,
    pub server_version: u32,
}

/// The probe is implemented by the shadNet client transport.
pub trait ServerProbe {
    fn probe_server(&mut self, host: &str, port: u16, timeout_ms: u32) -> ProbeInfo;
}

fn get_le32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// `ProbeServer`; a direct port of the TCP ServerInfo handshake.
pub fn probe_server(host: &str, port: u16, timeout_ms: u32) -> ProbeInfo {
    let mut info = ProbeInfo::default();
    if host.is_empty() {
        return info;
    }
    let Ok(mut addrs) = (host, port).to_socket_addrs() else {
        return info;
    };
    let Some(addr) = addrs.find(|addr| matches!(addr, SocketAddr::V4(_))) else {
        return info;
    };
    let timeout = Duration::from_millis(u64::from(timeout_ms));
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, timeout) else {
        return info;
    };
    let _ = stream.set_read_timeout(Some(timeout));
    info.result = ProbeResult::ProtocolError;

    let mut header = [0u8; SHAD_HEADER_SIZE as usize];
    if stream.read_exact(&mut header).is_err() {
        return info;
    }
    if header[0] != PacketType::ServerInfo as u8 {
        return info;
    }
    let total_size = get_le32(&header[3..7]);
    if !(SHAD_HEADER_SIZE..=SHAD_MAX_PACKET_SIZE).contains(&total_size) {
        return info;
    }
    let payload_size = total_size - SHAD_HEADER_SIZE;
    let mut payload = vec![0u8; payload_size as usize];
    if payload_size > 0 && stream.read_exact(&mut payload).is_err() {
        return info;
    }
    if payload_size < 4 {
        info.result = ProbeResult::Ok;
        return info;
    }
    info.server_version = get_le32(&payload);
    if info.server_version != SHAD_PROTOCOL_VERSION {
        info.result = ProbeResult::VersionMismatch;
        return info;
    }
    info.result = ProbeResult::Ok;
    info
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::net::TcpListener;

    #[test]
    fn the_probe_rejects_empty_bad_and_connection_refused_hosts() {
        assert_eq!(probe_server("", 1, 10).result, ProbeResult::Unreachable);
        assert_eq!(
            probe_server("127.0.0.1", 1, 10).result,
            ProbeResult::Unreachable
        );
        assert_eq!(SHAD_PROBE_TIMEOUT_MS, 3000);
    }

    #[test]
    fn the_probe_accepts_a_matching_server_info() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut header = [0u8; SHAD_HEADER_SIZE as usize];
            header[0] = PacketType::ServerInfo as u8;
            header[3..7].copy_from_slice(&(SHAD_HEADER_SIZE + 4).to_le_bytes());
            socket.write_all(&header).unwrap();
            socket
                .write_all(&SHAD_PROTOCOL_VERSION.to_le_bytes())
                .unwrap();
            let mut sink = [0u8; 1];
            let _ = socket.read(&mut sink);
        });
        let info = probe_server("127.0.0.1", port, 1000);
        assert_eq!(info.result, ProbeResult::Ok);
        assert_eq!(info.server_version, SHAD_PROTOCOL_VERSION);
        let _ = server.join();
    }
}
