//! The interpolated-integer fix, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/interpolated_integer_pass.cpp`.
//!
//! An integer attribute carried through a smooth float varying can truncate to the next lower
//! integer when the hardware interpolates it. Scaling an interpolated value by `1 + 2^-16` before
//! the truncation keeps the exact integer. The C++ prints a diagnostic; the port drops it.

use crate::gpu::shader_recompiler::ir::attribute::is_param;
use crate::gpu::shader_recompiler::ir::basic_block::BlockRef;
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::microinstruction::{set_arg, InstRef};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::value::{Value, F32, F32F64};
use std::rc::Rc;

fn position(block: &BlockRef, inst: &InstRef) -> Option<usize> {
    block
        .borrow()
        .instructions
        .iter()
        .position(|candidate| Rc::ptr_eq(candidate, inst))
}

/// `IsTruncation`.
fn is_truncation(op: Opcode) -> bool {
    matches!(
        op,
        Opcode::ConvertU32F32 | Opcode::ConvertS32F32 | Opcode::FPTrunc32 | Opcode::FPFloor32
    )
}

/// `IsInterpolatedParam`: a parameter attribute read, which the interpolator produced.
fn is_interpolated_param(value: &Value) -> bool {
    if value.is_immediate() {
        return false;
    }
    let Some(inst) = value.try_inst() else {
        return false;
    };
    inst.borrow().get_opcode() == Opcode::GetAttribute && is_param(inst.borrow().arg(0).attribute())
}

/// `InterpolatedIntegerPass`.
pub fn interpolated_integer_pass(program: &Program) {
    for block in &program.blocks {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            if !is_truncation(inst.borrow().get_opcode()) {
                continue;
            }
            let value = inst.borrow().arg(0);
            if !is_interpolated_param(&value) {
                continue;
            }

            let index = position(block, inst).expect("the instruction is in its block");
            let mut ir = IrEmitter::at(block, index);
            let factor = ir.imm32_float(1.0 + 1.0 / 65536.0);
            let input = F32::new(value);
            let product = ir.fp_mul(
                &F32F64::new(input.into_value()),
                &F32F64::new(factor.into_value()),
            );
            let nudged = F32::new(product.into_value());
            set_arg(inst, 0, nudged.into_value());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::attribute::Attribute;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block, BlockRef};
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage};
    use std::cell::RefCell;

    fn program(block: &BlockRef) -> Program {
        let info = Rc::new(RefCell::new(Info::new(
            HwStage::Fragment,
            SwStage::Fragment,
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

    fn attribute_read(block: &BlockRef, attribute: Attribute) -> InstRef {
        append(
            block,
            Opcode::GetAttribute,
            &[
                Value::from_attribute(attribute),
                Value::from_u32(1),
                Value::from_u32(0),
            ],
        )
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
    fn each_truncation_of_a_parameter_is_nudged() {
        let block = Block::new_ref();
        let mut truncations = Vec::new();
        for op in [
            Opcode::ConvertU32F32,
            Opcode::ConvertS32F32,
            Opcode::FPTrunc32,
            Opcode::FPFloor32,
        ] {
            let read = attribute_read(&block, Attribute::Param3);
            truncations.push(append(&block, op, &[Value::from_inst(read)]));
        }

        interpolated_integer_pass(&program(&block));

        let ops = opcodes(&block);
        assert_eq!(
            ops.len(),
            12,
            "four inserts on top of four reads and conversions"
        );
        assert_eq!(ops.iter().filter(|op| **op == Opcode::FPMul32).count(), 4);
        for truncation in &truncations {
            let nudged = truncation.borrow().arg(0);
            assert!(!nudged.is_immediate());
            let mul = nudged.inst();
            assert_eq!(mul.borrow().get_opcode(), Opcode::FPMul32);
            assert_eq!(
                mul.borrow().arg(1).f32(),
                1.0 + 1.0 / 65536.0,
                "the scale is 1 + 2^-16"
            );
            assert!(matches!(
                mul.borrow()
                    .arg(0)
                    .try_inst()
                    .map(|read| read.borrow().get_opcode()),
                Some(Opcode::GetAttribute)
            ));
        }
    }

    #[test]
    fn a_non_parameter_attribute_is_left_alone() {
        let block = Block::new_ref();
        let read = attribute_read(&block, Attribute::Position0);
        let truncation = append(
            &block,
            Opcode::ConvertU32F32,
            &[Value::from_inst(read.clone())],
        );

        interpolated_integer_pass(&program(&block));

        assert_eq!(
            opcodes(&block),
            vec![Opcode::GetAttribute, Opcode::ConvertU32F32]
        );
        assert!(Rc::ptr_eq(&truncation.borrow().arg(0).inst(), &read));
    }

    #[test]
    fn a_non_truncation_is_left_alone() {
        let block = Block::new_ref();
        let read = attribute_read(&block, Attribute::Param3);
        let negate = append(&block, Opcode::FPNeg32, &[Value::from_inst(read.clone())]);

        interpolated_integer_pass(&program(&block));

        assert_eq!(opcodes(&block), vec![Opcode::GetAttribute, Opcode::FPNeg32]);
        assert!(Rc::ptr_eq(&negate.borrow().arg(0).inst(), &read));
    }
}
