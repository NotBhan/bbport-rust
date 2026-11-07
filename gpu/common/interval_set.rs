//! A sorted interval list, ported from `legacy/gpu/shadps4/common/interval_set.h`.

/// The interval a list stores. The C++ uses a CRTP base plus a concept; the port uses a trait
/// with the same two hooks.
pub trait IntervalType: Copy {
    fn start(&self) -> u64;
    fn end(&self) -> u64;
    fn set_end(&mut self, end: u64);

    /// `CanMergeWith`.
    fn can_merge_with(&self, _other: &Self) -> bool {
        true
    }

    /// `SubRange`.
    fn sub_range(&self, start: u64, end: u64) -> Self;
}

/// `Common::Interval`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Interval {
    pub start: u64,
    pub end: u64,
}

impl Interval {
    pub const fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }

    /// `CanMergeWith`: always true.
    pub const fn can_merge_with(&self, _other: &Interval) -> bool {
        true
    }

    /// `SubRange`.
    pub const fn sub_range(&self, start: u64, end: u64) -> Interval {
        Interval { start, end }
    }
}

impl IntervalType for Interval {
    fn start(&self) -> u64 {
        self.start
    }

    fn end(&self) -> u64 {
        self.end
    }

    fn set_end(&mut self, end: u64) {
        self.end = end;
    }

    fn can_merge_with(&self, other: &Self) -> bool {
        Interval::can_merge_with(self, other)
    }

    fn sub_range(&self, start: u64, end: u64) -> Self {
        Interval::sub_range(self, start, end)
    }
}

/// `Common::IntervalList`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntervalList<IV: IntervalType = Interval> {
    intervals: Vec<IV>,
}

impl<IV: IntervalType> IntervalList<IV> {
    /// `IntervalList()`.
    pub fn new() -> Self {
        Self {
            intervals: Vec::new(),
        }
    }

    /// `begin()`/`end()`.
    pub fn iter(&self) -> impl Iterator<Item = &IV> {
        self.intervals.iter()
    }

    /// `Clear`.
    pub fn clear(&mut self) {
        self.intervals.clear();
    }

    /// `Empty`.
    pub fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    /// `Size`.
    pub fn size(&self) -> usize {
        self.intervals.len()
    }

    /// `Reserve`.
    pub fn reserve(&mut self, count: usize) {
        self.intervals.reserve(count);
    }

    /// `Add`: inserts a range, splitting any interval it overwrites.
    pub fn add(&mut self, value: IV) {
        let (start, end) = (value.start(), value.end());
        if start >= end {
            return;
        }
        let (first, last) = self.overlap_run(start, end);
        let mut out = Vec::with_capacity(3);
        if first != last {
            if self.intervals[first].start() < start {
                out.push(self.intervals[first].sub_range(self.intervals[first].start(), start));
            }
            out.push(value);
            let previous = self.intervals[last - 1];
            if previous.end() > end {
                out.push(previous.sub_range(end, previous.end()));
            }
        } else {
            out.push(value);
        }
        self.splice(first, last, &out);
        self.coalesce(first, out.len());
    }

    /// `Subtract`: removes a range, splitting the intervals it cuts.
    pub fn subtract(&mut self, start: u64, end: u64) {
        if start >= end {
            return;
        }
        let first = self
            .intervals
            .partition_point(|interval| interval.end() <= start);
        let mut last = first;
        while last < self.intervals.len() && self.intervals[last].start() < end {
            last += 1;
        }
        if first == last {
            return;
        }
        let mut out = Vec::with_capacity(2);
        if self.intervals[first].start() < start {
            out.push(self.intervals[first].sub_range(self.intervals[first].start(), start));
        }
        let previous = self.intervals[last - 1];
        if previous.end() > end {
            out.push(previous.sub_range(end, previous.end()));
        }
        self.splice(first, last, &out);
    }

    /// `Find`: the interval containing the address.
    pub fn find(&self, address: u64) -> Option<&IV> {
        let index = self
            .intervals
            .partition_point(|interval| interval.start() <= address);
        if index == 0 {
            return None;
        }
        let interval = &self.intervals[index - 1];
        (interval.start() <= address && address < interval.end()).then_some(interval)
    }

    /// `Contains(addr)`.
    pub fn contains_addr(&self, address: u64) -> bool {
        self.find(address).is_some()
    }

    /// `Contains(start, end)`.
    pub fn contains(&self, start: u64, end: u64) -> bool {
        if start >= end {
            return false;
        }
        let mut index = self
            .intervals
            .partition_point(|interval| interval.end() <= start);
        let mut current = start;
        while index < self.intervals.len()
            && self.intervals[index].start() <= current
            && current < end
        {
            current = self.intervals[index].end();
            index += 1;
        }
        current >= end
    }

    /// `Overlaps`.
    pub fn overlaps(&self, start: u64, end: u64) -> bool {
        if start >= end {
            return false;
        }
        let index = self
            .intervals
            .partition_point(|interval| interval.end() <= start);
        index < self.intervals.len() && self.intervals[index].start() < end
    }

    /// `ForEachGap`: calls `func` for each uncovered range inside the query.
    pub fn for_each_gap(&self, start: u64, end: u64, mut func: impl FnMut(u64, u64)) {
        if start >= end {
            return;
        }
        let mut index = self
            .intervals
            .partition_point(|interval| interval.end() <= start);
        let mut current = start;
        while index < self.intervals.len() && self.intervals[index].start() < end {
            let interval = &self.intervals[index];
            if interval.start() > current {
                func(current, interval.start());
            }
            current = current.max(interval.end());
            index += 1;
        }
        if current < end {
            func(current, end);
        }
    }

    /// `ForEachInRange`: calls `func` for each interval overlapping the query.
    pub fn for_each_in_range(&self, start: u64, end: u64, mut func: impl FnMut(&IV)) {
        if start >= end {
            return;
        }
        let index = self
            .intervals
            .partition_point(|interval| interval.end() <= start);
        let mut index = index;
        while index < self.intervals.len() && self.intervals[index].start() < end {
            func(&self.intervals[index]);
            index += 1;
        }
    }

    /// `OverlapRun`: the first interval overlapping `[start, end)` and the first one after it.
    fn overlap_run(&self, start: u64, end: u64) -> (usize, usize) {
        let first = self
            .intervals
            .partition_point(|interval| interval.end() <= start);
        let mut last = first;
        while last < self.intervals.len() && self.intervals[last].start() <= end {
            last += 1;
        }
        (first, last)
    }

    /// `Splice`: replaces `[first, last)` with `src`.
    fn splice(&mut self, first: usize, last: usize, src: &[IV]) {
        if first == last {
            self.intervals.splice(first..first, src.iter().copied());
            return;
        }
        let slots = last - first;
        if src.len() <= slots {
            for (index, value) in src.iter().enumerate() {
                self.intervals[first + index] = *value;
            }
            if src.len() < slots {
                self.intervals.drain(first + src.len()..last);
            }
        } else {
            self.intervals[first..first + slots].copy_from_slice(&src[..slots]);
            self.intervals
                .splice(last..last, src[slots..].iter().copied());
        }
    }

    /// `Coalesce`: merges compatible adjacent intervals.
    fn coalesce(&mut self, start: usize, count: usize) {
        let mut stop = (start + count + 1).min(self.intervals.len());
        let mut index = if start > 0 { start - 1 } else { start };
        while index + 1 < stop {
            if self.intervals[index].end() == self.intervals[index + 1].start()
                && self.intervals[index].can_merge_with(&self.intervals[index + 1])
            {
                let end = self.intervals[index + 1].end();
                self.intervals[index].set_end(end);
                self.intervals.remove(index + 1);
                stop -= 1;
            } else {
                index += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ranges(list: &IntervalList) -> Vec<(u64, u64)> {
        list.iter()
            .map(|interval| (interval.start, interval.end))
            .collect()
    }

    #[test]
    fn adding_merges_adjacent_and_overlapping_ranges() {
        let mut list = IntervalList::new();
        list.add(Interval::new(0, 4));
        list.add(Interval::new(4, 8));
        list.add(Interval::new(10, 20));
        assert_eq!(ranges(&list), vec![(0, 8), (10, 20)]);
        list.add(Interval::new(6, 12));
        assert_eq!(ranges(&list), vec![(0, 20)]);
        assert!(list.contains(0, 20));
        assert!(!list.contains(0, 21));
        assert!(list.overlaps(19, 30));
        assert!(!list.overlaps(20, 30));
    }

    #[test]
    fn subtracting_splits_an_interval() {
        let mut list = IntervalList::new();
        list.add(Interval::new(0, 10));
        list.subtract(3, 7);
        assert_eq!(ranges(&list), vec![(0, 3), (7, 10)]);
        assert_eq!(list.find(2).map(|i| (i.start, i.end)), Some((0, 3)));
        assert_eq!(list.find(5), None);
        assert_eq!(list.find(8).map(|i| (i.start, i.end)), Some((7, 10)));
    }

    #[test]
    fn gaps_and_in_range_iteration_are_clipped() {
        let mut list = IntervalList::new();
        list.add(Interval::new(2, 4));
        list.add(Interval::new(6, 8));
        let mut gaps = Vec::new();
        list.for_each_gap(0, 10, |start, end| gaps.push((start, end)));
        assert_eq!(gaps, vec![(0, 2), (4, 6), (8, 10)]);
        let mut inside = Vec::new();
        list.for_each_in_range(3, 7, |interval| inside.push((interval.start, interval.end)));
        assert_eq!(inside, vec![(2, 4), (6, 8)]);
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct TaggedInterval {
        start: u64,
        end: u64,
        tag: u32,
    }

    impl IntervalType for TaggedInterval {
        fn start(&self) -> u64 {
            self.start
        }

        fn end(&self) -> u64 {
            self.end
        }

        fn set_end(&mut self, end: u64) {
            self.end = end;
        }

        fn can_merge_with(&self, other: &Self) -> bool {
            self.tag == other.tag
        }

        fn sub_range(&self, start: u64, end: u64) -> Self {
            TaggedInterval {
                start,
                end,
                tag: self.tag,
            }
        }
    }

    #[test]
    fn incompatible_intervals_stay_separate() {
        let mut list = IntervalList::<TaggedInterval>::new();
        list.add(TaggedInterval {
            start: 0,
            end: 4,
            tag: 1,
        });
        list.add(TaggedInterval {
            start: 4,
            end: 8,
            tag: 2,
        });
        assert_eq!(list.size(), 2);
    }
}
