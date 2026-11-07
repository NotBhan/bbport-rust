//! A small TCP implementation used by the P2P transport, ported from
//! `legacy/gpu/bbnet/vendor/shadps4/core/net/p2p_tcp.h` and `p2p_tcp.cpp`.

use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

use crate::gpu::bbnet::core::net::host_net::Error;

/// `Clock`.
pub type Clock = Instant;

/// `TcpFlag`.
pub mod tcp_flag {
    pub const FIN: u8 = 0x01;
    pub const SYN: u8 = 0x02;
    pub const RST: u8 = 0x04;
    pub const PSH: u8 = 0x08;
    pub const ACK: u8 = 0x10;
}

/// `SeqLt`.
pub fn seq_lt(a: u32, b: u32) -> bool {
    (a.wrapping_sub(b) as i32) < 0
}

/// `SeqLe`.
pub fn seq_le(a: u32, b: u32) -> bool {
    (a.wrapping_sub(b) as i32) <= 0
}

/// `SeqGt`.
pub fn seq_gt(a: u32, b: u32) -> bool {
    seq_lt(b, a)
}

/// `SeqGe`.
pub fn seq_ge(a: u32, b: u32) -> bool {
    seq_le(b, a)
}

/// `PseudoHeader`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PseudoHeader {
    pub family: i32,
    pub local: [u8; 16],
    pub remote: [u8; 16],
}

impl Default for PseudoHeader {
    fn default() -> Self {
        Self {
            family: libc::AF_INET,
            local: [0; 16],
            remote: [0; 16],
        }
    }
}

/// `TcpSegment`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpSegment<'a> {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq: u32,
    pub ack: u32,
    pub flags: u8,
    pub window: u16,
    pub mss: Option<u16>,
    pub payload: &'a [u8],
}

fn read16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([bytes[offset], bytes[offset + 1]])
}

fn read32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn write16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

fn write32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn sum_words(mut sum: u64, data: &[u8]) -> u64 {
    let mut index = 0;
    while index + 1 < data.len() {
        sum += u64::from(read16(data, index));
        index += 2;
    }
    if index < data.len() {
        sum += u64::from(data[index]) << 8;
    }
    sum
}

/// `TcpChecksum`.
pub fn tcp_checksum(family: i32, src: &[u8], dst: &[u8], segment: &[u8]) -> u16 {
    let address_size = if family == libc::AF_INET6 { 16 } else { 4 };
    let mut sum = 0;
    sum = sum_words(sum, &src[..address_size]);
    sum = sum_words(sum, &dst[..address_size]);
    let length = segment.len() as u64;
    sum += (length >> 16) + (length & 0xffff);
    sum += 6; // IPPROTO_TCP
    sum = sum_words(sum, segment);
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !sum as u16
}

/// `ParseTcpSegment`.
pub fn parse_tcp_segment<'a>(
    bytes: &'a [u8],
    pseudo: Option<&PseudoHeader>,
) -> Option<TcpSegment<'a>> {
    if bytes.len() < 20 {
        return None;
    }
    let header_size = usize::from(bytes[12] >> 4) * 4;
    if header_size < 20 || header_size > bytes.len() {
        return None;
    }
    if let Some(pseudo) = pseudo {
        if tcp_checksum(pseudo.family, &pseudo.remote, &pseudo.local, bytes) != 0 {
            return None;
        }
    }
    let mut segment = TcpSegment {
        src_port: read16(bytes, 0),
        dst_port: read16(bytes, 2),
        seq: read32(bytes, 4),
        ack: read32(bytes, 8),
        flags: bytes[13],
        window: read16(bytes, 14),
        mss: None,
        payload: &bytes[header_size..],
    };
    let mut index = 20;
    while index < header_size {
        let kind = bytes[index];
        if kind == 0 {
            break;
        }
        if kind == 1 {
            index += 1;
            continue;
        }
        if index + 1 >= header_size {
            break;
        }
        let length = usize::from(bytes[index + 1]);
        if length < 2 || index + length > header_size {
            break;
        }
        if kind == 2 && length == 4 {
            segment.mss = Some(read16(bytes, index + 2));
        }
        index += length;
    }
    Some(segment)
}

/// `BuildTcpSegment`.
#[allow(clippy::too_many_arguments)]
pub fn build_tcp_segment(
    pseudo: &PseudoHeader,
    src_port: u16,
    dst_port: u16,
    seq: u32,
    ack: u32,
    flags: u8,
    window: u16,
    mss: Option<u16>,
    payload: &[u8],
) -> Vec<u8> {
    let header_size = if mss.is_some() { 24 } else { 20 };
    let mut segment = vec![0u8; header_size + payload.len()];
    write16(&mut segment, 0, src_port);
    write16(&mut segment, 2, dst_port);
    write32(&mut segment, 4, seq);
    write32(&mut segment, 8, ack);
    segment[12] = ((header_size / 4) << 4) as u8;
    segment[13] = flags;
    write16(&mut segment, 14, window);
    if let Some(mss) = mss {
        segment[20] = 2;
        segment[21] = 4;
        write16(&mut segment, 22, mss);
    }
    segment[header_size..].copy_from_slice(payload);
    let checksum = tcp_checksum(pseudo.family, &pseudo.local, &pseudo.remote, &segment);
    write16(&mut segment, 16, checksum);
    segment
}

/// `BuildResetFor`.
pub fn build_reset_for(pseudo: &PseudoHeader, segment: &TcpSegment<'_>) -> Option<Vec<u8>> {
    if segment.flags & tcp_flag::RST != 0 {
        return None;
    }
    if segment.flags & tcp_flag::ACK != 0 {
        return Some(build_tcp_segment(
            pseudo,
            segment.dst_port,
            segment.src_port,
            segment.ack,
            0,
            tcp_flag::RST,
            0,
            None,
            &[],
        ));
    }
    let length = segment.payload.len() as u32
        + u32::from(segment.flags & tcp_flag::SYN != 0)
        + u32::from(segment.flags & tcp_flag::FIN != 0);
    Some(build_tcp_segment(
        pseudo,
        segment.dst_port,
        segment.src_port,
        0,
        segment.seq.wrapping_add(length),
        tcp_flag::RST | tcp_flag::ACK,
        0,
        None,
        &[],
    ))
}

/// `TcpConfig`.
#[derive(Debug, Clone)]
pub struct TcpConfig {
    pub recv_buffer: u32,
    pub send_buffer: u32,
    pub mss: u16,
    pub initial_rto: Duration,
    pub min_rto: Duration,
    pub max_rto: Duration,
    pub delayed_ack: Duration,
    pub time_wait: Duration,
    pub syn_retries: i32,
    pub data_retries: i32,
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            recv_buffer: 65535,
            send_buffer: 32 * 1024,
            mss: 1200,
            initial_rto: Duration::from_millis(1000),
            min_rto: Duration::from_millis(200),
            max_rto: Duration::from_millis(60000),
            delayed_ack: Duration::from_millis(40),
            time_wait: Duration::from_millis(2000),
            syn_retries: 6,
            data_retries: 12,
        }
    }
}

/// `TcpState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

impl TcpState {
    /// `ToString`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Closed => "Closed",
            Self::SynSent => "SynSent",
            Self::SynReceived => "SynReceived",
            Self::Established => "Established",
            Self::FinWait1 => "FinWait1",
            Self::FinWait2 => "FinWait2",
            Self::CloseWait => "CloseWait",
            Self::Closing => "Closing",
            Self::LastAck => "LastAck",
            Self::TimeWait => "TimeWait",
        }
    }
}

/// `TcpConnection::IoResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoResult {
    pub bytes: usize,
    pub error: Error,
}

/// The segment sink.
pub type Emit = Box<dyn FnMut(&[u8])>;

/// `TcpConnection`.
pub struct TcpConnection {
    config: TcpConfig,
    pseudo: PseudoHeader,
    local_port: u16,
    remote_port: u16,
    emit: Emit,

    state: TcpState,
    error: Error,

    iss: u32,
    snd_una: u32,
    snd_nxt: u32,
    snd_max: u32,
    buf_seq: u32,
    snd_wnd: u32,
    snd_wl1: u32,
    snd_wl2: u32,
    snd_mss: u16,
    send_buf: VecDeque<u8>,
    fin_queued: bool,

    cwnd: u32,
    ssthresh: u32,
    dupacks: i32,
    in_recovery: bool,
    recover: u32,
    srtt: Duration,
    rttvar: Duration,
    rto: Duration,
    have_rtt: bool,
    rtt_active: bool,
    rtt_seq: u32,
    rtt_start: Clock,
    retries: i32,
    persist_interval: Duration,

    retransmit_deadline: Option<Clock>,
    persist_deadline: Option<Clock>,
    delayed_ack_deadline: Option<Clock>,
    time_wait_deadline: Option<Clock>,
    irs: u32,
    rcv_nxt: u32,
    rcv_offset: u64,
    recv_buf: VecDeque<u8>,
    out_of_order: BTreeMap<u64, Vec<u8>>,
    out_of_order_bytes: usize,
    peer_fin_seq: Option<u32>,
    fin_received: bool,
    unacked_segments: i32,
    last_advertised_window: u16,

    retransmissions: u32,
}

impl TcpConnection {
    /// `TcpConnection(...)`.
    pub fn new(
        config: TcpConfig,
        pseudo: PseudoHeader,
        local_port: u16,
        remote_port: u16,
        iss: u32,
        emit: impl FnMut(&[u8]) + 'static,
    ) -> Self {
        Self {
            rto: config.initial_rto,
            config,
            pseudo,
            local_port,
            remote_port,
            emit: Box::new(emit),
            state: TcpState::Closed,
            error: Error::Ok,
            iss,
            snd_una: iss,
            snd_nxt: iss,
            snd_max: iss,
            buf_seq: iss.wrapping_add(1),
            snd_wnd: 0,
            snd_wl1: 0,
            snd_wl2: 0,
            snd_mss: 536,
            send_buf: VecDeque::new(),
            fin_queued: false,
            cwnd: 0,
            ssthresh: 0x7fff_ffff,
            dupacks: 0,
            in_recovery: false,
            recover: 0,
            srtt: Duration::ZERO,
            rttvar: Duration::ZERO,
            have_rtt: false,
            rtt_active: false,
            rtt_seq: 0,
            rtt_start: Clock::now(),
            retries: 0,
            persist_interval: Duration::ZERO,
            retransmit_deadline: None,
            persist_deadline: None,
            delayed_ack_deadline: None,
            time_wait_deadline: None,
            irs: 0,
            rcv_nxt: 0,
            rcv_offset: 0,
            recv_buf: VecDeque::new(),
            out_of_order: BTreeMap::new(),
            out_of_order_bytes: 0,
            peer_fin_seq: None,
            fin_received: false,
            unacked_segments: 0,
            last_advertised_window: 0,
            retransmissions: 0,
        }
    }

    fn synchronized(&self) -> bool {
        !matches!(
            self.state,
            TcpState::Closed | TcpState::SynSent | TcpState::SynReceived
        )
    }

    fn receive_window(&self) -> u16 {
        let used = self.recv_buf.len();
        if used >= self.config.recv_buffer as usize {
            0
        } else {
            (self.config.recv_buffer as usize - used).min(65535) as u16
        }
    }

    fn fin_seq(&self) -> u32 {
        self.buf_seq.wrapping_add(self.send_buf.len() as u32)
    }

    fn fin_sent(&self) -> bool {
        self.fin_queued && seq_gt(self.snd_max, self.fin_seq())
    }

    fn fin_acked(&self) -> bool {
        self.fin_queued && seq_gt(self.snd_una, self.fin_seq())
    }

    /// `SendSpace`.
    pub fn send_space(&self) -> usize {
        if self.send_buf.len() >= self.config.send_buffer as usize {
            0
        } else {
            self.config.send_buffer as usize - self.send_buf.len()
        }
    }

    /// `Readable`.
    pub fn readable(&self) -> bool {
        !self.recv_buf.is_empty()
            || self.fin_received
            || self.error != Error::Ok
            || self.state == TcpState::Closed
    }

    /// `Writable`.
    pub fn writable(&self) -> bool {
        if self.error != Error::Ok || self.state == TcpState::Closed {
            return true;
        }
        matches!(self.state, TcpState::Established | TcpState::CloseWait)
            && !self.fin_queued
            && self.send_space() > 0
    }

    fn emit_segment(&mut self, seq: u32, flags: u8, payload: &[u8], with_mss: bool) {
        let window = self.receive_window();
        let ack = if flags & tcp_flag::ACK != 0 {
            self.rcv_nxt
        } else {
            0
        };
        let segment = build_tcp_segment(
            &self.pseudo,
            self.local_port,
            self.remote_port,
            seq,
            ack,
            flags,
            window,
            with_mss.then_some(self.config.mss),
            payload,
        );
        if flags & tcp_flag::ACK != 0 {
            self.last_advertised_window = window;
            self.unacked_segments = 0;
            self.delayed_ack_deadline = None;
        }
        (self.emit)(&segment);
    }

    fn send_ack(&mut self) {
        self.emit_segment(self.snd_max, tcp_flag::ACK, &[], false);
    }

    fn emit_reset(&mut self, seq: u32) {
        let segment = build_tcp_segment(
            &self.pseudo,
            self.local_port,
            self.remote_port,
            seq,
            0,
            tcp_flag::RST,
            0,
            None,
            &[],
        );
        (self.emit)(&segment);
    }

    fn send_data(&mut self, seq: u32, len: u32) {
        let offset = seq.wrapping_sub(self.buf_seq) as usize;
        let payload: Vec<u8> = self
            .send_buf
            .iter()
            .skip(offset)
            .take(len as usize)
            .copied()
            .collect();
        let last = seq.wrapping_add(len) == self.fin_seq();
        let flags = tcp_flag::ACK | if last { tcp_flag::PSH } else { 0 };
        self.emit_segment(seq, flags, &payload, false);
    }

    fn arm_retransmit(&mut self, now: Clock) {
        if self.retransmit_deadline.is_none() {
            self.retransmit_deadline = Some(now + self.rto);
        }
    }

    fn try_send(&mut self, now: Clock) {
        if !matches!(
            self.state,
            TcpState::Established
                | TcpState::CloseWait
                | TcpState::FinWait1
                | TcpState::Closing
                | TcpState::LastAck
        ) {
            return;
        }
        loop {
            if self.fin_sent() && self.snd_nxt == self.fin_seq().wrapping_add(1) {
                return;
            }
            let offset = self.snd_nxt.wrapping_sub(self.buf_seq);
            let available = if (offset as usize) < self.send_buf.len() {
                self.send_buf.len() as u32 - offset
            } else {
                0
            };
            let flight = self.snd_nxt.wrapping_sub(self.snd_una);
            let window = self.snd_wnd.min(self.cwnd);
            let usable = window.saturating_sub(flight);
            let len = available.min(usable).min(u32::from(self.snd_mss));

            if len == 0 {
                if available == 0 && self.fin_queued && self.snd_nxt == self.fin_seq() {
                    self.emit_segment(self.snd_nxt, tcp_flag::FIN | tcp_flag::ACK, &[], false);
                    self.snd_nxt = self.snd_nxt.wrapping_add(1);
                    if seq_gt(self.snd_nxt, self.snd_max) {
                        self.snd_max = self.snd_nxt;
                    }
                    self.arm_retransmit(now);
                } else if available > 0
                    && self.snd_wnd == 0
                    && self.snd_max == self.snd_una
                    && self.persist_deadline.is_none()
                {
                    self.persist_interval = self.rto;
                    self.persist_deadline = Some(now + self.persist_interval);
                }
                return;
            }

            self.send_data(self.snd_nxt, len);
            if !self.rtt_active && self.snd_nxt == self.snd_max {
                self.rtt_active = true;
                self.rtt_seq = self.snd_nxt;
                self.rtt_start = now;
            }
            self.snd_nxt = self.snd_nxt.wrapping_add(len);
            if seq_gt(self.snd_nxt, self.snd_max) {
                self.snd_max = self.snd_nxt;
            }
            self.arm_retransmit(now);
        }
    }

    fn retransmit_first(&mut self) {
        self.retransmissions += 1;
        self.rtt_active = false;
        let offset = self.snd_una.wrapping_sub(self.buf_seq);
        if (offset as usize) < self.send_buf.len() {
            let len = (self.send_buf.len() as u32 - offset).min(u32::from(self.snd_mss));
            self.send_data(self.snd_una, len);
        } else if self.fin_sent() {
            self.emit_segment(self.fin_seq(), tcp_flag::FIN | tcp_flag::ACK, &[], false);
        }
    }

    /// `Connect`.
    pub fn connect(&mut self, now: Clock) {
        self.state = TcpState::SynSent;
        self.snd_nxt = self.iss.wrapping_add(1);
        self.snd_max = self.snd_nxt;
        self.emit_segment(self.iss, tcp_flag::SYN, &[], true);
        self.rtt_active = true;
        self.rtt_seq = self.iss;
        self.rtt_start = now;
        self.arm_retransmit(now);
    }

    /// `AcceptSyn`.
    pub fn accept_syn(&mut self, syn: &TcpSegment<'_>, now: Clock) {
        self.irs = syn.seq;
        self.rcv_nxt = self.irs.wrapping_add(1);
        self.snd_mss = syn.mss.unwrap_or(536).min(self.config.mss);
        self.cwnd = (4 * u32::from(self.snd_mss)).min((2 * u32::from(self.snd_mss)).max(4380));
        self.snd_wnd = u32::from(syn.window);
        self.snd_wl1 = syn.seq;
        self.snd_wl2 = self.iss;
        self.state = TcpState::SynReceived;
        self.snd_nxt = self.iss.wrapping_add(1);
        self.snd_max = self.snd_nxt;
        self.emit_segment(self.iss, tcp_flag::SYN | tcp_flag::ACK, &[], true);
        self.rtt_active = true;
        self.rtt_seq = self.iss;
        self.rtt_start = now;
        self.arm_retransmit(now);
    }

    fn handle_syn_sent(&mut self, segment: &TcpSegment<'_>, now: Clock) {
        let ack_ok = seq_gt(segment.ack, self.iss) && seq_le(segment.ack, self.snd_max);
        if segment.flags & tcp_flag::ACK != 0 && !ack_ok {
            if segment.flags & tcp_flag::RST == 0 {
                self.emit_reset(segment.ack);
            }
            return;
        }
        if segment.flags & tcp_flag::RST != 0 {
            if segment.flags & tcp_flag::ACK != 0 {
                self.terminate(Error::ConnRefused);
            }
            return;
        }
        if segment.flags & tcp_flag::SYN == 0 {
            return;
        }
        self.irs = segment.seq;
        self.rcv_nxt = self.irs.wrapping_add(1);
        self.snd_mss = segment.mss.unwrap_or(536).min(self.config.mss);
        self.cwnd = (4 * u32::from(self.snd_mss)).min((2 * u32::from(self.snd_mss)).max(4380));
        self.snd_wnd = u32::from(segment.window);
        self.snd_wl1 = segment.seq;
        self.snd_wl2 = segment.ack;
        if segment.flags & tcp_flag::ACK != 0 {
            self.snd_una = segment.ack;
            if self.rtt_active && seq_gt(segment.ack, self.rtt_seq) {
                self.sample_rtt(now.saturating_duration_since(self.rtt_start));
                self.rtt_active = false;
            }
            self.retries = 0;
            self.retransmit_deadline = None;
            self.state = TcpState::Established;
            self.send_ack();
            self.try_send(now);
        } else {
            self.state = TcpState::SynReceived;
            self.emit_segment(self.iss, tcp_flag::SYN | tcp_flag::ACK, &[], true);
        }
    }

    fn acceptable(&self, seq: u32, len: u32) -> bool {
        let window = self.receive_window();
        let in_window = |value: u32| {
            seq_le(self.rcv_nxt, value)
                && seq_lt(value, self.rcv_nxt.wrapping_add(u32::from(window)))
        };
        if len == 0 {
            return if window == 0 {
                seq == self.rcv_nxt
            } else {
                in_window(seq)
            };
        }
        if window == 0 {
            return false;
        }
        in_window(seq) || in_window(seq.wrapping_add(len).wrapping_sub(1))
    }

    /// `OnSegment`.
    pub fn on_segment(&mut self, segment: &TcpSegment<'_>, now: Clock) {
        match self.state {
            TcpState::Closed => return,
            TcpState::SynSent => {
                self.handle_syn_sent(segment, now);
                return;
            }
            TcpState::TimeWait => {
                if segment.flags & tcp_flag::FIN != 0 && segment.flags & tcp_flag::RST == 0 {
                    self.send_ack();
                    self.time_wait_deadline = Some(now + self.config.time_wait);
                }
                return;
            }
            _ => {}
        }

        if self.state == TcpState::SynReceived
            && segment.flags & (tcp_flag::SYN | tcp_flag::ACK | tcp_flag::RST) == tcp_flag::SYN
            && segment.seq == self.irs
        {
            self.retransmissions += 1;
            self.emit_segment(self.iss, tcp_flag::SYN | tcp_flag::ACK, &[], true);
            return;
        }

        let length = segment.payload.len() as u32
            + u32::from(segment.flags & tcp_flag::SYN != 0)
            + u32::from(segment.flags & tcp_flag::FIN != 0);
        let mut ack_only = false;
        if !self.acceptable(segment.seq, length) {
            if self.receive_window() == 0
                && segment.seq == self.rcv_nxt
                && segment.flags & tcp_flag::ACK != 0
                && segment.flags & (tcp_flag::RST | tcp_flag::SYN) == 0
            {
                ack_only = true;
            } else {
                if segment.flags & tcp_flag::RST == 0 {
                    self.send_ack();
                }
                return;
            }
        }

        if segment.flags & tcp_flag::RST != 0 {
            if segment.seq == self.rcv_nxt {
                let closing = matches!(self.state, TcpState::Closing | TcpState::LastAck);
                let error = if closing {
                    Error::Ok
                } else if self.state == TcpState::SynReceived {
                    Error::ConnRefused
                } else {
                    Error::ConnReset
                };
                self.terminate(error);
            } else {
                self.send_ack();
            }
            return;
        }
        if segment.flags & tcp_flag::SYN != 0 {
            self.send_ack();
            return;
        }
        if segment.flags & tcp_flag::ACK == 0 {
            return;
        }

        if self.state == TcpState::SynReceived {
            if !(seq_gt(segment.ack, self.snd_una) && seq_le(segment.ack, self.snd_max)) {
                self.emit_reset(segment.ack);
                return;
            }
            self.state = TcpState::Established;
            self.snd_wnd = u32::from(segment.window);
            self.snd_wl1 = segment.seq;
            self.snd_wl2 = segment.ack;
        }

        if !self.process_ack(segment, now) {
            return;
        }
        if ack_only {
            self.send_ack();
            return;
        }
        self.process_data(segment, now);
        self.try_send(now);
    }

    fn process_ack(&mut self, segment: &TcpSegment<'_>, now: Clock) -> bool {
        if seq_gt(segment.ack, self.snd_max) {
            self.send_ack();
            return false;
        }
        if seq_gt(segment.ack, self.snd_una) {
            self.on_new_ack(segment.ack, now);
        } else if segment.ack == self.snd_una
            && segment.payload.is_empty()
            && segment.flags & (tcp_flag::SYN | tcp_flag::FIN) == 0
            && u32::from(segment.window) == self.snd_wnd
            && self.snd_max != self.snd_una
        {
            self.on_duplicate_ack();
        }

        if seq_lt(self.snd_wl1, segment.seq)
            || (self.snd_wl1 == segment.seq && seq_le(self.snd_wl2, segment.ack))
        {
            self.snd_wnd = u32::from(segment.window);
            self.snd_wl1 = segment.seq;
            self.snd_wl2 = segment.ack;
            if self.snd_wnd > 0 {
                self.persist_deadline = None;
            }
        }

        if self.fin_acked() {
            match self.state {
                TcpState::FinWait1 => self.state = TcpState::FinWait2,
                TcpState::Closing => {
                    self.enter_time_wait(now);
                    return false;
                }
                TcpState::LastAck => {
                    self.state = TcpState::Closed;
                    self.clear_timers();
                    return false;
                }
                _ => {}
            }
        }
        true
    }

    fn on_new_ack(&mut self, ack: u32, now: Clock) {
        let acked = ack.wrapping_sub(self.snd_una);
        let data_acked = if seq_gt(ack, self.buf_seq) {
            ack.wrapping_sub(self.buf_seq)
                .min(self.send_buf.len() as u32)
        } else {
            0
        };
        self.send_buf.drain(..data_acked as usize);
        self.buf_seq = self.buf_seq.wrapping_add(data_acked);
        self.snd_una = ack;
        if seq_lt(self.snd_nxt, self.snd_una) {
            self.snd_nxt = self.snd_una;
        }

        if self.rtt_active && seq_gt(ack, self.rtt_seq) {
            self.sample_rtt(now.saturating_duration_since(self.rtt_start));
            self.rtt_active = false;
        }
        self.retries = 0;

        if self.in_recovery {
            if seq_ge(ack, self.recover) {
                self.in_recovery = false;
                self.cwnd = self.ssthresh;
            } else {
                self.retransmit_first();
                self.cwnd = if self.cwnd > acked {
                    self.cwnd - acked + u32::from(self.snd_mss)
                } else {
                    u32::from(self.snd_mss)
                };
            }
        } else if self.cwnd < self.ssthresh {
            self.cwnd += acked.min(u32::from(self.snd_mss));
        } else {
            self.cwnd +=
                1.max(u32::from(self.snd_mss) * u32::from(self.snd_mss) / self.cwnd.max(1));
        }
        self.cwnd = self.cwnd.min(1 << 30);
        self.dupacks = 0;

        self.retransmit_deadline = None;
        if self.snd_una != self.snd_max {
            self.retransmit_deadline = Some(now + self.rto);
        }
    }

    fn on_duplicate_ack(&mut self) {
        if self.in_recovery {
            self.cwnd += u32::from(self.snd_mss);
            return;
        }
        self.dupacks += 1;
        if self.dupacks == 3 {
            let flight = self.snd_max.wrapping_sub(self.snd_una);
            self.ssthresh = (flight / 2).max(2 * u32::from(self.snd_mss));
            self.recover = self.snd_max;
            self.in_recovery = true;
            self.retransmit_first();
            self.cwnd = self.ssthresh + 3 * u32::from(self.snd_mss);
        }
    }

    fn process_data(&mut self, segment: &TcpSegment<'_>, now: Clock) {
        if !matches!(
            self.state,
            TcpState::Established | TcpState::FinWait1 | TcpState::FinWait2
        ) {
            if segment.flags & tcp_flag::FIN != 0 {
                self.send_ack();
            }
            return;
        }

        let mut seq = segment.seq;
        let mut data = segment.payload;
        if segment.flags & tcp_flag::FIN != 0 {
            self.peer_fin_seq = Some(segment.seq.wrapping_add(segment.payload.len() as u32));
        }
        if seq_lt(seq, self.rcv_nxt) {
            let skip = self.rcv_nxt - seq;
            data = if skip as usize >= data.len() {
                &[]
            } else {
                &data[skip as usize..]
            };
            seq = self.rcv_nxt;
        }
        let window = u32::from(self.receive_window());
        let room = if seq.wrapping_sub(self.rcv_nxt) < window {
            self.rcv_nxt.wrapping_add(window).wrapping_sub(seq)
        } else {
            0
        };
        if data.len() as u32 > room {
            data = &data[..room as usize];
        }

        let mut ack_now = false;
        if !data.is_empty() {
            if seq == self.rcv_nxt {
                self.recv_buf.extend(data.iter().copied());
                self.rcv_nxt = self.rcv_nxt.wrapping_add(data.len() as u32);
                self.rcv_offset += data.len() as u64;
                if !self.out_of_order.is_empty() {
                    self.pull_out_of_order();
                    ack_now = true;
                }
                self.unacked_segments += 1;
            } else {
                let offset = self.rcv_offset + u64::from(seq - self.rcv_nxt);
                if self.out_of_order_bytes + data.len() <= self.config.recv_buffer as usize {
                    let slot = self.out_of_order.entry(offset).or_default();
                    if slot.len() < data.len() {
                        self.out_of_order_bytes += data.len() - slot.len();
                        *slot = data.to_vec();
                    }
                }
                ack_now = true;
            }
        }

        if self.peer_fin_seq == Some(self.rcv_nxt) && !self.fin_received {
            self.rcv_nxt = self.rcv_nxt.wrapping_add(1);
            self.fin_received = true;
            ack_now = true;
            match self.state {
                TcpState::Established => self.state = TcpState::CloseWait,
                TcpState::FinWait1 => {
                    if self.fin_acked() {
                        self.enter_time_wait(now);
                    } else {
                        self.state = TcpState::Closing;
                    }
                }
                TcpState::FinWait2 => self.enter_time_wait(now),
                _ => {}
            }
        }

        if ack_now || self.unacked_segments >= 2 {
            self.send_ack();
        } else if self.unacked_segments > 0 && self.delayed_ack_deadline.is_none() {
            self.delayed_ack_deadline = Some(now + self.config.delayed_ack);
        }
    }

    fn pull_out_of_order(&mut self) {
        while let Some((&offset, _)) = self.out_of_order.iter().next() {
            if offset > self.rcv_offset {
                return;
            }
            let entry = self.out_of_order.remove(&offset).expect("the entry exists");
            let end = offset + entry.len() as u64;
            if end > self.rcv_offset {
                let skip = (self.rcv_offset - offset) as usize;
                self.recv_buf.extend(entry[skip..].iter().copied());
                let added = (end - self.rcv_offset) as u32;
                self.rcv_nxt = self.rcv_nxt.wrapping_add(added);
                self.rcv_offset += u64::from(added);
            }
            self.out_of_order_bytes -= entry.len();
        }
    }

    /// `NextDeadline`.
    pub fn next_deadline(&self) -> Option<Clock> {
        let mut next = None;
        for deadline in [
            self.retransmit_deadline,
            self.persist_deadline,
            self.delayed_ack_deadline,
            self.time_wait_deadline,
        ]
        .into_iter()
        .flatten()
        {
            if next.is_none_or(|current| deadline < current) {
                next = Some(deadline);
            }
        }
        next
    }

    /// `OnTimer`.
    pub fn on_timer(&mut self, now: Clock) {
        if self
            .time_wait_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.state = TcpState::Closed;
            self.clear_timers();
            return;
        }
        if self
            .delayed_ack_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.send_ack();
        }
        if self
            .retransmit_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.retransmit_deadline = None;
            self.on_retransmit_timeout(now);
        }
        if self
            .persist_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.emit_segment(self.snd_una.wrapping_sub(1), tcp_flag::ACK, &[], false);
            self.persist_interval = (self.persist_interval * 2).min(self.config.max_rto);
            self.persist_deadline = Some(now + self.persist_interval);
        }
    }

    fn on_retransmit_timeout(&mut self, now: Clock) {
        let handshake = matches!(self.state, TcpState::SynSent | TcpState::SynReceived);
        self.retries += 1;
        if self.retries
            > if handshake {
                self.config.syn_retries
            } else {
                self.config.data_retries
            }
        {
            if self.synchronized() {
                self.emit_reset(self.snd_max);
            }
            self.terminate(Error::TimedOut);
            return;
        }
        self.retransmissions += 1;
        self.rtt_active = false;
        self.rto = (self.rto * 2).min(self.config.max_rto);

        if self.state == TcpState::SynSent {
            self.emit_segment(self.iss, tcp_flag::SYN, &[], true);
        } else if self.state == TcpState::SynReceived {
            self.emit_segment(self.iss, tcp_flag::SYN | tcp_flag::ACK, &[], true);
        } else {
            let flight = self.snd_max.wrapping_sub(self.snd_una);
            self.ssthresh = (flight / 2).max(2 * u32::from(self.snd_mss));
            self.cwnd = u32::from(self.snd_mss);
            self.in_recovery = false;
            self.dupacks = 0;
            self.snd_nxt = self.snd_una;
            self.try_send(now);
        }
        self.retransmit_deadline = Some(now + self.rto);
    }

    fn sample_rtt(&mut self, rtt: Duration) {
        if !self.have_rtt {
            self.srtt = rtt;
            self.rttvar = rtt / 2;
            self.have_rtt = true;
        } else {
            let delta = self.srtt.abs_diff(rtt);
            self.rttvar = (self.rttvar * 3 + delta) / 4;
            self.srtt = (self.srtt * 7 + rtt) / 8;
        }
        let variance = (self.rttvar * 4).max(Duration::from_millis(10));
        self.rto = (self.srtt + variance).clamp(self.config.min_rto, self.config.max_rto);
    }

    fn enter_time_wait(&mut self, now: Clock) {
        self.clear_timers();
        self.state = TcpState::TimeWait;
        self.time_wait_deadline = Some(now + self.config.time_wait);
    }

    fn terminate(&mut self, error: Error) {
        self.error = error;
        self.state = TcpState::Closed;
        self.clear_timers();
        self.send_buf.clear();
        self.out_of_order.clear();
        self.out_of_order_bytes = 0;
    }

    fn clear_timers(&mut self) {
        self.retransmit_deadline = None;
        self.persist_deadline = None;
        self.delayed_ack_deadline = None;
        self.time_wait_deadline = None;
    }

    /// `Send`.
    pub fn send(&mut self, data: &[u8], now: Clock) -> IoResult {
        if self.error != Error::Ok {
            return IoResult {
                bytes: 0,
                error: self.error,
            };
        }
        match self.state {
            TcpState::SynSent | TcpState::SynReceived => {
                return IoResult {
                    bytes: 0,
                    error: Error::WouldBlock,
                }
            }
            TcpState::Established | TcpState::CloseWait => {}
            _ => {
                return IoResult {
                    bytes: 0,
                    error: if self.fin_queued {
                        Error::Pipe
                    } else {
                        Error::NotConn
                    },
                }
            }
        }
        if self.fin_queued {
            return IoResult {
                bytes: 0,
                error: Error::Pipe,
            };
        }
        let count = self.send_space().min(data.len());
        if count == 0 {
            return IoResult {
                bytes: 0,
                error: if data.is_empty() {
                    Error::Ok
                } else {
                    Error::WouldBlock
                },
            };
        }
        self.send_buf.extend(data[..count].iter().copied());
        self.try_send(now);
        IoResult {
            bytes: count,
            error: Error::Ok,
        }
    }

    /// `Recv`.
    pub fn recv(&mut self, out: &mut [u8], peek: bool) -> IoResult {
        if !self.recv_buf.is_empty() {
            let count = out.len().min(self.recv_buf.len());
            for (index, byte) in self.recv_buf.iter().take(count).enumerate() {
                out[index] = *byte;
            }
            if !peek {
                self.recv_buf.drain(..count);
                self.maybe_send_window_update();
            }
            return IoResult {
                bytes: count,
                error: Error::Ok,
            };
        }
        if self.fin_received {
            return IoResult {
                bytes: 0,
                error: Error::Ok,
            };
        }
        if self.error != Error::Ok {
            return IoResult {
                bytes: 0,
                error: self.error,
            };
        }
        if self.state == TcpState::Closed {
            return IoResult {
                bytes: 0,
                error: Error::NotConn,
            };
        }
        IoResult {
            bytes: 0,
            error: Error::WouldBlock,
        }
    }

    fn maybe_send_window_update(&mut self) {
        if !self.synchronized() || self.fin_received {
            return;
        }
        let threshold = (self.config.recv_buffer / 2).min(2 * u32::from(self.snd_mss));
        if i32::from(self.receive_window()) - i32::from(self.last_advertised_window)
            >= threshold as i32
        {
            self.send_ack();
        }
    }

    /// `Shutdown`.
    pub fn shutdown(&mut self, now: Clock) {
        if self.fin_queued {
            return;
        }
        match self.state {
            TcpState::SynSent => {
                self.terminate(Error::Ok);
                return;
            }
            TcpState::SynReceived => {
                self.abort();
                return;
            }
            TcpState::Established => self.state = TcpState::FinWait1,
            TcpState::CloseWait => self.state = TcpState::LastAck,
            _ => return,
        }
        self.fin_queued = true;
        self.try_send(now);
    }

    /// `Close`.
    pub fn close(&mut self, now: Clock) {
        if self.state == TcpState::SynSent || (self.synchronized() && !self.recv_buf.is_empty()) {
            self.abort();
            return;
        }
        self.shutdown(now);
    }

    /// `Abort`.
    pub fn abort(&mut self) {
        if !matches!(
            self.state,
            TcpState::Closed | TcpState::SynSent | TcpState::TimeWait
        ) {
            self.emit_reset(self.snd_max);
        }
        self.terminate(Error::Ok);
    }

    /// `State`.
    pub fn state(&self) -> TcpState {
        self.state
    }

    /// `PendingError`.
    pub fn pending_error(&self) -> Error {
        self.error
    }

    /// `ReceivedFin`.
    pub fn received_fin(&self) -> bool {
        self.fin_received
    }

    /// `BytesToRead`.
    pub fn bytes_to_read(&self) -> usize {
        self.recv_buf.len()
    }

    /// `LocalPort`.
    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    /// `RemotePort`.
    pub fn remote_port(&self) -> u16 {
        self.remote_port
    }

    /// `Retransmissions`.
    pub fn retransmissions(&self) -> u32 {
        self.retransmissions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    type Segments = Arc<Mutex<Vec<Vec<u8>>>>;

    fn pseudo() -> PseudoHeader {
        let mut header = PseudoHeader::default();
        header.local[..4].copy_from_slice(&[127, 0, 0, 1]);
        header.remote[..4].copy_from_slice(&[127, 0, 0, 2]);
        header
    }

    fn collector() -> (Segments, impl FnMut(&[u8]) + 'static) {
        let segments = Arc::new(Mutex::new(Vec::new()));
        let sink = segments.clone();
        (segments, move |segment: &[u8]| {
            sink.lock().unwrap().push(segment.to_vec());
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn built(
        header: &PseudoHeader,
        src: u16,
        dst: u16,
        seq: u32,
        ack: u32,
        flags: u8,
        window: u16,
        mss: Option<u16>,
        payload: &[u8],
    ) -> TcpSegment<'static> {
        let bytes = build_tcp_segment(header, src, dst, seq, ack, flags, window, mss, payload);
        let parsed = parse_tcp_segment(&bytes, None).expect("a segment");
        TcpSegment {
            payload: Box::leak(parsed.payload.to_vec().into_boxed_slice()),
            ..parsed
        }
    }

    fn last_segment(
        segments: &Arc<Mutex<Vec<Vec<u8>>>>,
        header: &PseudoHeader,
    ) -> TcpSegment<'static> {
        let bytes = segments.lock().unwrap().last().unwrap().clone();
        let parsed = parse_tcp_segment(&bytes, Some(header)).expect("a valid segment");
        TcpSegment {
            payload: Box::leak(parsed.payload.to_vec().into_boxed_slice()),
            ..parsed
        }
    }

    #[test]
    fn segments_round_trip_with_a_valid_checksum() {
        let header = pseudo();
        let bytes = build_tcp_segment(
            &header,
            1234,
            3658,
            10,
            20,
            tcp_flag::SYN | tcp_flag::ACK,
            4096,
            Some(1200),
            b"payload",
        );
        let segment = parse_tcp_segment(&bytes, Some(&header)).expect("valid");
        assert_eq!(segment.src_port, 1234);
        assert_eq!(segment.dst_port, 3658);
        assert_eq!(segment.seq, 10);
        assert_eq!(segment.ack, 20);
        assert_eq!(segment.flags, tcp_flag::SYN | tcp_flag::ACK);
        assert_eq!(segment.window, 4096);
        assert_eq!(segment.mss, Some(1200));
        assert_eq!(segment.payload, b"payload");

        let mut corrupted = bytes.clone();
        corrupted[13] ^= 1;
        assert!(parse_tcp_segment(&corrupted, Some(&header)).is_none());
        assert!(parse_tcp_segment(&[0u8; 10], None).is_none());
    }

    #[test]
    fn a_reset_answers_an_unexpected_segment() {
        let header = pseudo();
        let segment = built(&header, 1, 2, 7, 0, tcp_flag::SYN, 0, None, b"x");
        let reset = build_reset_for(&header, &segment).unwrap();
        let reset = parse_tcp_segment(&reset, Some(&header)).unwrap();
        assert_eq!(reset.flags, tcp_flag::RST | tcp_flag::ACK);
        assert_eq!(reset.seq, 0);
        assert_eq!(reset.ack, 9);

        let acking = built(&header, 1, 2, 7, 9, tcp_flag::ACK, 0, None, &[]);
        let reset = build_reset_for(&header, &acking).unwrap();
        let reset = parse_tcp_segment(&reset, Some(&header)).unwrap();
        assert_eq!(reset.seq, 9);
        assert_eq!(reset.flags, tcp_flag::RST);
        assert!(build_reset_for(&header, &reset).is_none());
    }

    fn established() -> (TcpConnection, Segments, PseudoHeader) {
        let header = pseudo();
        let (segments, emit) = collector();
        let mut connection =
            TcpConnection::new(TcpConfig::default(), header.clone(), 4000, 5000, 100, emit);
        let now = Clock::now();
        connection.connect(now);
        let syn = last_segment(&segments, &header);
        assert_eq!(syn.flags, tcp_flag::SYN);
        let syn_ack = built(
            &header,
            5000,
            4000,
            1000,
            101,
            tcp_flag::SYN | tcp_flag::ACK,
            65535,
            Some(1200),
            &[],
        );
        connection.on_segment(&syn_ack, now);
        assert_eq!(connection.state(), TcpState::Established);
        (connection, segments, header)
    }

    #[test]
    fn the_handshake_reaches_established_and_sends_an_ack() {
        let (connection, segments, header) = established();
        let ack = last_segment(&segments, &header);
        assert_eq!(ack.flags, tcp_flag::ACK);
        assert_eq!(ack.ack, 1001);
        assert!(connection.writable());
        assert!(!connection.readable());
    }

    #[test]
    fn sending_data_emits_segments_and_acks_release_the_buffer() {
        let (mut connection, segments, header) = established();
        let now = Clock::now();
        let result = connection.send(b"hello", now);
        assert_eq!(result.bytes, 5);
        let data = last_segment(&segments, &header);
        assert_eq!(data.payload, b"hello");
        assert_eq!(data.flags & tcp_flag::ACK, tcp_flag::ACK);

        let ack = built(
            &header,
            5000,
            4000,
            1001,
            data.seq.wrapping_add(5),
            tcp_flag::ACK,
            65535,
            None,
            &[],
        );
        connection.on_segment(&ack, now);
        assert_eq!(
            connection.send_space(),
            TcpConfig::default().send_buffer as usize
        );
    }

    #[test]
    fn received_data_is_readable_and_peek_does_not_consume() {
        let (mut connection, _segments, header) = established();
        let now = Clock::now();
        let data = built(
            &header,
            5000,
            4000,
            1001,
            101,
            tcp_flag::ACK | tcp_flag::PSH,
            65535,
            None,
            b"world",
        );
        connection.on_segment(&data, now);
        assert!(connection.readable());
        assert_eq!(connection.bytes_to_read(), 5);
        let mut buffer = [0u8; 5];
        assert_eq!(connection.recv(&mut buffer, true).bytes, 5);
        assert_eq!(connection.bytes_to_read(), 5);
        assert_eq!(connection.recv(&mut buffer, false).bytes, 5);
        assert_eq!(&buffer, b"world");
        assert_eq!(connection.bytes_to_read(), 0);
    }

    #[test]
    fn shutdown_enters_fin_wait_and_peer_fin_enters_time_wait() {
        let (mut connection, segments, header) = established();
        let now = Clock::now();
        connection.shutdown(now);
        assert_eq!(connection.state(), TcpState::FinWait1);
        let fin = last_segment(&segments, &header);
        assert_eq!(fin.flags, tcp_flag::FIN | tcp_flag::ACK);

        let fin_ack = built(
            &header,
            5000,
            4000,
            1001,
            fin.seq.wrapping_add(1),
            tcp_flag::ACK,
            65535,
            None,
            &[],
        );
        connection.on_segment(&fin_ack, now);
        assert_eq!(connection.state(), TcpState::FinWait2);

        let peer_fin = built(
            &header,
            5000,
            4000,
            1001,
            101,
            tcp_flag::FIN | tcp_flag::ACK,
            65535,
            None,
            &[],
        );
        connection.on_segment(&peer_fin, now);
        assert_eq!(connection.state(), TcpState::TimeWait);
        assert!(connection.received_fin());
        assert!(connection.next_deadline().is_some());
    }

    #[test]
    fn a_reset_in_established_sets_the_peer_error() {
        let (mut connection, _segments, header) = established();
        let now = Clock::now();
        let reset = built(&header, 5000, 4000, 1001, 0, tcp_flag::RST, 0, None, &[]);
        connection.on_segment(&reset, now);
        assert_eq!(connection.state(), TcpState::Closed);
        assert_eq!(connection.pending_error(), Error::ConnReset);
    }
}
