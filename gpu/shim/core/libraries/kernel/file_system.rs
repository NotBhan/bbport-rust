//! The stat structure network sockets fill, ported from
//! `legacy/gpu/shim/core/libraries/kernel/file_system.h`.

/// `OrbisKernelTimespec` (`time.h`): seconds and nanoseconds.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisKernelTimespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisKernelStat {
    pub st_dev: u32,
    pub st_ino: u32,
    pub st_mode: u16,
    pub st_nlink: u16,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u32,
    pub st_atim: OrbisKernelTimespec,
    pub st_mtim: OrbisKernelTimespec,
    pub st_ctim: OrbisKernelTimespec,
    pub st_size: i64,
    pub st_blocks: i64,
    pub st_blksize: u32,
    pub st_flags: u32,
    pub st_gen: u32,
    pub st_lspare: i32,
    pub st_birthtim: OrbisKernelTimespec,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn stat_layout_matches_the_c_struct() {
        assert_eq!(size_of::<OrbisKernelTimespec>(), 16);
        assert_eq!(size_of::<OrbisKernelStat>(), 120);
    }
}
