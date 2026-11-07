//! Subgroup emitters, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv_warp.cpp`.

use super::builder::Id;
use super::emit_context::EmitContext;
use super::spv;

fn subgroup_scope(ctx: &mut EmitContext) -> Id {
    ctx.ConstU32(spv::Scope::Subgroup)
}

pub fn emit_warp_id(_ctx: &mut EmitContext) -> Id {
    unreachable!()
}

pub fn emit_lane_id(ctx: &mut EmitContext) -> Id {
    let types = ctx.type_ids();
    ctx.OpLoad(types.U32[1], types.subgroup_local_invocation_id, &[])
}

pub fn emit_quad_broadcast(ctx: &mut EmitContext, value: Id, index: Id) -> Id {
    let ty = ctx.type_ids().U32[1];
    let scope = subgroup_scope(ctx);
    ctx.OpGroupNonUniformQuadBroadcast(ty, scope, value, index)
}

pub fn emit_read_first_lane(ctx: &mut EmitContext, value: Id) -> Id {
    let ty = ctx.type_ids().U32[1];
    let scope = subgroup_scope(ctx);
    ctx.OpGroupNonUniformBroadcastFirst(ty, scope, value)
}

pub fn emit_shuffle(ctx: &mut EmitContext, value: Id, index: Id) -> Id {
    let ty = ctx.type_ids().U32[1];
    let scope = subgroup_scope(ctx);
    ctx.OpGroupNonUniformShuffle(ty, scope, value, index)
}

pub fn emit_shuffle_xor(ctx: &mut EmitContext, value: Id, mask: Id) -> Id {
    let ty = ctx.type_ids().U32[1];
    let scope = subgroup_scope(ctx);
    ctx.OpGroupNonUniformShuffleXor(ty, scope, value, mask)
}

pub fn emit_read_lane(ctx: &mut EmitContext, value: Id, lane: Id) -> Id {
    let ty = ctx.type_ids().U32[1];
    let scope = subgroup_scope(ctx);
    ctx.OpGroupNonUniformBroadcast(ty, scope, value, lane)
}

pub fn emit_write_lane(ctx: &mut EmitContext, _value: Id, _write_value: Id, _lane: u32) -> Id {
    let types = ctx.type_ids();
    types.u32_zero_value
}

pub fn emit_ballot(ctx: &mut EmitContext, bit: Id) -> Id {
    let types = ctx.type_ids();
    let scope = subgroup_scope(ctx);
    let ballot = ctx.OpGroupNonUniformBallot(types.U32[4], scope, bit);
    let shuffled = ctx.OpVectorShuffle(types.U32[2], ballot, ballot, &[0, 1]);
    ctx.OpBitcast(types.U64, shuffled)
}

pub fn emit_ballot_find_lsb(ctx: &mut EmitContext, mask: Id) -> Id {
    let types = ctx.type_ids();
    let packed = ctx.OpBitcast(types.U32[2], mask);
    let value = ctx.OpCompositeConstruct(
        types.U32[4],
        &[packed, types.u32_zero_value, types.u32_zero_value],
    );
    let scope = subgroup_scope(ctx);
    ctx.OpGroupNonUniformBallotFindLSB(types.U32[1], scope, value)
}

pub fn emit_inverse_ballot(ctx: &mut EmitContext, mask: Id) -> Id {
    let types = ctx.type_ids();
    let packed = ctx.OpBitcast(types.U32[2], mask);
    let value = ctx.OpCompositeConstruct(
        types.U32[4],
        &[packed, types.u32_zero_value, types.u32_zero_value],
    );
    let scope = subgroup_scope(ctx);
    ctx.OpGroupNonUniformInverseBallot(types.U1[1], scope, value)
}

pub fn emit_group_any(ctx: &mut EmitContext, bit: Id) -> Id {
    let ty = ctx.type_ids().U1[1];
    let scope = subgroup_scope(ctx);
    ctx.OpGroupNonUniformAny(ty, scope, bit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::backend::bindings::Bindings;

    fn opcode(word: u32) -> u32 {
        word & 0xffff
    }

    #[test]
    fn a_ballot_is_a_u32x4_shuffled_into_a_u64() {
        let read = |_| 0u32;
        let mut binding = Bindings::default();
        let mut ctx = super::super::emit_context::test_fragment_context(&mut binding, &read);
        let result = emit_ballot(&mut ctx, 1);
        assert_ne!(result, 0);
        let words = ctx.Assemble();
        assert!(words
            .iter()
            .any(|word| opcode(*word) == spv::Op::GroupNonUniformBallot));
        assert!(words
            .iter()
            .any(|word| opcode(*word) == spv::Op::VectorShuffle));
        assert!(words.iter().any(|word| opcode(*word) == spv::Op::Bitcast));
    }
}
