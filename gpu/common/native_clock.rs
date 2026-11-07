//! The guest's time base, ported from `legacy/gpu/shadps4/common/native_clock.h` and its
//! `.cpp`. The C++ keeps fixed-point factors and multiplies the counter reading by them, so
//! the port does the same arithmetic rather than dividing after the fact.

use crate::gpu::common::rdtsc::{
    estimate_rdtsc_frequency, fenced_rdtsc, fixed_point64_factor, multiply_high,
};

/// `Common::NativeClock`.
#[derive(Debug)]
pub struct NativeClock {
    rdtsc_frequency: u64,
    ns_rdtsc_factor: u64,
    us_rdtsc_factor: u64,
    ms_rdtsc_factor: u64,
}

impl Default for NativeClock {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeClock {
    /// The C++ constructor: measure the counter and build the three factors from it.
    pub fn new() -> Self {
        let rdtsc_frequency = estimate_rdtsc_frequency().max(1);
        Self {
            rdtsc_frequency,
            ns_rdtsc_factor: fixed_point64_factor(1_000_000_000, rdtsc_frequency),
            us_rdtsc_factor: fixed_point64_factor(1_000_000, rdtsc_frequency),
            ms_rdtsc_factor: fixed_point64_factor(1_000, rdtsc_frequency),
        }
    }

    /// `NativeClock::GetTscFrequency`.
    pub fn get_tsc_frequency(&self) -> u64 {
        self.rdtsc_frequency
    }

    /// `NativeClock::GetUptime`: the fenced counter reading.
    pub fn get_uptime(&self) -> u64 {
        fenced_rdtsc()
    }

    /// `NativeClock::GetTimeNS`.
    pub fn get_time_ns(&self, base_ptc: u64) -> u64 {
        multiply_high(
            self.get_uptime().wrapping_sub(base_ptc),
            self.ns_rdtsc_factor,
        )
    }

    /// `NativeClock::GetTimeUS`.
    pub fn get_time_us(&self, base_ptc: u64) -> u64 {
        multiply_high(
            self.get_uptime().wrapping_sub(base_ptc),
            self.us_rdtsc_factor,
        )
    }

    /// `NativeClock::GetTimeMS`.
    pub fn get_time_ms(&self, base_ptc: u64) -> u64 {
        multiply_high(
            self.get_uptime().wrapping_sub(base_ptc),
            self.ms_rdtsc_factor,
        )
    }

    pub fn ns_rdtsc_factor(&self) -> u64 {
        self.ns_rdtsc_factor
    }

    pub fn us_rdtsc_factor(&self) -> u64 {
        self.us_rdtsc_factor
    }

    pub fn ms_rdtsc_factor(&self) -> u64 {
        self.ms_rdtsc_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clock_builds_its_factors_from_the_measured_frequency() {
        let clock = NativeClock::new();
        let frequency = clock.get_tsc_frequency();
        assert!(frequency > 1_000, "a measured frequency: {frequency}");
        assert_eq!(
            clock.ns_rdtsc_factor(),
            fixed_point64_factor(1_000_000_000, frequency)
        );
        assert_eq!(
            clock.us_rdtsc_factor(),
            fixed_point64_factor(1_000_000, frequency)
        );
        assert_eq!(
            clock.ms_rdtsc_factor(),
            fixed_point64_factor(1_000, frequency)
        );
        // `GetFixedPoint64Factor` returns a u64, so the nanosecond factor only survives when
        // the counter is at least gigahertz — the C++'s arithmetic wraps below that and the
        // port wraps with it. The equalities above hold either way.
        if frequency >= 1_000_000_000 {
            assert!(clock.ns_rdtsc_factor() >= clock.us_rdtsc_factor());
            assert!(clock.us_rdtsc_factor() >= clock.ms_rdtsc_factor());
        }
    }

    #[test]
    fn the_elapsed_time_scales_the_counter_reading() {
        let clock = NativeClock::new();
        // Each call reads the counter itself, so each is measured from its own base.
        let ns = clock.get_time_ns(clock.get_uptime());
        assert!(ns < 1_000_000_000, "no second passes during a test: {ns}");
        let us = clock.get_time_us(clock.get_uptime());
        assert!(us < 1_000_000, "no minute passes during a test: {us}");
        let ms = clock.get_time_ms(clock.get_uptime());
        assert!(ms < 1_000, "no second passes during a test: {ms}");
        // A base ahead of the counter wraps, as the C++'s unsigned subtraction does, so the
        // reading is the counter's own uptime rather than zero.
        let wrapped = clock.get_time_ns(u64::MAX);
        let from_zero = clock.get_time_ns(0);
        assert!(wrapped > 0);
        assert!(
            wrapped > from_zero / 2,
            "a wrapped base still measures the machine's uptime: {wrapped} vs {from_zero}"
        );
    }
}
