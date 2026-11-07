//! Integer emitters, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv_integer.cpp`.

use super::builder::Id;
use super::emit_context::EmitContext;
use crate::gpu::shader_recompiler::ir::microinstruction::InstRef;

fn set_zero_flag(_ctx: &mut EmitContext, _inst: &InstRef, _result: Id) {}
fn set_sign_flag(_ctx: &mut EmitContext, _inst: &InstRef, _result: Id) {}

pub fn emit_i_add32(ctx: &mut EmitContext, _inst: &InstRef, a: Id, b: Id) -> Id {
    let ty = ctx.type_ids().U32[1];
    ctx.OpIAdd(ty, a, b)
}

pub fn emit_i_add64(ctx: &mut EmitContext, a: Id, b: Id) -> Id {
    let ty = ctx.type_ids().U64;
    ctx.OpIAdd(ty, a, b)
}

pub fn emit_i_add_carry32(ctx: &mut EmitContext, a: Id, b: Id) -> Id {
    let ty = ctx.type_ids().full_result_u32x2;
    ctx.OpIAddCarry(ty, a, b)
}

pub fn emit_i_sub32(ctx: &mut EmitContext, a: Id, b: Id) -> Id {
    let ty = ctx.type_ids().U32[1];
    ctx.OpISub(ty, a, b)
}

pub fn emit_i_sub64(ctx: &mut EmitContext, a: Id, b: Id) -> Id {
    let ty = ctx.type_ids().U64;
    ctx.OpISub(ty, a, b)
}

pub fn emit_s_mul_hi(ctx: &mut EmitContext, a: Id, b: Id) -> Id {
    let types = ctx.type_ids();
    let signed_a = ctx.OpBitcast(types.S32[1], a);
    let signed_b = ctx.OpBitcast(types.S32[1], b);
    let mul_ext = ctx.OpSMulExtended(types.full_result_i32x2, signed_a, signed_b);
    let signed_hi = ctx.OpCompositeExtract(types.S32[1], mul_ext, &[1]);
    ctx.OpBitcast(types.U32[1], signed_hi)
}

pub fn emit_u_mul_hi(ctx: &mut EmitContext, a: Id, b: Id) -> Id {
    let types = ctx.type_ids();
    let mul_ext = ctx.OpUMulExtended(types.full_result_u32x2, a, b);
    ctx.OpCompositeExtract(types.U32[1], mul_ext, &[1])
}

pub fn emit_i_mul32(ctx: &mut EmitContext, a: Id, b: Id) -> Id {
    let ty = ctx.type_ids().U32[1];
    ctx.OpIMul(ty, a, b)
}

pub fn emit_i_mul64(ctx: &mut EmitContext, a: Id, b: Id) -> Id {
    let ty = ctx.type_ids().U64;
    ctx.OpIMul(ty, a, b)
}

pub fn emit_s_div32(ctx: &mut EmitContext, a: Id, b: Id) -> Id {
    let ty = ctx.type_ids().U32[1];
    ctx.OpSDiv(ty, a, b)
}

pub fn emit_u_div32(ctx: &mut EmitContext, a: Id, b: Id) -> Id {
    let ty = ctx.type_ids().U32[1];
    ctx.OpUDiv(ty, a, b)
}

pub fn emit_s_mod32(ctx: &mut EmitContext, a: Id, b: Id) -> Id {
    let ty = ctx.type_ids().U32[1];
    ctx.OpSMod(ty, a, b)
}

pub fn emit_u_mod32(ctx: &mut EmitContext, a: Id, b: Id) -> Id {
    let ty = ctx.type_ids().U32[1];
    ctx.OpUMod(ty, a, b)
}

macro_rules! unary_ops {
    ($(($name:ident, $field:ident, $op:ident)),* $(,)?) => {
        $(
            pub fn $name(ctx: &mut EmitContext, value: Id) -> Id {
                let ty = ctx.type_ids().$field[1];
                ctx.$op(ty, value)
            }
        )*
    };
}

unary_ops! {
    (emit_i_neg32, U32, OpSNegate),
    (emit_i_abs32, U32, OpSAbs),
    (emit_bit_reverse32, U32, OpBitReverse),
    (emit_bit_count32, U32, OpBitCount),
    (emit_bitwise_not32, U32, OpNot),
    (emit_find_s_msb32, U32, OpFindSMsb),
    (emit_find_u_msb32, U32, OpFindUMsb),
    (emit_find_i_lsb32, U32, OpFindILsb),
}

pub fn emit_i_neg64(ctx: &mut EmitContext, value: Id) -> Id {
    let ty = ctx.type_ids().U64;
    ctx.OpSNegate(ty, value)
}

pub fn emit_bitwise_not64(ctx: &mut EmitContext, value: Id) -> Id {
    let ty = ctx.type_ids().U64;
    ctx.OpNot(ty, value)
}

macro_rules! shift_ops {
    ($(($name:ident, $field:ident, $op:ident)),* $(,)?) => {
        $(
            pub fn $name(ctx: &mut EmitContext, base: Id, shift: Id) -> Id {
                let ty = ctx.type_ids().$field[1];
                ctx.$op(ty, base, shift)
            }
        )*
    };
}

shift_ops! {
    (emit_shift_left_logical32, U32, OpShiftLeftLogical),
    (emit_shift_right_logical32, U32, OpShiftRightLogical),
    (emit_shift_right_arithmetic32, U32, OpShiftRightArithmetic),
}

pub fn emit_shift_left_logical64(ctx: &mut EmitContext, base: Id, shift: Id) -> Id {
    let ty = ctx.type_ids().U64;
    ctx.OpShiftLeftLogical(ty, base, shift)
}

pub fn emit_shift_right_logical64(ctx: &mut EmitContext, base: Id, shift: Id) -> Id {
    let ty = ctx.type_ids().U64;
    ctx.OpShiftRightLogical(ty, base, shift)
}

pub fn emit_shift_right_arithmetic64(ctx: &mut EmitContext, base: Id, shift: Id) -> Id {
    let ty = ctx.type_ids().U64;
    ctx.OpShiftRightArithmetic(ty, base, shift)
}

macro_rules! bitwise_ops {
    ($(($name:ident, $field:ident, $op:ident)),* $(,)?) => {
        $(
            pub fn $name(
                ctx: &mut EmitContext,
                inst: &InstRef,
                a: Id,
                b: Id,
            ) -> Id {
                let ty = ctx.type_ids().$field[1];
                let result = ctx.$op(ty, a, b);
                set_zero_flag(ctx, inst, result);
                set_sign_flag(ctx, inst, result);
                result
            }
        )*
    };
}

bitwise_ops! {
    (emit_bitwise_and32, U32, OpBitwiseAnd),
    (emit_bitwise_or32, U32, OpBitwiseOr),
    (emit_bitwise_xor32, U32, OpBitwiseXor),
}

macro_rules! bitwise64_ops {
    ($(($name:ident, $op:ident)),* $(,)?) => {
        $(
            pub fn $name(
                ctx: &mut EmitContext,
                inst: &InstRef,
                a: Id,
                b: Id,
            ) -> Id {
                let ty = ctx.type_ids().U64;
                let result = ctx.$op(ty, a, b);
                set_zero_flag(ctx, inst, result);
                set_sign_flag(ctx, inst, result);
                result
            }
        )*
    };
}

bitwise64_ops! {
    (emit_bitwise_and64, OpBitwiseAnd),
    (emit_bitwise_or64, OpBitwiseOr),
    (emit_bitwise_xor64, OpBitwiseXor),
}

pub fn emit_bit_field_insert32(
    ctx: &mut EmitContext,
    base: Id,
    insert: Id,
    offset: Id,
    count: Id,
) -> Id {
    let ty = ctx.type_ids().U32[1];
    ctx.OpBitFieldInsert(ty, base, insert, offset, count)
}

pub fn emit_bit_field_insert64(
    ctx: &mut EmitContext,
    base: Id,
    insert: Id,
    offset: Id,
    count: Id,
) -> Id {
    let ty = ctx.type_ids().U64;
    ctx.OpBitFieldInsert(ty, base, insert, offset, count)
}

pub fn emit_bit_field_s_extract(
    ctx: &mut EmitContext,
    inst: &InstRef,
    base: Id,
    offset: Id,
    count: Id,
) -> Id {
    let ty = ctx.type_ids().U32[1];
    let result = ctx.OpBitFieldSExtract(ty, base, offset, count);
    set_zero_flag(ctx, inst, result);
    set_sign_flag(ctx, inst, result);
    result
}

pub fn emit_bit_field_u_extract(
    ctx: &mut EmitContext,
    inst: &InstRef,
    base: Id,
    offset: Id,
    count: Id,
) -> Id {
    let ty = ctx.type_ids().U32[1];
    let result = ctx.OpBitFieldUExtract(ty, base, offset, count);
    set_zero_flag(ctx, inst, result);
    set_sign_flag(ctx, inst, result);
    result
}

pub fn emit_bit_count64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let unpacked = ctx.OpBitcast(types.U32[2], value);
    let lo = ctx.OpCompositeExtract(types.U32[1], unpacked, &[0]);
    let hi = ctx.OpCompositeExtract(types.U32[1], unpacked, &[1]);
    let lo_count = ctx.OpBitCount(types.U32[1], lo);
    let hi_count = ctx.OpBitCount(types.U32[1], hi);
    ctx.OpIAdd(types.U32[1], lo_count, hi_count)
}

pub fn emit_find_u_msb64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let unpacked = ctx.OpBitcast(types.U32[2], value);
    let hi = ctx.OpCompositeExtract(types.U32[1], unpacked, &[1]);
    let lo = ctx.OpCompositeExtract(types.U32[1], unpacked, &[0]);
    let hi_msb = ctx.OpFindUMsb(types.U32[1], hi);
    let lo_msb = ctx.OpFindUMsb(types.U32[1], lo);
    let minus_one = ctx.ConstU32(u32::MAX);
    let found_hi = ctx.OpINotEqual(types.U1[1], hi_msb, minus_one);
    let thirty_two = ctx.ConstU32(32);
    let shifted_hi = ctx.OpIAdd(types.U32[1], hi_msb, thirty_two);
    ctx.OpSelect(types.U32[1], found_hi, shifted_hi, lo_msb)
}

pub fn emit_find_i_lsb64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let unpacked = ctx.OpBitcast(types.U32[2], value);
    let lo = ctx.OpCompositeExtract(types.U32[1], unpacked, &[0]);
    let hi = ctx.OpCompositeExtract(types.U32[1], unpacked, &[1]);
    let lo_lsb = ctx.OpFindILsb(types.U32[1], lo);
    let hi_lsb = ctx.OpFindILsb(types.U32[1], hi);
    let minus_one = ctx.ConstU32(u32::MAX);
    let found_lo = ctx.OpINotEqual(types.U1[1], lo_lsb, minus_one);
    ctx.OpSelect(types.U32[1], found_lo, lo_lsb, hi_lsb)
}

macro_rules! minmax_ops {
    ($(($name:ident, $op:ident)),* $(,)?) => {
        $(
            pub fn $name(ctx: &mut EmitContext, a: Id, b: Id) -> Id {
                let ty = ctx.type_ids().U32[1];
                ctx.$op(ty, a, b)
            }
        )*
    };
}

minmax_ops! {
    (emit_s_min32, OpSMin),
    (emit_u_min32, OpUMin),
    (emit_s_max32, OpSMax),
    (emit_u_max32, OpUMax),
}

macro_rules! tri_ops {
    ($(($name:ident, $amd:ident, $op:ident)),* $(,)?) => {
        $(
            pub fn $name(ctx: &mut EmitContext, a: Id, b: Id, c: Id) -> Id {
                let ty = ctx.type_ids().U32[1];
                if ctx.profile.supports_trinary_minmax {
                    return ctx.$amd(ty, a, b, c);
                }
                let inner = ctx.$op(ty, b, c);
                ctx.$op(ty, a, inner)
            }
        )*
    };
}

tri_ops! {
    (emit_s_min_tri32, OpSMin3AMD, OpSMin),
    (emit_u_min_tri32, OpUMin3AMD, OpUMin),
    (emit_s_max_tri32, OpSMax3AMD, OpSMax),
    (emit_u_max_tri32, OpUMax3AMD, OpUMax),
}

pub fn emit_s_med_tri32(ctx: &mut EmitContext, a: Id, b: Id, c: Id) -> Id {
    let types = ctx.type_ids();
    if ctx.profile.supports_trinary_minmax {
        return ctx.OpSMid3AMD(types.U32[1], a, b, c);
    }
    let max_ab = ctx.OpSMax(types.U32[1], a, b);
    let mmx = ctx.OpSMin(types.U32[1], max_ab, c);
    let min_ab = ctx.OpSMin(types.U32[1], a, b);
    ctx.OpSMax(types.U32[1], min_ab, mmx)
}

pub fn emit_u_med_tri32(ctx: &mut EmitContext, a: Id, b: Id, c: Id) -> Id {
    let types = ctx.type_ids();
    if ctx.profile.supports_trinary_minmax {
        return ctx.OpUMid3AMD(types.U32[1], a, b, c);
    }
    let max_ab = ctx.OpUMax(types.U32[1], a, b);
    let mmx = ctx.OpUMin(types.U32[1], max_ab, c);
    let min_ab = ctx.OpUMin(types.U32[1], a, b);
    ctx.OpUMax(types.U32[1], min_ab, mmx)
}

pub fn emit_s_clamp32(
    ctx: &mut EmitContext,
    inst: &InstRef,
    mut value: Id,
    mut min: Id,
    mut max: Id,
) -> Id {
    let types = ctx.type_ids();
    let result = if ctx.profile.has_broken_spirv_clamp {
        value = ctx.OpBitcast(types.S32[1], value);
        min = ctx.OpBitcast(types.S32[1], min);
        max = ctx.OpBitcast(types.S32[1], max);
        let clamped = ctx.OpSMin(types.S32[1], value, max);
        let clamped = ctx.OpSMax(types.S32[1], clamped, min);
        ctx.OpBitcast(types.U32[1], clamped)
    } else {
        ctx.OpSClamp(types.U32[1], value, min, max)
    };
    set_zero_flag(ctx, inst, result);
    set_sign_flag(ctx, inst, result);
    result
}

pub fn emit_u_clamp32(ctx: &mut EmitContext, inst: &InstRef, value: Id, min: Id, max: Id) -> Id {
    let types = ctx.type_ids();
    let result = if ctx.profile.has_broken_spirv_clamp {
        let clamped = ctx.OpUMin(types.U32[1], value, max);
        ctx.OpUMax(types.U32[1], clamped, min)
    } else {
        ctx.OpUClamp(types.U32[1], value, min, max)
    };
    set_zero_flag(ctx, inst, result);
    set_sign_flag(ctx, inst, result);
    result
}

macro_rules! compare_ops {
    ($(($name:ident, $op:ident)),* $(,)?) => {
        $(
            pub fn $name(ctx: &mut EmitContext, lhs: Id, rhs: Id) -> Id {
                let ty = ctx.type_ids().U1[1];
                ctx.$op(ty, lhs, rhs)
            }
        )*
    };
}

compare_ops! {
    (emit_s_less_than32, OpSLessThan),
    (emit_s_less_than64, OpSLessThan),
    (emit_u_less_than32, OpULessThan),
    (emit_u_less_than64, OpULessThan),
    (emit_i_equal32, OpIEqual),
    (emit_i_equal64, OpIEqual),
    (emit_s_less_than_equal32, OpSLessThanEqual),
    (emit_s_less_than_equal64, OpSLessThanEqual),
    (emit_u_less_than_equal32, OpULessThanEqual),
    (emit_u_less_than_equal64, OpULessThanEqual),
    (emit_s_greater_than32, OpSGreaterThan),
    (emit_s_greater_than64, OpSGreaterThan),
    (emit_u_greater_than32, OpUGreaterThan),
    (emit_u_greater_than64, OpUGreaterThan),
    (emit_i_not_equal32, OpINotEqual),
    (emit_i_not_equal64, OpINotEqual),
    (emit_s_greater_than_equal32, OpSGreaterThanEqual),
    (emit_s_greater_than_equal64, OpSGreaterThanEqual),
    (emit_u_greater_than_equal32, OpUGreaterThanEqual),
    (emit_u_greater_than_equal64, OpUGreaterThanEqual),
}

pub fn emit_masked_bit_count32(_ctx: &mut EmitContext, _value: Id, _addend: Id, _hi: bool) -> Id {
    unreachable!("Unreachable instruction")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::backend::bindings::Bindings;
    use crate::gpu::shader_recompiler::backend::spirv::spv;

    fn opcode(word: u32) -> u32 {
        word & 0xffff
    }

    #[test]
    fn an_add_uses_the_u32_result_type() {
        let read = |_| 0u32;
        let mut binding = Bindings::default();
        let mut ctx = super::super::emit_context::test_fragment_context(&mut binding, &read);
        let inst = std::rc::Rc::new(std::cell::RefCell::new(
            crate::gpu::shader_recompiler::ir::microinstruction::Inst::new(
                crate::gpu::shader_recompiler::ir::opcodes::Opcode::IAdd32,
                0,
            ),
        ));
        let result = emit_i_add32(&mut ctx, &inst, 1, 2);
        assert_ne!(result, 0);
        let words = ctx.Assemble();
        let at = words
            .windows(2)
            .position(|pair| opcode(pair[0]) == spv::Op::IAdd)
            .expect("an add");
        assert_eq!(words[at + 1], ctx.U32[1]);
    }

    #[test]
    fn a_64_bit_msb_splits_the_value_in_two() {
        let read = |_| 0u32;
        let mut binding = Bindings::default();
        let mut ctx = super::super::emit_context::test_fragment_context(&mut binding, &read);
        let result = emit_find_u_msb64(&mut ctx, 1);
        assert_ne!(result, 0);
        let words = ctx.Assemble();
        assert_eq!(
            words
                .windows(5)
                .filter(|window| {
                    opcode(window[0]) == spv::Op::ExtInst && window[4] == spv::GLSLstd450::FindUMsb
                })
                .count(),
            2,
            "one per half"
        );
        assert!(words.iter().any(|word| opcode(*word) == spv::Op::Select));
    }
}
