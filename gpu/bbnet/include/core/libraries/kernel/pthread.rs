//! The pthread surface used by the NP libraries, ported from
//! `bbnet/include/core/libraries/kernel/threads/pthread.h`.

use core::ffi::c_void;

/// `BbNetMutex`; the implementation lives in the runtime glue.
#[repr(C)]
pub struct BbNetMutex {
    pub _opaque: [u8; 0],
}
/// `BbNetMutexAttr`.
#[repr(C)]
pub struct BbNetMutexAttr {
    pub _opaque: [u8; 0],
}
/// `BbNetCond`.
#[repr(C)]
pub struct BbNetCond {
    pub _opaque: [u8; 0],
}
/// `BbNetCondAttr`.
#[repr(C)]
pub struct BbNetCondAttr {
    pub _opaque: [u8; 0],
}
/// `BbNetThreadAttr`.
#[repr(C)]
pub struct BbNetThreadAttr {
    pub _opaque: [u8; 0],
}

pub type PthreadT = *mut c_void;
pub type PthreadMutexT = *mut BbNetMutex;
pub type PthreadMutexAttrT = *mut BbNetMutexAttr;
pub type PthreadCondT = *mut BbNetCond;
pub type PthreadCondAttrT = *mut BbNetCondAttr;
pub type PthreadAttrT = *mut BbNetThreadAttr;
pub type PthreadEntryFunc = Option<unsafe extern "C" fn(*mut c_void) -> *mut c_void>;

/// `PthreadMutexType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PthreadMutexType {
    ErrorCheck = 1,
    Recursive = 2,
    Normal = 3,
    AdaptiveNp = 4,
}

/// `SchedPolicy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SchedPolicy {
    Fifo = 1,
    Other = 2,
    RoundRobin = 3,
}

/// `SchedParam`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SchedParam {
    pub sched_priority: i32,
}

pub const ORBIS_KERNEL_PRIO_FIFO_DEFAULT: i32 = 700;
pub const ORBIS_KERNEL_PRIO_FIFO_HIGHEST: i32 = 256;
pub const ORBIS_KERNEL_PRIO_FIFO_LOWEST: i32 = 767;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_priority_constants_and_enums_match() {
        assert_eq!(PthreadMutexType::Recursive as u32, 2);
        assert_eq!(SchedPolicy::RoundRobin as u32, 3);
        assert_eq!(ORBIS_KERNEL_PRIO_FIFO_DEFAULT, 700);
        assert_eq!(ORBIS_KERNEL_PRIO_FIFO_HIGHEST, 256);
        assert_eq!(ORBIS_KERNEL_PRIO_FIFO_LOWEST, 767);
        assert_eq!(core::mem::size_of::<SchedParam>(), 4);
    }
}
