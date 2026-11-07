//! Attribute, register, patch and buffer access emitters, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv_context_get_set.cpp`.

#![allow(clippy::too_many_arguments)]

use super::builder::{Id, ValidId};
use super::emit_context::{EmitContext, PointerSize, PointerType};
use super::spv;
use crate::gpu::shader_recompiler::ir::attribute::Attribute;
use crate::gpu::shader_recompiler::ir::microinstruction::InstRef;
use crate::gpu::shader_recompiler::ir::patch::Patch;
use crate::gpu::shader_recompiler::ir::reg::ScalarReg;
use crate::gpu::shader_recompiler::resource::PushData;
use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage};

fn is_param(attr: Attribute) -> bool {
    matches!(
        attr,
        Attribute::Param0
            | Attribute::Param1
            | Attribute::Param2
            | Attribute::Param3
            | Attribute::Param4
            | Attribute::Param5
            | Attribute::Param6
            | Attribute::Param7
            | Attribute::Param8
            | Attribute::Param9
            | Attribute::Param10
            | Attribute::Param11
            | Attribute::Param12
            | Attribute::Param13
            | Attribute::Param14
            | Attribute::Param15
            | Attribute::Param16
            | Attribute::Param17
            | Attribute::Param18
            | Attribute::Param19
            | Attribute::Param20
            | Attribute::Param21
            | Attribute::Param22
            | Attribute::Param23
            | Attribute::Param24
            | Attribute::Param25
            | Attribute::Param26
            | Attribute::Param27
            | Attribute::Param28
            | Attribute::Param29
            | Attribute::Param30
            | Attribute::Param31
    )
}

fn is_mrt(attr: Attribute) -> bool {
    matches!(
        attr,
        Attribute::RenderTarget0
            | Attribute::RenderTarget1
            | Attribute::RenderTarget2
            | Attribute::RenderTarget3
            | Attribute::RenderTarget4
            | Attribute::RenderTarget5
            | Attribute::RenderTarget6
            | Attribute::RenderTarget7
    )
}

fn is_barycentric_coord(attr: Attribute) -> bool {
    matches!(
        attr,
        Attribute::BaryCoordNoPersp
            | Attribute::BaryCoordNoPerspCentroid
            | Attribute::BaryCoordNoPerspSample
            | Attribute::BaryCoordSmooth
            | Attribute::BaryCoordSmoothCentroid
            | Attribute::BaryCoordSmoothSample
    )
}

fn output_attr_component_type(ctx: &mut EmitContext, attr: Attribute) -> (Id, bool) {
    let types = ctx.type_ids();
    if is_param(attr) {
        let index = attr as u64 - Attribute::Param0 as u64;
        let info = ctx.output_params[index as usize];
        return (info.component_type, info.is_integer);
    }
    if is_mrt(attr) {
        let index = attr as u64 - Attribute::RenderTarget0 as u64;
        let info = ctx.frag_outputs[index as usize];
        return (info.component_type, info.is_integer);
    }
    match attr {
        Attribute::Position0
        | Attribute::ClipDistance
        | Attribute::CullDistance
        | Attribute::Depth
        | Attribute::PointSize => (types.F32[1], false),
        Attribute::RenderTargetIndex | Attribute::ViewportIndex | Attribute::SampleMask => {
            (types.U32[1], true)
        }
        Attribute::StencilRef => (types.S32[1], true),
        other => unreachable!("Write attribute {other:?}"),
    }
}

pub fn emit_get_user_data(ctx: &mut EmitContext, reg: ScalarReg) -> Id {
    let types = ctx.type_ids();
    let index = ctx.binding.user_data + ctx.info.borrow().persistent.ud_mask.index(reg);
    let half = PushData::UD_REGS_INDEX + (index >> 2);
    let pointer_type = ctx.TypePointer(spv::StorageClass::PushConstant, types.U32[1]);
    let push = types.push_data_block;
    let half = ctx.ConstU32(half);
    let component = ctx.ConstU32(index & 3);
    let ud_ptr = ctx.OpAccessChain(pointer_type, push, &[half, component]);
    let ud_reg = ctx.OpLoad(types.U32[1], ud_ptr, &[]);
    ctx.Name(ud_reg, &format!("ud_{}", reg as u32));
    ud_reg
}

pub fn emit_read_const(ctx: &mut EmitContext, inst: &InstRef, addr: Id, offset: Id) -> Id {
    let flatbuf_off_dw = inst.borrow().flags_u32();
    if !crate::gpu::shim::core::emulator_settings::emulator_settings()
        .is_direct_memory_access_enabled()
    {
        let index = ctx.ConstU32(flatbuf_off_dw);
        return ctx.EmitFlatbufferLoad(index);
    }
    if flatbuf_off_dw == 0 {
        let types = ctx.type_ids();
        let function = types.read_const_dynamic;
        ctx.OpFunctionCall(types.U32[1], function, &[addr, offset])
    } else {
        let index = ctx.ConstU32(flatbuf_off_dw);
        ctx.EmitFlatbufferLoad(index)
    }
}

pub fn emit_read_const_buffer(ctx: &mut EmitContext, handle: u32, index: Id) -> Id {
    let types = ctx.type_ids();
    let buffer = ctx.buffers[handle as usize].clone();
    let index = if ValidId(buffer.offset(PointerSize::B32)) {
        let offset = buffer.offset(PointerSize::B32);
        ctx.OpIAdd(types.U32[1], index, offset)
    } else {
        index
    };
    let alias = buffer.alias(PointerType::U32);
    let zero = types.u32_zero_value;
    let ptr = ctx.OpAccessChain(alias.pointer_type, alias.id, &[zero, index]);
    ctx.OpLoad(types.U32[1], ptr, &[])
}

pub fn emit_get_attribute(ctx: &mut EmitContext, attr: Attribute, comp: u32, index: u32) -> Id {
    let types = ctx.type_ids();
    if is_param(attr) {
        let param_index = attr as u64 - Attribute::Param0 as u64;
        let param = ctx.input_params[param_index as usize];
        let value = if param.is_array {
            assert!(param.num_components > 1);
            if param.is_loaded {
                ctx.OpCompositeExtract(
                    param.component_type,
                    param.id_array[index as usize],
                    &[comp],
                )
            } else {
                let index = ctx.ConstU32(index);
                let comp = ctx.ConstU32(comp);
                let ptr = ctx.OpAccessChain(param.pointer_type, param.id(), &[index, comp]);
                ctx.OpLoad(param.component_type, ptr, &[])
            }
        } else {
            assert!(!param.is_loaded);
            if param.num_components > 1 {
                let comp = ctx.ConstU32(comp);
                let ptr = ctx.OpAccessChain(param.pointer_type, param.id(), &[comp]);
                ctx.OpLoad(param.component_type, ptr, &[])
            } else {
                ctx.OpLoad(param.component_type, param.id(), &[])
            }
        };
        return if param.is_integer {
            ctx.OpBitcast(types.F32[1], value)
        } else {
            value
        };
    }
    let mut comp = comp;
    if is_barycentric_coord(attr)
        && attr != Attribute::BaryCoordPullModel
        && ctx.profile.supports_fragment_shader_barycentric
    {
        comp += 1;
    }
    match attr {
        Attribute::Position0 => {
            assert_eq!(ctx.sw_stage, SwStage::Geometry);
            let index = ctx.ConstU32(index);
            let zero = ctx.ConstU32(0);
            let comp = ctx.ConstU32(comp);
            let ptr = ctx.OpAccessChain(types.input_f32, types.gl_in, &[index, zero, comp]);
            ctx.OpLoad(types.F32[1], ptr, &[])
        }
        Attribute::FragCoord => {
            let comp = ctx.ConstU32(comp);
            let ptr = ctx.OpAccessChain(types.input_f32, types.frag_coord, &[comp]);
            let value = ctx.OpLoad(types.F32[1], ptr, &[]);
            if comp >= 2 {
                return value;
            }
            let pointer_type = ctx.TypePointer(spv::StorageClass::PushConstant, types.U32[1]);
            let push = types.push_data_block;
            let scene_index = ctx.ConstU32(PushData::SCENE_SIZE_INDEX);
            let ptr = ctx.OpAccessChain(pointer_type, push, &[scene_index]);
            let packed = ctx.OpLoad(types.U32[1], ptr, &[]);
            let dimension = if comp == 0 {
                let mask = ctx.ConstU32(65535);
                ctx.OpBitwiseAnd(types.U32[1], packed, mask)
            } else {
                let shift = ctx.ConstU32(16);
                ctx.OpShiftRightLogical(types.U32[1], packed, shift)
            };
            let native = ctx.ConstU32(if comp == 0 { 1920 } else { 1080 });
            let zero = types.u32_zero_value;
            let is_zero = ctx.OpIEqual(types.U1[1], dimension, zero);
            let safe = ctx.OpSelect(types.U32[1], is_zero, native, dimension);
            let native_f = ctx.OpConvertUToF(types.F32[1], native);
            let safe_f = ctx.OpConvertUToF(types.F32[1], safe);
            let scale = ctx.OpFDiv(types.F32[1], native_f, safe_f);
            ctx.OpFMul(types.F32[1], value, scale)
        }
        Attribute::TessellationEvaluationPointU => {
            let zero = types.u32_zero_value;
            let ptr = ctx.OpAccessChain(types.input_f32, types.tess_coord, &[zero]);
            ctx.OpLoad(types.F32[1], ptr, &[])
        }
        Attribute::TessellationEvaluationPointV => {
            let one = ctx.ConstU32(1);
            let ptr = ctx.OpAccessChain(types.input_f32, types.tess_coord, &[one]);
            ctx.OpLoad(types.F32[1], ptr, &[])
        }
        Attribute::BaryCoordSmooth => {
            if ctx.profile.supports_amd_shader_explicit_vertex_parameter {
                let comp = ctx.ConstU32(comp);
                let ptr = ctx.OpAccessChain(types.input_f32, types.bary_coord_smooth, &[comp]);
                ctx.OpLoad(types.F32[1], ptr, &[])
            } else {
                let value = ctx.OpLoad(types.F32[3], types.bary_coord, &[]);
                ctx.OpCompositeExtract(types.F32[1], value, &[comp])
            }
        }
        Attribute::BaryCoordSmoothCentroid => {
            if ctx.profile.supports_amd_shader_explicit_vertex_parameter {
                let comp = ctx.ConstU32(comp);
                let ptr =
                    ctx.OpAccessChain(types.input_f32, types.bary_coord_smooth_centroid, &[comp]);
                ctx.OpLoad(types.F32[1], ptr, &[])
            } else {
                let value = ctx.OpInterpolateAtCentroid(types.F32[3], types.bary_coord);
                ctx.OpCompositeExtract(types.F32[1], value, &[comp])
            }
        }
        Attribute::BaryCoordSmoothSample => {
            if ctx.profile.supports_amd_shader_explicit_vertex_parameter {
                let comp = ctx.ConstU32(comp);
                let ptr =
                    ctx.OpAccessChain(types.input_f32, types.bary_coord_smooth_sample, &[comp]);
                ctx.OpLoad(types.F32[1], ptr, &[])
            } else {
                let sample = ctx.OpLoad(types.U32[1], types.sample_index, &[]);
                let value = ctx.OpInterpolateAtSample(types.F32[3], types.bary_coord, sample);
                ctx.OpCompositeExtract(types.F32[1], value, &[comp])
            }
        }
        Attribute::BaryCoordNoPersp => {
            let comp = ctx.ConstU32(comp);
            let ptr = ctx.OpAccessChain(types.input_f32, types.bary_coord_nopersp, &[comp]);
            ctx.OpLoad(types.F32[1], ptr, &[])
        }
        Attribute::BaryCoordNoPerspSample => {
            let comp = ctx.ConstU32(comp);
            let ptr = ctx.OpAccessChain(types.input_f32, types.bary_coord_nopersp_sample, &[comp]);
            ctx.OpLoad(types.F32[1], ptr, &[])
        }
        Attribute::BaryCoordPullModel => {
            assert!(comp < 3, "Invalid BaryCoordPullModel component {comp}");
            if ctx.profile.supports_amd_shader_explicit_vertex_parameter {
                let comp = ctx.ConstU32(comp);
                let ptr = ctx.OpAccessChain(types.input_f32, types.bary_coord_pull_model, &[comp]);
                return ctx.OpLoad(types.F32[1], ptr, &[]);
            }
            let three = ctx.ConstU32(3);
            let ptr = ctx.OpAccessChain(types.input_f32, types.frag_coord, &[three]);
            let inv_w = ctx.OpLoad(types.F32[1], ptr, &[]);
            if comp == 2 {
                return inv_w;
            }
            let value = ctx.OpLoad(types.F32[3], types.bary_coord, &[]);
            let ij = ctx.OpCompositeExtract(types.F32[1], value, &[comp + 1]);
            ctx.OpFMul(types.F32[1], ij, inv_w)
        }
        other => unreachable!("Read attribute {other:?}"),
    }
}

pub fn emit_get_attribute_u1(ctx: &mut EmitContext, attr: Attribute, comp: u32) -> Id {
    assert_eq!(comp, 0);
    let types = ctx.type_ids();
    match attr {
        Attribute::IsFrontFace => ctx.OpLoad(types.U1[1], types.front_facing, &[]),
        Attribute::IsHelperInvocation => ctx.OpLoad(types.U1[1], types.helper_invocation, &[]),
        other => unreachable!("Unsupported U1 attribute {other:?}"),
    }
}

pub fn emit_get_attribute_u32(ctx: &mut EmitContext, attr: Attribute, comp: u32) -> Id {
    let types = ctx.type_ids();
    match attr {
        Attribute::VertexId => ctx.OpLoad(types.U32[1], types.vertex_index, &[]),
        Attribute::InstanceId => ctx.OpLoad(types.U32[1], types.instance_id, &[]),
        Attribute::BaseVertex => ctx.OpLoad(types.U32[1], types.base_vertex, &[]),
        Attribute::BaseInstance => ctx.OpLoad(types.U32[1], types.base_instance, &[]),
        Attribute::WorkgroupIndex => types.workgroup_index_id,
        Attribute::WorkgroupId => {
            let value = ctx.OpLoad(types.U32[3], types.workgroup_id, &[]);
            ctx.OpCompositeExtract(types.U32[1], value, &[comp])
        }
        Attribute::LocalInvocationId => {
            let value = ctx.OpLoad(types.U32[3], types.local_invocation_id, &[]);
            ctx.OpCompositeExtract(types.U32[1], value, &[comp])
        }
        Attribute::LocalInvocationIndex => {
            ctx.OpLoad(types.U32[1], types.local_invocation_index, &[])
        }
        Attribute::SampleIndex => ctx.OpLoad(types.U32[1], types.sample_index, &[]),
        Attribute::SampleMask => {
            let zero = types.u32_zero_value;
            let ptr = ctx.OpAccessChain(types.input_u32, types.sample_mask_in, &[zero]);
            ctx.OpLoad(types.U32[1], ptr, &[])
        }
        Attribute::RenderTargetIndex => ctx.OpLoad(types.U32[1], types.output_layer, &[]),
        Attribute::PrimitiveId => ctx.OpLoad(types.U32[1], types.primitive_id, &[]),
        Attribute::InvocationId => {
            let stage = ctx.info.borrow().persistent.sw_stage;
            assert!(stage == SwStage::Geometry || stage == SwStage::TessellationControl);
            ctx.OpLoad(types.U32[1], types.invocation_id, &[])
        }
        Attribute::SubgroupLtMask => {
            let comp = ctx.ConstU32(comp);
            let ptr = ctx.OpAccessChain(types.input_u32, types.subgroup_lt_mask, &[comp]);
            ctx.OpLoad(types.U32[1], ptr, &[])
        }
        Attribute::PatchVertices => {
            let stage = ctx.info.borrow().persistent.sw_stage;
            assert_eq!(stage, SwStage::TessellationControl);
            ctx.OpLoad(types.U32[1], types.patch_vertices, &[])
        }
        Attribute::PackedHullInvocationInfo => {
            let stage = ctx.info.borrow().persistent.sw_stage;
            assert_eq!(stage, SwStage::TessellationControl);
            let tcs = match ctx.runtime_info.sw {
                crate::gpu::shader_recompiler::runtime_info::SwInfo::Tcs(tcs) => tcs,
                _ => unreachable!(),
            };
            if tcs.is_passthrough() {
                types.u32_zero_value
            } else {
                let invocation = ctx.OpLoad(types.U32[1], types.invocation_id, &[]);
                let shift = ctx.ConstU32(8);
                ctx.OpShiftLeftLogical(types.U32[1], invocation, shift)
            }
        }
        other => unreachable!("Read U32 attribute {other:?}"),
    }
}

fn store_component(ctx: &mut EmitContext, attr: Attribute, pointer: Id, value: Id) {
    let (component_type, is_integer) = output_attr_component_type(ctx, attr);
    if is_integer {
        let value = ctx.OpBitcast(component_type, value);
        ctx.OpStore(pointer, value, &[]);
    } else {
        ctx.OpStore(pointer, value, &[]);
    }
}

pub fn emit_set_attribute(ctx: &mut EmitContext, attr: Attribute, value: Id, element: u32) {
    let types = ctx.type_ids();
    if is_param(attr) {
        let attr_index = attr as u64 - Attribute::Param0 as u64;
        if ctx.hw_stage == HwStage::Local {
            let component_ptr = ctx.TypePointer(spv::StorageClass::Output, types.F32[1]);
            let attr_index = ctx.ConstU32(attr_index as u32);
            let element = ctx.ConstU32(element);
            let pointer = ctx.OpAccessChain(
                component_ptr,
                types.output_attr_array,
                &[attr_index, element],
            );
            store_component(ctx, attr, pointer, value);
        } else {
            let info = ctx.output_params[attr_index as usize];
            assert!(info.num_components > 0);
            if info.num_components == 1 {
                store_component(ctx, attr, info.id(), value);
            } else {
                let element = ctx.ConstU32(element);
                let pointer = ctx.OpAccessChain(info.pointer_type, info.id(), &[element]);
                store_component(ctx, attr, pointer, value);
            }
        }
        return;
    }
    if is_mrt(attr) {
        let index = (attr as u64 - Attribute::RenderTarget0 as u64) as usize;
        let info = ctx.frag_outputs[index];
        let fs = match ctx.runtime_info.hw {
            crate::gpu::shader_recompiler::runtime_info::HwInfo::Fs(fs) => fs,
            _ => unreachable!(),
        };
        let value = if element < 3 && fs.color_buffers[index].blend_self_scale {
            ctx.OpFMul(types.F32[1], value, value)
        } else {
            value
        };
        if info.num_components == 1 {
            store_component(ctx, attr, info.id(), value);
        } else {
            let element = ctx.ConstU32(element);
            let pointer = ctx.OpAccessChain(info.pointer_type, info.id(), &[element]);
            store_component(ctx, attr, pointer, value);
        }
        return;
    }
    match attr {
        Attribute::Position0 => {
            let element = ctx.ConstU32(element);
            let pointer = ctx.OpAccessChain(types.output_f32, types.output_position, &[element]);
            store_component(ctx, attr, pointer, value);
        }
        Attribute::ClipDistance => {
            let element = ctx.ConstU32(element);
            let pointer = ctx.OpAccessChain(types.output_f32, types.clip_distances, &[element]);
            store_component(ctx, attr, pointer, value);
        }
        Attribute::CullDistance => {
            let element = ctx.ConstU32(element);
            let pointer = ctx.OpAccessChain(types.output_f32, types.cull_distances, &[element]);
            store_component(ctx, attr, pointer, value);
        }
        Attribute::PointSize => store_component(ctx, attr, types.output_point_size, value),
        Attribute::RenderTargetIndex => store_component(ctx, attr, types.output_layer, value),
        Attribute::ViewportIndex => store_component(ctx, attr, types.output_viewport_index, value),
        Attribute::Depth => store_component(ctx, attr, types.frag_depth, value),
        Attribute::SampleMask => {
            let zero = types.u32_zero_value;
            let pointer = ctx.OpAccessChain(types.output_u32, types.sample_mask, &[zero]);
            store_component(ctx, attr, pointer, value);
        }
        Attribute::StencilRef => {
            if ctx.profile.supports_shader_stencil_export {
                store_component(ctx, attr, types.stencil_ref, value);
            }
        }
        other => unreachable!("Write attribute {other:?}"),
    }
}

pub fn emit_get_tess_generic_attribute(
    ctx: &mut EmitContext,
    vertex_index: Id,
    attr_index: Id,
    comp_index: Id,
) -> Id {
    let types = ctx.type_ids();
    let attr_comp_ptr = ctx.TypePointer(spv::StorageClass::Input, types.F32[1]);
    let pointer = ctx.OpAccessChain(
        attr_comp_ptr,
        types.input_attr_array,
        &[vertex_index, attr_index, comp_index],
    );
    ctx.OpLoad(types.F32[1], pointer, &[])
}

pub fn emit_read_tcs_generic_ouput_attribute(
    ctx: &mut EmitContext,
    vertex_index: Id,
    attr_index: Id,
    comp_index: Id,
) -> Id {
    let types = ctx.type_ids();
    let attr_comp_ptr = ctx.TypePointer(spv::StorageClass::Output, types.F32[1]);
    let pointer = ctx.OpAccessChain(
        attr_comp_ptr,
        types.output_attr_array,
        &[vertex_index, attr_index, comp_index],
    );
    ctx.OpLoad(types.F32[1], pointer, &[])
}

pub fn emit_set_tcs_generic_attribute(
    ctx: &mut EmitContext,
    value: Id,
    attr_index: Id,
    comp_index: Id,
) {
    let types = ctx.type_ids();
    let component_ptr = ctx.TypePointer(spv::StorageClass::Output, types.F32[1]);
    let invocation = ctx.OpLoad(types.U32[1], types.invocation_id, &[]);
    let pointer = ctx.OpAccessChain(
        component_ptr,
        types.output_attr_array,
        &[invocation, attr_index, comp_index],
    );
    ctx.OpStore(pointer, value, &[]);
}

pub fn emit_get_patch(ctx: &mut EmitContext, patch: Patch) -> Id {
    let types = ctx.type_ids();
    let index = patch.generic_index();
    let element = ctx.ConstU32(patch.generic_element());
    let ty = if ctx.sw_stage == SwStage::TessellationControl {
        types.output_f32
    } else {
        types.input_f32
    };
    let target = ctx.patches[index as usize];
    let pointer = ctx.OpAccessChain(ty, target, &[element]);
    ctx.OpLoad(types.F32[1], pointer, &[])
}

pub fn emit_set_patch(ctx: &mut EmitContext, patch: Patch, value: Id) {
    let types = ctx.type_ids();
    let pointer = if patch.is_generic() {
        let index = patch.generic_index();
        let element = ctx.ConstU32(patch.generic_element());
        let target = ctx.patches[index as usize];
        ctx.OpAccessChain(types.output_f32, target, &[element])
    } else {
        match patch {
            Patch::TESSELLATION_LOD_LEFT
            | Patch::TESSELLATION_LOD_RIGHT
            | Patch::TESSELLATION_LOD_TOP
            | Patch::TESSELLATION_LOD_BOTTOM => {
                let index = patch.value() as u32 - Patch::TESSELLATION_LOD_LEFT.value() as u32;
                let index = ctx.ConstU32(index);
                ctx.OpAccessChain(types.output_f32, types.output_tess_level_outer, &[index])
            }
            Patch::TESSELLATION_LOD_INTERIOR_U => {
                let zero = types.u32_zero_value;
                ctx.OpAccessChain(types.output_f32, types.output_tess_level_inner, &[zero])
            }
            Patch::TESSELLATION_LOD_INTERIOR_V => {
                let one = ctx.ConstU32(1);
                ctx.OpAccessChain(types.output_f32, types.output_tess_level_inner, &[one])
            }
            other => unreachable!("Patch {}", other.value()),
        }
    };
    ctx.OpStore(pointer, value, &[]);
}

fn load_buffer_b32x_n(
    ctx: &mut EmitContext,
    handle: u32,
    address: Id,
    count: u32,
    alias: PointerType,
    is_float: bool,
) -> Id {
    let types = ctx.type_ids();
    let spv_buffer = ctx.buffers[handle as usize].clone();
    let address = if ValidId(spv_buffer.offset(PointerSize::B32)) {
        let offset = spv_buffer.offset(PointerSize::B32);
        ctx.OpIAdd(types.U32[1], address, offset)
    } else {
        address
    };
    let data_types = if is_float { types.F32 } else { types.U32 };
    let spv_alias = spv_buffer.alias(alias);
    let mut ids = Vec::with_capacity(count as usize);
    for index in 0..count {
        let index_id = if index == 0 {
            address
        } else {
            let offset = ctx.ConstU32(index);
            ctx.OpIAdd(types.U32[1], address, offset)
        };
        let zero = types.u32_zero_value;
        let ptr = ctx.OpAccessChain(spv_alias.pointer_type, spv_alias.id, &[zero, index_id]);
        ids.push(ctx.OpLoad(data_types[1], ptr, &[]));
    }
    if count == 1 {
        ids[0]
    } else {
        ctx.OpCompositeConstruct(data_types[count as usize], &ids)
    }
}

pub fn emit_load_buffer_u8(ctx: &mut EmitContext, _inst: &InstRef, handle: u32, address: Id) -> Id {
    let types = ctx.type_ids();
    let spv_buffer = ctx.buffers[handle as usize].clone();
    let address = if ValidId(spv_buffer.offset(PointerSize::B8)) {
        let offset = spv_buffer.offset(PointerSize::B8);
        ctx.OpIAdd(types.U32[1], address, offset)
    } else {
        address
    };
    let alias = spv_buffer.alias(PointerType::U8);
    let zero = types.u32_zero_value;
    let ptr = ctx.OpAccessChain(alias.pointer_type, alias.id, &[zero, address]);
    ctx.OpLoad(types.U8, ptr, &[])
}

pub fn emit_load_buffer_u16(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
) -> Id {
    let types = ctx.type_ids();
    let spv_buffer = ctx.buffers[handle as usize].clone();
    let address = if ValidId(spv_buffer.offset(PointerSize::B16)) {
        let offset = spv_buffer.offset(PointerSize::B16);
        ctx.OpIAdd(types.U32[1], address, offset)
    } else {
        address
    };
    let alias = spv_buffer.alias(PointerType::U16);
    let zero = types.u32_zero_value;
    let ptr = ctx.OpAccessChain(alias.pointer_type, alias.id, &[zero, address]);
    ctx.OpLoad(types.U16, ptr, &[])
}

macro_rules! load_buffer_ops {
    ($(($name:ident, $count:expr, $alias:expr, $is_float:expr)),* $(,)?) => {
        $(
            pub fn $name(
                ctx: &mut EmitContext,
                _inst: &InstRef,
                handle: u32,
                address: Id,
            ) -> Id {
                load_buffer_b32x_n(ctx, handle, address, $count, $alias, $is_float)
            }
        )*
    };
}

load_buffer_ops! {
    (emit_load_buffer_u32, 1, PointerType::U32, false),
    (emit_load_buffer_u32x2, 2, PointerType::U32, false),
    (emit_load_buffer_u32x3, 3, PointerType::U32, false),
    (emit_load_buffer_u32x4, 4, PointerType::U32, false),
    (emit_load_buffer_f32, 1, PointerType::F32, true),
    (emit_load_buffer_f32x2, 2, PointerType::F32, true),
    (emit_load_buffer_f32x3, 3, PointerType::F32, true),
    (emit_load_buffer_f32x4, 4, PointerType::F32, true),
}

pub fn emit_load_buffer_u64(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
) -> Id {
    let types = ctx.type_ids();
    let spv_buffer = ctx.buffers[handle as usize].clone();
    let address = if ValidId(spv_buffer.offset(PointerSize::B64)) {
        let offset = spv_buffer.offset(PointerSize::B64);
        ctx.OpIAdd(types.U32[1], address, offset)
    } else {
        address
    };
    let alias = spv_buffer.alias(PointerType::U64);
    let zero = types.u64_zero_value;
    let ptr = ctx.OpAccessChain(alias.pointer_type, alias.id, &[zero, address]);
    ctx.OpLoad(types.U64, ptr, &[])
}

pub fn emit_load_buffer_format_f32(
    _ctx: &mut EmitContext,
    _inst: &InstRef,
    _handle: u32,
    _address: Id,
) -> Id {
    unreachable!("SPIR-V instruction")
}

fn store_buffer_b32x_n(
    ctx: &mut EmitContext,
    handle: u32,
    address: Id,
    value: Id,
    count: u32,
    alias: PointerType,
    is_float: bool,
) {
    let types = ctx.type_ids();
    let spv_buffer = ctx.buffers[handle as usize].clone();
    let address = if ValidId(spv_buffer.offset(PointerSize::B32)) {
        let offset = spv_buffer.offset(PointerSize::B32);
        ctx.OpIAdd(types.U32[1], address, offset)
    } else {
        address
    };
    let data_types = if is_float { types.F32 } else { types.U32 };
    let spv_alias = spv_buffer.alias(alias);
    for index in 0..count {
        let index_id = if index == 0 {
            address
        } else {
            let offset = ctx.ConstU32(index);
            ctx.OpIAdd(types.U32[1], address, offset)
        };
        let zero = types.u32_zero_value;
        let ptr = ctx.OpAccessChain(spv_alias.pointer_type, spv_alias.id, &[zero, index_id]);
        let value_i = if count == 1 {
            value
        } else {
            ctx.OpCompositeExtract(data_types[1], value, &[index])
        };
        ctx.OpStore(ptr, value_i, &[]);
    }
}

pub fn emit_store_buffer_u8(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) {
    let types = ctx.type_ids();
    let spv_buffer = ctx.buffers[handle as usize].clone();
    let address = if ValidId(spv_buffer.offset(PointerSize::B8)) {
        let offset = spv_buffer.offset(PointerSize::B8);
        ctx.OpIAdd(types.U32[1], address, offset)
    } else {
        address
    };
    let alias = spv_buffer.alias(PointerType::U8);
    let zero = types.u32_zero_value;
    let ptr = ctx.OpAccessChain(alias.pointer_type, alias.id, &[zero, address]);
    ctx.OpStore(ptr, value, &[]);
}

pub fn emit_store_buffer_u16(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) {
    let types = ctx.type_ids();
    let spv_buffer = ctx.buffers[handle as usize].clone();
    let address = if ValidId(spv_buffer.offset(PointerSize::B16)) {
        let offset = spv_buffer.offset(PointerSize::B16);
        ctx.OpIAdd(types.U32[1], address, offset)
    } else {
        address
    };
    let alias = spv_buffer.alias(PointerType::U16);
    let zero = types.u32_zero_value;
    let ptr = ctx.OpAccessChain(alias.pointer_type, alias.id, &[zero, address]);
    ctx.OpStore(ptr, value, &[]);
}

macro_rules! store_buffer_ops {
    ($(($name:ident, $count:expr, $alias:expr, $is_float:expr)),* $(,)?) => {
        $(
            pub fn $name(
                ctx: &mut EmitContext,
                _inst: &InstRef,
                handle: u32,
                address: Id,
                value: Id,
            ) {
                store_buffer_b32x_n(
                    ctx, handle, address, value, $count, $alias, $is_float,
                );
            }
        )*
    };
}

store_buffer_ops! {
    (emit_store_buffer_u32, 1, PointerType::U32, false),
    (emit_store_buffer_u32x2, 2, PointerType::U32, false),
    (emit_store_buffer_u32x3, 3, PointerType::U32, false),
    (emit_store_buffer_u32x4, 4, PointerType::U32, false),
    (emit_store_buffer_f32, 1, PointerType::F32, true),
    (emit_store_buffer_f32x2, 2, PointerType::F32, true),
    (emit_store_buffer_f32x3, 3, PointerType::F32, true),
    (emit_store_buffer_f32x4, 4, PointerType::F32, true),
}

pub fn emit_store_buffer_u64(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    address: Id,
    value: Id,
) {
    let types = ctx.type_ids();
    let spv_buffer = ctx.buffers[handle as usize].clone();
    let address = if ValidId(spv_buffer.offset(PointerSize::B64)) {
        let offset = spv_buffer.offset(PointerSize::B64);
        ctx.OpIAdd(types.U32[1], address, offset)
    } else {
        address
    };
    let alias = spv_buffer.alias(PointerType::U64);
    let zero = types.u64_zero_value;
    let ptr = ctx.OpAccessChain(alias.pointer_type, alias.id, &[zero, address]);
    ctx.OpStore(ptr, value, &[]);
}

pub fn emit_store_buffer_format_f32(
    _ctx: &mut EmitContext,
    _inst: &InstRef,
    _handle: u32,
    _address: Id,
    _value: Id,
) {
    unreachable!("SPIR-V instruction")
}

pub fn emit_get_scalar_register(_ctx: &mut EmitContext) {
    unreachable!("Unreachable instruction")
}

pub fn emit_set_scalar_register(_ctx: &mut EmitContext) {
    unreachable!("Unreachable instruction")
}

pub fn emit_get_vector_register(_ctx: &mut EmitContext) {
    unreachable!("Unreachable instruction")
}

pub fn emit_set_vector_register(_ctx: &mut EmitContext) {
    unreachable!("Unreachable instruction")
}

pub fn emit_get_virtual_register(_ctx: &mut EmitContext) {
    unreachable!("Unreachable instruction")
}

pub fn emit_set_virtual_register(_ctx: &mut EmitContext) {
    unreachable!("Unreachable instruction")
}

pub fn emit_get_goto_variable(_ctx: &mut EmitContext) {
    unreachable!("Unreachable instruction")
}

pub fn emit_set_goto_variable(_ctx: &mut EmitContext) {
    unreachable!("Unreachable instruction")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::backend::bindings::Bindings;

    fn opcode(word: u32) -> u32 {
        word & 0xffff
    }

    #[test]
    fn a_vertex_id_attribute_loads_the_input() {
        let read = |_| 0u32;
        let mut binding = Bindings::default();
        let mut ctx = super::super::emit_context::test_fragment_context(&mut binding, &read);
        let value = emit_get_attribute_u32(&mut ctx, Attribute::VertexId, 0);
        assert_ne!(value, 0);
        let words = ctx.Assemble();
        let load_at = words
            .windows(4)
            .position(|window| opcode(window[0]) == spv::Op::Load && window[1] == ctx.U32[1])
            .expect("an input load");
        assert_eq!(words[load_at + 3], ctx.vertex_index);
    }

    #[test]
    fn a_vertex_id_and_an_instance_id_are_distinct_inputs() {
        let read = |_| 0u32;
        let mut binding = Bindings::default();
        let mut ctx = super::super::emit_context::test_vertex_context(&mut binding, &read);
        assert_ne!(ctx.vertex_index, ctx.instance_id);
        let value = emit_get_attribute_u32(&mut ctx, Attribute::InstanceId, 0);
        assert_ne!(value, 0);
    }
}
