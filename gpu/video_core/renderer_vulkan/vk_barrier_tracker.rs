//! Resource-range tracking for barriers, ported from
//! `shadps4/video_core/renderer_vulkan/vk_barrier_tracker.h` and `.cpp`.

/// `Access`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Access {
    None = 0,
    Read = 1,
    Write = 2,
}

/// `AddressRange`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AddressRange {
    pub resource: u64,
    pub range_start: u64,
    pub range_end: u64,
}

impl AddressRange {
    /// `Contains`.
    pub fn contains(&self, other: &AddressRange) -> bool {
        self.resource == other.resource
            && self.range_start <= other.range_start
            && self.range_end >= other.range_end
    }

    /// `Overlaps`.
    pub fn overlaps(&self, other: &AddressRange) -> bool {
        self.resource == other.resource
            && self.range_end >= other.range_start
            && self.range_start <= other.range_end
    }

    /// `operator<`.
    pub fn less(&self, other: &AddressRange) -> bool {
        self.resource < other.resource
            || (self.resource == other.resource && self.range_start < other.range_start)
    }
}

/// `BarrierTreeNode`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BarrierTreeNode {
    pub header: u64,
    pub address_range: AddressRange,
}

impl BarrierTreeNode {
    /// `NodeIndexMask`.
    pub const NODE_INDEX_MASK: u64 = (1 << 21) - 1;

    /// `SetRed`.
    pub fn set_red(&mut self, red: bool) {
        self.header &= !1u64;
        self.header |= u64::from(red);
    }

    /// `IsRed`.
    pub fn is_red(&self) -> bool {
        self.header & 1 != 0
    }

    /// `SetParent`.
    pub fn set_parent(&mut self, node: u32) {
        self.header &= !(Self::NODE_INDEX_MASK << 43);
        self.header |= u64::from(node) << 43;
    }

    /// `SetChild`.
    pub fn set_child(&mut self, index: u32, node: u32) {
        let shift = if index != 0 { 22 } else { 1 };
        self.header &= !(Self::NODE_INDEX_MASK << shift);
        self.header |= u64::from(node) << shift;
    }

    /// `Parent`.
    pub fn parent(&self) -> u32 {
        ((self.header >> 43) & Self::NODE_INDEX_MASK) as u32
    }

    /// `Child`.
    pub fn child(&self, index: u32) -> u32 {
        let shift = if index != 0 { 22 } else { 1 };
        ((self.header >> shift) & Self::NODE_INDEX_MASK) as u32
    }

    /// `IsRoot`.
    pub fn is_root(&self) -> bool {
        self.parent() == 0
    }
}

/// `BarrierTracker`.
#[derive(Debug, Default)]
pub struct BarrierTracker {
    root_mask_valid: u64,
    root_mask_subtree: u64,
    nodes: Vec<BarrierTreeNode>,
    free: Vec<u32>,
}

impl BarrierTracker {
    /// `HashTableSize`.
    pub const HASH_TABLE_SIZE: u32 = 32;

    /// `BarrierTracker()`.
    pub fn new() -> Self {
        let mut tracker = Self {
            nodes: vec![BarrierTreeNode::default()],
            ..Self::default()
        };
        for _ in 0..2 * Self::HASH_TABLE_SIZE {
            tracker.allocate_node();
        }
        tracker
    }

    /// `FindRange`.
    pub fn find_range(&self, range: &AddressRange, access_type: Access) -> bool {
        let root_index = Self::compute_root_index(range, access_type);
        self.find_node(range, root_index) != 0
    }

    /// `InsertRange`.
    pub fn insert_range(&mut self, range: &AddressRange, access_type: Access) {
        let root_index = Self::compute_root_index(range, access_type);
        let node_index = self.insert_node(range, root_index);
        if node_index == 0 {
            return;
        }
        let node = self.nodes[node_index as usize];
        if node.address_range.contains(range) {
            return;
        }
        let mut has_overlap = false;
        if range.range_start < node.address_range.range_start {
            let test = AddressRange {
                resource: range.resource,
                range_start: range.range_start,
                range_end: node.address_range.range_start - 1,
            };
            has_overlap = self.find_node(&test, root_index) != 0;
        }
        if range.range_end > node.address_range.range_end && !has_overlap {
            let test = AddressRange {
                resource: range.resource,
                range_start: node.address_range.range_end + 1,
                range_end: range.range_end,
            };
            has_overlap = self.find_node(&test, root_index) != 0;
        }
        if !has_overlap {
            let node = &mut self.nodes[node_index as usize];
            node.address_range.range_start = node.address_range.range_start.min(range.range_start);
            node.address_range.range_end = node.address_range.range_end.max(range.range_end);
            return;
        }
        let mut merged = *range;
        let mut node_index = node_index;
        while node_index != 0 {
            let node = self.nodes[node_index as usize];
            merged.range_start = merged.range_start.min(node.address_range.range_start);
            merged.range_end = merged.range_end.max(node.address_range.range_end);
            self.remove_node(node_index, root_index);
            node_index = self.find_node(range, root_index);
        }
        self.insert_node(&merged, root_index);
    }

    /// `Clear`.
    pub fn clear(&mut self) {
        self.root_mask_valid = 0;
        while self.root_mask_subtree != 0 {
            let root_index = self.root_mask_subtree.trailing_zeros() + 1;
            let (left, right, header) = {
                let root = &self.nodes[root_index as usize];
                (root.child(0), root.child(1), root.header)
            };
            if header != 0 {
                self.free_node(left);
                self.free_node(right);
                self.nodes[root_index as usize].header = 0;
            }
            self.root_mask_subtree &= self.root_mask_subtree - 1;
        }
    }

    /// `Empty`.
    pub fn is_empty(&self) -> bool {
        self.root_mask_valid == 0
    }

    fn allocate_node(&mut self) -> u32 {
        if let Some(node_index) = self.free.pop() {
            let (left, right) = {
                let node = &self.nodes[node_index as usize];
                (node.child(0), node.child(1))
            };
            self.free_node(left);
            self.free_node(right);
            self.nodes[node_index as usize].header = 0;
            node_index
        } else {
            let node_index = self.nodes.len() as u32;
            self.nodes.push(BarrierTreeNode::default());
            node_index
        }
    }

    fn free_node(&mut self, node: u32) {
        if node != 0 {
            self.free.push(node);
        }
    }

    fn find_node(&self, range: &AddressRange, root_index: u32) -> u32 {
        let root_bit = 1u64 << (root_index - 1);
        if self.root_mask_valid & root_bit == 0 {
            return 0;
        }
        let mut node_index = root_index;
        while node_index != 0 {
            let node = &self.nodes[node_index as usize];
            if node.address_range.overlaps(range) {
                return node_index;
            }
            node_index = node.child(u32::from(node.address_range.less(range)));
        }
        0
    }

    fn insert_node(&mut self, range: &AddressRange, root_index: u32) -> u32 {
        let root_bit = 1u64 << (root_index - 1);
        if self.root_mask_valid & root_bit == 0 {
            self.root_mask_valid |= root_bit;
            let node = &mut self.nodes[root_index as usize];
            node.header = 0;
            node.address_range = *range;
            return 0;
        }
        let mut parent_index = root_index;
        let mut child_index;
        loop {
            let parent = &self.nodes[parent_index as usize];
            if parent.address_range.overlaps(range) {
                return parent_index;
            }
            child_index = u32::from(parent.address_range.less(range));
            let child = parent.child(child_index);
            if child == 0 {
                break;
            }
            parent_index = child;
        }
        let node_index = self.allocate_node();
        self.nodes[parent_index as usize].set_child(child_index, node_index);
        {
            let node = &mut self.nodes[node_index as usize];
            node.set_red(true);
            node.set_parent(parent_index);
            node.address_range = *range;
        }
        if parent_index != root_index && !self.nodes[root_index as usize].is_red() {
            self.rebalance_post_insert(node_index, root_index);
        }
        self.root_mask_subtree |= root_bit;
        0
    }

    fn remove_node(&mut self, node_index: u32, root_index: u32) {
        let node = self.nodes[node_index as usize];
        let left = node.child(0);
        let right = node.child(1);
        if left != 0 && right != 0 {
            let mut child_index = right;
            while self.nodes[child_index as usize].child(0) != 0 {
                child_index = self.nodes[child_index as usize].child(0);
            }
            self.nodes[node_index as usize].address_range =
                self.nodes[child_index as usize].address_range;
            self.remove_node(child_index, root_index);
            return;
        }
        if !node.is_red() && node_index != root_index {
            self.nodes[root_index as usize].set_red(true);
        }
        let child_index = left.max(right);
        let parent_index = node.parent();
        if child_index != 0 {
            let child = self.nodes[child_index as usize];
            let node = &mut self.nodes[node_index as usize];
            node.set_child(0, child.child(0));
            node.set_child(1, child.child(1));
            if node_index != root_index {
                node.set_red(child.is_red());
            }
            node.address_range = child.address_range;
            if child.child(0) != 0 {
                self.nodes[child.child(0) as usize].set_parent(node_index);
            }
            if child.child(1) != 0 {
                self.nodes[child.child(1) as usize].set_parent(node_index);
            }
            self.nodes[child_index as usize].header = 0;
            self.free_node(child_index);
        } else if node_index != root_index {
            let parent = self.nodes[parent_index as usize];
            let which = u32::from(parent.child(1) == node_index);
            self.nodes[parent_index as usize].set_child(which, 0);
            self.nodes[node_index as usize].header = 0;
            self.free_node(node_index);
        } else {
            let root_bit = 1u64 << (root_index - 1);
            self.root_mask_subtree &= !root_bit;
            self.root_mask_valid &= !root_bit;
        }
    }

    fn rebalance_post_insert(&mut self, mut node_index: u32, root_index: u32) {
        while node_index != root_index {
            let parent_index = self.nodes[node_index as usize].parent();
            if !self.nodes[parent_index as usize].is_red() {
                break;
            }
            let grand_index = self.nodes[parent_index as usize].parent();
            let grand = self.nodes[grand_index as usize];
            if grand.child(1) == parent_index {
                let uncle_index = grand.child(0);
                if uncle_index != 0 && self.nodes[uncle_index as usize].is_red() {
                    self.nodes[grand_index as usize].set_red(true);
                    self.nodes[uncle_index as usize].set_red(false);
                    self.nodes[parent_index as usize].set_red(false);
                    node_index = self.nodes[parent_index as usize].parent();
                } else {
                    if self.nodes[parent_index as usize].child(0) == node_index {
                        self.rotate_right(parent_index, root_index);
                    }
                    self.nodes[parent_index as usize].set_red(false);
                    self.nodes[grand_index as usize].set_red(true);
                    self.rotate_left(self.nodes[parent_index as usize].parent(), root_index);
                }
            } else {
                let uncle_index = grand.child(1);
                if uncle_index != 0 && self.nodes[uncle_index as usize].is_red() {
                    self.nodes[grand_index as usize].set_red(true);
                    self.nodes[uncle_index as usize].set_red(false);
                    self.nodes[parent_index as usize].set_red(false);
                    node_index = self.nodes[parent_index as usize].parent();
                } else {
                    if self.nodes[parent_index as usize].child(1) == node_index {
                        self.rotate_left(parent_index, root_index);
                    }
                    self.nodes[parent_index as usize].set_red(false);
                    self.nodes[grand_index as usize].set_red(true);
                    self.rotate_right(self.nodes[parent_index as usize].parent(), root_index);
                }
            }
        }
        self.nodes[root_index as usize].set_red(false);
    }

    fn rotate_left(&mut self, node_index: u32, root_index: u32) {
        let node = self.nodes[node_index as usize];
        let left = node.child(0);
        let right = node.child(1);
        let right_node = self.nodes[right as usize];
        let right_left = right_node.child(0);
        let right_right = right_node.child(1);

        self.nodes[left as usize].set_parent(right);
        let is_red = right_node.is_red();
        self.nodes[right as usize].set_red(node.is_red());
        self.nodes[right as usize].set_child(0, left);
        self.nodes[right as usize].set_child(1, right_left);
        self.nodes[right_right as usize].set_parent(node_index);
        let node = &mut self.nodes[node_index as usize];
        node.set_red(is_red && node_index != root_index);
        node.set_child(0, right);
        node.set_child(1, right_right);
        let node_range = self.nodes[node_index as usize].address_range;
        self.nodes[node_index as usize].address_range = self.nodes[right as usize].address_range;
        self.nodes[right as usize].address_range = node_range;
    }

    fn rotate_right(&mut self, node_index: u32, root_index: u32) {
        let node = self.nodes[node_index as usize];
        let left = node.child(0);
        let right = node.child(1);
        let left_node = self.nodes[left as usize];
        let left_left = left_node.child(0);
        let left_right = left_node.child(1);

        self.nodes[right as usize].set_parent(left);
        let is_red = left_node.is_red();
        self.nodes[left as usize].set_red(node.is_red());
        self.nodes[left as usize].set_child(0, left_right);
        self.nodes[left as usize].set_child(1, right);
        self.nodes[left_left as usize].set_parent(node_index);
        let node = &mut self.nodes[node_index as usize];
        node.set_red(is_red && node_index != root_index);
        node.set_child(0, left_left);
        node.set_child(1, left);
        let node_range = self.nodes[node_index as usize].address_range;
        self.nodes[node_index as usize].address_range = self.nodes[left as usize].address_range;
        self.nodes[left as usize].address_range = node_range;
    }

    /// `ComputeRootIndex`.
    fn compute_root_index(range: &AddressRange, access: Access) -> u32 {
        let mut hash = range.resource.wrapping_mul(93887);
        hash ^= hash >> 16;
        1 + (hash % u64::from(Self::HASH_TABLE_SIZE)) as u32
            + if access == Access::Write {
                Self::HASH_TABLE_SIZE
            } else {
                0
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(start: u64, end: u64) -> AddressRange {
        AddressRange {
            resource: 1,
            range_start: start,
            range_end: end,
        }
    }

    #[test]
    fn inserted_ranges_are_found_per_access() {
        let mut tracker = BarrierTracker::new();
        assert!(tracker.is_empty());
        tracker.insert_range(&range(10, 20), Access::Read);
        assert!(!tracker.is_empty());
        assert!(tracker.find_range(&range(15, 16), Access::Read));
        assert!(!tracker.find_range(&range(15, 16), Access::Write));
        tracker.insert_range(&range(15, 16), Access::Write);
        assert!(tracker.find_range(&range(15, 16), Access::Write));
        tracker.clear();
        assert!(tracker.is_empty());
        assert!(!tracker.find_range(&range(15, 16), Access::Read));
    }

    #[test]
    fn contained_ranges_do_not_change_the_tree() {
        let mut tracker = BarrierTracker::new();
        tracker.insert_range(&range(0, 100), Access::Read);
        tracker.insert_range(&range(10, 20), Access::Read);
        assert!(tracker.find_range(&range(0, 100), Access::Read));
        assert!(tracker.find_range(&range(10, 20), Access::Read));
        assert!(!tracker.find_range(&range(101, 200), Access::Read));
    }

    #[test]
    fn overlapping_ranges_merge() {
        let mut tracker = BarrierTracker::new();
        tracker.insert_range(&range(10, 20), Access::Read);
        tracker.insert_range(&range(19, 30), Access::Read);
        assert!(tracker.find_range(&range(10, 30), Access::Read));
        assert!(!tracker.find_range(&range(31, 40), Access::Read));
    }

    #[test]
    fn different_resources_do_not_overlap() {
        let mut tracker = BarrierTracker::new();
        let mut other = range(10, 20);
        other.resource = 2;
        tracker.insert_range(&range(10, 20), Access::Read);
        assert!(!tracker.find_range(&other, Access::Read));
        assert!(tracker.find_range(&range(10, 20), Access::Read));
    }

    #[test]
    fn many_ranges_stay_findable() {
        let mut tracker = BarrierTracker::new();
        for index in 0..200u64 {
            let start = index * 100;
            tracker.insert_range(&range(start, start + 50), Access::Read);
        }
        for index in 0..200u64 {
            let start = index * 100;
            assert!(tracker.find_range(&range(start + 10, start + 20), Access::Read));
        }
        tracker.clear();
        assert!(tracker.is_empty());
    }
}
