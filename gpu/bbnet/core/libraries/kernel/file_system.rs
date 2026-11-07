//! The guest stat structure, ported from
//! `legacy/gpu/bbnet/include/core/libraries/kernel/file_system.h`. The bbnet copy of this
//! header is identical to the shim's — same fields, same 120-byte layout — so the port keeps
//! the path and re-exports the already-ported structure rather than declaring it twice.

pub use crate::gpu::shim::core::libraries::kernel::file_system::{
    OrbisKernelStat, OrbisKernelTimespec,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn the_reexported_layout_is_the_one_sockets_fill() {
        assert_eq!(size_of::<OrbisKernelTimespec>(), 16);
        assert_eq!(size_of::<OrbisKernelStat>(), 120);
        // The header's trailing bitfields are zero-width: `16 - sizeof(timespec)` is zero.
        let stat = OrbisKernelStat {
            st_size: 0x1234,
            ..Default::default()
        };
        assert_eq!(stat.st_size, 0x1234);
        assert_eq!(stat.st_atim.tv_sec, 0);
    }
}
