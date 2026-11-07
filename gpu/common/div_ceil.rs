//! Ceiled integer division, ported from `legacy/gpu/shadps4/common/div_ceil.h`.

macro_rules! define_div_ceil {
    ($n:ty, $ceil:ident, $ceil_log2:ident) => {
        /// `Common::DivCeil`: rounds up, and leaves zero alone.
        pub const fn $ceil(number: $n, divisor: $n) -> $n {
            if divisor == 0 {
                return 0;
            }
            number.div_ceil(divisor)
        }

        /// `Common::DivCeilLog2`: rounds up to a power-of-two alignment.
        pub const fn $ceil_log2(value: $n, alignment_log2: u32) -> $n {
            ((value + (1 as $n).wrapping_shl(alignment_log2) - 1) >> alignment_log2) as $n
        }
    };
}

define_div_ceil!(u32, div_ceil_u32, div_ceil_log2_u32);
define_div_ceil!(u64, div_ceil_u64, div_ceil_log2_u64);
define_div_ceil!(usize, div_ceil_usize, div_ceil_log2_usize);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn division_rounds_up_and_exact_multiples_stay() {
        assert_eq!(div_ceil_u32(0, 4), 0);
        assert_eq!(div_ceil_u32(1, 4), 1);
        assert_eq!(div_ceil_u32(4, 4), 1);
        assert_eq!(div_ceil_u32(5, 4), 2);
        assert_eq!(div_ceil_u64(1 << 20, 4096), 256);
        assert_eq!(div_ceil_usize(0, 1), 0);
        assert_eq!(div_ceil_usize(7, 3), 3);
    }

    #[test]
    fn the_log2_form_aligns_to_powers_of_two() {
        assert_eq!(div_ceil_log2_u32(0, 12), 0);
        assert_eq!(div_ceil_log2_u32(1, 12), 1);
        assert_eq!(div_ceil_log2_u32(4096, 12), 1);
        assert_eq!(div_ceil_log2_u32(4097, 12), 2);
        assert_eq!(
            div_ceil_log2_u64(1, 0),
            1,
            "a zero alignment changes nothing"
        );
        assert_eq!(div_ceil_log2_usize(1025, 10), 2);
    }
}
