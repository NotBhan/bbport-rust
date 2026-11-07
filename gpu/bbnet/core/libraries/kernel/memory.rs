//! The memory calls the co-op network code makes for its private heap, ported from
//! `legacy/gpu/bbnet/include/core/libraries/kernel/memory.h`. The C build serves these from
//! the guest's address space; the Rust port backs them with host pages, as the header notes.

use std::ffi::{c_char, c_void};

/// `ORBIS_OK`.
const ORBIS_OK: i32 = 0;
/// The guest's `EINVAL`.
const EINVAL: i32 = 22;
/// The guest's `ENOMEM`.
const ENOMEM: i32 = 12;

/// Reads the guest's `sceKernelMapNamedFlexibleMemory` call. `addr_in_out` receives the
/// mapping; the name is only a diagnostic label.
///
/// # Safety
/// `addr_in_out` must be a writable pointer to a `*mut c_void`.
pub unsafe fn sce_kernel_map_named_flexible_memory(
    addr_in_out: *mut *mut c_void,
    len: u64,
    _prot: i32,
    _flags: i32,
    _name: *const c_char,
) -> i32 {
    if addr_in_out.is_null() || len == 0 || len > isize::MAX as u64 {
        return EINVAL;
    }
    let mapping = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            len as usize,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if mapping == libc::MAP_FAILED {
        return ENOMEM;
    }
    unsafe {
        *addr_in_out = mapping;
    }
    ORBIS_OK
}

/// `sceKernelMunmap`.
///
/// # Safety
/// `addr` must come from [`sce_kernel_map_named_flexible_memory`] with the same `len`.
pub unsafe fn sce_kernel_munmap(addr: *mut c_void, len: u64) -> i32 {
    if addr.is_null() || len == 0 {
        return EINVAL;
    }
    if unsafe { libc::munmap(addr, len as usize) } != 0 {
        return EINVAL;
    }
    ORBIS_OK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_named_mapping_is_writable_and_unmaps() {
        let mut address: *mut c_void = std::ptr::null_mut();
        let result = unsafe {
            sce_kernel_map_named_flexible_memory(&mut address, 8192, 3, 0, c"np_heap".as_ptr())
        };
        assert_eq!(result, ORBIS_OK);
        assert!(!address.is_null());
        unsafe {
            // The page is ours to write and read back.
            std::ptr::write_volatile(address as *mut u8, 0x5A);
            assert_eq!(std::ptr::read_volatile(address as *const u8), 0x5A);
        }
        assert_eq!(unsafe { sce_kernel_munmap(address, 8192) }, ORBIS_OK);
        // Zero length and null addresses are refused.
        assert_eq!(
            unsafe { sce_kernel_munmap(std::ptr::null_mut(), 4096) },
            EINVAL
        );
        let mut null_address: *mut c_void = std::ptr::null_mut();
        assert_eq!(
            unsafe {
                sce_kernel_map_named_flexible_memory(&mut null_address, 0, 0, 0, std::ptr::null())
            },
            EINVAL
        );
    }
}
