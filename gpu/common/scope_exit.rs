//! Scope guards, ported from `legacy/gpu/shadps4/common/scope_exit.h`. The C++'s `SCOPE_EXIT`
//! macro becomes a guard value that runs its closure when it is dropped, and `SCOPE_GUARD`
//! becomes the same guard with `cancel` available.

/// `detail::ScopeGuard<F>`: runs its closure on drop unless it was cancelled.
pub struct ScopeGuard<F: FnOnce()> {
    callback: Option<F>,
}

impl<F: FnOnce()> ScopeGuard<F> {
    /// `detail::MakeScopeGuard`.
    pub fn new(callback: F) -> Self {
        Self {
            callback: Some(callback),
        }
    }

    /// `ScopeGuard::Cancel`.
    pub fn cancel(&mut self) {
        self.callback = None;
    }

    /// Whether the guard will still run.
    pub fn is_active(&self) -> bool {
        self.callback.is_some()
    }
}

impl<F: FnOnce()> Drop for ScopeGuard<F> {
    fn drop(&mut self) {
        if let Some(callback) = self.callback.take() {
            callback();
        }
    }
}

/// The C++'s `SCOPE_GUARD` / `SCOPE_EXIT`: a guard the caller holds and may cancel.
pub fn scope_guard<F: FnOnce()>(callback: F) -> ScopeGuard<F> {
    ScopeGuard::new(callback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn the_guard_runs_once_when_it_goes_out_of_scope() {
        let log = RefCell::new(Vec::new());
        {
            let _guard = scope_guard(|| log.borrow_mut().push("exit"));
            assert!(log.borrow().is_empty(), "nothing runs before the drop");
        }
        assert_eq!(*log.borrow(), vec!["exit"]);

        // A moved guard still runs exactly once.
        let second = RefCell::new(Vec::new());
        {
            let guard = scope_guard(|| second.borrow_mut().push("moved"));
            let _moved = guard;
            second.borrow_mut().push("before");
        }
        assert_eq!(*second.borrow(), vec!["before", "moved"]);
    }

    #[test]
    fn a_cancelled_guard_does_not_run() {
        let log = RefCell::new(Vec::new());
        {
            let mut guard = scope_guard(|| log.borrow_mut().push("cancelled"));
            assert!(guard.is_active());
            guard.cancel();
            assert!(!guard.is_active());
        }
        assert!(log.borrow().is_empty());

        // Nested guards unwind in reverse, as the C++'s locals do.
        let order = RefCell::new(Vec::new());
        {
            let _outer = scope_guard(|| order.borrow_mut().push("outer"));
            let _inner = scope_guard(|| order.borrow_mut().push("inner"));
        }
        assert_eq!(*order.borrow(), vec!["inner", "outer"]);
    }
}
