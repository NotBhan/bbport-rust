//! Undefined-value emitters, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv_undefined.cpp`.

use super::builder::Id;
use super::emit_context::EmitContext;

pub fn emit_undef_u1(ctx: &mut EmitContext) -> Id {
    let types = ctx.type_ids();
    ctx.OpUndef(types.U1[1])
}

pub fn emit_undef_u8(_ctx: &mut EmitContext) -> Id {
    unreachable!("SPIR-V Instruction")
}

pub fn emit_undef_u16(_ctx: &mut EmitContext) -> Id {
    unreachable!("SPIR-V Instruction")
}

pub fn emit_undef_u32(ctx: &mut EmitContext) -> Id {
    let types = ctx.type_ids();
    ctx.OpUndef(types.U32[1])
}

pub fn emit_undef_u32x2(ctx: &mut EmitContext) -> Id {
    let types = ctx.type_ids();
    ctx.OpUndef(types.U32[2])
}

pub fn emit_undef_u32x3(ctx: &mut EmitContext) -> Id {
    let types = ctx.type_ids();
    ctx.OpUndef(types.U32[3])
}

pub fn emit_undef_u32x4(ctx: &mut EmitContext) -> Id {
    let types = ctx.type_ids();
    ctx.OpUndef(types.U32[4])
}

pub fn emit_undef_f32(ctx: &mut EmitContext) -> Id {
    let types = ctx.type_ids();
    ctx.OpUndef(types.F32[1])
}

pub fn emit_undef_u64(_ctx: &mut EmitContext) -> Id {
    unreachable!("SPIR-V Instruction")
}
