//! Floating-point emitters, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv_floating_point.cpp`.

use super::builder::Id;
use super::emit_context::EmitContext;
use super::spv;
use crate::gpu::shader_recompiler::ir::microinstruction::InstRef;

fn decorate(ctx: &mut EmitContext, _inst: &InstRef, op: Id) -> Id {
    ctx.Decorate(op, spv::Decoration::NoContraction, &[]);
    op
}

// The macro above cannot dispatch by name; the operations below are written out instead.
pub fn emit_fp_abs32(ctx: &mut EmitContext, value: Id) -> Id {
    let ty = ctx.type_ids().F32[1];
    ctx.OpFAbs(ty, value)
}

pub fn emit_fp_abs64(ctx: &mut EmitContext, value: Id) -> Id {
    let ty = ctx.type_ids().F64[1];
    ctx.OpFAbs(ty, value)
}

macro_rules! binary_decorated {
    ($(($name:ident, $field:ident, $op:ident)),* $(,)?) => {
        $(
            pub fn $name(
                ctx: &mut EmitContext,
                inst: &InstRef,
                a: Id,
                b: Id,
            ) -> Id {
                let ty = ctx.type_ids().$field[1];
                let op = ctx.$op(ty, a, b);
                decorate(ctx, inst, op)
            }
        )*
    };
}

binary_decorated! {
    (emit_fp_add32, F32, OpFAdd),
    (emit_fp_add64, F64, OpFAdd),
    (emit_fp_sub32, F32, OpFSub),
    (emit_fp_mul32, F32, OpFMul),
    (emit_fp_mul64, F64, OpFMul),
    (emit_fp_div32, F32, OpFDiv),
    (emit_fp_div64, F64, OpFDiv),
}

macro_rules! ternary_decorated {
    ($(($name:ident, $field:ident, $op:ident)),* $(,)?) => {
        $(
            pub fn $name(
                ctx: &mut EmitContext,
                inst: &InstRef,
                a: Id,
                b: Id,
                c: Id,
            ) -> Id {
                let ty = ctx.type_ids().$field[1];
                let op = ctx.$op(ty, a, b, c);
                decorate(ctx, inst, op)
            }
        )*
    };
}

ternary_decorated! {
    (emit_fp_fma32, F32, OpFma),
    (emit_fp_fma64, F64, OpFma),
}

macro_rules! binary_f32_f64 {
    ($(($name:ident, $field:ident, $op:ident)),* $(,)?) => {
        $(
            pub fn $name(ctx: &mut EmitContext, a: Id, b: Id) -> Id {
                let ty = ctx.type_ids().$field[1];
                ctx.$op(ty, a, b)
            }
        )*
    };
}

binary_f32_f64! {
    (emit_fp_max32, F32, OpFMax),
    (emit_fp_max64, F64, OpFMax),
    (emit_fp_min32, F32, OpFMin),
    (emit_fp_min64, F64, OpFMin),
}

pub fn emit_fp_min_tri32(ctx: &mut EmitContext, a: Id, b: Id, c: Id) -> Id {
    let types = ctx.type_ids();
    if ctx.profile.supports_trinary_minmax {
        return ctx.OpFMin3AMD(types.F32[1], a, b, c);
    }
    let inner = ctx.OpFMin(types.F32[1], b, c);
    ctx.OpFMin(types.F32[1], a, inner)
}

pub fn emit_fp_max_tri32(ctx: &mut EmitContext, a: Id, b: Id, c: Id) -> Id {
    let types = ctx.type_ids();
    if ctx.profile.supports_trinary_minmax {
        return ctx.OpFMax3AMD(types.F32[1], a, b, c);
    }
    let inner = ctx.OpFMax(types.F32[1], b, c);
    ctx.OpFMax(types.F32[1], a, inner)
}

pub fn emit_fp_med_tri32(ctx: &mut EmitContext, a: Id, b: Id, c: Id) -> Id {
    let types = ctx.type_ids();
    if ctx.profile.supports_trinary_minmax {
        return ctx.OpFMid3AMD(types.F32[1], a, b, c);
    }
    let max_ab = ctx.OpFMax(types.F32[1], a, b);
    let mmx = ctx.OpFMin(types.F32[1], max_ab, c);
    let min_ab = ctx.OpFMin(types.F32[1], a, b);
    ctx.OpFMax(types.F32[1], min_ab, mmx)
}

macro_rules! unary_std {
    ($(($name:ident, $field:ident, $op:ident)),* $(,)?) => {
        $(
            pub fn $name(ctx: &mut EmitContext, value: Id) -> Id {
                let ty = ctx.type_ids().$field[1];
                ctx.$op(ty, value)
            }
        )*
    };
}

unary_std! {
    (emit_fp_neg32, F32, OpFNegate),
    (emit_fp_neg64, F64, OpFNegate),
    (emit_fp_exp2, F32, OpExp2),
    (emit_fp_log2, F32, OpLog2),
    (emit_fp_recip_sqrt32, F32, OpInverseSqrt),
    (emit_fp_recip_sqrt64, F64, OpInverseSqrt),
    (emit_fp_sqrt, F32, OpSqrt),
    (emit_fp_round_even32, F32, OpRoundEven),
    (emit_fp_round_even64, F64, OpRoundEven),
    (emit_fp_floor32, F32, OpFloor),
    (emit_fp_floor64, F64, OpFloor),
    (emit_fp_ceil32, F32, OpCeil),
    (emit_fp_ceil64, F64, OpCeil),
    (emit_fp_trunc32, F32, OpTrunc),
    (emit_fp_trunc64, F64, OpTrunc),
    (emit_fp_fract32, F32, OpFract),
    (emit_fp_fract64, F64, OpFract),
    (emit_fp_is_nan16, U1, OpIsNan),
    (emit_fp_is_nan32, U1, OpIsNan),
    (emit_fp_is_nan64, U1, OpIsNan),
    (emit_fp_is_inf32, U1, OpIsInf),
    (emit_fp_is_inf64, U1, OpIsInf),
}

pub fn emit_fp_sin(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let scaled = ctx.OpFMul(types.F32[1], types.pi_x2, value);
    ctx.OpSin(types.F32[1], scaled)
}

pub fn emit_fp_cos(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let scaled = ctx.OpFMul(types.F32[1], types.pi_x2, value);
    ctx.OpCos(types.F32[1], scaled)
}

pub fn emit_fp_pow(ctx: &mut EmitContext, x: Id, y: Id) -> Id {
    let ty = ctx.type_ids().F32[1];
    ctx.OpPow(ty, x, y)
}

pub fn emit_fp_ldexp(ctx: &mut EmitContext, value: Id, exp: Id) -> Id {
    let ty = ctx.type_ids().F32[1];
    ctx.OpLdexp(ty, value, exp)
}

pub fn emit_fp_recip32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let one = ctx.ConstF32(1.0);
    ctx.OpFDiv(types.F32[1], one, value)
}

pub fn emit_fp_recip64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let ty = types.F64[1];
    let one = ctx.Constant(ty, 1.0f64);
    ctx.OpFDiv(ty, one, value)
}

pub fn emit_fp_saturate32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let zero = ctx.ConstF32(0.0);
    let one = ctx.ConstF32(1.0);
    ctx.OpFClamp(types.F32[1], value, zero, one)
}

pub fn emit_fp_saturate64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let ty = types.F64[1];
    let zero = ctx.Constant(ty, 0.0f64);
    let one = ctx.Constant(ty, 1.0f64);
    ctx.OpFClamp(ty, value, zero, one)
}

pub fn emit_fp_clamp32(ctx: &mut EmitContext, value: Id, min_value: Id, max_value: Id) -> Id {
    let ty = ctx.type_ids().F32[1];
    ctx.OpFClamp(ty, value, min_value, max_value)
}

pub fn emit_fp_clamp64(ctx: &mut EmitContext, value: Id, min_value: Id, max_value: Id) -> Id {
    let ty = ctx.type_ids().F64[1];
    ctx.OpFClamp(ty, value, min_value, max_value)
}

pub fn emit_fp_frexp_sig32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let frexp = ctx.OpFrexpStruct(types.frexp_result_f32, value);
    ctx.OpCompositeExtract(types.F32[1], frexp, &[0])
}

pub fn emit_fp_frexp_sig64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let frexp = ctx.OpFrexpStruct(types.frexp_result_f64, value);
    ctx.OpCompositeExtract(types.F64[1], frexp, &[0])
}

pub fn emit_fp_frexp_exp32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let frexp = ctx.OpFrexpStruct(types.frexp_result_f32, value);
    let exponent = ctx.OpCompositeExtract(types.S32[1], frexp, &[1]);
    ctx.OpBitcast(types.U32[1], exponent)
}

pub fn emit_fp_frexp_exp64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let frexp = ctx.OpFrexpStruct(types.frexp_result_f64, value);
    let exponent = ctx.OpCompositeExtract(types.S32[1], frexp, &[1]);
    ctx.OpBitcast(types.U32[1], exponent)
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
    (emit_fp_ord_equal16, OpFOrdEqual),
    (emit_fp_ord_equal32, OpFOrdEqual),
    (emit_fp_ord_equal64, OpFOrdEqual),
    (emit_fp_unord_equal16, OpFUnordEqual),
    (emit_fp_unord_equal32, OpFUnordEqual),
    (emit_fp_unord_equal64, OpFUnordEqual),
    (emit_fp_ord_not_equal16, OpFOrdNotEqual),
    (emit_fp_ord_not_equal32, OpFOrdNotEqual),
    (emit_fp_ord_not_equal64, OpFOrdNotEqual),
    (emit_fp_unord_not_equal16, OpFUnordNotEqual),
    (emit_fp_unord_not_equal32, OpFUnordNotEqual),
    (emit_fp_unord_not_equal64, OpFUnordNotEqual),
    (emit_fp_ord_less_than16, OpFOrdLessThan),
    (emit_fp_ord_less_than32, OpFOrdLessThan),
    (emit_fp_ord_less_than64, OpFOrdLessThan),
    (emit_fp_unord_less_than16, OpFUnordLessThan),
    (emit_fp_unord_less_than32, OpFUnordLessThan),
    (emit_fp_unord_less_than64, OpFUnordLessThan),
    (emit_fp_ord_greater_than16, OpFOrdGreaterThan),
    (emit_fp_ord_greater_than32, OpFOrdGreaterThan),
    (emit_fp_ord_greater_than64, OpFOrdGreaterThan),
    (emit_fp_unord_greater_than16, OpFUnordGreaterThan),
    (emit_fp_unord_greater_than32, OpFUnordGreaterThan),
    (emit_fp_unord_greater_than64, OpFUnordGreaterThan),
    (emit_fp_ord_less_than_equal16, OpFOrdLessThanEqual),
    (emit_fp_ord_less_than_equal32, OpFOrdLessThanEqual),
    (emit_fp_ord_less_than_equal64, OpFOrdLessThanEqual),
    (emit_fp_unord_less_than_equal16, OpFUnordLessThanEqual),
    (emit_fp_unord_less_than_equal32, OpFUnordLessThanEqual),
    (emit_fp_unord_less_than_equal64, OpFUnordLessThanEqual),
    (emit_fp_ord_greater_than_equal16, OpFOrdGreaterThanEqual),
    (emit_fp_ord_greater_than_equal32, OpFOrdGreaterThanEqual),
    (emit_fp_ord_greater_than_equal64, OpFOrdGreaterThanEqual),
    (emit_fp_unord_greater_than_equal16, OpFUnordGreaterThanEqual),
    (emit_fp_unord_greater_than_equal32, OpFUnordGreaterThanEqual),
    (emit_fp_unord_greater_than_equal64, OpFUnordGreaterThanEqual),
}

pub fn emit_fp_cmp_class32(_ctx: &mut EmitContext) {
    unreachable!()
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
    fn an_add_is_marked_no_contraction() {
        let read = |_| 0u32;
        let mut binding = Bindings::default();
        let mut ctx = super::super::emit_context::test_fragment_context(&mut binding, &read);
        let inst = std::rc::Rc::new(std::cell::RefCell::new(
            crate::gpu::shader_recompiler::ir::microinstruction::Inst::new(
                crate::gpu::shader_recompiler::ir::opcodes::Opcode::FPAdd32,
                0,
            ),
        ));
        let result = emit_fp_add32(&mut ctx, &inst, 1, 2);
        assert_ne!(result, 0);
        let words = ctx.Assemble();
        let decorate_at = words
            .windows(3)
            .position(|window| {
                opcode(window[0]) == spv::Op::Decorate
                    && window[2] == spv::Decoration::NoContraction
            })
            .expect("the no-contraction decoration");
        assert_eq!(words[decorate_at + 1], result);
    }

    #[test]
    fn frexp_extracts_the_exponent_and_bitcasts_it() {
        let read = |_| 0u32;
        let mut binding = Bindings::default();
        let mut ctx = super::super::emit_context::test_fragment_context(&mut binding, &read);
        let result = emit_fp_frexp_exp32(&mut ctx, 1);
        assert_ne!(result, 0);
        let words = ctx.Assemble();
        let _ext_at = words
            .windows(6)
            .position(|window| {
                opcode(window[0]) == spv::Op::ExtInst && window[4] == spv::GLSLstd450::FrexpStruct
            })
            .expect("the frexp ext inst");
        assert!(words.iter().any(|word| opcode(*word) == spv::Op::Bitcast));
    }
}
