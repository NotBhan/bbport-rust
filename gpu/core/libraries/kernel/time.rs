//! Kernel clock types, ported from `shadps4/core/libraries/kernel/time.h`.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// `OrbisKernelTimeval`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisKernelTimeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

/// `OrbisKernelTimezone`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisKernelTimezone {
    pub tz_minuteswest: i32,
    pub tz_dsttime: i32,
}

/// `OrbisKernelTimespec`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisKernelTimespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

impl OrbisKernelTimespec {
    /// `TimePoint`.
    pub fn time_point(&self) -> SystemTime {
        if self.tv_sec >= 0 {
            UNIX_EPOCH
                + Duration::new(
                    self.tv_sec as u64,
                    self.tv_nsec.clamp(0, 999_999_999) as u32,
                )
        } else {
            UNIX_EPOCH
                - Duration::new(
                    (-self.tv_sec) as u64,
                    self.tv_nsec.clamp(0, 999_999_999) as u32,
                )
        }
    }
}

/// `OrbisTimesec`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisTimesec {
    pub t: i64,
    pub west_sec: u32,
    pub dst_sec: u32,
}

pub const ORBIS_CLOCK_REALTIME: i32 = 0;
pub const ORBIS_CLOCK_VIRTUAL: i32 = 1;
pub const ORBIS_CLOCK_PROF: i32 = 2;
pub const ORBIS_CLOCK_MONOTONIC: i32 = 4;
pub const ORBIS_CLOCK_UPTIME: i32 = 5;
pub const ORBIS_CLOCK_UPTIME_PRECISE: i32 = 7;
pub const ORBIS_CLOCK_UPTIME_FAST: i32 = 8;
pub const ORBIS_CLOCK_REALTIME_PRECISE: i32 = 9;
pub const ORBIS_CLOCK_REALTIME_FAST: i32 = 10;
pub const ORBIS_CLOCK_MONOTONIC_PRECISE: i32 = 11;
pub const ORBIS_CLOCK_MONOTONIC_FAST: i32 = 12;
pub const ORBIS_CLOCK_SECOND: i32 = 13;
pub const ORBIS_CLOCK_THREAD_CPUTIME_ID: i32 = 14;
pub const ORBIS_CLOCK_PROCTIME: i32 = 15;
pub const ORBIS_CLOCK_EXT_NETWORK: i32 = 16;
pub const ORBIS_CLOCK_EXT_DEBUG_NETWORK: i32 = 17;
pub const ORBIS_CLOCK_EXT_AD_NETWORK: i32 = 18;
pub const ORBIS_CLOCK_EXT_RAW_NETWORK: i32 = 19;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_structs_keep_their_layout() {
        assert_eq!(core::mem::size_of::<OrbisKernelTimeval>(), 16);
        assert_eq!(core::mem::size_of::<OrbisKernelTimezone>(), 8);
        assert_eq!(core::mem::size_of::<OrbisKernelTimespec>(), 16);
        assert_eq!(core::mem::size_of::<OrbisTimesec>(), 16);
        assert_eq!(ORBIS_CLOCK_REALTIME, 0);
        assert_eq!(ORBIS_CLOCK_EXT_RAW_NETWORK, 19);
    }

    #[test]
    fn a_timespec_converts_to_a_system_time() {
        let timespec = OrbisKernelTimespec {
            tv_sec: 1_700_000_000,
            tv_nsec: 500_000_000,
        };
        let system_time = timespec.time_point();
        let elapsed = system_time
            .duration_since(UNIX_EPOCH)
            .expect("after the epoch");
        assert_eq!(elapsed.as_secs(), 1_700_000_000);
        assert_eq!(elapsed.subsec_nanos(), 500_000_000);
    }
}
