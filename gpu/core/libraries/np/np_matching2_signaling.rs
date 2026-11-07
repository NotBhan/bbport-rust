//! Matching2 signaling glue, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_matching2/np_matching2_signaling.h`.

use crate::gpu::core::libraries::np::np_types2::*;

/// `StartMatching2SignalingRuntime` / `StopMatching2SignalingRuntime`.
pub trait Matching2SignalingRuntime {
    fn start(&mut self);
    fn stop(&mut self);
}

/// The signaling interactions of the matching2 context.
pub trait Matching2SignalingApi {
    fn send_stun_ping(&mut self, ctx_id: OrbisNpMatching2ContextId) -> bool;
    fn queue_dead_for_room_peers(
        &mut self,
        ctx_id: OrbisNpMatching2ContextId,
        room_id: OrbisNpMatching2RoomId,
        error_code: i32,
    );
    fn queue_signaling_event(
        &mut self,
        ctx_id: OrbisNpMatching2ContextId,
        room_id: OrbisNpMatching2RoomId,
        member_id: OrbisNpMatching2RoomMemberId,
        event: OrbisNpMatching2Event,
        error_code: i32,
    );
    fn room_ping_us(
        &self,
        ctx_id: OrbisNpMatching2ContextId,
        room_id: OrbisNpMatching2RoomId,
    ) -> u32;
    fn fill_connection_info(
        &self,
        ctx_id: OrbisNpMatching2ContextId,
        room_id: OrbisNpMatching2RoomId,
        member_id: OrbisNpMatching2RoomMemberId,
        info_type: u32,
    ) -> Option<[u8; 16]>;
    fn fill_local_net_info(&self) -> Option<OrbisNpSignalingNetInfo>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signaling_events_follow_the_guest_enum() {
        assert_eq!(OrbisNpMatching2Event::SignalingEvent as u16, 30);
        assert_eq!(OrbisNpMatching2Event::RequestError as u16, 3);
    }
}

use crate::gpu::core::libraries::np::np_matching2_cache::Matching2ContextCache;

pub const MATCHING2_SIGNALING_EVENT_DEAD: u16 = 0x5101;
pub const MATCHING2_SIGNALING_EVENT_ESTABLISHED: u16 = 0x5102;

/// `Matching2StunPing`.
#[repr(C, packed(1))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Matching2StunPing {
    pub cmd: u8,
    pub online_id: [u8; 16],
    pub local_ip: u32,
}

/// `SendMatching2StunPing`.
pub fn send_stun_ping(
    enabled: bool,
    started: bool,
    online_id: &[u8; 16],
    advertised_addr: u32,
    server_addr: u32,
    server_port: u16,
    send: &mut dyn FnMut(&[u8], u32, u16) -> i32,
) -> bool {
    if !enabled || !started || online_id[0] == 0 || server_addr == 0 || server_port == 0 {
        return false;
    }
    let ping = Matching2StunPing {
        cmd: 0x01,
        online_id: *online_id,
        local_ip: advertised_addr,
    };
    let bytes = unsafe {
        core::slice::from_raw_parts(
            &ping as *const Matching2StunPing as *const u8,
            core::mem::size_of::<Matching2StunPing>(),
        )
    };
    send(bytes, server_addr, server_port) >= 0
}

/// `QueueMatching2SignalingEvent`; the callback sink receives the queued event.
pub trait SignalingEventSink {
    fn queue_signaling_event(
        &mut self,
        ctx_id: OrbisNpMatching2ContextId,
        room_id: OrbisNpMatching2RoomId,
        member_id: OrbisNpMatching2RoomMemberId,
        event: u16,
        error_code: i32,
    );
}

pub fn queue_signaling_event(
    sink: &mut dyn SignalingEventSink,
    ctx_id: OrbisNpMatching2ContextId,
    room_id: OrbisNpMatching2RoomId,
    member_id: OrbisNpMatching2RoomMemberId,
    event: u16,
    error_code: i32,
) {
    sink.queue_signaling_event(ctx_id, room_id, member_id, event, error_code);
}

/// `QueueMatching2DeadForRoomPeers`; marks every other room peer inactive and raises DEAD.
pub fn queue_dead_for_room_peers(
    cache: &mut Matching2ContextCache,
    sink: &mut dyn SignalingEventSink,
    ctx_id: OrbisNpMatching2ContextId,
    room_id: OrbisNpMatching2RoomId,
    my_member_id: OrbisNpMatching2RoomMemberId,
    error_code: i32,
) -> usize {
    let Some(room) = cache.rooms.get(&room_id) else {
        return 0;
    };
    let dead: Vec<_> = room
        .members
        .keys()
        .copied()
        .filter(|member_id| *member_id != 0 && *member_id != my_member_id)
        .collect();
    for member_id in &dead {
        if let Some(peer) = cache.peers.get_mut(member_id) {
            peer.status = 0;
        }
        queue_signaling_event(
            sink,
            ctx_id,
            room_id,
            *member_id,
            MATCHING2_SIGNALING_EVENT_DEAD,
            error_code,
        );
    }
    dead.len()
}

/// `GetRoomPingUs`; average of the room peers' known ping.
pub fn get_room_ping_us(
    cache: &Matching2ContextCache,
    room_id: OrbisNpMatching2RoomId,
    my_member_id: OrbisNpMatching2RoomMemberId,
) -> u32 {
    let Some(room) = cache.rooms.get(&room_id) else {
        return 0;
    };
    let mut total = 0u64;
    let mut count = 0u32;
    for member_id in room.members.keys() {
        if *member_id == my_member_id {
            continue;
        }
        if let Some(peer) = cache.peers.get(member_id) {
            if peer.ping_us != 0 {
                total += u64::from(peer.ping_us);
                count += 1;
            }
        }
    }
    if count == 0 {
        0
    } else {
        (total / u64::from(count)) as u32
    }
}

/// `OrbisNpMatching2SignalingGetPingInfoResponse`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SignalingPingInfoResponse {
    pub server_id: OrbisNpMatching2ServerId,
    pub world_id: OrbisNpMatching2WorldId,
    pub room_id: OrbisNpMatching2RoomId,
    pub rtt: u32,
}

/// `BuildSignalingGetPingInfoPayload`.
pub fn build_ping_info_payload(
    cache: &Matching2ContextCache,
    ctx_id: OrbisNpMatching2ContextId,
    room_id: OrbisNpMatching2RoomId,
    my_member_id: OrbisNpMatching2RoomMemberId,
    fallback_server: OrbisNpMatching2ServerId,
    fallback_world: OrbisNpMatching2WorldId,
) -> SignalingPingInfoResponse {
    let _ = ctx_id;
    let mut out = SignalingPingInfoResponse {
        server_id: fallback_server,
        world_id: fallback_world,
        room_id,
        rtt: get_room_ping_us(cache, room_id, my_member_id),
    };
    if let Some(room) = cache.rooms.get(&room_id) {
        out.server_id = room.server_id;
        out.world_id = room.world_id;
        out.room_id = room.room_id;
    }
    out
}

/// `FillMatching2LocalNetInfo`; the guest struct reports our addresses and NAT type.
pub fn fill_local_net_info(
    local_addr: u32,
    mapped_addr: u32,
    nat_status: i32,
) -> OrbisNpSignalingNetInfo {
    OrbisNpSignalingNetInfo {
        size: core::mem::size_of::<OrbisNpSignalingNetInfo>() as u64,
        local_addr,
        mapped_addr,
        nat_status,
        pad: 0,
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;
    use crate::gpu::core::libraries::np::np_matching2_cache::{MemberCache, PeerInfo, RoomCache};

    #[derive(Default)]
    struct Sink(Vec<(u16, u16, u16, i32)>);

    impl SignalingEventSink for Sink {
        fn queue_signaling_event(&mut self, _: u16, _: u64, member: u16, event: u16, error: i32) {
            self.0.push((0, 0, member, event as i32));
            let _ = error;
        }
    }

    #[test]
    fn signaling_ping_and_dead_events_follow_the_cpp() {
        assert_eq!(core::mem::size_of::<Matching2StunPing>(), 21);
        let mut online = [0u8; 16];
        online[..6].copy_from_slice(b"hunter");
        let mut sent = false;
        assert!(send_stun_ping(
            true,
            true,
            &online,
            0x1000,
            0x2000,
            3658,
            &mut |bytes, _, _| {
                sent = bytes.len() == 21;
                0
            }
        ));
        assert!(sent);
        assert!(!send_stun_ping(
            false,
            true,
            &online,
            0,
            1,
            2,
            &mut |_, _, _| 0
        ));
        let mut cache = Matching2ContextCache::default();
        let mut room = RoomCache {
            room_id: 7,
            server_id: 3,
            ..RoomCache::default()
        };
        room.members.insert(1, MemberCache::default());
        room.members.insert(2, MemberCache::default());
        cache.rooms.insert(7, room);
        cache.peers.insert(
            1,
            PeerInfo {
                ping_us: 20,
                ..PeerInfo::default()
            },
        );
        cache.peers.insert(
            2,
            PeerInfo {
                ping_us: 30,
                ..PeerInfo::default()
            },
        );
        assert_eq!(get_room_ping_us(&cache, 7, 1), 30);
        let ping = build_ping_info_payload(&cache, 1, 7, 1, 0, 0);
        assert_eq!((ping.server_id, ping.room_id, ping.rtt), (3, 7, 30));
        let mut sink = Sink::default();
        assert_eq!(
            queue_dead_for_room_peers(&mut cache, &mut sink, 1, 7, 1, -1),
            1
        );
        assert_eq!(sink.0.len(), 1);
        assert_eq!(sink.0[0].2, 2);
        let info = fill_local_net_info(1, 2, 3);
        assert_eq!(info.size, 24);
    }
}
