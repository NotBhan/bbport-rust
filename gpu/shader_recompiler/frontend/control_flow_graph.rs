//! The GCN control flow graph, ported from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/control_flow_graph.h` and
//! `control_flow_graph.cpp`. The builder runs the same five steps as the C++: emit labels,
//! cut the instructions into blocks between them, link the blocks, split the divergence
//! scopes, and drop what nothing reaches.
//!
//! Two things differ, both because Rust has no raw pointers. `Block`'s links are handles
//! (`Rc<RefCell<Block>>`) rather than `Block*`, and the intrusive set the C++ keeps its blocks
//! in is a `Vec` that stays in the same order — insertion happens where the C++'s insertion
//! hint puts it, since the header's `operator<` orders only by `begin`. The C++ header declares
//! `Block::Contains` and never defines it, so the port has no `contains`. `LOG_DEBUG` and
//! `LOG_WARNING` calls are dropped: the port has no logging module.

use crate::gpu::common::object_pool::ObjectPool;
use crate::gpu::shader_recompiler::frontend::instruction::GcnInst;
use crate::gpu::shader_recompiler::frontend::opcodes::{InstCategory, Opcode, OperandField};
use crate::gpu::shader_recompiler::ir::basic_block::BlockRef as IrBlockRef;
use crate::gpu::shader_recompiler::ir::condition::Condition;
use std::cell::RefCell;
use std::rc::Rc;

/// A shared handle to a GCN block, the port's stand-in for `Shader::Gcn::Block*`.
pub type BlockRef = Rc<RefCell<Block>>;

/// `Shader::Gcn::EndClass`: how a block ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EndClass {
    /// Block ends with a (un)conditional branch.
    #[default]
    Branch,
    /// Block ends with an exit instruction.
    Exit,
}

/// `Shader::Gcn::Block`: a linear range of instructions. The C++'s `Hook` base is intrusive-set
/// plumbing with no Rust equivalent, so the port tracks the order in `CFG::blocks` instead.
#[derive(Debug, Clone, Default)]
pub struct Block {
    pub begin: u32,
    pub end: u32,
    pub begin_index: u32,
    pub end_index: u32,
    pub num_predecessors: u32,
    pub cond: Condition,
    pub end_inst: GcnInst,
    pub end_class: EndClass,
    pub branch_true: Option<BlockRef>,
    pub branch_false: Option<BlockRef>,
    /// The IR block this GCN block becomes, which the translation stage fills in.
    pub ir_block: Option<IrBlockRef>,
    pub is_dummy: bool,
}

impl Block {
    /// A shareable block, the port's `block_pool.Create()`.
    pub fn new_ref() -> BlockRef {
        Rc::new(RefCell::new(Block::default()))
    }

    /// The C++ header's `operator<`: blocks are ordered by where they begin.
    pub fn order_key(&self) -> u32 {
        self.begin
    }
}

/// `MakeCondition`: the condition a conditional branch tests.
pub fn make_condition(inst: &GcnInst) -> Condition {
    if inst.is_cmpx() {
        return Condition::Execnz;
    }
    match inst.opcode {
        Opcode::S_CBRANCH_SCC0 => Condition::Scc0,
        Opcode::S_CBRANCH_SCC1 => Condition::Scc1,
        Opcode::S_CBRANCH_VCCZ => Condition::Vccz,
        Opcode::S_CBRANCH_VCCNZ => Condition::Vccnz,
        Opcode::S_CBRANCH_EXECZ => Condition::Execz,
        Opcode::S_CBRANCH_EXECNZ => Condition::Execnz,
        _ => Condition::True,
    }
}

/// `IgnoresExecMask`: the scalar instructions, and the lane read/writes, run regardless of the
/// exec mask.
pub fn ignores_exec_mask(inst: &GcnInst) -> bool {
    // EXEC mask does not affect scalar instructions or branches.
    if matches!(
        inst.category,
        InstCategory::ScalarALU | InstCategory::ScalarMemory | InstCategory::FlowControl
    ) {
        return true;
    }
    // Read/Write Lane instructions are not affected either.
    matches!(
        inst.opcode,
        Opcode::V_READLANE_B32 | Opcode::V_WRITELANE_B32 | Opcode::V_READFIRSTLANE_B32
    )
}

/// `ResolveSetPcTarget`: the target of an `S_SETPC_B64` whose address was built three
/// instructions earlier. The C++ logs the resolved target; the port returns it.
pub fn resolve_set_pc_target(list: &[GcnInst], setpc_index: u32, pc_map: &[u32]) -> Option<u32> {
    if setpc_index < 3 {
        return None;
    }
    let setpc_index = setpc_index as usize;

    let getpc = list[setpc_index - 3];
    let arith = list[setpc_index - 2];
    let setpc = list[setpc_index];

    if getpc.opcode != Opcode::S_GETPC_B64
        || !(arith.opcode == Opcode::S_ADD_U32 || arith.opcode == Opcode::S_SUB_U32)
        || setpc.opcode != Opcode::S_SETPC_B64
    {
        return None;
    }

    if getpc.dst[0].code != setpc.src[0].code || arith.dst[0].code != setpc.src[0].code {
        return None;
    }

    if arith.src_count < 2 || arith.src[1].field != OperandField::LiteralConst.0 {
        return None;
    }

    let imm = arith.src[1].code;

    let signed_offset = if arith.opcode == Opcode::S_ADD_U32 {
        imm as i32
    } else {
        -(imm as i32)
    };

    let base_pc = pc_map[setpc_index - 3] + getpc.length;

    let result_pc = (base_pc as i32).wrapping_add(signed_offset) as u32;
    Some(result_pc & !0x3u32)
}

/// `static constexpr size_t LabelReserveSize`.
const LABEL_RESERVE_SIZE: usize = 32;

/// `Shader::Gcn::CFG`.
pub struct CFG {
    /// `block_pool`: the C++ holds a pool of `Block`s and hands out pointers into it. The port
    /// pools the handles, so the blocks a branch points at outlive their removal from `blocks`.
    pub block_pool: ObjectPool<BlockRef>,
    /// `inst_list`: the C++ keeps a span into the caller's list; the port owns the list.
    pub inst_list: Vec<GcnInst>,
    pub index_to_pc: Vec<u32>,
    pub labels: Vec<u32>,
    /// `blocks`: kept in the C++ set's order.
    pub blocks: Vec<BlockRef>,
}

impl CFG {
    /// `CFG::CFG`: build the graph for an instruction list.
    pub fn new(block_pool: ObjectPool<BlockRef>, inst_list: Vec<GcnInst>) -> Self {
        let mut cfg = CFG {
            block_pool,
            index_to_pc: vec![0; inst_list.len() + 1],
            inst_list,
            labels: Vec::with_capacity(LABEL_RESERVE_SIZE),
            blocks: Vec::new(),
        };
        cfg.emit_labels();
        cfg.emit_blocks();
        cfg.link_blocks();
        cfg.split_divergence_scopes();
        cfg.remove_unreachable_blocks();
        cfg
    }

    /// `CFG::begin`.
    pub fn blocks(&self) -> &[BlockRef] {
        &self.blocks
    }

    /// `CFG::AddLabel`: labels are unique, and sorted once every one is known.
    fn add_label(&mut self, address: u32) {
        if !self.labels.contains(&address) {
            self.labels.push(address);
        }
    }

    /// `CFG::GetIndex`: the instruction index a pc maps to.
    fn get_index(&self, label: u32) -> usize {
        if label == 0 {
            return 0;
        }
        let index = self.index_to_pc.partition_point(|&pc| pc < label);
        assert!(
            index < self.index_to_pc.len() || label > *self.index_to_pc.last().unwrap(),
            "label {label:#x} is past the end of the instruction map"
        );
        index
    }

    /// `LinkBlocks`'s `get_block`: the block that begins at an address.
    fn get_block(&self, address: u32) -> BlockRef {
        self.blocks
            .iter()
            .find(|block| block.borrow().begin == address)
            .unwrap_or_else(|| panic!("no block begins at {address:#x}"))
            .clone()
    }

    /// `CFG::EmitLabels`.
    fn emit_labels(&mut self) {
        // Always set a label at entry point.
        let mut pc = 0u32;
        self.add_label(pc);

        // Iterate instruction list and add labels to branch targets.
        for i in 0..self.inst_list.len() {
            self.index_to_pc[i] = pc;
            let inst = self.inst_list[i];
            if inst.is_unconditional_branch() {
                let mut target = inst.branch_target(pc);
                if inst.opcode == Opcode::S_SETPC_B64 {
                    match resolve_set_pc_target(&self.inst_list, i as u32, &self.index_to_pc) {
                        Some(resolved) => target = resolved,
                        None => panic!(
                            "S_SETPC_B64 without a resolvable offset at PC {pc:#x} (Index {i}): \
                             Involved instructions not recognized or invalid pattern"
                        ),
                    }
                }
                self.add_label(target);
                // Emit this label so that the block ends with the branching instruction
                self.add_label(pc + inst.length);
            } else if inst.is_conditional_branch() {
                let true_label = inst.branch_target(pc);
                let false_label = pc + inst.length;
                if true_label != false_label {
                    self.add_label(true_label);
                    self.add_label(false_label);
                }
            } else if inst.opcode == Opcode::S_ENDPGM {
                let next_label = pc + inst.length;
                self.add_label(next_label);
            }

            pc += inst.length;
        }
        self.index_to_pc[self.inst_list.len()] = pc;

        // Sort labels to make sure block insertion is correct.
        self.labels.sort_unstable();
    }

    /// `CFG::EmitBlocks`.
    fn emit_blocks(&mut self) {
        for i in 0..self.labels.len() {
            let start = self.labels[i];
            if i + 1 == self.labels.len() {
                // Last label is special.
                return;
            }
            // The end label is the start instruction of next block.
            // The end instruction of this block is the previous one.
            let end = self.labels[i + 1];
            let end_index = self.get_index(end) - 1;
            let end_inst = self.inst_list[end_index];

            // Insert block between the labels using the last instruction
            // as an indicator for branching type.
            let block = self.block_pool.create(Block::new_ref()).clone();
            {
                let mut block = block.borrow_mut();
                block.begin = start;
                block.end = end;
                block.begin_index = self.get_index(start) as u32;
                block.end_index = end_index as u32;
                block.end_inst = end_inst;
                block.cond = make_condition(&end_inst);
            }
            self.blocks.push(block);
        }
    }

    /// `CFG::LinkBlocks`.
    fn link_blocks(&mut self) {
        let blocks = self.blocks.clone();
        for block in blocks {
            let end_inst = block.borrow().end_inst;

            // If the block doesn't end with a branch we simply
            // need to link with the next block.
            if !end_inst.is_terminate_instruction() {
                let next_block = self.get_block(block.borrow().end);
                block.borrow_mut().branch_true = Some(next_block.clone());
                block.borrow_mut().end_class = EndClass::Branch;
                next_block.borrow_mut().num_predecessors += 1;
                continue;
            }

            // Find the branch targets from the instruction and link the blocks.
            // Note: Block end address is one instruction after end_inst.
            let branch_pc = block.borrow().end - end_inst.length;
            let target_pc = if end_inst.opcode == Opcode::S_SETPC_B64 {
                let end_index = block.borrow().end_index;
                match resolve_set_pc_target(&self.inst_list, end_index, &self.index_to_pc) {
                    Some(target) => target,
                    None => panic!(
                        "S_SETPC_B64 without a resolvable offset at PC {branch_pc:#x} (Index \
                         {end_index}): Involved instructions not recognized or invalid pattern"
                    ),
                }
            } else {
                end_inst.branch_target(branch_pc)
            };

            if end_inst.is_unconditional_branch() {
                let target_block = self.get_block(target_pc);
                block.borrow_mut().branch_true = Some(target_block.clone());
                block.borrow_mut().end_class = EndClass::Branch;
                target_block.borrow_mut().num_predecessors += 1;
            } else if end_inst.is_conditional_branch() {
                let end_block = self.get_block(block.borrow().end);
                block.borrow_mut().end_class = EndClass::Branch;
                if target_pc != block.borrow().end {
                    let target_block = self.get_block(target_pc);
                    block.borrow_mut().branch_true = Some(target_block.clone());
                    block.borrow_mut().branch_false = Some(end_block.clone());
                    target_block.borrow_mut().num_predecessors += 1;
                } else {
                    block.borrow_mut().branch_true = Some(end_block.clone());
                    block.borrow_mut().cond = Condition::True;
                }
                end_block.borrow_mut().num_predecessors += 1;
            } else if end_inst.opcode == Opcode::S_ENDPGM {
                block.borrow_mut().end_class = EndClass::Exit;
            } else {
                panic!("Unreachable");
            }
        }
    }

    /// `CFG::SplitDivergenceScopes`.
    fn split_divergence_scopes(&mut self) {
        // An open scope instruction is an instruction that modifies EXEC
        // but also saves the previous value to restore later. This indicates
        // we are entering a scope.
        let is_open_scope = |inst: &GcnInst| {
            inst.opcode == Opcode::S_AND_SAVEEXEC_B64
                // While this instruction does not save EXEC it is often used paired
                // with SAVEEXEC to mask the threads that didn't pass the condition
                // of initial branch.
                || (inst.opcode == Opcode::S_ANDN2_B64
                    && inst.dst[0].field == OperandField::ExecLo.0)
                || inst.is_cmpx()
        };
        // Closing an EXEC scope can be either a branch instruction
        // (typical case when S_AND_SAVEEXEC_B64 is right before a branch)
        // or by a move instruction to EXEC that restores the backup.
        let is_close_scope = |inst: &GcnInst| {
            (inst.opcode == Opcode::S_MOV_B64 && inst.dst[0].field == OperandField::ExecLo.0)
                // Sometimes compiler might insert instructions between the SAVEEXEC and the
                // branch. Those instructions need to be wrapped in the condition as well so
                // allow branch as end scope instruction.
                || inst.opcode == Opcode::S_CBRANCH_EXECZ
                || inst.opcode == Opcode::S_ENDPGM
                || (inst.opcode == Opcode::S_ANDN2_B64
                    && inst.dst[0].field == OperandField::ExecLo.0)
        };

        // The C++ walks the set while inserting into it; the port walks the list by index and
        // inserts the new blocks right after the current one, which is where the insertion hint
        // puts them. The next iteration then visits the divergence block, as the C++ does.
        let mut block_index = 0usize;
        while block_index < self.blocks.len() {
            let blk = self.blocks[block_index].clone();
            let mut curr_begin: i32 = -1;
            let mut index = blk.borrow().begin_index;
            loop {
                let block_end_index = blk.borrow().end_index;
                if index > block_end_index {
                    break;
                }
                let inst = self.inst_list[index as usize];
                let is_close = is_close_scope(&inst);
                if (is_close || index == block_end_index) && curr_begin != -1 {
                    // If there are no instructions inside scope don't do anything.
                    if index as i32 - curr_begin == 1 && is_close {
                        curr_begin = -1;
                        index += 1;
                        continue;
                    }
                    // If all instructions in the scope ignore exec masking, we shouldn't insert
                    // a scope. The C++ walks `[curr_begin + 1, index + !is_close]`.
                    let scope_end = index as usize + usize::from(!is_close);
                    let ignore_all = self.inst_list[curr_begin as usize + 1..scope_end]
                        .iter()
                        .all(ignores_exec_mask);
                    if !ignore_all {
                        // Determine the first instruction affected by the exec mask.
                        loop {
                            curr_begin += 1;
                            if !ignores_exec_mask(&self.inst_list[curr_begin as usize]) {
                                break;
                            }
                        }

                        // Determine the last instruction affected by the exec mask.
                        let mut curr_end = index as i32;
                        while ignores_exec_mask(&self.inst_list[curr_end as usize]) {
                            curr_end -= 1;
                        }

                        // Create a new block for the divergence scope.
                        let block = self.block_pool.create(Block::new_ref()).clone();
                        {
                            let mut block = block.borrow_mut();
                            block.begin = self.index_to_pc[curr_begin as usize];
                            block.end = self.index_to_pc[curr_end as usize];
                            block.begin_index = curr_begin as u32;
                            block.end_index = curr_end as u32;
                            block.end_inst = self.inst_list[curr_end as usize];
                            block.num_predecessors = 1;
                        }

                        // If we are inside the parent block, make an epilogue block and jump
                        // to it.
                        let epilogue = if curr_end != blk.borrow().end_index as i32 {
                            let epi_block = self.block_pool.create(Block::new_ref()).clone();
                            {
                                let parent = blk.borrow();
                                let mut epi = epi_block.borrow_mut();
                                epi.begin = self.index_to_pc[curr_end as usize + 1];
                                epi.end = parent.end;
                                epi.begin_index = curr_end as u32 + 1;
                                epi.end_index = parent.end_index;
                                epi.end_inst = parent.end_inst;
                                epi.cond = parent.cond;
                                epi.end_class = parent.end_class;
                                epi.branch_true = parent.branch_true.clone();
                                epi.branch_false = parent.branch_false.clone();
                                epi.num_predecessors = 2;
                            }

                            // Have divergence block always jump to epilogue block.
                            block.borrow_mut().cond = Condition::True;
                            block.borrow_mut().branch_true = Some(epi_block.clone());
                            block.borrow_mut().branch_false = None;

                            // If the parent block fails to enter divergence block make it jump
                            // to epilogue too
                            blk.borrow_mut().branch_false = Some(epi_block.clone());
                            Some(epi_block)
                        } else {
                            // No epilogue block is needed since the divergence block
                            // also ends the parent block. Inherit the end condition.
                            assert!(
                                blk.borrow().cond == Condition::True
                                    && blk.borrow().branch_true.is_some(),
                                "the parent block has to end in a branch"
                            );
                            block.borrow_mut().cond = Condition::True;
                            block.borrow_mut().branch_true = blk.borrow().branch_true.clone();
                            block.borrow_mut().branch_false = None;

                            // If the parent block didn't enter the divergence scope
                            // have it jump directly to the next one
                            let next_true = blk.borrow().branch_true.clone().unwrap();
                            blk.borrow_mut().branch_false = Some(next_true.clone());
                            next_true.borrow_mut().num_predecessors += 1;
                            None
                        };

                        // Shrink parent block to end right before curr_begin
                        // and make it jump to divergence block
                        curr_begin -= 1;
                        {
                            let mut parent = blk.borrow_mut();
                            parent.end = self.index_to_pc[curr_begin as usize];
                            parent.end_index = curr_begin as u32;
                            parent.end_inst = self.inst_list[curr_begin as usize];
                            parent.cond = Condition::Execnz;
                            parent.end_class = EndClass::Branch;
                            parent.branch_true = Some(block.clone());
                        }

                        // Insert the new blocks right after the parent, which is where the
                        // C++'s insertion hint places them.
                        self.blocks.insert(block_index + 1, block);
                        if let Some(epilogue) = epilogue {
                            self.blocks.insert(block_index + 2, epilogue);
                        }
                    }
                    // Reset scope begin.
                    curr_begin = -1;
                }
                // Mark a potential start of an exec scope.
                if is_open_scope(&inst) {
                    curr_begin = index as i32;
                }
                index += 1;
            }
            block_index += 1;
        }
    }

    /// `CFG::RemoveUnreachableBlocks`. The C++ logs each block it removes.
    fn remove_unreachable_blocks(&mut self) {
        let mut index = 1;
        while index < self.blocks.len() {
            if self.blocks[index].borrow().num_predecessors == 0 {
                self.blocks.remove(index);
            } else {
                index += 1;
            }
        }
    }

    /// `CFG::Dot`: the graph in the dot language.
    pub fn dot(&self) -> String {
        let mut node_uid = 0;

        let name_of = |block: &Block| format!("\"{:#x}\"", block.begin);

        let mut dot = String::from("digraph shader {\n");
        dot += &format!("\tsubgraph cluster_{} {{\n", 0);
        dot += "\t\tnode [style=filled];\n";
        for block in &self.blocks {
            let block = block.borrow();
            let name = name_of(&block);
            let add_branch = |dot: &mut String, branch: &Block, add_label: bool| {
                dot.push_str(&format!("\t\t{}->{}", name, name_of(branch)));
                if add_label && block.cond != Condition::True && block.cond != Condition::False {
                    dot.push_str(&format!(" [label=\"{}\"]", block.cond));
                }
                dot.push('\n');
            };
            dot += &format!("\t\t{};\n", name);
            match block.end_class {
                EndClass::Branch => {
                    if block.cond != Condition::False {
                        if let Some(branch_true) = &block.branch_true {
                            add_branch(&mut dot, &branch_true.borrow(), true);
                        }
                    }
                    if block.cond != Condition::True {
                        if let Some(branch_false) = &block.branch_false {
                            add_branch(&mut dot, &branch_false.borrow(), false);
                        }
                    }
                }
                EndClass::Exit => {
                    dot += &format!("\t\t{}->N{};\n", name, node_uid);
                    dot += &format!(
                        "\t\tN{} [label=\"Exit\"][shape=square][style=stripped];\n",
                        node_uid
                    );
                    node_uid += 1;
                }
            }
        }
        dot += "\t\tlabel = \"main\";\n\t}\n";
        if self.blocks.is_empty() {
            dot += "Start;\n";
        } else {
            dot += &format!("\tStart -> {};\n", name_of(&self.blocks[0].borrow()));
        }
        dot += "\tStart [shape=diamond];\n";
        dot += "}\n";
        dot
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::frontend::instruction::InstControl;
    use crate::gpu::shader_recompiler::frontend::opcodes::{InstClass, InstEncoding};

    /// An instruction of `length` words with an opcode and a category.
    fn inst(opcode: Opcode, length: u32, category: InstCategory) -> GcnInst {
        GcnInst {
            opcode,
            encoding: InstEncoding::ILLEGAL,
            inst_class: InstClass::Undefined,
            category,
            length,
            ..GcnInst::default()
        }
    }

    /// A scalar instruction, four words long, which the exec mask does not affect.
    fn scalar(opcode: Opcode) -> GcnInst {
        inst(opcode, 4, InstCategory::ScalarALU)
    }

    /// A vector instruction, four words long, which the exec mask does affect.
    fn vector(opcode: Opcode) -> GcnInst {
        inst(opcode, 4, InstCategory::VectorALU)
    }

    /// A branch with an offset in instructions.
    fn branch(opcode: Opcode, offset: i16) -> GcnInst {
        let mut instruction = inst(opcode, 4, InstCategory::FlowControl);
        instruction.control = InstControl::from_raw(offset as u16 as u64);
        instruction
    }

    fn cfg(instructions: Vec<GcnInst>) -> CFG {
        CFG::new(ObjectPool::new(4), instructions)
    }

    fn ends(cfg: &CFG) -> Vec<(u32, u32)> {
        cfg.blocks
            .iter()
            .map(|block| {
                let block = block.borrow();
                (block.begin, block.end)
            })
            .collect()
    }

    #[test]
    fn a_straight_line_becomes_one_block_ending_at_the_exit() {
        // The exit's own label is the last one, so the single block runs to the end of the list.
        let mut list = vec![scalar(Opcode::S_MOV_B32); 4];
        list.push(scalar(Opcode::S_ENDPGM));
        let cfg = cfg(list);
        assert_eq!(
            cfg.labels,
            vec![0, 20],
            "entry, and the word after the exit"
        );
        assert_eq!(ends(&cfg), vec![(0, 20)]);
        let block = cfg.blocks[0].borrow();
        assert_eq!(block.begin_index, 0);
        assert_eq!(block.end_index, 4);
        assert_eq!(block.end_class, EndClass::Exit);
        assert_eq!(block.cond, Condition::True);
        assert!(block.branch_true.is_none());
        assert_eq!(block.num_predecessors, 0);
    }

    #[test]
    fn an_unconditional_branch_ends_its_block_and_links_the_target() {
        // 0: branch over the instruction at 4, to the exit at 8.
        let list = vec![
            branch(Opcode::S_BRANCH, 1),
            scalar(Opcode::S_MOV_B32),
            scalar(Opcode::S_ENDPGM),
        ];
        let cfg = cfg(list);
        // The branch's target, the word after the branch, and the word after the exit all get
        // a label.
        assert_eq!(cfg.labels, vec![0, 4, 8, 12]);
        let first = cfg.blocks[0].borrow();
        assert_eq!(first.end_class, EndClass::Branch);
        assert_eq!(first.cond, Condition::True);
        let target = first.branch_true.clone().unwrap();
        assert_eq!(target.borrow().begin, 8);
        assert_eq!(target.borrow().end_index, 2, "the exit instruction");
        // The block the branch jumps over is unreachable and dropped. Its link into the exit
        // block had already been recorded, so that count stays.
        assert_eq!(ends(&cfg), vec![(0, 4), (8, 12)]);
        assert_eq!(target.borrow().num_predecessors, 2);
        assert_eq!(
            cfg.block_pool.created(),
            3,
            "the pool still owns the dropped block"
        );
    }

    #[test]
    fn a_conditional_branch_has_two_sides() {
        let list = vec![
            branch(Opcode::S_CBRANCH_SCC1, 1),
            scalar(Opcode::S_MOV_B32),
            scalar(Opcode::S_ENDPGM),
        ];
        let cfg = cfg(list);
        assert_eq!(cfg.labels, vec![0, 4, 8, 12]);
        assert_eq!(cfg.blocks.len(), 3);
        let first = cfg.blocks[0].borrow();
        assert_eq!(first.cond, Condition::Scc1);
        assert_eq!(first.end_class, EndClass::Branch);
        assert_eq!(first.branch_true.clone().unwrap().borrow().begin, 8);
        assert_eq!(first.branch_false.clone().unwrap().borrow().begin, 4);
        assert_eq!(
            first.branch_true.clone().unwrap().borrow().num_predecessors,
            2,
            "the branch and the fall-through block both reach it"
        );
        assert_eq!(
            first
                .branch_false
                .clone()
                .unwrap()
                .borrow()
                .num_predecessors,
            1
        );
        assert_eq!(
            first.branch_false.clone().unwrap().borrow().end_class,
            EndClass::Branch
        );
    }

    #[test]
    fn a_branch_to_its_own_fallthrough_is_conditionless() {
        // The first branch jumps to pc 12; the second one at pc 8 falls through to the same
        // place, so it has one target and no condition of its own.
        let list = vec![
            branch(Opcode::S_CBRANCH_EXECZ, 2),
            scalar(Opcode::S_MOV_B32),
            branch(Opcode::S_CBRANCH_SCC1, 0),
            scalar(Opcode::S_ENDPGM),
        ];
        let cfg = cfg(list);
        assert_eq!(cfg.labels, vec![0, 4, 12, 16]);
        let first = cfg.blocks[0].borrow();
        assert_eq!(first.cond, Condition::Execz);
        assert_eq!(first.branch_true.clone().unwrap().borrow().begin, 12);
        assert_eq!(first.branch_false.clone().unwrap().borrow().begin, 4);
        let second = cfg.blocks[1].borrow();
        assert_eq!(second.end_index, 2, "the second branch ends this block");
        assert_eq!(second.cond, Condition::True, "one target, so no condition");
        assert_eq!(second.end_class, EndClass::Branch);
        assert_eq!(second.branch_true.clone().unwrap().borrow().begin, 12);
        assert!(
            second.branch_false.is_none(),
            "the C++ leaves the second side unset when both targets are the same"
        );
        assert_eq!(
            second
                .branch_true
                .clone()
                .unwrap()
                .borrow()
                .num_predecessors,
            2,
            "the first branch and this one both reach the target"
        );
    }

    #[test]
    fn a_cmpx_is_not_a_branch_to_the_block_builder() {
        // The compare-and-write-instructions only carry a condition; the branch builder does
        // not treat them as branches, which is why `MakeCondition` answers for them.
        let cmpx = inst(Opcode::V_CMPX_F_F32, 4, InstCategory::VectorALU);
        assert!(cmpx.is_cmpx());
        assert!(!cmpx.is_conditional_branch());
        assert!(!cmpx.is_terminate_instruction());
        assert_eq!(make_condition(&cmpx), Condition::Execnz);
    }

    #[test]
    fn a_setpc_pattern_resolves_its_target() {
        // S_GETPC_B64 s0; S_ADD_U32 s0, s0, 24; S_MOV_B32; S_SETPC_B64 s0.
        let mut getpc = scalar(Opcode::S_GETPC_B64);
        getpc.dst[0].code = 0;
        let mut add = scalar(Opcode::S_ADD_U32);
        add.dst[0].code = 0;
        add.src_count = 2;
        add.src[1].field = OperandField::LiteralConst.0;
        add.src[1].code = 24;
        let mut setpc = scalar(Opcode::S_SETPC_B64);
        setpc.src[0].code = 0;
        let list = vec![
            getpc,
            add,
            scalar(Opcode::S_MOV_B32),
            setpc,
            scalar(Opcode::S_ENDPGM),
        ];

        // The getpc sits at index 0, so the target is its pc plus its length plus the
        // immediate.
        let pc_map: Vec<u32> = vec![0, 4, 8, 12, 16, 20];
        assert_eq!(resolve_set_pc_target(&list, 3, &pc_map), Some(28));
        // A subtract walks the other way, and the result is rounded down to a word.
        let mut sub = add;
        sub.opcode = Opcode::S_SUB_U32;
        let mut list_sub = list.clone();
        list_sub[1] = sub;
        assert_eq!(
            resolve_set_pc_target(&list_sub, 3, &pc_map),
            Some(4u32.wrapping_sub(24) & !3)
        );
        // The three registers have to agree.
        let mut list_bad = list.clone();
        list_bad[1].dst[0].code = 5;
        assert_eq!(resolve_set_pc_target(&list_bad, 3, &pc_map), None);
        let mut list_bad = list.clone();
        list_bad[3].src[0].code = 5;
        assert_eq!(resolve_set_pc_target(&list_bad, 3, &pc_map), None);
        // The arithmetic has to add a literal.
        let mut list_bad = list.clone();
        list_bad[1].src[1].field = OperandField::ScalarGPR.0;
        assert_eq!(resolve_set_pc_target(&list_bad, 3, &pc_map), None);
        let mut list_bad = list.clone();
        list_bad[1].src_count = 1;
        assert_eq!(resolve_set_pc_target(&list_bad, 3, &pc_map), None);
        // A pattern that starts before index 3 has nothing to resolve.
        assert_eq!(resolve_set_pc_target(&list, 2, &pc_map), None);
    }

    #[test]
    fn a_setpc_jump_uses_the_resolved_target() {
        let mut getpc = scalar(Opcode::S_GETPC_B64);
        getpc.dst[0].code = 0;
        let mut add = scalar(Opcode::S_ADD_U32);
        add.dst[0].code = 0;
        add.src_count = 2;
        add.src[1].field = OperandField::LiteralConst.0;
        add.src[1].code = 16;
        let mut setpc = scalar(Opcode::S_SETPC_B64);
        setpc.src[0].code = 0;
        let list = vec![
            getpc,
            add,
            scalar(Opcode::S_MOV_B32),
            setpc,
            scalar(Opcode::S_MOV_B32),
            scalar(Opcode::S_ENDPGM),
        ];
        let cfg = cfg(list);
        // The getpc at pc 0 is four words long, so the jump goes to 4 + 16, which is the exit.
        let first = cfg.blocks[0].borrow();
        assert_eq!(first.branch_true.clone().unwrap().borrow().begin, 20);
    }

    #[test]
    fn a_divergence_scope_becomes_its_own_block() {
        // S_AND_SAVEEXEC_B64, two vector instructions, S_MOV_B64 exec, S_ENDPGM.
        let mut saveexec = scalar(Opcode::S_AND_SAVEEXEC_B64);
        saveexec.dst[0].field = OperandField::ExecLo.0;
        let mut restore = scalar(Opcode::S_MOV_B64);
        restore.dst[0].field = OperandField::ExecLo.0;
        let list = vec![
            saveexec,
            vector(Opcode::V_MOV_B32),
            vector(Opcode::V_MOV_B32),
            restore,
            scalar(Opcode::S_ENDPGM),
        ];
        let cfg = cfg(list);
        // The parent is cut to end at the SAVEEXEC and branches into the scope; the scope
        // holds the two vector instructions and hands off to an epilogue.
        assert_eq!(cfg.blocks.len(), 3, "parent, scope, epilogue");
        let parent = cfg.blocks[0].borrow();
        assert_eq!((parent.begin_index, parent.end_index), (0, 0));
        assert_eq!(parent.cond, Condition::Execnz);
        assert_eq!(parent.end_class, EndClass::Branch);
        let scope = parent.branch_true.clone().unwrap();
        assert_eq!(scope.borrow().begin_index, 1);
        assert_eq!(scope.borrow().end_index, 2);
        assert_eq!(scope.borrow().cond, Condition::True);
        assert_eq!(scope.borrow().num_predecessors, 1);
        assert!(scope.borrow().branch_false.is_none());
        let epilogue = scope.borrow().branch_true.clone().unwrap();
        assert_eq!(epilogue.borrow().begin_index, 3);
        assert_eq!(epilogue.borrow().end_index, 4);
        assert_eq!(epilogue.borrow().num_predecessors, 2);
        assert!(Rc::ptr_eq(&epilogue, &parent.branch_false.clone().unwrap()));
        assert_eq!(epilogue.borrow().end_class, EndClass::Exit);
        // The parent still ends on the SAVEEXEC, whose pc is where the scope begins.
        assert_eq!(parent.end_index, 0);
        assert_eq!(scope.borrow().begin, 4);
    }

    #[test]
    fn a_scope_that_reaches_the_end_of_its_block_inherits_the_branch() {
        // The scope's last affected instruction is also the last instruction of its block, so
        // the divergence block takes over the parent's branch instead of getting an epilogue.
        let mut saveexec = scalar(Opcode::S_AND_SAVEEXEC_B64);
        saveexec.dst[0].field = OperandField::ExecLo.0;
        let list = vec![
            branch(Opcode::S_CBRANCH_SCC1, 2),
            saveexec,
            vector(Opcode::V_MOV_B32),
            vector(Opcode::V_MOV_B32),
            scalar(Opcode::S_ENDPGM),
        ];
        let cfg = cfg(list);
        assert_eq!(cfg.labels, vec![0, 4, 12, 20]);
        assert_eq!(cfg.blocks.len(), 4, "branch block, parent, scope, exit");
        let parent = cfg.blocks[1].borrow();
        assert_eq!(parent.begin_index, 1);
        assert_eq!(parent.end_index, 1, "the parent ends at the SAVEEXEC");
        assert_eq!(parent.cond, Condition::Execnz);
        let scope = parent.branch_true.clone().unwrap();
        assert_eq!(scope.borrow().begin_index, 2);
        assert_eq!(scope.borrow().end_index, 2);
        assert_eq!(scope.borrow().cond, Condition::True);
        assert!(scope.borrow().branch_false.is_none());
        // The parent is redirected to whatever the scope falls through to.
        assert!(Rc::ptr_eq(
            &scope.borrow().branch_true.clone().unwrap(),
            &parent.branch_false.clone().unwrap()
        ));
        let exit = scope.borrow().branch_true.clone().unwrap();
        assert_eq!(exit.borrow().begin, 12);
        assert_eq!(
            exit.borrow().num_predecessors,
            3,
            "the branch, the fall-through and the parent all reach it"
        );
    }

    #[test]
    fn a_scope_that_only_ignores_the_exec_mask_is_not_split() {
        let mut saveexec = scalar(Opcode::S_AND_SAVEEXEC_B64);
        saveexec.dst[0].field = OperandField::ExecLo.0;
        let list = vec![
            saveexec,
            scalar(Opcode::S_MOV_B32),
            scalar(Opcode::S_ENDPGM),
        ];
        let cfg = cfg(list);
        assert_eq!(cfg.blocks.len(), 1);
        assert_eq!(cfg.blocks[0].borrow().end_index, 2);
    }

    #[test]
    fn an_empty_scope_is_left_alone() {
        let mut saveexec = scalar(Opcode::S_AND_SAVEEXEC_B64);
        saveexec.dst[0].field = OperandField::ExecLo.0;
        let mut restore = scalar(Opcode::S_MOV_B64);
        restore.dst[0].field = OperandField::ExecLo.0;
        let list = vec![saveexec, restore, scalar(Opcode::S_ENDPGM)];
        let cfg = cfg(list);
        assert_eq!(cfg.blocks.len(), 1, "no instruction sits inside the scope");
    }

    #[test]
    fn the_ignored_exec_mask_set_comes_from_the_header() {
        assert!(ignores_exec_mask(&scalar(Opcode::S_MOV_B32)));
        assert!(ignores_exec_mask(&inst(
            Opcode::V_READLANE_B32,
            8,
            InstCategory::VectorALU
        )));
        assert!(!ignores_exec_mask(&vector(Opcode::V_MOV_B32)));
        let mut mov = vector(Opcode::V_MOV_B32);
        mov.category = InstCategory::VectorMemory;
        assert!(!ignores_exec_mask(&mov));
    }

    #[test]
    fn the_condition_table_follows_the_branch_opcode() {
        for (opcode, condition) in [
            (Opcode::S_CBRANCH_SCC0, Condition::Scc0),
            (Opcode::S_CBRANCH_SCC1, Condition::Scc1),
            (Opcode::S_CBRANCH_VCCZ, Condition::Vccz),
            (Opcode::S_CBRANCH_VCCNZ, Condition::Vccnz),
            (Opcode::S_CBRANCH_EXECZ, Condition::Execz),
            (Opcode::S_CBRANCH_EXECNZ, Condition::Execnz),
            (Opcode::S_BRANCH, Condition::True),
        ] {
            assert_eq!(make_condition(&branch(opcode, 0)), condition);
        }
    }

    #[test]
    fn the_dot_output_names_the_blocks_and_their_sides() {
        let list = vec![
            branch(Opcode::S_CBRANCH_EXECZ, 2),
            scalar(Opcode::S_MOV_B32),
            branch(Opcode::S_CBRANCH_SCC1, 0),
            scalar(Opcode::S_ENDPGM),
        ];
        let cfg = cfg(list);
        let dot = cfg.dot();
        assert!(
            dot.starts_with("digraph shader {\n\tsubgraph cluster_0 {\n"),
            "{dot}"
        );
        assert!(dot.contains("\t\t\"0x0\";"), "{dot}");
        assert!(dot.contains("\"0x0\"->\"0xc\" [label=\"Execz\"]"), "{dot}");
        assert!(dot.contains("\"0x0\"->\"0x4\""), "{dot}");
        // The second branch has a true condition, so its edge carries no label.
        assert!(dot.contains("\"0x4\"->\"0xc\"\n"), "{dot}");
        assert!(!dot.contains("\"0x4\"->\"0xc\" [label="), "{dot}");
        assert!(dot.contains("\"0xc\"->N0;"), "{dot}");
        assert!(
            dot.contains("N0 [label=\"Exit\"][shape=square][style=stripped];"),
            "{dot}"
        );
        assert!(dot.contains("\tStart -> \"0x0\";"), "{dot}");
        assert!(dot.ends_with("}\n"), "{dot}");
    }
}
