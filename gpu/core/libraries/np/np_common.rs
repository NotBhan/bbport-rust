//! NP common helpers, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_common.h`.

use core::ffi::c_void;

use crate::gpu::bbnet::include::core::libraries::kernel::pthread::{
    PthreadCondT, PthreadMutexT, PthreadT,
};

/// `OrbisNpFreeKernelMemoryArgs`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpFreeKernelMemoryArgs {
    pub length: u64,
    pub unk: u64,
    pub addr: *mut c_void,
}

/// `NpThreadEntry`.
pub type NpThreadEntry = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
/// `OrbisNpCalloutHandler`.
pub type OrbisNpCalloutHandler = unsafe extern "C" fn(u64);

/// `OrbisNpCalloutEntry`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpCalloutEntry {
    pub next: *mut OrbisNpCalloutEntry,
    pub handler: Option<OrbisNpCalloutHandler>,
    pub deadline_usec: i64,
    pub arg: u64,
}

/// `OrbisNpCalloutContext`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisNpCalloutContext {
    pub thread: PthreadT,
    pub mutex: PthreadMutexT,
    pub cond: PthreadCondT,
    pub active: i32,
    pub stop_requested: i32,
    pub head: *mut OrbisNpCalloutEntry,
}

/// The NpCommon entry points; the HLE loader implements them.
pub trait NpCommonApi {
    fn get_system_clock_usec(&mut self, usec: &mut i64) -> i32;
    fn get_platform_type(
        &mut self,
        np_id: &crate::gpu::bbnet::include::core::libraries::np::np_types::OrbisNpId,
    ) -> i32;
    fn is_valid_online_id(
        &mut self,
        id: &crate::gpu::bbnet::include::core::libraries::np::np_types::OrbisNpOnlineId,
    ) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_common_structs_match_the_header() {
        assert_eq!(core::mem::size_of::<OrbisNpFreeKernelMemoryArgs>(), 24);
        assert_eq!(core::mem::size_of::<OrbisNpCalloutEntry>(), 32);
        assert_eq!(core::mem::size_of::<OrbisNpCalloutContext>(), 40);
        assert_eq!(
            core::mem::offset_of!(OrbisNpCalloutContext, stop_requested),
            28
        );
        assert_eq!(core::mem::offset_of!(OrbisNpCalloutContext, head), 32);
    }
}

/// `NpCommon` initialization flags.
#[derive(Debug, Default)]
pub struct NpCommonState {
    pub initialized: bool,
    pub title_ctx_count: u32,
}

impl NpCommonState {
    pub fn initialize(&mut self) -> i32 {
        if self.initialized {
            return crate::gpu::core::libraries::np::np_error::ORBIS_NP_ERROR_ALREADY_INITIALIZED;
        }
        self.initialized = true;
        0
    }

    pub fn terminate(&mut self) -> i32 {
        if !self.initialized {
            return crate::gpu::core::libraries::np::np_error::ORBIS_NP_ERROR_NOT_INITIALIZED;
        }
        self.initialized = false;
        self.title_ctx_count = 0;
        0
    }

    pub fn create_title_ctx(&mut self) -> Option<u32> {
        if !self.initialized {
            return None;
        }
        self.title_ctx_count += 1;
        Some(self.title_ctx_count)
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn common_init_is_idempotent_guarded() {
        let mut common = NpCommonState::default();
        assert_eq!(
            common.terminate(),
            crate::gpu::core::libraries::np::np_error::ORBIS_NP_ERROR_NOT_INITIALIZED
        );
        assert_eq!(common.initialize(), 0);
        assert_eq!(
            common.initialize(),
            crate::gpu::core::libraries::np::np_error::ORBIS_NP_ERROR_ALREADY_INITIALIZED
        );
        assert_eq!(common.create_title_ctx(), Some(1));
        assert_eq!(common.terminate(), 0);
    }
}
