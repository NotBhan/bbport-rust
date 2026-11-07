//! Select emitters, ported from
//! `legacy/gpu/shadsps4/shader_recompiler/backend/spirv/emit_spirv_select.cpp`.

use super::builder::Id;
use super::emit_context::EmitContext;

pub fn emit_select_u1(ctx: &mut EmitContext, cond: Id, true_value: Id, false_value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpSelect(types.U1[1], cond, true_value, false_value)
}

pub fn emit_select_u32(ctx: &mut EmitContext, cond: Id, true_value: Id, false_value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpSelect(types.U32[1], cond, true_value, false_value)
}

pub fn emit_select_f32(ctx: &mut EmitContext, cond: Id, true_value: Id, false_value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpSelect(types.F32[1], cond, true_value, false_value)
}

pub fn emit_select_u64(ctx: &mut EmitContext, cond: Id, true_value: Id, false_value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpSelect(types.U64, cond, true_value, false_value)
}
