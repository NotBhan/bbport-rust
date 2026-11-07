//! Barrier emitters, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv_barriers.cpp`.

use super::emit_context::EmitContext;
use super::spv;
use crate::gpu::shader_recompiler::runtime_info::SwStage;

fn memory_barrier(ctx: &mut EmitContext, scope: u32) {
    let semantics = spv::MemorySemanticsMask::AcquireRelease
        | spv::MemorySemanticsMask::UniformMemory
        | spv::MemorySemanticsMask::WorkgroupMemory
        | spv::MemorySemanticsMask::AtomicCounterMemory
        | spv::MemorySemanticsMask::ImageMemory;
    let scope = ctx.ConstU32(scope);
    let semantics = ctx.ConstU32(semantics);
    ctx.OpMemoryBarrier(scope, semantics);
}

pub fn emit_barrier(ctx: &mut EmitContext) {
    let execution = spv::Scope::Workgroup;
    let (memory, memory_semantics) = if ctx.sw_stage == SwStage::TessellationControl {
        (spv::Scope::Invocation, spv::MemorySemanticsMask::MaskNone)
    } else {
        (
            spv::Scope::Workgroup,
            spv::MemorySemanticsMask::AcquireRelease | spv::MemorySemanticsMask::WorkgroupMemory,
        )
    };
    let execution = ctx.ConstU32(execution);
    let memory = ctx.ConstU32(memory);
    let memory_semantics = ctx.ConstU32(memory_semantics);
    ctx.OpControlBarrier(execution, memory, memory_semantics);
}

pub fn emit_workgroup_memory_barrier(ctx: &mut EmitContext) {
    memory_barrier(ctx, spv::Scope::Workgroup);
}

pub fn emit_device_memory_barrier(ctx: &mut EmitContext) {
    memory_barrier(ctx, spv::Scope::Device);
}
