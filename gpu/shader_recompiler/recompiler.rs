//! The shader translation driver, ported from
//! `legacy/gpu/shadps4/shader_recompiler/recompiler.cpp` and `recompiler.h`.
//!
//! The C++ keeps instruction and block object pools in `Pools`; the port's blocks are shared
//! handles, so only the block pool remains. `TranslateProgram` also gained a guest-memory
//! reader, which the port's translator takes instead of reaching for a global.

use crate::gpu::common::object_pool::ObjectPool;
use crate::gpu::shader_recompiler::frontend::control_flow_graph::CFG;
use crate::gpu::shader_recompiler::frontend::decode::{GcnCodeSlice, GcnDecodeContext};
use crate::gpu::shader_recompiler::frontend::structured_control_flow::build_asl;
use crate::gpu::shader_recompiler::frontend::translate::translator::Translator;
use crate::gpu::shader_recompiler::info::Info;
use crate::gpu::shader_recompiler::ir::abstract_syntax_list::{
    AbstractSyntaxList, NodeData, NodeType,
};
use crate::gpu::shader_recompiler::ir::basic_block::{add_branch, Block, BlockList, BlockRef};
use crate::gpu::shader_recompiler::ir::passes::ir_passes;
use crate::gpu::shader_recompiler::ir::post_order::post_order;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::profile::Profile;
use crate::gpu::shader_recompiler::runtime_info::{RuntimeInfo, SwStage};
use std::cell::RefCell;
use std::rc::Rc;

/// `Shader::Pools`. The C++'s instruction pool has no counterpart: the port allocates
/// instructions directly behind their shared handles.
pub struct Pools {
    pub block_pool: ObjectPool<BlockRef>,
}

impl Default for Pools {
    fn default() -> Self {
        Self::new()
    }
}

impl Pools {
    /// The C++'s `InstPoolSize` and `BlockPoolSize`.
    pub const INST_POOL_SIZE: u32 = 8192;
    pub const BLOCK_POOL_SIZE: u32 = 32;

    pub fn new() -> Self {
        // The instruction pool's size has no effect; the block pool's chunk size is kept.
        Pools {
            block_pool: ObjectPool::new(Self::BLOCK_POOL_SIZE as usize),
        }
    }

    /// `Pools::ReleaseContents`.
    pub fn release_contents(&mut self) {
        self.block_pool.release_contents();
    }
}

/// `Shader::GenerateBlocks`: the block nodes of an abstract syntax list, in order.
pub fn generate_blocks(syntax_list: &AbstractSyntaxList) -> BlockList {
    let mut blocks = BlockList::with_capacity(
        syntax_list
            .iter()
            .filter(|node| node.ty == NodeType::Block)
            .count(),
    );
    for node in syntax_list {
        if node.ty != NodeType::Block {
            continue;
        }
        if let NodeData::Block(block) = &node.data {
            blocks.push(block.clone());
        }
    }
    blocks
}

/// `Shader::EmitControlFlowGraph`.
fn emit_control_flow_graph(
    program: &mut Program,
    pools: &mut Pools,
    cfg: &CFG,
    runtime_info: &RuntimeInfo,
    profile: &Profile,
    read: &dyn Fn(u64) -> u32,
) {
    let mut translator = Translator::new(program.info.clone(), runtime_info, profile, read);
    for block in cfg.blocks() {
        let start = block.borrow().begin_index as usize;
        let size = (block.borrow().end_index - block.borrow().begin_index + 1) as usize;
        let ir_block = pools.block_pool.create(Block::new_ref()).clone();
        let begin = block.borrow().begin;
        let cond = block.borrow().cond;
        block.borrow_mut().ir_block = Some(ir_block.clone());
        translator.translate(
            &ir_block,
            begin,
            cond,
            &program.ins_list[start..start + size],
        );
        program.blocks.push(ir_block);
    }
    translator.emit_prologue(&program.blocks[0]);
    assert!(
        !program.info.borrow().translation_failed,
        "Shader translation has failed"
    );
    for block in cfg.blocks() {
        let ir_block = block
            .borrow()
            .ir_block
            .clone()
            .expect("the block was emitted");
        if let Some(true_block) = &block.borrow().branch_true {
            let true_block = true_block
                .borrow()
                .ir_block
                .clone()
                .expect("the true block was emitted");
            add_branch(&ir_block, &true_block);
        }
        if let Some(false_block) = &block.borrow().branch_false {
            let false_block = false_block
                .borrow()
                .ir_block
                .clone()
                .expect("the false block was emitted");
            add_branch(&ir_block, &false_block);
        }
    }
    program.post_order_blocks = post_order(&program.blocks[0]);
}

/// `Shader::TranslateProgram`.
pub fn translate_program(
    code: &[u32],
    pools: &mut Pools,
    info: Rc<RefCell<Info>>,
    runtime_info: &mut RuntimeInfo,
    profile: &Profile,
    read: &dyn Fn(u64) -> u32,
) -> Program {
    // The C++ warns when the first instruction is not `s_mov_b32 vcc_hi, #imm`; the port has no
    // logging module, so the check only has to stay harmless.
    const TOKEN_MOV_VCCHI: u32 = 0xBEEB03FF;
    assert!(!code.is_empty(), "a shader has at least one instruction");
    let _first_is_expected = code[0] == TOKEN_MOV_VCCHI;

    let mut slice = GcnCodeSlice::new(code);
    let mut decoder = GcnDecodeContext::new();

    let mut program = Program::new(info);
    program.ins_list.reserve(code.len());
    while !slice.at_end() {
        program
            .ins_list
            .push(decoder.decode_instruction(&mut slice));
    }

    pools.release_contents();

    let gcn_block_pool = ObjectPool::new(64);
    let cfg = CFG::new(gcn_block_pool, program.ins_list.clone());
    emit_control_flow_graph(&mut program, pools, &cfg, runtime_info, profile, read);

    ir_passes::inject_clip_distance_attributes(&program, runtime_info);

    if !profile.support_float64 {
        ir_passes::lower_fp64_to_fp32(&program);
    }
    ir_passes::ssa_rewrite_pass(&program);
    ir_passes::constant_propagation_pass(&program.post_order_blocks);
    if program.info.borrow().persistent.sw_stage == SwStage::TessellationControl {
        ir_passes::tessellation_preprocess(&program, runtime_info, read);
        ir_passes::hull_shader_transform(&program, runtime_info);
    } else if program.info.borrow().persistent.sw_stage == SwStage::TessellationEval {
        ir_passes::tessellation_preprocess(&program, runtime_info, read);
        ir_passes::domain_shader_transform(&program, runtime_info);
    }
    ir_passes::ring_access_elimination(&program, runtime_info);
    ir_passes::readlane_elimination_pass(&program);
    dump(&program, "pre-res-discover.");
    let resources = ir_passes::resource_discover_pass(&program, profile);
    ir_passes::flatten_extended_userdata_pass(&mut program);
    dump(&program, "pre-res-patch.");
    {
        let mut info = program.info.borrow_mut();
        ir_passes::resource_patching_pass(&mut info, &resources, profile);
    }
    ir_passes::lower_buffer_format_to_raw(&program);
    ir_passes::shared_memory_simplify_pass(&program, profile);
    ir_passes::shared_memory_to_storage_pass(&program, runtime_info, profile);
    ir_passes::lower_user_clip_planes(&program, runtime_info);
    ir_passes::phi_simplification_pass(&program);
    ir_passes::inverse_ballot_elimination_pass(&program);
    ir_passes::lower_hardware_intrinsics(&program);
    dump(&program, "pre-lower-phi.");

    // Prepare for structurization by clearing flow graph and lowering phis.
    for ir_block in &program.blocks {
        let mut block = ir_block.borrow_mut();
        block.imm_predecessors.clear();
        block.imm_successors.clear();
        block.ssa_state.reset();
    }
    ir_passes::lower_phis_to_regs_pass(&mut program);

    {
        let mut info = program.info.borrow_mut();
        program.syntax_list = build_asl(&cfg, &mut info);
    }
    program.blocks = generate_blocks(&program.syntax_list);
    program.post_order_blocks = match program.syntax_list.first() {
        Some(node) => match &node.data {
            NodeData::Block(block) => post_order(block),
            _ => BlockList::new(),
        },
        None => BlockList::new(),
    };

    ir_passes::ssa_repair_pass(&mut program);
    ir_passes::ssa_rewrite_pass(&program);
    ir_passes::shared_memory_barrier_pass(&program, runtime_info, profile);
    ir_passes::dead_code_elimination_pass(&program.post_order_blocks);
    ir_passes::lower_wave64_ballot_pass(&program, runtime_info, profile);
    ir_passes::constant_propagation_pass(&program.post_order_blocks);
    ir_passes::dead_code_elimination_pass(&program.post_order_blocks);
    if program.info.borrow().persistent.sw_stage == SwStage::Fragment
        && profile.needs_integer_interpolation_fix
    {
        ir_passes::interpolated_integer_pass(&program);
    }
    ir_passes::collect_shader_info_pass(&program, profile);
    dump(&program, "");

    program
}

/// `IR::DumpProgram(program, info, type)`: the C++ gates the write on the emulator settings; the
/// port's caller decides, so this helper writes unconditionally when called.
fn dump(program: &Program, type_: &str) {
    let info = program.info.borrow();
    let _ = program.dump_program(&info, type_);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::ir::abstract_syntax_list::AbstractSyntaxNode;

    #[test]
    fn only_block_nodes_become_blocks() {
        let first = Block::new_ref();
        let second = Block::new_ref();
        let syntax_list = vec![
            AbstractSyntaxNode::block(first.clone()),
            AbstractSyntaxNode::new(NodeType::If, NodeData::Block(second.clone())),
            AbstractSyntaxNode::block(second.clone()),
        ];
        let blocks = generate_blocks(&syntax_list);
        assert_eq!(blocks.len(), 2);
        assert!(Rc::ptr_eq(&blocks[0], &first));
        assert!(Rc::ptr_eq(&blocks[1], &second));
    }

    #[test]
    fn the_pools_release_their_blocks() {
        let mut pools = Pools::new();
        let _ = pools.block_pool.create(Block::new_ref());
        assert_eq!(pools.block_pool.created(), 1);
        pools.release_contents();
        assert_eq!(pools.block_pool.created(), 0);
    }
}
