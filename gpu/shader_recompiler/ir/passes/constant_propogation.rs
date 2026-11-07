//! Constant propagation, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/constant_propagation_pass.cpp`.
//!
//! The C++ infers each fold's operand types from its lambda (`Arg<T>` and
//! `FoldWhenAllImmediates`); the port spells them with an `Immediate` trait and one- to
//! four-argument fold helpers. Unsigned arithmetic and shifts wrap, matching the C++'s unsigned
//! operations and the hardware's masked shift counts.

use crate::gpu::shader_recompiler::ir::basic_block::{BlockList, BlockRef};
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::microinstruction::{
    invalidate, replace_uses_with, set_arg, InstRef,
};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::value::Value;

/// `Arg<T>`: reads an immediate as one of the types a fold is written for.
trait Immediate: Copy {
    fn read(value: &Value) -> Self;
    fn into_value(self) -> Value;
}

impl Immediate for bool {
    fn read(value: &Value) -> Self {
        value.u1()
    }

    fn into_value(self) -> Value {
        Value::from_u1(self)
    }
}

impl Immediate for u32 {
    fn read(value: &Value) -> Self {
        value.u32()
    }

    fn into_value(self) -> Value {
        Value::from_u32(self)
    }
}

impl Immediate for i32 {
    fn read(value: &Value) -> Self {
        value.u32() as i32
    }

    fn into_value(self) -> Value {
        Value::from_u32(self as u32)
    }
}

impl Immediate for f32 {
    fn read(value: &Value) -> Self {
        value.f32()
    }

    fn into_value(self) -> Value {
        Value::from_f32(self)
    }
}

impl Immediate for u64 {
    fn read(value: &Value) -> Self {
        value.u64()
    }

    fn into_value(self) -> Value {
        Value::from_u64(self)
    }
}

impl Immediate for i64 {
    fn read(value: &Value) -> Self {
        value.u64() as i64
    }

    fn into_value(self) -> Value {
        Value::from_u64(self as u64)
    }
}

fn arguments(inst: &InstRef) -> Vec<Value> {
    let inst = inst.borrow();
    (0..inst.num_args()).map(|index| inst.arg(index)).collect()
}

fn fold_when_all_immediates1<A: Immediate, R: Immediate>(
    inst: &InstRef,
    f: impl FnOnce(A) -> R,
) -> bool {
    if !inst.borrow().are_all_args_immediates() {
        return false;
    }
    let args = arguments(inst);
    replace_uses_with(inst, f(A::read(&args[0])).into_value());
    true
}

fn fold_when_all_immediates2<A: Immediate, B: Immediate, R: Immediate>(
    inst: &InstRef,
    f: impl FnOnce(A, B) -> R,
) -> bool {
    if !inst.borrow().are_all_args_immediates() {
        return false;
    }
    let args = arguments(inst);
    replace_uses_with(inst, f(A::read(&args[0]), B::read(&args[1])).into_value());
    true
}

fn fold_when_all_immediates3<A: Immediate, B: Immediate, C: Immediate, R: Immediate>(
    inst: &InstRef,
    f: impl FnOnce(A, B, C) -> R,
) -> bool {
    if !inst.borrow().are_all_args_immediates() {
        return false;
    }
    let args = arguments(inst);
    replace_uses_with(
        inst,
        f(A::read(&args[0]), B::read(&args[1]), C::read(&args[2])).into_value(),
    );
    true
}

fn fold_when_all_immediates4<
    A: Immediate,
    B: Immediate,
    C: Immediate,
    D: Immediate,
    R: Immediate,
>(
    inst: &InstRef,
    f: impl FnOnce(A, B, C, D) -> R,
) -> bool {
    if !inst.borrow().are_all_args_immediates() {
        return false;
    }
    let args = arguments(inst);
    replace_uses_with(
        inst,
        f(
            A::read(&args[0]),
            B::read(&args[1]),
            C::read(&args[2]),
            D::read(&args[3]),
        )
        .into_value(),
    );
    true
}

/// `FoldCommutative`.
fn fold_commutative<T: Immediate>(inst: &InstRef, imm_fn: impl Fn(T, T) -> T) -> bool {
    let lhs = inst.borrow().arg(0);
    let rhs = inst.borrow().arg(1);
    let opcode = inst.borrow().get_opcode();

    let lhs_immediate = lhs.is_immediate();
    let rhs_immediate = rhs.is_immediate();

    if lhs_immediate && rhs_immediate {
        let result = imm_fn(T::read(&lhs), T::read(&rhs));
        replace_uses_with(inst, result.into_value());
        return false;
    }
    if lhs_immediate && !rhs_immediate {
        let rhs_inst = rhs.inst();
        let combinable =
            rhs_inst.borrow().get_opcode() == opcode && rhs_inst.borrow().arg(1).is_immediate();
        if combinable {
            let rhs_arg = rhs_inst.borrow().arg(1);
            let combined = imm_fn(T::read(&lhs), T::read(&rhs_arg));
            let rhs_arg_0 = rhs_inst.borrow().arg(0);
            set_arg(inst, 0, rhs_arg_0);
            set_arg(inst, 1, combined.into_value());
        } else {
            set_arg(inst, 0, rhs.clone());
            set_arg(inst, 1, lhs.clone());
        }
    }
    if !lhs_immediate && rhs_immediate {
        let lhs_inst = lhs.inst();
        if lhs_inst.borrow().get_opcode() == opcode && lhs_inst.borrow().arg(1).is_immediate() {
            let lhs_arg = lhs_inst.borrow().arg(1);
            let combined = imm_fn(T::read(&rhs), T::read(&lhs_arg));
            let lhs_arg_0 = lhs_inst.borrow().arg(0);
            set_arg(inst, 0, lhs_arg_0);
            set_arg(inst, 1, combined.into_value());
        }
    }
    true
}

/// `FoldBitCast`.
fn fold_bit_cast(inst: &InstRef, reverse: Opcode, fold: impl Fn(&Value) -> Value) {
    let value = inst.borrow().arg(0);
    if value.is_immediate() {
        replace_uses_with(inst, fold(&value));
        return;
    }
    let arg_inst = value.inst();
    if arg_inst.borrow().get_opcode() == reverse {
        let arg = arg_inst.borrow().arg(0);
        replace_uses_with(inst, arg);
    }
}

/// `FoldCompositeExtractImpl`.
fn fold_composite_extract_impl(
    inst_value: Value,
    insert: Opcode,
    construct: Opcode,
    first_index: u32,
) -> Option<Value> {
    let inst = inst_value.inst();
    let opcode = inst.borrow().get_opcode();
    if opcode == construct {
        let element = inst.borrow().arg(first_index as usize);
        return Some(element);
    }
    if opcode != insert {
        return None;
    }
    let value_index = inst.borrow().arg(2);
    if !value_index.is_immediate() {
        return None;
    }
    let second_index = value_index.u32();
    if first_index != second_index {
        let value_composite = inst.borrow().arg(0);
        if value_composite.is_immediate() {
            return None;
        }
        return fold_composite_extract_impl(value_composite, insert, construct, first_index);
    }
    let element = inst.borrow().arg(1);
    Some(element)
}

/// `FoldCompositeExtract`.
fn fold_composite_extract(inst: &InstRef, construct: Opcode, insert: Opcode) {
    let value_1 = inst.borrow().arg(0);
    let value_2 = inst.borrow().arg(1);
    if value_1.is_immediate() {
        return;
    }
    if !value_2.is_immediate() {
        return;
    }
    let first_index = value_2.u32();
    if let Some(result) = fold_composite_extract_impl(value_1, insert, construct, first_index) {
        replace_uses_with(inst, result);
    }
}

/// `FoldCompositeConstruct`.
fn fold_composite_construct(inst: &InstRef, extract: Opcode) {
    let mut result = Value::empty();
    for (index, value) in arguments(inst).into_iter().enumerate() {
        if value.is_immediate() {
            return;
        }
        let producer = value.inst();
        if producer.borrow().get_opcode() != extract {
            return;
        }
        if producer.borrow().arg(1).u32() != index as u32 {
            return;
        }
        let element = producer.borrow().arg(0);
        if result.is_empty() {
            result = element;
        } else if result != element {
            return;
        }
    }
    replace_uses_with(inst, result);
}

/// `FoldConvert`.
fn fold_convert(inst: &InstRef, opposite: Opcode) {
    let value = inst.borrow().arg(0);
    if value.is_immediate() {
        return;
    }
    let producer = value.inst();
    if producer.borrow().get_opcode() == opposite {
        let arg = producer.borrow().arg(0);
        replace_uses_with(inst, arg);
    }
}

fn fold_logical_and(inst: &InstRef) {
    if !fold_commutative(inst, |a: bool, b: bool| a && b) {
        return;
    }
    let rhs = inst.borrow().arg(1);
    if rhs.is_immediate() {
        if rhs.u1() {
            let lhs = inst.borrow().arg(0);
            replace_uses_with(inst, lhs);
        } else {
            replace_uses_with(inst, Value::from_u1(false));
        }
    }
}

fn fold_select(inst: &InstRef) {
    let cond = inst.borrow().arg(0);
    if cond.is_immediate() {
        let replacement = if cond.u1() {
            inst.borrow().arg(1)
        } else {
            inst.borrow().arg(2)
        };
        replace_uses_with(inst, replacement);
        return;
    }
    if inst.borrow().get_opcode() == Opcode::SelectU1 {
        let arg_1 = inst.borrow().arg(1);
        let arg_2 = inst.borrow().arg(2);
        if arg_1.is_immediate() && arg_1.u1() && arg_2.is_immediate() && !arg_2.u1() {
            replace_uses_with(inst, cond);
        }
    }
}

fn fold_logical_or(inst: &InstRef) {
    if !fold_commutative(inst, |a: bool, b: bool| a || b) {
        return;
    }
    let rhs = inst.borrow().arg(1);
    if rhs.is_immediate() {
        if rhs.u1() {
            replace_uses_with(inst, Value::from_u1(true));
        } else {
            let lhs = inst.borrow().arg(0);
            replace_uses_with(inst, lhs);
        }
    }
}

fn fold_logical_not(inst: &InstRef) {
    let value = inst.borrow().arg(0);
    if value.is_immediate() {
        replace_uses_with(inst, Value::from_u1(!value.u1()));
        return;
    }
    let arg = value.inst();
    if arg.borrow().get_opcode() == Opcode::LogicalNot {
        let inner = arg.borrow().arg(0);
        replace_uses_with(inst, inner);
    }
}

fn fold_unpack32x2(block: &BlockRef, inst: &InstRef, index: usize, reverse: Opcode) {
    let value = inst.borrow().arg(0);
    if value.is_immediate() {
        let bits = value.u64();
        let mut ir = IrEmitter::at(block, index);
        let lo = ir.imm32(bits as u32);
        let hi = ir.imm32((bits >> 32) as u32);
        let composite = ir.composite_construct(lo, hi);
        replace_uses_with(inst, composite);
        return;
    }
    let arg_inst = value.inst();
    if arg_inst.borrow().get_opcode() == reverse {
        let arg = arg_inst.borrow().arg(0);
        replace_uses_with(inst, arg);
    }
}

fn fold_inverse_ballot(inst: &InstRef) {
    let value = inst.borrow().arg(0);
    if value.is_immediate() {
        if value.u64() == 0 {
            replace_uses_with(inst, Value::from_u1(false));
        } else if value.u64() == u64::MAX {
            replace_uses_with(inst, Value::from_u1(true));
        }
        return;
    }
    let arg_inst = value.inst();
    if arg_inst.borrow().get_opcode() == Opcode::Ballot {
        let arg = arg_inst.borrow().arg(0);
        replace_uses_with(inst, arg);
    }
}

fn fold_inverse_func(inst: &InstRef, reverse: Opcode) {
    let value = inst.borrow().arg(0);
    if value.is_immediate() {
        return;
    }
    let arg_inst = value.inst();
    if arg_inst.borrow().get_opcode() == reverse {
        let arg = arg_inst.borrow().arg(0);
        replace_uses_with(inst, arg);
    }
}

fn fold_discard_cond(inst: &InstRef) {
    let cond = inst.borrow().arg(0);
    if !cond.is_immediate() {
        return;
    }
    if cond.u1() {
        inst.borrow_mut().replace_opcode(Opcode::Discard);
    } else {
        invalidate(inst);
    }
}

fn fold_add(inst: &InstRef) {
    if !fold_commutative(inst, |a: u32, b: u32| a.wrapping_add(b)) {
        return;
    }
    let rhs = inst.borrow().arg(1);
    if rhs.is_immediate() && rhs.u32() == 0 {
        let lhs = inst.borrow().arg(0);
        replace_uses_with(inst, lhs);
    }
}

fn fold_mul(inst: &InstRef) {
    if !fold_commutative(inst, |a: u32, b: u32| a.wrapping_mul(b)) {
        return;
    }
    let rhs = inst.borrow().arg(1);
    if rhs.is_immediate() {
        if rhs.u32() == 0 {
            replace_uses_with(inst, Value::from_u32(0));
            return;
        }
        if rhs.u32() == 1 {
            let lhs = inst.borrow().arg(0);
            replace_uses_with(inst, lhs);
        }
    }
}

/// `ConstantPropagation`.
fn constant_propagation(block: &BlockRef, inst: &InstRef, index: usize) {
    let opcode = inst.borrow().get_opcode();
    match opcode {
        Opcode::IAdd32 => fold_add(inst),
        Opcode::ISub32 => {
            fold_when_all_immediates2(inst, |a: u32, b: u32| a.wrapping_sub(b));
        }
        Opcode::ConvertF32U32 => {
            fold_when_all_immediates1(inst, |a: u32| a as f32);
        }
        Opcode::IMul32 => fold_mul(inst),
        Opcode::UDiv32 => {
            fold_when_all_immediates2(inst, |a: u32, b: u32| {
                assert!(b != 0, "Folding UDiv32 with divisor 0");
                a / b
            });
        }
        Opcode::UMod32 => {
            fold_when_all_immediates2(inst, |a: u32, b: u32| {
                assert!(b != 0, "Folding UMod32 with modulo 0");
                a % b
            });
        }
        Opcode::ShiftLeftLogical32 => {
            fold_when_all_immediates2(inst, |a: u32, b: u32| a.wrapping_shl(b));
        }
        Opcode::ShiftRightLogical32 => {
            fold_when_all_immediates2(inst, |a: u32, b: u32| a.wrapping_shr(b));
        }
        Opcode::ShiftRightArithmetic32 => {
            fold_when_all_immediates2(inst, |a: i32, b: i32| a.wrapping_shr(b as u32) as u32);
        }
        Opcode::BitCastF32U32 => {
            fold_bit_cast(inst, Opcode::BitCastU32F32, |value| {
                Value::from_f32(f32::from_bits(value.u32()))
            });
        }
        Opcode::BitCastU32F32 => {
            fold_bit_cast(inst, Opcode::BitCastF32U32, |value| {
                Value::from_u32(value.f32().to_bits())
            });
        }
        Opcode::PackUint2x32 => fold_inverse_func(inst, Opcode::UnpackUint2x32),
        Opcode::UnpackUint2x32 => {
            fold_unpack32x2(block, inst, index, Opcode::PackUint2x32);
        }
        Opcode::PackDouble2x32 => fold_inverse_func(inst, Opcode::UnpackDouble2x32),
        Opcode::UnpackDouble2x32 => {
            fold_unpack32x2(block, inst, index, Opcode::PackDouble2x32);
        }
        Opcode::InverseBallot => fold_inverse_ballot(inst),
        Opcode::PackUnorm2x16 => fold_inverse_func(inst, Opcode::UnpackUnorm2x16),
        Opcode::UnpackUnorm2x16 => fold_inverse_func(inst, Opcode::PackUnorm2x16),
        Opcode::PackSnorm2x16 => fold_inverse_func(inst, Opcode::UnpackSnorm2x16),
        Opcode::UnpackSnorm2x16 => fold_inverse_func(inst, Opcode::PackSnorm2x16),
        Opcode::PackUint2x16 => fold_inverse_func(inst, Opcode::UnpackUint2x16),
        Opcode::UnpackUint2x16 => fold_inverse_func(inst, Opcode::PackUint2x16),
        Opcode::PackSint2x16 => fold_inverse_func(inst, Opcode::UnpackSint2x16),
        Opcode::UnpackSint2x16 => fold_inverse_func(inst, Opcode::PackSint2x16),
        Opcode::PackHalf2x16 => fold_inverse_func(inst, Opcode::UnpackHalf2x16),
        Opcode::UnpackHalf2x16 => fold_inverse_func(inst, Opcode::PackHalf2x16),
        Opcode::PackUnorm4x8 => fold_inverse_func(inst, Opcode::UnpackUnorm4x8),
        Opcode::UnpackUnorm4x8 => fold_inverse_func(inst, Opcode::PackUnorm4x8),
        Opcode::PackSnorm4x8 => fold_inverse_func(inst, Opcode::UnpackSnorm4x8),
        Opcode::UnpackSnorm4x8 => fold_inverse_func(inst, Opcode::PackSnorm4x8),
        Opcode::PackUint4x8 => fold_inverse_func(inst, Opcode::UnpackUint4x8),
        Opcode::UnpackUint4x8 => fold_inverse_func(inst, Opcode::PackUint4x8),
        Opcode::PackSint4x8 => fold_inverse_func(inst, Opcode::UnpackSint4x8),
        Opcode::UnpackSint4x8 => fold_inverse_func(inst, Opcode::PackSint4x8),
        Opcode::PackUfloat10_11_11 => {
            fold_inverse_func(inst, Opcode::UnpackUfloat10_11_11);
        }
        Opcode::UnpackUfloat10_11_11 => {
            fold_inverse_func(inst, Opcode::PackUfloat10_11_11);
        }
        Opcode::PackUnorm2_10_10_10 => {
            fold_inverse_func(inst, Opcode::UnpackUnorm2_10_10_10);
        }
        Opcode::UnpackUnorm2_10_10_10 => {
            fold_inverse_func(inst, Opcode::PackUnorm2_10_10_10);
        }
        Opcode::PackSnorm2_10_10_10 => {
            fold_inverse_func(inst, Opcode::UnpackSnorm2_10_10_10);
        }
        Opcode::UnpackSnorm2_10_10_10 => {
            fold_inverse_func(inst, Opcode::PackSnorm2_10_10_10);
        }
        Opcode::PackUint2_10_10_10 => {
            fold_inverse_func(inst, Opcode::UnpackUint2_10_10_10);
        }
        Opcode::UnpackUint2_10_10_10 => {
            fold_inverse_func(inst, Opcode::PackUint2_10_10_10);
        }
        Opcode::PackSint2_10_10_10 => {
            fold_inverse_func(inst, Opcode::UnpackSint2_10_10_10);
        }
        Opcode::UnpackSint2_10_10_10 => {
            fold_inverse_func(inst, Opcode::PackSint2_10_10_10);
        }
        Opcode::SelectU1 | Opcode::SelectU32 | Opcode::SelectF32 => fold_select(inst),
        Opcode::FPNeg32 => {
            fold_when_all_immediates1(inst, |a: f32| -a);
        }
        Opcode::LogicalAnd => fold_logical_and(inst),
        Opcode::LogicalOr => fold_logical_or(inst),
        Opcode::LogicalNot => fold_logical_not(inst),
        Opcode::UMin32 => {
            fold_when_all_immediates2(inst, |a: u32, b: u32| a.min(b));
        }
        Opcode::UMax32 => {
            fold_when_all_immediates2(inst, |a: u32, b: u32| a.max(b));
        }
        Opcode::SLessThan32 => {
            fold_when_all_immediates2(inst, |a: i32, b: i32| a < b);
        }
        Opcode::SLessThan64 => {
            fold_when_all_immediates2(inst, |a: i64, b: i64| a < b);
        }
        Opcode::ULessThan32 => {
            fold_when_all_immediates2(inst, |a: u32, b: u32| a < b);
        }
        Opcode::ULessThan64 => {
            fold_when_all_immediates2(inst, |a: u64, b: u64| a < b);
        }
        Opcode::SLessThanEqual32 => {
            fold_when_all_immediates2(inst, |a: i32, b: i32| a <= b);
        }
        Opcode::SLessThanEqual64 => {
            fold_when_all_immediates2(inst, |a: i64, b: i64| a <= b);
        }
        Opcode::ULessThanEqual32 => {
            fold_when_all_immediates2(inst, |a: u32, b: u32| a <= b);
        }
        Opcode::ULessThanEqual64 => {
            fold_when_all_immediates2(inst, |a: u64, b: u64| a <= b);
        }
        Opcode::SGreaterThan32 => {
            fold_when_all_immediates2(inst, |a: i32, b: i32| a > b);
        }
        Opcode::SGreaterThan64 => {
            fold_when_all_immediates2(inst, |a: i64, b: i64| a > b);
        }
        Opcode::UGreaterThan32 => {
            fold_when_all_immediates2(inst, |a: u32, b: u32| a > b);
        }
        Opcode::UGreaterThan64 => {
            fold_when_all_immediates2(inst, |a: u64, b: u64| a > b);
        }
        Opcode::SGreaterThanEqual32 => {
            fold_when_all_immediates2(inst, |a: i32, b: i32| a >= b);
        }
        Opcode::SGreaterThanEqual64 => {
            fold_when_all_immediates2(inst, |a: i64, b: i64| a >= b);
        }
        Opcode::UGreaterThanEqual32 => {
            fold_when_all_immediates2(inst, |a: u32, b: u32| a >= b);
        }
        Opcode::UGreaterThanEqual64 => {
            fold_when_all_immediates2(inst, |a: u64, b: u64| a >= b);
        }
        Opcode::IEqual32 => {
            fold_when_all_immediates2(inst, |a: u32, b: u32| a == b);
        }
        Opcode::IEqual64 => {
            fold_when_all_immediates2(inst, |a: u64, b: u64| a == b);
        }
        Opcode::INotEqual32 => {
            fold_when_all_immediates2(inst, |a: u32, b: u32| a != b);
        }
        Opcode::INotEqual64 => {
            fold_when_all_immediates2(inst, |a: u64, b: u64| a != b);
        }
        Opcode::BitwiseAnd32 => {
            fold_when_all_immediates2(inst, |a: u32, b: u32| a & b);
        }
        Opcode::BitwiseAnd64 => {
            fold_when_all_immediates2(inst, |a: u64, b: u64| a & b);
        }
        Opcode::BitwiseOr32 => {
            fold_when_all_immediates2(inst, |a: u32, b: u32| a | b);
        }
        Opcode::BitwiseOr64 => {
            fold_when_all_immediates2(inst, |a: u64, b: u64| a | b);
        }
        Opcode::BitwiseXor32 => {
            fold_when_all_immediates2(inst, |a: u32, b: u32| a ^ b);
        }
        Opcode::BitwiseNot32 => {
            fold_when_all_immediates1(inst, |a: u32| !a);
        }
        Opcode::BitReverse32 => {
            fold_when_all_immediates1(inst, |a: u32| a.reverse_bits());
        }
        Opcode::BitCount32 => {
            fold_when_all_immediates1(inst, |a: u32| a.count_ones());
        }
        Opcode::BitCount64 => {
            fold_when_all_immediates1(inst, |a: u64| a.count_ones());
        }
        Opcode::BitFieldUExtract => {
            fold_when_all_immediates3(inst, |base: u32, shift: u32, count: u32| {
                if shift as usize + count as usize > 32 {
                    panic!(
                        "Undefined result in {}({}, {}, {})",
                        Opcode::BitFieldUExtract.name(),
                        base,
                        shift,
                        count
                    );
                }
                base.wrapping_shr(shift) & 1u32.wrapping_shl(count).wrapping_sub(1)
            });
        }
        Opcode::BitFieldSExtract => {
            fold_when_all_immediates3(inst, |base: i32, shift: u32, count: u32| {
                let back_shift = shift as usize + count as usize;
                let left_shift = 32usize.wrapping_sub(back_shift);
                let right_shift = 32usize.wrapping_sub(count as usize);
                if back_shift > 32 || left_shift >= 32 || right_shift >= 32 {
                    panic!(
                        "Undefined result in {}({}, {}, {})",
                        Opcode::BitFieldSExtract.name(),
                        base,
                        shift,
                        count
                    );
                }
                base.wrapping_shl(left_shift as u32)
                    .wrapping_shr(right_shift as u32) as u32
            });
        }
        Opcode::BitFieldInsert32 => {
            fold_when_all_immediates4(inst, |base: u32, insert: u32, offset: u32, bits: u32| {
                if bits >= 32 || offset >= 32 {
                    panic!(
                        "Undefined result in {}({}, {}, {}, {})",
                        Opcode::BitFieldInsert32.name(),
                        base,
                        insert,
                        offset,
                        bits
                    );
                }
                let mask = !(!u32::MAX.wrapping_shl(bits)).wrapping_shl(offset);
                (base & mask) | insert.wrapping_shl(offset)
            });
        }
        Opcode::BitFieldInsert64 => {
            fold_when_all_immediates4(inst, |base: u64, insert: u64, offset: u32, bits: u32| {
                if bits >= 64 || offset >= 64 {
                    panic!(
                        "Undefined result in {}({}, {}, {}, {})",
                        Opcode::BitFieldInsert64.name(),
                        base,
                        insert,
                        offset,
                        bits
                    );
                }
                let mask = !(!u64::MAX.wrapping_shl(bits)).wrapping_shl(offset);
                (base & mask) | insert.wrapping_shl(offset)
            });
        }
        Opcode::CompositeExtractU32x2 => fold_composite_extract(
            inst,
            Opcode::CompositeConstructU32x2,
            Opcode::CompositeInsertU32x2,
        ),
        Opcode::CompositeExtractU32x3 => fold_composite_extract(
            inst,
            Opcode::CompositeConstructU32x3,
            Opcode::CompositeInsertU32x3,
        ),
        Opcode::CompositeExtractU32x4 => fold_composite_extract(
            inst,
            Opcode::CompositeConstructU32x4,
            Opcode::CompositeInsertU32x4,
        ),
        Opcode::CompositeExtractF32x2 => fold_composite_extract(
            inst,
            Opcode::CompositeConstructF32x2,
            Opcode::CompositeInsertF32x2,
        ),
        Opcode::CompositeExtractF32x3 => fold_composite_extract(
            inst,
            Opcode::CompositeConstructF32x3,
            Opcode::CompositeInsertF32x3,
        ),
        Opcode::CompositeExtractF32x4 => fold_composite_extract(
            inst,
            Opcode::CompositeConstructF32x4,
            Opcode::CompositeInsertF32x4,
        ),
        Opcode::CompositeConstructU32x2 => {
            fold_composite_construct(inst, Opcode::CompositeExtractU32x2);
        }
        Opcode::CompositeConstructU32x3 => {
            fold_composite_construct(inst, Opcode::CompositeExtractU32x3);
        }
        Opcode::CompositeConstructU32x4 => {
            fold_composite_construct(inst, Opcode::CompositeExtractU32x4);
        }
        Opcode::ConvertF32F16 => fold_convert(inst, Opcode::ConvertF16F32),
        Opcode::ConvertF16F32 => fold_convert(inst, Opcode::ConvertF32F16),
        Opcode::DiscardCond => fold_discard_cond(inst),
        _ => {}
    }
}

/// `ConstantPropagationPass`: blocks are visited in reverse and each block forwards. The inner
/// loop steps over instructions a fold inserted before the current one, since the C++'s forward
/// iterator has already passed that position.
pub fn constant_propagation_pass(program: &BlockList) {
    for block in program.iter().rev() {
        let mut index = 0;
        while index < block.borrow().instructions.len() {
            let inst = block.borrow().instructions[index].clone();
            let before = block.borrow().instructions.len();
            constant_propagation(block, &inst, index);
            let inserted = block.borrow().instructions.len() - before;
            index += inserted + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block};
    use std::panic::AssertUnwindSafe;
    use std::rc::Rc;

    fn append(block: &BlockRef, op: Opcode, args: &[Value]) -> InstRef {
        let index = append_new_inst(block, op, args);
        block.borrow().instructions[index].clone()
    }

    /// An instruction that consumes a value but is itself never folded.
    fn sink(block: &BlockRef, value: &InstRef) -> InstRef {
        append(
            block,
            Opcode::ConvertF16F32,
            &[Value::from_inst(value.clone())],
        )
    }

    fn run(block: &BlockRef) {
        constant_propagation_pass(&vec![block.clone()]);
    }

    fn sink_value(sink: &InstRef) -> Value {
        sink.borrow().arg(0)
    }

    fn sink_error(block: &BlockRef) -> Option<String> {
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| run(block)));
        let payload = result.err()?;
        if let Some(text) = payload.downcast_ref::<&str>() {
            return Some((*text).to_string());
        }
        payload.downcast_ref::<String>().cloned()
    }

    #[test]
    fn arithmetic_folds_immediates_and_simplifies() {
        let block = Block::new_ref();
        let x = append(&block, Opcode::GetScc, &[]);

        let add = append(
            &block,
            Opcode::IAdd32,
            &[Value::from_u32(2), Value::from_u32(3)],
        );
        let add_sink = sink(&block, &add);
        let sub = append(
            &block,
            Opcode::ISub32,
            &[Value::from_u32(1), Value::from_u32(3)],
        );
        let sub_sink = sink(&block, &sub);
        let mul_zero = append(
            &block,
            Opcode::IMul32,
            &[Value::from_inst(x.clone()), Value::from_u32(0)],
        );
        let mul_zero_sink = sink(&block, &mul_zero);
        let mul_one = append(
            &block,
            Opcode::IMul32,
            &[Value::from_inst(x.clone()), Value::from_u32(1)],
        );
        let mul_one_sink = sink(&block, &mul_one);

        run(&block);

        assert_eq!(add.borrow().get_opcode(), Opcode::Void);
        assert_eq!(sink_value(&add_sink).u32(), 5);
        assert_eq!(sink_value(&sub_sink).u32(), 1u32.wrapping_sub(3));
        assert_eq!(sink_value(&mul_zero_sink).u32(), 0);
        assert!(Rc::ptr_eq(&sink_value(&mul_one_sink).inst(), &x));
    }

    #[test]
    fn additions_normalize_and_reassociate() {
        let block = Block::new_ref();
        let x = append(&block, Opcode::GetScc, &[]);
        let y = append(&block, Opcode::GetScc, &[]);

        // A left immediate with a non-add right side swaps its operands.
        let normalize = append(
            &block,
            Opcode::IAdd32,
            &[Value::from_u32(9), Value::from_inst(x.clone())],
        );
        // The same opcode on the right, but its second operand is not immediate: still swaps.
        let non_immediate_right = append(
            &block,
            Opcode::IAdd32,
            &[Value::from_inst(x.clone()), Value::from_inst(y.clone())],
        );
        let swap_same_opcode = append(
            &block,
            Opcode::IAdd32,
            &[
                Value::from_u32(5),
                Value::from_inst(non_immediate_right.clone()),
            ],
        );
        // An add of an add folds the two immediates together.
        let inner = append(
            &block,
            Opcode::IAdd32,
            &[Value::from_inst(x.clone()), Value::from_u32(3)],
        );
        let outer = append(
            &block,
            Opcode::IAdd32,
            &[Value::from_inst(inner), Value::from_u32(4)],
        );

        run(&block);

        assert!(Rc::ptr_eq(&normalize.borrow().arg(0).inst(), &x));
        assert_eq!(normalize.borrow().arg(1).u32(), 9);
        assert!(Rc::ptr_eq(
            &swap_same_opcode.borrow().arg(0).inst(),
            &non_immediate_right
        ));
        assert_eq!(swap_same_opcode.borrow().arg(1).u32(), 5);
        assert!(Rc::ptr_eq(&outer.borrow().arg(0).inst(), &x));
        assert_eq!(outer.borrow().arg(1).u32(), 7);
    }

    #[test]
    fn typed_folds_produce_typed_immediates() {
        let block = Block::new_ref();
        let convert = append(&block, Opcode::ConvertF32U32, &[Value::from_u32(3)]);
        let convert_sink = sink(&block, &convert);
        let negate = append(&block, Opcode::FPNeg32, &[Value::from_f32(1.5)]);
        let negate_sink = sink(&block, &negate);
        let shift_left = append(
            &block,
            Opcode::ShiftLeftLogical32,
            &[Value::from_u32(1), Value::from_u32(31)],
        );
        let shift_left_sink = sink(&block, &shift_left);
        let shift_arithmetic = append(
            &block,
            Opcode::ShiftRightArithmetic32,
            &[Value::from_u32(0x8000_0000), Value::from_u32(4)],
        );
        let shift_arithmetic_sink = sink(&block, &shift_arithmetic);

        run(&block);

        assert_eq!(sink_value(&convert_sink).f32(), 3.0);
        assert_eq!(sink_value(&negate_sink).f32(), -1.5);
        assert_eq!(sink_value(&shift_left_sink).u32(), 0x8000_0000);
        assert_eq!(sink_value(&shift_arithmetic_sink).u32(), 0xF800_0000);
    }

    #[test]
    fn bit_casts_fold_and_round_trip() {
        let block = Block::new_ref();
        let to_float = append(
            &block,
            Opcode::BitCastF32U32,
            &[Value::from_u32(0x3F80_0000)],
        );
        let to_float_sink = sink(&block, &to_float);
        let to_bits = append(&block, Opcode::BitCastU32F32, &[Value::from_f32(1.0)]);
        let to_bits_sink = sink(&block, &to_bits);
        let x = append(&block, Opcode::GetScc, &[]);
        let inner = append(
            &block,
            Opcode::BitCastU32F32,
            &[Value::from_inst(x.clone())],
        );
        let round_trip = append(
            &block,
            Opcode::BitCastF32U32,
            &[Value::from_inst(inner.clone())],
        );
        let round_trip_sink = sink(&block, &round_trip);

        run(&block);

        assert_eq!(sink_value(&to_float_sink).f32(), 1.0);
        assert_eq!(sink_value(&to_bits_sink).u32(), 0x3F80_0000);
        assert_eq!(inner.borrow().get_opcode(), Opcode::BitCastU32F32);
        assert!(Rc::ptr_eq(&sink_value(&round_trip_sink).inst(), &x));
    }

    #[test]
    fn composite_extract_and_construct_fold() {
        let block = Block::new_ref();
        let construct = append(
            &block,
            Opcode::CompositeConstructU32x2,
            &[Value::from_u32(1), Value::from_u32(2)],
        );
        let extract = append(
            &block,
            Opcode::CompositeExtractU32x2,
            &[Value::from_inst(construct), Value::from_u32(1)],
        );
        let extract_sink = sink(&block, &extract);

        let base = append(&block, Opcode::GetScc, &[]);
        let insert = append(
            &block,
            Opcode::CompositeInsertU32x2,
            &[
                Value::from_inst(base),
                Value::from_u32(9),
                Value::from_u32(0),
            ],
        );
        let insert_extract = append(
            &block,
            Opcode::CompositeExtractU32x2,
            &[Value::from_inst(insert), Value::from_u32(0)],
        );
        let insert_sink = sink(&block, &insert_extract);

        let x = append(&block, Opcode::GetScc, &[]);
        let e0 = append(
            &block,
            Opcode::CompositeExtractU32x2,
            &[Value::from_inst(x.clone()), Value::from_u32(0)],
        );
        let e1 = append(
            &block,
            Opcode::CompositeExtractU32x2,
            &[Value::from_inst(x.clone()), Value::from_u32(1)],
        );
        let rebuilt = append(
            &block,
            Opcode::CompositeConstructU32x2,
            &[Value::from_inst(e0), Value::from_inst(e1)],
        );
        let rebuilt_sink = sink(&block, &rebuilt);

        run(&block);

        assert_eq!(sink_value(&extract_sink).u32(), 2);
        assert_eq!(sink_value(&insert_sink).u32(), 9);
        assert!(Rc::ptr_eq(&sink_value(&rebuilt_sink).inst(), &x));
    }

    #[test]
    fn selects_and_logical_operations_fold() {
        let block = Block::new_ref();
        let x = append(&block, Opcode::GetScc, &[]);

        let select = append(
            &block,
            Opcode::SelectU32,
            &[Value::from_u1(true), Value::from_u32(7), Value::from_u32(9)],
        );
        let select_sink = sink(&block, &select);
        let select_u1 = append(
            &block,
            Opcode::SelectU1,
            &[
                Value::from_inst(x.clone()),
                Value::from_u1(true),
                Value::from_u1(false),
            ],
        );
        let select_u1_sink = sink(&block, &select_u1);
        let and_true = append(
            &block,
            Opcode::LogicalAnd,
            &[Value::from_inst(x.clone()), Value::from_u1(true)],
        );
        let and_true_sink = sink(&block, &and_true);
        let and_false = append(
            &block,
            Opcode::LogicalAnd,
            &[Value::from_inst(x.clone()), Value::from_u1(false)],
        );
        let and_false_sink = sink(&block, &and_false);
        let or_false = append(
            &block,
            Opcode::LogicalOr,
            &[Value::from_inst(x.clone()), Value::from_u1(false)],
        );
        let or_false_sink = sink(&block, &or_false);
        let or_true = append(
            &block,
            Opcode::LogicalOr,
            &[Value::from_inst(x.clone()), Value::from_u1(true)],
        );
        let or_true_sink = sink(&block, &or_true);
        let not_x = append(&block, Opcode::LogicalNot, &[Value::from_inst(x.clone())]);
        let double_not = append(&block, Opcode::LogicalNot, &[Value::from_inst(not_x)]);
        let double_not_sink = sink(&block, &double_not);
        let not_true = append(&block, Opcode::LogicalNot, &[Value::from_u1(true)]);
        let not_true_sink = sink(&block, &not_true);

        run(&block);

        assert_eq!(sink_value(&select_sink).u32(), 7);
        assert!(Rc::ptr_eq(&sink_value(&select_u1_sink).inst(), &x));
        assert!(Rc::ptr_eq(&sink_value(&and_true_sink).inst(), &x));
        assert!(!sink_value(&and_false_sink).u1());
        assert!(Rc::ptr_eq(&sink_value(&or_false_sink).inst(), &x));
        assert!(sink_value(&or_true_sink).u1());
        assert!(Rc::ptr_eq(&sink_value(&double_not_sink).inst(), &x));
        assert!(!sink_value(&not_true_sink).u1());
    }

    #[test]
    fn ballots_and_discards_fold() {
        let block = Block::new_ref();
        let zero = append(&block, Opcode::InverseBallot, &[Value::from_u64(0)]);
        let zero_sink = sink(&block, &zero);
        let all = append(&block, Opcode::InverseBallot, &[Value::from_u64(u64::MAX)]);
        let all_sink = sink(&block, &all);
        let x = append(&block, Opcode::GetScc, &[]);
        let ballot = append(&block, Opcode::Ballot, &[Value::from_inst(x.clone())]);
        let inverse = append(&block, Opcode::InverseBallot, &[Value::from_inst(ballot)]);
        let inverse_sink = sink(&block, &inverse);
        let discard_true = append(&block, Opcode::DiscardCond, &[Value::from_u1(true)]);
        let discard_false = append(&block, Opcode::DiscardCond, &[Value::from_u1(false)]);

        run(&block);

        assert!(!sink_value(&zero_sink).u1());
        assert!(sink_value(&all_sink).u1());
        assert!(Rc::ptr_eq(&sink_value(&inverse_sink).inst(), &x));
        assert_eq!(discard_true.borrow().get_opcode(), Opcode::Discard);
        assert_eq!(discard_false.borrow().get_opcode(), Opcode::Void);
    }

    #[test]
    fn bit_operations_fold() {
        let block = Block::new_ref();
        let cases: [(Opcode, [Value; 3], usize); 8] = [
            (
                Opcode::BitReverse32,
                [Value::from_u32(1), Value::from_u32(0), Value::from_u32(0)],
                1,
            ),
            (
                Opcode::BitCount32,
                [
                    Value::from_u32(0xF0F0),
                    Value::from_u32(0),
                    Value::from_u32(0),
                ],
                1,
            ),
            (
                Opcode::BitCount64,
                [
                    Value::from_u64(0xFFFF_FFFF_0000_0000),
                    Value::from_u32(0),
                    Value::from_u32(0),
                ],
                1,
            ),
            (
                Opcode::BitFieldUExtract,
                [
                    Value::from_u32(0xABCD),
                    Value::from_u32(4),
                    Value::from_u32(8),
                ],
                3,
            ),
            (
                Opcode::BitFieldSExtract,
                [
                    Value::from_u32(0x80),
                    Value::from_u32(0),
                    Value::from_u32(8),
                ],
                3,
            ),
            (
                Opcode::BitwiseAnd32,
                [
                    Value::from_u32(0xF0),
                    Value::from_u32(0x0F),
                    Value::from_u32(0),
                ],
                2,
            ),
            (
                Opcode::BitwiseOr64,
                [Value::from_u64(1), Value::from_u64(2), Value::from_u32(0)],
                2,
            ),
            (
                Opcode::BitwiseNot32,
                [Value::from_u32(0), Value::from_u32(0), Value::from_u32(0)],
                1,
            ),
        ];
        let mut sinks = Vec::new();
        for (op, args, arity) in cases {
            let inst = append(&block, op, &args[..arity]);
            sinks.push((op, sink(&block, &inst)));
        }
        // BitFieldInsert32 takes a fourth argument.
        let insert32 = append(
            &block,
            Opcode::BitFieldInsert32,
            &[
                Value::from_u32(0),
                Value::from_u32(0xFF),
                Value::from_u32(8),
                Value::from_u32(8),
            ],
        );
        let insert32_sink = sink(&block, &insert32);
        let insert64 = append(
            &block,
            Opcode::BitFieldInsert64,
            &[
                Value::from_u64(0),
                Value::from_u64(0xFFFF),
                Value::from_u32(16),
                Value::from_u32(16),
            ],
        );
        let insert64_sink = sink(&block, &insert64);

        run(&block);

        let value = |op: Opcode| -> Value {
            let (_, sink) = sinks
                .iter()
                .find(|(candidate, _)| *candidate == op)
                .expect("the case is present");
            sink_value(sink)
        };
        assert_eq!(value(Opcode::BitReverse32).u32(), 0x8000_0000);
        assert_eq!(value(Opcode::BitCount32).u32(), 8);
        assert_eq!(value(Opcode::BitCount64).u32(), 32);
        assert_eq!(value(Opcode::BitFieldUExtract).u32(), 0xBC);
        assert_eq!(value(Opcode::BitFieldSExtract).u32(), 0xFFFF_FF80);
        assert_eq!(value(Opcode::BitwiseAnd32).u32(), 0);
        assert_eq!(value(Opcode::BitwiseOr64).u64(), 3);
        assert_eq!(value(Opcode::BitwiseNot32).u32(), 0xFFFF_FFFF);
        assert_eq!(sink_value(&insert32_sink).u32(), 0xFF00);
        assert_eq!(sink_value(&insert64_sink).u64(), 0xFFFF_0000);
    }

    #[test]
    fn unpacking_an_immediate_emits_a_composite() {
        let block = Block::new_ref();
        let bits = 0x0000_0002_0000_0001u64;
        let unpack = append(&block, Opcode::UnpackUint2x32, &[Value::from_u64(bits)]);
        let unpack_sink = sink(&block, &unpack);

        run(&block);

        assert_eq!(unpack.borrow().get_opcode(), Opcode::Void);
        let instructions = block.borrow().instructions.clone();
        assert_eq!(instructions.len(), 3);
        let composite = &instructions[0];
        assert_eq!(
            composite.borrow().get_opcode(),
            Opcode::CompositeConstructU32x2
        );
        assert_eq!(composite.borrow().arg(0).u32(), 1);
        assert_eq!(composite.borrow().arg(1).u32(), 2);
        assert!(Rc::ptr_eq(&sink_value(&unpack_sink).inst(), composite));
    }

    #[test]
    fn packing_an_unpack_round_trips() {
        let block = Block::new_ref();
        let x = append(&block, Opcode::GetScc, &[]);
        let unpack = append(
            &block,
            Opcode::UnpackUint2x32,
            &[Value::from_inst(x.clone())],
        );
        let pack = append(
            &block,
            Opcode::PackUint2x32,
            &[Value::from_inst(unpack.clone())],
        );
        let pack_sink = sink(&block, &pack);

        run(&block);

        assert_eq!(unpack.borrow().get_opcode(), Opcode::UnpackUint2x32);
        assert_eq!(pack.borrow().get_opcode(), Opcode::Void);
        assert!(Rc::ptr_eq(&sink_value(&pack_sink).inst(), &x));
    }

    #[test]
    fn dividing_by_zero_is_refused() {
        let block = Block::new_ref();
        append(
            &block,
            Opcode::UDiv32,
            &[Value::from_u32(1), Value::from_u32(0)],
        );
        assert_eq!(
            sink_error(&block).as_deref(),
            Some("Folding UDiv32 with divisor 0")
        );

        let block = Block::new_ref();
        append(
            &block,
            Opcode::UMod32,
            &[Value::from_u32(1), Value::from_u32(0)],
        );
        assert_eq!(
            sink_error(&block).as_deref(),
            Some("Folding UMod32 with modulo 0")
        );
    }

    #[test]
    fn undefined_bitfield_arguments_are_refused() {
        let block = Block::new_ref();
        append(
            &block,
            Opcode::BitFieldUExtract,
            &[Value::from_u32(0), Value::from_u32(31), Value::from_u32(2)],
        );
        assert_eq!(
            sink_error(&block).as_deref(),
            Some("Undefined result in BitFieldUExtract(0, 31, 2)")
        );

        let block = Block::new_ref();
        append(
            &block,
            Opcode::BitFieldInsert32,
            &[
                Value::from_u32(0),
                Value::from_u32(0),
                Value::from_u32(0),
                Value::from_u32(32),
            ],
        );
        assert_eq!(
            sink_error(&block).as_deref(),
            Some("Undefined result in BitFieldInsert32(0, 0, 0, 32)")
        );
    }
}
