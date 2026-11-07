//! NpUtility contexts, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_utility/np_utility_ctx.h`.

use std::sync::{Condvar, Mutex};

use crate::gpu::bbnet::include::core::libraries::np::np_types::OrbisNpId;

/// `LookupTimeouts`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LookupTimeouts {
    pub resolve_retry: i32,
    pub resolve_timeout: u32,
    pub conn_timeout: u32,
    pub send_timeout: u32,
    pub recv_timeout: u32,
}

/// `LookupTitleCtx`.
#[derive(Debug, Clone)]
pub struct LookupTitleCtx {
    pub user_id: i32,
    pub self_np_id: OrbisNpId,
    pub timeouts: Option<LookupTimeouts>,
}

impl Default for LookupTitleCtx {
    fn default() -> Self {
        Self {
            user_id: -1,
            self_np_id: OrbisNpId::default(),
            timeouts: None,
        }
    }
}

/// `LookupRequestCtx`.
#[derive(Debug, Default)]
pub struct LookupRequestCtx {
    mutex: Mutex<Option<i32>>,
    cv: Condvar,
    pub title_ctx_id: i32,
    pub is_async: bool,
    pub used: bool,
    pub aborted: bool,
    pub timeouts: Option<LookupTimeouts>,
}

impl LookupRequestCtx {
    pub fn new() -> Self {
        Self::default()
    }

    /// `SetResult`.
    pub fn set_result(&self, result: i32) {
        {
            let mut slot = self.mutex.lock().unwrap_or_else(|error| error.into_inner());
            if slot.is_some() {
                return;
            }
            *slot = Some(result);
        }
        self.cv.notify_all();
    }

    /// `HasResult`.
    pub fn has_result(&self) -> bool {
        self.mutex
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .is_some()
    }

    /// `Wait`.
    pub fn wait(&self) -> i32 {
        let mut slot = self.mutex.lock().unwrap_or_else(|error| error.into_inner());
        while slot.is_none() {
            slot = self
                .cv
                .wait(slot)
                .unwrap_or_else(|error| error.into_inner());
        }
        slot.expect("the result is present")
    }
}

/// `WordFilterTitleCtx`.
#[derive(Debug, Clone)]
pub struct WordFilterTitleCtx {
    pub user_id: i32,
    pub self_np_id: OrbisNpId,
    pub timeouts: Option<LookupTimeouts>,
}

impl Default for WordFilterTitleCtx {
    fn default() -> Self {
        Self {
            user_id: -1,
            self_np_id: OrbisNpId::default(),
            timeouts: None,
        }
    }
}

/// `WordFilterRequestCtx`.
pub type WordFilterRequestCtx = LookupRequestCtx;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn lookup_requests_share_the_result_protocol() {
        let ctx = Arc::new(LookupRequestCtx::new());
        assert_eq!(LookupTitleCtx::default().user_id, -1);
        let waiter = {
            let ctx = ctx.clone();
            std::thread::spawn(move || ctx.wait())
        };
        std::thread::sleep(std::time::Duration::from_millis(5));
        ctx.set_result(3);
        assert_eq!(waiter.join().unwrap(), 3);
        assert!(ctx.has_result());
        assert_eq!(LookupTimeouts::default().resolve_retry, 0);
    }
}
