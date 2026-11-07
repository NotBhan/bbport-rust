//! Invitation prompts, ported from `bbnet/include/imgui/invitation_prompt_layer.h`.

use std::sync::{Mutex, OnceLock};

/// A queued accept/decline prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    pub user_id: i32,
    pub invitation_id: String,
    pub session_id: String,
    pub from_npid: String,
}

fn queue() -> &'static Mutex<Vec<Prompt>> {
    static QUEUE: OnceLock<Mutex<Vec<Prompt>>> = OnceLock::new();
    QUEUE.get_or_init(|| Mutex::new(Vec::new()))
}

/// `Push`; a repeated invitation id replaces the older prompt.
pub fn push(user_id: i32, invitation_id: String, session_id: String, from_npid: String) {
    let mut queue = queue().lock().unwrap_or_else(|error| error.into_inner());
    if let Some(existing) = queue
        .iter_mut()
        .find(|prompt| prompt.invitation_id == invitation_id)
    {
        *existing = Prompt {
            user_id,
            invitation_id,
            session_id,
            from_npid,
        };
    } else {
        queue.push(Prompt {
            user_id,
            invitation_id,
            session_id,
            from_npid,
        });
    }
}

/// `Dismiss`.
pub fn dismiss(invitation_id: &str) {
    queue()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .retain(|prompt| prompt.invitation_id != invitation_id);
}

/// The queued prompts, for the UI.
pub fn prompts() -> Vec<Prompt> {
    queue()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

fn registered() -> &'static Mutex<bool> {
    static REGISTERED: Mutex<bool> = Mutex::new(false);
    &REGISTERED
}

/// `Register`.
pub fn register() {
    *registered()
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = true;
}

/// `Unregister`.
pub fn unregister() {
    *registered()
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = false;
}

/// Whether the overlay is registered.
pub fn is_registered() -> bool {
    *registered()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invitations_replace_by_id_and_dismiss() {
        push(1, "inv-1".into(), "sess-1".into(), "hunter".into());
        push(2, "inv-2".into(), "sess-2".into(), "other".into());
        assert_eq!(prompts().len(), 2);
        push(3, "inv-1".into(), "sess-3".into(), "new".into());
        let queued = prompts();
        assert_eq!(queued.len(), 2);
        assert_eq!(queued[0].user_id, 3);
        assert_eq!(queued[0].session_id, "sess-3");
        dismiss("inv-1");
        assert_eq!(prompts().len(), 1);
        dismiss("missing");
        assert_eq!(prompts().len(), 1);
    }

    #[test]
    fn the_registration_flag_round_trips() {
        register();
        assert!(is_registered());
        unregister();
        assert!(!is_registered());
    }
}
