//! Expression elimination, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/expression_elimination.h`. The C++ is
//! header-only and runs a local common-subexpression elimination over the four opcodes whose
//! results are pure functions of their arguments.

use crate::gpu::shader_recompiler::ir::basic_block::BlockRef;
use crate::gpu::shader_recompiler::ir::microinstruction::{is_phi, replace_uses_with, InstRef};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::value::Value;
use std::rc::Rc;

/// `IsAllowedInst`: only these four are considered, as their results depend on nothing else.
pub fn is_allowed_inst(inst: &InstRef) -> bool {
    matches!(
        inst.borrow().get_opcode(),
        Opcode::Ballot | Opcode::UnpackUint2x32 | Opcode::BitCastU32F32 | Opcode::BitCastF32U32
    )
}

/// `IsIdenticalInst`: the same opcode and the same arguments, value by value.
pub fn is_identical_inst(a: &InstRef, b: &InstRef) -> bool {
    if a.borrow().get_opcode() != b.borrow().get_opcode() {
        return false;
    }
    let arg_count = a.borrow().num_args();
    if arg_count != b.borrow().num_args() {
        return false;
    }
    for index in 0..arg_count {
        let lhs = a.borrow().arg(index);
        let rhs = b.borrow().arg(index);
        if lhs != rhs {
            return false;
        }
    }
    true
}

/// `RunLocalCSE`: walks a block from its first non-phi instruction, and rewires a repeat of an
/// earlier instruction to that earlier one. The replaced instruction is left void, which is what
/// `ReplaceUsesWithAndRemove` does.
pub fn run_local_cse(block: &BlockRef) {
    let instructions = block.borrow().instructions.clone();
    // `std::ranges::find_if_not(*block, IR::IsPhi)`: the phi nodes at the front are skipped.
    let Some(start) = instructions.iter().position(|inst| !is_phi(inst)) else {
        return;
    };

    let mut seen: Vec<InstRef> = Vec::new();
    for inst in instructions[start..].iter() {
        if !is_allowed_inst(inst) {
            continue;
        }
        match seen
            .iter()
            .find(|earlier| is_identical_inst(inst, earlier))
            .cloned()
        {
            Some(earlier) => replace_uses_with(inst, Value::from_inst(earlier)),
            None => seen.push(Rc::clone(inst)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::microinstruction::{add_phi_operand, set_arg, Inst, InstRef};
    use super::super::super::value::Value;
    use super::*;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block, BlockRef};
    use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
    use std::rc::Rc;

    fn bit_cast(block: &BlockRef, arg: Value) -> InstRef {
        let point = append_new_inst(block, Opcode::BitCastU32F32, &[arg]);
        block.borrow().instructions[point].clone()
    }

    #[test]
    fn only_the_four_allowed_opcodes_are_considered() {
        let block = Block::new_ref();
        let source = append_new_inst(
            &block,
            Opcode::ReadConst,
            &[Value::from_u32(0), Value::from_u32(0)],
        );
        let read = block.borrow().instructions[source].clone();
        assert!(!is_allowed_inst(&read));
        let cast = bit_cast(&block, Value::from_u32(1));
        assert!(is_allowed_inst(&cast));
        for opcode in [
            Opcode::Ballot,
            Opcode::UnpackUint2x32,
            Opcode::BitCastU32F32,
            Opcode::BitCastF32U32,
        ] {
            assert!(is_allowed_inst(&Rc::new(std::cell::RefCell::new(
                Inst::new(opcode, 0)
            ))));
        }
    }

    #[test]
    fn a_repeated_instruction_is_replaced_by_the_earlier_one() {
        let block = Block::new_ref();
        let first = bit_cast(&block, Value::from_u32(0x3F80_0000));
        let second = bit_cast(&block, Value::from_u32(0x3F80_0000));
        // A reader of the second cast keeps its argument once the pass runs.
        let reader_point = append_new_inst(
            &block,
            Opcode::IAdd32,
            &[Value::from_inst(second.clone()), Value::from_u32(1)],
        );
        let reader = block.borrow().instructions[reader_point].clone();
        assert_eq!(second.borrow().use_count(), 1);

        run_local_cse(&block);
        assert_eq!(
            first.borrow().use_count(),
            1,
            "the reader moved to the first cast"
        );
        assert_eq!(second.borrow().use_count(), 0);
        assert_eq!(
            second.borrow().get_opcode(),
            Opcode::Void,
            "the duplicate is invalidated"
        );
        assert!(Rc::ptr_eq(&reader.borrow().arg(0).inst(), &first));
        assert!(is_identical_inst(&first, &first));
    }

    #[test]
    fn different_arguments_are_left_alone_and_phis_are_skipped() {
        let block = Block::new_ref();
        let first = bit_cast(&block, Value::from_u32(1));
        let second = bit_cast(&block, Value::from_u32(2));
        run_local_cse(&block);
        assert_ne!(first.borrow().get_opcode(), Opcode::Void);
        assert_ne!(second.borrow().get_opcode(), Opcode::Void);
        assert!(!is_identical_inst(&first, &second));
        // Different opcodes are never identical.
        let other = append_new_inst(&block, Opcode::BitCastF32U32, &[Value::from_u32(1)]);
        let other = block.borrow().instructions[other].clone();
        assert!(!is_identical_inst(&first, &other));

        // A phi node at the front does not confuse the walk.
        let phi = Rc::new(std::cell::RefCell::new(Inst::new(Opcode::Phi, 0)));
        phi.borrow_mut()
            .set_type_flags(crate::gpu::shader_recompiler::ir::r#type::Type::F32);
        add_phi_operand(&phi, &Block::new_ref(), Value::from_u32(0));
        block.borrow_mut().instructions.insert(0, phi.clone());
        run_local_cse(&block);
        assert_ne!(phi.borrow().get_opcode(), Opcode::Void);
        // The two identical u32 casts collapse to one, and the earlier one survives.
        let live = block
            .borrow()
            .instructions
            .iter()
            .filter(|inst| inst.borrow().get_opcode() == Opcode::BitCastU32F32)
            .count();
        assert_eq!(live, 2, "the two casts had different arguments");
        let _ = set_arg;
    }
}
