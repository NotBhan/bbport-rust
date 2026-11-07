//! Fault-handler registry, ported from `legacy/gpu/shim/core/signals.h`.
//! Handlers run in priority order; the loader's SIGSEGV handler calls this first.

use std::collections::BTreeMap;

pub type AccessViolationHandler = fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> bool;
pub type IllegalInstructionHandler = fn(*mut std::ffi::c_void) -> bool;

#[derive(Default)]
pub struct SignalDispatch {
    access_violation_handlers: BTreeMap<u32, AccessViolationHandler>,
    illegal_instruction_handlers: BTreeMap<u32, IllegalInstructionHandler>,
}

impl SignalDispatch {
    pub fn remove_handlers(&mut self) {
        self.access_violation_handlers.clear();
        self.illegal_instruction_handlers.clear();
    }

    pub fn register_access_violation(&mut self, handler: AccessViolationHandler, priority: u32) {
        self.access_violation_handlers.insert(priority, handler);
    }

    pub fn register_illegal_instruction(
        &mut self,
        handler: IllegalInstructionHandler,
        priority: u32,
    ) {
        self.illegal_instruction_handlers.insert(priority, handler);
    }

    pub fn dispatch_access_violation(
        &self,
        context: *mut std::ffi::c_void,
        fault: *mut std::ffi::c_void,
    ) -> bool {
        self.access_violation_handlers
            .values()
            .any(|handler| handler(context, fault))
    }

    pub fn dispatch_illegal_instruction(&self, context: *mut std::ffi::c_void) -> bool {
        self.illegal_instruction_handlers
            .values()
            .any(|handler| handler(context))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static ORDER: AtomicUsize = AtomicUsize::new(0);

    fn low(_: *mut std::ffi::c_void, _: *mut std::ffi::c_void) -> bool {
        ORDER.store(1, Ordering::Relaxed);
        true
    }
    fn high(_: *mut std::ffi::c_void, _: *mut std::ffi::c_void) -> bool {
        ORDER.store(2, Ordering::Relaxed);
        false
    }

    #[test]
    fn handlers_run_in_priority_order_until_one_claims_the_fault() {
        let mut dispatch = SignalDispatch::default();
        dispatch.register_access_violation(low, 10);
        dispatch.register_access_violation(high, 1);
        assert!(dispatch.dispatch_access_violation(std::ptr::null_mut(), std::ptr::null_mut()));
        assert_eq!(ORDER.load(Ordering::Relaxed), 1);
        dispatch.remove_handlers();
        assert!(!dispatch.dispatch_access_violation(std::ptr::null_mut(), std::ptr::null_mut()));
    }
}
