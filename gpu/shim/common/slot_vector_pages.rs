//! SlotVector page reservation, ported from `legacy/gpu/shim/common/slot_vector_pages.cpp`.
//! Reserved pages cost no memory until committed, so a SlotVector keeps every entry at a fixed
//! address.

use std::ffi::c_void;

/// Reserve `size` bytes of inaccessible address space.
pub fn slot_vector_reserve_pages(size: usize) -> *mut c_void {
    #[cfg(unix)]
    {
        let pointer = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_NONE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
                -1,
                0,
            )
        };
        if pointer == libc::MAP_FAILED {
            std::ptr::null_mut()
        } else {
            pointer
        }
    }
    #[cfg(not(unix))]
    {
        let _ = size;
        std::ptr::null_mut()
    }
}

/// Commit reserved pages as read/write at the same address.
///
/// # Safety
/// `address` must be the base returned by [`slot_vector_reserve_pages`].
pub unsafe fn slot_vector_commit_pages(address: *mut c_void, size: usize) {
    #[cfg(unix)]
    {
        let result = unsafe {
            libc::mmap(
                address,
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_FIXED,
                -1,
                0,
            )
        };
        assert_eq!(result, address, "SlotVector: committing memory failed");
    }
    #[cfg(not(unix))]
    {
        let _ = (address, size);
    }
}

/// # Safety
/// `base` must be the address returned by [`slot_vector_reserve_pages`].
pub unsafe fn slot_vector_release_pages(base: *mut c_void, size: usize) {
    if base.is_null() {
        return;
    }
    #[cfg(unix)]
    unsafe {
        libc::munmap(base, size);
    }
    #[cfg(not(unix))]
    {
        let _ = size;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn reserved_pages_can_be_committed_written_and_released() {
        let size = 4096 * 2;
        let base = slot_vector_reserve_pages(size);
        assert!(!base.is_null());
        unsafe {
            slot_vector_commit_pages(base, size);
            (base as *mut u8).write(0x5a);
            assert_eq!((base as *const u8).read(), 0x5a);
            slot_vector_release_pages(base, size);
        }
    }
}
