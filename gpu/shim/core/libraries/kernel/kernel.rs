//! The kernel.h parts used by vendored libraries, ported from
//! `legacy/gpu/shim/core/libraries/kernel/kernel.h`.

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SwVersionStruct {
    pub struct_size: u64,
    pub text_representation: [std::ffi::c_char; 0x1c],
    pub hex_representation: u32,
}

extern "C" {
    pub fn __Error() -> *mut i32;
    pub fn ErrnoToSceKernelError(error: i32) -> i32;
    pub fn sceKernelGetSystemSwVersion(version: *mut SwVersionStruct) -> i32;
}
