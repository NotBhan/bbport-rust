//! Shared-memory emitters, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv_shared_memory.cpp`.

use super::builder::Id;
use super::emit_context::EmitContext;

pub fn emit_load_shared_u16(ctx: &mut EmitContext, offset: Id) -> Id {
    let types = ctx.type_ids();
    let shift = ctx.ConstU32(1);
    let index = ctx.OpShiftRightLogical(types.U32[1], offset, shift);
    let pointer = ctx.EmitSharedMemoryAccess(types.shared_u16, types.shared_memory_u16, index);
    ctx.OpLoad(types.U16, pointer, &[])
}

pub fn emit_load_shared_u32(ctx: &mut EmitContext, offset: Id) -> Id {
    let types = ctx.type_ids();
    let shift = ctx.ConstU32(2);
    let index = ctx.OpShiftRightLogical(types.U32[1], offset, shift);
    let pointer = ctx.EmitSharedMemoryAccess(types.shared_u32, types.shared_memory_u32, index);
    ctx.OpLoad(types.U32[1], pointer, &[])
}

pub fn emit_load_shared_u64(ctx: &mut EmitContext, offset: Id) -> Id {
    let types = ctx.type_ids();
    let shift = ctx.ConstU32(3);
    let index = ctx.OpShiftRightLogical(types.U32[1], offset, shift);
    let pointer = ctx.EmitSharedMemoryAccess(types.shared_u64, types.shared_memory_u64, index);
    ctx.OpLoad(types.U64, pointer, &[])
}

pub fn emit_write_shared_u16(ctx: &mut EmitContext, offset: Id, value: Id) {
    let types = ctx.type_ids();
    let shift = ctx.ConstU32(1);
    let index = ctx.OpShiftRightLogical(types.U32[1], offset, shift);
    let pointer = ctx.EmitSharedMemoryAccess(types.shared_u16, types.shared_memory_u16, index);
    ctx.OpStore(pointer, value, &[]);
}

pub fn emit_write_shared_u32(ctx: &mut EmitContext, offset: Id, value: Id) {
    let types = ctx.type_ids();
    let shift = ctx.ConstU32(2);
    let index = ctx.OpShiftRightLogical(types.U32[1], offset, shift);
    let pointer = ctx.EmitSharedMemoryAccess(types.shared_u32, types.shared_memory_u32, index);
    ctx.OpStore(pointer, value, &[]);
}

pub fn emit_write_shared_u64(ctx: &mut EmitContext, offset: Id, value: Id) {
    let types = ctx.type_ids();
    let shift = ctx.ConstU32(3);
    let index = ctx.OpShiftRightLogical(types.U32[1], offset, shift);
    let pointer = ctx.EmitSharedMemoryAccess(types.shared_u64, types.shared_memory_u64, index);
    ctx.OpStore(pointer, value, &[]);
}
