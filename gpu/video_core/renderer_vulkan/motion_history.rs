//! Motion-history bookkeeping, ported from
//! `shadps4/video_core/renderer_vulkan/motion_history.h`.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// `VertexRange`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct VertexRange {
    pub first: u32,
    pub count: u32,
}

/// The index value trait used by [`indexed_range`].
pub trait IndexValue: Copy {
    fn to_u32(self) -> u32;
    fn max_value() -> u32;
}

impl IndexValue for u16 {
    fn to_u32(self) -> u32 {
        u32::from(self)
    }

    fn max_value() -> u32 {
        u16::MAX as u32
    }
}

impl IndexValue for u32 {
    fn to_u32(self) -> u32 {
        self
    }

    fn max_value() -> u32 {
        u32::MAX
    }
}

/// `IndexedRange`.
pub fn indexed_range<I: IndexValue>(indices: &[I], base_vertex: i32, restart: bool) -> VertexRange {
    let mut low = u32::MAX;
    let mut high = 0;
    for index in indices {
        if restart && index.to_u32() == I::max_value() {
            continue;
        }
        low = low.min(index.to_u32());
        high = high.max(index.to_u32());
    }
    if low > high
        || i64::from(low) + i64::from(base_vertex) < 0
        || i64::from(high) + i64::from(base_vertex) > i32::MAX as i64
    {
        return VertexRange::default();
    }
    VertexRange {
        first: (i64::from(low) + i64::from(base_vertex)) as u32,
        count: high - low + 1,
    }
}

/// `BufferRole`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferRole {
    Other,
    Skeleton,
    SmallSkeleton,
}

/// `ClassifyBuffer`.
pub fn classify_buffer(size: u32) -> BufferRole {
    if size == 864 || size > 16384 {
        return BufferRole::Other;
    }
    if size >= 640 {
        return BufferRole::Skeleton;
    }
    if size >= 96 && size.is_multiple_of(48) {
        BufferRole::SmallSkeleton
    } else {
        BufferRole::Other
    }
}

/// `IndexRangeCache`.
#[derive(Debug, Default)]
pub struct IndexRangeCache {
    entries: HashMap<IndexRangeKey, Entry>,
    pub stats: Stats,
}

/// `IndexRangeCache::Key`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct IndexRangeKey {
    pub address: u64,
    pub count: u32,
    pub index_size: u32,
    pub base_vertex: i32,
    pub restart: bool,
}

/// `IndexRangeCache::Result`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Result_ {
    pub range: VertexRange,
    pub topology: u64,
}

/// `IndexRangeCache::Stats`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    pub hits: u64,
    pub scans: u64,
    pub stale: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct Entry {
    result: Result_,
    verified: u64,
    last_use: u64,
    valid: bool,
}

impl IndexRangeCache {
    /// `Revalidate`.
    pub const REVALIDATE: u64 = 32;
    /// `Unused`.
    pub const UNUSED: u64 = 600;

    /// `Get`.
    pub fn get(
        &mut self,
        key: &IndexRangeKey,
        frame: u64,
        scan: impl FnOnce() -> Result_,
    ) -> Result_ {
        let entry = self.entries.entry(*key).or_default();
        entry.last_use = frame;
        if entry.valid && frame - entry.verified < Self::REVALIDATE {
            self.stats.hits += 1;
            return entry.result;
        }
        self.stats.scans += 1;
        let fresh = scan();
        if entry.valid
            && (fresh.topology != entry.result.topology || fresh.range != entry.result.range)
        {
            self.stats.stale += 1;
        }
        entry.result = fresh;
        entry.verified = frame;
        entry.valid = true;
        fresh
    }

    /// `Trim`.
    pub fn trim(&mut self, frame: u64) {
        self.entries
            .retain(|_, entry| frame - entry.last_use <= Self::UNUSED);
    }

    /// `Size`.
    pub fn size(&self) -> usize {
        self.entries.len()
    }
}

/// `FlatMap`: an open-addressing map cleared by generation.
#[derive(Debug)]
pub struct FlatMap<K, V> {
    slots: Vec<Slot<K, V>>,
    size: usize,
    generation: u32,
}

#[derive(Debug, Clone)]
struct Slot<K, V> {
    key: K,
    value: V,
    generation: u32,
}

impl<K: Default, V: Default> Default for Slot<K, V> {
    fn default() -> Self {
        Self {
            key: K::default(),
            value: V::default(),
            generation: 0,
        }
    }
}

impl<K, V> Slot<K, V> {
    fn new(key: K, value: V, generation: u32) -> Self {
        Self {
            key,
            value,
            generation,
        }
    }
}

impl<K: Eq + Hash + Clone + Default, V: Clone + Default> Default for FlatMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Eq + Hash + Clone + Default, V: Clone + Default> FlatMap<K, V> {
    pub fn new() -> Self {
        Self {
            slots: (0..64).map(|_| Slot::default()).collect(),
            size: 0,
            generation: 1,
        }
    }

    fn start(key: &K) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let h = hasher.finish().wrapping_mul(0x9e37_79b9_7f4a_7c15);
        (h ^ (h >> 29)) as usize
    }

    /// `operator[]`.
    pub fn get_or_insert_mut(&mut self, key: &K) -> &mut V {
        let (slot, _) = self.insert(key);
        &mut self.slots[slot].value
    }

    /// `emplace`.
    pub fn emplace(&mut self, key: &K, value: V) {
        let (slot, inserted) = self.insert(key);
        if inserted {
            self.slots[slot].value = value;
        }
    }

    /// `find`.
    pub fn find(&self, key: &K) -> Option<&V> {
        let mask = self.slots.len() - 1;
        let mut index = Self::start(key) & mask;
        loop {
            let slot = &self.slots[index];
            if slot.generation != self.generation {
                return None;
            }
            if slot.key == *key {
                return Some(&slot.value);
            }
            index = (index + 1) & mask;
        }
    }

    /// `clear`.
    pub fn clear(&mut self) {
        self.size = 0;
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            for slot in &mut self.slots {
                slot.generation = 0;
            }
            self.generation = 1;
        }
    }

    /// `swap`.
    pub fn swap(&mut self, other: &mut Self) {
        std::mem::swap(&mut self.slots, &mut other.slots);
        std::mem::swap(&mut self.size, &mut other.size);
        std::mem::swap(&mut self.generation, &mut other.generation);
    }

    fn insert(&mut self, key: &K) -> (usize, bool) {
        if (self.size + 1) * 2 > self.slots.len() {
            self.grow();
        }
        let mask = self.slots.len() - 1;
        let mut index = Self::start(key) & mask;
        loop {
            let generation = self.slots[index].generation;
            if generation != self.generation {
                self.slots[index] = Slot::new(key.clone(), V::default(), self.generation);
                self.size += 1;
                return (index, true);
            }
            if self.slots[index].key == *key {
                return (index, false);
            }
            index = (index + 1) & mask;
        }
    }

    fn grow(&mut self) {
        let old = std::mem::take(&mut self.slots);
        self.slots = (0..old.len() * 2).map(|_| Slot::default()).collect();
        let live = self.generation;
        self.generation = 1;
        self.size = 0;
        for slot in old {
            if slot.generation == live {
                let (index, _) = self.insert(&slot.key);
                self.slots[index].value = slot.value;
            }
        }
    }
}

/// `JitterPhases`.
pub fn jitter_phases(render_width: u32, output_width: u32) -> u32 {
    if render_width == 0 || output_width == 0 {
        return 8;
    }
    let ratio = f64::from(output_width) / f64::from(render_width);
    (8.0 * ratio * ratio).ceil().clamp(8.0, 256.0) as u32
}

/// `Draw`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Draw {
    pub shader: u64,
    pub geometry: u64,
    pub indices: u64,
    pub topology: u64,
    pub index_count: u32,
    pub instances: u32,
    pub first_instance: u32,
    pub vertices: VertexRange,
}

/// `History`.
#[derive(Debug)]
pub struct History {
    capacity: u32,
    used: u32,
    frame: u64,
    previous: FlatMap<HistoryKey, u32>,
    current: FlatMap<HistoryKey, u32>,
    occurrences: FlatMap<HistoryKey, u32>,
    gate_previous: FlatMap<GateKey, u64>,
    gate_current: FlatMap<GateKey, u64>,
    gate_occurrences: FlatMap<GateKey, u32>,
    pub stats: HistoryStats,
}

/// `History::Allocation`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Allocation {
    pub store: u32,
    pub load: u32,
    pub vertices: u32,
    pub instances: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

/// `History::Stats`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HistoryStats {
    pub draws: u64,
    pub stored: u64,
    pub loaded: u64,
    pub unmatched: u64,
    pub exhausted: u64,
    pub invalid: u64,
    pub still: u64,
}

/// `History::GateKey`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct GateKey {
    pub shader: u64,
    pub geometry: u64,
    pub indices: u64,
    pub index_count: u32,
    pub instances: u32,
    pub first_instance: u32,
    pub occurrence: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
struct HistoryKey {
    draw: Draw,
    occurrence: u32,
}

impl History {
    pub fn new(capacity: u32) -> Self {
        Self {
            capacity,
            used: 0,
            frame: 0,
            previous: FlatMap::new(),
            current: FlatMap::new(),
            occurrences: FlatMap::new(),
            gate_previous: FlatMap::new(),
            gate_current: FlatMap::new(),
            gate_occurrences: FlatMap::new(),
            stats: HistoryStats::default(),
        }
    }

    /// `Moving`.
    pub fn moving(&mut self, mut key: GateKey, palette: u64) -> bool {
        key.occurrence = 0;
        let occurrence = {
            let value = self.gate_occurrences.get_or_insert_mut(&key);
            let previous = *value;
            *value += 1;
            previous
        };
        key.occurrence = occurrence;
        let previous = self.gate_previous.find(&key).copied();
        let moving = previous.is_some_and(|previous| previous != palette);
        self.gate_current.emplace(&key, palette);
        if !moving {
            self.stats.still += 1;
        }
        moving
    }

    /// `NextFrame`.
    pub fn next_frame(&mut self) {
        self.previous.swap(&mut self.current);
        self.current.clear();
        self.occurrences.clear();
        self.gate_previous.swap(&mut self.gate_current);
        self.gate_current.clear();
        self.gate_occurrences.clear();
        self.frame += 1;
        self.used = 0;
    }

    /// `Prepare`.
    pub fn prepare(&mut self, draw: &Draw) -> Allocation {
        self.stats.draws += 1;
        let mut key = HistoryKey {
            draw: *draw,
            occurrence: 0,
        };
        key.occurrence = {
            let value = self.occurrences.get_or_insert_mut(&key);
            let previous = *value;
            *value += 1;
            previous
        };
        let count = u64::from(draw.vertices.count) * u64::from(draw.instances);
        if count == 0
            || u64::from(draw.vertices.first) + u64::from(draw.vertices.count) - 1 > i32::MAX as u64
            || u64::from(draw.first_instance) + u64::from(draw.instances) > u32::MAX as u64
        {
            self.stats.invalid += 1;
            return Allocation::default();
        }
        let previous = self.previous.find(&key).copied();
        let mut result = Allocation {
            store: 0,
            load: 0,
            vertices: draw.vertices.count,
            instances: draw.instances,
            first_vertex: draw.vertices.first,
            first_instance: draw.first_instance,
        };
        match previous {
            Some(load) => {
                result.load = load;
                self.stats.loaded += 1;
            }
            None => self.stats.unmatched += 1,
        }
        if count <= u64::from(self.capacity - self.used) {
            result.store = 1 + (self.frame & 1) as u32 * self.capacity + self.used;
            self.used += count as u32;
            self.current.emplace(&key, result.store);
            self.stats.stored += 1;
        } else {
            self.stats.exhausted += 1;
        }
        result
    }

    /// `Used`.
    pub fn used(&self) -> u32 {
        self.used
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_ranges_skip_restart_and_reject_overflow() {
        let indices = [0u16, 4, 2, u16::MAX, 7];
        assert_eq!(
            indexed_range(&indices, 10, true),
            VertexRange {
                first: 10,
                count: 8
            }
        );
        assert_eq!(
            indexed_range(&indices, 10, false),
            VertexRange {
                first: 10,
                count: 65536
            }
        );
        assert_eq!(indexed_range(&indices, -10, false), VertexRange::default());
        assert_eq!(indexed_range::<u16>(&[], 0, false), VertexRange::default());
        assert_eq!(indexed_range(&[u32::MAX], 0, false), VertexRange::default());
    }

    #[test]
    fn buffers_are_classified_by_size() {
        assert_eq!(classify_buffer(864), BufferRole::Other);
        assert_eq!(classify_buffer(20000), BufferRole::Other);
        assert_eq!(classify_buffer(640), BufferRole::Skeleton);
        assert_eq!(classify_buffer(96), BufferRole::SmallSkeleton);
        assert_eq!(classify_buffer(144), BufferRole::SmallSkeleton);
        assert_eq!(classify_buffer(100), BufferRole::Other);
    }

    #[test]
    fn the_index_range_cache_revalidates_and_trims() {
        let mut cache = IndexRangeCache::default();
        let key = IndexRangeKey::default();
        let result = Result_ {
            range: VertexRange { first: 1, count: 2 },
            topology: 3,
        };
        let mut scans = 0;
        assert_eq!(
            cache.get(&key, 0, || {
                scans += 1;
                result
            }),
            result
        );
        assert_eq!(
            cache.get(&key, 1, || {
                scans += 1;
                result
            }),
            result
        );
        assert_eq!(scans, 1);
        assert_eq!(cache.stats.hits, 1);
        assert_eq!(cache.stats.scans, 1);

        let changed = Result_ {
            range: VertexRange { first: 4, count: 1 },
            topology: 3,
        };
        assert_eq!(
            cache.get(&key, IndexRangeCache::REVALIDATE, || changed),
            changed
        );
        assert_eq!(cache.stats.stale, 1);
        cache.trim(IndexRangeCache::REVALIDATE + IndexRangeCache::UNUSED + 1);
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn flat_map_handles_generations_growth_and_clearing() {
        let mut map: FlatMap<u32, u32> = FlatMap::new();
        for index in 0..100 {
            map.emplace(&index, index * 2);
        }
        assert_eq!(map.find(&50), Some(&100));
        assert_eq!(map.find(&101), None);
        assert_eq!(*map.get_or_insert_mut(&101), 0);
        *map.get_or_insert_mut(&101) = 7;
        assert_eq!(map.find(&101), Some(&7));
        map.clear();
        assert_eq!(map.find(&50), None);
        map.emplace(&50, 9);
        assert_eq!(map.find(&50), Some(&9));
        let mut other: FlatMap<u32, u32> = FlatMap::new();
        other.emplace(&1, 2);
        map.swap(&mut other);
        assert_eq!(map.find(&1), Some(&2));
        assert_eq!(other.find(&50), Some(&9));
    }

    #[test]
    fn jitter_phases_scale_with_the_ratio() {
        assert_eq!(jitter_phases(0, 1920), 8);
        assert_eq!(jitter_phases(1920, 1920), 8);
        assert_eq!(jitter_phases(1920, 3840), 32);
        assert_eq!(jitter_phases(1, 100000), 256);
    }

    #[test]
    fn history_loads_the_previous_frame_and_tracks_exhaustion() {
        let mut history = History::new(8);
        let draw = Draw {
            shader: 1,
            indices: 2,
            topology: 3,
            instances: 1,
            vertices: VertexRange { first: 0, count: 4 },
            ..Draw::default()
        };
        let first = history.prepare(&draw);
        assert_eq!(first.store, 1);
        assert_eq!(first.load, 0);
        assert_eq!(history.stats.unmatched, 1);
        assert_eq!(history.used(), 4);
        history.next_frame();
        let second = history.prepare(&draw);
        assert_eq!(second.load, first.store);
        assert_eq!(second.store, 1 + 8);
        assert_eq!(history.stats.loaded, 1);

        history.next_frame();
        let third = history.prepare(&draw);
        assert_eq!(third.store, 1);
        history.next_frame();
        let exhausted = history.prepare(&Draw {
            vertices: VertexRange {
                first: 0,
                count: 100,
            },
            instances: 1,
            ..Draw::default()
        });
        assert_eq!(exhausted.store, 0);
        assert_eq!(history.stats.exhausted, 1);

        let invalid = history.prepare(&Draw {
            vertices: VertexRange {
                first: u32::MAX,
                count: 2,
            },
            instances: 1,
            ..Draw::default()
        });
        assert_eq!(invalid, Allocation::default());
        assert_eq!(history.stats.invalid, 1);
    }
}
