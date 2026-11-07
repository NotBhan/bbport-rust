//! RTC types and constants, ported from `bbnet/include/core/libraries/rtc/rtc.h`.

pub const ORBIS_RTC_DAYOFWEEK_SUNDAY: i32 = 0;
pub const ORBIS_RTC_DAYOFWEEK_MONDAY: i32 = 1;
pub const ORBIS_RTC_DAYOFWEEK_TUESDAY: i32 = 2;
pub const ORBIS_RTC_DAYOFWEEK_WEDNESDAY: i32 = 3;
pub const ORBIS_RTC_DAYOFWEEK_THURSDAY: i32 = 4;
pub const ORBIS_RTC_DAYOFWEEK_FRIDAY: i32 = 5;
pub const ORBIS_RTC_DAYOFWEEK_SATURDAY: i32 = 6;

pub const ORBIS_RTC_CLOCK_ID_NETWORK: i32 = 0x10;
pub const ORBIS_RTC_CLOCK_ID_DEBUG_NETWORK: i32 = 0x11;
pub const ORBIS_RTC_CLOCK_ID_AD_NETWORK: i32 = 0x12;

pub const UNIX_EPOCH_TICKS: i64 = 0x00dc_bffe_ff2b_c000;
pub const WIN32_FILETIME_EPOCH_TICKS: i64 = 0x00b3_6168_b6a5_8000;

/// `OrbisRtcTick`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisRtcTick {
    pub tick: u64,
}

/// `OrbisRtcDateTime`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisRtcDateTime {
    pub year: u16,
    pub month: u16,
    pub day: u16,
    pub hour: u16,
    pub minute: u16,
    pub second: u16,
    pub microsecond: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_constants_and_struct_layouts_match() {
        assert_eq!(ORBIS_RTC_DAYOFWEEK_SATURDAY, 6);
        assert_eq!(UNIX_EPOCH_TICKS, 0xdcbffeff2bc000);
        assert_eq!(WIN32_FILETIME_EPOCH_TICKS, 0xb36168b6a58000);
        assert_eq!(core::mem::size_of::<OrbisRtcTick>(), 8);
        assert_eq!(core::mem::size_of::<OrbisRtcDateTime>(), 16);
    }
}
