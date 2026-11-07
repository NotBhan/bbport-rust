//! FP64-to-FP32 lowering, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/lower_fp64_to_fp32.cpp`.
//!
//! Every 64-bit floating operation is rewritten to its 32-bit counterpart. Packing and
//! unpacking between a packed pair of `u32` and a double reinterpret the IEEE fields by hand;
//! the conversions between `F64` and `F32` become copies of their argument.

use crate::gpu::shader_recompiler::ir::basic_block::BlockRef;
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::microinstruction::{replace_uses_with, InstRef};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::value::{Value, F32, U32, U32U64};

const F64_TO_F32_EXP: i32 = 1023 - 127;
const F32_TO_F64_EXP: i32 = 127 - 1023;

fn wide(value: &U32) -> U32U64 {
    U32U64::new(value.value().clone())
}

fn bfe(ir: &mut IrEmitter, base: &U32, offset: u32, count: u32) -> U32 {
    let offset = ir.imm32(offset);
    let count = ir.imm32(count);
    ir.bit_field_extract(base, &offset, &count, false)
}

fn shift_left(ir: &mut IrEmitter, base: &U32, shift: u32) -> U32 {
    let shift = ir.imm32(shift);
    U32::new(ir.shift_left_logical(&wide(base), &shift).into_value())
}

fn bitwise_or(ir: &mut IrEmitter, a: &U32, b: &U32) -> U32 {
    U32::new(ir.bitwise_or(&wide(a), &wide(b)).into_value())
}

fn i_add(ir: &mut IrEmitter, a: &U32, b: &U32) -> U32 {
    U32::new(ir.i_add(&wide(a), &wide(b)).into_value())
}

fn i_equal(ir: &mut IrEmitter, a: &U32, b: &U32) -> crate::gpu::shader_recompiler::ir::value::U1 {
    ir.i_equal(&wide(a), &wide(b))
}

fn select_u32(
    ir: &mut IrEmitter,
    condition: &crate::gpu::shader_recompiler::ir::value::U1,
    a: &U32,
    b: &U32,
) -> U32 {
    U32::new(ir.select(condition, &a.value().clone(), &b.value().clone()))
}

/// `PackedF64ToF32`.
fn packed_f64_to_f32(ir: &mut IrEmitter, packed: &Value) -> F32 {
    let lo = U32::new(ir.composite_extract(packed, 0));
    let hi = U32::new(ir.composite_extract(packed, 1));
    let sign = bfe(ir, &hi, 31, 1);
    let exp = bfe(ir, &hi, 20, 11);
    let mantissa_hi = bfe(ir, &hi, 0, 20);
    let mantissa_lo = bfe(ir, &lo, 29, 3);
    let shifted_hi = shift_left(ir, &mantissa_hi, 3);
    let mantissa = bitwise_or(ir, &shifted_hi, &mantissa_lo);
    let zero = ir.imm32(0);
    let bias = ir.imm32(F64_TO_F32_EXP as u32);
    let exp_if_subnorm = {
        let is_zero = i_equal(ir, &exp, &zero);
        let biased = i_add(ir, &exp, &bias);
        select_u32(ir, &is_zero, &zero, &biased)
    };
    let all_ones_exp = ir.imm32(0xff);
    let f64_infnan = ir.imm32(0x7ff);
    let exp_if_infnan = {
        let is_infnan = i_equal(ir, &exp, &f64_infnan);
        select_u32(ir, &is_infnan, &all_ones_exp, &exp_if_subnorm)
    };
    let sign_shifted = shift_left(ir, &sign, 31);
    let exp_shifted = shift_left(ir, &exp_if_infnan, 23);
    let upper = bitwise_or(ir, &exp_shifted, &mantissa);
    let result = bitwise_or(ir, &sign_shifted, &upper);
    ir.bit_cast_f32_u32(&result)
}

/// `F32ToPackedF64`.
fn f32_to_packed_f64(ir: &mut IrEmitter, raw: &Value) -> Value {
    let value = ir.bit_cast_u32_f32(&F32::new(raw.clone()));
    let sign = bfe(ir, &value, 31, 1);
    let exp = bfe(ir, &value, 23, 8);
    let mantissa = bfe(ir, &value, 0, 23);
    let mantissa_hi = bfe(ir, &mantissa, 3, 20);
    let mantissa_lo = bfe(ir, &mantissa, 0, 3);
    let zero = ir.imm32(0);
    let bias = ir.imm32_signed(F32_TO_F64_EXP);
    let exp_if_subnorm = {
        let is_zero = i_equal(ir, &exp, &zero);
        let biased = i_add(ir, &exp, &bias);
        select_u32(ir, &is_zero, &zero, &biased)
    };
    let all_ones_exp = ir.imm32(0x7ff);
    let f32_infnan = ir.imm32(0xff);
    let exp_if_infnan = {
        let is_infnan = i_equal(ir, &exp, &f32_infnan);
        select_u32(ir, &is_infnan, &all_ones_exp, &exp_if_subnorm)
    };
    let lo = shift_left(ir, &mantissa_lo, 29);
    let sign_shifted = shift_left(ir, &sign, 31);
    let exp_shifted = shift_left(ir, &exp_if_infnan, 20);
    let upper = bitwise_or(ir, &exp_shifted, &mantissa_hi);
    let hi = bitwise_or(ir, &sign_shifted, &upper);
    ir.composite_construct(lo, hi)
}

/// `Replace`: the 32-bit counterpart of an FP64 opcode, or the opcode itself.
fn replace(opcode: Opcode) -> Opcode {
    match opcode {
        Opcode::FPAbs64 => Opcode::FPAbs32,
        Opcode::FPAdd64 => Opcode::FPAdd32,
        Opcode::FPFma64 => Opcode::FPFma32,
        Opcode::FPMax64 => Opcode::FPMax32,
        Opcode::FPMin64 => Opcode::FPMin32,
        Opcode::FPMul64 => Opcode::FPMul32,
        Opcode::FPDiv64 => Opcode::FPDiv32,
        Opcode::FPNeg64 => Opcode::FPNeg32,
        Opcode::FPRecip64 => Opcode::FPRecip32,
        Opcode::FPRecipSqrt64 => Opcode::FPRecipSqrt32,
        Opcode::FPSaturate64 => Opcode::FPSaturate32,
        Opcode::FPClamp64 => Opcode::FPClamp32,
        Opcode::FPRoundEven64 => Opcode::FPRoundEven32,
        Opcode::FPFloor64 => Opcode::FPFloor32,
        Opcode::FPCeil64 => Opcode::FPCeil32,
        Opcode::FPTrunc64 => Opcode::FPTrunc32,
        Opcode::FPFract64 => Opcode::FPFract32,
        Opcode::FPFrexpSig64 => Opcode::FPFrexpSig32,
        Opcode::FPFrexpExp64 => Opcode::FPFrexpExp32,
        Opcode::FPOrdEqual64 => Opcode::FPOrdEqual32,
        Opcode::FPUnordEqual64 => Opcode::FPUnordEqual32,
        Opcode::FPOrdNotEqual64 => Opcode::FPOrdNotEqual32,
        Opcode::FPUnordNotEqual64 => Opcode::FPUnordNotEqual32,
        Opcode::FPOrdLessThan64 => Opcode::FPOrdLessThan32,
        Opcode::FPUnordLessThan64 => Opcode::FPUnordLessThan32,
        Opcode::FPOrdGreaterThan64 => Opcode::FPOrdGreaterThan32,
        Opcode::FPUnordGreaterThan64 => Opcode::FPUnordGreaterThan32,
        Opcode::FPOrdLessThanEqual64 => Opcode::FPOrdLessThanEqual32,
        Opcode::FPUnordLessThanEqual64 => Opcode::FPUnordLessThanEqual32,
        Opcode::FPOrdGreaterThanEqual64 => Opcode::FPOrdGreaterThanEqual32,
        Opcode::FPUnordGreaterThanEqual64 => Opcode::FPUnordGreaterThanEqual32,
        Opcode::FPIsNan64 => Opcode::FPIsNan32,
        Opcode::FPIsInf64 => Opcode::FPIsInf32,
        Opcode::ConvertS32F64 => Opcode::ConvertS32F32,
        Opcode::ConvertF64S32 => Opcode::ConvertF32S32,
        Opcode::ConvertF64U32 => Opcode::ConvertF32U32,
        _ => opcode,
    }
}

/// `Lower`.
fn lower(block: &BlockRef, inst: &InstRef) {
    let opcode = inst.borrow().get_opcode();
    match opcode {
        Opcode::PackDouble2x32 => {
            let index = position(block, inst).expect("the instruction is in its block");
            let mut ir = IrEmitter::at(block, index);
            let result = packed_f64_to_f32(&mut ir, &inst.borrow().arg(0));
            replace_uses_with(inst, result.into_value());
        }
        Opcode::UnpackDouble2x32 => {
            let index = position(block, inst).expect("the instruction is in its block");
            let mut ir = IrEmitter::at(block, index);
            let result = f32_to_packed_f64(&mut ir, &inst.borrow().arg(0));
            replace_uses_with(inst, result);
        }
        Opcode::ConvertF64F32 | Opcode::ConvertF32F64 => {
            let arg = inst.borrow().arg(0);
            replace_uses_with(inst, arg);
        }
        _ => inst.borrow_mut().replace_opcode(replace(opcode)),
    }
}

fn position(block: &BlockRef, inst: &InstRef) -> Option<usize> {
    block
        .borrow()
        .instructions
        .iter()
        .position(|candidate| std::rc::Rc::ptr_eq(candidate, inst))
}

/// `LowerFp64ToFp32`.
pub fn lower_fp64_to_fp32(program: &Program) {
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
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage};
    use std::cell::RefCell;
    use std::rc::Rc;

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
    fn fp64_opcodes_map_to_fp32() {
        assert_eq!(replace(Opcode::FPAdd64), Opcode::FPAdd32);
        assert_eq!(
            replace(Opcode::FPOrdGreaterThanEqual64),
            Opcode::FPOrdGreaterThanEqual32
        );
        assert_eq!(replace(Opcode::ConvertF64U32), Opcode::ConvertF32U32);
        assert_eq!(replace(Opcode::FPMul32), Opcode::FPMul32);
    }

    #[test]
    fn lowering_replaces_the_operation_and_drops_the_conversions() {
        let block = Block::new_ref();
        let x = append(&block, Opcode::GetScc, &[]);
        let y = append(&block, Opcode::GetScc, &[]);
        let add = append(
            &block,
            Opcode::FPAdd64,
            &[Value::from_inst(x.clone()), Value::from_inst(y.clone())],
        );
        let to_f32 = append(&block, Opcode::ConvertF64F32, &[Value::from_inst(add)]);
        let sink = append(&block, Opcode::ConvertF16F32, &[Value::from_inst(to_f32)]);

        lower_fp64_to_fp32(&program(&block));

        assert_eq!(opcodes(&block)[2], Opcode::FPAdd32);
        assert_eq!(opcodes(&block)[3], Opcode::Void);
        assert!(Rc::ptr_eq(
            &sink.borrow().arg(0).inst(),
            &block.borrow().instructions[2]
        ));
    }

    #[test]
    fn packing_a_double_emits_the_ieee_field_shuffle() {
        let block = Block::new_ref();
        let lo = append(&block, Opcode::GetScc, &[]);
        let hi = append(&block, Opcode::GetScc, &[]);
        let packed = append(
            &block,
            Opcode::CompositeConstructU32x2,
            &[Value::from_inst(lo), Value::from_inst(hi)],
        );
        let unpack = append(&block, Opcode::PackDouble2x32, &[Value::from_inst(packed)]);
        let sink = append(&block, Opcode::ConvertF16F32, &[Value::from_inst(unpack)]);

        lower_fp64_to_fp32(&program(&block));

        let result = sink.borrow().arg(0).inst();
        assert_eq!(result.borrow().get_opcode(), Opcode::BitCastF32U32);
        let ops = opcodes(&block);
        assert!(ops.contains(&Opcode::BitFieldUExtract));
        assert!(ops.contains(&Opcode::ShiftLeftLogical32));
        assert!(ops.contains(&Opcode::BitwiseOr32));
        assert!(ops.contains(&Opcode::SelectU32));
    }

    #[test]
    fn unpacking_a_float_emits_a_packed_pair() {
        let block = Block::new_ref();
        let integer = append(&block, Opcode::GetScc, &[]);
        let value = append(&block, Opcode::BitCastF32U32, &[Value::from_inst(integer)]);
        let unpack = append(&block, Opcode::UnpackDouble2x32, &[Value::from_inst(value)]);
        let sink = append(&block, Opcode::ConvertF16F32, &[Value::from_inst(unpack)]);

        lower_fp64_to_fp32(&program(&block));

        let result = sink.borrow().arg(0).inst();
        assert_eq!(
            result.borrow().get_opcode(),
            Opcode::CompositeConstructU32x2
        );
        let ops = opcodes(&block);
        assert!(ops.contains(&Opcode::BitCastU32F32));
        assert!(ops.contains(&Opcode::ShiftLeftLogical32));
        assert!(ops.contains(&Opcode::BitwiseOr32));
    }
}
