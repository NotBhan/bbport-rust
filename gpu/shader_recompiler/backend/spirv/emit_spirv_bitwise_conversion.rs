//! Bitwise conversion emitters, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv_bitwise_conversion.cpp`.

use super::builder::Id;
use super::emit_context::{EmitContext, VectorIds};

/// `R`: a bit-field offset and width.
#[derive(Debug, Clone, Copy)]
struct R {
    offset: u32,
    size: u32,
}

fn extract_bit_fields(ctx: &mut EmitContext, value: Id, ranges: &[R], is_signed: bool) -> Vec<Id> {
    let types = ctx.type_ids();
    let mut result = Vec::with_capacity(ranges.len());
    for range in ranges {
        let offset = ctx.ConstU32(range.offset);
        let size = ctx.ConstU32(range.size);
        let extracted = if is_signed {
            ctx.OpBitFieldSExtract(types.U32[1], value, offset, size)
        } else {
            ctx.OpBitFieldUExtract(types.U32[1], value, offset, size)
        };
        result.push(extracted);
    }
    result
}

fn insert_bit_fields(ctx: &mut EmitContext, values: &[Id], ranges: &[R]) -> Id {
    let types = ctx.type_ids();
    let mut result = values[0];
    for (value, range) in values.iter().zip(ranges.iter()).skip(1) {
        let offset = ctx.ConstU32(range.offset);
        let size = ctx.ConstU32(range.size);
        result = ctx.OpBitFieldInsert(types.U32[1], result, *value, offset, size);
    }
    result
}

fn extract_composite(ctx: &mut EmitContext, ty: VectorIds, value: Id, count: usize) -> Vec<Id> {
    let component = ty[1];
    (0..count)
        .map(|index| ctx.OpCompositeExtract(component, value, &[index as u32]))
        .collect()
}

pub fn emit_bit_cast_u16f16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpBitcast(types.U16, value)
}

pub fn emit_bit_cast_u32f32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpBitcast(types.U32[1], value)
}

pub fn emit_bit_cast_f16u16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpBitcast(types.F16[1], value)
}

pub fn emit_bit_cast_f32u32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpBitcast(types.F32[1], value)
}

pub fn emit_pack_uint2x32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpBitcast(types.U64, value)
}

pub fn emit_unpack_uint2x32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpBitcast(types.U32[2], value)
}

pub fn emit_pack_double2x32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpBitcast(types.F64[1], value)
}

pub fn emit_unpack_double2x32(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpBitcast(types.U32[2], value)
}

pub fn emit_pack_unorm2x16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpPackUnorm2x16(types.U32[1], value)
}

pub fn emit_unpack_unorm2x16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpUnpackUnorm2x16(types.F32[2], value)
}

pub fn emit_pack_snorm2x16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpPackSnorm2x16(types.U32[1], value)
}

pub fn emit_unpack_snorm2x16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpUnpackSnorm2x16(types.F32[2], value)
}

pub fn emit_pack_uint2x16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let unpacked = ctx.OpBitcast(types.U32[2], value);
    let values = extract_composite(ctx, types.U32, unpacked, 2);
    insert_bit_fields(
        ctx,
        &values[..2],
        &[
            R {
                offset: 0,
                size: 16,
            },
            R {
                offset: 16,
                size: 16,
            },
        ],
    )
}

pub fn emit_unpack_uint2x16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let values = extract_bit_fields(
        ctx,
        value,
        &[
            R {
                offset: 0,
                size: 16,
            },
            R {
                offset: 16,
                size: 16,
            },
        ],
        false,
    );
    let unpacked = ctx.OpCompositeConstruct(types.U32[2], &values);
    ctx.OpBitcast(types.F32[2], unpacked)
}

pub fn emit_pack_sint2x16(ctx: &mut EmitContext, value: Id) -> Id {
    emit_pack_uint2x16(ctx, value)
}

pub fn emit_unpack_sint2x16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let values = extract_bit_fields(
        ctx,
        value,
        &[
            R {
                offset: 0,
                size: 16,
            },
            R {
                offset: 16,
                size: 16,
            },
        ],
        true,
    );
    let unpacked = ctx.OpCompositeConstruct(types.U32[2], &values);
    ctx.OpBitcast(types.F32[2], unpacked)
}

pub fn emit_pack_half2x16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpPackHalf2x16(types.U32[1], value)
}

pub fn emit_unpack_half2x16(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpUnpackHalf2x16(types.F32[2], value)
}

pub fn emit_pack_unorm4x8(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpPackUnorm4x8(types.U32[1], value)
}

pub fn emit_unpack_unorm4x8(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpUnpackUnorm4x8(types.F32[4], value)
}

pub fn emit_pack_snorm4x8(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpPackSnorm4x8(types.U32[1], value)
}

pub fn emit_unpack_snorm4x8(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpUnpackSnorm4x8(types.F32[4], value)
}

pub fn emit_pack_uint4x8(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let unpacked = ctx.OpBitcast(types.U32[4], value);
    let values = extract_composite(ctx, types.U32, unpacked, 4);
    insert_bit_fields(
        ctx,
        &values,
        &[
            R { offset: 0, size: 8 },
            R { offset: 8, size: 8 },
            R {
                offset: 16,
                size: 8,
            },
            R {
                offset: 24,
                size: 8,
            },
        ],
    )
}

pub fn emit_unpack_uint4x8(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let values = extract_bit_fields(
        ctx,
        value,
        &[
            R { offset: 0, size: 8 },
            R { offset: 8, size: 8 },
            R {
                offset: 16,
                size: 8,
            },
            R {
                offset: 24,
                size: 8,
            },
        ],
        false,
    );
    let unpacked = ctx.OpCompositeConstruct(types.U32[4], &values);
    ctx.OpBitcast(types.F32[4], unpacked)
}

pub fn emit_pack_sint4x8(ctx: &mut EmitContext, value: Id) -> Id {
    emit_pack_uint4x8(ctx, value)
}

pub fn emit_unpack_sint4x8(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let values = extract_bit_fields(
        ctx,
        value,
        &[
            R { offset: 0, size: 8 },
            R { offset: 8, size: 8 },
            R {
                offset: 16,
                size: 8,
            },
            R {
                offset: 24,
                size: 8,
            },
        ],
        true,
    );
    let unpacked = ctx.OpCompositeConstruct(types.U32[4], &values);
    ctx.OpBitcast(types.F32[4], unpacked)
}

pub fn emit_pack_ufloat10_11_11(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let values = extract_composite(ctx, types.F32, value, 3);
    let function_11 = types.f32_to_uf11;
    let x = ctx.OpFunctionCall(types.U32[1], function_11, &[values[0]]);
    let y = ctx.OpFunctionCall(types.U32[1], function_11, &[values[1]]);
    let function_10 = types.f32_to_uf10;
    let z = ctx.OpFunctionCall(types.U32[1], function_10, &[values[2]]);
    insert_bit_fields(
        ctx,
        &[x, y, z],
        &[
            R {
                offset: 0,
                size: 11,
            },
            R {
                offset: 11,
                size: 11,
            },
            R {
                offset: 22,
                size: 10,
            },
        ],
    )
}

pub fn emit_unpack_ufloat10_11_11(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let values = extract_bit_fields(
        ctx,
        value,
        &[
            R {
                offset: 0,
                size: 11,
            },
            R {
                offset: 11,
                size: 11,
            },
            R {
                offset: 22,
                size: 10,
            },
        ],
        false,
    );
    let function_11 = types.uf11_to_f32;
    let x = ctx.OpFunctionCall(types.F32[1], function_11, &[values[0]]);
    let y = ctx.OpFunctionCall(types.F32[1], function_11, &[values[1]]);
    let function_10 = types.uf10_to_f32;
    let z = ctx.OpFunctionCall(types.F32[1], function_10, &[values[2]]);
    ctx.OpCompositeConstruct(types.F32[3], &[x, y, z])
}

pub fn emit_pack_unorm2_10_10_10(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let zero = ctx.ConstF32(0.0);
    let one = ctx.ConstF32(1.0);
    let unorm_min = ctx.ConstantComposite(types.F32[4], &[zero, zero, zero, zero]);
    let unorm_max = ctx.ConstantComposite(types.F32[4], &[one, one, one, one]);
    let clamped = ctx.OpFClamp(types.F32[4], value, unorm_min, unorm_max);
    let high = ctx.ConstF32(1023.0);
    let low = ctx.ConstF32(3.0);
    let unorm_mul = ctx.ConstantComposite(types.F32[4], &[high, high, high, low]);
    let as_float = ctx.OpFMul(types.F32[4], clamped, unorm_mul);
    let rounded = ctx.OpRoundEven(types.F32[4], as_float);
    let as_uint = ctx.OpConvertFToU(types.U32[4], rounded);
    let packed = ctx.OpBitcast(types.F32[4], as_uint);
    emit_pack_uint2_10_10_10(ctx, packed)
}

pub fn emit_unpack_unorm2_10_10_10(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let unpacked_value = emit_unpack_uint2_10_10_10(ctx, value);
    let unpacked = ctx.OpBitcast(types.U32[4], unpacked_value);
    let as_float = ctx.OpConvertUToF(types.F32[4], unpacked);
    let high = ctx.ConstF32(1023.0);
    let low = ctx.ConstF32(3.0);
    let unorm_div = ctx.ConstantComposite(types.F32[4], &[high, high, high, low]);
    ctx.OpFDiv(types.F32[4], as_float, unorm_div)
}

pub fn emit_pack_snorm2_10_10_10(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let neg_one = ctx.ConstF32(-1.0);
    let one = ctx.ConstF32(1.0);
    let snorm_min = ctx.ConstantComposite(types.F32[4], &[neg_one, neg_one, neg_one, neg_one]);
    let snorm_max = ctx.ConstantComposite(types.F32[4], &[one, one, one, one]);
    let clamped = ctx.OpFClamp(types.F32[4], value, snorm_min, snorm_max);
    let high = ctx.ConstF32(511.0);
    let low = ctx.ConstF32(1.0);
    let snorm_mul = ctx.ConstantComposite(types.F32[4], &[high, high, high, low]);
    let as_float = ctx.OpFMul(types.F32[4], clamped, snorm_mul);
    let rounded = ctx.OpRoundEven(types.F32[4], as_float);
    let as_sint = ctx.OpConvertFToS(types.U32[4], rounded);
    let packed = ctx.OpBitcast(types.F32[4], as_sint);
    emit_pack_sint2_10_10_10(ctx, packed)
}

pub fn emit_unpack_snorm2_10_10_10(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let unpacked_value = emit_unpack_sint2_10_10_10(ctx, value);
    let unpacked = ctx.OpBitcast(types.U32[4], unpacked_value);
    let as_float = ctx.OpConvertSToF(types.F32[4], unpacked);
    let high = ctx.ConstF32(511.0);
    let low = ctx.ConstF32(1.0);
    let snorm_div = ctx.ConstantComposite(types.F32[4], &[high, high, high, low]);
    ctx.OpFDiv(types.F32[4], as_float, snorm_div)
}

pub fn emit_pack_uint2_10_10_10(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let unpacked = ctx.OpBitcast(types.U32[4], value);
    let values = extract_composite(ctx, types.U32, unpacked, 4);
    insert_bit_fields(
        ctx,
        &values,
        &[
            R {
                offset: 0,
                size: 10,
            },
            R {
                offset: 10,
                size: 10,
            },
            R {
                offset: 20,
                size: 10,
            },
            R {
                offset: 30,
                size: 2,
            },
        ],
    )
}

pub fn emit_unpack_uint2_10_10_10(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let values = extract_bit_fields(
        ctx,
        value,
        &[
            R {
                offset: 0,
                size: 10,
            },
            R {
                offset: 10,
                size: 10,
            },
            R {
                offset: 20,
                size: 10,
            },
            R {
                offset: 30,
                size: 2,
            },
        ],
        false,
    );
    let unpacked = ctx.OpCompositeConstruct(types.U32[4], &values);
    ctx.OpBitcast(types.F32[4], unpacked)
}

pub fn emit_pack_sint2_10_10_10(ctx: &mut EmitContext, value: Id) -> Id {
    emit_pack_uint2_10_10_10(ctx, value)
}

pub fn emit_unpack_sint2_10_10_10(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    let values = extract_bit_fields(
        ctx,
        value,
        &[
            R {
                offset: 0,
                size: 10,
            },
            R {
                offset: 10,
                size: 10,
            },
            R {
                offset: 20,
                size: 10,
            },
            R {
                offset: 30,
                size: 2,
            },
        ],
        true,
    );
    let unpacked = ctx.OpCompositeConstruct(types.U32[4], &values);
    ctx.OpBitcast(types.F32[4], unpacked)
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
    fn packing_two_u16_fields_extracts_and_inserts() {
        let read = |_| 0u32;
        let mut binding = Bindings::default();
        let mut ctx = super::super::emit_context::test_fragment_context(&mut binding, &read);
        let packed = emit_pack_uint2x16(&mut ctx, 1);
        assert_ne!(packed, 0);
        let words = ctx.Assemble();
        assert_eq!(
            words
                .iter()
                .filter(|word| opcode(**word) == spv::Op::CompositeExtract)
                .count(),
            2
        );
        assert_eq!(
            words
                .iter()
                .filter(|word| opcode(**word) == spv::Op::BitFieldInsert)
                .count(),
            1
        );
    }

    #[test]
    fn unpacking_to_float_constructs_then_bitcasts() {
        let read = |_| 0u32;
        let mut binding = Bindings::default();
        let mut ctx = super::super::emit_context::test_fragment_context(&mut binding, &read);
        let unpacked = emit_unpack_uint2x16(&mut ctx, 1);
        assert_ne!(unpacked, 0);
        let words = ctx.Assemble();
        assert!(words
            .iter()
            .any(|word| opcode(*word) == spv::Op::CompositeConstruct));
        assert!(words.iter().any(|word| opcode(*word) == spv::Op::Bitcast));
    }
}
