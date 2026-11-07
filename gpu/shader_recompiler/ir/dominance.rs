//! Dominator analysis, ported from `legacy/gpu/shads4/shader_recompiler/ir/dominance.h` and
//! `dominance.cpp` — the algorithms of "A Simple, Fast Dominance Algorithm". The C++ walks
//! pointers and may look at a null `immediate_dominator`; the port walks handles and treats a
//! missing dominator the same way, as "not yet assigned".

use super::basic_block::BlockRef;
use super::post_order::BlockList;
use std::rc::Rc;

/// `ComputeDominators`: iterates the intersection algorithm until nothing changes. The entry is
/// the last block of the post order, and it dominates itself.
pub fn compute_dominators(post_order: &BlockList) {
    let Some(entry) = post_order.last() else {
        return;
    };
    entry.borrow_mut().immediate_dominator = Some(entry.clone());

    fn po_index(block: &BlockRef) -> u32 {
        block.borrow().po_index
    }

    let intersect = |mut block1: BlockRef, mut block2: BlockRef| -> BlockRef {
        while !Rc::ptr_eq(&block1, &block2) {
            if po_index(&block1) < po_index(&block2) {
                match immediate_dominator(&block1) {
                    Some(next) => {
                        block1 = next;
                        continue;
                    }
                    None => return block1,
                }
            }
            if po_index(&block2) < po_index(&block1) {
                match immediate_dominator(&block2) {
                    Some(next) => {
                        block2 = next;
                        continue;
                    }
                    None => return block2,
                }
            }
        }
        block1
    };

    loop {
        let mut modified = false;
        // `post_order.rbegin() + 1`: every block but the entry, in reverse.
        for block in post_order[..post_order.len().saturating_sub(1)]
            .iter()
            .rev()
        {
            let mut new_idom: Option<BlockRef> = None;
            for predecessor in block.borrow().imm_predecessors().to_vec() {
                if predecessor.borrow().immediate_dominator.is_none() {
                    continue;
                }
                new_idom = Some(match new_idom {
                    None => predecessor,
                    Some(current) => intersect(current, predecessor),
                });
            }
            if new_idom.is_none() {
                continue;
            }
            let changed = match &block.borrow().immediate_dominator {
                Some(current) => !Rc::ptr_eq(current, new_idom.as_ref().expect("assigned")),
                None => true,
            };
            if changed {
                block.borrow_mut().immediate_dominator = new_idom;
                modified = true;
            }
        }
        if !modified {
            break;
        }
    }
}

/// `ComputeDominanceFrontiers`: a block with two or more predecessors gives every predecessor
/// on the way up to its dominator a frontier entry.
fn immediate_dominator(block: &BlockRef) -> Option<BlockRef> {
    block.borrow().immediate_dominator.clone()
}

pub fn compute_dominance_frontiers(post_order: &BlockList) {
    for block in post_order {
        let predecessors = block.borrow().imm_predecessors().to_vec();
        if predecessors.len() < 2 {
            continue;
        }
        let idom = immediate_dominator(block);
        for predecessor in predecessors {
            let mut current = predecessor;
            loop {
                if Some(Rc::as_ptr(&current)) == idom.as_ref().map(Rc::as_ptr) {
                    break;
                }
                current.borrow_mut().dominance_frontiers.push(block.clone());
                match immediate_dominator(&current) {
                    Some(next) => current = next,
                    None => break,
                }
            }
        }
    }
}

/// `Dominates`: whether `a` is on the dominator chain of `b`, the entry being its own dominator.
pub fn dominates(a: &BlockRef, b: &BlockRef) -> bool {
    let mut current = Some(b.clone());
    while let Some(block) = current {
        if Rc::ptr_eq(&block, a) {
            return true;
        }
        let idom = block.borrow().immediate_dominator.clone();
        if let Some(idom) = &idom {
            if Rc::ptr_eq(idom, &block) {
                break;
            }
        }
        current = idom;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::super::basic_block::{add_branch, Block};
    use super::super::post_order::post_order;
    use super::*;

    /// entry -> left -> join, entry -> right -> join: the classic diamond.
    fn diamond() -> (BlockRef, BlockRef, BlockRef, BlockRef) {
        let entry = Block::new_ref();
        let left = Block::new_ref();
        let right = Block::new_ref();
        let join = Block::new_ref();
        add_branch(&entry, &left);
        add_branch(&entry, &right);
        add_branch(&left, &join);
        add_branch(&right, &join);
        (entry, left, right, join)
    }

    #[test]
    fn a_diamond_gives_the_entry_and_the_join_their_dominators() {
        let (entry, left, right, join) = diamond();
        let order = post_order(&entry);
        compute_dominators(&order);

        assert!(Rc::ptr_eq(
            &entry
                .borrow()
                .immediate_dominator
                .clone()
                .expect("entry idom"),
            &entry
        ));
        assert!(Rc::ptr_eq(
            &left
                .borrow()
                .immediate_dominator
                .clone()
                .expect("left idom"),
            &entry
        ));
        assert!(Rc::ptr_eq(
            &right
                .borrow()
                .immediate_dominator
                .clone()
                .expect("right idom"),
            &entry
        ));
        assert!(Rc::ptr_eq(
            &join
                .borrow()
                .immediate_dominator
                .clone()
                .expect("join idom"),
            &entry
        ));

        compute_dominance_frontiers(&order);
        // The join's two predecessors both have the join in their frontiers.
        let left_frontiers = left.borrow().dominance_frontiers.clone();
        assert_eq!(left_frontiers.len(), 1);
        assert!(Rc::ptr_eq(&left_frontiers[0], &join));
        let right_frontiers = right.borrow().dominance_frontiers.clone();
        assert_eq!(right_frontiers.len(), 1);
        assert!(Rc::ptr_eq(&right_frontiers[0], &join));
        assert!(entry.borrow().dominance_frontiers.is_empty());

        assert!(dominates(&entry, &join));
        assert!(dominates(&entry, &entry));
        assert!(!dominates(&join, &entry));
        assert!(!dominates(&left, &right));
        assert!(dominates(&entry, &left) && dominates(&entry, &right));
    }

    #[test]
    fn a_chain_dominates_every_block_below_it() {
        let first = Block::new_ref();
        let second = Block::new_ref();
        let third = Block::new_ref();
        add_branch(&first, &second);
        add_branch(&second, &third);
        let order = post_order(&first);
        compute_dominators(&order);
        compute_dominance_frontiers(&order);

        assert!(dominates(&first, &third));
        assert!(dominates(&second, &third));
        assert!(!dominates(&third, &second));
        assert!(Rc::ptr_eq(
            &third.borrow().immediate_dominator.clone().expect("idom"),
            &second
        ));
        assert!(second.borrow().dominance_frontiers.is_empty());
    }

    #[test]
    fn a_loop_header_dominates_its_body_and_the_frontier_closes() {
        // header -> body -> header, header -> exit, body -> exit.
        let header = Block::new_ref();
        let body = Block::new_ref();
        let exit = Block::new_ref();
        add_branch(&header, &body);
        add_branch(&body, &header);
        add_branch(&header, &exit);
        add_branch(&body, &exit);

        let order = post_order(&header);
        // The traversal starts at the header, so the exit is visited before the header.
        assert_eq!(order.len(), 3);
        compute_dominators(&order);
        compute_dominance_frontiers(&order);

        assert!(dominates(&header, &body));
        assert!(dominates(&header, &exit));
        assert!(Rc::ptr_eq(
            &body.borrow().immediate_dominator.clone().expect("idom"),
            &header
        ));
        let exit_frontiers = exit.borrow().dominance_frontiers.clone();
        assert!(
            exit_frontiers.is_empty(),
            "the exit has one predecessor here"
        );
        assert!(order
            .last()
            .map(|block| Rc::ptr_eq(block, &header))
            .unwrap_or(false));
    }

    #[test]
    fn a_lone_block_and_an_empty_order_are_handled() {
        let alone = Block::new_ref();
        let order = post_order(&alone);
        compute_dominators(&order);
        compute_dominance_frontiers(&order);
        assert!(dominates(&alone, &alone));
        let entry_idom = alone.borrow().immediate_dominator.clone().expect("idom");
        assert!(Rc::ptr_eq(&entry_idom, &alone));

        compute_dominators(&Vec::new());
        compute_dominance_frontiers(&Vec::new());
    }
}
