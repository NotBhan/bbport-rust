//! The kernel memory calls NpSignaling makes for its private heap, ported from
//! `legacy/gpu/shim/core/libraries/kernel/memory.h`. The host-memory implementation is part of
//! the pending `runtime_memory` port; these are the C ABI entry points.

#[repr(C)]
pub struct MappedMemory {
    pub address: *mut std::ffi::c_void,
}

extern "C" {
    pub fn sceKernelMapNamedFlexibleMemory(
        addr_in_out: *mut *mut std::ffi::c_void,
        len: u64,
        prot: i32,
        flags: i32,
        name: *const std::ffi::c_char,
    ) -> i32;
    pub fn sceKernelMunmap(addr: *mut std::ffi::c_void, len: u64) -> i32;
}
