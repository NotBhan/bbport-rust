//! The SPIR-V module driver and IR dispatcher, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv.cpp` and `emit_spirv.h`.
//!
//! The C++ dispatches with `opcodes.inc` and template argument conversion; the port keeps the
//! same emitter calls as one explicit match generated from that table.

use super::builder::{Id, ValidId};
use super::emit_context::EmitContext;
use super::emit_spirv_instructions::*;
use super::spv;
use crate::gpu::amdgpu::regs_shader::{FpDenormMode, FpRoundMode};
use crate::gpu::amdgpu::regs_vertex::{
    GsOutputPrimitiveType, PrimitiveType, TessellationPartitioning, TessellationTopology,
    TessellationType,
};
use crate::gpu::shader_recompiler::backend::bindings::Bindings;
use crate::gpu::shader_recompiler::ir::abstract_syntax_list::{NodeData, NodeType};
use crate::gpu::shader_recompiler::ir::attribute::Attribute;
use crate::gpu::shader_recompiler::ir::microinstruction::InstRef;
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::r#type::Type;
use crate::gpu::shader_recompiler::ir::value::Value;
use crate::gpu::shader_recompiler::profile::Profile;
use crate::gpu::shader_recompiler::runtime_info::{HwInfo, HwStage, RuntimeInfo, SwInfo, SwStage};

/// `EmitPhi`.
pub fn emit_phi(ctx: &mut EmitContext, inst: &InstRef) -> Id {
    let num_args = inst.borrow().num_args();
    let mut blocks = Vec::with_capacity(num_args);
    for index in 0..num_args {
        blocks.push(inst.borrow().phi_block(index).borrow().definition_bits());
    }
    let ty = Type::from_bits(inst.borrow().flags_u32());
    let result_type = type_id(ctx, ty);
    ctx.DeferredOpPhi(result_type, &blocks)
}

/// `EmitConditionRef`.
pub fn emit_condition_ref(ctx: &mut EmitContext, value: &Value) -> Id {
    let id = ctx.Def(value);
    assert!(ValidId(id), "Forward identity declaration");
    id
}

/// `EmitGetPcLo`.
pub fn emit_get_pc_lo(ctx: &mut EmitContext, value: &Value) -> Id {
    let id = ctx.Def(value);
    assert!(ValidId(id), "Forward identity declaration");
    id
}

fn type_id(ctx: &EmitContext, ty: Type) -> Id {
    let types = ctx.type_ids();
    match ty {
        Type::U1 => types.U1[1],
        Type::U32 => types.U32[1],
        Type::U32x2 => types.U32[2],
        Type::U32x3 => types.U32[3],
        Type::U32x4 => types.U32[4],
        Type::F32 => types.F32[1],
        Type::U64 => types.U64,
        other => unreachable!("Phi node type {other:?}"),
    }
}

fn traverse(ctx: &mut EmitContext, program: &Program) {
    let mut has_current_block = false;
    for node in &program.syntax_list {
        match node.ty {
            NodeType::Block => {
                let NodeData::Block(block) = &node.data else {
                    unreachable!()
                };
                let label = block.borrow().definition_bits();
                if has_current_block {
                    ctx.OpBranch(label);
                }
                has_current_block = true;
                ctx.AddExistingLabel(label);
                let instructions = block.borrow().instructions().to_vec();
                for inst in &instructions {
                    emit_inst(ctx, inst);
                }
                ctx.first_to_last_label_map.insert(label, ctx.last_label);
            }
            NodeType::If => {
                let NodeData::If { cond, body, merge } = &node.data else {
                    unreachable!()
                };
                let if_label = body.borrow().definition_bits();
                let endif_label = merge.borrow().definition_bits();
                ctx.OpSelectionMerge(endif_label, spv::SelectionControlMask::MaskNone);
                let condition = ctx.Def(cond.value());
                ctx.OpBranchConditional(condition, if_label, endif_label);
            }
            NodeType::Loop => {
                let NodeData::Loop {
                    body,
                    continue_block,
                    merge,
                } = &node.data
                else {
                    unreachable!()
                };
                let body_label = body.borrow().definition_bits();
                let continue_label = continue_block.borrow().definition_bits();
                let endloop_label = merge.borrow().definition_bits();
                ctx.OpLoopMerge(
                    endloop_label,
                    continue_label,
                    spv::LoopControlMask::MaskNone,
                );
                ctx.OpBranch(body_label);
            }
            NodeType::Break => {
                let NodeData::Break { cond, merge, skip } = &node.data else {
                    unreachable!()
                };
                let break_label = merge.borrow().definition_bits();
                let skip_label = skip.borrow().definition_bits();
                let condition = ctx.Def(cond.value());
                ctx.OpBranchConditional(condition, break_label, skip_label);
            }
            NodeType::EndIf => {
                let NodeData::EndIf { merge } = &node.data else {
                    unreachable!()
                };
                if has_current_block {
                    let label = merge.borrow().definition_bits();
                    ctx.OpBranch(label);
                }
            }
            NodeType::Repeat => {
                let NodeData::Repeat {
                    cond,
                    loop_header,
                    merge,
                } = &node.data
                else {
                    unreachable!()
                };
                let condition = ctx.Def(cond.value());
                let loop_header_label = loop_header.borrow().definition_bits();
                let merge_label = merge.borrow().definition_bits();
                ctx.OpBranchConditional(condition, loop_header_label, merge_label);
            }
            NodeType::Return => ctx.OpReturn(),
            NodeType::Unreachable => ctx.OpUnreachable(),
        }
        if node.ty != NodeType::Block {
            has_current_block = false;
        }
    }
}

fn define_main(ctx: &mut EmitContext, program: &Program) -> Id {
    let void_id = ctx.void_id;
    let void_function = ctx.TypeFunction(void_id, &[]);
    let main = ctx.OpFunction(void_id, spv::FunctionControlMask::MaskNone, void_function);
    for block in &program.blocks {
        let label = ctx.OpLabel();
        block.borrow_mut().set_definition_bits(label);
    }
    traverse(ctx, program);
    ctx.OpFunctionEnd();
    main
}

fn input_primitive_type(ty: PrimitiveType) -> u32 {
    match ty {
        PrimitiveType::PointList => spv::ExecutionMode::InputPoints,
        PrimitiveType::LineList | PrimitiveType::LineStrip => spv::ExecutionMode::InputLines,
        PrimitiveType::TriangleList | PrimitiveType::TriangleStrip | PrimitiveType::RectList => {
            spv::ExecutionMode::Triangles
        }
        PrimitiveType::AdjTriangleList => spv::ExecutionMode::InputTrianglesAdjacency,
        PrimitiveType::AdjLineList => spv::ExecutionMode::InputLinesAdjacency,
        other => unreachable!("Unknown input primitive type {other:?}"),
    }
}

fn output_primitive_type(ty: GsOutputPrimitiveType) -> u32 {
    match ty {
        GsOutputPrimitiveType::PointList => spv::ExecutionMode::OutputVertices,
        GsOutputPrimitiveType::LineStrip => spv::ExecutionMode::OutputLineStrip,
        GsOutputPrimitiveType::TriangleStrip => spv::ExecutionMode::OutputTriangleStrip,
    }
}

fn tessellation_mode(ty: TessellationType) -> u32 {
    match ty {
        TessellationType::Isoline => spv::ExecutionMode::Isolines,
        TessellationType::Triangle => spv::ExecutionMode::Triangles,
        TessellationType::Quad => spv::ExecutionMode::Quads,
    }
}

fn tessellation_partitioning(partitioning: TessellationPartitioning) -> u32 {
    match partitioning {
        TessellationPartitioning::Integer => spv::ExecutionMode::SpacingEqual,
        TessellationPartitioning::FracOdd => spv::ExecutionMode::SpacingFractionalOdd,
        TessellationPartitioning::FracEven => spv::ExecutionMode::SpacingFractionalEven,
        TessellationPartitioning::Pow2 => spv::ExecutionMode::SpacingEqual,
    }
}

fn setup_capabilities(ctx: &mut EmitContext, profile: &Profile, runtime_info: &RuntimeInfo) {
    let info = ctx.info.clone();
    let info = info.borrow();
    ctx.AddCapability(spv::Capability::Image1D);
    ctx.AddCapability(spv::Capability::Sampled1D);
    ctx.AddCapability(spv::Capability::ImageQuery);
    ctx.AddCapability(spv::Capability::Int8);
    ctx.AddCapability(spv::Capability::Int16);
    ctx.AddCapability(spv::Capability::Int64);
    ctx.AddCapability(spv::Capability::StorageBuffer8BitAccess);
    ctx.AddCapability(spv::Capability::StorageBuffer16BitAccess);
    if info.uses_fp16 {
        ctx.AddCapability(spv::Capability::Float16);
    }
    if info.uses_fp64 {
        ctx.AddCapability(spv::Capability::Float64);
    }
    if info.has_storage_images {
        ctx.AddCapability(spv::Capability::StorageImageExtendedFormats);
        ctx.AddCapability(spv::Capability::StorageImageReadWithoutFormat);
        ctx.AddCapability(spv::Capability::StorageImageWriteWithoutFormat);
        if profile.supports_image_load_store_lod {
            ctx.AddExtension("SPV_AMD_shader_image_load_store_lod");
            ctx.AddCapability(spv::Capability::ImageReadWriteLodAMD);
        }
    }
    if info.has_image_gather {
        ctx.AddCapability(spv::Capability::ImageGatherExtended);
    }
    if info.has_image_query {
        ctx.AddCapability(spv::Capability::ImageQuery);
    }
    if (info.uses_image_atomic_float_min_max && profile.supports_image_fp32_atomic_min_max)
        || (info.uses_buffer_atomic_float_min_max && profile.supports_buffer_fp32_atomic_min_max)
    {
        ctx.AddExtension("SPV_EXT_shader_atomic_float_min_max");
        ctx.AddCapability(spv::Capability::AtomicFloat32MinMaxEXT);
    }
    if info.uses_lane_id {
        ctx.AddCapability(spv::Capability::GroupNonUniform);
    }
    if info.uses_group_quad {
        ctx.AddCapability(spv::Capability::GroupNonUniformQuad);
    }
    if info.uses_group_shuffle {
        ctx.AddCapability(spv::Capability::GroupNonUniformShuffle);
    }
    if info.uses_group_ballot || info.loads.get(Attribute::SubgroupLtMask, 0) {
        ctx.AddCapability(spv::Capability::GroupNonUniformBallot);
    }
    if info.uses_shader_clock && ctx.profile.supports_shader_subgroup_clock {
        ctx.AddExtension("SPV_KHR_shader_clock");
        ctx.AddCapability(spv::Capability::ShaderClockKHR);
    }
    let stage = info.persistent.sw_stage;
    if stage == SwStage::Vertex {
        ctx.AddExtension("SPV_KHR_shader_draw_parameters");
        ctx.AddCapability(spv::Capability::DrawParameters);
    }
    if stage == SwStage::Geometry {
        ctx.AddCapability(spv::Capability::Geometry);
    }
    if info.hw_stage() == HwStage::Fragment {
        if profile.supports_amd_shader_explicit_vertex_parameter {
            ctx.AddExtension("SPV_AMD_shader_explicit_vertex_parameter");
        } else if profile.supports_fragment_shader_barycentric {
            ctx.AddExtension("SPV_KHR_fragment_shader_barycentric");
            ctx.AddCapability(spv::Capability::FragmentBarycentricKHR);
            ctx.AddCapability(spv::Capability::InterpolationFunction);
        }
        let fs = match &runtime_info.hw {
            HwInfo::Fs(fs) => Some(*fs),
            _ => None,
        };
        if let Some(fs) = fs {
            if info.loads.get(Attribute::SampleIndex, 0)
                || fs.addr_flags.linear_sample_ena()
                || fs.addr_flags.persp_sample_ena()
                || (!profile.supports_amd_shader_explicit_vertex_parameter
                    && profile.supports_fragment_shader_barycentric
                    && info.loads.get(Attribute::BaryCoordSmoothSample, 0))
            {
                ctx.AddCapability(spv::Capability::SampleRateShading);
            }
        }
        if info.loads.get_any(Attribute::RenderTargetIndex) {
            ctx.AddCapability(spv::Capability::Geometry);
        }
        if info.stores.get(Attribute::StencilRef, 0) && profile.supports_shader_stencil_export {
            ctx.AddExtension("SPV_EXT_shader_stencil_export");
            ctx.AddCapability(spv::Capability::StencilExportEXT);
        }
    }
    if stage == SwStage::TessellationControl || stage == SwStage::TessellationEval {
        ctx.AddCapability(spv::Capability::Tessellation);
    }
    if stage == SwStage::Vertex
        || stage == SwStage::TessellationControl
        || stage == SwStage::TessellationEval
    {
        if info.stores.get_any(Attribute::RenderTargetIndex) {
            ctx.AddCapability(spv::Capability::ShaderLayer);
        }
        if info.stores.get_any(Attribute::ViewportIndex) {
            ctx.AddCapability(spv::Capability::ShaderViewportIndex);
        }
    } else if stage == SwStage::Geometry && info.stores.get_any(Attribute::ViewportIndex) {
        ctx.AddCapability(spv::Capability::MultiViewport);
    }
    if info.persistent.uses_dma || ctx.VertexMotion() {
        ctx.AddCapability(spv::Capability::PhysicalStorageBufferAddresses);
        ctx.AddExtension("SPV_KHR_physical_storage_buffer");
    }
    let shared_type_count = info.shared_types.bits().count_ones();
    if shared_type_count > 1 && profile.supports_workgroup_explicit_memory_layout {
        ctx.AddExtension("SPV_KHR_workgroup_memory_explicit_layout");
        ctx.AddCapability(spv::Capability::WorkgroupMemoryExplicitLayoutKHR);
        ctx.AddCapability(spv::Capability::WorkgroupMemoryExplicitLayout16BitAccessKHR);
    }
    if info.uses_buffer_int64_atomics || info.uses_shared_int64_atomics {
        if info.uses_buffer_int64_atomics {
            assert!(
                profile.supports_buffer_int64_atomics,
                "Shader requires support for atomic Int64 buffer operations that your \
                 Vulkan instance does not advertise"
            );
        }
        if info.uses_shared_int64_atomics {
            assert!(
                profile.supports_shared_int64_atomics,
                "Shader requires support for atomic Int64 shared memory operations that \
                 your Vulkan instance does not advertise"
            );
        }
        ctx.AddCapability(spv::Capability::Int64Atomics);
    }
}

fn define_entry_point(ctx: &mut EmitContext, main: Id) {
    let info = ctx.info.clone();
    let info = info.borrow();
    let interfaces = ctx.interfaces.clone();
    let execution_model;
    match info.persistent.sw_stage {
        SwStage::Compute => {
            execution_model = spv::ExecutionModel::GLCompute;
            let cs = match &ctx.runtime_info.hw {
                HwInfo::Cs(cs) => *cs,
                _ => unreachable!(),
            };
            ctx.AddExecutionMode(main, spv::ExecutionMode::LocalSize, &cs.workgroup_size);
        }
        SwStage::Vertex => {
            execution_model = spv::ExecutionModel::Vertex;
        }
        SwStage::TessellationControl => {
            execution_model = spv::ExecutionModel::TessellationControl;
            ctx.AddCapability(spv::Capability::Tessellation);
            let tcs = match &ctx.runtime_info.sw {
                SwInfo::Tcs(tcs) => *tcs,
                _ => unreachable!(),
            };
            ctx.AddExecutionMode(
                main,
                spv::ExecutionMode::OutputVertices,
                &[tcs.num_output_control_points()],
            );
        }
        SwStage::TessellationEval => {
            execution_model = spv::ExecutionModel::TessellationEvaluation;
            let tes = match &ctx.runtime_info.sw {
                SwInfo::Tes(tes) => *tes,
                _ => unreachable!(),
            };
            ctx.AddExecutionMode(main, tessellation_mode(tes.tess_type), &[]);
            ctx.AddExecutionMode(main, tessellation_partitioning(tes.tess_partitioning), &[]);
            let order = if tes.tess_topology == TessellationTopology::TriangleCcw {
                spv::ExecutionMode::VertexOrderCcw
            } else {
                spv::ExecutionMode::VertexOrderCw
            };
            ctx.AddExecutionMode(main, order, &[]);
        }
        SwStage::Fragment => {
            execution_model = spv::ExecutionModel::Fragment;
            let origin = if ctx.profile.lower_left_origin_mode {
                spv::ExecutionMode::OriginLowerLeft
            } else {
                spv::ExecutionMode::OriginUpperLeft
            };
            ctx.AddExecutionMode(main, origin, &[]);
            if info.has_discard {
                ctx.AddCapability(spv::Capability::DemoteToHelperInvocation);
            }
            if info.stores.get_any(Attribute::Depth) {
                ctx.AddExecutionMode(main, spv::ExecutionMode::DepthReplacing, &[]);
            }
        }
        SwStage::Geometry => {
            execution_model = spv::ExecutionModel::Geometry;
            let (in_primitive, out_primitive, output_vertices, num_invocations) =
                match &ctx.runtime_info.hw {
                    HwInfo::Gs(gs) => (
                        gs.in_primitive,
                        gs.out_primitive[0],
                        gs.output_vertices,
                        gs.num_invocations,
                    ),
                    _ => unreachable!(),
                };
            ctx.AddExecutionMode(main, input_primitive_type(in_primitive), &[]);
            ctx.AddExecutionMode(main, output_primitive_type(out_primitive), &[]);
            ctx.AddExecutionMode(main, spv::ExecutionMode::OutputVertices, &[output_vertices]);
            ctx.AddExecutionMode(main, spv::ExecutionMode::Invocations, &[num_invocations]);
        }
        _ => unreachable!("Stage {:?}", info.hw_stage()),
    }
    ctx.AddEntryPoint(execution_model, main, "main", &interfaces);
}

fn setup_denorm_flush_mode(ctx: &mut EmitContext, profile: &Profile, main: Id) {
    let fp32_denorm_mode = ctx.runtime_info.props.fp_denorm_mode32;
    if fp32_denorm_mode == FpDenormMode::InOutFlush && profile.support_fp32_denorm_flush {
        ctx.AddCapability(spv::Capability::DenormFlushToZero);
        ctx.AddExecutionMode(main, spv::ExecutionMode::DenormFlushToZero, &[32]);
    }
    let info = ctx.info.clone();
    let info = info.borrow();
    if !info.uses_fp16 && !info.uses_fp64 {
        return;
    }
    let fp16_64_denorm_mode = ctx.runtime_info.props.fp_denorm_mode16_64;
    let same_denorm_mode = fp16_64_denorm_mode == fp32_denorm_mode;
    if !same_denorm_mode && !profile.supports_denorm_behavior_independence {
        return;
    }
    if fp16_64_denorm_mode == FpDenormMode::InOutFlush {
        if info.uses_fp16 && profile.support_fp16_denorm_flush {
            ctx.AddCapability(spv::Capability::DenormFlushToZero);
            ctx.AddExecutionMode(main, spv::ExecutionMode::DenormFlushToZero, &[16]);
        }
        if info.uses_fp64 && profile.support_fp64_denorm_flush {
            ctx.AddCapability(spv::Capability::DenormFlushToZero);
            ctx.AddExecutionMode(main, spv::ExecutionMode::DenormFlushToZero, &[64]);
        }
    } else if fp16_64_denorm_mode == FpDenormMode::InOutAllow {
        if info.uses_fp16 && profile.support_fp16_denorm_preserve {
            ctx.AddCapability(spv::Capability::DenormPreserve);
            ctx.AddExecutionMode(main, spv::ExecutionMode::DenormPreserve, &[16]);
        }
        if info.uses_fp64 && profile.support_fp64_denorm_preserve {
            ctx.AddCapability(spv::Capability::DenormPreserve);
            ctx.AddExecutionMode(main, spv::ExecutionMode::DenormPreserve, &[64]);
        }
    }
}

fn setup_rounding_mode(ctx: &mut EmitContext, profile: &Profile, main: Id) {
    let fp_round_mode = ctx.runtime_info.props.fp_round_mode32;
    if fp_round_mode == FpRoundMode::ToZero && profile.support_fp32_round_to_zero {
        ctx.AddCapability(spv::Capability::RoundingModeRTZ);
        ctx.AddExecutionMode(main, spv::ExecutionMode::RoundingModeRTZ, &[32]);
    }
    let info = ctx.info.clone();
    let info = info.borrow();
    if info.uses_fp16 || info.uses_fp64 {
        let fp16_64_round_mode = ctx.runtime_info.props.fp_round_mode16_64;
        let same_round_mode = fp16_64_round_mode == fp_round_mode;
        if !same_round_mode && !profile.supports_rounding_mode_independence {
            return;
        }
        if fp16_64_round_mode == FpRoundMode::ToZero {
            if info.uses_fp16 && profile.support_fp16_round_to_zero {
                ctx.AddCapability(spv::Capability::RoundingModeRTZ);
                ctx.AddExecutionMode(main, spv::ExecutionMode::RoundingModeRTZ, &[16]);
            }
            if info.uses_fp64 && profile.support_fp64_round_to_zero {
                ctx.AddCapability(spv::Capability::RoundingModeRTZ);
                ctx.AddExecutionMode(main, spv::ExecutionMode::RoundingModeRTZ, &[64]);
            }
        }
    }
}

fn setup_inf_nan_preserve_mode(ctx: &mut EmitContext, profile: &Profile, main: Id) {
    if profile.support_fp16_signed_zero_inf_nan_preserve
        || profile.support_fp32_signed_zero_inf_nan_preserve
        || profile.support_fp64_signed_zero_inf_nan_preserve
    {
        ctx.AddCapability(spv::Capability::SignedZeroInfNanPreserve);
    }
    if profile.support_fp32_signed_zero_inf_nan_preserve {
        ctx.AddExecutionMode(main, spv::ExecutionMode::SignedZeroInfNanPreserve, &[32]);
    }
    let info = ctx.info.clone();
    let info = info.borrow();
    if info.uses_fp16 && profile.support_fp16_signed_zero_inf_nan_preserve {
        ctx.AddExecutionMode(main, spv::ExecutionMode::SignedZeroInfNanPreserve, &[16]);
    }
    if info.uses_fp64 && profile.support_fp64_signed_zero_inf_nan_preserve {
        ctx.AddExecutionMode(main, spv::ExecutionMode::SignedZeroInfNanPreserve, &[64]);
    }
}

fn setup_float_mode(ctx: &mut EmitContext, profile: &Profile, main: Id) {
    ctx.AddExtension("SPV_KHR_float_controls");
    setup_denorm_flush_mode(ctx, profile, main);
    setup_rounding_mode(ctx, profile, main);
    setup_inf_nan_preserve_mode(ctx, profile, main);
}

fn patch_phi_nodes(program: &Program, ctx: &mut EmitContext) {
    let mut phis: Vec<InstRef> = Vec::new();
    for block in &program.blocks {
        let instructions = block.borrow().instructions().to_vec();
        for inst in instructions {
            if inst.borrow().get_opcode() == Opcode::Phi {
                phis.push(inst);
            }
        }
    }
    let nodes = ctx.module.DeferredPhiNodes().to_vec();
    let mut patches: Vec<Vec<(Id, Id)>> = Vec::with_capacity(nodes.len());
    for (current, &node) in nodes.iter().enumerate() {
        let count = ctx.module.DeferredPhiArgCount(node);
        let mut args = Vec::with_capacity(count);
        for arg in 0..count {
            let inst = phis[current].clone();
            let value = ctx.Def(&inst.borrow().arg(arg));
            let first_parent = ctx.module.DeferredPhiParent(node, arg);
            let parent = *ctx
                .first_to_last_label_map
                .get(&first_parent)
                .expect("the parent label was emitted");
            args.push((value, parent));
        }
        patches.push(args);
    }
    for (&node, args) in nodes.iter().zip(patches.iter()) {
        for (arg, (value, parent)) in args.iter().enumerate() {
            ctx.module.SetDeferredPhiArg(node, arg, *value, *parent);
        }
    }
}

/// `EmitSPIRV`.
pub fn emit_spirv(
    profile: &Profile,
    runtime_info: &RuntimeInfo,
    program: &Program,
    binding: &mut Bindings,
    fetch_code: &[u32],
    read: &dyn Fn(u64) -> u32,
) -> Vec<u32> {
    let mut ctx = EmitContext::new(
        profile,
        runtime_info,
        program.info.clone(),
        binding,
        fetch_code,
        read,
    );
    let main = define_main(&mut ctx, program);
    define_entry_point(&mut ctx, main);
    setup_capabilities(&mut ctx, profile, runtime_info);
    setup_float_mode(&mut ctx, profile, main);
    patch_phi_nodes(program, &mut ctx);
    let user_data_regs = program.info.borrow().persistent.ud_mask.num_regs();
    ctx.binding.user_data += user_data_regs;
    ctx.module.Assemble()
}

fn emit_inst(ctx: &mut EmitContext, inst: &InstRef) {
    let opcode = inst.borrow().get_opcode();
    match opcode {
        Opcode::Phi => {
            let value = emit_phi(ctx, inst);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::Void => {}
        Opcode::ConditionRef => {
            let value = emit_condition_ref(ctx, &inst.borrow().arg(0));
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::Prologue => emit_prologue(ctx),
        Opcode::Epilogue => emit_epilogue(ctx),
        Opcode::Discard => emit_discard(ctx),
        Opcode::DiscardCond => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            emit_discard_cond(ctx, a0);
        }
        Opcode::DebugPrint => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let a3 = ctx.Def(&inst.borrow().arg(3));
            let a4 = ctx.Def(&inst.borrow().arg(4));
            emit_debug_print(ctx, inst, a0, a1, a2, a3, a4);
        }
        Opcode::ReadConst => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_read_const(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ReadConstBuffer => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_read_const_buffer(ctx, inst.borrow().arg(0).u32(), a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::Barrier => emit_barrier(ctx),
        Opcode::WorkgroupMemoryBarrier => emit_workgroup_memory_barrier(ctx),
        Opcode::DeviceMemoryBarrier => emit_device_memory_barrier(ctx),
        Opcode::EmitVertex => emit_emit_vertex(ctx),
        Opcode::EmitPrimitive => emit_emit_primitive(ctx),
        Opcode::LoadSharedU16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_load_shared_u16(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LoadSharedU32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_load_shared_u32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LoadSharedU64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_load_shared_u64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::WriteSharedU16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            emit_write_shared_u16(ctx, a0, a1);
        }
        Opcode::WriteSharedU32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            emit_write_shared_u32(ctx, a0, a1);
        }
        Opcode::WriteSharedU64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            emit_write_shared_u64(ctx, a0, a1);
        }
        Opcode::SharedAtomicIAdd32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_i_add32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicIAdd64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_i_add64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicISub32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_i_sub32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicISub64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_i_sub64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicSMin32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_s_min32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicSMin64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_s_min64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicUMin32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_u_min32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicUMin64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_u_min64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicSMax32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_s_max32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicSMax64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_s_max64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicUMax32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_u_max32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicUMax64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_u_max64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicInc32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_shared_atomic_inc32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicInc64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_shared_atomic_inc64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicDec32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_shared_atomic_dec32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicDec64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_shared_atomic_dec64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicAnd32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_and32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicAnd64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_and64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicOr32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_or32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicOr64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_or64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicXor32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_xor32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicXor64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shared_atomic_xor64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicCmpSwap32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_shared_atomic_cmp_swap32(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SharedAtomicCmpSwap64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_shared_atomic_cmp_swap64(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::GetUserData => {
            let value = emit_get_user_data(ctx, inst.borrow().arg(0).scalar_reg());
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::GetScalarRegister => emit_get_scalar_register(ctx),
        Opcode::SetScalarRegister => emit_set_scalar_register(ctx),
        Opcode::GetVectorRegister => emit_get_vector_register(ctx),
        Opcode::SetVectorRegister => emit_set_vector_register(ctx),
        Opcode::GetVirtualRegister => emit_get_virtual_register(ctx),
        Opcode::SetVirtualRegister => emit_set_virtual_register(ctx),
        Opcode::GetGotoVariable => emit_get_goto_variable(ctx),
        Opcode::SetGotoVariable => emit_set_goto_variable(ctx),
        Opcode::GetAttribute => {
            let value = emit_get_attribute(
                ctx,
                inst.borrow().arg(0).attribute(),
                inst.borrow().arg(1).u32(),
                inst.borrow().arg(2).u32(),
            );
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::GetAttributeU1 => {
            let value = emit_get_attribute_u1(
                ctx,
                inst.borrow().arg(0).attribute(),
                inst.borrow().arg(1).u32(),
            );
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::GetAttributeU32 => {
            let value = emit_get_attribute_u32(
                ctx,
                inst.borrow().arg(0).attribute(),
                inst.borrow().arg(1).u32(),
            );
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SetAttribute => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            emit_set_attribute(
                ctx,
                inst.borrow().arg(0).attribute(),
                a1,
                inst.borrow().arg(2).u32(),
            );
        }
        Opcode::GetPatch => {
            let value = emit_get_patch(ctx, inst.borrow().arg(0).patch());
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SetPatch => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            emit_set_patch(ctx, inst.borrow().arg(0).patch(), a1);
        }
        Opcode::GetTessGenericAttribute => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_get_tess_generic_attribute(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SetTcsGenericAttribute => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            emit_set_tcs_generic_attribute(ctx, a0, a1, a2);
        }
        Opcode::ReadTcsGenericOuputAttribute => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_read_tcs_generic_ouput_attribute(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::GetPcLo => {
            let value = emit_get_pc_lo(ctx, &inst.borrow().arg(0));
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::GetScc => unreachable!("Unreachable instruction"),
        Opcode::GetExec => unreachable!("Unreachable instruction"),
        Opcode::GetVcc => unreachable!("Unreachable instruction"),
        Opcode::GetVccLo => unreachable!("Unreachable instruction"),
        Opcode::GetVccHi => unreachable!("Unreachable instruction"),
        Opcode::GetM0 => unreachable!("Unreachable instruction"),
        Opcode::SetScc => unreachable!("Unreachable instruction"),
        Opcode::SetExec => unreachable!("Unreachable instruction"),
        Opcode::SetVcc => unreachable!("Unreachable instruction"),
        Opcode::SetVccLo => unreachable!("Unreachable instruction"),
        Opcode::SetVccHi => unreachable!("Unreachable instruction"),
        Opcode::SetM0 => unreachable!("Unreachable instruction"),
        Opcode::UndefU1 => {
            let value = emit_undef_u1(ctx);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UndefU8 => {
            let value = emit_undef_u8(ctx);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UndefU16 => {
            let value = emit_undef_u16(ctx);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UndefU32 => {
            let value = emit_undef_u32(ctx);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UndefU32x2 => {
            let value = emit_undef_u32x2(ctx);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UndefU32x3 => {
            let value = emit_undef_u32x3(ctx);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UndefU32x4 => {
            let value = emit_undef_u32x4(ctx);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UndefF32 => {
            let value = emit_undef_f32(ctx);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UndefU64 => {
            let value = emit_undef_u64(ctx);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LoadBufferU8 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_load_buffer_u8(ctx, inst, inst.borrow().arg(0).u32(), a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LoadBufferU16 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_load_buffer_u16(ctx, inst, inst.borrow().arg(0).u32(), a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LoadBufferU32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_load_buffer_u32(ctx, inst, inst.borrow().arg(0).u32(), a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LoadBufferU32x2 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_load_buffer_u32x2(ctx, inst, inst.borrow().arg(0).u32(), a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LoadBufferU32x3 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_load_buffer_u32x3(ctx, inst, inst.borrow().arg(0).u32(), a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LoadBufferU32x4 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_load_buffer_u32x4(ctx, inst, inst.borrow().arg(0).u32(), a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LoadBufferU64 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_load_buffer_u64(ctx, inst, inst.borrow().arg(0).u32(), a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LoadBufferF32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_load_buffer_f32(ctx, inst, inst.borrow().arg(0).u32(), a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LoadBufferF32x2 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_load_buffer_f32x2(ctx, inst, inst.borrow().arg(0).u32(), a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LoadBufferF32x3 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_load_buffer_f32x3(ctx, inst, inst.borrow().arg(0).u32(), a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LoadBufferF32x4 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_load_buffer_f32x4(ctx, inst, inst.borrow().arg(0).u32(), a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LoadBufferFormatF32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_load_buffer_format_f32(ctx, inst, inst.borrow().arg(0).u32(), a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::StoreBufferU8 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            emit_store_buffer_u8(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
        }
        Opcode::StoreBufferU16 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            emit_store_buffer_u16(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
        }
        Opcode::StoreBufferU32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            emit_store_buffer_u32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
        }
        Opcode::StoreBufferU32x2 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            emit_store_buffer_u32x2(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
        }
        Opcode::StoreBufferU32x3 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            emit_store_buffer_u32x3(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
        }
        Opcode::StoreBufferU32x4 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            emit_store_buffer_u32x4(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
        }
        Opcode::StoreBufferU64 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            emit_store_buffer_u64(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
        }
        Opcode::StoreBufferF32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            emit_store_buffer_f32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
        }
        Opcode::StoreBufferF32x2 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            emit_store_buffer_f32x2(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
        }
        Opcode::StoreBufferF32x3 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            emit_store_buffer_f32x3(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
        }
        Opcode::StoreBufferF32x4 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            emit_store_buffer_f32x4(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
        }
        Opcode::StoreBufferFormatF32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            emit_store_buffer_format_f32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
        }
        Opcode::BufferAtomicIAdd32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_buffer_atomic_i_add32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicIAdd64 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_buffer_atomic_i_add64(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicISub32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_buffer_atomic_i_sub32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicSMin32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_buffer_atomic_s_min32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicSMin64 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_buffer_atomic_s_min64(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicUMin32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_buffer_atomic_u_min32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicUMin64 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_buffer_atomic_u_min64(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicFMin32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_buffer_atomic_f_min32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicSMax32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_buffer_atomic_s_max32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicSMax64 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_buffer_atomic_s_max64(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicUMax32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_buffer_atomic_u_max32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicUMax64 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_buffer_atomic_u_max64(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicFMax32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_buffer_atomic_f_max32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicInc32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_buffer_atomic_inc32(ctx, inst, inst.borrow().arg(0).u32(), a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicDec32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_buffer_atomic_dec32(ctx, inst, inst.borrow().arg(0).u32(), a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicAnd32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_buffer_atomic_and32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicOr32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_buffer_atomic_or32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicXor32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_buffer_atomic_xor32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicSwap32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_buffer_atomic_swap32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicCmpSwap32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let a3 = ctx.Def(&inst.borrow().arg(3));
            let value =
                emit_buffer_atomic_cmp_swap32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2, a3);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BufferAtomicFCmpSwap32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let a3 = ctx.Def(&inst.borrow().arg(3));
            let value =
                emit_buffer_atomic_f_cmp_swap32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2, a3);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeConstructU32x2 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_composite_construct_u32x2(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeConstructU32x3 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_composite_construct_u32x3(ctx, inst, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeConstructU32x4 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let a3 = ctx.Def(&inst.borrow().arg(3));
            let value = emit_composite_construct_u32x4(ctx, inst, a0, a1, a2, a3);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeConstructU32x2x2 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_composite_construct_u32x2x2(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeExtractU32x2 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_composite_extract_u32x2(ctx, a0, inst.borrow().arg(1).u32());
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeExtractU32x3 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_composite_extract_u32x3(ctx, a0, inst.borrow().arg(1).u32());
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeExtractU32x4 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_composite_extract_u32x4(ctx, a0, inst.borrow().arg(1).u32());
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeInsertU32x2 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_composite_insert_u32x2(ctx, a0, a1, inst.borrow().arg(2).u32());
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeInsertU32x3 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_composite_insert_u32x3(ctx, a0, a1, inst.borrow().arg(2).u32());
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeInsertU32x4 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_composite_insert_u32x4(ctx, a0, a1, inst.borrow().arg(2).u32());
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeShuffleU32x2 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_composite_shuffle_u32x2(
                ctx,
                a0,
                a1,
                inst.borrow().arg(2).u32(),
                inst.borrow().arg(3).u32(),
            );
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeShuffleU32x3 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_composite_shuffle_u32x3(
                ctx,
                a0,
                a1,
                inst.borrow().arg(2).u32(),
                inst.borrow().arg(3).u32(),
                inst.borrow().arg(4).u32(),
            );
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeShuffleU32x4 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_composite_shuffle_u32x4(
                ctx,
                a0,
                a1,
                inst.borrow().arg(2).u32(),
                inst.borrow().arg(3).u32(),
                inst.borrow().arg(4).u32(),
                inst.borrow().arg(5).u32(),
            );
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeConstructF32x2 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_composite_construct_f32x2(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeConstructF32x3 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_composite_construct_f32x3(ctx, inst, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeConstructF32x4 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let a3 = ctx.Def(&inst.borrow().arg(3));
            let value = emit_composite_construct_f32x4(ctx, inst, a0, a1, a2, a3);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeConstructF32x2x2 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_composite_construct_f32x2x2(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeExtractF32x2 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_composite_extract_f32x2(ctx, a0, inst.borrow().arg(1).u32());
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeExtractF32x3 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_composite_extract_f32x3(ctx, a0, inst.borrow().arg(1).u32());
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeExtractF32x4 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_composite_extract_f32x4(ctx, a0, inst.borrow().arg(1).u32());
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeInsertF32x2 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_composite_insert_f32x2(ctx, a0, a1, inst.borrow().arg(2).u32());
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeInsertF32x3 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_composite_insert_f32x3(ctx, a0, a1, inst.borrow().arg(2).u32());
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeInsertF32x4 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_composite_insert_f32x4(ctx, a0, a1, inst.borrow().arg(2).u32());
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeShuffleF32x2 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_composite_shuffle_f32x2(
                ctx,
                a0,
                a1,
                inst.borrow().arg(2).u32(),
                inst.borrow().arg(3).u32(),
            );
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeShuffleF32x3 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_composite_shuffle_f32x3(
                ctx,
                a0,
                a1,
                inst.borrow().arg(2).u32(),
                inst.borrow().arg(3).u32(),
                inst.borrow().arg(4).u32(),
            );
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CompositeShuffleF32x4 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_composite_shuffle_f32x4(
                ctx,
                a0,
                a1,
                inst.borrow().arg(2).u32(),
                inst.borrow().arg(3).u32(),
                inst.borrow().arg(4).u32(),
                inst.borrow().arg(5).u32(),
            );
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SelectU1 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_select_u1(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SelectU32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_select_u32(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SelectF32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_select_f32(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SelectU64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_select_u64(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitCastU16F16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_bit_cast_u16f16(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitCastU32F32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_bit_cast_u32f32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitCastF16U16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_bit_cast_f16u16(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitCastF32U32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_bit_cast_f32u32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::PackUint2x32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_pack_uint2x32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UnpackUint2x32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_unpack_uint2x32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::PackDouble2x32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_pack_double2x32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UnpackDouble2x32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_unpack_double2x32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::PackUnorm2x16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_pack_unorm2x16(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UnpackUnorm2x16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_unpack_unorm2x16(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::PackSnorm2x16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_pack_snorm2x16(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UnpackSnorm2x16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_unpack_snorm2x16(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::PackUint2x16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_pack_uint2x16(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UnpackUint2x16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_unpack_uint2x16(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::PackSint2x16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_pack_sint2x16(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UnpackSint2x16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_unpack_sint2x16(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::PackHalf2x16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_pack_half2x16(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UnpackHalf2x16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_unpack_half2x16(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::PackUnorm4x8 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_pack_unorm4x8(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UnpackUnorm4x8 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_unpack_unorm4x8(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::PackSnorm4x8 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_pack_snorm4x8(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UnpackSnorm4x8 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_unpack_snorm4x8(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::PackUint4x8 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_pack_uint4x8(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UnpackUint4x8 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_unpack_uint4x8(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::PackSint4x8 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_pack_sint4x8(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UnpackSint4x8 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_unpack_sint4x8(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::PackUfloat10_11_11 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_pack_ufloat10_11_11(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UnpackUfloat10_11_11 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_unpack_ufloat10_11_11(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::PackUnorm2_10_10_10 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_pack_unorm2_10_10_10(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UnpackUnorm2_10_10_10 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_unpack_unorm2_10_10_10(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::PackSnorm2_10_10_10 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_pack_snorm2_10_10_10(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UnpackSnorm2_10_10_10 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_unpack_snorm2_10_10_10(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::PackUint2_10_10_10 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_pack_uint2_10_10_10(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UnpackUint2_10_10_10 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_unpack_uint2_10_10_10(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::PackSint2_10_10_10 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_pack_sint2_10_10_10(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UnpackSint2_10_10_10 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_unpack_sint2_10_10_10(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPAbs32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_abs32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPAbs64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_abs64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPAdd32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_add32(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPAdd64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_add64(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPSub32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_sub32(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPFma32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_fp_fma32(ctx, inst, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPFma64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_fp_fma64(ctx, inst, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPMax32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_max32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPMax64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_max64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPMin32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_min32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPMin64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_min64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPMinTri32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_fp_min_tri32(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPMaxTri32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_fp_max_tri32(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPMedTri32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_fp_med_tri32(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPMul32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_mul32(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPMul64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_mul64(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPDiv32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_div32(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPDiv64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_div64(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPNeg32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_neg32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPNeg64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_neg64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPRecip32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_recip32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPRecip64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_recip64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPRecipSqrt32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_recip_sqrt32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPRecipSqrt64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_recip_sqrt64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPSqrt => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_sqrt(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPSin => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_sin(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPExp2 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_exp2(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPPow => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_pow(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPLdexp => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_ldexp(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPCos => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_cos(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPLog2 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_log2(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPSaturate32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_saturate32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPSaturate64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_saturate64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPClamp32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_fp_clamp32(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPClamp64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_fp_clamp64(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPRoundEven32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_round_even32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPRoundEven64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_round_even64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPFloor32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_floor32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPFloor64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_floor64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPCeil32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_ceil32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPCeil64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_ceil64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPTrunc32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_trunc32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPTrunc64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_trunc64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPFract32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_fract32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPFract64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_fract64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPFrexpSig32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_frexp_sig32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPFrexpSig64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_frexp_sig64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPFrexpExp32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_frexp_exp32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPFrexpExp64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_frexp_exp64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPOrdEqual32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_ord_equal32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPOrdEqual64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_ord_equal64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPUnordEqual32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_unord_equal32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPUnordEqual64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_unord_equal64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPOrdNotEqual32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_ord_not_equal32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPOrdNotEqual64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_ord_not_equal64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPUnordNotEqual32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_unord_not_equal32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPUnordNotEqual64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_unord_not_equal64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPOrdLessThan32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_ord_less_than32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPOrdLessThan64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_ord_less_than64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPUnordLessThan32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_unord_less_than32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPUnordLessThan64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_unord_less_than64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPOrdGreaterThan32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_ord_greater_than32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPOrdGreaterThan64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_ord_greater_than64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPUnordGreaterThan32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_unord_greater_than32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPUnordGreaterThan64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_unord_greater_than64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPOrdLessThanEqual32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_ord_less_than_equal32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPOrdLessThanEqual64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_ord_less_than_equal64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPUnordLessThanEqual32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_unord_less_than_equal32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPUnordLessThanEqual64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_unord_less_than_equal64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPOrdGreaterThanEqual32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_ord_greater_than_equal32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPOrdGreaterThanEqual64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_ord_greater_than_equal64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPUnordGreaterThanEqual32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_unord_greater_than_equal32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPUnordGreaterThanEqual64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_fp_unord_greater_than_equal64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPIsNan32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_is_nan32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPIsNan64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_is_nan64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPIsInf32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_is_inf32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPIsInf64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_fp_is_inf64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FPCmpClass32 => emit_fp_cmp_class32(ctx),
        Opcode::IAdd32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_i_add32(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::IAdd64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_i_add64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::IAddCarry32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_i_add_carry32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ISub32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_i_sub32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ISub64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_i_sub64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::IMul32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_i_mul32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::IMul64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_i_mul64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SMulHi => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_s_mul_hi(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UMulHi => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_u_mul_hi(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SDiv32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_s_div32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UDiv32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_u_div32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SMod32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_s_mod32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UMod32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_u_mod32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::INeg32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_i_neg32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::INeg64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_i_neg64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::IAbs32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_i_abs32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ShiftLeftLogical32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shift_left_logical32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ShiftLeftLogical64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shift_left_logical64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ShiftRightLogical32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shift_right_logical32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ShiftRightLogical64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shift_right_logical64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ShiftRightArithmetic32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shift_right_arithmetic32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ShiftRightArithmetic64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shift_right_arithmetic64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitwiseAnd32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_bitwise_and32(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitwiseAnd64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_bitwise_and64(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitwiseOr32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_bitwise_or32(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitwiseOr64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_bitwise_or64(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitwiseXor32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_bitwise_xor32(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitwiseXor64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_bitwise_xor64(ctx, inst, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitFieldInsert32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let a3 = ctx.Def(&inst.borrow().arg(3));
            let value = emit_bit_field_insert32(ctx, a0, a1, a2, a3);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitFieldInsert64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let a3 = ctx.Def(&inst.borrow().arg(3));
            let value = emit_bit_field_insert64(ctx, a0, a1, a2, a3);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitFieldSExtract => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_bit_field_s_extract(ctx, inst, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitFieldUExtract => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_bit_field_u_extract(ctx, inst, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitReverse32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_bit_reverse32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitCount32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_bit_count32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitCount64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_bit_count64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitwiseNot32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_bitwise_not32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BitwiseNot64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_bitwise_not64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::MaskedBitCount32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_masked_bit_count32(ctx, a0, a1, inst.borrow().arg(2).u1());
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FindSMsb32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_find_s_msb32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FindUMsb32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_find_u_msb32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FindUMsb64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_find_u_msb64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FindILsb32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_find_i_lsb32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::FindILsb64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_find_i_lsb64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SMin32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_s_min32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UMin32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_u_min32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SMax32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_s_max32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UMax32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_u_max32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SMinTri32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_s_min_tri32(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UMinTri32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_u_min_tri32(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SMaxTri32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_s_max_tri32(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UMaxTri32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_u_max_tri32(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SMedTri32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_s_med_tri32(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UMedTri32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_u_med_tri32(ctx, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SClamp32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_s_clamp32(ctx, inst, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UClamp32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_u_clamp32(ctx, inst, a0, a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SLessThan32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_s_less_than32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SLessThan64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_s_less_than64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ULessThan32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_u_less_than32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ULessThan64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_u_less_than64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::IEqual32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_i_equal32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::IEqual64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_i_equal64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SLessThanEqual32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_s_less_than_equal32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SLessThanEqual64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_s_less_than_equal64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ULessThanEqual32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_u_less_than_equal32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ULessThanEqual64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_u_less_than_equal64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SGreaterThan32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_s_greater_than32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SGreaterThan64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_s_greater_than64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UGreaterThan32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_u_greater_than32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UGreaterThan64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_u_greater_than64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::INotEqual32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_i_not_equal32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::INotEqual64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_i_not_equal64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SGreaterThanEqual32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_s_greater_than_equal32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::SGreaterThanEqual64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_s_greater_than_equal64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UGreaterThanEqual32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_u_greater_than_equal32(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::UGreaterThanEqual64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_u_greater_than_equal64(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LogicalOr => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_logical_or(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LogicalAnd => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_logical_and(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LogicalXor => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_logical_xor(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LogicalNot => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_logical_not(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertS32F32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_s32f32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertS32F64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_s32f64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertU32F32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_u32f32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertF16F32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_f16f32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertF32F16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_f32f16(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertF32F64 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_f32f64(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertF64F32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_f64f32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertF32S32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_f32s32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertF32U32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_f32u32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertF64S32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_f64s32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertF64U32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_f64u32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertF32U16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_f32u16(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertU16U32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_u16u32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertU32U16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_u32u16(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertU64U32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_u64u32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertU8U32 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_u8u32(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertU32U8 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_u32u8(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertS32S8 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_s32s8(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ConvertS32S16 => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_convert_s32s16(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageHandle => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_image_handle(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageSampleRaw => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let a3 = ctx.Def(&inst.borrow().arg(3));
            let a4 = ctx.Def(&inst.borrow().arg(4));
            let value =
                emit_image_sample_raw(ctx, inst, inst.borrow().arg(0).u32(), a1, a2, a3, a4);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageSampleImplicitLod => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_image_sample_implicit_lod(
                ctx,
                inst,
                inst.borrow().arg(0).u32(),
                a1,
                a2,
                &inst.borrow().arg(3),
            );
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageSampleExplicitLod => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_image_sample_explicit_lod(
                ctx,
                inst,
                inst.borrow().arg(0).u32(),
                a1,
                a2,
                &inst.borrow().arg(3),
            );
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageSampleDrefImplicitLod => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let a3 = ctx.Def(&inst.borrow().arg(3));
            let value = emit_image_sample_dref_implicit_lod(
                ctx,
                inst,
                inst.borrow().arg(0).u32(),
                a1,
                a2,
                a3,
                &inst.borrow().arg(4),
            );
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageSampleDrefExplicitLod => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let a3 = ctx.Def(&inst.borrow().arg(3));
            let value = emit_image_sample_dref_explicit_lod(
                ctx,
                inst,
                inst.borrow().arg(0).u32(),
                a1,
                a2,
                a3,
                &inst.borrow().arg(4),
            );
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageGather => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_image_gather(
                ctx,
                inst,
                inst.borrow().arg(0).u32(),
                a1,
                &inst.borrow().arg(2),
            );
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageGatherDref => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a3 = ctx.Def(&inst.borrow().arg(3));
            let value = emit_image_gather_dref(
                ctx,
                inst,
                inst.borrow().arg(0).u32(),
                a1,
                &inst.borrow().arg(2),
                a3,
            );
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageQueryDimensions => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_image_query_dimensions(
                ctx,
                inst,
                inst.borrow().arg(0).u32(),
                a1,
                inst.borrow().arg(2).u1(),
            );
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageQueryLod => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_image_query_lod(ctx, inst, inst.borrow().arg(0).u32(), a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageGradient => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let a3 = ctx.Def(&inst.borrow().arg(3));
            let value = emit_image_gradient(
                ctx,
                inst,
                inst.borrow().arg(0).u32(),
                a1,
                a2,
                a3,
                &inst.borrow().arg(4),
                &inst.borrow().arg(5),
            );
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageRead => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let a3 = ctx.Def(&inst.borrow().arg(3));
            let value = emit_image_read(ctx, inst, inst.borrow().arg(0).u32(), a1, a2, a3);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageWrite => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let a3 = ctx.Def(&inst.borrow().arg(3));
            let a4 = ctx.Def(&inst.borrow().arg(4));
            emit_image_write(ctx, inst, inst.borrow().arg(0).u32(), a1, a2, a3, a4);
        }
        Opcode::ImageAtomicIAdd32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_image_atomic_i_add32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageAtomicSMin32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_image_atomic_s_min32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageAtomicUMin32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_image_atomic_u_min32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageAtomicSMax32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_image_atomic_s_max32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageAtomicUMax32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_image_atomic_u_max32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageAtomicFMax32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_image_atomic_f_max32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageAtomicFMin32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_image_atomic_f_min32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageAtomicInc32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_image_atomic_inc32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageAtomicDec32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_image_atomic_dec32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageAtomicAnd32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_image_atomic_and32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageAtomicOr32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_image_atomic_or32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageAtomicXor32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_image_atomic_xor32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageAtomicExchange32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let value = emit_image_atomic_exchange32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ImageAtomicCmpSwap32 => {
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let a2 = ctx.Def(&inst.borrow().arg(2));
            let a3 = ctx.Def(&inst.borrow().arg(3));
            let value =
                emit_image_atomic_cmp_swap32(ctx, inst, inst.borrow().arg(0).u32(), a1, a2, a3);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::CubeFaceIndex => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_cube_face_index(ctx, inst, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::LaneId => {
            let value = emit_lane_id(ctx);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::WarpId => {
            let value = emit_warp_id(ctx);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::QuadBroadcast => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_quad_broadcast(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::Shuffle => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shuffle(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ShuffleXor => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_shuffle_xor(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ReadFirstLane => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_read_first_lane(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::ReadLane => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_read_lane(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::WriteLane => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_write_lane(ctx, a0, a1, inst.borrow().arg(2).u32());
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::Ballot => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_ballot(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::BallotFindLsb => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_ballot_find_lsb(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::InverseBallot => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_inverse_ballot(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::DataAppend => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_data_append(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::DataConsume => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let a1 = ctx.Def(&inst.borrow().arg(1));
            let value = emit_data_consume(ctx, a0, a1);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::GroupAny => {
            let a0 = ctx.Def(&inst.borrow().arg(0));
            let value = emit_group_any(ctx, a0);
            inst.borrow_mut().set_definition_bits(value);
        }
        Opcode::Memtime => {
            let value = emit_memtime(ctx);
            inst.borrow_mut().set_definition_bits(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::ir::abstract_syntax_list::AbstractSyntaxNode;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block};
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn opcode(word: u32) -> u32 {
        word & 0xffff
    }

    fn make_program() -> (Program, RuntimeInfo) {
        let info = Rc::new(RefCell::new(
            crate::gpu::shader_recompiler::info::Info::new(
                HwStage::Fragment,
                SwStage::Fragment,
                ShaderParams::new([0; 16], &[], 0),
            ),
        ));
        let runtime = RuntimeInfo::new(HwStage::Fragment, SwStage::Fragment);
        (Program::new(info), runtime)
    }

    #[test]
    fn an_empty_program_still_modules_its_entry_point() {
        let (program, runtime) = make_program();
        let mut binding = Bindings::default();
        let read = |_| 0u32;
        let words = emit_spirv(
            &Profile::default(),
            &runtime,
            &program,
            &mut binding,
            &[],
            &read,
        );
        assert_eq!(words[0], super::super::builder::MAGIC_NUMBER);
        assert!(words.windows(2).any(|window| {
            opcode(window[0]) == spv::Op::EntryPoint && window[1] == spv::ExecutionModel::Fragment
        }));
        assert!(words
            .iter()
            .any(|word| opcode(*word) == spv::Op::FunctionEnd));
        assert_eq!(binding.user_data, 0);
    }

    #[test]
    fn a_block_is_labelled_and_its_instructions_emitted() {
        let (mut program, runtime) = make_program();
        let block = Block::new_ref();
        append_new_inst(&block, Opcode::Prologue, &[]);
        program.blocks.push(block.clone());
        program
            .syntax_list
            .push(AbstractSyntaxNode::block(block.clone()));
        let mut binding = Bindings::default();
        let read = |_| 0u32;
        let words = emit_spirv(
            &Profile::default(),
            &runtime,
            &program,
            &mut binding,
            &[],
            &read,
        );
        assert_ne!(block.borrow().definition_bits(), 0);
        assert!(words.iter().any(|word| opcode(*word) == spv::Op::Label));
        assert!(words
            .iter()
            .any(|word| opcode(*word) == spv::Op::FunctionEnd));
    }
}
