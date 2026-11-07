//! Phi simplification, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/phi_simplification_pass.cpp`.
//!
//! Tarjan's algorithm groups the phi instructions into strongly connected components. An SCC
//! whose external arguments all pass through the same unary operation has that operation moved
//! to its users, and SCCs with the same structure and same external definitions are deduplicated.
//! The C++ uses `unordered_map`/`small_vector`; the port uses `HashMap`/`Vec`, with the same
//! visit and worklist orders.

use crate::gpu::common::hash::hash_combine_u64;
use crate::gpu::shader_recompiler::ir::basic_block::prepend_new_inst;
use crate::gpu::shader_recompiler::ir::microinstruction::{is_phi, set_arg, InstRef};
use crate::gpu::shader_recompiler::ir::opcodes::{arg_type_of, Opcode};
use crate::gpu::shader_recompiler::ir::passes::expression_elimination::run_local_cse;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::ssa::EMPTY_REG_TAG;
use crate::gpu::shader_recompiler::ir::value::Value;
use std::collections::HashMap;
use std::rc::Rc;

fn key(inst: &InstRef) -> usize {
    Rc::as_ptr(inst) as usize
}

/// `IsCompositeExtract`.
fn is_composite_extract(opcode: Opcode) -> bool {
    matches!(
        opcode,
        Opcode::CompositeExtractU32x2
            | Opcode::CompositeExtractU32x3
            | Opcode::CompositeExtractU32x4
    )
}

/// `IsDisallowedOp`.
fn is_disallowed_op(opcode: Opcode) -> bool {
    opcode == Opcode::InverseBallot
}

/// `CommonOp`.
#[derive(Clone, Copy)]
struct CommonOp {
    op: Opcode,
    extract_comp: u32,
}

impl Default for CommonOp {
    fn default() -> Self {
        CommonOp {
            op: Opcode::Void,
            extract_comp: 0,
        }
    }
}

/// `PhiScc`.
#[derive(Default)]
struct PhiScc {
    queued: bool,
    dead: bool,
    hash: u64,
    phis: Vec<InstRef>,
}

impl PhiScc {
    /// `HasCommonOp`.
    fn has_common_op(&self) -> Option<CommonOp> {
        let mut out = CommonOp::default();
        for phi in &self.phis {
            let num_args = phi.borrow().num_args();
            for index in 0..num_args {
                let arg_value = phi.borrow().arg(index);
                if arg_value.is_immediate() {
                    return None;
                }
                let arg = arg_value.inst();
                let opcode = arg.borrow().get_opcode();
                if opcode == Opcode::Phi {
                    if arg.borrow().scc_index == phi.borrow().scc_index {
                        continue;
                    }
                    return None;
                }
                if out.op == Opcode::Void {
                    out.op = opcode;
                    if is_composite_extract(opcode) {
                        out.extract_comp = arg.borrow().arg(1).u32();
                    } else if arg.borrow().num_args() != 1 || is_disallowed_op(opcode) {
                        return None;
                    }
                } else if opcode != out.op
                    || (is_composite_extract(out.op)
                        && arg.borrow().arg(1).u32() != out.extract_comp)
                {
                    return None;
                }
            }
        }
        assert_ne!(out.op, Opcode::Void);
        Some(out)
    }

    /// `SccUses`: the phi users outside this SCC.
    fn scc_uses(&self) -> Vec<InstRef> {
        let mut uses = Vec::new();
        for phi in &self.phis {
            for r#use in phi.borrow().uses() {
                if r#use.user.borrow().get_opcode() == Opcode::Phi
                    && r#use.user.borrow().scc_index != phi.borrow().scc_index
                {
                    uses.push(r#use.user.clone());
                }
            }
        }
        uses
    }

    /// `FoldCommonOp`: rewrites each phi's external arguments to the operation's operand, and
    /// moves one copy of the operation in front of the first non-phi for the external users.
    fn fold_common_op(&mut self, common: &CommonOp) {
        let new_type = arg_type_of(common.op, 0);
        for phi in &self.phis {
            let block = phi.borrow().parent().expect("the phi has a parent");
            let num_args = phi.borrow().num_args();
            for index in 0..num_args {
                let arg_value = phi.borrow().arg(index);
                let arg = arg_value.inst();
                if arg.borrow().get_opcode() == Opcode::Phi {
                    continue;
                }
                let replacement = arg.borrow().arg(0);
                set_arg(phi, index, replacement);
            }
            phi.borrow_mut().set_reg_tag(EMPTY_REG_TAG);
            phi.borrow_mut().set_type_flags(new_type);

            let mut replacement: Option<InstRef> = None;
            let uses = phi.borrow().uses().to_vec();
            for r#use in uses {
                if r#use.user.borrow().get_opcode() == Opcode::Phi
                    && r#use.user.borrow().scc_index == phi.borrow().scc_index
                {
                    continue;
                }
                let replacement = replacement.get_or_insert_with(|| {
                    let instructions = block.borrow().instructions.clone();
                    let point = instructions
                        .iter()
                        .position(|inst| !is_phi(inst))
                        .unwrap_or(instructions.len());
                    let args = if is_composite_extract(common.op) {
                        vec![
                            Value::from_inst(phi.clone()),
                            Value::from_u32(common.extract_comp),
                        ]
                    } else {
                        vec![Value::from_inst(phi.clone())]
                    };
                    let position = prepend_new_inst(&block, point, common.op, &args, 0);
                    block.borrow().instructions[position].clone()
                });
                set_arg(
                    &r#use.user,
                    r#use.operand as usize,
                    Value::from_inst(replacement.clone()),
                );
            }
        }
    }

    /// `ComputeStructureHash`.
    fn compute_structure_hash(&mut self) {
        let mut hash = hash_combine_u64(0, self.phis.len() as u64);
        for phi in &self.phis {
            let block = phi.borrow().parent().expect("the phi has a parent");
            hash = hash_combine_u64(hash, Rc::as_ptr(&block) as u64);
            let num_args = phi.borrow().num_args();
            for index in 0..num_args {
                let arg = phi.borrow().arg(index);
                hash = hash_combine_u64(hash, u64::from(arg.is_immediate()));
                if arg.is_immediate() {
                    hash = hash_combine_u64(hash, arg.hash_value());
                    continue;
                }
                let arg_inst = arg.inst();
                let is_scc_arg = arg_inst.borrow().get_opcode() == Opcode::Phi
                    && arg_inst.borrow().scc_index == phi.borrow().scc_index;
                hash = hash_combine_u64(hash, u64::from(is_scc_arg));
                if is_scc_arg {
                    let position = self
                        .phis
                        .iter()
                        .position(|candidate| Rc::ptr_eq(candidate, &arg_inst))
                        .expect("the SCC argument is in this SCC");
                    hash = hash_combine_u64(hash, position as u64);
                }
            }
        }
        self.hash = hash;
    }

    /// `IsSameAs`: the same blocks and argument structure, and the same external definitions.
    fn is_same_as(&self, other: &PhiScc) -> bool {
        if self.phis.len() != other.phis.len() {
            return false;
        }
        for (phi_a, phi_b) in self.phis.iter().zip(&other.phis) {
            let block_a = phi_a.borrow().parent().expect("the phi has a parent");
            let block_b = phi_b.borrow().parent().expect("the phi has a parent");
            if !Rc::ptr_eq(&block_a, &block_b) {
                return false;
            }
            let num_args = phi_a.borrow().num_args();
            for index in 0..num_args {
                let arg_a = phi_a.borrow().arg(index);
                let arg_b = phi_b.borrow().arg(index);
                if arg_a.is_immediate() != arg_b.is_immediate() {
                    return false;
                }
                if arg_a.is_immediate() {
                    if arg_a.hash_value() != arg_b.hash_value() {
                        return false;
                    }
                    continue;
                }
                let inst_a = arg_a.inst();
                let inst_b = arg_b.inst();
                let opcode_a = inst_a.borrow().get_opcode();
                let opcode_b = inst_b.borrow().get_opcode();
                if opcode_a != opcode_b {
                    return false;
                }
                if opcode_a == Opcode::Phi {
                    if inst_a.borrow().type_flags() != inst_b.borrow().type_flags() {
                        return false;
                    }
                    let is_scc_a = inst_a.borrow().scc_index == phi_a.borrow().scc_index;
                    let is_scc_b = inst_b.borrow().scc_index == phi_b.borrow().scc_index;
                    if is_scc_a != is_scc_b {
                        return false;
                    }
                    if is_scc_a {
                        let position_a = self
                            .phis
                            .iter()
                            .position(|candidate| Rc::ptr_eq(candidate, &inst_a))
                            .expect("the argument is in this SCC");
                        let position_b = other
                            .phis
                            .iter()
                            .position(|candidate| Rc::ptr_eq(candidate, &inst_b))
                            .expect("the argument is in the other SCC");
                        if position_a != position_b {
                            return false;
                        }
                        continue;
                    }
                }
                if arg_a != arg_b {
                    return false;
                }
            }
        }
        true
    }
}

/// `PhiSccFinder`'s `NodeInfo`.
#[derive(Clone, Copy)]
struct NodeInfo {
    index: u32,
    lowlink: u32,
    on_stack: bool,
}

impl Default for NodeInfo {
    fn default() -> Self {
        NodeInfo {
            index: u32::MAX,
            lowlink: u32::MAX,
            on_stack: false,
        }
    }
}

/// The `StrongConnect` frame.
struct Frame {
    phi: InstRef,
    arg_idx: usize,
}

/// `PhiSccFinder`: iterative Tarjan over the phi graph.
struct PhiSccFinder {
    nodes: HashMap<usize, NodeInfo>,
    stack: Vec<InstRef>,
    sccs: Vec<PhiScc>,
    index_counter: u32,
}

impl PhiSccFinder {
    fn new(all_phis: &[InstRef]) -> Self {
        let mut finder = PhiSccFinder {
            nodes: HashMap::with_capacity(all_phis.len()),
            stack: Vec::new(),
            sccs: Vec::new(),
            index_counter: 0,
        };
        for phi in all_phis {
            let index = finder.nodes.entry(key(phi)).or_default().index;
            if index == u32::MAX {
                finder.strong_connect(phi);
            }
        }
        for (index, scc) in finder.sccs.iter().enumerate() {
            for phi in &scc.phis {
                phi.borrow_mut().scc_index = index as u32;
            }
        }
        finder
    }

    fn push_phi(&mut self, phi: &InstRef, dfs: &mut Vec<Frame>) {
        let index = self.index_counter;
        self.index_counter += 1;
        let info = self.nodes.get_mut(&key(phi)).expect("the node");
        info.index = index;
        info.lowlink = index;
        info.on_stack = true;
        self.stack.push(phi.clone());
        dfs.push(Frame {
            phi: phi.clone(),
            arg_idx: 0,
        });
    }

    /// `StrongConnect`.
    fn strong_connect(&mut self, root: &InstRef) {
        let mut dfs: Vec<Frame> = Vec::new();
        self.push_phi(root, &mut dfs);

        while !dfs.is_empty() {
            let frame_index = dfs.len() - 1;
            let phi = dfs[frame_index].phi.clone();
            let mut arg_idx = dfs[frame_index].arg_idx;
            let num_args = phi.borrow().num_args();
            let mut pushed_child = false;

            while arg_idx < num_args {
                let arg_value = phi.borrow().arg(arg_idx);
                arg_idx += 1;
                if arg_value.is_immediate() {
                    continue;
                }
                let arg = arg_value.inst();
                if arg.borrow().get_opcode() != Opcode::Phi {
                    continue;
                }
                let arg_key = key(&arg);
                let (arg_index, arg_on_stack) = {
                    let info = self.nodes.entry(arg_key).or_default();
                    (info.index, info.on_stack)
                };
                if arg_index == u32::MAX {
                    self.push_phi(&arg, &mut dfs);
                    pushed_child = true;
                    break;
                } else if arg_on_stack {
                    let info = self.nodes.get_mut(&key(&phi)).expect("the node");
                    info.lowlink = info.lowlink.min(arg_index);
                }
            }
            dfs[frame_index].arg_idx = arg_idx;
            if pushed_child {
                continue;
            }

            let info = *self.nodes.get(&key(&phi)).expect("the node");
            if info.lowlink == info.index {
                let mut scc = PhiScc::default();
                loop {
                    let w = self.stack.pop().expect("the SCC stack is not empty");
                    self.nodes.get_mut(&key(&w)).expect("the node").on_stack = false;
                    scc.phis.push(w.clone());
                    if Rc::ptr_eq(&w, &phi) {
                        break;
                    }
                }
                self.sccs.push(scc);
            }

            dfs.pop();
            if let Some(parent) = dfs.last() {
                let parent_phi = parent.phi.clone();
                let child_lowlink = info.lowlink;
                let parent_info = self
                    .nodes
                    .get_mut(&key(&parent_phi))
                    .expect("the parent node");
                parent_info.lowlink = parent_info.lowlink.min(child_lowlink);
            }
        }
    }
}

/// `FoldSccCommonOps`.
fn fold_scc_common_ops(sccs: &mut [PhiScc]) {
    let mut worklist: Vec<u32> = Vec::with_capacity(sccs.len());
    for (index, scc) in sccs.iter_mut().enumerate() {
        scc.queued = true;
        worklist.push(index as u32);
    }

    while let Some(scc_index) = worklist.pop() {
        let index = scc_index as usize;
        sccs[index].queued = false;

        let Some(mut op) = sccs[index].has_common_op() else {
            continue;
        };
        let scc_uses = sccs[index].scc_uses();
        loop {
            sccs[index].fold_common_op(&op);
            match sccs[index].has_common_op() {
                Some(next) => op = next,
                None => break,
            }
        }

        for user in scc_uses {
            let user_scc = user.borrow().scc_index as usize;
            if !std::mem::replace(&mut sccs[user_scc].queued, true) {
                worklist.push(user_scc as u32);
            }
        }
    }
}

/// `DeduplicateSccs`'s group.
#[derive(Default)]
struct SccGroup {
    sccs: Vec<u32>,
    queued: bool,
}

/// `DeduplicateSccs`.
fn deduplicate_sccs(sccs: &mut [PhiScc]) {
    let mut groups: HashMap<u64, SccGroup> = HashMap::new();
    for (index, scc) in sccs.iter_mut().enumerate() {
        scc.compute_structure_hash();
        groups.entry(scc.hash).or_default().sccs.push(index as u32);
    }

    let mut worklist: Vec<u64> = Vec::new();
    for (hash, group) in groups.iter_mut() {
        if group.sccs.len() > 1 {
            group.queued = true;
            worklist.push(*hash);
        }
    }

    while let Some(hash) = worklist.pop() {
        let members = {
            let group = groups.get_mut(&hash).expect("the group");
            group.queued = false;
            group.sccs.clone()
        };
        for i in 0..members.len() {
            let scc_index = members[i] as usize;
            if sccs[scc_index].dead {
                continue;
            }
            for &remove_index in members.iter().skip(i + 1) {
                let remove_index = remove_index as usize;
                if sccs[remove_index].dead {
                    continue;
                }
                if !sccs[scc_index].is_same_as(&sccs[remove_index]) {
                    continue;
                }
                let old_phis = sccs[remove_index].phis.clone();
                let new_phis = sccs[scc_index].phis.clone();
                for (old_phi, new_phi) in old_phis.iter().zip(&new_phis) {
                    let uses = old_phi.borrow().uses().to_vec();
                    for r#use in uses {
                        let user_opcode = r#use.user.borrow().get_opcode();
                        if user_opcode == Opcode::Phi {
                            if r#use.user.borrow().scc_index == old_phi.borrow().scc_index {
                                continue;
                            }
                            let user_scc = r#use.user.borrow().scc_index as usize;
                            let user_hash = sccs[user_scc].hash;
                            let user_group = groups.entry(user_hash).or_default();
                            if !std::mem::replace(&mut user_group.queued, true) {
                                worklist.push(user_hash);
                            }
                        }
                        if user_opcode != Opcode::Void {
                            set_arg(
                                &r#use.user,
                                r#use.operand as usize,
                                Value::from_inst(new_phi.clone()),
                            );
                            let block = r#use.user.borrow().parent().expect("the use has a parent");
                            run_local_cse(&block);
                        }
                    }
                }
                sccs[remove_index].dead = true;
            }
        }
    }
}

/// `PhiSimplificationPass`.
pub fn phi_simplification_pass(program: &Program) {
    let mut all_phis: Vec<InstRef> = Vec::new();
    for block in &program.blocks {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            if inst.borrow().get_opcode() != Opcode::Phi {
                break;
            }
            all_phis.push(inst.clone());
        }
    }

    let mut finder = PhiSccFinder::new(&all_phis);
    fold_scc_common_ops(&mut finder.sccs);
    deduplicate_sccs(&mut finder.sccs);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block, BlockRef};
    use crate::gpu::shader_recompiler::ir::microinstruction::add_phi_operand;
    use crate::gpu::shader_recompiler::ir::r#type::Type;
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

    fn phi(block: &BlockRef, args: &[(&BlockRef, Value)], ty: Type) -> InstRef {
        let phi = append(block, Opcode::Phi, &[]);
        phi.borrow_mut().set_type_flags(ty);
        for (predecessor, value) in args {
            add_phi_operand(&phi, predecessor, value.clone());
        }
        phi
    }

    #[test]
    fn a_common_unary_op_moves_to_the_users() {
        // The casts live in the predecessor; the phi is the first instruction of its block.
        let pred = Block::new_ref();
        let x1 = append(&pred, Opcode::GetScc, &[]);
        let x2 = append(&pred, Opcode::GetScc, &[]);
        let cast1 = append(
            &pred,
            Opcode::BitCastU32F32,
            &[Value::from_inst(x1.clone())],
        );
        let cast2 = append(
            &pred,
            Opcode::BitCastU32F32,
            &[Value::from_inst(x2.clone())],
        );

        let block = Block::new_ref();
        let phi = phi(
            &block,
            &[
                (&pred, Value::from_inst(cast1)),
                (&pred, Value::from_inst(cast2)),
            ],
            Type::U32,
        );
        let sink = append(
            &block,
            Opcode::ConvertF16F32,
            &[Value::from_inst(phi.clone())],
        );

        phi_simplification_pass(&program(&block));

        // The phi now carries the operands themselves, with the operation's result type.
        assert!(Rc::ptr_eq(&phi.borrow().arg(0).inst(), &x1));
        assert!(Rc::ptr_eq(&phi.borrow().arg(1).inst(), &x2));
        assert_eq!(phi.borrow().type_flags(), Type::F32);

        // One cast remains: the one for the external user, inserted after the phi.
        let ops: Vec<Opcode> = block
            .borrow()
            .instructions()
            .iter()
            .map(|inst| inst.borrow().get_opcode())
            .collect();
        assert_eq!(
            &ops[..3],
            &[Opcode::Phi, Opcode::BitCastU32F32, Opcode::ConvertF16F32]
        );
        let replacement = block.borrow().instructions[1].clone();
        assert!(Rc::ptr_eq(&sink.borrow().arg(0).inst(), &replacement));
        assert!(Rc::ptr_eq(&replacement.borrow().arg(0).inst(), &phi));
    }

    #[test]
    fn identical_phi_cycles_are_deduplicated() {
        let block = Block::new_ref();
        let pred = Block::new_ref();
        // Two two-phi cycles with the same structure and the same external immediate.
        let a1 = phi(
            &block,
            &[(&pred, Value::empty()), (&pred, Value::from_u32(5))],
            Type::U32,
        );
        let a2 = phi(
            &block,
            &[
                (&pred, Value::from_inst(a1.clone())),
                (&pred, Value::from_u32(5)),
            ],
            Type::U32,
        );
        // Point a1's first operand at a2 to close the cycle; the second operand stays 5.
        let mut args = vec![Value::from_inst(a2.clone()), Value::from_u32(5)];
        a1.borrow_mut().set_type_flags(Type::U32);
        for (index, arg) in args.drain(..).enumerate() {
            set_arg(&a1, index, arg);
        }

        let b1 = phi(
            &block,
            &[(&pred, Value::empty()), (&pred, Value::from_u32(5))],
            Type::U32,
        );
        let b2 = phi(
            &block,
            &[
                (&pred, Value::from_inst(b1.clone())),
                (&pred, Value::from_u32(5)),
            ],
            Type::U32,
        );
        set_arg(&b1, 0, Value::from_inst(b2.clone()));
        let sink = append(
            &block,
            Opcode::ConvertF16F32,
            &[Value::from_inst(b1.clone())],
        );

        phi_simplification_pass(&program(&block));

        assert_eq!(a1.borrow().scc_index, a2.borrow().scc_index);
        assert_ne!(a1.borrow().scc_index, b1.borrow().scc_index);
        assert!(Rc::ptr_eq(&sink.borrow().arg(0).inst(), &a1));
    }
}
