//! Read-lane elimination, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/readlane_elimination_pass.cpp`.
//!
//! An immediate-lane `ReadLane` whose chain consists only of lane writes, reads and phis is
//! replaced with the value written to that lane, duplicating the phis so each operand is the real
//! written value. Phis that become trivial are folded afterwards.

use crate::gpu::shader_recompiler::ir::microinstruction::{
    add_phi_operand, replace_uses_with, set_arg, InstRef,
};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::r#type::Type;
use crate::gpu::shader_recompiler::ir::value::Value;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::rc::Rc;

fn key(inst: &InstRef) -> usize {
    Rc::as_ptr(inst) as usize
}

/// `SearchChain`.
fn search_chain(inst: &InstRef, lane: u32) -> InstRef {
    let mut inst = inst.clone();
    while inst.borrow().get_opcode() == Opcode::WriteLane {
        if inst.borrow().arg(2).u32() == lane {
            return inst;
        }
        let next = inst.borrow().arg(0).inst();
        inst = next;
    }
    inst
}

/// `IsPossibleToEliminate`.
fn is_possible_to_eliminate(inst: &InstRef, lane: u32) -> bool {
    let mut visited: Vec<InstRef> = Vec::new();
    let mut queue: VecDeque<InstRef> = VecDeque::new();
    queue.push_back(inst.clone());

    while let Some(candidate) = queue.pop_front() {
        let inst = search_chain(&candidate, lane);
        let opcode = inst.borrow().get_opcode();
        if opcode == Opcode::WriteLane {
            // A possible write-lane source: stop looking here.
            continue;
        }
        if opcode != Opcode::ReadLane && opcode != Opcode::Phi && opcode != Opcode::UndefU32 {
            return false;
        }
        let num_args = inst.borrow().num_args();
        for index in (0..num_args).rev() {
            let arg = inst.borrow().arg(index);
            if arg.is_immediate() {
                continue;
            }
            let arg_inst = arg.inst();
            if !visited.iter().any(|seen| Rc::ptr_eq(seen, &arg_inst)) {
                visited.push(arg_inst.clone());
                queue.push_back(arg_inst);
            }
        }
    }
    true
}

/// `GetRealValue`.
fn get_real_value(phi_map: &mut HashMap<usize, Value>, inst: &InstRef, lane: u32) -> Value {
    let inst = search_chain(inst, lane);
    if inst.borrow().get_opcode() == Opcode::WriteLane {
        return inst.borrow().arg(1);
    }

    if inst.borrow().get_opcode() == Opcode::Phi {
        let inst_key = key(&inst);
        if let Some(value) = phi_map.get(&inst_key) {
            return value.clone();
        }

        let block = inst.borrow().parent().expect("the phi has a parent");
        let index = block
            .borrow()
            .instructions
            .iter()
            .position(|candidate| Rc::ptr_eq(candidate, &inst))
            .expect("the phi is in its block");
        let new_index = crate::gpu::shader_recompiler::ir::basic_block::prepend_new_inst(
            &block,
            index,
            Opcode::Phi,
            &[],
            0,
        );
        let new_phi = block.borrow().instructions[new_index].clone();
        new_phi.borrow_mut().set_type_flags(Type::U32);
        phi_map.insert(inst_key, Value::from_inst(new_phi.clone()));

        let num_args = inst.borrow().num_args();
        let mut phi_args = Vec::with_capacity(num_args);
        for arg_index in 0..num_args {
            let arg = inst.borrow().arg(arg_index);
            if arg.is_immediate() {
                phi_args.push(arg);
            } else {
                phi_args.push(get_real_value(phi_map, &arg.inst(), lane));
            }
        }
        for (arg_index, arg) in phi_args.into_iter().enumerate() {
            let phi_block = inst.borrow().phi_block(arg_index);
            add_phi_operand(&new_phi, &phi_block, arg);
        }
        return Value::from_inst(new_phi);
    }
    if inst.borrow().get_opcode() == Opcode::UndefU32 {
        return Value::from_inst(inst);
    }
    panic!("Unreachable");
}

/// `ReplaceUsesWith`, without removing the instruction.
fn replace_uses(inst: &InstRef, replacement: Value) {
    let uses = inst.borrow().uses().to_vec();
    for r#use in uses {
        set_arg(&r#use.user, r#use.operand as usize, replacement.clone());
    }
}

/// `ReadLaneEliminationPass`.
pub fn readlane_elimination_pass(program: &Program) {
    let mut phi_map: HashMap<usize, Value> = HashMap::new();
    let mut worklist: Vec<InstRef> = Vec::new();

    for block in &program.blocks {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            if inst.borrow().get_opcode() != Opcode::ReadLane {
                continue;
            }
            if !inst.borrow().arg(1).is_immediate() {
                continue;
            }

            let lane = inst.borrow().arg(1).u32();
            let mut prod = inst.borrow().arg(0).inst();
            let mut replacement = Value::empty();

            prod = search_chain(&prod, lane);
            if prod.borrow().get_opcode() == Opcode::WriteLane {
                replacement = prod.borrow().arg(1);
            } else if prod.borrow().get_opcode() == Opcode::Phi
                && is_possible_to_eliminate(&prod, lane)
            {
                replacement = get_real_value(&mut phi_map, &prod, lane);
                for value in phi_map.values() {
                    worklist.push(value.inst());
                }
                phi_map.clear();
            }
            if replacement.is_empty() {
                continue;
            }

            for r#use in inst.borrow().uses().to_vec() {
                if r#use.user.borrow().get_opcode() == Opcode::Phi {
                    worklist.push(r#use.user.clone());
                }
            }
            replace_uses(inst, replacement);
        }
    }

    while let Some(phi) = worklist.pop() {
        if phi.borrow().get_opcode() == Opcode::Void {
            continue;
        }

        let mut same = Value::empty();
        let mut non_trivial = false;
        let num_args = phi.borrow().num_args();
        for index in 0..num_args {
            let op = phi.borrow().arg(index);
            if op == same || op == Value::from_inst(phi.clone()) {
                continue;
            }
            if !same.is_empty() {
                non_trivial = true;
                break;
            }
            same = op;
        }
        if non_trivial {
            continue;
        }

        let block = phi.borrow().parent().expect("the phi has a parent");
        assert!(!same.is_empty());

        for r#use in phi.borrow().uses().to_vec() {
            if r#use.user.borrow().get_opcode() == Opcode::Phi && !Rc::ptr_eq(&r#use.user, &phi) {
                worklist.push(r#use.user.clone());
            }
        }
        replace_uses_with(&phi, same);
        block
            .borrow_mut()
            .instructions
            .retain(|candidate| !Rc::ptr_eq(candidate, &phi));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block, BlockRef};
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage};
    use std::cell::RefCell;

    fn program(block: &BlockRef) -> Program {
        let info = Rc::new(RefCell::new(Info::new(
            HwStage::Vertex,
            SwStage::Vertex,
            ShaderParams::new([0; 16], &[], 0),
        )));
        let mut program = Program::new(info);
        program.blocks = vec![block.clone()];
        program
    }

    fn append(block: &BlockRef, op: Opcode, args: &[Value]) -> InstRef {
        let index = append_new_inst(block, op, args);
        block.borrow().instructions[index].clone()
    }

    fn lane_id(block: &BlockRef) -> InstRef {
        append(block, Opcode::LaneId, &[])
    }

    #[test]
    fn a_read_of_the_same_lane_uses_the_written_value() {
        let block = Block::new_ref();
        let chain = lane_id(&block);
        let data = lane_id(&block);
        let write = append(
            &block,
            Opcode::WriteLane,
            &[
                Value::from_inst(chain),
                Value::from_inst(data.clone()),
                Value::from_u32(2),
            ],
        );
        let read = append(
            &block,
            Opcode::ReadLane,
            &[Value::from_inst(write), Value::from_u32(2)],
        );
        let sink = append(
            &block,
            Opcode::BitCastF32U32,
            &[Value::from_inst(read.clone())],
        );

        readlane_elimination_pass(&program(&block));
        assert!(Rc::ptr_eq(&sink.borrow().arg(0).inst(), &data));
        assert_eq!(read.borrow().get_opcode(), Opcode::ReadLane);
    }

    #[test]
    fn the_search_follows_a_chain_of_writes() {
        let block = Block::new_ref();
        let chain = lane_id(&block);
        let first_data = lane_id(&block);
        let second_data = lane_id(&block);
        let first = append(
            &block,
            Opcode::WriteLane,
            &[
                Value::from_inst(chain),
                Value::from_inst(first_data.clone()),
                Value::from_u32(1),
            ],
        );
        let second = append(
            &block,
            Opcode::WriteLane,
            &[
                Value::from_inst(first),
                Value::from_inst(second_data),
                Value::from_u32(3),
            ],
        );
        let read = append(
            &block,
            Opcode::ReadLane,
            &[Value::from_inst(second), Value::from_u32(1)],
        );
        let sink = append(&block, Opcode::BitCastF32U32, &[Value::from_inst(read)]);

        readlane_elimination_pass(&program(&block));
        assert!(Rc::ptr_eq(&sink.borrow().arg(0).inst(), &first_data));
    }

    #[test]
    fn a_phi_of_writes_is_rebuilt_from_the_written_values() {
        let block = Block::new_ref();
        let left = Block::new_ref();
        let right = Block::new_ref();
        let chain = append(&block, Opcode::LaneId, &[]);
        let left_data = append(&left, Opcode::LaneId, &[]);
        let right_data = append(&right, Opcode::LaneId, &[]);
        let left_write = append(
            &left,
            Opcode::WriteLane,
            &[
                Value::from_inst(chain.clone()),
                Value::from_inst(left_data.clone()),
                Value::from_u32(1),
            ],
        );
        let right_write = append(
            &right,
            Opcode::WriteLane,
            &[
                Value::from_inst(chain),
                Value::from_inst(right_data.clone()),
                Value::from_u32(1),
            ],
        );
        let phi = append(&block, Opcode::Phi, &[]);
        add_phi_operand(&phi, &left, Value::from_inst(left_write));
        add_phi_operand(&phi, &right, Value::from_inst(right_write));
        let read = append(
            &block,
            Opcode::ReadLane,
            &[Value::from_inst(phi), Value::from_u32(1)],
        );
        let sink = append(&block, Opcode::BitCastF32U32, &[Value::from_inst(read)]);

        readlane_elimination_pass(&program(&block));
        let new_phi = sink.borrow().arg(0).inst();
        assert_eq!(new_phi.borrow().get_opcode(), Opcode::Phi);
        let values: Vec<usize> = (0..2)
            .map(|index| Rc::as_ptr(&new_phi.borrow().arg(index).inst()) as usize)
            .collect();
        assert!(values.contains(&(Rc::as_ptr(&left_data) as usize)));
        assert!(values.contains(&(Rc::as_ptr(&right_data) as usize)));
    }

    #[test]
    fn a_new_phi_with_one_value_folds_away() {
        let block = Block::new_ref();
        let left = Block::new_ref();
        let right = Block::new_ref();
        let chain = append(&block, Opcode::LaneId, &[]);
        let data = append(&block, Opcode::LaneId, &[]);
        let left_write = append(
            &left,
            Opcode::WriteLane,
            &[
                Value::from_inst(chain.clone()),
                Value::from_inst(data.clone()),
                Value::from_u32(1),
            ],
        );
        let right_write = append(
            &right,
            Opcode::WriteLane,
            &[
                Value::from_inst(chain),
                Value::from_inst(data.clone()),
                Value::from_u32(1),
            ],
        );
        let phi = append(&block, Opcode::Phi, &[]);
        add_phi_operand(&phi, &left, Value::from_inst(left_write));
        add_phi_operand(&phi, &right, Value::from_inst(right_write));
        let read = append(
            &block,
            Opcode::ReadLane,
            &[Value::from_inst(phi), Value::from_u32(1)],
        );
        let sink = append(&block, Opcode::BitCastF32U32, &[Value::from_inst(read)]);

        readlane_elimination_pass(&program(&block));
        assert!(Rc::ptr_eq(&sink.borrow().arg(0).inst(), &data));
    }
}
