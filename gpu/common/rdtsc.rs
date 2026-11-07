//! The counter the emulator times guest work with, ported from
//! `legacy/gpu/shadps4/common/rdtsc.h` and `rdtsc.cpp`. The C++ measures the counter against
//! the wall clock and rounds the result; the port does the same arithmetic, with `CLOCK_REALTIME`
//! standing in for the `clock_gettime` call.

use std::time::Duration;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
use std::time::Instant;

pub use crate::gpu::common::uint128::{get_fixed_point64_factor, multiply_high};

/// The C++'s `Common::GetFixedPoint64Factor`, under the name that file's callers use.
pub use crate::gpu::common::uint128::get_fixed_point64_factor as fixed_point64_factor;

/// `Common::FencedRDTSC`: the counter read, fenced so it cannot be reordered past the
/// surrounding code.
#[cfg(target_arch = "x86_64")]
pub fn fenced_rdtsc() -> u64 {
    // The header's asm does `lfence; rdtsc; lfence`; the intrinsics are the same instruction
    // sequence, with the second fence keeping later reads behind this one.
    unsafe {
        core::arch::x86_64::_mm_lfence();
        let value = core::arch::x86_64::_rdtsc();
        core::arch::x86_64::_mm_lfence();
        value
    }
}

/// The arm64 form: `isb; mrs cntvct_el0; isb`.
#[cfg(target_arch = "aarch64")]
pub fn fenced_rdtsc() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!("isb", "mrs {}, cntvct_el0", "isb", out(reg) value, options(nomem, nostack));
    }
    value
}

/// The header's `#error` for any other target: the port reads the monotonic clock in
/// nanoseconds instead, which keeps the guest's clock monotonic but not counter-scaled.
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub fn fenced_rdtsc() -> u64 {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_nanos() as u64
}

/// `static constexpr size_t SecondToNanoseconds`.
const SECOND_TO_NANOSECONDS: u64 = 1_000_000_000;

/// `GetTimeNs`: the wall clock in nanoseconds, which the C++ reads from `CLOCK_REALTIME`.
fn get_time_ns() -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(elapsed) => elapsed.as_nanos() as u64,
        Err(_) => 0,
    }
}

/// `RoundToNearest`: the C++ rounds to the closest multiple of `nearest`, up on a tie.
fn round_to_nearest(value: u64, nearest: u64) -> u64 {
    if nearest == 0 {
        return value;
    }
    let modulus = value % nearest;
    if modulus >= nearest / 2 {
        value - modulus + nearest
    } else {
        value - modulus
    }
}

/// `Common::EstimateRDTSCFrequency`: the counter's frequency in hertz, measured over a tenth of
/// a second, rounded to the nearest hundred thousand.
pub fn estimate_rdtsc_frequency() -> u64 {
    // Discard the first result measuring the rdtsc.
    fenced_rdtsc();
    std::thread::sleep(Duration::from_millis(1));
    fenced_rdtsc();

    // Get the current time.
    let start_time = get_time_ns();
    let tsc_start = fenced_rdtsc();
    // Wait for 100 milliseconds.
    std::thread::sleep(Duration::from_millis(100));
    let end_time = get_time_ns();
    let tsc_end = fenced_rdtsc();
    // Calculate differences.
    let tsc_diff = tsc_end.wrapping_sub(tsc_start);
    let elapsed = end_time.saturating_sub(start_time).max(1);
    let tsc_freq = crate::gpu::common::uint128::multiply_and_divide64(
        tsc_diff,
        SECOND_TO_NANOSECONDS,
        elapsed,
    );
    round_to_nearest(tsc_freq, 100_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_counter_moves_forward() {
        let first = fenced_rdtsc();
        let second = fenced_rdtsc();
        assert!(second >= first, "a counter never runs backwards");
    }

    #[test]
    fn the_fixed_point_helpers_scale_by_multiplication() {
        // A three-gigacycle counter: the ns factor is 2^64 / 3, truncated to 64 bits.
        let ns_factor = fixed_point64_factor(1_000_000_000, 3_000_000_000);
        assert_eq!(ns_factor, 0x5555_5555_5555_5555);
        // Three ticks are just under a nanosecond, so the truncation drops them.
        assert_eq!(multiply_high(3, ns_factor), 0);
        assert_eq!(multiply_high(3_000_000_001, ns_factor), 1_000_000_000);
        assert_eq!(multiply_high(3_000_000_000, ns_factor), 999_999_999);
        assert_eq!(fixed_point64_factor(1, 2), 1 << 63);
        assert_eq!(fixed_point64_factor(0, 3), 0);
        assert_eq!(
            fixed_point64_factor(5, 0),
            0,
            "a zero denominator has no factor"
        );
        assert_eq!(multiply_high(0, u64::MAX), 0);
    }

    #[test]
    fn the_rounding_goes_to_the_nearest_multiple() {
        assert_eq!(round_to_nearest(149_999, 100_000), 100_000);
        assert_eq!(round_to_nearest(150_000, 100_000), 200_000, "a tie goes up");
        assert_eq!(round_to_nearest(3_000_012_345, 100_000), 3_000_000_000);
        assert_eq!(round_to_nearest(0, 100_000), 0);
        assert_eq!(round_to_nearest(7, 0), 7);
        assert_eq!(round_to_nearest(3_000_050_000, 100_000), 3_000_100_000);
    }

    #[test]
    fn the_frequency_is_measured_in_hertz() {
        let frequency = estimate_rdtsc_frequency();
        assert!(frequency > 1_000, "a measured frequency: {frequency}");
        assert_eq!(frequency % 100_000, 0, "the measurement is rounded");
        #[cfg(target_arch = "x86_64")]
        assert!(
            frequency < 100_000_000_000,
            "a sane counter frequency: {frequency}"
        );
    }
}
