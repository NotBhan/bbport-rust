//! Atomic emitters, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv_atomic.cpp`.
//!
//! The C++ passes pointers to Sirit member functions; the port selects the same instruction
//! with an enum and one dispatch helper.

use super::builder::Id;
use super::emit_context::{EmitContext, PointerSize, PointerType};
use super::spv;
use crate::gpu::shader_recompiler::ir::microinstruction::InstRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AtomicValueOp {
    IAdd,
    ISub,
    SMin,
    UMin,
    SMax,
    UMax,
    And,
    Or,
    Xor,
    Exchange,
    FMin,
    FMax,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AtomicIncDec {
    Inc,
    Dec,
}

fn atomic_args(ctx: &mut EmitContext) -> (Id, Id) {
    let types = ctx.type_ids();
    let scope = ctx.ConstU32(spv::Scope::Device);
    (scope, types.u32_zero_value)
}

fn apply_atomic(
    ctx: &mut EmitContext,
    ty: Id,
    pointer: Id,
    scope: Id,
    semantics: Id,
    value: Id,
    op: AtomicValueOp,
) -> Id {
    match op {
        AtomicValueOp::IAdd => ctx.OpAtomicIAdd(ty, pointer, scope, semantics, value),
        AtomicValueOp::ISub => ctx.OpAtomicISub(ty, pointer, scope, semantics, value),
        AtomicValueOp::SMin => ctx.OpAtomicSMin(ty, pointer, scope, semantics, value),
        AtomicValueOp::UMin => ctx.OpAtomicUMin(ty, pointer, scope, semantics, value),
        AtomicValueOp::SMax => ctx.OpAtomicSMax(ty, pointer, scope, semantics, value),
        AtomicValueOp::UMax => ctx.OpAtomicUMax(ty, pointer, scope, semantics, value),
        AtomicValueOp::And => ctx.OpAtomicAnd(ty, pointer, scope, semantics, value),
        AtomicValueOp::Or => ctx.OpAtomicOr(ty, pointer, scope, semantics, value),
        AtomicValueOp::Xor => ctx.OpAtomicXor(ty, pointer, scope, semantics, value),
        AtomicValueOp::Exchange => ctx.OpAtomicExchange(ty, pointer, scope, semantics, value),
        AtomicValueOp::FMin => ctx.OpAtomicFMin(ty, pointer, scope, semantics, value),
        AtomicValueOp::FMax => ctx.OpAtomicFMax(ty, pointer, scope, semantics, value),
    }
}

fn shared_atomic_u32(ctx: &mut EmitContext, offset: Id, value: Id, op: AtomicValueOp) -> Id {
    let types = ctx.type_ids();
    let shift = ctx.ConstU32(2);
    let index = ctx.OpShiftRightLogical(types.U32[1], offset, shift);
    let pointer = ctx.EmitSharedMemoryAccess(types.shared_u32, types.shared_memory_u32, index);
    let (scope, semantics) = atomic_args(ctx);
    apply_atomic(ctx, types.U32[1], pointer, scope, semantics, value, op)
}

fn shared_atomic_u64(ctx: &mut EmitContext, offset: Id, value: Id, op: AtomicValueOp) -> Id {
    let types = ctx.type_ids();
    let shift = ctx.ConstU32(3);
    let index = ctx.OpShiftRightLogical(types.U32[1], offset, shift);
    let pointer = ctx.EmitSharedMemoryAccess(types.shared_u64, types.shared_memory_u64, index);
    let (scope, semantics) = atomic_args(ctx);
    apply_atomic(ctx, types.U64, pointer, scope, semantics, value, op)
}

fn shared_atomic_u32_inc_dec(ctx: &mut EmitContext, offset: Id, op: AtomicIncDec) -> Id {
    let types = ctx.type_ids();
    let shift = ctx.ConstU32(2);
    let index = ctx.OpShiftRightLogical(types.U32[1], offset, shift);
    let pointer = ctx.EmitSharedMemoryAccess(types.shared_u32, types.shared_memory_u32, index);
    let (scope, semantics) = atomic_args(ctx);
    match op {
        AtomicIncDec::Inc => ctx.OpAtomicIIncrement(types.U32[1], pointer, scope, semantics),
        AtomicIncDec::Dec => ctx.OpAtomicIDecrement(types.U32[1], pointer, scope, semantics),
    }
}

fn shared_atomic_u64_inc_dec(ctx: &mut EmitContext, offset: Id, op: AtomicIncDec) -> Id {
    let types = ctx.type_ids();
    let shift = ctx.ConstU32(3);
    let index = ctx.OpShiftRightLogical(types.U32[1], offset, shift);
    let pointer = ctx.EmitSharedMemoryAccess(types.shared_u64, types.shared_memory_u64, index);
    let (scope, semantics) = atomic_args(ctx);
    match op {
        AtomicIncDec::Inc => ctx.OpAtomicIIncrement(types.U64, pointer, scope, semantics),
        AtomicIncDec::Dec => ctx.OpAtomicIDecrement(types.U64, pointer, scope, semantics),
    }
}

fn shared_atomic_u32_cmp_swap(ctx: &mut EmitContext, offset: Id, value: Id, cmp_value: Id) -> Id {
    let types = ctx.type_ids();
    let shift = ctx.ConstU32(2);
    let index = ctx.OpShiftRightLogical(types.U32[1], offset, shift);
    let pointer = ctx.EmitSharedMemoryAccess(types.shared_u32, types.shared_memory_u32, index);
    let (scope, semantics) = atomic_args(ctx);
    ctx.OpAtomicCompareExchange(
        types.U32[1],
        pointer,
        scope,
        semantics,
        semantics,
        value,
        cmp_value,
    )
}

fn shared_atomic_u64_cmp_swap(ctx: &mut EmitContext, offset: Id, value: Id, cmp_value: Id) -> Id {
    let types = ctx.type_ids();
    let shift = ctx.ConstU32(3);
    let index = ctx.OpShiftRightLogical(types.U32[1], offset, shift);
    let pointer = ctx.EmitSharedMemoryAccess(types.shared_u64, types.shared_memory_u64, index);
    let (scope, semantics) = atomic_args(ctx);
    ctx.OpAtomicCompareExchange(
        types.U64, pointer, scope, semantics, semantics, value, cmp_value,
    )
}

fn buffer_atomic_u32(
    ctx: &mut EmitContext,
    handle: u32,
    address: Id,
    value: Id,
    op: AtomicValueOp,
    is_float: bool,
) -> Id {
    let types = ctx.type_ids();
    let buffer = ctx.buffers[handle as usize].clone();
    let ty = if is_float { types.F32[1] } else { types.U32[1] };
    let address = if super::builder::ValidId(buffer.offset(PointerSize::B32)) {
        let offset = buffer.offset(PointerSize::B32);
        ctx.OpIAdd(types.U32[1], address, offset)
    } else {
        address
    };
    let alias = buffer.alias(if is_float {
        PointerType::F32
    } else {
        PointerType::U32
    });
    let zero = types.u32_zero_value;
    let ptr = ctx.OpAccessChain(alias.pointer_type, alias.id, &[zero, address]);
    let (scope, semantics) = atomic_args(ctx);
    apply_atomic(ctx, ty, ptr, scope, semantics, value, op)
}

fn buffer_atomic_u32_inc_dec(
    ctx: &mut EmitContext,
    handle: u32,
    address: Id,
    op: AtomicIncDec,
) -> Id {
    let types = ctx.type_ids();
    let buffer = ctx.buffers[handle as usize].clone();
    let address = if super::builder::ValidId(buffer.offset(PointerSize::B32)) {
        let offset = buffer.offset(PointerSize::B32);
        ctx.OpIAdd(types.U32[1], address, offset)
    } else {
        address
    };
    let alias = buffer.alias(PointerType::U32);
    let zero = types.u32_zero_value;
    let ptr = ctx.OpAccessChain(alias.pointer_type, alias.id, &[zero, address]);
    let (scope, semantics) = atomic_args(ctx);
    match op {
        AtomicIncDec::Inc => ctx.OpAtomicIIncrement(types.U32[1], ptr, scope, semantics),
        AtomicIncDec::Dec => ctx.OpAtomicIDecrement(types.U32[1], ptr, scope, semantics),
    }
}

fn buffer_atomic_u32_cmp_swap(
    ctx: &mut EmitContext,
    handle: u32,
    address: Id,
    value: Id,
    cmp_value: Id,
) -> Id {
    let types = ctx.type_ids();
    let buffer = ctx.buffers[handle as usize].clone();
    let address = if super::builder::ValidId(buffer.offset(PointerSize::B32)) {
        let offset = buffer.offset(PointerSize::B32);
        ctx.OpIAdd(types.U32[1], address, offset)
    } else {
        address
    };
    let alias = buffer.alias(PointerType::U32);
    let zero = types.u32_zero_value;
    let ptr = ctx.OpAccessChain(alias.pointer_type, alias.id, &[zero, address]);
    let (scope, semantics) = atomic_args(ctx);
    ctx.OpAtomicCompareExchange(
        types.U32[1],
        ptr,
        scope,
        semantics,
        semantics,
        value,
        cmp_value,
    )
}

fn buffer_atomic_u64(
    ctx: &mut EmitContext,
    handle: u32,
    address: Id,
    value: Id,
    op: AtomicValueOp,
) -> Id {
    let types = ctx.type_ids();
    let buffer = ctx.buffers[handle as usize].clone();
    let address = if super::builder::ValidId(buffer.offset(PointerSize::B64)) {
        let offset = buffer.offset(PointerSize::B64);
        ctx.OpIAdd(types.U32[1], address, offset)
    } else {
        address
    };
    let alias = buffer.alias(PointerType::U64);
    let zero = types.u32_zero_value;
    let ptr = ctx.OpAccessChain(alias.pointer_type, alias.id, &[zero, address]);
    let (scope, semantics) = atomic_args(ctx);
    apply_atomic(ctx, types.U64, ptr, scope, semantics, value, op)
}

fn image_atomic_u32(
    ctx: &mut EmitContext,
    handle: u32,
    coords: Id,
    value: Id,
    op: AtomicValueOp,
) -> Id {
    let types = ctx.type_ids();
    let texture = ctx.images[(handle & 0xFFFF) as usize];
    let sample = ctx.ConstU32(0);
    let pointer = ctx.OpImageTexelPointer(types.image_u32, texture.id, coords, sample);
    let (scope, semantics) = atomic_args(ctx);
    apply_atomic(ctx, types.U32[1], pointer, scope, semantics, value, op)
}

fn image_atomic_f32(
    ctx: &mut EmitContext,
    handle: u32,
    coords: Id,
    value: Id,
    op: AtomicValueOp,
) -> Id {
    let types = ctx.type_ids();
    let texture = ctx.images[(handle & 0xFFFF) as usize];
    let sample = ctx.ConstU32(0);
    let pointer = ctx.OpImageTexelPointer(types.image_f32, texture.id, coords, sample);
    let (scope, semantics) = atomic_args(ctx);
    apply_atomic(ctx, types.F32[1], pointer, scope, semantics, value, op)
}

fn image_atomic_u32_cmp_swap(
    ctx: &mut EmitContext,
    handle: u32,
    coords: Id,
    value: Id,
    cmp_value: Id,
) -> Id {
    let types = ctx.type_ids();
    let texture = ctx.images[(handle & 0xFFFF) as usize];
    let sample = ctx.ConstU32(0);
    let pointer = ctx.OpImageTexelPointer(types.image_u32, texture.id, coords, sample);
    let (scope, semantics) = atomic_args(ctx);
    ctx.OpAtomicCompareExchange(
        types.U32[1],
        pointer,
        scope,
        semantics,
        semantics,
        value,
        cmp_value,
    )
}

pub fn emit_shared_atomic_i_add32(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u32(ctx, offset, value, AtomicValueOp::IAdd)
}

pub fn emit_shared_atomic_i_add64(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u64(ctx, offset, value, AtomicValueOp::IAdd)
}

pub fn emit_shared_atomic_u_max32(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u32(ctx, offset, value, AtomicValueOp::UMax)
}

pub fn emit_shared_atomic_u_max64(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u64(ctx, offset, value, AtomicValueOp::UMax)
}

pub fn emit_shared_atomic_s_max32(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u32(ctx, offset, value, AtomicValueOp::SMax)
}

pub fn emit_shared_atomic_s_max64(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u64(ctx, offset, value, AtomicValueOp::SMax)
}

pub fn emit_shared_atomic_u_min32(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u32(ctx, offset, value, AtomicValueOp::UMin)
}

pub fn emit_shared_atomic_u_min64(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u64(ctx, offset, value, AtomicValueOp::UMin)
}

pub fn emit_shared_atomic_s_min32(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u32(ctx, offset, value, AtomicValueOp::SMin)
}

pub fn emit_shared_atomic_s_min64(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u64(ctx, offset, value, AtomicValueOp::SMin)
}

pub fn emit_shared_atomic_and32(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u32(ctx, offset, value, AtomicValueOp::And)
}

pub fn emit_shared_atomic_and64(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u64(ctx, offset, value, AtomicValueOp::And)
}

pub fn emit_shared_atomic_or32(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u32(ctx, offset, value, AtomicValueOp::Or)
}

pub fn emit_shared_atomic_or64(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u64(ctx, offset, value, AtomicValueOp::Or)
}

pub fn emit_shared_atomic_xor32(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u32(ctx, offset, value, AtomicValueOp::Xor)
}

pub fn emit_shared_atomic_xor64(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u64(ctx, offset, value, AtomicValueOp::Xor)
}

pub fn emit_shared_atomic_i_sub32(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u32(ctx, offset, value, AtomicValueOp::ISub)
}

pub fn emit_shared_atomic_i_sub64(ctx: &mut EmitContext, offset: Id, value: Id) -> Id {
    shared_atomic_u64(ctx, offset, value, AtomicValueOp::ISub)
}

pub fn emit_shared_atomic_cmp_swap32(
    ctx: &mut EmitContext,
    offset: Id,
    value: Id,
    cmp_value: Id,
) -> Id {
    shared_atomic_u32_cmp_swap(ctx, offset, value, cmp_value)
}

pub fn emit_shared_atomic_cmp_swap64(
    ctx: &mut EmitContext,
    offset: Id,
    value: Id,
    cmp_value: Id,
) -> Id {
    shared_atomic_u64_cmp_swap(ctx, offset, value, cmp_value)
}

pub fn emit_shared_atomic_inc32(ctx: &mut EmitContext, offset: Id) -> Id {
    shared_atomic_u32_inc_dec(ctx, offset, AtomicIncDec::Inc)
}

pub fn emit_shared_atomic_inc64(ctx: &mut EmitContext, offset: Id) -> Id {
    shared_atomic_u64_inc_dec(ctx, offset, AtomicIncDec::Inc)
}

pub fn emit_shared_atomic_dec32(ctx: &mut EmitContext, offset: Id) -> Id {
    shared_atomic_u32_inc_dec(ctx, offset, AtomicIncDec::Dec)
}

pub fn emit_shared_atomic_dec64(ctx: &mut EmitContext, offset: Id) -> Id {
    shared_atomic_u64_inc_dec(ctx, offset, AtomicIncDec::Dec)
}

pub fn emit_buffer_atomic_i_add32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) -> Id {
    buffer_atomic_u32(ctx, handle, address, value, AtomicValueOp::IAdd, false)
}

pub fn emit_buffer_atomic_i_add64(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) -> Id {
    buffer_atomic_u64(ctx, handle, address, value, AtomicValueOp::IAdd)
}

pub fn emit_buffer_atomic_i_sub32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) -> Id {
    buffer_atomic_u32(ctx, handle, address, value, AtomicValueOp::ISub, false)
}

pub fn emit_buffer_atomic_s_min32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) -> Id {
    buffer_atomic_u32(ctx, handle, address, value, AtomicValueOp::SMin, false)
}

pub fn emit_buffer_atomic_s_min64(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) -> Id {
    buffer_atomic_u64(ctx, handle, address, value, AtomicValueOp::SMin)
}

pub fn emit_buffer_atomic_u_min32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) -> Id {
    buffer_atomic_u32(ctx, handle, address, value, AtomicValueOp::UMin, false)
}

pub fn emit_buffer_atomic_u_min64(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) -> Id {
    buffer_atomic_u64(ctx, handle, address, value, AtomicValueOp::UMin)
}

pub fn emit_buffer_atomic_f_min32(
    ctx: &mut EmitContext,
    inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) -> Id {
    if ctx.profile.supports_buffer_fp32_atomic_min_max {
        return buffer_atomic_u32(ctx, handle, address, value, AtomicValueOp::FMin, true);
    }
    let types = ctx.type_ids();
    let u32_value = ctx.OpBitcast(types.U32[1], value);
    let shift = ctx.ConstU32(31);
    let one = ctx.ConstU32(1);
    let sign_bits = ctx.OpBitFieldUExtract(types.U32[1], u32_value, shift, one);
    let sign_bit_set = ctx.OpINotEqual(types.U1[1], sign_bits, types.u32_zero_value);
    let umax = emit_buffer_atomic_u_max32(ctx, inst, handle, address, u32_value);
    let umax = super::emit_spirv_bitwise_conversion::emit_bit_cast_f32u32(ctx, umax);
    let smin = emit_buffer_atomic_s_min32(ctx, inst, handle, address, u32_value);
    let smin = super::emit_spirv_bitwise_conversion::emit_bit_cast_f32u32(ctx, smin);
    ctx.OpSelect(types.F32[1], sign_bit_set, umax, smin)
}

pub fn emit_buffer_atomic_s_max32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) -> Id {
    buffer_atomic_u32(ctx, handle, address, value, AtomicValueOp::SMax, false)
}

pub fn emit_buffer_atomic_s_max64(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) -> Id {
    buffer_atomic_u64(ctx, handle, address, value, AtomicValueOp::SMax)
}

pub fn emit_buffer_atomic_u_max32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) -> Id {
    buffer_atomic_u32(ctx, handle, address, value, AtomicValueOp::UMax, false)
}

pub fn emit_buffer_atomic_u_max64(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) -> Id {
    buffer_atomic_u64(ctx, handle, address, value, AtomicValueOp::UMax)
}

pub fn emit_buffer_atomic_f_max32(
    ctx: &mut EmitContext,
    inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) -> Id {
    if ctx.profile.supports_buffer_fp32_atomic_min_max {
        return buffer_atomic_u32(ctx, handle, address, value, AtomicValueOp::FMax, true);
    }
    let types = ctx.type_ids();
    let u32_value = ctx.OpBitcast(types.U32[1], value);
    let shift = ctx.ConstU32(31);
    let one = ctx.ConstU32(1);
    let sign_bits = ctx.OpBitFieldUExtract(types.U32[1], u32_value, shift, one);
    let sign_bit_set = ctx.OpINotEqual(types.U1[1], sign_bits, types.u32_zero_value);
    let umin = emit_buffer_atomic_u_min32(ctx, inst, handle, address, u32_value);
    let umin = super::emit_spirv_bitwise_conversion::emit_bit_cast_f32u32(ctx, umin);
    let smax = emit_buffer_atomic_s_max32(ctx, inst, handle, address, u32_value);
    let smax = super::emit_spirv_bitwise_conversion::emit_bit_cast_f32u32(ctx, smax);
    ctx.OpSelect(types.F32[1], sign_bit_set, umin, smax)
}

pub fn emit_buffer_atomic_inc32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
) -> Id {
    buffer_atomic_u32_inc_dec(ctx, handle, address, AtomicIncDec::Inc)
}

pub fn emit_buffer_atomic_dec32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
) -> Id {
    buffer_atomic_u32_inc_dec(ctx, handle, address, AtomicIncDec::Dec)
}

pub fn emit_buffer_atomic_and32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) -> Id {
    buffer_atomic_u32(ctx, handle, address, value, AtomicValueOp::And, false)
}

pub fn emit_buffer_atomic_or32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) -> Id {
    buffer_atomic_u32(ctx, handle, address, value, AtomicValueOp::Or, false)
}

pub fn emit_buffer_atomic_xor32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) -> Id {
    buffer_atomic_u32(ctx, handle, address, value, AtomicValueOp::Xor, false)
}

pub fn emit_buffer_atomic_swap32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) -> Id {
    buffer_atomic_u32(ctx, handle, address, value, AtomicValueOp::Exchange, false)
}

pub fn emit_buffer_atomic_cmp_swap32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
    cmp_value: Id,
) -> Id {
    buffer_atomic_u32_cmp_swap(ctx, handle, address, value, cmp_value)
}

pub fn emit_buffer_atomic_f_cmp_swap32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
    cmp_value: Id,
) -> Id {
    let types = ctx.type_ids();
    let u32_value = ctx.OpBitcast(types.U32[1], value);
    let u32_cmp = ctx.OpBitcast(types.U32[1], cmp_value);
    let result = buffer_atomic_u32_cmp_swap(ctx, handle, address, u32_value, u32_cmp);
    ctx.OpBitcast(types.F32[1], result)
}

pub fn emit_image_atomic_i_add32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    value: Id,
) -> Id {
    image_atomic_u32(ctx, handle, coords, value, AtomicValueOp::IAdd)
}

pub fn emit_image_atomic_s_min32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    value: Id,
) -> Id {
    image_atomic_u32(ctx, handle, coords, value, AtomicValueOp::SMin)
}

pub fn emit_image_atomic_u_min32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    value: Id,
) -> Id {
    image_atomic_u32(ctx, handle, coords, value, AtomicValueOp::UMin)
}

pub fn emit_image_atomic_s_max32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    value: Id,
) -> Id {
    image_atomic_u32(ctx, handle, coords, value, AtomicValueOp::SMax)
}

pub fn emit_image_atomic_u_max32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    value: Id,
) -> Id {
    image_atomic_u32(ctx, handle, coords, value, AtomicValueOp::UMax)
}

pub fn emit_image_atomic_f_max32(
    ctx: &mut EmitContext,
    inst: &InstRef,
    handle: u32,
    coords: Id,
    value: Id,
) -> Id {
    if ctx.profile.supports_image_fp32_atomic_min_max {
        return image_atomic_f32(ctx, handle, coords, value, AtomicValueOp::FMax);
    }
    let types = ctx.type_ids();
    let u32_value = ctx.OpBitcast(types.U32[1], value);
    let shift = ctx.ConstU32(31);
    let one = ctx.ConstU32(1);
    let sign_bits = ctx.OpBitFieldUExtract(types.U32[1], u32_value, shift, one);
    let sign_bit_set = ctx.OpINotEqual(types.U1[1], sign_bits, types.u32_zero_value);
    let umin = emit_image_atomic_u_min32(ctx, inst, handle, coords, u32_value);
    let umin = super::emit_spirv_bitwise_conversion::emit_bit_cast_f32u32(ctx, umin);
    let smax = emit_image_atomic_s_max32(ctx, inst, handle, coords, u32_value);
    let smax = super::emit_spirv_bitwise_conversion::emit_bit_cast_f32u32(ctx, smax);
    ctx.OpSelect(types.F32[1], sign_bit_set, umin, smax)
}

pub fn emit_image_atomic_f_min32(
    ctx: &mut EmitContext,
    inst: &InstRef,
    handle: u32,
    coords: Id,
    value: Id,
) -> Id {
    if ctx.profile.supports_image_fp32_atomic_min_max {
        return image_atomic_f32(ctx, handle, coords, value, AtomicValueOp::FMin);
    }
    let types = ctx.type_ids();
    let u32_value = ctx.OpBitcast(types.U32[1], value);
    let shift = ctx.ConstU32(31);
    let one = ctx.ConstU32(1);
    let sign_bits = ctx.OpBitFieldUExtract(types.U32[1], u32_value, shift, one);
    let sign_bit_set = ctx.OpINotEqual(types.U1[1], sign_bits, types.u32_zero_value);
    let umax = emit_image_atomic_u_max32(ctx, inst, handle, coords, u32_value);
    let umax = super::emit_spirv_bitwise_conversion::emit_bit_cast_f32u32(ctx, umax);
    let smin = emit_image_atomic_s_min32(ctx, inst, handle, coords, u32_value);
    let smin = super::emit_spirv_bitwise_conversion::emit_bit_cast_f32u32(ctx, smin);
    ctx.OpSelect(types.F32[1], sign_bit_set, umax, smin)
}

pub fn emit_image_atomic_inc32(
    _ctx: &mut EmitContext,
    _inst: &InstRef,
    _handle: u32,
    _coords: Id,
    _value: Id,
) -> Id {
    unreachable!("SPIR-V Instruction")
}

pub fn emit_image_atomic_dec32(
    _ctx: &mut EmitContext,
    _inst: &InstRef,
    _handle: u32,
    _coords: Id,
    _value: Id,
) -> Id {
    unreachable!("SPIR-V Instruction")
}

pub fn emit_image_atomic_and32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    value: Id,
) -> Id {
    image_atomic_u32(ctx, handle, coords, value, AtomicValueOp::And)
}

pub fn emit_image_atomic_or32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    value: Id,
) -> Id {
    image_atomic_u32(ctx, handle, coords, value, AtomicValueOp::Or)
}

pub fn emit_image_atomic_xor32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    value: Id,
) -> Id {
    image_atomic_u32(ctx, handle, coords, value, AtomicValueOp::Xor)
}

pub fn emit_image_atomic_exchange32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    value: Id,
) -> Id {
    image_atomic_u32(ctx, handle, coords, value, AtomicValueOp::Exchange)
}

pub fn emit_image_atomic_cmp_swap32(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    value: Id,
    cmp_value: Id,
) -> Id {
    image_atomic_u32_cmp_swap(ctx, handle, coords, value, cmp_value)
}

pub fn emit_data_append(_ctx: &mut EmitContext, _gds_dw_offset: Id, _exec: Id) -> Id {
    unreachable!("SPIR-V Instruction")
}

pub fn emit_data_consume(_ctx: &mut EmitContext, _gds_dw_offset: Id, _exec: Id) -> Id {
    unreachable!("SPIR-V Instruction")
}
