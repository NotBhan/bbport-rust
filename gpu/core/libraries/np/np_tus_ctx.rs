//! NpTus request contexts, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_tus/np_tus_ctx.h`.

use std::sync::{Condvar, Mutex};

/// `TusRequestCtx`.
#[derive(Debug, Default)]
pub struct TusRequestCtx {
    mutex: Mutex<Option<i32>>,
    cv: Condvar,
}

impl TusRequestCtx {
    pub fn new() -> Self {
        Self::default()
    }

    /// `SetResult`; the first result wins.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn the_first_result_wins_and_wakes_waiters() {
        let ctx = Arc::new(TusRequestCtx::new());
        assert!(!ctx.has_result());
        let waiter = {
            let ctx = ctx.clone();
            std::thread::spawn(move || ctx.wait())
        };
        std::thread::sleep(std::time::Duration::from_millis(5));
        ctx.set_result(7);
        ctx.set_result(9);
        assert!(ctx.has_result());
        assert_eq!(ctx.wait(), 7);
        assert_eq!(waiter.join().unwrap(), 7);
    }
}
