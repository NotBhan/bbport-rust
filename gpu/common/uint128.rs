//! 128-bit helpers, ported from `legacy/gpu/shadps4/common/uint128.h`. The C++ splits into an
//! MSVC intrinsic path and a portable one; Rust's `u128` is the portable answer to both, so the
//! port keeps the arithmetic each branch computes — including the fallback formulas, which are
//! what the C++ uses when neither intrinsics nor `__int128` are available.

/// `Common::MultiplyAndDivide64`: `a * b / d`, with the product taken in 128 bits.
///
/// The C++'s non-intrinsic branch computes it as
/// `(a/d)*b + (a%d)*(b/d) + (a%d)*(b%d)/d`, which is the same value.
pub fn multiply_and_divide64(a: u64, b: u64, d: u64) -> u64 {
    if d == 0 {
        return 0;
    }
    ((a as u128 * b as u128) / d as u128) as u64
}

/// `Common::Multiply64Into128`.
pub fn multiply64_into128(a: u64, b: u64) -> u128 {
    a as u128 * b as u128
}

/// `Common::GetFixedPoint64Factor`: the 64-bit fixed-point factor for `numerator / divisor`.
pub fn get_fixed_point64_factor(numerator: u64, divisor: u64) -> u64 {
    if divisor == 0 {
        return 0;
    }
    (((numerator as u128) << 64) / divisor as u128) as u64
}

/// `Common::MultiplyHigh`: the high word of the 128-bit product.
pub fn multiply_high(a: u64, b: u64) -> u64 {
    ((a as u128 * b as u128) >> 64) as u64
}

/// `Common::Divide128On32`: the quotient and remainder of a 128-bit dividend by a 32-bit
/// divisor. The C++ walks the two halves with a carry fix-up, which the port reproduces.
pub fn divide128_on32(dividend: u128, divisor: u32) -> (u64, u64) {
    if divisor == 0 {
        return (0, 0);
    }
    let low = dividend as u64;
    let high = (dividend >> 64) as u64;
    let divisor64 = divisor as u64;
    let mut remainder = low % divisor64;
    let mut accum = low / divisor64;
    if high != 0 {
        // `dividend[1] / divisor` would overflow, so the C++ splits it in 32-bit segments.
        let first_segment = (high % divisor64) << 32;
        accum += (first_segment / divisor64) << 32;
        let second_segment = (first_segment % divisor64) << 32;
        accum += second_segment / divisor64;
        remainder += second_segment % divisor64;
        if remainder >= divisor64 {
            accum += 1;
            remainder -= divisor64;
        }
    }
    (accum, remainder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_product_and_quotient_keeps_the_high_bits() {
        assert_eq!(multiply_and_divide64(1_000, 3, 2), 1_500);
        // (2^63) * 2 / 2^32 stays exact, which a 64-bit product could not hold.
        let a = 1u64 << 63;
        assert_eq!(multiply_and_divide64(a, 2, 1u64 << 32), 1u64 << 32);
        assert_eq!(
            multiply_and_divide64(7, 5, 0),
            0,
            "a zero divisor has no quotient"
        );
        assert_eq!(
            multiply_and_divide64(u64::MAX, u64::MAX, u64::MAX),
            u64::MAX
        );
        assert_eq!(
            multiply64_into128(u64::MAX, u64::MAX),
            (u64::MAX as u128) * (u64::MAX as u128)
        );
    }

    #[test]
    fn the_fixed_point_factor_and_high_multiply_agree_with_a_wide_product() {
        // One half is exactly 2^63.
        assert_eq!(get_fixed_point64_factor(1, 2), 1 << 63);
        assert_eq!(get_fixed_point64_factor(0, 3), 0);
        assert_eq!(get_fixed_point64_factor(5, 0), 0);
        // Three gigahertz: the nanosecond factor is 2^64 / 3, truncated.
        let ns_factor = get_fixed_point64_factor(1_000_000_000, 3_000_000_000);
        assert_eq!(ns_factor, 0x5555_5555_5555_5555);
        assert_eq!(multiply_high(3_000_000_000, ns_factor), 999_999_999);
        assert_eq!(multiply_high(0, u64::MAX), 0);
        let product = multiply64_into128(1_234_567_891_u64, 987_654_321);
        assert_eq!(
            multiply_high(1_234_567_891, 987_654_321),
            (product >> 64) as u64
        );
    }

    #[test]
    fn the_segmented_division_matches_a_wide_division() {
        // Where the quotient fits in 64 bits the result is exact.
        for (dividend, divisor) in [
            (0u128, 7u32),
            (12345, 97),
            ((1u128 << 64) + 5, 3),
            ((1u128 << 72), 0xFFFF_FFFF),
            (u64::MAX as u128, 1),
            ((u64::MAX as u128) << 8, 1u32 << 8),
        ] {
            let (quotient, remainder) = divide128_on32(dividend, divisor);
            assert_eq!(quotient as u128, dividend / divisor as u128, "{dividend}");
            assert_eq!(remainder as u128, dividend % divisor as u128, "{dividend}");
        }
        // A quotient wider than 64 bits is not representable, and the C++ ignores
        // `dividend[1] / divisor` for that reason: the port hands back the low bits, with the
        // remainder still exact.
        for (dividend, divisor) in [
            ((1u128 << 96) | (12345 << 32), 0xFFFF_FFFFu32),
            (u128::MAX, 0xFFFF_FFFF),
        ] {
            let (quotient, remainder) = divide128_on32(dividend, divisor);
            assert_eq!(
                quotient as u128,
                (dividend / divisor as u128) & u64::MAX as u128,
                "{dividend}"
            );
            assert_eq!(remainder as u128, dividend % divisor as u128, "{dividend}");
        }
        assert_eq!(divide128_on32(1234, 0), (0, 0));
        // A dividend that fits in 64 bits takes the short path.
        assert_eq!(divide128_on32(100, 7), (14, 2));
    }
}
