//! The base type vocabulary, ported from `legacy/gpu/shadps4/common/types.h`.

/// The Rust spelling of the guest's calling convention (`PS4_SYSV_ABI`): the loader declares
/// guest entry points as `extern "sysv64"`.
pub const PS4_SYSV_ABI: &str = "sysv64";

/// `VAddr`: a guest virtual address.
pub type VAddr = usize;
/// `PAddr`: a guest physical address.
pub type PAddr = usize;

/// `u128`: the C++ carries the wide type as a pair of 64-bit words, not as a primitive.
pub type U128 = [u64; 2];

/// A kilobyte, as the `_KB` literal suffix defines it.
pub const fn kb(value: u64) -> u64 {
    1024 * value
}

/// `_MB`.
pub const fn mb(value: u64) -> u64 {
    1024 * kb(value)
}

/// `_GB`.
pub const fn gb(value: u64) -> u64 {
    1024 * mb(value)
}

/// `1_KB`.
pub const KIB: u64 = 1024;
/// `1_MB`.
pub const MIB: u64 = kb(1024);
/// `1_GB`.
pub const GIB: u64 = mb(1024);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wide_type_is_a_pair_of_words() {
        assert_eq!(std::mem::size_of::<U128>(), 16);
        let value: U128 = [1, 0];
        assert_eq!(value[0], 1);
    }

    #[test]
    fn the_size_suffixes_follow_the_header() {
        assert_eq!(kb(1), 1024);
        assert_eq!(kb(4), 4096);
        assert_eq!(mb(4), 4 << 20, "the renderer's region size");
        assert_eq!(gb(1), 1024 * 1024 * 1024);
        assert_eq!(MIB, mb(1));
        assert_eq!(GIB, gb(1));
        assert_eq!(MIB / KIB, 1024);
        assert_eq!(GIB / MIB, 1024);
    }

    #[test]
    fn addresses_are_pointer_sized() {
        assert_eq!(
            std::mem::size_of::<VAddr>(),
            std::mem::size_of::<*const u8>()
        );
        assert_eq!(std::mem::size_of::<PAddr>(), std::mem::size_of::<VAddr>());
        assert_eq!(PS4_SYSV_ABI, "sysv64");
    }
}
