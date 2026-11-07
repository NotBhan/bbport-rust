//! A tick-ordered least-recently-used list, ported from
//! `legacy/gpu/shadsps4/common/lru_cache.h`.
//!
//! The C++ keeps an intrusive doubly linked list over a deque of items; the port keeps the
//! same links as indices into a slot vector, so ids stay stable and freed slots are reused.

use std::collections::VecDeque;

#[derive(Debug, Clone)]
struct Item<O, T> {
    obj: O,
    tick: T,
    next: Option<usize>,
    prev: Option<usize>,
}

/// `Common::LeastRecentlyUsedCache`.
#[derive(Debug, Clone)]
pub struct LeastRecentlyUsedCache<O, T> {
    items: Vec<Option<Item<O, T>>>,
    free_items: VecDeque<usize>,
    first_item: Option<usize>,
    last_item: Option<usize>,
}

impl<O, T> Default for LeastRecentlyUsedCache<O, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<O, T> LeastRecentlyUsedCache<O, T> {
    /// `LeastRecentlyUsedCache()`.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            free_items: VecDeque::new(),
            first_item: None,
            last_item: None,
        }
    }

    fn build(&mut self) -> usize {
        if let Some(id) = self.free_items.pop_front() {
            self.items[id] = None;
            return id;
        }
        let id = self.items.len();
        self.items.push(None);
        id
    }

    fn attach(&mut self, id: usize) {
        if self.first_item.is_none() {
            self.first_item = Some(id);
        }
        if let Some(last) = self.last_item {
            self.items[id].as_mut().expect("the item exists").prev = Some(last);
            self.items[last]
                .as_mut()
                .expect("the last item exists")
                .next = Some(id);
            self.items[id].as_mut().expect("the item exists").next = None;
        }
        self.last_item = Some(id);
    }

    fn detach(&mut self, id: usize) {
        let (prev, next) = {
            let item = self.items[id].as_ref().expect("the item exists");
            (item.prev, item.next)
        };
        if let Some(prev) = prev {
            self.items[prev]
                .as_mut()
                .expect("the previous item exists")
                .next = next;
        }
        if let Some(next) = next {
            self.items[next]
                .as_mut()
                .expect("the next item exists")
                .prev = prev;
        }
        if self.first_item == Some(id) {
            self.first_item = next;
            if let Some(first) = self.first_item {
                self.items[first]
                    .as_mut()
                    .expect("the first item exists")
                    .prev = None;
            }
        }
        if self.last_item == Some(id) {
            self.last_item = prev;
            if let Some(last) = self.last_item {
                self.items[last]
                    .as_mut()
                    .expect("the last item exists")
                    .next = None;
            }
        }
    }

    /// `Insert`.
    pub fn insert(&mut self, obj: O, tick: T) -> usize {
        let id = self.build();
        self.items[id] = Some(Item {
            obj,
            tick,
            next: None,
            prev: None,
        });
        self.attach(id);
        id
    }

    /// `Free`.
    pub fn free(&mut self, id: usize) {
        self.detach(id);
        self.items[id] = None;
        self.free_items.push_back(id);
    }

    /// The object stored under an id.
    pub fn get(&self, id: usize) -> Option<&O> {
        self.items
            .get(id)
            .and_then(|item| item.as_ref())
            .map(|item| &item.obj)
    }

    /// The tick stored under an id.
    pub fn tick(&self, id: usize) -> Option<&T> {
        self.items
            .get(id)
            .and_then(|item| item.as_ref())
            .map(|item| &item.tick)
    }

    /// The item ids from oldest to newest, for diagnostics and tests.
    pub fn order(&self) -> Vec<usize> {
        let mut order = Vec::new();
        let mut current = self.first_item;
        while let Some(id) = current {
            order.push(id);
            current = self.items[id].as_ref().expect("the item exists").next;
        }
        order
    }
}

impl<O, T: Copy + PartialOrd> LeastRecentlyUsedCache<O, T> {
    /// `Touch`.
    pub fn touch(&mut self, id: usize, tick: T) {
        let item = self.items[id].as_ref().expect("the item exists");
        if item.tick >= tick {
            return;
        }
        self.items[id].as_mut().expect("the item exists").tick = tick;
        if self.last_item == Some(id) {
            return;
        }
        self.detach(id);
        self.attach(id);
    }

    /// `ForEachItemBelow`, without the early-exit form.
    pub fn for_each_item_below(&self, tick: T, mut func: impl FnMut(&O)) {
        let mut current = self.first_item;
        while let Some(id) = current {
            let item = self.items[id].as_ref().expect("the item exists");
            if tick < item.tick {
                return;
            }
            let next = item.next;
            func(&item.obj);
            current = next;
        }
    }

    /// `ForEachItemBelow` for a callback returning `bool`; `true` stops the walk.
    pub fn for_each_item_below_until(&self, tick: T, mut func: impl FnMut(&O) -> bool) {
        let mut current = self.first_item;
        while let Some(id) = current {
            let item = self.items[id].as_ref().expect("the item exists");
            if tick < item.tick {
                return;
            }
            let next = item.next;
            if func(&item.obj) {
                return;
            }
            current = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insertion_keeps_the_order_and_touch_moves_to_the_back() {
        let mut cache = LeastRecentlyUsedCache::new();
        let first = cache.insert("a", 1u64);
        let second = cache.insert("b", 2);
        let third = cache.insert("c", 3);
        assert_eq!(cache.order(), vec![first, second, third]);
        cache.touch(first, 4);
        assert_eq!(cache.order(), vec![second, third, first]);
        assert_eq!(cache.get(second), Some(&"b"));
        assert_eq!(cache.tick(first), Some(&4));
        cache.touch(first, 2);
        assert_eq!(cache.tick(first), Some(&4), "an older tick is ignored");
    }

    #[test]
    fn freed_ids_are_reused() {
        let mut cache = LeastRecentlyUsedCache::new();
        let a = cache.insert(1u32, 1u32);
        let b = cache.insert(2, 2);
        cache.free(a);
        assert_eq!(cache.order(), vec![b]);
        assert_eq!(cache.get(a), None);
        let reused = cache.insert(3, 3);
        assert_eq!(reused, a);
        assert_eq!(cache.get(reused), Some(&3));
    }

    #[test]
    fn iteration_stops_at_the_tick_and_can_stop_early() {
        let mut cache = LeastRecentlyUsedCache::new();
        cache.insert(1u32, 1u32);
        cache.insert(2u32, 2);
        cache.insert(3u32, 3);
        let mut seen = Vec::new();
        cache.for_each_item_below(2, |obj| seen.push(*obj));
        assert_eq!(seen, vec![1, 2]);

        let mut seen = Vec::new();
        cache.for_each_item_below_until(3, |obj| {
            seen.push(*obj);
            *obj == 2
        });
        assert_eq!(seen, vec![1, 2]);
    }
}
