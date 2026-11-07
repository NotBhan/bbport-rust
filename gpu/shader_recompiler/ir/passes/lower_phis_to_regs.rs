//! Lower phis to registers, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/lower_phis_to_regs_pass.cpp`.
//!
//! A phi that already carries a register tag is replaced by a load from that register. Otherwise
//! the pass allocates a virtual register of the phi's type, stores each phi operand at the end of
//! its predecessor, and replaces the phi's uses with a load of that register.

use crate::gpu::shader_recompiler::ir::basic_block::BlockRef;
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::microinstruction::{replace_uses_with, InstRef};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::reg::{
    RegType as RegisterTrait, ScalarReg, VectorReg, VirtualReg,
};
use crate::gpu::shader_recompiler::ir::ssa::{RegTag, RegTagRegister, RegType};
use crate::gpu::shader_recompiler::ir::value::Value;
use std::rc::Rc;

fn position(block: &BlockRef, inst: &InstRef) -> Option<usize> {
    block
        .borrow()
        .instructions
        .iter()
        .position(|candidate| Rc::ptr_eq(candidate, inst))
}

/// `LoadForRegType`.
fn load_for_reg_type(ir: &mut IrEmitter, tag: RegTag) -> Value {
    match tag.ty {
        RegType::ScalarReg => ir
            .get_scalar_reg(ScalarReg::from_raw(tag.raw_index()))
            .into(),
        RegType::VectorReg => ir
            .get_vector_reg(VectorReg::from_raw(tag.raw_index()))
            .into(),
        RegType::Scc => ir.get_scc().into(),
        RegType::Exec => ir.get_exec().into(),
        RegType::VccLo => ir.get_vcc_lo().into(),
        RegType::VccHi => ir.get_vcc_hi().into(),
        RegType::M0 => ir.get_m0().into(),
        RegType::VirtualReg => match tag.register() {
            RegTagRegister::Virtual(reg) => ir.get_virtual_reg(reg),
            _ => panic!("Unknown reg type {}", tag.ty as u32),
        },
        _ => panic!("Unknown reg type {}", tag.ty as u32),
    }
}

/// `LowerPhisToRegsPass`.
pub fn lower_phis_to_regs_pass(program: &mut Program) {
    let blocks = program.post_order_blocks.clone();
    for block in blocks.iter().rev() {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            let opcode = inst.borrow().get_opcode();
            if opcode == Opcode::Void {
                continue;
            }
            if opcode != Opcode::Phi {
                break;
            }

            let index = position(block, inst).expect("the phi is in its block");
            let tag = inst.borrow().reg_tag();
            if tag.is_set() {
                let mut ir = IrEmitter::at(block, index);
                let load = load_for_reg_type(&mut ir, tag);
                replace_uses_with(inst, load);
                continue;
            }

            let ty = inst.borrow().type_flags();
            let reg = VirtualReg::new(program.next_reg_index, ty);
            program.next_reg_index += 1;
            let arg_count = inst.borrow().num_args();
            for arg_index in 0..arg_count {
                let predecessor = inst.borrow().phi_block(arg_index);
                let value = inst.borrow().arg(arg_index);
                let mut ir = IrEmitter::new(&predecessor);
                ir.set_virtual_reg(reg, &value);
            }

            let mut ir = IrEmitter::at(block, index);
            let load = ir.get_virtual_reg(reg);
            replace_uses_with(inst, load);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block};
    use crate::gpu::shader_recompiler::ir::microinstruction::{add_phi_operand, invalidate};
    use crate::gpu::shader_recompiler::ir::r#type::Type;
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

    fn phi(block: &BlockRef, predecessors: &[&BlockRef], values: &[Value], ty: Type) -> InstRef {
        let phi = append(block, Opcode::Phi, &[]);
        phi.borrow_mut().set_type_flags(ty);
        for (predecessor, value) in predecessors.iter().zip(values) {
            add_phi_operand(&phi, predecessor, value.clone());
        }
        phi
    }

    #[test]
    fn a_phi_becomes_stores_and_a_load() {
        let header = Block::new_ref();
        let pred0 = Block::new_ref();
        let pred1 = Block::new_ref();
        let phi = phi(
            &header,
            &[&pred0, &pred1],
            &[Value::from_u32(1), Value::from_u32(2)],
            Type::U32,
        );
        let user = append(
            &header,
            Opcode::ConvertF16F32,
            &[Value::from_inst(phi.clone())],
        );

        let mut program = program();
        program.post_order_blocks = vec![header.clone()];
        lower_phis_to_regs_pass(&mut program);

        assert_eq!(program.next_reg_index, 1);
        let reg = VirtualReg::new(0, Type::U32);

        assert_eq!(pred0.borrow().instructions.len(), 1);
        let store0 = pred0.borrow().instructions[0].clone();
        assert_eq!(store0.borrow().get_opcode(), Opcode::SetVirtualRegister);
        assert_eq!(store0.borrow().arg(0).virtual_reg(), reg);
        assert_eq!(store0.borrow().arg(1).u32(), 1);
        assert_eq!(pred1.borrow().instructions[0].borrow().arg(1).u32(), 2);

        let instructions = header.borrow().instructions.clone();
        assert_eq!(instructions.len(), 3);
        let load = &instructions[0];
        assert_eq!(load.borrow().get_opcode(), Opcode::GetVirtualRegister);
        assert_eq!(load.borrow().arg(0).virtual_reg(), reg);
        assert_eq!(instructions[1].borrow().get_opcode(), Opcode::Void);
        assert!(Rc::ptr_eq(&user.borrow().arg(0).inst(), load));
    }

    #[test]
    fn a_tagged_phi_loads_its_register() {
        let header = Block::new_ref();
        let pred0 = Block::new_ref();
        let phi = phi(&header, &[&pred0], &[Value::from_u32(1)], Type::U1);
        phi.borrow_mut().set_reg_tag(RegTag::new(RegType::Scc, 0));
        let user = append(
            &header,
            Opcode::ConvertF16F32,
            &[Value::from_inst(phi.clone())],
        );

        let mut program = program();
        program.post_order_blocks = vec![header.clone()];
        lower_phis_to_regs_pass(&mut program);

        assert_eq!(program.next_reg_index, 0);
        assert!(pred0.borrow().instructions.is_empty());
        let instructions = header.borrow().instructions.clone();
        assert_eq!(instructions.len(), 3);
        assert_eq!(instructions[0].borrow().get_opcode(), Opcode::GetScc);
        assert_eq!(instructions[1].borrow().get_opcode(), Opcode::Void);
        assert!(Rc::ptr_eq(&user.borrow().arg(0).inst(), &instructions[0]));
    }

    #[test]
    fn void_instructions_before_the_phis_do_not_stop_the_scan() {
        let header = Block::new_ref();
        let pred0 = Block::new_ref();
        let skipped = append(&header, Opcode::Barrier, &[]);
        invalidate(&skipped);
        let phi = phi(&header, &[&pred0], &[Value::from_u32(7)], Type::U32);

        let mut program = program();
        program.post_order_blocks = vec![header.clone()];
        lower_phis_to_regs_pass(&mut program);

        assert_eq!(program.next_reg_index, 1);
        assert_eq!(phi.borrow().get_opcode(), Opcode::Void);
        let instructions = header.borrow().instructions.clone();
        assert_eq!(instructions.len(), 3);
        assert_eq!(instructions[0].borrow().get_opcode(), Opcode::Void);
        assert_eq!(
            instructions[1].borrow().get_opcode(),
            Opcode::GetVirtualRegister
        );
    }

    #[test]
    fn a_non_phi_stops_the_scan() {
        let header = Block::new_ref();
        let pred0 = Block::new_ref();
        append(&header, Opcode::Barrier, &[]);
        let phi = phi(&header, &[&pred0], &[Value::from_u32(1)], Type::U32);

        let mut program = program();
        program.post_order_blocks = vec![header.clone()];
        lower_phis_to_regs_pass(&mut program);

        assert_eq!(program.next_reg_index, 0);
        assert_eq!(phi.borrow().get_opcode(), Opcode::Phi);
        assert_eq!(header.borrow().instructions.len(), 2);
    }
}
