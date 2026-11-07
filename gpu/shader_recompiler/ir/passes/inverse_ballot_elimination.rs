//! Inverse-ballot elimination, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/inverse_ballot_elimination_pass.cpp`.
//!
//! Pack/unpack and construct/extract pairs are folded first, then every `InverseBallot` whose
//! argument is built from ballots, bitwise operations or selects is rewritten in terms of the
//! ballots' predicate operands. Newly created inverse ballots go back on the worklist.

use crate::gpu::shader_recompiler::ir::basic_block::{prepend_new_inst, BlockRef};
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::microinstruction::{
    add_phi_operand, replace_uses_with, InstRef,
};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::passes::expression_elimination::run_local_cse;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::r#type::Type;
use crate::gpu::shader_recompiler::ir::value::{Value, U1, U64};
use std::collections::HashMap;
use std::rc::Rc;

fn key(inst: &InstRef) -> usize {
    Rc::as_ptr(inst) as usize
}

fn position(block: &BlockRef, inst: &InstRef) -> Option<usize> {
    block
        .borrow()
        .instructions
        .iter()
        .position(|candidate| Rc::ptr_eq(candidate, inst))
}

/// `FoldCompositeConstruct`: a construct of the extracts of one value is that value.
fn fold_composite_construct(inst: &InstRef, extract: Opcode) {
    let mut result = Value::empty();
    let num_args = inst.borrow().num_args();
    for index in 0..num_args {
        let value = inst.borrow().arg(index);
        if value.is_immediate() {
            return;
        }
        let arg_inst = value.inst();
        if arg_inst.borrow().get_opcode() != extract
            || arg_inst.borrow().arg(1).u32() != index as u32
        {
            return;
        }
        let composite = arg_inst.borrow().arg(0);
        if result.is_empty() {
            result = composite;
        } else if result != composite {
            return;
        }
    }
    replace_uses_with(inst, result);
}

/// `FoldInverseFunc`.
fn fold_inverse_func(inst: &InstRef, reverse: Opcode) {
    let value = inst.borrow().arg(0);
    if value.is_immediate() {
        return;
    }
    let arg_inst = value.inst();
    if arg_inst.borrow().get_opcode() == reverse {
        let arg = arg_inst.borrow().arg(0);
        replace_uses_with(inst, arg);
    }
}

/// `TrySimplifyInst`.
fn try_simplify_inst(inst: &InstRef) {
    let opcode = inst.borrow().get_opcode();
    match opcode {
        Opcode::PackUint2x32 => fold_inverse_func(inst, Opcode::UnpackUint2x32),
        Opcode::CompositeConstructU32x2 => {
            fold_composite_construct(inst, Opcode::CompositeExtractU32x2)
        }
        Opcode::CompositeConstructU32x3 => {
            fold_composite_construct(inst, Opcode::CompositeExtractU32x3)
        }
        Opcode::CompositeConstructU32x4 => {
            fold_composite_construct(inst, Opcode::CompositeExtractU32x4)
        }
        _ => {}
    }
}

/// `InverseBallotEliminationPass`.
pub fn inverse_ballot_elimination_pass(program: &Program) {
    let mut worklist: Vec<InstRef> = Vec::new();
    for block in &program.blocks {
        run_local_cse(block);
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            try_simplify_inst(inst);
            if inst.borrow().get_opcode() == Opcode::InverseBallot {
                worklist.push(inst.clone());
            }
        }
    }

    let mut inst_map: HashMap<usize, Value> = HashMap::new();
    while let Some(inst) = worklist.pop() {
        if inst.borrow().get_opcode() == Opcode::Void {
            continue;
        }
        let value = inst.borrow().arg(0);
        if value.is_immediate() {
            if value.u64() == 0 {
                replace_uses_with(&inst, Value::from_u1(false));
            } else if value.u64() == u64::MAX {
                replace_uses_with(&inst, Value::from_u1(true));
            } else {
                panic!(
                    "Unexpected immediate argument for InverseBallot {:#x}",
                    value.u64()
                );
            }
            continue;
        }

        let prod = value.inst();
        let prod_opcode = prod.borrow().get_opcode();
        if prod_opcode == Opcode::Ballot {
            let arg = prod.borrow().arg(0);
            replace_uses_with(&inst, arg);
            continue;
        }

        let block = prod.borrow().parent().expect("the producer has a parent");
        if prod_opcode == Opcode::UndefU64 {
            let index = position(&block, &prod).expect("the producer is in its block");
            let new_index = prepend_new_inst(&block, index, Opcode::UndefU1, &[], 0);
            let undef = block.borrow().instructions[new_index].clone();
            replace_uses_with(&inst, Value::from_inst(undef));
            continue;
        }

        if !matches!(
            prod_opcode,
            Opcode::BitwiseAnd64
                | Opcode::BitwiseNot64
                | Opcode::BitwiseOr64
                | Opcode::BitwiseXor64
                | Opcode::SelectU64
                | Opcode::Phi
        ) {
            continue;
        }

        let prod_key = key(&prod);
        if let std::collections::hash_map::Entry::Vacant(entry) = inst_map.entry(prod_key) {
            let index = position(&block, &prod).expect("the producer is in its block");
            let mapped = if prod_opcode == Opcode::Phi {
                let new_phi_index = prepend_new_inst(&block, index, Opcode::Phi, &[], 0);
                let new_phi = block.borrow().instructions[new_phi_index].clone();
                new_phi.borrow_mut().set_type_flags(Type::U1);
                let num_args = prod.borrow().num_args();
                for arg_index in 0..num_args {
                    let phi_block = prod.borrow().phi_block(arg_index);
                    let arg_value = prod.borrow().arg(arg_index);
                    let mut ir = IrEmitter::new(&phi_block);
                    let new_arg = ir.inverse_ballot(&U64::new(arg_value));
                    worklist.push(new_arg.value().inst());
                    add_phi_operand(&new_phi, &phi_block, new_arg.into_value());
                }
                Value::from_inst(new_phi)
            } else if prod_opcode == Opcode::SelectU64 {
                let mut ir = IrEmitter::at(&block, index);
                let first = U64::new(prod.borrow().arg(1));
                let a = ir.inverse_ballot(&first);
                worklist.push(a.value().inst());
                let second = U64::new(prod.borrow().arg(2));
                let b = ir.inverse_ballot(&second);
                worklist.push(b.value().inst());
                let selector = U1::new(prod.borrow().arg(0));
                ir.select(&selector, &a.into_value(), &b.into_value())
            } else {
                let mut ir = IrEmitter::at(&block, index);
                let first = U64::new(prod.borrow().arg(0));
                let a = ir.inverse_ballot(&first);
                worklist.push(a.value().inst());
                if prod_opcode == Opcode::BitwiseNot64 {
                    ir.logical_not(&a).into_value()
                } else {
                    let second = U64::new(prod.borrow().arg(1));
                    let b = ir.inverse_ballot(&second);
                    worklist.push(b.value().inst());
                    match prod_opcode {
                        Opcode::BitwiseAnd64 => ir.logical_and(&a, &b).into_value(),
                        Opcode::BitwiseOr64 => ir.logical_or(&a, &b).into_value(),
                        Opcode::BitwiseXor64 => ir.logical_xor(&a, &b).into_value(),
                        _ => panic!("Unreachable"),
                    }
                }
            };
            entry.insert(mapped);
        }

        let mapped = inst_map[&prod_key].clone();
        let uses = prod.borrow().uses().to_vec();
        for r#use in uses {
            if r#use.user.borrow().get_opcode() == Opcode::InverseBallot {
                replace_uses_with(&r#use.user, mapped.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block};
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage};
    use std::cell::RefCell;

    fn program(block: &BlockRef) -> Program {
        let info = Rc::new(RefCell::new(Info::new(
            HwStage::Vertex,
            SwStage::Vertex,
            ShaderParams::new([0; 16], &[], 0),
        )));
        let mut program = Program::new(info);
        program.blocks = vec![block.clone()];
        program
    }

    fn append(block: &BlockRef, op: Opcode, args: &[Value]) -> InstRef {
        let index = append_new_inst(block, op, args);
        block.borrow().instructions[index].clone()
    }

    fn opcodes(block: &BlockRef) -> Vec<Opcode> {
        block
            .borrow()
            .instructions()
            .iter()
            .map(|inst| inst.borrow().get_opcode())
            .collect()
    }

    #[test]
    fn pack_after_unpack_and_construct_of_extracts_fold() {
        let block = Block::new_ref();
        let x = append(&block, Opcode::GetScc, &[]);
        let unpack = append(
            &block,
            Opcode::UnpackUint2x32,
            &[Value::from_inst(x.clone())],
        );
        let pack = append(&block, Opcode::PackUint2x32, &[Value::from_inst(unpack)]);
        let pack_sink = append(&block, Opcode::ConvertF16F32, &[Value::from_inst(pack)]);

        let e0 = append(
            &block,
            Opcode::CompositeExtractU32x2,
            &[Value::from_inst(x.clone()), Value::from_u32(0)],
        );
        let e1 = append(
            &block,
            Opcode::CompositeExtractU32x2,
            &[Value::from_inst(x.clone()), Value::from_u32(1)],
        );
        let construct = append(
            &block,
            Opcode::CompositeConstructU32x2,
            &[Value::from_inst(e0), Value::from_inst(e1)],
        );
        let construct_sink = append(
            &block,
            Opcode::ConvertF16F32,
            &[Value::from_inst(construct)],
        );

        inverse_ballot_elimination_pass(&program(&block));
        assert!(Rc::ptr_eq(&pack_sink.borrow().arg(0).inst(), &x));
        assert!(Rc::ptr_eq(&construct_sink.borrow().arg(0).inst(), &x));
    }

    #[test]
    fn inverse_ballots_of_ballots_and_immediates_fold() {
        let block = Block::new_ref();
        let predicate = append(&block, Opcode::GetScc, &[]);
        let ballot = append(
            &block,
            Opcode::Ballot,
            &[Value::from_inst(predicate.clone())],
        );
        let inverse = append(&block, Opcode::InverseBallot, &[Value::from_inst(ballot)]);
        let inverse_sink = append(&block, Opcode::ConvertF16F32, &[Value::from_inst(inverse)]);
        let zero = append(&block, Opcode::InverseBallot, &[Value::from_u64(0)]);
        let zero_sink = append(&block, Opcode::ConvertF16F32, &[Value::from_inst(zero)]);
        let all = append(&block, Opcode::InverseBallot, &[Value::from_u64(u64::MAX)]);
        let all_sink = append(&block, Opcode::ConvertF16F32, &[Value::from_inst(all)]);

        inverse_ballot_elimination_pass(&program(&block));
        assert!(Rc::ptr_eq(&inverse_sink.borrow().arg(0).inst(), &predicate));
        assert!(!zero_sink.borrow().arg(0).u1());
        assert!(all_sink.borrow().arg(0).u1());
    }

    #[test]
    fn a_bitwise_combination_becomes_logical_operations() {
        let block = Block::new_ref();
        let x = append(&block, Opcode::GetScc, &[]);
        let y = append(&block, Opcode::GetScc, &[]);
        let ballot_x = append(&block, Opcode::Ballot, &[Value::from_inst(x.clone())]);
        let ballot_y = append(&block, Opcode::Ballot, &[Value::from_inst(y.clone())]);
        let and = append(
            &block,
            Opcode::BitwiseAnd64,
            &[Value::from_inst(ballot_x), Value::from_inst(ballot_y)],
        );
        let inverse = append(&block, Opcode::InverseBallot, &[Value::from_inst(and)]);
        let sink = append(&block, Opcode::ConvertF16F32, &[Value::from_inst(inverse)]);

        inverse_ballot_elimination_pass(&program(&block));
        let logical_and = sink.borrow().arg(0).inst();
        assert_eq!(logical_and.borrow().get_opcode(), Opcode::LogicalAnd);
        assert!(Rc::ptr_eq(&logical_and.borrow().arg(0).inst(), &x));
        assert!(Rc::ptr_eq(&logical_and.borrow().arg(1).inst(), &y));
    }

    #[test]
    fn a_select_becomes_a_u1_select() {
        let block = Block::new_ref();
        let condition = append(&block, Opcode::GetScc, &[]);
        let x = append(&block, Opcode::GetScc, &[]);
        let y = append(&block, Opcode::GetScc, &[]);
        let ballot_x = append(&block, Opcode::Ballot, &[Value::from_inst(x.clone())]);
        let ballot_y = append(&block, Opcode::Ballot, &[Value::from_inst(y.clone())]);
        let select = append(
            &block,
            Opcode::SelectU64,
            &[
                Value::from_inst(condition.clone()),
                Value::from_inst(ballot_x),
                Value::from_inst(ballot_y),
            ],
        );
        let inverse = append(&block, Opcode::InverseBallot, &[Value::from_inst(select)]);
        let sink = append(&block, Opcode::ConvertF16F32, &[Value::from_inst(inverse)]);

        inverse_ballot_elimination_pass(&program(&block));
        let selected = sink.borrow().arg(0).inst();
        assert_eq!(selected.borrow().get_opcode(), Opcode::SelectU1);
        assert!(Rc::ptr_eq(&selected.borrow().arg(0).inst(), &condition));
        assert!(Rc::ptr_eq(&selected.borrow().arg(1).inst(), &x));
        assert!(Rc::ptr_eq(&selected.borrow().arg(2).inst(), &y));
    }

    #[test]
    fn an_undefined_mask_becomes_an_undefined_u1() {
        let block = Block::new_ref();
        let undef = append(&block, Opcode::UndefU64, &[]);
        let inverse = append(&block, Opcode::InverseBallot, &[Value::from_inst(undef)]);
        let sink = append(&block, Opcode::ConvertF16F32, &[Value::from_inst(inverse)]);

        inverse_ballot_elimination_pass(&program(&block));
        assert!(opcodes(&block).contains(&Opcode::UndefU1));
        assert_eq!(
            sink.borrow().arg(0).inst().borrow().get_opcode(),
            Opcode::UndefU1
        );
    }
}
