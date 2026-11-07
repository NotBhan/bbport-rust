//! Wave64 lowering, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/lower_wave64_pass.cpp`.
//!
//! A compute shader with more than 32 threads is split into 32-wide subgroups. Ballots,
//! immediate-lane reads and `SubgroupLtMask` are rewritten to use a scratch slice of shared
//! memory, one dword per subgroup, only in control flow that is uniform across the workgroup.

use crate::gpu::common::alignment::align_up_u32;
use crate::gpu::shader_recompiler::ir::abstract_syntax_list::NodeData;
use crate::gpu::shader_recompiler::ir::attribute::Attribute;
use crate::gpu::shader_recompiler::ir::basic_block::BlockRef;
use crate::gpu::shader_recompiler::ir::breadth_first_search::breadth_first_search;
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::microinstruction::{replace_uses_with, InstRef};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::value::{U1, U32, U32U64, U64};
use crate::gpu::shader_recompiler::profile::Profile;
use crate::gpu::shader_recompiler::runtime_info::{HwInfo, HwStage, RuntimeInfo};
use std::rc::Rc;

fn wide(value: &U32) -> U32U64 {
    U32U64::new(value.value().clone())
}

fn position(block: &BlockRef, inst: &InstRef) -> Option<usize> {
    block
        .borrow()
        .instructions
        .iter()
        .position(|candidate| Rc::ptr_eq(candidate, inst))
}

/// `IsDivergentCondition`.
fn is_divergent_condition(condition: &U1) -> bool {
    if condition.value().is_immediate() {
        return false;
    }
    let inst = condition.value().inst();
    breadth_first_search(&inst, |inst| {
        let opcode = inst.borrow().get_opcode();
        match opcode {
            Opcode::LaneId => Some(true),
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

struct ConditionalScope {
    merge: BlockRef,
    divergent: bool,
}

/// `FindUniformBlocks`.
fn find_uniform_blocks(program: &Program) -> Vec<BlockRef> {
    let mut blocks = Vec::new();
    let mut conditionals: Vec<ConditionalScope> = Vec::new();
    let mut loops: Vec<BlockRef> = Vec::new();
    let mut divergence_depth = 0u32;

    for node in &program.syntax_list {
        match &node.data {
            NodeData::If { cond, merge, .. } => {
                let divergent = is_divergent_condition(cond);
                conditionals.push(ConditionalScope {
                    merge: merge.clone(),
                    divergent,
                });
                divergence_depth += u32::from(divergent);
            }
            NodeData::EndIf { merge } => {
                assert!(
                    !conditionals.is_empty()
                        && Rc::ptr_eq(&conditionals.last().expect("a scope").merge, merge)
                );
                let scope = conditionals.pop().expect("a scope");
                divergence_depth -= u32::from(scope.divergent);
            }
            NodeData::Loop { merge, .. } => loops.push(merge.clone()),
            NodeData::Repeat { merge, .. } => {
                if loops.last().is_none_or(|last| !Rc::ptr_eq(last, merge)) {
                    return Vec::new();
                }
                loops.pop();
            }
            NodeData::Block(block) if divergence_depth == 0 && loops.is_empty() => {
                blocks.push(block.clone());
            }
            _ => {}
        }
    }
    if !conditionals.is_empty() || !loops.is_empty() {
        return Vec::new();
    }
    blocks
}

/// `LowerWave64BallotPass`.
pub fn lower_wave64_ballot_pass(program: &Program, runtime_info: &RuntimeInfo, profile: &Profile) {
    if program.info.borrow().persistent.hw_stage != HwStage::Compute || profile.subgroup_size == 64
    {
        return;
    }
    let HwInfo::Cs(compute) = &runtime_info.hw else {
        return;
    };

    let [size_x, size_y, size_z] = compute.workgroup_size;
    let num_threads = size_x.wrapping_mul(size_y).wrapping_mul(size_z);
    if num_threads <= 32 {
        return;
    }

    let uniform_blocks = find_uniform_blocks(program);
    let mut worklist: Vec<InstRef> = Vec::new();
    for block in &program.blocks {
        let is_uniform = uniform_blocks
            .iter()
            .any(|candidate| Rc::ptr_eq(candidate, block));
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            let opcode = inst.borrow().get_opcode();
            if opcode == Opcode::ReadLane && inst.borrow().arg(1).is_immediate() {
                if is_uniform {
                    worklist.push(inst.clone());
                }
            } else if opcode == Opcode::Ballot {
                let unpacks = inst
                    .borrow()
                    .uses()
                    .iter()
                    .any(|r#use| r#use.user.borrow().get_opcode() == Opcode::UnpackUint2x32);
                if unpacks && is_uniform {
                    worklist.push(inst.clone());
                }
            } else if opcode == Opcode::GetAttributeU32
                && inst.borrow().arg(0).attribute() == Attribute::SubgroupLtMask
            {
                worklist.push(inst.clone());
            }
        }
    }
    if worklist.is_empty() {
        return;
    }

    let scratch_base = align_up_u32(compute.shared_memory_size, std::mem::size_of::<u64>());
    let scratch_size = (align_up_u32(num_threads, 64) / profile.subgroup_size) * 4;
    program
        .info
        .borrow_mut()
        .persistent
        .shared_memory_scratch_size = scratch_base + scratch_size - compute.shared_memory_size;

    for inst in worklist {
        let block = inst
            .borrow()
            .parent()
            .expect("the instruction has a parent");
        let index = position(&block, &inst).expect("the instruction is in its block");
        let mut ir = IrEmitter::at(&block, index);
        let invocation_index = ir.get_attribute_u32(Attribute::LocalInvocationIndex, 0);
        let five = ir.imm32(5);
        let subgroup_id = U32::new(
            ir.shift_right_logical(&wide(&invocation_index), &five)
                .into_value(),
        );

        let opcode = inst.borrow().get_opcode();
        if opcode == Opcode::Ballot {
            let condition = U1::new(inst.borrow().arg(0));
            let ballot = ir.ballot(&condition);
            let unpacked = ir.unpack_uint_2x32(&ballot);
            let mask_low = U32::new(ir.composite_extract(&unpacked, 0));
            let scratch = ir.imm32(scratch_base);
            let two = ir.imm32(2);
            let subgroup_offset = U32::new(
                ir.shift_left_logical(&wide(&subgroup_id), &two)
                    .into_value(),
            );
            let offset = U32::new(
                ir.i_add(&wide(&scratch), &wide(&subgroup_offset))
                    .into_value(),
            );
            ir.write_shared(32, &mask_low.into_value(), &offset, false);
            ir.barrier();
            let mask = U64::new(ir.load_shared(64, false, &offset, false));
            ir.barrier();
            replace_uses_with(&inst, mask.into_value());
        } else if opcode == Opcode::ReadLane {
            let lane = U32::new(inst.borrow().arg(1));
            let thirty_one = ir.imm32(31);
            let lane32 = U32::new(
                ir.bitwise_and(&wide(&lane), &wide(&thirty_one))
                    .into_value(),
            );
            let five = ir.imm32(5);
            let half = U32::new(ir.shift_right_logical(&wide(&lane), &five).into_value());
            let scratch = ir.imm32(scratch_base);
            let two = ir.imm32(2);
            let subgroup_offset = U32::new(
                ir.shift_left_logical(&wide(&subgroup_id), &two)
                    .into_value(),
            );
            let offset = U32::new(
                ir.i_add(&wide(&scratch), &wide(&subgroup_offset))
                    .into_value(),
            );
            let value = ir.read_lane(&U32::new(inst.borrow().arg(0)), &lane32);
            ir.write_shared(32, &value.into_value(), &offset, false);
            ir.barrier();
            let not_seven = ir.imm32(!7u32);
            let aligned = U32::new(
                ir.bitwise_and(&wide(&offset), &wide(&not_seven))
                    .into_value(),
            );
            let two = ir.imm32(2);
            let half_shifted = U32::new(ir.shift_left_logical(&wide(&half), &two).into_value());
            let load_offset =
                U32::new(ir.i_add(&wide(&aligned), &wide(&half_shifted)).into_value());
            let loaded = U32::new(ir.load_shared(32, false, &load_offset, false));
            ir.barrier();
            replace_uses_with(&inst, loaded.into_value());
        } else if opcode == Opcode::GetAttributeU32
            && inst.borrow().arg(0).attribute() == Attribute::SubgroupLtMask
        {
            let sixty_three = ir.imm32(63);
            let subgroup_invocation_id = U32::new(
                ir.bitwise_and(&wide(&invocation_index), &wide(&sixty_three))
                    .into_value(),
            );
            let one = ir.imm64(1);
            let shifted = U64::new(
                ir.shift_left_logical(&U32U64::new(one.into_value()), &subgroup_invocation_id)
                    .into_value(),
            );
            let one_again = ir.imm64(1);
            let mask = U64::new(
                ir.i_sub(
                    &U32U64::new(shifted.into_value()),
                    &U32U64::new(one_again.into_value()),
                )
                .into_value(),
            );
            let unpacked = ir.unpack_uint_2x32(&mask);
            let component = inst.borrow().arg(1).u32() as usize;
            let result = ir.composite_extract(&unpacked, component);
            replace_uses_with(&inst, result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::abstract_syntax_list::AbstractSyntaxNode;
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
            shared_memory_size: 0,
            workgroup_size: [64, 1, 1],
            tgid_enable: [false; 3],
        });
        runtime
    }

    fn profile() -> Profile {
        Profile {
            subgroup_size: 32,
            ..Profile::default()
        }
    }

    fn append(block: &BlockRef, op: Opcode, args: &[Value]) -> InstRef {
        let index = append_new_inst(block, op, args);
        block.borrow().instructions[index].clone()
    }

    fn opcodes(block: &BlockRef) -> Vec<Opcode> {
        block
            .borrow()
            .instructions()
            .iter()
            .map(|inst| inst.borrow().get_opcode())
            .collect()
    }

    #[test]
    fn a_ballot_is_written_to_and_read_back_from_shared_memory() {
        let block = Block::new_ref();
        let condition = append(&block, Opcode::GetScc, &[]);
        let ballot = append(&block, Opcode::Ballot, &[Value::from_inst(condition)]);
        let unpack = append(
            &block,
            Opcode::UnpackUint2x32,
            &[Value::from_inst(ballot.clone())],
        );
        let extract = append(
            &block,
            Opcode::CompositeExtractU32x2,
            &[Value::from_inst(unpack.clone()), Value::from_u32(0)],
        );
        let sink = append(&block, Opcode::ConvertF16F32, &[Value::from_inst(extract)]);

        let program = program(&block);
        lower_wave64_ballot_pass(&program, &runtime(), &profile());

        assert_eq!(ballot.borrow().get_opcode(), Opcode::Void);
        assert_eq!(
            unpack.borrow().arg(0).inst().borrow().get_opcode(),
            Opcode::LoadSharedU64
        );
        assert_eq!(
            program.info.borrow().persistent.shared_memory_scratch_size,
            8
        );
        let ops = opcodes(&block);
        assert!(ops.contains(&Opcode::GetAttributeU32));
        assert!(ops.contains(&Opcode::WriteSharedU32));
        assert!(ops.contains(&Opcode::Barrier));
        assert!(ops.contains(&Opcode::LoadSharedU64));
        let _ = sink;
    }

    #[test]
    fn an_immediate_lane_read_round_trips_through_shared_memory() {
        let block = Block::new_ref();
        let value = append(&block, Opcode::LaneId, &[]);
        let read = append(
            &block,
            Opcode::ReadLane,
            &[Value::from_inst(value), Value::from_u32(3)],
        );
        let sink = append(
            &block,
            Opcode::ConvertF16F32,
            &[Value::from_inst(read.clone())],
        );

        let program = program(&block);
        lower_wave64_ballot_pass(&program, &runtime(), &profile());

        assert_eq!(read.borrow().get_opcode(), Opcode::Void);
        assert_eq!(
            sink.borrow().arg(0).inst().borrow().get_opcode(),
            Opcode::LoadSharedU32
        );
        let ops = opcodes(&block);
        assert!(ops.contains(&Opcode::ReadLane));
        assert!(ops.contains(&Opcode::WriteSharedU32));
        assert!(ops.contains(&Opcode::LoadSharedU32));
    }

    #[test]
    fn subgroup_lt_mask_is_built_from_one_shifted_by_the_invocation() {
        let block = Block::new_ref();
        let mask = append(
            &block,
            Opcode::GetAttributeU32,
            &[
                Value::from_attribute(Attribute::SubgroupLtMask),
                Value::from_u32(1),
            ],
        );
        let sink = append(
            &block,
            Opcode::ConvertF16F32,
            &[Value::from_inst(mask.clone())],
        );

        let program = program(&block);
        lower_wave64_ballot_pass(&program, &runtime(), &profile());

        assert_eq!(mask.borrow().get_opcode(), Opcode::Void);
        assert_eq!(
            sink.borrow().arg(0).inst().borrow().get_opcode(),
            Opcode::CompositeExtractU32x2
        );
        let ops = opcodes(&block);
        assert!(ops.contains(&Opcode::ShiftLeftLogical64));
        assert!(ops.contains(&Opcode::ISub64));
    }

    #[test]
    fn a_wave64_shader_is_left_alone() {
        let block = Block::new_ref();
        let condition = append(&block, Opcode::GetScc, &[]);
        let ballot = append(&block, Opcode::Ballot, &[Value::from_inst(condition)]);
        let unpack = append(
            &block,
            Opcode::UnpackUint2x32,
            &[Value::from_inst(ballot.clone())],
        );
        let _ = unpack;

        let program = program(&block);
        let mut wave64_profile = profile();
        wave64_profile.subgroup_size = 64;
        lower_wave64_ballot_pass(&program, &runtime(), &wave64_profile);

        assert_eq!(ballot.borrow().get_opcode(), Opcode::Ballot);
    }
}
