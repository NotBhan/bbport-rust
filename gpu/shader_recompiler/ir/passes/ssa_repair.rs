//! SSA repair, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/ssa_repair_pass.cpp`. The pass recomputes
//! dominance on the block order and, for every value with a use in a block it does not dominate,
//! stores the value in a fresh virtual register at its definition and loads it at each such use.

use crate::gpu::shader_recompiler::ir::basic_block::BlockRef;
use crate::gpu::shader_recompiler::ir::dominance::{
    compute_dominance_frontiers, compute_dominators, dominates,
};
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::microinstruction::{set_arg, InstRef, Use};
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::reg::VirtualReg;
use crate::gpu::shader_recompiler::ir::value::Value;
use std::rc::Rc;

fn position(block: &BlockRef, inst: &InstRef) -> Option<usize> {
    block
        .borrow()
        .instructions
        .iter()
        .position(|candidate| Rc::ptr_eq(candidate, inst))
}

/// `SsaRepairPass`.
pub fn ssa_repair_pass(program: &mut Program) {
    let post_order = program.post_order_blocks.clone();
    compute_dominators(&post_order);
    compute_dominance_frontiers(&post_order);

    for block in &post_order {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            let uses = inst.borrow().uses().to_vec();
            if uses.is_empty() {
                continue;
            }

            let invalid_uses: Vec<Use> = uses
                .into_iter()
                .filter(|r#use| {
                    let user_block = r#use
                        .user
                        .borrow()
                        .parent()
                        .expect("the use has a parent block");
                    !dominates(block, &user_block)
                })
                .collect();
            if invalid_uses.is_empty() {
                continue;
            }

            let ty = inst.borrow().ty();
            let reg = VirtualReg::new(program.next_reg_index, ty);
            program.next_reg_index += 1;

            let index = position(block, inst).expect("the instruction is in its block") + 1;
            let mut ir = IrEmitter::at(block, index);
            let value = Value::from_inst(inst.clone());
            ir.set_virtual_reg(reg, &value);

            for r#use in invalid_uses {
                let user_block = r#use
                    .user
                    .borrow()
                    .parent()
                    .expect("the use has a parent block");
                let user_index =
                    position(&user_block, &r#use.user).expect("the use is in its block");
                let mut ir = IrEmitter::at(&user_block, user_index);
                let value = ir.get_virtual_reg(reg);
                set_arg(&r#use.user, r#use.operand as usize, value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::{add_branch, append_new_inst, Block};
    use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
    use crate::gpu::shader_recompiler::ir::post_order::post_order;
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage};
    use std::cell::RefCell;

    fn program() -> Program {
        let info = Rc::new(RefCell::new(Info::new(
            HwStage::Vertex,
            SwStage::Vertex,
            ShaderParams::new([0; 16], &[], 0),
        )));
        Program::new(info)
    }

    fn append(block: &BlockRef, op: Opcode, args: &[Value]) -> InstRef {
        let index = append_new_inst(block, op, args);
        block.borrow().instructions[index].clone()
    }

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
    fn a_use_outside_the_definitions_dominance_gets_a_virtual_register() {
        let (entry, left, right, _join) = diamond();
        let producer = append(&left, Opcode::GetScc, &[]);
        let user = append(
            &right,
            Opcode::ConvertF16F32,
            &[Value::from_inst(producer.clone())],
        );

        let mut program = program();
        program.post_order_blocks = post_order(&entry);
        ssa_repair_pass(&mut program);

        assert_eq!(program.next_reg_index, 1);
        let reg = VirtualReg::new(0, crate::gpu::shader_recompiler::ir::r#type::Type::U1);

        let left_instructions = left.borrow().instructions.clone();
        assert_eq!(left_instructions.len(), 2);
        let store = &left_instructions[1];
        assert_eq!(store.borrow().get_opcode(), Opcode::SetVirtualRegister);
        assert_eq!(store.borrow().arg(0).virtual_reg(), reg);
        assert!(Rc::ptr_eq(&store.borrow().arg(1).inst(), &producer));

        let right_instructions = right.borrow().instructions.clone();
        assert_eq!(right_instructions.len(), 2);
        let load = &right_instructions[0];
        assert_eq!(load.borrow().get_opcode(), Opcode::GetVirtualRegister);
        assert_eq!(load.borrow().arg(0).virtual_reg(), reg);
        assert!(Rc::ptr_eq(&user.borrow().arg(0).inst(), load));
    }

    #[test]
    fn a_dominated_use_is_left_alone() {
        let (entry, _left, _right, join) = diamond();
        let producer = append(&entry, Opcode::GetScc, &[]);
        let user = append(
            &join,
            Opcode::ConvertF16F32,
            &[Value::from_inst(producer.clone())],
        );

        let mut program = program();
        program.post_order_blocks = post_order(&entry);
        ssa_repair_pass(&mut program);

        assert_eq!(program.next_reg_index, 0);
        assert_eq!(entry.borrow().instructions.len(), 1);
        assert_eq!(join.borrow().instructions.len(), 1);
        assert!(Rc::ptr_eq(&user.borrow().arg(0).inst(), &producer));
    }

    #[test]
    fn a_value_without_uses_is_skipped() {
        let (entry, _left, _right, _join) = diamond();
        append(&entry, Opcode::GetScc, &[]);

        let mut program = program();
        program.post_order_blocks = post_order(&entry);
        ssa_repair_pass(&mut program);

        assert_eq!(program.next_reg_index, 0);
        assert_eq!(entry.borrow().instructions.len(), 1);
    }
}
