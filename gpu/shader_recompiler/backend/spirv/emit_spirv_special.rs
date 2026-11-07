//! Prologue/epilogue and special emitters, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv_special.cpp`.

use super::builder::Id;
use super::emit_context::EmitContext;
use super::spv;
use crate::gpu::shader_recompiler::ir::attribute::Attribute;
use crate::gpu::shader_recompiler::ir::debug_print::{DebugPrintFlags, DEBUGPRINT_NUM_FORMAT_ARGS};
use crate::gpu::shader_recompiler::ir::microinstruction::InstRef;
use crate::gpu::shader_recompiler::ir::value::Value;
use crate::gpu::shader_recompiler::resource::PushData;
use crate::gpu::shader_recompiler::runtime_info::{HwInfo, HwStage, MotionVectors};

pub fn emit_prologue(ctx: &mut EmitContext) {
    if ctx.hw_stage == HwStage::Fragment {
        ctx.DefineAmdPerVertexAttribs();
    }
    if ctx.info.borrow().loads.get(Attribute::WorkgroupIndex, 0) {
        ctx.DefineWorkgroupIndex();
    }
    ctx.DefineBufferProperties();
}

pub fn emit_epilogue(ctx: &mut EmitContext) {
    if super::builder::ValidId(ctx.motion_out_cur) {
        emit_vertex_motion(ctx);
    }
    if super::builder::ValidId(ctx.motion_frag_out) {
        emit_fragment_motion(ctx);
    }
    let vs = match &ctx.runtime_info.hw {
        HwInfo::Vs(vs) => *vs,
        _ => return,
    };
    if ctx.hw_stage == HwStage::Vertex && vs.emulate_depth_negative_one_to_one {
        convert_depth_mode(ctx);
    }
    if ctx.hw_stage == HwStage::Vertex && vs.clip_disable {
        convert_position_to_clip_space(ctx);
    }
}

fn convert_depth_mode(ctx: &mut EmitContext) {
    let types = ctx.type_ids();
    let ty = types.F32[1];
    let position = ctx.OpLoad(types.F32[4], types.output_position, &[]);
    let z = ctx.OpCompositeExtract(ty, position, &[2]);
    let w = ctx.OpCompositeExtract(ty, position, &[3]);
    let sum = ctx.OpFAdd(ty, z, w);
    let half = ctx.Constant(ty, 0.5f32);
    let screen_depth = ctx.OpFMul(ty, sum, half);
    let vector = ctx.OpCompositeInsert(types.F32[4], screen_depth, position, &[2]);
    ctx.OpStore(types.output_position, vector, &[]);
}

fn push_constant_ptr(ctx: &mut EmitContext, ty: Id, index: u32) -> Id {
    let types = ctx.type_ids();
    let pointer_type = ctx.TypePointer(spv::StorageClass::PushConstant, ty);
    let index = ctx.ConstU32(index);
    ctx.OpAccessChain(pointer_type, types.push_data_block, &[index])
}

fn convert_position_to_clip_space(ctx: &mut EmitContext) {
    assert!(
        !ctx.info.borrow().stores.get_any(Attribute::ViewportIndex),
        "Multi-viewport with shader clip space conversion not yet implemented."
    );
    let types = ctx.type_ids();
    let ty = types.F32[1];
    let position = ctx.OpLoad(types.F32[4], types.output_position, &[]);
    let x = ctx.OpCompositeExtract(ty, position, &[0]);
    let y = ctx.OpCompositeExtract(ty, position, &[1]);
    let z = ctx.OpCompositeExtract(ty, position, &[2]);
    let w = ctx.OpCompositeExtract(ty, position, &[3]);

    let xoffset_ptr = push_constant_ptr(ctx, ty, PushData::X_OFFSET_INDEX);
    let xoffset = ctx.OpLoad(ty, xoffset_ptr, &[]);
    let yoffset_ptr = push_constant_ptr(ctx, ty, PushData::Y_OFFSET_INDEX);
    let yoffset = ctx.OpLoad(ty, yoffset_ptr, &[]);
    let xscale_ptr = push_constant_ptr(ctx, ty, PushData::X_SCALE_INDEX);
    let xscale = ctx.OpLoad(ty, xscale_ptr, &[]);
    let yscale_ptr = push_constant_ptr(ctx, ty, PushData::Y_SCALE_INDEX);
    let yscale = ctx.OpLoad(ty, yscale_ptr, &[]);

    let max_width = ctx.profile.max_viewport_width;
    let vport_w = ctx.Constant(ty, (max_width / 2).min(8 * 1024) as f32);
    let scaled_x = ctx.OpFMul(ty, x, xscale);
    let wnd_x = ctx.OpFAdd(ty, scaled_x, xoffset);
    let divided = ctx.OpFDiv(ty, wnd_x, vport_w);
    let one = ctx.Constant(ty, 1.0f32);
    let ndc_x = ctx.OpFSub(ty, divided, one);

    let max_height = ctx.profile.max_viewport_height;
    let vport_h = ctx.Constant(ty, (max_height / 2).min(8 * 1024) as f32);
    let scaled_y = ctx.OpFMul(ty, y, yscale);
    let wnd_y = ctx.OpFAdd(ty, scaled_y, yoffset);
    let divided = ctx.OpFDiv(ty, wnd_y, vport_h);
    let one = ctx.Constant(ty, 1.0f32);
    let ndc_y = ctx.OpFSub(ty, divided, one);
    let vector = ctx.OpCompositeConstruct(types.F32[4], &[ndc_x, ndc_y, z, w]);
    ctx.OpStore(types.output_position, vector, &[]);
}

fn motion_address(ctx: &mut EmitContext, base: u64, index: Id, stride: u32) -> Id {
    let types = ctx.type_ids();
    let u64_ty = types.U64;
    let base = ctx.Constant(u64_ty, base);
    let converted = ctx.OpUConvert(u64_ty, index);
    let stride = ctx.Constant(u64_ty, u64::from(stride));
    let scaled = ctx.OpIMul(u64_ty, converted, stride);
    ctx.OpIAdd(u64_ty, base, scaled)
}

fn motion_flag(ctx: &mut EmitContext, in_range: Id, flags: Id, bit: u32) -> Id {
    let types = ctx.type_ids();
    let mask = ctx.ConstU32(bit);
    let masked = ctx.OpBitwiseAnd(types.U32[1], flags, mask);
    let nonzero = ctx.OpINotEqual(types.U1[1], masked, types.u32_zero_value);
    ctx.OpLogicalAnd(types.U1[1], in_range, nonzero)
}

fn emit_vertex_motion(ctx: &mut EmitContext) {
    let types = ctx.type_ids();
    let u32_ty = types.U32[1];
    let position = ctx.OpLoad(types.F32[4], types.output_position, &[]);
    let param_ptr = push_constant_ptr(ctx, u32_ty, PushData::MOTION_PARAM_INDEX);
    let param_index = ctx.OpLoad(u32_ty, param_ptr, &[]);
    let u32x4_ptr = ctx.TypePointer(spv::StorageClass::PhysicalStorageBuffer, types.U32[4]);
    let f32x4_ptr = ctx.TypePointer(spv::StorageClass::PhysicalStorageBuffer, types.F32[4]);

    let params_address = MotionVectors::params_address();
    let address = motion_address(ctx, params_address, param_index, 32);
    let pointer = ctx.OpConvertUToPtr(u32x4_ptr, address);
    let params = ctx.OpLoad(types.U32[4], pointer, &[spv::MemoryAccessMask::Aligned, 16]);
    let store_base = ctx.OpCompositeExtract(u32_ty, params, &[0]);
    let load_base = ctx.OpCompositeExtract(u32_ty, params, &[1]);
    let vertices = ctx.OpCompositeExtract(u32_ty, params, &[2]);
    let flags = ctx.OpCompositeExtract(u32_ty, params, &[3]);

    let address = motion_address(ctx, params_address + 16, param_index, 32);
    let pointer = ctx.OpConvertUToPtr(u32x4_ptr, address);
    let offsets = ctx.OpLoad(types.U32[4], pointer, &[spv::MemoryAccessMask::Aligned, 16]);
    let first_vertex = ctx.OpCompositeExtract(u32_ty, offsets, &[0]);
    let first_instance = ctx.OpCompositeExtract(u32_ty, offsets, &[1]);
    let instances = ctx.OpCompositeExtract(u32_ty, offsets, &[2]);

    let loaded_vertex = ctx.OpLoad(u32_ty, types.vertex_index, &[]);
    let vertex = ctx.OpISub(u32_ty, loaded_vertex, first_vertex);
    let loaded_instance = ctx.OpLoad(u32_ty, types.instance_id, &[]);
    let instance = ctx.OpISub(u32_ty, loaded_instance, first_instance);
    let bool_ty = types.U1[1];
    let vertex_ok = ctx.OpULessThan(bool_ty, vertex, vertices);
    let instance_ok = ctx.OpULessThan(bool_ty, instance, instances);
    let in_range = ctx.OpLogicalAnd(bool_ty, vertex_ok, instance_ok);
    let instance_scaled = ctx.OpIMul(u32_ty, instance, vertices);
    let slot = ctx.OpIAdd(u32_ty, vertex, instance_scaled);

    let do_store = motion_flag(ctx, in_range, flags, MotionVectors::FLAG_STORE);
    let do_load = motion_flag(ctx, in_range, flags, MotionVectors::FLAG_LOAD);

    let store_label = ctx.OpLabel();
    let store_merge = ctx.OpLabel();
    ctx.OpSelectionMerge(store_merge, spv::SelectionControlMask::MaskNone);
    ctx.OpBranchConditional(do_store, store_label, store_merge);
    ctx.AddExistingLabel(store_label);
    let base_plus_slot = ctx.OpIAdd(u32_ty, store_base, slot);
    let address = motion_address(ctx, MotionVectors::positions_address(), base_plus_slot, 16);
    let pointer = ctx.OpConvertUToPtr(f32x4_ptr, address);
    ctx.OpStore(pointer, position, &[spv::MemoryAccessMask::Aligned, 16]);
    ctx.OpBranch(store_merge);
    ctx.AddExistingLabel(store_merge);

    let load_label = ctx.OpLabel();
    let load_merge = ctx.OpLabel();
    ctx.OpSelectionMerge(load_merge, spv::SelectionControlMask::MaskNone);
    ctx.OpBranchConditional(do_load, load_label, load_merge);
    ctx.AddExistingLabel(load_label);
    let base_plus_slot = ctx.OpIAdd(u32_ty, load_base, slot);
    let address = motion_address(ctx, MotionVectors::positions_address(), base_plus_slot, 16);
    let pointer = ctx.OpConvertUToPtr(f32x4_ptr, address);
    let loaded = ctx.OpLoad(types.F32[4], pointer, &[spv::MemoryAccessMask::Aligned, 16]);
    ctx.OpBranch(load_merge);
    ctx.AddExistingLabel(load_merge);
    let previous = ctx.OpPhi(types.F32[4], &[position, store_merge, loaded, load_label]);
    let one = ctx.Constant(types.F32[1], 1.0f32);
    let zero = ctx.Constant(types.F32[1], 0.0f32);
    let valid = ctx.OpSelect(types.F32[1], do_load, one, zero);
    let previous = ctx.OpCompositeInsert(types.F32[4], valid, previous, &[2]);
    ctx.OpStore(types.motion_out_cur, position, &[]);
    ctx.OpStore(types.motion_out_prev, previous, &[]);
}

fn emit_fragment_motion(ctx: &mut EmitContext) {
    let types = ctx.type_ids();
    let f32_ty = types.F32[1];
    let current = ctx.OpLoad(types.F32[4], types.motion_in_cur, &[]);
    let previous = ctx.OpLoad(types.F32[4], types.motion_in_prev, &[]);
    let xscale_ptr = push_constant_ptr(ctx, f32_ty, PushData::X_SCALE_INDEX);
    let xscale = ctx.OpLoad(f32_ty, xscale_ptr, &[]);
    let yscale_ptr = push_constant_ptr(ctx, f32_ty, PushData::Y_SCALE_INDEX);
    let yscale = ctx.OpLoad(f32_ty, yscale_ptr, &[]);

    let prev_x = ctx.OpCompositeExtract(f32_ty, previous, &[0]);
    let prev_w = ctx.OpCompositeExtract(f32_ty, previous, &[3]);
    let prev_ndc_x = ctx.OpFDiv(f32_ty, prev_x, prev_w);
    let cur_x = ctx.OpCompositeExtract(f32_ty, current, &[0]);
    let cur_w = ctx.OpCompositeExtract(f32_ty, current, &[3]);
    let cur_ndc_x = ctx.OpFDiv(f32_ty, cur_x, cur_w);
    let dx = ctx.OpFSub(f32_ty, prev_ndc_x, cur_ndc_x);
    let dx = ctx.OpFMul(f32_ty, dx, xscale);

    let prev_y = ctx.OpCompositeExtract(f32_ty, previous, &[1]);
    let prev_ndc_y = ctx.OpFDiv(f32_ty, prev_y, prev_w);
    let cur_y = ctx.OpCompositeExtract(f32_ty, current, &[1]);
    let cur_ndc_y = ctx.OpFDiv(f32_ty, cur_y, cur_w);
    let dy = ctx.OpFSub(f32_ty, prev_ndc_y, cur_ndc_y);
    let dy = ctx.OpFMul(f32_ty, dy, yscale);

    let zero = ctx.Constant(f32_ty, 0.0f32);
    let epsilon = ctx.Constant(f32_ty, 1e-5f32);
    let prev_z = ctx.OpCompositeExtract(f32_ty, previous, &[3]);
    let prev_valid = ctx.OpFOrdGreaterThan(types.U1[1], prev_z, epsilon);
    let cur_z = ctx.OpCompositeExtract(f32_ty, current, &[3]);
    let cur_valid = ctx.OpFOrdGreaterThan(types.U1[1], cur_z, epsilon);
    let front = ctx.OpLogicalAnd(types.U1[1], prev_valid, cur_valid);
    let prev_alpha = ctx.OpCompositeExtract(f32_ty, previous, &[2]);
    let valid = ctx.OpSelect(f32_ty, front, prev_alpha, zero);

    let frag_coord = ctx.OpLoad(types.F32[4], types.frag_coord, &[]);
    let depth = ctx.OpCompositeExtract(f32_ty, frag_coord, &[2]);
    let out_x = ctx.OpSelect(f32_ty, front, dx, zero);
    let out_y = ctx.OpSelect(f32_ty, front, dy, zero);
    let vector = ctx.OpCompositeConstruct(types.F32[4], &[out_x, out_y, valid, depth]);
    ctx.OpStore(types.motion_frag_out, vector, &[]);
}

pub fn emit_discard(ctx: &mut EmitContext) {
    ctx.OpDemoteToHelperInvocationEXT();
}

pub fn emit_discard_cond(ctx: &mut EmitContext, condition: Id) {
    let kill_label = ctx.OpLabel();
    let merge_label = ctx.OpLabel();
    ctx.OpSelectionMerge(merge_label, spv::SelectionControlMask::MaskNone);
    ctx.OpBranchConditional(condition, kill_label, merge_label);
    ctx.AddExistingLabel(kill_label);
    ctx.OpDemoteToHelperInvocationEXT();
    ctx.OpBranch(merge_label);
    ctx.AddExistingLabel(merge_label);
}

pub fn emit_emit_vertex(ctx: &mut EmitContext) {
    ctx.OpEmitVertex();
}

pub fn emit_emit_primitive(ctx: &mut EmitContext) {
    ctx.OpEndPrimitive();
}

pub fn emit_emit_vertex_stream(_ctx: &mut EmitContext, _stream: &Value) {
    unreachable!("Geometry streams")
}

pub fn emit_end_primitive_stream(_ctx: &mut EmitContext, _stream: &Value) {
    unreachable!("Geometry streams")
}

pub fn emit_debug_print(
    ctx: &mut EmitContext,
    inst: &InstRef,
    fmt: Id,
    arg0: Id,
    arg1: Id,
    arg2: Id,
    arg3: Id,
) {
    let flags = DebugPrintFlags::from_raw(inst.borrow().flags_u32());
    let args = [arg0, arg1, arg2, arg3];
    let count = (flags.num_args() as usize).min(DEBUGPRINT_NUM_FORMAT_ARGS);
    ctx.OpDebugPrintf(fmt, &args[..count]);
}

pub fn emit_memtime(ctx: &mut EmitContext) -> Id {
    let types = ctx.type_ids();
    if ctx.profile.supports_shader_subgroup_clock {
        let scope = ctx.ConstU32(spv::Scope::Subgroup);
        ctx.OpReadClockKHR(types.U64, scope)
    } else {
        ctx.Constant(types.U64, 1u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::backend::bindings::Bindings;

    fn opcode(word: u32) -> u32 {
        word & 0xffff
    }

    #[test]
    fn memtime_without_a_clock_reads_a_constant() {
        let read = |_| 0u32;
        let mut binding = Bindings::default();
        let mut ctx = super::super::emit_context::test_fragment_context(&mut binding, &read);
        let result = emit_memtime(&mut ctx);
        assert_ne!(result, 0);
        let words = ctx.Assemble();
        let constant = words
            .windows(5)
            .find(|window| {
                opcode(window[0]) == spv::Op::Constant
                    && window[0] >> 16 == 5
                    && window[1] == ctx.U64
            })
            .expect("a 64-bit constant");
        assert_eq!(constant[3], 1);
    }
}
