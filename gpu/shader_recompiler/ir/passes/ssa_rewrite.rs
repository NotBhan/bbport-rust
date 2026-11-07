//! SSA rewriting, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/ssa_rewrite_pass.cpp`.
//!
//! This is the algorithm from Braun et al., "Simple and Efficient Construction of Static Single
//! Assignment Form": register reads are rewritten to the definition reaching the block, a phi is
//! created at joins, and trivial phis are folded afterwards. Blocks are visited in reverse
//! post-order.

use crate::gpu::shader_recompiler::ir::basic_block::{prepend_new_inst, BlockRef};
use crate::gpu::shader_recompiler::ir::microinstruction::{
    add_phi_operand, is_phi, replace_uses_with, InstRef,
};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::r#type::Type;
use crate::gpu::shader_recompiler::ir::ssa::{RegTag, RegTagRegister, RegType};
use crate::gpu::shader_recompiler::ir::value::Value;
use std::collections::HashMap;
use std::rc::Rc;

fn key(block: &BlockRef) -> usize {
    Rc::as_ptr(block) as usize
}

fn virtual_reg(tag: RegTag) -> crate::gpu::shader_recompiler::ir::reg::VirtualReg {
    match tag.register() {
        RegTagRegister::Virtual(reg) => reg,
        _ => unreachable!("the tag does not carry a virtual register"),
    }
}

/// `TypeOf`.
fn type_of(tag: RegTag) -> Type {
    match tag.ty {
        RegType::ScalarReg | RegType::VectorReg | RegType::VccLo | RegType::VccHi | RegType::M0 => {
            Type::U32
        }
        RegType::Scc | RegType::Exec | RegType::GotoVariable => Type::U1,
        RegType::VirtualReg => virtual_reg(tag).ty,
        _ => unreachable!("unknown register type"),
    }
}

/// `UndefOpcode`.
fn undef_opcode(tag: RegTag) -> Opcode {
    match tag.ty {
        RegType::ScalarReg | RegType::VectorReg | RegType::VccLo | RegType::VccHi | RegType::M0 => {
            Opcode::UndefU32
        }
        RegType::Scc | RegType::Exec | RegType::GotoVariable => Opcode::UndefU1,
        RegType::VirtualReg => match virtual_reg(tag).ty {
            Type::U64 => Opcode::UndefU64,
            Type::U32 => Opcode::UndefU32,
            Type::U32x2 => Opcode::UndefU32x2,
            Type::U32x3 => Opcode::UndefU32x3,
            Type::U32x4 => Opcode::UndefU32x4,
            Type::F32 => Opcode::UndefF32,
            Type::U1 => Opcode::UndefU1,
            other => panic!("Unknown virtual reg type {:?}", other),
        },
        _ => unreachable!("unknown register type"),
    }
}

/// `DefTable`.
#[derive(Default)]
struct DefTable {
    goto_vars: HashMap<u32, HashMap<usize, Value>>,
    reg_vars: HashMap<u64, HashMap<usize, Value>>,
}

impl DefTable {
    /// `Def`.
    fn def(&self, block: &BlockRef, tag: RegTag) -> Value {
        if tag.is_intrusive() {
            return block.borrow().ssa_state.values[tag.index() as usize].clone();
        }
        match tag.ty {
            RegType::GotoVariable => self
                .goto_vars
                .get(&tag.raw_index())
                .and_then(|values| values.get(&key(block)))
                .cloned()
                .unwrap_or_else(Value::empty),
            RegType::VirtualReg => self
                .reg_vars
                .get(&virtual_reg(tag).key())
                .and_then(|values| values.get(&key(block)))
                .cloned()
                .unwrap_or_else(Value::empty),
            _ => unreachable!("unknown register type"),
        }
    }

    /// `SetDef`.
    fn set_def(&mut self, block: &BlockRef, tag: RegTag, value: Value) {
        if tag.is_intrusive() {
            block.borrow_mut().ssa_state.values[tag.index() as usize] = value;
            return;
        }
        match tag.ty {
            RegType::GotoVariable => {
                self.goto_vars
                    .entry(tag.raw_index())
                    .or_default()
                    .insert(key(block), value);
            }
            RegType::VirtualReg => {
                self.reg_vars
                    .entry(virtual_reg(tag).key())
                    .or_default()
                    .insert(key(block), value);
            }
            _ => unreachable!("unknown register type"),
        }
    }
}

/// `Pass`.
#[derive(Default)]
struct Pass {
    current_def: DefTable,
    pending_phis: Vec<InstRef>,
}

impl Pass {
    fn write_variable(&mut self, tag: RegTag, block: &BlockRef, value: Value) {
        self.current_def.set_def(block, tag, value);
    }

    /// `ReadVariable`.
    fn read_variable(&mut self, tag: RegTag, block: &BlockRef) -> Value {
        let mut chain: Vec<BlockRef> = Vec::new();
        let mut block = block.clone();
        let result: Value;

        loop {
            let def = self.current_def.def(&block, tag);
            if !def.is_empty() {
                result = def;
                break;
            }

            let predecessors = block.borrow().imm_predecessors().to_vec();
            if predecessors.len() == 1 {
                // Optimize the common case of one predecessor: no phi needed.
                chain.push(block);
                block = predecessors[0].clone();
                continue;
            } else if predecessors.is_empty() {
                let index = prepend_new_inst(&block, 0, undef_opcode(tag), &[], 0);
                result = Value::from_inst(block.borrow().instructions[index].clone());
                self.write_variable(tag, &block, result.clone());
                break;
            }

            // This is a join block which may require a phi. It is the current definition so a
            // potential cycle in the SSA graph is broken.
            let index = prepend_new_inst(&block, 0, Opcode::Phi, &[], 0);
            let phi = block.borrow().instructions[index].clone();
            phi.borrow_mut().set_type_flags(type_of(tag));
            phi.borrow_mut().set_reg_tag(tag);

            result = Value::from_inst(phi.clone());
            self.write_variable(tag, &block, result.clone());
            self.pending_phis.push(phi);
            break;
        }

        for chained in chain.iter().rev() {
            self.write_variable(tag, chained, result.clone());
        }
        result
    }

    /// `EvaluatePendingPhis`. Reading a predecessor may add more phis to the end of the list.
    fn evaluate_pending_phis(&mut self) {
        let mut index = 0;
        while index < self.pending_phis.len() {
            let phi = self.pending_phis[index].clone();
            let block = phi.borrow().parent().expect("the phi has a parent");
            let tag = phi.borrow().reg_tag();
            let predecessors = block.borrow().imm_predecessors().to_vec();
            for pred in predecessors {
                let value = self.read_variable(tag, &pred);
                add_phi_operand(&phi, &pred, value);
            }
            index += 1;
        }
    }

    /// `ResolveTrivialPhis`.
    fn resolve_trivial_phis(&mut self) {
        let mut worklist = std::mem::take(&mut self.pending_phis);
        while let Some(phi) = worklist.pop() {
            if phi.borrow().get_opcode() == Opcode::Void {
                continue;
            }

            let mut same = Value::empty();
            let mut non_trivial = false;
            let num_args = phi.borrow().num_args();
            for index in 0..num_args {
                let op = phi.borrow().arg(index);
                if op == same || op == Value::from_inst(phi.clone()) {
                    // A unique value or a self-reference.
                    continue;
                }
                if !same.is_empty() {
                    // The phi merges at least two values: not trivial.
                    non_trivial = true;
                    break;
                }
                same = op;
            }
            if non_trivial {
                continue;
            }

            let block = phi.borrow().parent().expect("the phi has a parent");
            if same.is_empty() {
                // All operands are self-references, or the phi has no operands.
                let instructions = block.borrow().instructions.clone();
                let point = instructions
                    .iter()
                    .position(|inst| !is_phi(inst))
                    .unwrap_or(instructions.len());
                let index =
                    prepend_new_inst(&block, point, undef_opcode(phi.borrow().reg_tag()), &[], 0);
                same = Value::from_inst(block.borrow().instructions[index].clone());
            }

            // Phi users may have become trivial too.
            let uses = phi.borrow().uses().to_vec();
            for r#use in uses {
                if r#use.user.borrow().get_opcode() == Opcode::Phi && !Rc::ptr_eq(&r#use.user, &phi)
                {
                    worklist.push(r#use.user.clone());
                }
            }
            replace_uses_with(&phi, same);
            block
                .borrow_mut()
                .instructions
                .retain(|candidate| !Rc::ptr_eq(candidate, &phi));
        }
    }
}

/// `VisitInst`.
fn visit_inst(pass: &mut Pass, block: &BlockRef, inst: &InstRef) {
    let opcode = inst.borrow().get_opcode();
    match opcode {
        Opcode::SetScalarRegister => {
            let tag = RegTag::from_scalar_reg(inst.borrow().arg(0).scalar_reg());
            let value = inst.borrow().arg(1);
            pass.write_variable(tag, block, value);
        }
        Opcode::SetVectorRegister => {
            let tag = RegTag::from_vector_reg(inst.borrow().arg(0).vector_reg());
            let value = inst.borrow().arg(1);
            pass.write_variable(tag, block, value);
        }
        Opcode::SetVirtualRegister => {
            let tag = RegTag::from_virtual_reg(inst.borrow().arg(0).virtual_reg());
            let value = inst.borrow().arg(1);
            pass.write_variable(tag, block, value);
        }
        Opcode::SetGotoVariable => {
            let tag = RegTag::new(RegType::GotoVariable, inst.borrow().arg(0).u32());
            let value = inst.borrow().arg(1);
            pass.write_variable(tag, block, value);
        }
        Opcode::SetExec => {
            let value = inst.borrow().arg(0);
            pass.write_variable(RegTag::new(RegType::Exec, 0), block, value);
        }
        Opcode::SetScc => {
            let value = inst.borrow().arg(0);
            pass.write_variable(RegTag::new(RegType::Scc, 0), block, value);
        }
        Opcode::SetVccLo => {
            let value = inst.borrow().arg(0);
            pass.write_variable(RegTag::new(RegType::VccLo, 0), block, value);
        }
        Opcode::SetVccHi => {
            let value = inst.borrow().arg(0);
            pass.write_variable(RegTag::new(RegType::VccHi, 0), block, value);
        }
        Opcode::SetM0 => {
            let value = inst.borrow().arg(0);
            pass.write_variable(RegTag::new(RegType::M0, 0), block, value);
        }
        Opcode::GetScalarRegister => {
            let tag = RegTag::from_scalar_reg(inst.borrow().arg(0).scalar_reg());
            let value = pass.read_variable(tag, block);
            replace_uses_with(inst, value);
        }
        Opcode::GetVectorRegister => {
            let tag = RegTag::from_vector_reg(inst.borrow().arg(0).vector_reg());
            let value = pass.read_variable(tag, block);
            replace_uses_with(inst, value);
        }
        Opcode::GetVirtualRegister => {
            let tag = RegTag::from_virtual_reg(inst.borrow().arg(0).virtual_reg());
            let value = pass.read_variable(tag, block);
            replace_uses_with(inst, value);
        }
        Opcode::GetGotoVariable => {
            let tag = RegTag::new(RegType::GotoVariable, inst.borrow().arg(0).u32());
            let value = pass.read_variable(tag, block);
            replace_uses_with(inst, value);
        }
        Opcode::GetExec => {
            let value = pass.read_variable(RegTag::new(RegType::Exec, 0), block);
            replace_uses_with(inst, value);
        }
        Opcode::GetScc => {
            let value = pass.read_variable(RegTag::new(RegType::Scc, 0), block);
            replace_uses_with(inst, value);
        }
        Opcode::GetVccLo => {
            let value = pass.read_variable(RegTag::new(RegType::VccLo, 0), block);
            replace_uses_with(inst, value);
        }
        Opcode::GetVccHi => {
            let value = pass.read_variable(RegTag::new(RegType::VccHi, 0), block);
            replace_uses_with(inst, value);
        }
        Opcode::GetM0 => {
            let value = pass.read_variable(RegTag::new(RegType::M0, 0), block);
            replace_uses_with(inst, value);
        }
        _ => {}
    }
}

/// `SsaRewritePass`.
pub fn ssa_rewrite_pass(program: &Program) {
    let mut pass = Pass::default();
    for block in program.post_order_blocks.iter().rev() {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            visit_inst(&mut pass, block, inst);
        }
    }
    pass.evaluate_pending_phis();
    pass.resolve_trivial_phis();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block};
    use crate::gpu::shader_recompiler::ir::reg::ScalarReg;
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage};
    use std::cell::RefCell;

    fn program(blocks: &[BlockRef]) -> Program {
        let info = Rc::new(RefCell::new(Info::new(
            HwStage::Vertex,
            SwStage::Vertex,
            ShaderParams::new([0; 16], &[], 0),
        )));
        let mut program = Program::new(info);
        program.blocks = blocks.to_vec();
        program.post_order_blocks = blocks.to_vec();
        program
    }

    fn append(block: &BlockRef, op: Opcode, args: &[Value]) -> InstRef {
        let index = append_new_inst(block, op, args);
        block.borrow().instructions[index].clone()
    }

    fn get_s1(block: &BlockRef) -> InstRef {
        append(
            block,
            Opcode::GetScalarRegister,
            &[Value::from_scalar_reg(ScalarReg::S1)],
        )
    }

    #[test]
    fn a_read_without_a_definition_becomes_an_undef() {
        let block = Block::new_ref();
        let first = get_s1(&block);
        let first_sink = append(
            &block,
            Opcode::BitCastF32U32,
            &[Value::from_inst(first.clone())],
        );
        let second = get_s1(&block);
        let second_sink = append(
            &block,
            Opcode::BitCastF32U32,
            &[Value::from_inst(second.clone())],
        );

        let program = program(std::slice::from_ref(&block));
        ssa_rewrite_pass(&program);

        assert_eq!(first.borrow().get_opcode(), Opcode::Void);
        let undef = first_sink.borrow().arg(0).inst();
        assert_eq!(undef.borrow().get_opcode(), Opcode::UndefU32);
        assert!(Rc::ptr_eq(&second_sink.borrow().arg(0).inst(), &undef));
    }

    #[test]
    fn a_read_after_a_write_uses_the_written_value() {
        let block = Block::new_ref();
        let value = append(&block, Opcode::LaneId, &[]);
        append(
            &block,
            Opcode::SetScalarRegister,
            &[
                Value::from_scalar_reg(ScalarReg::S2),
                Value::from_inst(value.clone()),
            ],
        );
        let read = get_s2(&block);
        let sink = append(
            &block,
            Opcode::BitCastF32U32,
            &[Value::from_inst(read.clone())],
        );

        let program = program(std::slice::from_ref(&block));
        ssa_rewrite_pass(&program);

        assert_eq!(read.borrow().get_opcode(), Opcode::Void);
        assert!(Rc::ptr_eq(&sink.borrow().arg(0).inst(), &value));
    }

    fn get_s2(block: &BlockRef) -> InstRef {
        append(
            block,
            Opcode::GetScalarRegister,
            &[Value::from_scalar_reg(ScalarReg::S2)],
        )
    }

    #[test]
    fn a_join_gets_a_phi_which_a_trivial_result_folds_away() {
        let left = Block::new_ref();
        let right = Block::new_ref();
        let merge = Block::new_ref();
        let value = append(&left, Opcode::LaneId, &[]);
        append(
            &left,
            Opcode::SetScalarRegister,
            &[
                Value::from_scalar_reg(ScalarReg::S1),
                Value::from_inst(value.clone()),
            ],
        );
        append(
            &right,
            Opcode::SetScalarRegister,
            &[
                Value::from_scalar_reg(ScalarReg::S1),
                Value::from_inst(value.clone()),
            ],
        );
        merge.borrow_mut().imm_predecessors = vec![left.clone(), right.clone()];
        let read = get_s1(&merge);
        let sink = append(
            &merge,
            Opcode::BitCastF32U32,
            &[Value::from_inst(read.clone())],
        );

        let program = program(&[left, right, merge.clone()]);
        ssa_rewrite_pass(&program);

        assert_eq!(read.borrow().get_opcode(), Opcode::Void);
        assert!(Rc::ptr_eq(&sink.borrow().arg(0).inst(), &value));
        assert!(merge
            .borrow()
            .instructions()
            .iter()
            .all(|inst| inst.borrow().get_opcode() != Opcode::Phi));
    }

    #[test]
    fn a_join_with_distinct_values_keeps_its_phi() {
        let left = Block::new_ref();
        let right = Block::new_ref();
        let merge = Block::new_ref();
        let x = append(&left, Opcode::LaneId, &[]);
        let y = append(&right, Opcode::LaneId, &[]);
        append(
            &left,
            Opcode::SetScalarRegister,
            &[Value::from_scalar_reg(ScalarReg::S1), Value::from_inst(x)],
        );
        append(
            &right,
            Opcode::SetScalarRegister,
            &[Value::from_scalar_reg(ScalarReg::S1), Value::from_inst(y)],
        );
        merge.borrow_mut().imm_predecessors = vec![left.clone(), right.clone()];
        let read = get_s1(&merge);
        let sink = append(&merge, Opcode::BitCastF32U32, &[Value::from_inst(read)]);

        let program = program(&[left, right, merge.clone()]);
        ssa_rewrite_pass(&program);

        let phi = sink.borrow().arg(0).inst();
        assert_eq!(phi.borrow().get_opcode(), Opcode::Phi);
        assert_eq!(phi.borrow().num_args(), 2);
        assert_eq!(phi.borrow().reg_tag().ty, RegType::ScalarReg);
    }
}
