//! Friendship state handling, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_handler/np_handler_friends.cpp`.

use std::collections::HashMap;

use crate::gpu::bbnet::shadnet::client::{
    ErrorType, LoginResult, NotifyFriendLost, NotifyFriendNew, NotifyFriendQuery,
    NotifyFriendStatus,
};
use crate::gpu::core::libraries::np::np_error::ORBIS_NP_ERROR_SIGNED_OUT;

/// `FriendInfo`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FriendInfo {
    pub npid: String,
    pub online: bool,
}

/// `FriendListSnapshot`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FriendListSnapshot {
    pub friends: Vec<FriendInfo>,
    pub requests_sent: Vec<String>,
    pub requests_received: Vec<String>,
    pub blocked: Vec<String>,
}

/// Per-user friendship state.
#[derive(Debug, Default)]
pub struct FriendState {
    pub users: HashMap<i32, FriendListSnapshot>,
    pub notifications: Vec<(String, String)>,
}

impl FriendState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_num_friends(&self, user_id: i32) -> u32 {
        self.users
            .get(&user_id)
            .map_or(0, |state| state.friends.len() as u32)
    }

    pub fn get_friend_npid(&self, user_id: i32, index: u32) -> Option<String> {
        self.users
            .get(&user_id)
            .and_then(|state| state.friends.get(index as usize))
            .map(|friend| friend.npid.clone())
    }

    /// `OnFriendQuery`.
    pub fn on_friend_query(&mut self, user_id: i32, notification: &NotifyFriendQuery) {
        self.notifications.push((
            "friend_request".to_string(),
            format!("Friend request from {}", notification.from_npid),
        ));
        let state = self.users.entry(user_id).or_default();
        if !state.requests_received.contains(&notification.from_npid) {
            state.requests_received.push(notification.from_npid.clone());
        }
    }

    /// `OnFriendNew`.
    pub fn on_friend_new(&mut self, user_id: i32, notification: &NotifyFriendNew) {
        self.notifications.push((
            "friend_new".to_string(),
            format!("{} is now your friend", notification.npid),
        ));
        let state = self.users.entry(user_id).or_default();
        if let Some(friend) = state
            .friends
            .iter_mut()
            .find(|friend| friend.npid == notification.npid)
        {
            friend.online = notification.online;
        } else {
            state.friends.push(FriendInfo {
                npid: notification.npid.clone(),
                online: notification.online,
            });
        }
        state
            .requests_received
            .retain(|npid| npid != &notification.npid);
        state
            .requests_sent
            .retain(|npid| npid != &notification.npid);
    }

    /// `OnFriendLost`.
    pub fn on_friend_lost(&mut self, user_id: i32, notification: &NotifyFriendLost) {
        self.notifications.push((
            "friend_lost".to_string(),
            format!("{} removed you as a friend", notification.npid),
        ));
        let state = self.users.entry(user_id).or_default();
        state
            .friends
            .retain(|friend| friend.npid != notification.npid);
    }

    /// `OnFriendStatus`.
    pub fn on_friend_status(&mut self, user_id: i32, notification: &NotifyFriendStatus) {
        if notification.online {
            self.notifications.push((
                "online".to_string(),
                format!("{} is online", notification.npid),
            ));
        }
        let state = self.users.entry(user_id).or_default();
        if let Some(friend) = state
            .friends
            .iter_mut()
            .find(|friend| friend.npid == notification.npid)
        {
            friend.online = notification.online;
        } else {
            state.friends.push(FriendInfo {
                npid: notification.npid.clone(),
                online: notification.online,
            });
        }
    }

    /// `OnLoginResult`.
    pub fn on_login_result(&mut self, user_id: i32, result: &LoginResult) {
        if result.error != ErrorType::NoError {
            return;
        }
        let mut snapshot = FriendListSnapshot {
            friends: result
                .friends
                .iter()
                .map(|friend| FriendInfo {
                    npid: friend.npid.clone(),
                    online: friend.online,
                })
                .collect(),
            requests_sent: result.requests_sent.clone(),
            requests_received: result.requests_received.clone(),
            blocked: result.blocked.clone(),
        };
        if !snapshot.requests_received.is_empty() {
            self.notifications.push((
                "friend_request".to_string(),
                format!(
                    "{} pending friend request(s)",
                    snapshot.requests_received.len()
                ),
            ));
        }
        snapshot.friends.sort_by(|a, b| a.npid.cmp(&b.npid));
        self.users.insert(user_id, snapshot);
    }

    pub fn friend_list(&self, user_id: i32) -> FriendListSnapshot {
        self.users.get(&user_id).cloned().unwrap_or_default()
    }

    /// `SendFriendRequest`; the client must be authenticated.
    pub fn send_friend_request(
        &mut self,
        user_id: i32,
        npid: &str,
        client: &mut dyn FriendCommandClient,
    ) -> i32 {
        if !client.is_authenticated() {
            return ORBIS_NP_ERROR_SIGNED_OUT;
        }
        client.add_friend(npid);
        let state = self.users.entry(user_id).or_default();
        if state.requests_received.contains(&npid.to_string()) {
            state.requests_received.retain(|request| request != npid);
            state.friends.push(FriendInfo {
                npid: npid.to_string(),
                online: false,
            });
        } else if !state.requests_sent.iter().any(|request| request == npid) {
            state.requests_sent.push(npid.to_string());
        }
        0
    }
}

pub trait FriendCommandClient {
    fn is_authenticated(&self) -> bool;
    fn add_friend(&mut self, npid: &str);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Client(bool);
    impl FriendCommandClient for Client {
        fn is_authenticated(&self) -> bool {
            self.0
        }
        fn add_friend(&mut self, _: &str) {}
    }

    #[test]
    fn friend_notifications_update_the_snapshot() {
        let mut friends = FriendState::new();
        friends.on_friend_new(
            3,
            &NotifyFriendNew {
                npid: "a".to_string(),
                online: true,
            },
        );
        assert_eq!(friends.get_num_friends(3), 1);
        assert_eq!(friends.get_friend_npid(3, 0).as_deref(), Some("a"));
        friends.on_friend_status(
            3,
            &NotifyFriendStatus {
                npid: "b".to_string(),
                online: true,
                timestamp: 1,
            },
        );
        assert_eq!(friends.get_num_friends(3), 2);
        friends.on_friend_lost(
            3,
            &NotifyFriendLost {
                npid: "a".to_string(),
            },
        );
        assert_eq!(friends.get_num_friends(3), 1);
        assert_eq!(
            friends.send_friend_request(3, "c", &mut Client(false)),
            ORBIS_NP_ERROR_SIGNED_OUT
        );
        assert_eq!(friends.send_friend_request(3, "c", &mut Client(true)), 0);
        assert_eq!(friends.friend_list(3).requests_sent, vec!["c"]);
    }
}
