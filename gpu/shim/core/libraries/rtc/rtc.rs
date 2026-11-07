//! libSceRtc surface, ported from `legacy/gpu/shim/core/libraries/rtc/rtc.h`.
//! The tick arithmetic lives in `src/runtime_rtc.rs`; this module keeps the guest types and the
//! library registration.

use crate::gpu::shim::core::libraries::libs::{SymbolType, SymbolsResolver};

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

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisRtcTick {
    pub tick: u64,
}

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

/// Every export shadPS4 registers with `LIB_FUNCTION` in the RTC library.
pub const EXPORT_NAMES: &[&str] = &[
    "sceRtcCheckValid",
    "sceRtcCompareTick",
    "sceRtcConvertLocalTimeToUtc",
    "sceRtcConvertUtcToLocalTime",
    "sceRtcEnd",
    "sceRtcFormatRFC2822",
    "sceRtcFormatRFC2822LocalTime",
    "sceRtcFormatRFC3339",
    "sceRtcFormatRFC3339LocalTime",
    "sceRtcFormatRFC3339Precise",
    "sceRtcFormatRFC3339PreciseLocalTime",
    "sceRtcGetCurrentAdNetworkTick",
    "sceRtcGetCurrentClock",
    "sceRtcGetCurrentClockLocalTime",
    "sceRtcGetCurrentDebugNetworkTick",
    "sceRtcGetCurrentNetworkTick",
    "sceRtcGetCurrentRawNetworkTick",
    "sceRtcGetCurrentTick",
    "sceRtcGetDayOfWeek",
    "sceRtcGetDaysInMonth",
    "sceRtcGetDosTime",
    "sceRtcGetTick",
    "sceRtcGetTickResolution",
    "sceRtcGetTime_t",
    "sceRtcGetWin32FileTime",
    "sceRtcInit",
    "sceRtcIsLeapYear",
    "sceRtcParseDateTime",
    "sceRtcParseRFC3339",
    "sceRtcSetConf",
    "sceRtcSetCurrentAdNetworkTick",
    "sceRtcSetCurrentDebugNetworkTick",
    "sceRtcSetCurrentNetworkTick",
    "sceRtcSetCurrentTick",
    "sceRtcSetDosTime",
    "sceRtcSetTick",
    "sceRtcSetTime_t",
    "sceRtcSetWin32FileTime",
    "sceRtcTickAddDays",
    "sceRtcTickAddHours",
    "sceRtcTickAddMicroseconds",
    "sceRtcTickAddMinutes",
    "sceRtcTickAddMonths",
    "sceRtcTickAddSeconds",
    "sceRtcTickAddTicks",
    "sceRtcTickAddWeeks",
    "sceRtcTickAddYears",
];

/// `RegisterLib`: `lookup` maps an export name to its NID and address in the runtime.
pub fn register_lib<F>(resolver: &mut SymbolsResolver, mut lookup: F)
where
    F: FnMut(&str) -> Option<(&'static str, u64)>,
{
    for name in EXPORT_NAMES {
        if let Some((nid, address)) = lookup(name) {
            resolver.add_symbol(nid, "libSceRtc", "rtc", SymbolType::Function, address);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registration_covers_every_export() {
        let seen = std::cell::Cell::new(0);
        let mut resolver = SymbolsResolver::default();
        register_lib(&mut resolver, |name| {
            seen.set(seen.get() + 1);
            (name == "sceRtcGetCurrentTick").then_some(("nid#tick", 0x1000))
        });
        assert_eq!(seen.get(), EXPORT_NAMES.len());
        assert_eq!(resolver.count(), 1);
        let symbol = resolver.resolve("nid#tick").unwrap();
        assert_eq!(symbol.library, "libSceRtc");
        assert_eq!(symbol.module, "rtc");
        assert_eq!(symbol.address, 0x1000);
    }

    #[test]
    fn unknown_exports_are_skipped() {
        let mut resolver = SymbolsResolver::default();
        register_lib(&mut resolver, |_| None);
        assert_eq!(resolver.count(), 0);
    }
}
