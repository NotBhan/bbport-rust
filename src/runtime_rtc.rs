//! libSceRtc: calendar time, ported from `legacy/src/runtime_rtc.c`.
//!
//! A tick is one microsecond since 0001-01-01 00:00:00 (proleptic Gregorian), the PS4
//! convention; the host clock supplies "now" and the host time zone supplies local time.

use std::ffi::{c_char, CStr};

/// `OrbisRtcDateTime`: six `uint16` fields and a `uint32` microsecond.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DateTime {
    pub year: u16,
    pub month: u16,
    pub day: u16,
    pub hour: u16,
    pub minute: u16,
    pub second: u16,
    pub microsecond: u32,
}

const _: () = assert!(std::mem::size_of::<DateTime>() == 16);

pub const ERR_INVALID_POINTER: i32 = 0x80B5_0002u32 as i32;
pub const ERR_INVALID_VALUE: i32 = 0x80B5_0003u32 as i32;
pub const ERR_INVALID_YEAR: i32 = 0x80B5_0008u32 as i32;
pub const ERR_INVALID_MONTH: i32 = 0x80B5_0009u32 as i32;
pub const ERR_INVALID_DAY: i32 = 0x80B5_000Au32 as i32;
pub const ERR_INVALID_HOUR: i32 = 0x80B5_000Bu32 as i32;
pub const ERR_INVALID_MINUTE: i32 = 0x80B5_000Cu32 as i32;
pub const ERR_INVALID_SECOND: i32 = 0x80B5_000Du32 as i32;
pub const ERR_INVALID_MICROSECOND: i32 = 0x80B5_000Eu32 as i32;

pub const UNIX_EPOCH_TICKS: u64 = 0x00dc_bffe_ff2b_c000;
pub const TICKS_PER_DAY: u64 = 86_400_000_000;
pub const TICK_RESOLUTION: u32 = 1_000_000;

fn leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: i64) -> i64 {
    const DAYS: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if month == 2 && leap(year) {
        29
    } else {
        DAYS[(month - 1).clamp(0, 11) as usize]
    }
}

/// Days since 0001-01-01 (H. Hinnant's `days_from_civil`, shifted epoch).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 306
}

fn civil_from_days(mut z: i64) -> (i64, i64, i64) {
    z += 306;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn validate(value: &DateTime) -> i32 {
    if value.year < 1 || value.year > 9999 {
        return ERR_INVALID_YEAR;
    }
    if value.month < 1 || value.month > 12 {
        return ERR_INVALID_MONTH;
    }
    if value.day < 1
        || i64::from(value.day) > days_in_month(i64::from(value.year), i64::from(value.month))
    {
        return ERR_INVALID_DAY;
    }
    if value.hour > 23 {
        return ERR_INVALID_HOUR;
    }
    if value.minute > 59 {
        return ERR_INVALID_MINUTE;
    }
    if value.second > 59 {
        return ERR_INVALID_SECOND;
    }
    if value.microsecond > 999_999 {
        return ERR_INVALID_MICROSECOND;
    }
    0
}

pub fn to_tick(value: &DateTime) -> u64 {
    days_from_civil(
        i64::from(value.year),
        i64::from(value.month),
        i64::from(value.day),
    ) as u64
        * TICKS_PER_DAY
        + (u64::from(value.hour) * 3600 + u64::from(value.minute) * 60 + u64::from(value.second))
            * 1_000_000
        + u64::from(value.microsecond)
}

pub fn from_tick(tick: u64) -> DateTime {
    let (year, month, day) = civil_from_days((tick / TICKS_PER_DAY) as i64);
    let us = tick % TICKS_PER_DAY;
    DateTime {
        year: year as u16,
        month: month as u16,
        day: day as u16,
        hour: (us / 3_600_000_000) as u16,
        minute: (us / 60_000_000 % 60) as u16,
        second: (us / 1_000_000 % 60) as u16,
        microsecond: (us % 1_000_000) as u32,
    }
}

/// Microseconds since 0001-01-01 for the current host UTC time.
pub fn now_utc() -> u64 {
    let mut ts: libc::timespec = unsafe { std::mem::zeroed() };
    if unsafe { libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts) } != 0 {
        crate::src::runtime::stop("host clock_gettime failed");
    }
    UNIX_EPOCH_TICKS + ts.tv_sec as u64 * 1_000_000 + ts.tv_nsec as u64 / 1000
}

/// Offset of local time from UTC at the given UTC tick, in microseconds.
pub fn local_offset(utc: u64) -> i64 {
    let seconds = ((utc as i64 - UNIX_EPOCH_TICKS as i64) / 1_000_000) as libc::time_t;
    let mut local: libc::tm = unsafe { std::mem::zeroed() };
    unsafe {
        libc::localtime_r(&seconds, &mut local);
    }
    local.tm_gmtoff as i64 * 1_000_000
}

/// Day index where 0 is Sunday (0001-01-01 was a Monday, index 1).
pub fn day_of_week(year: i32, month: i32, day: i32) -> i32 {
    if !(1..=9999).contains(&year) {
        return ERR_INVALID_YEAR;
    }
    if !(1..=12).contains(&month) {
        return ERR_INVALID_MONTH;
    }
    if !(1..=days_in_month(year as i64, month as i64) as i32).contains(&day) {
        return ERR_INVALID_DAY;
    }
    ((days_from_civil(year as i64, month as i64, day as i64) + 1) % 7) as i32
}

/// `"Wed, 25 Sep 2026 12:00:00 +0300"` for the given UTC tick (`None` = now) in local time.
pub fn format_rfc2822_local(utc_tick: Option<u64>) -> String {
    const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let utc = utc_tick.unwrap_or_else(now_utc);
    let offset = local_offset(utc);
    let value = from_tick((utc as i64 + offset) as u64);
    let minutes = offset / 60_000_000;
    let weekday = day_of_week(value.year as i32, value.month as i32, value.day as i32);
    let weekday = if (0..7).contains(&weekday) {
        weekday as usize
    } else {
        0
    };
    let sign = if minutes < 0 { '-' } else { '+' };
    let magnitude = minutes.abs();
    format!(
        "{}, {:02} {} {:04} {:02}:{:02}:{:02} {sign}{:02}{:02}",
        DAYS[weekday],
        value.day,
        MONTHS[(value.month as usize).saturating_sub(1).min(11)],
        value.year,
        value.hour,
        value.minute,
        value.second,
        magnitude / 60,
        magnitude % 60
    )
}

// --- Guest entry points --------------------------------------------------------------------

/// # Safety
/// `value` must be null or a valid `DateTime` in guest memory.
pub unsafe extern "C" fn rtc_current_local(value: *mut DateTime) -> i32 {
    if value.is_null() {
        return ERR_INVALID_POINTER;
    }
    let utc = now_utc();
    value.write(from_tick((utc as i64 + local_offset(utc)) as u64));
    0
}

/// # Safety
/// `value` must be null or a valid `DateTime` in guest memory.
pub unsafe extern "C" fn rtc_current_clock(value: *mut DateTime, minutes: i32) -> i32 {
    if value.is_null() {
        return ERR_INVALID_POINTER;
    }
    value.write(from_tick(
        (now_utc() as i64 + minutes as i64 * 60_000_000) as u64,
    ));
    0
}

/// # Safety
/// `tick` must be null or a valid `u64` in guest memory.
pub unsafe extern "C" fn rtc_current_tick(tick: *mut u64) -> i32 {
    if tick.is_null() {
        return ERR_INVALID_POINTER;
    }
    tick.write(now_utc());
    0
}

/// # Safety
/// Both pointers must be null or valid `u64`s in guest memory.
pub unsafe extern "C" fn rtc_utc_to_local(utc: *const u64, local: *mut u64) -> i32 {
    if utc.is_null() || local.is_null() {
        return ERR_INVALID_POINTER;
    }
    let utc = utc.read();
    local.write((utc as i64 + local_offset(utc)) as u64);
    0
}

/// # Safety
/// Both pointers must be null or valid `u64`s in guest memory.
pub unsafe extern "C" fn rtc_local_to_utc(local: *const u64, utc: *mut u64) -> i32 {
    if local.is_null() || utc.is_null() {
        return ERR_INVALID_POINTER;
    }
    let local = local.read();
    utc.write((local as i64 - local_offset(local)) as u64);
    0
}

/// # Safety
/// `value` and `tick` must be null or valid guest memory.
pub unsafe extern "C" fn rtc_get_tick(value: *const DateTime, tick: *mut u64) -> i32 {
    if value.is_null() || tick.is_null() {
        return ERR_INVALID_POINTER;
    }
    let value = value.read();
    let error = validate(&value);
    if error != 0 {
        return error;
    }
    tick.write(to_tick(&value));
    0
}

/// # Safety
/// `value` and `tick` must be null or valid guest memory.
pub unsafe extern "C" fn rtc_set_tick(value: *mut DateTime, tick: *const u64) -> i32 {
    if value.is_null() || tick.is_null() {
        return ERR_INVALID_POINTER;
    }
    value.write(from_tick(tick.read()));
    0
}

pub extern "C" fn rtc_tick_resolution() -> u32 {
    TICK_RESOLUTION
}

/// # Safety
/// `value` must be null or a valid `DateTime` in guest memory.
pub unsafe extern "C" fn rtc_set_time_t(value: *mut DateTime, seconds: i64) -> i32 {
    if value.is_null() {
        return ERR_INVALID_POINTER;
    }
    if seconds < 0 {
        return ERR_INVALID_VALUE;
    }
    value.write(from_tick(UNIX_EPOCH_TICKS + seconds as u64 * 1_000_000));
    0
}

/// # Safety
/// `value` and `seconds` must be null or valid guest memory.
pub unsafe extern "C" fn rtc_get_time_t(value: *const DateTime, seconds: *mut i64) -> i32 {
    if value.is_null() || seconds.is_null() {
        return ERR_INVALID_POINTER;
    }
    let value = value.read();
    let error = validate(&value);
    if error != 0 {
        return error;
    }
    let tick = to_tick(&value);
    seconds.write(if tick < UNIX_EPOCH_TICKS {
        0
    } else {
        ((tick - UNIX_EPOCH_TICKS) / 1_000_000) as i64
    });
    0
}

/// # Safety
/// `out` must point to at least 32 writable guest bytes; `utc_tick` must be null or a valid
/// `u64` in guest memory.
pub unsafe extern "C" fn rtc_rfc2822_local(out: *mut c_char, utc_tick: *const u64) -> i32 {
    if out.is_null() {
        return ERR_INVALID_POINTER;
    }
    let tick = if utc_tick.is_null() {
        None
    } else {
        Some(utc_tick.read())
    };
    let text = format_rfc2822_local(tick);
    let bytes = text.as_bytes();
    let length = bytes.len().min(31);
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), out as *mut u8, length);
    out.add(length).write(0);
    0
}

/// Export names to function addresses, like the `runtime_rtc_resolve` table.
#[allow(function_casts_as_integer)]
pub fn resolve(name: &str) -> usize {
    match name {
        "sceRtcGetCurrentClockLocalTime" => rtc_current_local as usize,
        "sceRtcGetCurrentClock" => rtc_current_clock as usize,
        "sceRtcGetCurrentTick" | "sceRtcGetCurrentNetworkTick" => rtc_current_tick as usize,
        "sceRtcConvertUtcToLocalTime" => rtc_utc_to_local as usize,
        "sceRtcConvertLocalTimeToUtc" => rtc_local_to_utc as usize,
        "sceRtcGetDayOfWeek" => day_of_week as usize,
        "sceRtcGetTick" => rtc_get_tick as usize,
        "sceRtcSetTick" => rtc_set_tick as usize,
        "sceRtcGetTickResolution" => rtc_tick_resolution as usize,
        "sceRtcSetTime_t" => rtc_set_time_t as usize,
        "sceRtcGetTime_t" => rtc_get_time_t as usize,
        "sceRtcFormatRFC2822LocalTime" => rtc_rfc2822_local as usize,
        _ => 0,
    }
}

/// C ABI entry point the loader calls while the legacy `runtime_rtc.c` is replaced.
///
/// # Safety
/// `name` must be a NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn runtime_rtc_resolve(name: *const c_char) -> usize {
    if name.is_null() {
        return 0;
    }
    match CStr::from_ptr(name).to_str() {
        Ok(nid) => match crate::src::runtime::runtime_symbol(nid) {
            Some(symbol) => resolve(symbol),
            None => 0,
        },
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::sync::Mutex as StdMutex;

    #[cfg(unix)]
    extern "C" {
        fn tzset();
    }

    static TZ_LOCK: StdMutex<()> = StdMutex::new(());

    #[test]
    fn civil_day_math_round_trips() {
        assert_eq!(days_from_civil(1, 1, 1), 0);
        assert_eq!(civil_from_days(0), (1, 1, 1));
        for tick in [
            0u64,
            1,
            TICKS_PER_DAY - 1,
            UNIX_EPOCH_TICKS,
            UNIX_EPOCH_TICKS * 3 / 2,
        ] {
            let value = from_tick(tick);
            assert_eq!(to_tick(&value), tick);
        }
    }

    #[test]
    fn known_weekdays_match_the_calendar() {
        assert_eq!(day_of_week(1, 1, 1), 1); // Monday
        assert_eq!(day_of_week(1970, 1, 1), 4); // Thursday
        assert_eq!(day_of_week(2000, 1, 1), 6); // Saturday
        assert_eq!(day_of_week(2024, 1, 1), 1); // Monday
        assert_eq!(day_of_week(2024, 2, 29), 4); // leap day, Thursday
        assert_eq!(day_of_week(0, 1, 1), ERR_INVALID_YEAR);
        assert_eq!(day_of_week(2023, 2, 29), ERR_INVALID_DAY);
    }

    #[test]
    fn unix_epoch_ticks_are_1970() {
        let value = from_tick(UNIX_EPOCH_TICKS);
        assert_eq!(
            value,
            DateTime {
                year: 1970,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
                microsecond: 0,
            }
        );
    }

    #[test]
    fn guest_entry_points_validate_and_convert() {
        let mut value = DateTime::default();
        assert_eq!(unsafe { rtc_set_time_t(&mut value, 0) }, 0);
        assert_eq!(value.year, 1970);
        let mut seconds = 0i64;
        assert_eq!(unsafe { rtc_get_time_t(&value, &mut seconds) }, 0);
        assert_eq!(seconds, 0);
        assert_eq!(unsafe { rtc_set_time_t(&mut value, -1) }, ERR_INVALID_VALUE);

        let mut tick = 0u64;
        assert_eq!(unsafe { rtc_get_tick(&value, &mut tick) }, 0);
        assert_eq!(tick, UNIX_EPOCH_TICKS);
        assert_eq!(unsafe { rtc_set_tick(&mut value, &tick) }, 0);
        assert_eq!(
            unsafe { rtc_get_tick(std::ptr::null(), &mut tick) },
            ERR_INVALID_POINTER
        );
        assert_eq!(
            unsafe { rtc_current_tick(std::ptr::null_mut()) },
            ERR_INVALID_POINTER
        );
        assert_eq!(rtc_tick_resolution(), 1_000_000);
    }

    #[cfg(unix)]
    #[test]
    fn rfc2822_formats_the_unix_epoch_in_utc() {
        let _guard = TZ_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let previous = std::env::var("TZ").ok();
        std::env::set_var("TZ", "UTC");
        unsafe { tzset() };
        let text = format_rfc2822_local(Some(UNIX_EPOCH_TICKS));
        assert_eq!(text, "Thu, 01 Jan 1970 00:00:00 +0000");
        let mut buffer = [0i8; 64];
        assert_eq!(
            unsafe { rtc_rfc2822_local(buffer.as_mut_ptr(), &UNIX_EPOCH_TICKS) },
            0
        );
        let written = unsafe { CStr::from_ptr(buffer.as_ptr()) }.to_str().unwrap();
        assert_eq!(written, text);
        match previous {
            Some(value) => std::env::set_var("TZ", value),
            None => std::env::remove_var("TZ"),
        }
        unsafe { tzset() };
    }

    #[test]
    fn resolves_export_names_by_nid() {
        assert_eq!(resolve("sceRtcGetCurrentTick"), rtc_current_tick as usize);
        assert_eq!(resolve("sceRtcNope"), 0);
        // Guest NIDs go through the generated symbol table, like the C lookup.
        let nid = CString::new("nope#q#q").unwrap();
        assert_eq!(unsafe { runtime_rtc_resolve(nid.as_ptr()) }, 0);
        assert_eq!(unsafe { runtime_rtc_resolve(std::ptr::null()) }, 0);
    }
}
