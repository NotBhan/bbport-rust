//! Dead code elimination, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/dead_code_elimination_pass.cpp`.
//!
//! The C++ takes an `IR::Program` and reads only its `post_order_blocks`, so the port takes
//! that list; `IR::Program` itself is still unported. The pass runs in the three sweeps the
//! C++ does: a backwards sweep that drops instructions with no uses and no side effects, a
//! worklist that marks everything reachable from a side-effecting instruction as live, and a
//! forwards sweep that drops whatever is not live.

use crate::gpu::shader_recompiler::ir::basic_block::{BlockList, BlockRef};
use crate::gpu::shader_recompiler::ir::microinstruction::{invalidate, InstRef};
use std::collections::HashSet;
use std::rc::Rc;

/// `Shader::Optimization::DeadCodeEliminationPass`.
pub fn dead_code_elimination_pass(post_order_blocks: &BlockList) {
    // We iterate over the instructions in reverse order.
    // This is because removing an instruction reduces the number of uses for earlier
    // instructions.
    let mut live: HashSet<usize> = HashSet::new();
    let mut worklist: Vec<InstRef> = Vec::new();

    for block in post_order_blocks {
        let instructions = block.borrow().instructions.clone();
        for inst in instructions.iter().rev() {
            let has_side_effects = inst.borrow().may_have_side_effects();
            if !inst.borrow().has_uses() && !has_side_effects {
                invalidate(inst);
                remove(block, inst);
            } else if has_side_effects {
                live.insert(Rc::as_ptr(inst) as usize);
                worklist.push(Rc::clone(inst));
            }
        }
    }

    while let Some(inst) = worklist.pop() {
        let arg_count = inst.borrow().num_args();
        for index in 0..arg_count {
            let arg = inst.borrow().arg(index);
            if arg.is_immediate() {
                continue;
            }
            let operand = arg.inst();
            if live.insert(Rc::as_ptr(&operand) as usize) {
                worklist.push(operand);
            }
        }
    }

    for block in post_order_blocks {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            if !live.contains(&(Rc::as_ptr(inst) as usize)) {
                invalidate(inst);
                remove(block, inst);
            }
        }
    }
}

/// `block->Instructions().erase(it)`, which in the port is the removal from the block's list.
fn remove(block: &BlockRef, inst: &InstRef) {
    block
        .borrow_mut()
        .instructions
        .retain(|candidate| !Rc::ptr_eq(candidate, inst));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block};
    use crate::gpu::shader_recompiler::ir::microinstruction::Inst;
    use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
    use crate::gpu::shader_recompiler::ir::value::Value;
    use std::cell::RefCell;

    #[test]
    fn instructions_without_uses_or_effects_are_dropped() {
        let block = Block::new_ref();
        // A dead ReadConst: nothing reads it and it does nothing.
        let dead_point = append_new_inst(
            &block,
            Opcode::ReadConst,
            &[Value::from_u32(0), Value::from_u32(0)],
        );
        let dead = block.borrow().instructions[dead_point].clone();
        // A barrier stays: it has side effects even though nothing reads it.
        let barrier_point = append_new_inst(&block, Opcode::Barrier, &[]);
        let barrier = block.borrow().instructions[barrier_point].clone();
        // A store stays, and keeps the instruction feeding it alive.
        let source = std::rc::Rc::new(RefCell::new(Inst::new(Opcode::ReadConst, 0)));
        source.borrow_mut().set_parent_handle(&block);
        block.borrow_mut().instructions.push(source.clone());
        let store_point = append_new_inst(
            &block,
            Opcode::WriteSharedU32,
            &[Value::from_u32(0), Value::from_inst(source.clone())],
        );
        let store = block.borrow().instructions[store_point].clone();

        let list: BlockList = vec![block.clone()];
        dead_code_elimination_pass(&list);

        assert_eq!(
            dead.borrow().get_opcode(),
            Opcode::Void,
            "the dead read is void"
        );
        assert!(!block
            .borrow()
            .instructions
            .iter()
            .any(|inst| std::rc::Rc::ptr_eq(inst, &dead)));
        assert!(block
            .borrow()
            .instructions
            .iter()
            .any(|inst| std::rc::Rc::ptr_eq(inst, &barrier)));
        assert_eq!(
            barrier.borrow().get_opcode(),
            Opcode::Barrier,
            "a side effect stays"
        );
        assert!(block
            .borrow()
            .instructions
            .iter()
            .any(|inst| std::rc::Rc::ptr_eq(inst, &store)));
        assert_eq!(
            source.borrow().get_opcode(),
            Opcode::ReadConst,
            "the store's source is live"
        );
        assert!(block
            .borrow()
            .instructions
            .iter()
            .any(|inst| std::rc::Rc::ptr_eq(inst, &source)));
    }

    #[test]
    fn a_live_chain_up_to_an_immediate_is_kept_whole() {
        let block = Block::new_ref();
        let load_point = append_new_inst(
            &block,
            Opcode::ReadConst,
            &[Value::from_u32(0), Value::from_u32(0)],
        );
        let load = block.borrow().instructions[load_point].clone();
        let store_point = append_new_inst(
            &block,
            Opcode::WriteSharedU32,
            &[Value::from_u32(0), Value::from_inst(load.clone())],
        );
        assert!(store_point < block.borrow().instructions.len());

        let list: BlockList = vec![block.clone()];
        dead_code_elimination_pass(&list);
        assert_eq!(block.borrow().instructions.len(), 2, "both survive");
        assert_eq!(load.borrow().use_count(), 1);
    }

    #[test]
    fn an_empty_program_is_handled() {
        dead_code_elimination_pass(&Vec::new());
        let empty = Block::new_ref();
        dead_code_elimination_pass(&vec![empty.clone()]);
        assert!(empty.borrow().instructions.is_empty());
    }
}
