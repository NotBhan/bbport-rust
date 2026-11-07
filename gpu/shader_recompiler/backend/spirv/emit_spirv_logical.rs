//! Logical emitters, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv_logical.cpp`.

use super::builder::Id;
use super::emit_context::EmitContext;

pub fn emit_logical_or(ctx: &mut EmitContext, a: Id, b: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpLogicalOr(types.U1[1], a, b)
}

pub fn emit_logical_and(ctx: &mut EmitContext, a: Id, b: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpLogicalAnd(types.U1[1], a, b)
}

pub fn emit_logical_xor(ctx: &mut EmitContext, a: Id, b: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpLogicalNotEqual(types.U1[1], a, b)
}

pub fn emit_logical_not(ctx: &mut EmitContext, value: Id) -> Id {
    let types = ctx.type_ids();
    ctx.OpLogicalNot(types.U1[1], value)
}
