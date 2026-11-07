//! NpMatching2 context/dispatch internals, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_matching2/np_matching2_internal.h`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::gpu::core::libraries::np::np_matching2_cache::Matching2ContextCache;
use crate::gpu::core::libraries::np::np_types2::*;

/// `CallbackPayload`; response pointers are opaque in the port.
#[derive(Debug, Default)]
pub struct CallbackPayload {
    pub room_data: Option<usize>,
    pub create_join_response: Option<usize>,
    pub leave_room_response: Option<usize>,
    pub search_room_response: Option<usize>,
    pub world_info_response: Option<usize>,
    pub world_list: Vec<OrbisNpMatching2World>,
    pub member_data: Vec<u8>,
    pub room_groups: Vec<OrbisNpMatching2RoomGroup>,
    pub room_bin_attrs: Vec<OrbisNpMatching2RoomBinAttrInternal>,
    pub member_bin_attrs: Vec<OrbisNpMatching2RoomMemberBinAttrInternal>,
    pub user_info: Vec<OrbisNpMatching2UserInfo>,
    pub user_bin_attrs: Vec<OrbisNpMatching2BinAttr>,
    pub ext_int_attrs: Vec<OrbisNpMatching2IntAttr>,
    pub request_data: *mut core::ffi::c_void,
    pub room_message_data: Vec<u8>,
    pub room_event_data: *mut core::ffi::c_void,
}

impl CallbackPayload {
    pub fn reset(&mut self) {
        self.room_data = None;
        self.create_join_response = None;
        self.leave_room_response = None;
        self.search_room_response = None;
        self.world_info_response = None;
        self.world_list.clear();
        self.member_data.clear();
        self.room_groups.clear();
        self.room_bin_attrs.clear();
        self.member_bin_attrs.clear();
        self.user_info.clear();
        self.user_bin_attrs.clear();
        self.ext_int_attrs.clear();
        self.request_data = std::ptr::null_mut();
        self.room_message_data.clear();
        self.room_event_data = std::ptr::null_mut();
    }
}

/// `ContextObject`.
#[derive(Debug, Default)]
pub struct ContextObject {
    pub ctx_id: OrbisNpMatching2ContextId,
    pub started: bool,
    pub stop_pending: bool,
    pub destroy_pending: bool,
    pub a_variant: bool,
    pub server_id: OrbisNpMatching2ServerId,
    pub service_label: u32,
    pub world_id: OrbisNpMatching2WorldId,
    pub lobby_id: OrbisNpMatching2LobbyId,
    pub room_id: OrbisNpMatching2RoomId,
    pub my_member_id: OrbisNpMatching2RoomMemberId,
    pub is_room_owner: bool,
    pub max_slot: u32,
    pub flag_attr: OrbisNpMatching2Flags,
    pub cache: Matching2ContextCache,
}

impl ContextObject {
    pub fn new(ctx_id: OrbisNpMatching2ContextId) -> Self {
        Self {
            ctx_id,
            server_id: 1,
            max_slot: 5,
            ..Self::default()
        }
    }
}

/// `RequestCallbackInfo`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RequestCallbackInfo {
    pub callback: OrbisNpMatching2RequestCallback,
    pub arg: *mut core::ffi::c_void,
}

/// `ContextManager`.
#[derive(Debug, Default)]
pub struct ContextManager {
    pub contexts: HashMap<OrbisNpMatching2ContextId, ContextObject>,
    pub next_id: OrbisNpMatching2ContextId,
    pub pending_context_callback: OrbisNpMatching2ContextCallback,
}

pub const MAX_CONTEXTS: u32 = 10;

impl ContextManager {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            ..Self::default()
        }
    }

    pub fn create_context(&mut self) -> Option<OrbisNpMatching2ContextId> {
        if self.contexts.len() >= MAX_CONTEXTS as usize {
            return None;
        }
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.contexts.insert(id, ContextObject::new(id));
        Some(id)
    }

    pub fn get(&self, ctx_id: OrbisNpMatching2ContextId) -> Option<&ContextObject> {
        self.contexts.get(&ctx_id)
    }

    pub fn destroy(&mut self, ctx_id: OrbisNpMatching2ContextId) -> bool {
        self.contexts.remove(&ctx_id).is_some()
    }

    pub fn start(&mut self, ctx_id: OrbisNpMatching2ContextId) -> i32 {
        match self.contexts.get_mut(&ctx_id) {
            Some(context) => {
                context.started = true;
                0
            }
            None => -1,
        }
    }

    pub fn stop(&mut self, ctx_id: OrbisNpMatching2ContextId) -> i32 {
        match self.contexts.get_mut(&ctx_id) {
            Some(context) => {
                context.stop_pending = true;
                0
            }
            None => -1,
        }
    }
}

/// `PendingEvent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PendingEventType {
    ContextCb = 0,
    RequestCb = 1,
    SignalingCb = 2,
    RoomEventCb = 3,
    LobbyEventCb = 4,
    LobbyMessageCb = 5,
    RoomMessageCb = 6,
}

/// `NpMatching2State`.
#[derive(Debug, Default)]
pub struct NpMatching2State {
    pub initialized: AtomicBool,
    pub next_request_id: OrbisNpMatching2RequestId,
    pub pending_events: Vec<u8>,
    pub dispatch_running: AtomicBool,
}

impl NpMatching2State {
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }

    pub fn set_initialized(&self, value: bool) {
        self.initialized.store(value, Ordering::Release);
    }

    pub fn alloc_request_id(&mut self) -> OrbisNpMatching2RequestId {
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1).max(1);
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contexts_are_created_started_and_destroyed() {
        let mut manager = ContextManager::new();
        let id = manager.create_context().expect("context");
        assert_eq!(manager.get(id).unwrap().max_slot, 5);
        assert_eq!(manager.start(id), 0);
        assert!(manager.get(id).unwrap().started);
        assert_eq!(manager.stop(id), 0);
        assert!(manager.get(id).unwrap().stop_pending);
        assert!(manager.destroy(id));
        assert!(manager.get(id).is_none());
        for _ in 0..MAX_CONTEXTS {
            let _ = manager.create_context();
        }
        assert!(manager.create_context().is_none());
        let mut payload = CallbackPayload::default();
        payload.world_list.push(OrbisNpMatching2World::default());
        payload.reset();
        assert!(payload.world_list.is_empty());
        let mut state = NpMatching2State {
            next_request_id: 1,
            ..NpMatching2State::default()
        };
        assert_eq!(state.alloc_request_id(), 1);
        assert_eq!(state.alloc_request_id(), 2);
    }
}

/// The queued-event dispatcher used by the matching2 cpp.
#[derive(Debug, Default)]
pub struct EventDispatcher {
    pub queue: std::collections::VecDeque<u8>,
    pub running: bool,
}

impl EventDispatcher {
    pub fn init(&mut self) {
        self.running = true;
    }

    pub fn term(&mut self) {
        self.running = false;
        self.queue.clear();
    }

    pub fn schedule(&mut self, event_type: PendingEventType) {
        if self.running {
            self.queue.push_back(event_type as u8);
        }
    }

    pub fn drain(&mut self) -> Vec<PendingEventType> {
        let events = self
            .queue
            .drain(..)
            .filter_map(|event| match event {
                0 => Some(PendingEventType::ContextCb),
                1 => Some(PendingEventType::RequestCb),
                2 => Some(PendingEventType::SignalingCb),
                3 => Some(PendingEventType::RoomEventCb),
                4 => Some(PendingEventType::LobbyEventCb),
                5 => Some(PendingEventType::LobbyMessageCb),
                6 => Some(PendingEventType::RoomMessageCb),
                _ => None,
            })
            .collect();
        events
    }
}

#[cfg(test)]
mod dispatcher_cpp_tests {
    use super::*;

    #[test]
    fn dispatcher_queues_only_when_running() {
        let mut dispatcher = EventDispatcher::default();
        dispatcher.schedule(PendingEventType::ContextCb);
        assert!(dispatcher.drain().is_empty());
        dispatcher.init();
        dispatcher.schedule(PendingEventType::RoomEventCb);
        assert_eq!(dispatcher.drain(), vec![PendingEventType::RoomEventCb]);
        dispatcher.term();
        assert!(!dispatcher.running);
    }
}
