//! Hash combining, ported from `legacy/gpu/shadps4/common/hash.h`.

/// `HashCombine`: the 64-bit variant.
pub const fn hash_combine_u64(seed: u64, hash: u64) -> u64 {
    seed ^ hash
        .wrapping_add(0x9e37_79b9)
        .wrapping_add(seed << 12)
        .wrapping_add(seed >> 4)
}

/// `HashCombine`: the 32-bit variant (different shifts than the 64-bit one).
pub const fn hash_combine_u32(seed: u32, hash: u32) -> u32 {
    seed ^ hash
        .wrapping_add(0x9e37_79b9)
        .wrapping_add(seed << 6)
        .wrapping_add(seed >> 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_variants_use_their_own_shifts() {
        assert_eq!(hash_combine_u64(0, 0), 0x9e37_79b9);
        assert_eq!(hash_combine_u64(1, 0), 1 ^ (0x9e37_79b9 + (1 << 12)));
        assert_eq!(
            hash_combine_u32(0, 0),
            0x9e37_79b9,
            "the 32-bit mix wraps to the low half"
        );
        assert_eq!(
            hash_combine_u32(1, 0),
            1 ^ 0x9e37_79b9u32.wrapping_add(1 << 6)
        );
        // Combining is order-sensitive and never panics on extreme inputs.
        assert_ne!(hash_combine_u64(7, 9), hash_combine_u64(9, 7));
        let _ = hash_combine_u32(u32::MAX, u32::MAX);
    }
}
