//! WebAPI push-event handling, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_handler/np_handler_web_api.cpp`.

use std::collections::HashMap;

use crate::gpu::bbnet::shadnet::client::NotifyWebApiPushEvent;
use crate::gpu::core::libraries::np::np_web_api::PushEventInput;

/// `PendingInvitation`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PendingInvitation {
    pub session_id: String,
    pub invitation_id: String,
    pub from_npid: String,
    pub to_npid: String,
    pub valid_until: i64,
    pub from_account_id: u64,
}

/// The NpHandler state this file reads: per-user NP ids and pending invitations.
#[derive(Debug, Default)]
pub struct HandlerWebApiState {
    pub np_ids: HashMap<i32, [u8; 16]>,
    pub pending_invites: HashMap<i32, Vec<PendingInvitation>>,
}

impl HandlerWebApiState {
    pub fn get_account_id(&self, _user_id: i32) -> u64 {
        0
    }
}

/// `SetNpOnlineId`; the guest online id is a fixed 16-byte buffer.
pub fn set_np_online_id(online_id: &mut [u8; 16], value: &str) {
    let bytes = value.as_bytes();
    let count = bytes.len().min(online_id.len());
    online_id[..count].copy_from_slice(&bytes[..count]);
    online_id[count..].fill(0);
}

pub trait PushEventSink {
    fn enqueue_push_event(&mut self, event: PushEventInput);
}

/// `NpHandler::OnWebApiPushEvent`; also returns an invitation prompt target when one is raised.
pub fn on_web_api_push_event(
    user_id: i32,
    notification: &NotifyWebApiPushEvent,
    state: &mut HandlerWebApiState,
    sink: &mut dyn PushEventSink,
) -> Option<(i32, String, String, String)> {
    let mut event = PushEventInput {
        target_user_id: user_id,
        np_service_name: notification.np_service_name.clone(),
        np_service_label: notification.np_service_label,
        data_type: notification.data_type.clone(),
        data: notification.data.clone(),
        extd_data: notification.extd_data.clone(),
        ..PushEventInput::default()
    };
    if !notification.from_npid.is_empty() {
        event.has_from = true;
        set_np_online_id(&mut event.from_online_id.data, &notification.from_npid);
    }
    if !notification.to_npid.is_empty() {
        event.has_to = true;
        set_np_online_id(&mut event.to_online_id.data, &notification.to_npid);
    }
    event.to_account_id = if notification.to_account_id != 0 {
        notification.to_account_id
    } else {
        state.get_account_id(user_id)
    };
    event.from_account_id = notification.from_account_id;
    if event.from_account_id == 0 {
        if let Some((_, value)) = notification
            .extd_data
            .iter()
            .find(|(key, _)| key == "fromAccountId")
        {
            event.from_account_id = value.parse().unwrap_or(0);
        }
    }
    if event.to_account_id != 0 {
        event.has_to = true;
        if notification.to_npid.is_empty() {
            if let Some(online_id) = state.np_ids.get(&user_id) {
                event.to_online_id.data.copy_from_slice(online_id);
            }
        }
    }
    sink.enqueue_push_event(event.clone());

    if notification.np_service_name == "sessionInvitation" {
        let mut session_id = String::new();
        let mut invitation_id = String::new();
        let mut valid_until = 0i64;
        for (key, value) in &notification.extd_data {
            match key.as_str() {
                "sessionId" => session_id = value.clone(),
                "invitationId" => invitation_id = value.clone(),
                "validUntil" => valid_until = value.parse().unwrap_or(0),
                _ => {}
            }
        }
        if !session_id.is_empty() {
            let invites = state.pending_invites.entry(user_id).or_default();
            invites.retain(|invite| invite.invitation_id != invitation_id);
            invites.push(PendingInvitation {
                session_id: session_id.clone(),
                invitation_id: invitation_id.clone(),
                from_npid: notification.from_npid.clone(),
                to_npid: notification.to_npid.clone(),
                valid_until,
                from_account_id: event.from_account_id,
            });
            return Some((
                user_id,
                invitation_id,
                session_id,
                notification.from_npid.clone(),
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Sink {
        events: Vec<PushEventInput>,
    }

    impl PushEventSink for Sink {
        fn enqueue_push_event(&mut self, event: PushEventInput) {
            self.events.push(event);
        }
    }

    #[test]
    fn push_events_copy_ids_accounts_and_invitations() {
        let mut state = HandlerWebApiState::default();
        let mut np_id = [0u8; 16];
        set_np_online_id(&mut np_id, "hunter");
        state.np_ids.insert(3, np_id);
        let notification = NotifyWebApiPushEvent {
            np_service_name: "sessionInvitation".to_string(),
            np_service_label: 7,
            data_type: "type".to_string(),
            data: "body".to_string(),
            from_npid: "host".to_string(),
            to_npid: String::new(),
            extd_data: vec![
                ("fromAccountId".to_string(), "42".to_string()),
                ("sessionId".to_string(), "s1".to_string()),
                ("invitationId".to_string(), "i1".to_string()),
                ("validUntil".to_string(), "99".to_string()),
            ],
            from_account_id: 0,
            to_account_id: 5,
        };
        let mut sink = Sink::default();
        let prompt = on_web_api_push_event(3, &notification, &mut state, &mut sink);
        assert!(prompt.is_some());
        assert_eq!(sink.events.len(), 1);
        let event = &sink.events[0];
        assert_eq!(event.from_account_id, 42);
        assert_eq!(event.to_account_id, 5);
        assert!(event.has_from);
        assert!(event.has_to);
        assert_eq!(state.pending_invites[&3].len(), 1);
        assert_eq!(state.pending_invites[&3][0].session_id, "s1");
    }
}
