//! Session invitations, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_handler/np_handler_invitation.cpp`.

use std::collections::HashMap;

use crate::gpu::core::libraries::np::np_handler_web_api::PendingInvitation;

/// `JsonEscape`.
pub fn json_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 8);
    for character in input.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if (character as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => out.push(character),
        }
    }
    out
}

/// Builds the multipart body the shadNet server expects for `invitation-request`.
pub fn build_invitation_body(
    user_id: i32,
    session_id: &str,
    recipients: &[String],
    message: &str,
) -> Option<(String, String)> {
    if session_id.is_empty() || recipients.is_empty() {
        return None;
    }
    let mut json = String::from("{\"to\":[");
    for (index, recipient) in recipients.iter().enumerate() {
        if index != 0 {
            json.push(',');
        }
        json.push('"');
        json.push_str(&json_escape(recipient));
        json.push('"');
    }
    json.push(']');
    if !message.is_empty() {
        json.push_str(",\"message\":\"");
        json.push_str(&json_escape(message));
        json.push('"');
    }
    json.push('}');

    let boundary = format!("shadps4invite{user_id}");
    let mut body = String::new();
    body.push_str(&format!("--{boundary}\r\n"));
    body.push_str("Content-Type: application/json; charset=utf-8\r\n");
    body.push_str("Content-Description: invitation-request\r\n\r\n");
    body.push_str(&json);
    body.push_str(&format!("\r\n--{boundary}--\r\n"));
    let path = format!("/v1/sessions/{session_id}/invitations");
    let content_type = format!("multipart/mixed; boundary={boundary}");
    Some((format!("{path}\n{content_type}"), body))
}

pub trait InvitationHttpClient {
    fn post(&mut self, path: &str, body: &str, content_type: &str) -> Option<u16>;
}

/// `SendSessionInvitation`.
pub fn send_session_invitation(
    user_id: i32,
    session_id: &str,
    recipients: &[String],
    message: &str,
    base_url: &str,
    token: &str,
    client: &mut dyn InvitationHttpClient,
) -> bool {
    if base_url.is_empty() || token.is_empty() {
        return false;
    }
    let Some((request, body)) = build_invitation_body(user_id, session_id, recipients, message)
    else {
        return false;
    };
    let mut lines = request.split('\n');
    let path = lines.next().unwrap_or_default();
    let content_type = lines.next().unwrap_or_default();
    matches!(client.post(path, &body, content_type), Some(200 | 204))
}

/// `OrbisNpSessionInvitationEventParam`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionInvitationEventParam {
    pub session_id: [u8; 32],
    pub invitation_id: [u8; 32],
    pub flag: u32,
    pub online_id: [u8; 16],
    pub user_id: i32,
    pub referral_online_id: [u8; 16],
    pub referral_account_id: u64,
}

pub const SESSION_INVITATION_EVENT_FLAG_INVITATION: u32 = 1;

fn copy_fixed<const N: usize>(value: &str) -> [u8; N] {
    let mut out = [0u8; N];
    let count = value.len().min(N);
    out[..count].copy_from_slice(&value.as_bytes()[..count]);
    out
}

/// `PostSessionInvitationEvent`; returns the guest event the system service pushes.
pub fn post_session_invitation_event(
    user_id: i32,
    session_id: &str,
    invitation_id: &str,
    accepter_online_id: &str,
    inviter_online_id: &str,
    inviter_account_id: u64,
) -> SessionInvitationEventParam {
    SessionInvitationEventParam {
        session_id: copy_fixed(session_id),
        invitation_id: copy_fixed(invitation_id),
        flag: if invitation_id.is_empty() {
            0
        } else {
            SESSION_INVITATION_EVENT_FLAG_INVITATION
        },
        online_id: copy_fixed(accepter_online_id),
        user_id,
        referral_online_id: copy_fixed(inviter_online_id),
        referral_account_id: inviter_account_id,
    }
}

/// `GetPendingInvitations`; drops invitations whose `valid_until` passed.
pub fn get_pending_invitations(
    pending: &mut HashMap<i32, Vec<PendingInvitation>>,
    user_id: i32,
    now_ms: i64,
) -> Vec<PendingInvitation> {
    let Some(invitations) = pending.get_mut(&user_id) else {
        return Vec::new();
    };
    invitations
        .retain(|invitation| invitation.valid_until == 0 || invitation.valid_until >= now_ms);
    invitations.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Client(Option<u16>);

    impl InvitationHttpClient for Client {
        fn post(&mut self, _: &str, _: &str, _: &str) -> Option<u16> {
            self.0
        }
    }

    #[test]
    fn invitation_bodies_escape_and_post() {
        assert_eq!(json_escape("a\"b\\c\n"), "a\\\"b\\\\c\\n");
        let (request, body) =
            build_invitation_body(7, "s1", &["a\"b".to_string()], "hi").expect("body");
        assert!(request.starts_with("/v1/sessions/s1/invitations"));
        assert!(body.contains("Content-Description: invitation-request"));
        assert!(body.contains("a\\\"b"));
        assert!(!send_session_invitation(
            7,
            "s1",
            &["a".into()],
            "",
            "",
            "token",
            &mut Client(Some(200))
        ));
        assert!(send_session_invitation(
            7,
            "s1",
            &["a".into()],
            "",
            "http://x",
            "token",
            &mut Client(Some(204))
        ));
        let param = post_session_invitation_event(1, "s", "i", "accepter", "inviter", 9);
        assert_eq!(param.flag, 1);
        assert_eq!(param.referral_account_id, 9);
    }

    #[test]
    fn pending_invitations_expire() {
        let mut pending = HashMap::new();
        pending.insert(
            1,
            vec![PendingInvitation {
                session_id: "s".into(),
                invitation_id: "i".into(),
                valid_until: 10,
                ..PendingInvitation::default()
            }],
        );
        assert_eq!(get_pending_invitations(&mut pending, 1, 5).len(), 1);
        assert_eq!(get_pending_invitations(&mut pending, 1, 11).len(), 0);
    }
}
