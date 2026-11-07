//! Value swizzles and number conversions, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/reinterpret.h`. The functions emit through the IR
//! emitter, so they take a mutable emitter and return the value they built.

use super::ir_emitter::IrEmitter;
use super::value::{Value, F32, F32F64, U32, U32U64};
use crate::gpu::amdgpu::pixel_format::{CompMapping, NumberConversion};

fn narrow(value: F32F64) -> F32 {
    F32::new(value.into_value())
}

fn widen(value: &F32) -> F32F64 {
    F32F64::new(value.value().clone())
}

/// `ApplySwizzle`: the constants `0` and `1` are indices 0 and 1 of the first vector, and the
/// components are indices 4 through 7 of the second.
pub fn apply_swizzle(ir: &mut IrEmitter, vector: &Value, swizzle: CompMapping) -> Value {
    let zero = ir.imm32_float(0.0);
    let one = ir.imm32_float(1.0);
    let zero_b = ir.imm32_float(0.0);
    let zero_a = ir.imm32_float(0.0);
    let constants = ir.composite_construct4(zero, one, zero_b, zero_a);
    ir.composite_shuffle4(
        &constants,
        vector,
        swizzle.r as usize,
        swizzle.g as usize,
        swizzle.b as usize,
        swizzle.a as usize,
    )
}

/// `ApplyGammaToLinear`.
pub fn apply_gamma_to_linear(ir: &mut IrEmitter, c: &F32) -> F32 {
    let bias = ir.imm32_float(0.055);
    let added = narrow(ir.fp_add(&widen(c), &widen(&bias)));
    let scale = ir.imm32_float(1.0 / 1.055);
    let scaled = narrow(ir.fp_mul(&widen(&added), &widen(&scale)));
    let exponent = ir.imm32_float(2.4);
    let a = ir.fp_pow(&scaled, &exponent);
    let slope = ir.imm32_float(1.0 / 12.92);
    let b = narrow(ir.fp_mul(&widen(c), &widen(&slope)));
    let threshold = ir.imm32_float(0.04045);
    let cond = ir.fp_greater_than(&widen(c), &widen(&threshold), true);
    F32::new(ir.select(&cond, &a.into_value(), &b.into_value()))
}

/// `ApplyForceDegamma`: only the RGB components are converted.
pub fn apply_force_degamma(ir: &mut IrEmitter, value: &Value) -> Value {
    let x = F32::new(ir.composite_extract(value, 0));
    let y = F32::new(ir.composite_extract(value, 1));
    let z = F32::new(ir.composite_extract(value, 2));
    let w = F32::new(ir.composite_extract(value, 3));
    let x = apply_gamma_to_linear(ir, &x);
    let y = apply_gamma_to_linear(ir, &y);
    let z = apply_gamma_to_linear(ir, &z);
    ir.composite_construct4(x, y, z, w)
}

/// `ApplyReadNumberConversion`.
pub fn apply_read_number_conversion(
    ir: &mut IrEmitter,
    value: &F32,
    conversion: NumberConversion,
) -> F32 {
    match conversion {
        NumberConversion::None => value.clone(),
        NumberConversion::UintToUscaled => {
            let bits = ir.bit_cast_u32_f32(value);
            narrow(ir.convert_u_to_f(32, 32, &bits.into_value()))
        }
        NumberConversion::SintToSscaled => {
            let bits = ir.bit_cast_u32_f32(value);
            narrow(ir.convert_s_to_f(32, 32, &bits.into_value()))
        }
        NumberConversion::UnormToUbnorm => {
            let two = ir.imm32_float(2.0);
            let scaled = narrow(ir.fp_mul(&widen(value), &widen(&two)));
            let one = ir.imm32_float(1.0);
            narrow(ir.fp_sub(&widen(&scaled), &widen(&one)))
        }
        NumberConversion::Sint8ToSnormNz => {
            let bits = ir.bit_cast_u32_f32(value);
            let two = ir.imm32(2u32);
            let doubled = ir.i_mul(
                &U32U64::new(bits.into_value()),
                &U32U64::new(two.into_value()),
            );
            let one = ir.imm32(1u32);
            let sum = ir.i_add(&doubled, &U32U64::new(one.into_value()));
            let left = narrow(ir.convert_s_to_f(32, 32, &sum.into_value()));
            let max = ir.imm32_float(u8::MAX as f32);
            narrow(ir.fp_div(&widen(&left), &widen(&max)))
        }
        NumberConversion::Sint16ToSnormNz => {
            let bits = ir.bit_cast_u32_f32(value);
            let two = ir.imm32(2u32);
            let doubled = ir.i_mul(
                &U32U64::new(bits.into_value()),
                &U32U64::new(two.into_value()),
            );
            let one = ir.imm32(1u32);
            let sum = ir.i_add(&doubled, &U32U64::new(one.into_value()));
            let left = narrow(ir.convert_s_to_f(32, 32, &sum.into_value()));
            let max = ir.imm32_float(u16::MAX as f32);
            narrow(ir.fp_div(&widen(&left), &widen(&max)))
        }
        NumberConversion::Uint32ToUnorm => {
            let bits = ir.bit_cast_u32_f32(value);
            let value = narrow(ir.convert_u_to_f(32, 32, &bits.into_value()));
            let max = ir.imm32_float(u32::MAX as f32);
            narrow(ir.fp_div(&widen(&value), &widen(&max)))
        }
        NumberConversion::SrgbToNorm => apply_gamma_to_linear(ir, value),
    }
}

/// `ApplyReadNumberConversionVec4`.
pub fn apply_read_number_conversion_vec4(
    ir: &mut IrEmitter,
    value: &Value,
    conversion: NumberConversion,
) -> Value {
    if conversion == NumberConversion::None {
        return value.clone();
    }
    let x = F32::new(ir.composite_extract(value, 0));
    let y = F32::new(ir.composite_extract(value, 1));
    let z = F32::new(ir.composite_extract(value, 2));
    let w = F32::new(ir.composite_extract(value, 3));
    let x = apply_read_number_conversion(ir, &x, conversion);
    let y = apply_read_number_conversion(ir, &y, conversion);
    let z = apply_read_number_conversion(ir, &z, conversion);
    let w = apply_read_number_conversion(ir, &w, conversion);
    ir.composite_construct4(x, y, z, w)
}

/// `ApplyWriteNumberConversion`.
pub fn apply_write_number_conversion(
    ir: &mut IrEmitter,
    value: &F32,
    conversion: NumberConversion,
) -> F32 {
    match conversion {
        NumberConversion::None => value.clone(),
        NumberConversion::UintToUscaled => {
            let raw = U32::new(ir.convert_f_to_u(32, &widen(value)).into_value());
            ir.bit_cast_f32_u32(&raw)
        }
        NumberConversion::SintToSscaled => {
            let raw = U32::new(ir.convert_f_to_s(32, &widen(value)).into_value());
            ir.bit_cast_f32_u32(&raw)
        }
        NumberConversion::UnormToUbnorm => {
            let one = ir.imm32_float(1.0);
            let shifted = narrow(ir.fp_add(&widen(value), &widen(&one)));
            let two = ir.imm32_float(2.0);
            narrow(ir.fp_div(&widen(&shifted), &widen(&two)))
        }
        NumberConversion::Sint8ToSnormNz => {
            let minus_one = ir.imm32_float(-1.0);
            let one = ir.imm32_float(1.0);
            let clamped = narrow(ir.fp_clamp(&widen(value), &widen(&minus_one), &widen(&one)));
            let max = ir.imm32_float(u8::MAX as f32);
            let scaled = narrow(ir.fp_mul(&widen(&clamped), &widen(&max)));
            let one = ir.imm32_float(1.0);
            let left = narrow(ir.fp_sub(&widen(&scaled), &widen(&one)));
            let two = ir.imm32_float(2.0);
            let divided = narrow(ir.fp_div(&widen(&left), &widen(&two)));
            let raw = U32::new(ir.convert_f_to_s(32, &widen(&divided)).into_value());
            ir.bit_cast_f32_u32(&raw)
        }
        NumberConversion::Sint16ToSnormNz => {
            let minus_one = ir.imm32_float(-1.0);
            let one = ir.imm32_float(1.0);
            let clamped = narrow(ir.fp_clamp(&widen(value), &widen(&minus_one), &widen(&one)));
            let max = ir.imm32_float(u16::MAX as f32);
            let scaled = narrow(ir.fp_mul(&widen(&clamped), &widen(&max)));
            let one = ir.imm32_float(1.0);
            let left = narrow(ir.fp_sub(&widen(&scaled), &widen(&one)));
            let two = ir.imm32_float(2.0);
            let divided = narrow(ir.fp_div(&widen(&left), &widen(&two)));
            let raw = U32::new(ir.convert_f_to_s(32, &widen(&divided)).into_value());
            ir.bit_cast_f32_u32(&raw)
        }
        NumberConversion::Uint32ToUnorm => {
            let zero = ir.imm32_float(0.0);
            let one = ir.imm32_float(1.0);
            let clamped = narrow(ir.fp_clamp(&widen(value), &widen(&zero), &widen(&one)));
            let max = ir.imm32_float(u32::MAX as f32);
            let unnormalized = narrow(ir.fp_mul(&widen(&clamped), &widen(&max)));
            let raw = U32::new(ir.convert_f_to_u(32, &widen(&unnormalized)).into_value());
            ir.bit_cast_f32_u32(&raw)
        }
        NumberConversion::SrgbToNorm => panic!("Unreachable"),
    }
}

/// `ApplyWriteNumberConversionVec4`.
pub fn apply_write_number_conversion_vec4(
    ir: &mut IrEmitter,
    value: &Value,
    conversion: NumberConversion,
) -> Value {
    if conversion == NumberConversion::None {
        return value.clone();
    }
    let x = F32::new(ir.composite_extract(value, 0));
    let y = F32::new(ir.composite_extract(value, 1));
    let z = F32::new(ir.composite_extract(value, 2));
    let w = F32::new(ir.composite_extract(value, 3));
    let x = apply_write_number_conversion(ir, &x, conversion);
    let y = apply_write_number_conversion(ir, &y, conversion);
    let z = apply_write_number_conversion(ir, &z, conversion);
    let w = apply_write_number_conversion(ir, &w, conversion);
    ir.composite_construct4(x, y, z, w)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::amdgpu::pixel_format::CompSwizzle;
    use crate::gpu::shader_recompiler::ir::basic_block::Block;
    use crate::gpu::shader_recompiler::ir::opcodes::Opcode;

    fn emitter() -> (
        crate::gpu::shader_recompiler::ir::basic_block::BlockRef,
        IrEmitter,
    ) {
        let block = Block::new_ref();
        let ir = IrEmitter::new(&block);
        (block, ir)
    }

    fn opcodes(block: &crate::gpu::shader_recompiler::ir::basic_block::BlockRef) -> Vec<Opcode> {
        block
            .borrow()
            .instructions()
            .iter()
            .map(|inst| inst.borrow().get_opcode())
            .collect()
    }

    #[test]
    fn a_swizzle_builds_the_constant_vector_then_shuffles() {
        let (block, mut ir) = emitter();
        let vector = ir.composite_construct4(
            ir.imm32_float(1.0),
            ir.imm32_float(2.0),
            ir.imm32_float(3.0),
            ir.imm32_float(4.0),
        );
        let swizzle = CompMapping {
            r: CompSwizzle::Zero,
            g: CompSwizzle::One,
            b: CompSwizzle::Alpha,
            a: CompSwizzle::Blue,
        };
        apply_swizzle(&mut ir, &vector, swizzle);

        assert_eq!(
            opcodes(&block),
            vec![
                Opcode::CompositeConstructF32x4,
                Opcode::CompositeConstructF32x4,
                Opcode::CompositeShuffleF32x4,
            ]
        );
        let shuffle = block.borrow().instructions[2].clone();
        let inst = shuffle.borrow();
        assert_eq!(inst.arg(2).u32(), 0);
        assert_eq!(inst.arg(3).u32(), 1);
        assert_eq!(inst.arg(4).u32(), 7);
        assert_eq!(inst.arg(5).u32(), 6);
    }

    #[test]
    fn a_none_conversion_returns_the_same_value() {
        let (_block, mut ir) = emitter();
        let value = ir.imm32_float(0.25);
        let converted = apply_read_number_conversion(&mut ir, &value, NumberConversion::None);
        assert_eq!(converted.value(), value.value());
        let vector = ir.composite_construct4(
            ir.imm32_float(0.0),
            ir.imm32_float(0.0),
            ir.imm32_float(0.0),
            ir.imm32_float(0.0),
        );
        let converted = apply_read_number_conversion_vec4(&mut ir, &vector, NumberConversion::None);
        assert_eq!(converted, vector);
    }

    #[test]
    fn the_read_conversions_emit_their_instruction_chains() {
        let (block, mut ir) = emitter();
        let value = ir.imm32_float(0.5);
        apply_read_number_conversion(&mut ir, &value, NumberConversion::UnormToUbnorm);
        assert_eq!(opcodes(&block), vec![Opcode::FPMul32, Opcode::FPSub32]);

        let (block, mut ir) = emitter();
        let value = ir.imm32_float(0.5);
        apply_read_number_conversion(&mut ir, &value, NumberConversion::Uint32ToUnorm);
        assert_eq!(
            opcodes(&block),
            vec![
                Opcode::BitCastU32F32,
                Opcode::ConvertF32U32,
                Opcode::FPDiv32
            ]
        );

        let (block, mut ir) = emitter();
        let value = ir.imm32_float(0.5);
        apply_read_number_conversion(&mut ir, &value, NumberConversion::SrgbToNorm);
        assert_eq!(
            opcodes(&block),
            vec![
                Opcode::FPAdd32,
                Opcode::FPMul32,
                Opcode::FPPow,
                Opcode::FPMul32,
                Opcode::FPOrdGreaterThan32,
                Opcode::SelectF32,
            ]
        );
    }

    #[test]
    fn the_write_conversions_emit_their_instruction_chains() {
        let (block, mut ir) = emitter();
        let value = ir.imm32_float(0.5);
        apply_write_number_conversion(&mut ir, &value, NumberConversion::Uint32ToUnorm);
        assert_eq!(
            opcodes(&block),
            vec![
                Opcode::FPClamp32,
                Opcode::FPMul32,
                Opcode::ConvertU32F32,
                Opcode::BitCastF32U32,
            ]
        );

        let (block, mut ir) = emitter();
        let value = ir.imm32_float(0.5);
        apply_write_number_conversion(&mut ir, &value, NumberConversion::UnormToUbnorm);
        assert_eq!(opcodes(&block), vec![Opcode::FPAdd32, Opcode::FPDiv32]);
    }

    #[test]
    fn the_vec4_forms_extract_each_component_and_rebuild() {
        let (block, mut ir) = emitter();
        let vector = ir.composite_construct4(
            ir.imm32_float(0.0),
            ir.imm32_float(1.0),
            ir.imm32_float(2.0),
            ir.imm32_float(3.0),
        );
        apply_read_number_conversion_vec4(&mut ir, &vector, NumberConversion::UnormToUbnorm);
        let ops = opcodes(&block);
        assert_eq!(
            ops.iter()
                .filter(|op| **op == Opcode::CompositeExtractF32x4)
                .count(),
            4
        );
        assert_eq!(
            ops.iter()
                .filter(|op| **op == Opcode::CompositeConstructF32x4)
                .count(),
            2,
            "the input vector and the rebuilt one"
        );
    }

    #[test]
    fn force_degamma_converts_only_the_rgb_components() {
        let (block, mut ir) = emitter();
        let vector = ir.composite_construct4(
            ir.imm32_float(0.0),
            ir.imm32_float(1.0),
            ir.imm32_float(2.0),
            ir.imm32_float(3.0),
        );
        apply_force_degamma(&mut ir, &vector);
        let ops = opcodes(&block);
        assert_eq!(ops.iter().filter(|op| **op == Opcode::FPPow).count(), 3);
        assert_eq!(*ops.last().unwrap(), Opcode::CompositeConstructF32x4);
    }
}
