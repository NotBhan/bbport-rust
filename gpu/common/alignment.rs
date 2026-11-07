//! Alignment helpers, ported from `legacy/gpu/shadps4/common/alignment.h`.

macro_rules! define_alignment {
    ($t:ty, $up:ident, $down:ident) => {
        /// `Common::AlignUp`: rounds up to the next multiple, leaving multiples alone.
        pub const fn $up(value: $t, size: usize) -> $t {
            let size = size as $t;
            let remainder = value % size;
            if remainder == 0 {
                value
            } else {
                value - remainder + size
            }
        }

        /// `Common::AlignDown`: the last multiple at or below the value.
        pub const fn $down(value: $t, size: usize) -> $t {
            value - value % size as $t
        }
    };
}

define_alignment!(u32, align_up_u32, align_down_u32);
define_alignment!(u64, align_up_u64, align_down_u64);
define_alignment!(usize, align_up_usize, align_down_usize);

/// `Common::AlignUpPow2`, for a power-of-two size.
pub const fn align_up_pow2(value: u64, size: u64) -> u64 {
    (value + (size - 1)) & !(size - 1)
}

/// `Common::AlignDownPow2`.
pub const fn align_down_pow2(value: u64, size: u64) -> u64 {
    value & !(size - 1)
}

/// `Common::IsAligned`: the header masks against `alignment - 1`, so the alignment is a power
/// of two here as well.
pub const fn is_aligned(value: u64, alignment: usize) -> bool {
    value & (alignment as u64 - 1) == 0
}

/// `Common::Is16KBAligned`.
pub const fn is_16kb_aligned(value: u64) -> bool {
    value & 0x3FFF == 0
}

/// `Common::Is64KBAligned`.
pub const fn is_64kb_aligned(value: u64) -> bool {
    value & 0xFFFF == 0
}

/// `Common::Is2MBAligned`.
pub const fn is_2mb_aligned(value: u64) -> bool {
    value & 0x1F_FFFF == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_up_leaves_multiples_and_moves_the_rest() {
        assert_eq!(align_up_u32(0, 16), 0);
        assert_eq!(align_up_u32(16, 16), 16);
        assert_eq!(align_up_u32(17, 16), 32);
        assert_eq!(align_up_usize(1, 4096), 4096);
        assert_eq!(align_up_u64(4096, 4096), 4096);
        assert_eq!(align_up_u64(4097, 4096), 8192);
        assert_eq!(align_down_u32(31, 16), 16);
        assert_eq!(align_down_u32(32, 16), 32);
        assert_eq!(align_down_u64(0, 8), 0);
        assert_eq!(align_down_usize(4095, 4096), 0);
    }

    #[test]
    fn the_power_of_two_forms_agree_with_the_general_ones() {
        for value in [0u64, 1, 63, 64, 65, 4095, 4096, 1 << 20] {
            assert_eq!(align_up_pow2(value, 64), align_up_u64(value, 64), "{value}");
            assert_eq!(
                align_down_pow2(value, 64),
                align_down_u64(value, 64),
                "{value}"
            );
        }
        assert_eq!(align_up_pow2(1 << 20, 1 << 20), 1 << 20);
        assert_eq!(align_down_pow2(1 << 20 | 0xFFF, 1 << 20), 1 << 20);
    }

    #[test]
    fn the_guest_sizes_are_recognised() {
        assert!(is_aligned(4096, 4096));
        assert!(!is_aligned(4097, 4096));
        assert!(is_16kb_aligned(0x4000));
        assert!(!is_16kb_aligned(0x4001));
        assert!(is_64kb_aligned(0x1_0000));
        assert!(is_2mb_aligned(0x20_0000));
        assert!(!is_2mb_aligned(0x20_0001));
        assert!(is_16kb_aligned(0), "zero is aligned to everything");
    }
}
