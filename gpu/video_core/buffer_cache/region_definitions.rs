//! Buffer-cache tracking sizes, ported from
//! `legacy/gpu/shadps4/video_core/buffer_cache/region_definitions.h`.

use crate::gpu::common::bit_array::BitArray;

/// `VideoCore::TRACKER_PAGE_BITS`.
pub const TRACKER_PAGE_BITS: u32 = 12;
/// `VideoCore::TRACKER_BYTES_PER_PAGE`.
pub const TRACKER_BYTES_PER_PAGE: u64 = 1 << TRACKER_PAGE_BITS;
/// `VideoCore::TRACKER_HIGHER_PAGE_BITS`: a tracked region covers 4 MB.
pub const TRACKER_HIGHER_PAGE_BITS: u32 = 22;
/// `VideoCore::TRACKER_HIGHER_PAGE_SIZE`.
pub const TRACKER_HIGHER_PAGE_SIZE: u64 = 1 << TRACKER_HIGHER_PAGE_BITS;
/// `VideoCore::TRACKER_HIGHER_PAGE_MASK`.
pub const TRACKER_HIGHER_PAGE_MASK: u64 = TRACKER_HIGHER_PAGE_SIZE - 1;
/// `VideoCore::NUM_PAGES_PER_REGION`.
pub const NUM_PAGES_PER_REGION: u64 = TRACKER_HIGHER_PAGE_SIZE / TRACKER_BYTES_PER_PAGE;

/// `VideoCore::Type`: which side of the cache the region belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Cpu,
    Gpu,
}

/// `VideoCore::RegionBits`: one bit per page of a region.
pub type RegionBits = BitArray<{ NUM_PAGES_PER_REGION as usize }>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_region_is_a_megabyte_of_pages() {
        assert_eq!(TRACKER_HIGHER_PAGE_SIZE, 4 << 20);
        assert_eq!(TRACKER_BYTES_PER_PAGE, 4096);
        assert_eq!(NUM_PAGES_PER_REGION, 1024);
        assert_eq!(TRACKER_HIGHER_PAGE_MASK, (4 << 20) - 1);
        // A page's offset inside a region is the low bits of its address.
        assert_eq!(0x0012_3456u64 & TRACKER_HIGHER_PAGE_MASK, 0x12_3456);
        assert_eq!(
            0x0040_0000u64 & TRACKER_HIGHER_PAGE_MASK,
            0,
            "a region start"
        );
        assert_eq!(TRACKER_HIGHER_PAGE_SIZE & TRACKER_HIGHER_PAGE_MASK, 0);
    }

    #[test]
    fn region_bits_track_pages_independently() {
        let mut bits = RegionBits::new();
        assert!(bits.none());
        bits.set(0);
        bits.set(1023);
        assert_eq!(bits.first_range(), (0, 1));
        assert_eq!(bits.last_range(), (1023, 1024));
        assert!(bits.get(0) && bits.get(1023));
        assert!(!bits.get(1) && !bits.get(1022));
        bits.unset(1023);
        assert_eq!(bits.first_range(), (0, 1));
        assert!(!bits.none());
        bits.unset(0);
        assert!(bits.none());
        bits.set_range(128, 256);
        assert_eq!(bits.first_range(), (128, 256));
        assert_eq!(bits.size(), NUM_PAGES_PER_REGION as usize);
        assert_eq!(Type::Cpu, Type::Cpu);
        assert_ne!(Type::Cpu, Type::Gpu);
    }
}
