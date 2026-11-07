//! Hardware intrinsic lowering, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/lower_hardware_intrinsics.cpp`.
//!
//! `FPCmpClass32` becomes one of the float-class tests, packed ancillary reads become sample or
//! render-target index reads with the matching bitfield extract, and the masked bit count becomes
//! a range-mask read and a popcount.

use crate::gpu::shader_recompiler::ir::attribute::Attribute;
use crate::gpu::shader_recompiler::ir::basic_block::BlockRef;
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::microinstruction::{replace_uses_with, InstRef};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::reg::FloatClassFunc;
use crate::gpu::shader_recompiler::ir::value::{F32F64, U32U64};
use std::rc::Rc;

fn position(block: &BlockRef, inst: &InstRef) -> Option<usize> {
    block
        .borrow()
        .instructions
        .iter()
        .position(|candidate| Rc::ptr_eq(candidate, inst))
}

/// `LowerCmpClass`.
fn lower_cmp_class(block: &BlockRef, inst: &InstRef) {
    assert!(
        inst.borrow().arg(1).is_immediate(),
        "Unable to resolve compare operation"
    );
    let class_mask = FloatClassFunc::from_bits(inst.borrow().arg(1).u32());
    if class_mask.contains(FloatClassFunc::NaN) {
        inst.borrow_mut().replace_opcode(Opcode::FPIsNan32);
    } else if class_mask.contains(FloatClassFunc::Infinity) {
        inst.borrow_mut().replace_opcode(Opcode::FPIsInf32);
    } else if class_mask.contains(FloatClassFunc::Finite) {
        let index = position(block, inst).expect("the instruction is in its block");
        let mut ir = IrEmitter::at(block, index);
        let value = F32F64::new(inst.borrow().arg(0));
        let nan = ir.fp_is_nan(&value);
        let inf = ir.fp_is_inf(&value);
        let either = ir.logical_or(&nan, &inf);
        let not = ir.logical_not(&either);
        replace_uses_with(inst, not.into_value());
    } else {
        panic!("Unreachable");
    }
}

/// `LowerPackedAncillary`.
fn lower_packed_ancillary(block: &BlockRef, inst: &InstRef) {
    let arg_0 = inst.borrow().arg(0);
    let arg_1 = inst.borrow().arg(1);
    let arg_2 = inst.borrow().arg(2);
    if arg_0.is_immediate() || !arg_1.is_immediate() || !arg_2.is_immediate() {
        return;
    }
    let value = arg_0.inst();
    if value.borrow().get_opcode() != Opcode::GetAttributeU32
        || value.borrow().arg(0).attribute() != Attribute::PackedAncillary
    {
        return;
    }
    let offset = arg_1.u32();
    let bits = arg_2.u32();

    let index = position(block, inst).expect("the instruction is in its block");
    let mut ir = IrEmitter::at(block, index);
    if offset >= 8 && offset + bits <= 12 {
        let sample_index = ir.get_attribute_u32(Attribute::SampleIndex, 0);
        if offset == 8 && bits == 4 {
            replace_uses_with(inst, sample_index.into_value());
        } else {
            let shift = ir.imm32(offset - 8);
            let count = ir.imm32(bits);
            let extracted = ir.bit_field_extract(&sample_index, &shift, &count, false);
            replace_uses_with(inst, extracted.into_value());
        }
    } else if offset >= 16 && offset + bits <= 27 {
        let mrt_index = ir.get_attribute_u32(Attribute::RenderTargetIndex, 0);
        if offset == 16 && bits == 11 {
            replace_uses_with(inst, mrt_index.into_value());
        } else {
            let shift = ir.imm32(offset - 16);
            let count = ir.imm32(bits);
            let extracted = ir.bit_field_extract(&mrt_index, &shift, &count, false);
            replace_uses_with(inst, extracted.into_value());
        }
    } else {
        panic!("Unhandled bitfield extract from ancillary VGPR offset={offset}, bits={bits}");
    }
    let zero = ir.imm32(0u32);
    replace_uses_with(&value, zero.into_value());
}

/// `LowerMaskedBitCount`.
fn lower_masked_bit_count(block: &BlockRef, inst: &InstRef) {
    let index = position(block, inst).expect("the instruction is in its block");
    let mut ir = IrEmitter::at(block, index);
    let comp = u32::from(inst.borrow().arg(2).u1());
    let thread_mask = ir.get_attribute_u32(Attribute::SubgroupLtMask, comp);
    let value = U32U64::new(inst.borrow().arg(0));
    let mask = U32U64::new(thread_mask.into_value());
    let masked = ir.bitwise_and(&value, &mask);
    let count = ir.bit_count(&masked);
    let addend = U32U64::new(inst.borrow().arg(1));
    let sum = ir.i_add(&U32U64::new(count.into_value()), &addend);
    replace_uses_with(inst, sum.into_value());
}

fn lower(block: &BlockRef, inst: &InstRef) {
    let opcode = inst.borrow().get_opcode();
    match opcode {
        Opcode::FPCmpClass32 => lower_cmp_class(block, inst),
        Opcode::BitFieldUExtract => lower_packed_ancillary(block, inst),
        Opcode::MaskedBitCount32 => lower_masked_bit_count(block, inst),
        _ => {}
    }
}

/// `LowerHardwareIntrinsics`.
pub fn lower_hardware_intrinsics(program: &Program) {
    for block in &program.blocks {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            lower(block, inst);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block};
    use crate::gpu::shader_recompiler::ir::value::Value;
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage};
    use std::cell::RefCell;
    use std::panic::AssertUnwindSafe;

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

    fn sink(block: &BlockRef, value: &InstRef) -> InstRef {
        append(
            block,
            Opcode::ConvertF16F32,
            &[Value::from_inst(value.clone())],
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

    fn cmp_class(block: &BlockRef, mask: FloatClassFunc) -> InstRef {
        append(
            block,
            Opcode::FPCmpClass32,
            &[Value::from_f32(1.0), Value::from_u32(mask.bits())],
        )
    }

    fn packed_ancillary(block: &BlockRef, offset: u32, bits: u32) -> (InstRef, InstRef) {
        let read = append(
            block,
            Opcode::GetAttributeU32,
            &[
                Value::from_attribute(Attribute::PackedAncillary),
                Value::from_u32(0),
            ],
        );
        let extract = append(
            block,
            Opcode::BitFieldUExtract,
            &[
                Value::from_inst(read.clone()),
                Value::from_u32(offset),
                Value::from_u32(bits),
            ],
        );
        (read, extract)
    }

    fn run(block: &BlockRef) {
        lower_hardware_intrinsics(&program(block));
    }

    #[test]
    fn the_class_masks_become_the_single_tests() {
        let block = Block::new_ref();
        let nan = cmp_class(&block, FloatClassFunc::NaN);
        let infinity = cmp_class(&block, FloatClassFunc::Infinity);
        run(&block);
        assert_eq!(nan.borrow().get_opcode(), Opcode::FPIsNan32);
        assert_eq!(infinity.borrow().get_opcode(), Opcode::FPIsInf32);
    }

    #[test]
    fn the_finite_class_becomes_nan_or_infinity_negated() {
        let block = Block::new_ref();
        let finite = cmp_class(&block, FloatClassFunc::Finite);
        let finite_sink = sink(&block, &finite);
        run(&block);

        assert_eq!(
            opcodes(&block),
            vec![
                Opcode::FPIsNan32,
                Opcode::FPIsInf32,
                Opcode::LogicalOr,
                Opcode::LogicalNot,
                Opcode::Void,
                Opcode::ConvertF16F32,
            ]
        );
        let not = finite_sink.borrow().arg(0).inst();
        assert_eq!(not.borrow().get_opcode(), Opcode::LogicalNot);
        let or = not.borrow().arg(0).inst();
        assert_eq!(or.borrow().get_opcode(), Opcode::LogicalOr);
    }

    #[test]
    fn an_unsupported_class_mask_is_refused() {
        let block = Block::new_ref();
        cmp_class(&block, FloatClassFunc::Negative);
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| run(&block)));
        assert!(result.is_err());
    }

    #[test]
    fn the_sample_index_loads_are_extracted_from_the_ancillary_word() {
        let block = Block::new_ref();
        let (read, extract) = packed_ancillary(&block, 8, 4);
        let extract_sink = sink(&block, &extract);
        run(&block);

        assert_eq!(
            opcodes(&block),
            vec![
                Opcode::Void,
                Opcode::GetAttributeU32,
                Opcode::Void,
                Opcode::ConvertF16F32,
            ]
        );
        let sample = extract_sink.borrow().arg(0).inst();
        assert_eq!(sample.borrow().arg(0).attribute(), Attribute::SampleIndex);
        assert_eq!(sample.borrow().arg(1).u32(), 0);
        assert_eq!(read.borrow().get_opcode(), Opcode::Void);

        let block = Block::new_ref();
        let (_, extract) = packed_ancillary(&block, 9, 2);
        let extract_sink = sink(&block, &extract);
        run(&block);

        let ops = opcodes(&block);
        assert_eq!(
            &ops[..5],
            &[
                Opcode::Void,
                Opcode::GetAttributeU32,
                Opcode::BitFieldUExtract,
                Opcode::Void,
                Opcode::ConvertF16F32,
            ]
        );
        let extracted = extract_sink.borrow().arg(0).inst();
        assert_eq!(extracted.borrow().get_opcode(), Opcode::BitFieldUExtract);
        assert_eq!(extracted.borrow().arg(1).u32(), 1);
        assert_eq!(extracted.borrow().arg(2).u32(), 2);
    }

    #[test]
    fn the_render_target_index_is_read_from_the_ancillary_word() {
        let block = Block::new_ref();
        let (read, extract) = packed_ancillary(&block, 16, 11);
        let extract_sink = sink(&block, &extract);
        run(&block);

        let mrt = extract_sink.borrow().arg(0).inst();
        assert_eq!(
            mrt.borrow().arg(0).attribute(),
            Attribute::RenderTargetIndex
        );
        assert_eq!(read.borrow().get_opcode(), Opcode::Void);
    }

    #[test]
    fn an_unhandled_ancillary_range_is_refused() {
        let block = Block::new_ref();
        packed_ancillary(&block, 4, 2);
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| run(&block)));
        let payload = result.expect_err("the pass panics");
        let message = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| {
                payload
                    .downcast_ref::<&str>()
                    .map(|text| (*text).to_string())
            })
            .expect("a panic message");
        assert_eq!(
            message,
            "Unhandled bitfield extract from ancillary VGPR offset=4, bits=2"
        );
    }

    #[test]
    fn a_masked_bit_count_reads_the_subgroup_mask() {
        let block = Block::new_ref();
        let masked = append(
            &block,
            Opcode::MaskedBitCount32,
            &[
                Value::from_u32(0xF0),
                Value::from_u32(5),
                Value::from_u1(true),
            ],
        );
        let masked_sink = sink(&block, &masked);
        run(&block);

        assert_eq!(
            opcodes(&block),
            vec![
                Opcode::GetAttributeU32,
                Opcode::BitwiseAnd32,
                Opcode::BitCount32,
                Opcode::IAdd32,
                Opcode::Void,
                Opcode::ConvertF16F32,
            ]
        );
        let mask_read = block.borrow().instructions[0].clone();
        assert_eq!(
            mask_read.borrow().arg(0).attribute(),
            Attribute::SubgroupLtMask
        );
        assert_eq!(mask_read.borrow().arg(1).u32(), 1);
        let sum = masked_sink.borrow().arg(0).inst();
        assert_eq!(sum.borrow().get_opcode(), Opcode::IAdd32);
    }

    #[test]
    fn unrelated_instructions_are_left_alone() {
        let block = Block::new_ref();
        append(&block, Opcode::Barrier, &[]);
        run(&block);
        assert_eq!(opcodes(&block), vec![Opcode::Barrier]);
    }
}
