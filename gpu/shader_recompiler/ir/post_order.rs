//! Post-order traversal, ported from `legacy/gpu/shadps4/shader_recompiler/ir/post_order.h`
//! and `post_order.cpp`. The C++ builds the order with a stack, a flat set of visited blocks
//! and the same `po_index` the dominator pass reads; the port keeps that shape, including the
//! detail that only the first unvisited successor is pushed per visit.

use super::basic_block::BlockRef;
use std::collections::HashSet;
use std::rc::Rc;

/// `Shader::IR::BlockList`, which `basic_block.h` declares.
pub use super::basic_block::BlockList;

/// `Shader::IR::PostOrder`: every block reachable from `first_block`, in post order, with each
/// block's `po_index` set to its position in the result.
pub fn post_order(first_block: &BlockRef) -> BlockList {
    let mut block_stack: Vec<BlockRef> = Vec::new();
    let mut visited: HashSet<usize> = HashSet::new();
    let mut post_order_blocks: BlockList = Vec::new();

    visited.insert(Rc::as_ptr(first_block) as usize);
    block_stack.push(first_block.clone());

    while let Some(block) = block_stack.pop() {
        let successors = block.borrow().imm_successors().to_vec();
        let mut finished = true;
        for branch in successors {
            if !visited.insert(Rc::as_ptr(&branch) as usize) {
                continue;
            }
            block_stack.push(block.clone());
            block_stack.push(branch);
            finished = false;
            break;
        }
        if finished {
            block.borrow_mut().po_index = post_order_blocks.len() as u32;
            post_order_blocks.push(block);
        }
    }
    post_order_blocks
}

#[cfg(test)]
mod tests {
    use super::super::basic_block::{add_branch, Block};
    use super::*;

    #[test]
    fn a_chain_comes_out_back_to_front() {
        let first = Block::new_ref();
        let second = Block::new_ref();
        let third = Block::new_ref();
        add_branch(&first, &second);
        add_branch(&second, &third);
        let order = post_order(&first);
        assert_eq!(order.len(), 3);
        assert!(Rc::ptr_eq(&order[0], &third), "the last block comes first");
        assert!(Rc::ptr_eq(&order[1], &second));
        assert!(Rc::ptr_eq(&order[2], &first), "the entry comes last");
        assert_eq!(first.borrow().po_index, 2);
        assert_eq!(second.borrow().po_index, 1);
        assert_eq!(third.borrow().po_index, 0);
    }

    #[test]
    fn a_diamond_visits_each_block_once() {
        let entry = Block::new_ref();
        let left = Block::new_ref();
        let right = Block::new_ref();
        let join = Block::new_ref();
        add_branch(&entry, &left);
        add_branch(&entry, &right);
        add_branch(&left, &join);
        add_branch(&right, &join);

        let order = post_order(&entry);
        assert_eq!(order.len(), 4, "the join is visited once");
        assert!(Rc::ptr_eq(&order[0], &join));
        assert!(Rc::ptr_eq(&order[3], &entry));
        let mut indices: Vec<u32> = order.iter().map(|block| block.borrow().po_index).collect();
        assert_eq!(indices, vec![0, 1, 2, 3], "the indices follow the order");
        indices.sort_unstable();
        assert_eq!(indices, vec![0, 1, 2, 3]);
    }

    #[test]
    fn a_loop_and_a_lone_block_are_handled() {
        let header = Block::new_ref();
        let body = Block::new_ref();
        add_branch(&header, &body);
        add_branch(&body, &header);
        let order = post_order(&header);
        assert_eq!(order.len(), 2);
        assert!(Rc::ptr_eq(&order[0], &body));
        assert!(Rc::ptr_eq(&order[1], &header));

        let alone = Block::new_ref();
        let order = post_order(&alone);
        assert_eq!(order.len(), 1);
        assert!(Rc::ptr_eq(&order[0], &alone));
        assert_eq!(alone.borrow().po_index, 0);
    }
}
