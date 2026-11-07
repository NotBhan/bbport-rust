//! A bit array with range queries, ported from `legacy/gpu/shadps4/common/bit_array.h`. The
//! C++ has AVX2 special cases for whole-word runs; the port's word loops do the same work
//! without the intrinsics.

/// A half-open range of bits, as the C++'s `BitArray::Range`.
pub type Range = (usize, usize);

/// `Common::BitArray<N>`: `N` bits, which the header requires to be a multiple of 64.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitArray<const N: usize> {
    data: Vec<u64>,
}

impl<const N: usize> Default for BitArray<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> BitArray<N> {
    /// How many 64-bit words the array holds.
    pub const WORD_COUNT: usize = N / 64;
    const BITS_PER_WORD: usize = 64;

    /// `BitArray()`: every bit clear.
    pub fn new() -> Self {
        assert!(
            N.is_multiple_of(64),
            "BitArray size must be a multiple of 64 bits."
        );
        Self {
            data: vec![0; Self::WORD_COUNT],
        }
    }

    /// `BitArray(const BitArray& other, size_t start, size_t end)`: a masked copy that keeps
    /// only the bits of `other` inside the range.
    pub fn copy_range(other: &Self, start: usize, end: usize) -> Self {
        let mut result = Self::new();
        if start >= end || end > N {
            return result;
        }
        let first_word = start / 64;
        let last_word = (end - 1) / 64;
        let start_mask = !0u64 << (start % 64);
        let end_bit = (end - 1) % 64;
        let end_mask = if end_bit == Self::BITS_PER_WORD - 1 {
            !0u64
        } else {
            (1u64 << (end_bit + 1)) - 1
        };
        if first_word == last_word {
            result.data[first_word] = other.data[first_word] & (start_mask & end_mask);
        } else {
            result.data[first_word] = other.data[first_word] & start_mask;
            result.data[first_word + 1..last_word]
                .copy_from_slice(&other.data[first_word + 1..last_word]);
            result.data[last_word] = other.data[last_word] & end_mask;
        }
        result
    }

    /// `Size()`.
    pub const fn size(&self) -> usize {
        N
    }

    /// `Set(idx)`.
    pub fn set(&mut self, idx: usize) {
        self.data[idx / 64] |= 1u64 << (idx % 64);
    }

    /// `Unset(idx)`.
    pub fn unset(&mut self, idx: usize) {
        self.data[idx / 64] &= !(1u64 << (idx % 64));
    }

    /// `Get(idx)`.
    pub fn get(&self, idx: usize) -> bool {
        self.data[idx / 64] & (1u64 << (idx % 64)) != 0
    }

    /// `SetRange(start, end)`: every bit in the half-open range.
    pub fn set_range(&mut self, start: usize, end: usize) {
        if start >= end || end > N {
            return;
        }
        let first_word = start / 64;
        let last_word = (end - 1) / 64;
        let start_mask = !0u64 << (start % 64);
        let end_bit = (end - 1) % 64;
        let end_mask = if end_bit == Self::BITS_PER_WORD - 1 {
            !0u64
        } else {
            (1u64 << (end_bit + 1)) - 1
        };
        if first_word == last_word {
            self.data[first_word] |= start_mask & end_mask;
        } else {
            self.data[first_word] |= start_mask;
            for word in &mut self.data[first_word + 1..last_word] {
                *word = !0u64;
            }
            self.data[last_word] |= end_mask;
        }
    }

    /// `UnsetRange(start, end)`.
    pub fn unset_range(&mut self, start: usize, end: usize) {
        if start >= end || end > N {
            return;
        }
        let first_word = start / 64;
        let last_word = (end - 1) / 64;
        let start_mask = (1u64 << (start % 64)) - 1;
        let end_bit = (end - 1) % 64;
        let end_mask = if end_bit == Self::BITS_PER_WORD - 1 {
            0
        } else {
            !((1u64 << (end_bit + 1)) - 1)
        };
        if first_word == last_word {
            self.data[first_word] &= start_mask | end_mask;
        } else {
            self.data[first_word] &= start_mask;
            for word in &mut self.data[first_word + 1..last_word] {
                *word = 0;
            }
            self.data[last_word] &= end_mask;
        }
    }

    /// `SetRange(const Range&)`.
    pub fn set_bit_range(&mut self, range: Range) {
        self.set_range(range.0, range.1);
    }

    /// `UnsetRange(const Range&)`.
    pub fn unset_bit_range(&mut self, range: Range) {
        self.unset_range(range.0, range.1);
    }

    /// `Clear()`.
    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    /// `Fill()`.
    pub fn fill(&mut self) {
        self.data.fill(!0u64);
    }

    /// `None()`.
    pub fn none(&self) -> bool {
        self.data.iter().all(|word| *word == 0)
    }

    /// `Any()`.
    pub fn any(&self) -> bool {
        !self.none()
    }

    /// `AnyInRange(start, end)`.
    pub fn any_in_range(&self, start: usize, end: usize) -> bool {
        let first_word = start / 64;
        let last_word = (end - 1) / 64;
        let start_mask = !0u64 << (start % 64);
        let end_bit = (end - 1) % 64;
        let end_mask = if end_bit == Self::BITS_PER_WORD - 1 {
            !0u64
        } else {
            (1u64 << (end_bit + 1)) - 1
        };
        if first_word == last_word {
            return self.data[first_word] & (start_mask & end_mask) != 0;
        }
        if self.data[first_word] & start_mask != 0 {
            return true;
        }
        if self.data[first_word + 1..last_word]
            .iter()
            .any(|word| *word != 0)
        {
            return true;
        }
        self.data[last_word] & end_mask != 0
    }

    /// `FirstRangeFrom(start)`: the first run of set bits at or after `start`.
    pub fn first_range_from(&self, start: usize) -> Range {
        if start >= N {
            return (N, N);
        }
        let start_word = start / 64;
        let masked_first = self.data[start_word] & (!0u64 << (start % 64));
        if masked_first != 0 {
            return self.word_bits(start_word, masked_first);
        }
        for word in start_word + 1..Self::WORD_COUNT {
            if self.data[word] != 0 {
                return self.word_bits(word, self.data[word]);
            }
        }
        (N, N)
    }

    /// `FirstRange()`.
    pub fn first_range(&self) -> Range {
        self.first_range_from(0)
    }

    /// `LastRangeFrom(end)`: the last run of set bits ending at or before `end`.
    pub fn last_range_from(&self, end: usize) -> Range {
        if end == 0 {
            return (0, 0);
        }
        let end = end.min(N);
        let end_word = (end - 1) / 64 + 1;
        let end_bit = (end - 1) % 64;
        let mut masked_last = self.data[end_word - 1];
        if end_bit < Self::BITS_PER_WORD - 1 {
            masked_last &= (1u64 << (end_bit + 1)) - 1;
        }
        if masked_last != 0 {
            return self.word_bits_last(end_word, masked_last);
        }
        for word in (1..end_word).rev() {
            if self.data[word - 1] != 0 {
                return self.word_bits_last(word, self.data[word - 1]);
            }
        }
        (0, 0)
    }

    /// `LastRange()`.
    pub fn last_range(&self) -> Range {
        self.last_range_from(N)
    }

    /// The C++'s iterator: each range in turn.
    pub fn ranges(&self) -> Ranges<'_, N> {
        Ranges {
            array: self,
            next_start: 0,
        }
    }

    /// The run of set bits starting at `word`, extending over following all-ones words.
    fn word_bits(&self, index: usize, word: u64) -> Range {
        let empty_bits = word.trailing_zeros() as usize;
        let ones_count = (word >> empty_bits).trailing_ones() as usize;
        let start_bit = index * 64 + empty_bits;
        if ones_count + empty_bits < Self::BITS_PER_WORD {
            (start_bit, start_bit + ones_count)
        } else {
            (start_bit, self.find_end_bit(index + 1))
        }
    }

    fn find_end_bit(&self, mut word: usize) -> usize {
        while word < Self::WORD_COUNT {
            if self.data[word] != !0u64 {
                return word * 64 + self.data[word].trailing_ones() as usize;
            }
            word += 1;
        }
        N
    }

    /// The run of set bits ending at `word`, extending back over preceding all-ones words.
    fn word_bits_last(&self, index: usize, word: u64) -> Range {
        let empty_bits = word.leading_zeros() as usize;
        let ones_count = (word << empty_bits).leading_ones() as usize;
        let end_bit = index * 64 - empty_bits;
        if empty_bits + ones_count < Self::BITS_PER_WORD {
            (end_bit - ones_count, end_bit)
        } else {
            (self.find_start_bit(index - 1), end_bit)
        }
    }

    fn find_start_bit(&self, mut word: usize) -> usize {
        while word > 0 {
            if self.data[word - 1] != !0u64 {
                return word * 64 - self.data[word - 1].leading_ones() as usize;
            }
            word -= 1;
        }
        0
    }

    /// `operator|=`.
    pub fn union_with(&mut self, other: &Self) {
        for (word, other) in self.data.iter_mut().zip(&other.data) {
            *word |= *other;
        }
    }

    /// `operator&=`.
    pub fn intersect_with(&mut self, other: &Self) {
        for (word, other) in self.data.iter_mut().zip(&other.data) {
            *word &= *other;
        }
    }

    /// `operator^=`.
    pub fn toggle_with(&mut self, other: &Self) {
        for (word, other) in self.data.iter_mut().zip(&other.data) {
            *word ^= *other;
        }
    }

    /// `operator~`.
    pub fn inverted(&self) -> Self {
        let mut result = self.clone();
        for word in &mut result.data {
            *word = !*word;
        }
        result
    }
}

/// The C++'s `BitArray::Iterator`: walks the ranges of an array.
pub struct Ranges<'a, const N: usize> {
    array: &'a BitArray<N>,
    next_start: usize,
}

impl<const N: usize> Iterator for Ranges<'_, N> {
    type Item = Range;

    fn next(&mut self) -> Option<Range> {
        let range = self.array.first_range_from(self.next_start);
        if range == (N, N) {
            return None;
        }
        self.next_start = range.1;
        Some(range)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const N: usize = 256;

    #[test]
    fn single_bits_wrap_the_words() {
        let mut bits = BitArray::<N>::new();
        assert!(bits.none() && !bits.any());
        bits.set(0);
        bits.set(63);
        bits.set(64);
        bits.set(255);
        assert!(bits.get(0) && bits.get(63) && bits.get(64) && bits.get(255));
        assert!(!bits.get(1) && !bits.get(254));
        bits.unset(64);
        assert!(!bits.get(64));
        assert_eq!(bits.size(), N);
        bits.clear();
        assert!(bits.none());
        bits.fill();
        assert!(bits.any() && bits.get(128));
    }

    #[test]
    fn ranges_set_and_unset_across_words() {
        let mut bits = BitArray::<N>::new();
        bits.set_range(60, 130);
        assert_eq!(bits.first_range(), (60, 130));
        assert_eq!(bits.last_range(), (60, 130));
        assert!(bits.any_in_range(0, 61));
        assert!(!bits.any_in_range(0, 60));
        assert!(bits.any_in_range(129, 200));
        assert!(!bits.any_in_range(130, 256));
        bits.unset_range(64, 128);
        assert_eq!(
            bits.ranges().collect::<Vec<_>>(),
            vec![(60, 64), (128, 130)]
        );
        // A range covering only part of an end word leaves the rest alone.
        bits.set(2);
        bits.set_range(0, 4);
        assert!(bits.get(0) && bits.get(1) && bits.get(2) && bits.get(3));
        bits.unset_bit_range((0, 2));
        assert!(!bits.get(0) && !bits.get(1) && bits.get(2) && bits.get(3));
        bits.unset_range(10, 5);
        assert_eq!(bits.last_range(), (128, 130));
    }

    #[test]
    fn the_first_and_last_ranges_stop_at_the_ends() {
        let mut bits = BitArray::<N>::new();
        assert_eq!(bits.first_range(), (N, N));
        assert_eq!(bits.last_range(), (0, 0));
        bits.set_range(64, 128);
        bits.set_range(192, 200);
        assert_eq!(bits.first_range_from(0), (64, 128));
        assert_eq!(bits.first_range_from(64), (64, 128));
        assert_eq!(
            bits.first_range_from(65),
            (65, 128),
            "starting inside the run"
        );
        assert_eq!(bits.first_range_from(128), (192, 200));
        assert_eq!(bits.first_range_from(200), (N, N));
        assert_eq!(bits.last_range_from(N), (192, 200));
        assert_eq!(bits.last_range_from(200), (192, 200));
        assert_eq!(
            bits.last_range_from(199),
            (192, 199),
            "ending inside the run"
        );
        assert_eq!(bits.last_range_from(128), (64, 128));
        assert_eq!(bits.last_range_from(64), (0, 0));
        assert_eq!(
            bits.ranges().collect::<Vec<_>>(),
            vec![(64, 128), (192, 200)]
        );
        // A run that covers whole words extends over them.
        let mut bits = BitArray::<N>::new();
        bits.set_range(64, 256);
        assert_eq!(bits.first_range(), (64, 256));
        assert_eq!(bits.last_range(), (64, 256));
    }

    #[test]
    fn copying_a_range_masks_both_ends_and_the_set_operations_compose() {
        let mut source = BitArray::<N>::new();
        source.fill();
        let copy = BitArray::<N>::copy_range(&source, 60, 130);
        assert_eq!(copy.first_range(), (60, 130));
        assert_eq!(
            BitArray::<N>::copy_range(&source, 10, 10),
            BitArray::<N>::new()
        );
        assert_eq!(
            BitArray::<N>::copy_range(&source, 10, N + 1),
            BitArray::<N>::new()
        );

        let mut union = BitArray::<N>::new();
        union.set(1);
        union.union_with(&copy);
        assert_eq!(union.ranges().collect::<Vec<_>>(), vec![(1, 2), (60, 130)]);
        let mut intersection = copy.clone();
        let mut mask = BitArray::<N>::new();
        mask.set_range(64, 80);
        intersection.intersect_with(&mask);
        assert_eq!(intersection.first_range(), (64, 80));
        let mut toggled = copy.clone();
        toggled.toggle_with(&mask);
        assert_eq!(
            toggled.ranges().collect::<Vec<_>>(),
            vec![(60, 64), (80, 130)]
        );
        assert_eq!(
            BitArray::<N>::new().inverted(),
            BitArray::<N>::new().inverted()
        );
        assert!(BitArray::<N>::new().inverted().get(0));
        assert_eq!(copy.clone().inverted().inverted(), copy);
    }
}
