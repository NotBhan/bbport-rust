//! Fault context accessors, ported from `legacy/gpu/shadps4/common/signal_context.h`.
//!
//! The C++ reads the Linux x86-64 `ucontext_t` the signal handler receives. The port keeps
//! the same layout knowledge: `uc_mcontext.gregs[16]` is RIP, and the XMM registers live in
//! `uc_mcontext.fpregs`'s `_fpstate`, whose `_xmm[16]` array starts at byte 160.

/// Bytes from the `ucontext_t` base to `uc_mcontext.gregs`.
pub const GREGS_OFFSET: usize = 0x28;
/// `REG_RIP` in `gregs`.
pub const RIP_INDEX: usize = 16;
/// `REG_ERR` in `gregs`: the page-fault error descriptor.
pub const ERR_INDEX: usize = 19;
/// Bytes from the `ucontext_t` base to `sigcontext.fpregs`.
pub const FPREGS_OFFSET: usize = 0xE0;
/// Bytes from the `_fpstate` base to `_xmm[0]`.
pub const XMM_OFFSET: usize = 160;
/// One XMM register holds 16 bytes.
pub const XMM_STRIDE: usize = 16;

/// Reads `gregs[index]` from a `ucontext_t`.
fn greg(context: *mut std::ffi::c_void, index: usize) -> Option<u64> {
    if context.is_null() || index > 22 {
        return None;
    }
    let base = context as *const u8;
    let mut bytes = [0u8; 8];
    unsafe {
        std::ptr::copy_nonoverlapping(
            base.add(GREGS_OFFSET + index * 8),
            bytes.as_mut_ptr(),
            bytes.len(),
        );
    }
    Some(u64::from_ne_bytes(bytes))
}

/// `Common::GetRip`.
pub fn get_rip(context: *mut std::ffi::c_void) -> Option<u64> {
    greg(context, RIP_INDEX)
}

/// `Common::IncrementRip`.
pub fn increment_rip(context: *mut std::ffi::c_void, length: u64) -> bool {
    if context.is_null() {
        return false;
    }
    let Some(rip) = get_rip(context) else {
        return false;
    };
    let base = context as *mut u8;
    let address = (rip + length).to_ne_bytes();
    unsafe {
        std::ptr::copy_nonoverlapping(
            address.as_ptr(),
            base.add(GREGS_OFFSET + RIP_INDEX * 8),
            address.len(),
        );
    }
    true
}

/// `Common::GetXmmPointer`: the address of `_xmm[index]` inside the frame's `_fpstate`.
pub fn get_xmm_pointer(context: *mut std::ffi::c_void, index: u8) -> Option<*mut u8> {
    if context.is_null() || usize::from(index) >= 16 {
        return None;
    }
    let base = context as *const u8;
    let mut pointer_bytes = [0u8; 8];
    unsafe {
        std::ptr::copy_nonoverlapping(
            base.add(FPREGS_OFFSET),
            pointer_bytes.as_mut_ptr(),
            pointer_bytes.len(),
        );
    }
    let fpstate = usize::from_ne_bytes(pointer_bytes) as *mut u8;
    if fpstate.is_null() {
        return None;
    }
    Some(unsafe { fpstate.add(XMM_OFFSET + usize::from(index) * XMM_STRIDE) })
}

/// `Common::IsWriteError`: `REG_ERR`'s write bit, set when the fault was a write.
pub fn is_write_error(context: *mut std::ffi::c_void) -> bool {
    let Some(descriptor) = greg(context, ERR_INDEX) else {
        return false;
    };
    descriptor & 0x2 != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A context the size of the C layout: 0x28 + 23 gregs of 8 bytes, plus room for more.
    fn context() -> Vec<u8> {
        vec![0u8; 0x28 + 23 * 8 + 256]
    }

    #[test]
    fn the_rip_round_trips_and_increments() {
        let mut buffer = context();
        let pointer = buffer.as_mut_ptr() as *mut std::ffi::c_void;
        let rip = 0x7f00_1234_5678u64;
        buffer[GREGS_OFFSET + RIP_INDEX * 8..GREGS_OFFSET + RIP_INDEX * 8 + 8]
            .copy_from_slice(&rip.to_ne_bytes());
        assert_eq!(get_rip(pointer), Some(rip));
        assert!(increment_rip(pointer, 4));
        assert_eq!(get_rip(pointer), Some(rip + 4));
        assert_eq!(get_rip(std::ptr::null_mut()), None);
        assert!(!increment_rip(std::ptr::null_mut(), 1));
    }

    #[test]
    fn the_xmm_pointer_follows_the_fpstate_and_rejects_bad_indices() {
        let mut buffer = context();
        let pointer = buffer.as_mut_ptr() as *mut std::ffi::c_void;
        // A separate _fpstate block the context points at.
        let mut fpstate = vec![0u8; XMM_OFFSET + 16 * XMM_STRIDE];
        fpstate[XMM_OFFSET + 2 * XMM_STRIDE] = 0xAB;
        let fpstate_pointer = fpstate.as_mut_ptr() as usize;
        buffer[FPREGS_OFFSET..FPREGS_OFFSET + 8].copy_from_slice(&fpstate_pointer.to_ne_bytes());
        let xmm2 = get_xmm_pointer(pointer, 2).unwrap();
        assert_eq!(unsafe { *xmm2 }, 0xAB);
        assert!(get_xmm_pointer(pointer, 16).is_none());
    }

    #[test]
    fn the_write_bit_comes_from_the_error_descriptor() {
        let mut buffer = context();
        let pointer = buffer.as_mut_ptr() as *mut std::ffi::c_void;
        assert!(!is_write_error(pointer));
        buffer[GREGS_OFFSET + ERR_INDEX * 8..GREGS_OFFSET + ERR_INDEX * 8 + 8]
            .copy_from_slice(&2u64.to_ne_bytes());
        assert!(is_write_error(pointer));
    }
}
