//! Shared-memory barrier insertion, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/shared_memory_barrier_pass.cpp`.
//!
//! When a workgroup is exactly one hardware subgroup, barriers can be inserted only where a
//! shared-memory write and read meet: inside straight-line blocks, and after joins whose
//! condition depends on the local invocation id. Divergent loops defer their barrier to the
//! loop's merge block.

use crate::gpu::shader_recompiler::ir::abstract_syntax_list::{AbstractSyntaxNode, NodeData};
use crate::gpu::shader_recompiler::ir::attribute::Attribute;
use crate::gpu::shader_recompiler::ir::basic_block::BlockRef;
use crate::gpu::shader_recompiler::ir::breadth_first_search::breadth_first_search_value;
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::microinstruction::{is_phi, InstRef};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::value::U1;
use crate::gpu::shader_recompiler::profile::Profile;
use crate::gpu::shader_recompiler::runtime_info::{HwInfo, HwStage, RuntimeInfo};
use std::collections::HashSet;
use std::rc::Rc;

const GCN_SUBGROUP_SIZE: u32 = 64;

fn key(block: &BlockRef) -> usize {
    Rc::as_ptr(block) as usize
}

fn position(block: &BlockRef, inst: &InstRef) -> Option<usize> {
    block
        .borrow()
        .instructions
        .iter()
        .position(|candidate| Rc::ptr_eq(candidate, inst))
}

fn is_load_shared(inst: &InstRef) -> bool {
    matches!(
        inst.borrow().get_opcode(),
        Opcode::LoadSharedU16 | Opcode::LoadSharedU32 | Opcode::LoadSharedU64
    )
}

fn is_write_shared(inst: &InstRef) -> bool {
    let opcode = inst.borrow().get_opcode();
    if opcode >= Opcode::SharedAtomicIAdd32 && opcode <= Opcode::SharedAtomicCmpSwap64 {
        return inst.borrow().flags() == 0;
    }
    matches!(
        opcode,
        Opcode::WriteSharedU16 | Opcode::WriteSharedU32 | Opcode::WriteSharedU64
    )
}

/// `EmitBarrierInBlock`.
fn emit_barrier_in_block(block: &BlockRef) {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum BarrierAction {
        None,
        BarrierOnWrite,
        BarrierOnRead,
    }

    let mut action = BarrierAction::None;
    let instructions = block.borrow().instructions.clone();
    for inst in &instructions {
        if is_load_shared(inst) {
            if action == BarrierAction::BarrierOnRead {
                let index = position(block, inst).expect("the instruction is in its block");
                let mut ir = IrEmitter::at(block, index);
                ir.barrier();
            }
            action = BarrierAction::BarrierOnWrite;
            continue;
        }
        if is_write_shared(inst) {
            if action == BarrierAction::BarrierOnWrite {
                let index = position(block, inst).expect("the instruction is in its block");
                let mut ir = IrEmitter::at(block, index);
                ir.barrier();
            }
            action = BarrierAction::BarrierOnRead;
        }
    }
    if action != BarrierAction::None {
        let length = block.borrow().instructions.len();
        let point = length.saturating_sub(1);
        let mut ir = IrEmitter::at(block, point);
        ir.barrier();
    }
}

/// `EmitBarrierAtBlockStart`.
fn emit_barrier_at_block_start(block: &BlockRef) {
    let instructions = block.borrow().instructions.clone();
    let point = instructions
        .iter()
        .position(|inst| !is_phi(inst))
        .unwrap_or(instructions.len());
    let mut ir = IrEmitter::at(block, point);
    ir.barrier();
}

/// `IsDivergent`.
fn is_divergent(condition: &U1) -> bool {
    breadth_first_search_value(&condition.value().clone(), |inst| {
        let opcode = inst.borrow().get_opcode();
        match opcode {
            Opcode::GetAttributeU32 => {
                if inst.borrow().arg(0).attribute() == Attribute::LocalInvocationId {
                    Some(true)
                } else {
                    None
                }
            }
            _ => None,
        }
    })
    .unwrap_or(false)
}

/// `EmitBarrierInMergeBlock`.
fn emit_barrier_in_merge_block(
    condition: &U1,
    merge: &BlockRef,
    divergence_end: &mut HashSet<usize>,
    divergence_depth: &mut u32,
) {
    if is_divergent(condition) {
        if *divergence_depth == 0 {
            emit_barrier_at_block_start(merge);
        }
        *divergence_depth += 1;
        divergence_end.insert(key(merge));
    }
}

/// `FindDivergentLoops`.
fn find_divergent_loops(syntax_list: &[AbstractSyntaxNode]) -> HashSet<usize> {
    let mut divergent_loops = HashSet::new();
    for node in syntax_list {
        match &node.data {
            NodeData::Repeat { cond, merge, .. } if is_divergent(cond) => {
                divergent_loops.insert(key(merge));
            }
            NodeData::Break { cond, merge, .. } if is_divergent(cond) => {
                divergent_loops.insert(key(merge));
            }
            _ => {}
        }
    }
    divergent_loops
}

/// `SharedMemoryBarrierPass`.
pub fn shared_memory_barrier_pass(
    program: &Program,
    runtime_info: &RuntimeInfo,
    profile: &Profile,
) {
    if program.info.borrow().persistent.hw_stage != HwStage::Compute {
        return;
    }
    let HwInfo::Cs(compute) = &runtime_info.hw else {
        return;
    };
    let shared_memory_size = compute.shared_memory_size;
    let threadgroup_size = compute.workgroup_size[0]
        .wrapping_mul(compute.workgroup_size[1])
        .wrapping_mul(compute.workgroup_size[2]);
    if shared_memory_size == 0
        || threadgroup_size != GCN_SUBGROUP_SIZE
        || !profile.needs_lds_barriers
    {
        return;
    }

    let mut divergence_depth = 0u32;
    let mut divergence_end: HashSet<usize> = HashSet::new();
    let divergent_loops = find_divergent_loops(&program.syntax_list);
    for node in &program.syntax_list {
        match &node.data {
            NodeData::EndIf { merge } => {
                if divergence_end.contains(&key(merge)) {
                    divergence_depth -= 1;
                }
            }
            NodeData::If { cond, merge, .. } => {
                emit_barrier_in_merge_block(
                    cond,
                    merge,
                    &mut divergence_end,
                    &mut divergence_depth,
                );
            }
            NodeData::Loop { merge, .. } => {
                if divergent_loops.contains(&key(merge)) {
                    divergence_depth += 1;
                }
            }
            NodeData::Repeat { merge, .. } => {
                if divergent_loops.contains(&key(merge)) {
                    assert!(divergence_depth > 0);
                    divergence_depth -= 1;
                    if divergence_depth == 0 {
                        emit_barrier_at_block_start(merge);
                    }
                }
            }
            NodeData::Block(block) if divergence_depth == 0 => {
                emit_barrier_in_block(block);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block};
    use crate::gpu::shader_recompiler::ir::value::Value;
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{HwComputeRuntimeInfo, SwStage};
    use std::cell::RefCell;

    fn program(block: &BlockRef) -> Program {
        let info = Rc::new(RefCell::new(Info::new(
            HwStage::Compute,
            SwStage::Compute,
            ShaderParams::new([0; 16], &[], 0),
        )));
        let mut program = Program::new(info);
        program.blocks = vec![block.clone()];
        program.syntax_list = vec![AbstractSyntaxNode::block(block.clone())];
        program
    }

    fn runtime() -> RuntimeInfo {
        let mut runtime = RuntimeInfo::new(HwStage::Compute, SwStage::Compute);
        runtime.hw = HwInfo::Cs(HwComputeRuntimeInfo {
            shared_memory_size: 64,
            workgroup_size: [64, 1, 1],
            tgid_enable: [false; 3],
        });
        runtime
    }

    fn profile() -> Profile {
        Profile {
            needs_lds_barriers: true,
            ..Profile::default()
        }
    }

    fn append(block: &BlockRef, op: Opcode, args: &[Value]) -> InstRef {
        let index = append_new_inst(block, op, args);
        block.borrow().instructions[index].clone()
    }

    fn barriers(block: &BlockRef) -> usize {
        block
            .borrow()
            .instructions()
            .iter()
            .filter(|inst| inst.borrow().get_opcode() == Opcode::Barrier)
            .count()
    }

    #[test]
    fn a_write_followed_by_a_read_gets_barriers() {
        let block = Block::new_ref();
        let offset = Value::from_u32(0);
        append(
            &block,
            Opcode::WriteSharedU32,
            &[offset.clone(), Value::from_u32(1)],
        );
        append(&block, Opcode::LoadSharedU32, &[offset]);
        let sink = append(&block, Opcode::LaneId, &[]);

        let program = program(&block);
        shared_memory_barrier_pass(&program, &runtime(), &profile());

        assert_eq!(barriers(&block), 2);
        let _ = sink;
    }

    #[test]
    fn a_block_without_shared_access_gets_no_barrier() {
        let block = Block::new_ref();
        append(&block, Opcode::LaneId, &[]);
        append(&block, Opcode::LaneId, &[]);

        let program = program(&block);
        shared_memory_barrier_pass(&program, &runtime(), &profile());

        assert_eq!(barriers(&block), 0);
    }
}
