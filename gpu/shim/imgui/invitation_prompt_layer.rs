//! Session-invitation prompt queue, ported from
//! `legacy/gpu/shim/imgui/invitation_prompt_layer.h`.

use std::sync::{Mutex, OnceLock};

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

/// Queue a prompt, replacing an older one with the same invitation id.
pub fn push(user_id: i32, invitation_id: String, session_id: String, from_npid: String) {
    let prompt = Prompt {
        user_id,
        invitation_id: invitation_id.clone(),
        session_id,
        from_npid,
    };
    if let Ok(mut prompts) = queue().lock() {
        prompts.retain(|existing| existing.invitation_id != invitation_id);
        prompts.push(prompt);
    }
}

pub fn dismiss(invitation_id: &str) {
    if let Ok(mut prompts) = queue().lock() {
        prompts.retain(|prompt| prompt.invitation_id != invitation_id);
    }
}

pub fn pending() -> Vec<Prompt> {
    queue()
        .lock()
        .map(|prompts| prompts.clone())
        .unwrap_or_default()
}

pub fn register() {}
pub fn unregister() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pushes_deduplicate_and_dismiss_removes() {
        push(1, "inv".into(), "sess".into(), "hunter".into());
        push(1, "inv".into(), "sess2".into(), "hunter".into());
        let prompts = pending();
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].session_id, "sess2");
        dismiss("inv");
        assert!(pending().is_empty());
    }
}
