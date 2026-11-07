//! Conversion emitters, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv_convert.cpp`.

use super::builder::Id;
use super::emit_context::EmitContext;

fn extract_u16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpUConvert(types.U16, value)
}

fn extract_s16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpSConvert(types.S16, value)
}

fn extract_u8(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpUConvert(types.U8, value)
}

fn extract_s8(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpSConvert(types.S8, value)
}

pub fn emit_convert_s16f32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let narrowed = ctx.OpConvertFToS(types.U16, value);
    ctx.OpSConvert(types.U32[1], narrowed)
}

pub fn emit_convert_s16f64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let narrowed = ctx.OpConvertFToS(types.U16, value);
    ctx.OpSConvert(types.U32[1], narrowed)
}

pub fn emit_convert_s32f32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpConvertFToS(types.U32[1], value)
}

pub fn emit_convert_s32f64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpConvertFToS(types.U32[1], value)
}

pub fn emit_convert_s64f32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpConvertFToS(types.U64, value)
}

pub fn emit_convert_s64f64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpConvertFToS(types.U64, value)
}

pub fn emit_convert_u16f32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let narrowed = ctx.OpConvertFToU(types.U16, value);
    ctx.OpUConvert(types.U32[1], narrowed)
}

pub fn emit_convert_u16f64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let narrowed = ctx.OpConvertFToU(types.U16, value);
    ctx.OpUConvert(types.U32[1], narrowed)
}

pub fn emit_convert_u32f32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpConvertFToU(types.U32[1], value)
}

pub fn emit_convert_u32f64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpConvertFToU(types.U32[1], value)
}

pub fn emit_convert_u64f32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpConvertFToU(types.U64, value)
}

pub fn emit_convert_u64f64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpConvertFToU(types.U64, value)
}

pub fn emit_convert_u64u32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpUConvert(types.U64, value)
}

pub fn emit_convert_u32u64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpUConvert(types.U32[1], value)
}

pub fn emit_convert_f16f32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpFConvert(types.F16[1], value)
}

pub fn emit_convert_f32f16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpFConvert(types.F32[1], value)
}

pub fn emit_convert_f32f64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpFConvert(types.F32[1], value)
}

pub fn emit_convert_f64f32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpFConvert(types.F64[1], value)
}

pub fn emit_convert_f32s8(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let extracted = extract_s8(ctx, value);
    ctx.OpConvertSToF(types.F32[1], extracted)
}

pub fn emit_convert_f32s16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let extracted = extract_s16(ctx, value);
    ctx.OpConvertSToF(types.F32[1], extracted)
}

pub fn emit_convert_f32s32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpConvertSToF(types.F32[1], value)
}

pub fn emit_convert_f32s64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpConvertSToF(types.F32[1], value)
}

pub fn emit_convert_f32u8(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let extracted = extract_u8(ctx, value);
    ctx.OpConvertUToF(types.F32[1], extracted)
}

pub fn emit_convert_f32u16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let extracted = extract_u16(ctx, value);
    ctx.OpConvertUToF(types.F32[1], extracted)
}

pub fn emit_convert_f32u32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpConvertUToF(types.F32[1], value)
}

pub fn emit_convert_f32u64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpConvertUToF(types.F32[1], value)
}

pub fn emit_convert_f64s8(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let extracted = extract_s8(ctx, value);
    ctx.OpConvertSToF(types.F64[1], extracted)
}

pub fn emit_convert_f64s16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let extracted = extract_s16(ctx, value);
    ctx.OpConvertSToF(types.F64[1], extracted)
}

pub fn emit_convert_f64s32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpConvertSToF(types.F64[1], value)
}

pub fn emit_convert_f64s64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpConvertSToF(types.F64[1], value)
}

pub fn emit_convert_f64u8(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let extracted = extract_u8(ctx, value);
    ctx.OpConvertUToF(types.F64[1], extracted)
}

pub fn emit_convert_f64u16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let extracted = extract_u16(ctx, value);
    ctx.OpConvertUToF(types.F64[1], extracted)
}

pub fn emit_convert_f64u32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpConvertUToF(types.F64[1], value)
}

pub fn emit_convert_f64u64(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpConvertUToF(types.F64[1], value)
}

pub fn emit_convert_u16u32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpUConvert(types.U16, value)
}

pub fn emit_convert_u32u16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpUConvert(types.U32[1], value)
}

pub fn emit_convert_u8u32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpUConvert(types.U8, value)
}

pub fn emit_convert_u32u8(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpUConvert(types.U32[1], value)
}

pub fn emit_convert_s32s8(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpSConvert(types.U32[1], value)
}

pub fn emit_convert_s32s16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpSConvert(types.U32[1], value)
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
    fn a_s16_from_f32_narrows_then_widens() {
        let read = |_| 0u32;
        let mut binding = Bindings::default();
        let mut ctx = super::super::emit_context::test_fragment_context(&mut binding, &read);
        let result = emit_convert_s16f32(&mut ctx, 1);
        assert_ne!(result, 0);
        let words = ctx.Assemble();
        let convert_at = words
            .windows(2)
            .position(|pair| opcode(pair[0]) == spv::Op::ConvertFToS)
            .expect("the float to int conversion");
        let widen_at = words
            .windows(2)
            .position(|pair| opcode(pair[0]) == spv::Op::SConvert)
            .expect("the sign extension");
        assert_eq!(words[convert_at + 1], ctx.U16);
        assert_eq!(words[widen_at + 1], ctx.U32[1]);
        assert!(convert_at < widen_at);
    }
}
