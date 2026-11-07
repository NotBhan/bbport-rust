//! Fenced resource pools, ported from
//! `shadps4/video_core/renderer_vulkan/vk_resource_pool.h`.

use std::collections::HashMap;

/// `ResourcePool`.
#[derive(Debug, Default)]
pub struct ResourcePool {
    pub work_semaphore: Option<u64>,
    pub grow_step: usize,
    pub hint_iterator: usize,
    pub ticks: Vec<u64>,
    pub next_tick: u64,
}

impl ResourcePool {
    pub fn new(work_semaphore: Option<u64>, grow_step: usize) -> Self {
        Self {
            work_semaphore,
            grow_step,
            hint_iterator: 0,
            ticks: Vec::new(),
            next_tick: 1,
        }
    }

    /// `CommitResource`; returns the first tick that is free or grows the pool.
    pub fn commit_resource(&mut self) -> usize {
        if let Some(index) = self.ticks.iter().position(|&tick| tick == 0) {
            self.ticks[index] = self.next_tick;
            self.next_tick += 1;
            return index;
        }
        let index = self.ticks.len();
        self.ticks.push(self.next_tick);
        self.next_tick += 1;
        self.hint_iterator = index;
        index
    }

    pub fn release(&mut self, index: usize) {
        if let Some(tick) = self.ticks.get_mut(index) {
            *tick = 0;
        }
    }

    /// `ManageOverflow`; grows by `grow_step` resources.
    pub fn manage_overflow(&mut self) -> usize {
        let begin = self.ticks.len();
        self.ticks.resize(begin + self.grow_step.max(1), 0);
        begin
    }
}

/// `CommandPool`.
#[derive(Debug, Default)]
pub struct CommandPool {
    pub pool: ResourcePool,
    pub commands: Vec<u64>,
}

impl CommandPool {
    pub fn new(work_semaphore: Option<u64>) -> Self {
        Self {
            pool: ResourcePool::new(work_semaphore, 16),
            commands: Vec::new(),
        }
    }

    /// `Commit`.
    pub fn commit(&mut self) -> u64 {
        let index = self.pool.commit_resource();
        let handle = index as u64 + 1;
        if self.commands.len() <= index {
            self.commands.push(handle);
        }
        self.commands[index]
    }
}

/// `DescriptorHeap`.
#[derive(Debug, Default)]
pub struct DescriptorHeap {
    pub descriptor_heap_count: u32,
    pub descriptor_sets: HashMap<u64, Vec<u64>>,
    pub next_set: u64,
}

impl DescriptorHeap {
    pub fn new(descriptor_heap_count: u32) -> Self {
        Self {
            descriptor_heap_count,
            descriptor_sets: HashMap::new(),
            next_set: 1,
        }
    }

    /// `Commit`; caches a batch of sets per layout.
    pub fn commit(&mut self, set_layout: u64) -> u64 {
        let batch = self.descriptor_sets.entry(set_layout).or_default();
        if let Some(&set) = batch.first() {
            return set;
        }
        let set = self.next_set;
        self.next_set += 1;
        batch.push(set);
        set
    }

    /// `ForgetLayout`.
    pub fn forget_layout(&mut self, set_layout: u64) {
        self.descriptor_sets.remove(&set_layout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pools_reuse_released_ticks_and_commit_commands() {
        let mut pool = ResourcePool::new(None, 2);
        assert_eq!(pool.commit_resource(), 0);
        assert_eq!(pool.commit_resource(), 1);
        pool.release(0);
        assert_eq!(pool.commit_resource(), 0);
        assert_eq!(pool.manage_overflow(), 2);
        let mut commands = CommandPool::new(None);
        assert_eq!(commands.commit(), 1);
        assert_eq!(commands.commit(), 2);
        assert_eq!(commands.pool.ticks.len(), 2);
    }

    #[test]
    fn descriptor_heaps_cache_and_forget_layouts() {
        let mut heap = DescriptorHeap::new(1024);
        let first = heap.commit(5);
        assert_eq!(heap.commit(5), first);
        assert_ne!(heap.commit(6), first);
        heap.forget_layout(5);
        assert_ne!(heap.commit(5), first);
        assert_eq!(heap.descriptor_heap_count, 1024);
    }
}
