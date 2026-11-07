//! A two-level page table, ported from
//! `legacy/gpu/shadps4/video_core/multi_level_page_table.h`.
//!
//! The C++ allocates L1 pages from an object pool; the port allocates them on demand and keeps
//! them in a `Vec<Box<[Entry]>>` indexed by the L1 slot.

/// `VideoCore::MultiLevelPageTable`'s traits parameter.
pub trait PageTableTraits {
    type Entry: Default + Clone;

    const ADDRESS_SPACE_BITS: usize;
    const FIRST_LEVEL_BITS: usize;
    const PAGE_BITS: usize;
}

/// `VideoCore::MultiLevelPageTable`.
pub struct MultiLevelPageTable<T: PageTableTraits> {
    first_level_map: Vec<Option<usize>>,
    pages: Vec<Box<[T::Entry]>>,
}

impl<T: PageTableTraits> MultiLevelPageTable<T> {
    /// `FirstLevelShift = AddressSpaceBits - FirstLevelBits`.
    pub const FIRST_LEVEL_SHIFT: usize = T::ADDRESS_SPACE_BITS - T::FIRST_LEVEL_BITS;
    /// `SecondLevelBits = FirstLevelShift - PageBits`.
    pub const SECOND_LEVEL_BITS: usize = Self::FIRST_LEVEL_SHIFT - T::PAGE_BITS;
    /// `NumEntriesPerL1Page`.
    pub const NUM_ENTRIES_PER_L1_PAGE: usize = 1usize << Self::SECOND_LEVEL_BITS;

    /// `MultiLevelPageTable()`.
    pub fn new() -> Self {
        Self {
            first_level_map: vec![None; 1usize << T::FIRST_LEVEL_BITS],
            pages: Vec::new(),
        }
    }

    fn split(&self, page: usize) -> (usize, usize) {
        (
            page >> Self::SECOND_LEVEL_BITS,
            page & (Self::NUM_ENTRIES_PER_L1_PAGE - 1),
        )
    }

    fn allocate(&mut self, l1_page: usize) {
        if self.first_level_map[l1_page].is_none() {
            let index = self.pages.len();
            self.pages
                .push(vec![T::Entry::default(); Self::NUM_ENTRIES_PER_L1_PAGE].into_boxed_slice());
            self.first_level_map[l1_page] = Some(index);
        }
    }

    /// `find(page)`.
    pub fn find(&self, page: usize) -> Option<&T::Entry> {
        let (l1_page, l2_page) = self.split(page);
        let index = self.first_level_map[l1_page]?;
        Some(&self.pages[index][l2_page])
    }

    /// The mutable `find(page)`.
    pub fn find_mut(&mut self, page: usize) -> Option<&mut T::Entry> {
        let (l1_page, l2_page) = self.split(page);
        let index = self.first_level_map[l1_page]?;
        Some(&mut self.pages[index][l2_page])
    }

    /// `operator[](page)`, which allocates the L1 page on first use.
    pub fn entry(&mut self, page: usize) -> &T::Entry {
        let (l1_page, l2_page) = self.split(page);
        self.allocate(l1_page);
        let index = self.first_level_map[l1_page].expect("the L1 page was allocated");
        &self.pages[index][l2_page]
    }

    /// The mutable `operator[](page)`.
    pub fn entry_mut(&mut self, page: usize) -> &mut T::Entry {
        let (l1_page, l2_page) = self.split(page);
        self.allocate(l1_page);
        let index = self.first_level_map[l1_page].expect("the L1 page was allocated");
        &mut self.pages[index][l2_page]
    }
}

impl<T: PageTableTraits> Default for MultiLevelPageTable<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Traits;

    impl PageTableTraits for Traits {
        type Entry = u64;

        const ADDRESS_SPACE_BITS: usize = 40;
        const FIRST_LEVEL_BITS: usize = 10;
        const PAGE_BITS: usize = 12;
    }

    type Table = MultiLevelPageTable<Traits>;

    #[test]
    fn the_derived_shifts_match_the_header() {
        assert_eq!(Table::FIRST_LEVEL_SHIFT, 30);
        assert_eq!(Table::SECOND_LEVEL_BITS, 18);
        assert_eq!(Table::NUM_ENTRIES_PER_L1_PAGE, 1 << 18);
    }

    #[test]
    fn a_page_is_allocated_on_first_access() {
        let mut table = Table::new();
        assert!(table.find(7).is_none());
        *table.entry_mut(7) = 0x1234;
        assert_eq!(table.find(7), Some(&0x1234));
        assert_eq!(table.find(8), Some(&0));
        // A different L1 page is still absent.
        assert!(table.find(7 + (1 << 18)).is_none());
    }

    #[test]
    fn pages_in_the_same_l1_slot_share_the_allocation() {
        let mut table = Table::new();
        let first = 3usize << 18;
        *table.entry_mut(first) = 1;
        *table.entry_mut(first + 5) = 2;
        assert_eq!(table.pages.len(), 1);
        assert_eq!(table.find(first), Some(&1));
        assert_eq!(table.find(first + 5), Some(&2));
    }
}
