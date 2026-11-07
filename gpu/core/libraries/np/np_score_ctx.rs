//! NpScore request contexts, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_score/np_score_ctx.h`.

use std::sync::{Condvar, Mutex};

/// `ScoreRequestCtx`.
#[derive(Debug)]
pub struct ScoreRequestCtx {
    mutex: Mutex<Option<i32>>,
    cv: Condvar,
    pub title_ctx_id: i32,
    pub user_id: i32,
    pub pc_id: i32,
    pub tmp_rank_out: *mut u32,
}

impl Default for ScoreRequestCtx {
    fn default() -> Self {
        Self {
            mutex: Mutex::new(None),
            cv: Condvar::new(),
            title_ctx_id: 0,
            user_id: -1,
            pc_id: 0,
            tmp_rank_out: std::ptr::null_mut(),
        }
    }
}

impl ScoreRequestCtx {
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

// The rank output pointer is an opaque guest address guarded by the request lifetime.
unsafe impl Send for ScoreRequestCtx {}
unsafe impl Sync for ScoreRequestCtx {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn score_requests_carry_their_ids_and_wait() {
        let ctx = Arc::new(ScoreRequestCtx::new());
        assert_eq!(ctx.user_id, -1);
        assert_eq!(ctx.title_ctx_id, 0);
        assert!(ctx.tmp_rank_out.is_null());
        let waiter = {
            let ctx = ctx.clone();
            std::thread::spawn(move || ctx.wait())
        };
        std::thread::sleep(std::time::Duration::from_millis(5));
        ctx.set_result(11);
        assert_eq!(waiter.join().unwrap(), 11);
        ctx.set_result(12);
        assert_eq!(ctx.wait(), 11);
    }
}
