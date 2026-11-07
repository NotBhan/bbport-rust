//! The recompiled program, ported from `legacy/gpu/shadsps4/shader_recompiler/ir/program.h` and
//! `program.cpp`.
//!
//! The C++ holds `Info&`; the port shares the same information through an `Rc<RefCell<Info>>`,
//! because the passes write back into it. `DumpProgram` is split in two: `dump_program` writes
//! the files, and `dump_program_text` builds their contents, so the text can be checked without
//! touching the user directory. The C++ gates the dump on `EmulatorSettings.IsDumpShaders()`,
//! which has no port, so the caller decides.

use crate::gpu::common::path_util::{get_user_path, PathType};
use crate::gpu::shader_recompiler::frontend::instruction::GcnInst;
use crate::gpu::shader_recompiler::info::Info;
use crate::gpu::shader_recompiler::ir::abstract_syntax_list::{dump_asl_node, AbstractSyntaxList};
use crate::gpu::shader_recompiler::ir::basic_block::{dump_block_with, BlockList, BlockRef};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// `Shader::IR::Program`.
#[derive(Clone)]
pub struct Program {
    pub syntax_list: AbstractSyntaxList,
    pub blocks: BlockList,
    pub post_order_blocks: BlockList,
    pub ins_list: Vec<GcnInst>,
    pub next_reg_index: u32,
    pub info: Rc<RefCell<Info>>,
}

impl Program {
    /// `Program(Info&)`.
    pub fn new(info: Rc<RefCell<Info>>) -> Self {
        Program {
            syntax_list: Vec::new(),
            blocks: Vec::new(),
            post_order_blocks: Vec::new(),
            ins_list: Vec::new(),
            next_reg_index: 0,
            info,
        }
    }

    /// The dump's two files, as text: the block listing and the abstract syntax listing.
    pub fn dump_program_text(&self, info: &Info, type_: &str) -> (String, String) {
        let mut index = 0usize;
        let mut inst_to_index = HashMap::new();
        let mut block_to_index = HashMap::new();

        for block in &self.blocks {
            block_to_index.insert(Rc::as_ptr(block) as usize, index);
            index += 1;
        }

        let mut ir_text = String::new();
        for block in &self.blocks {
            ir_text.push_str(&dump_block_with(
                block,
                &block_to_index,
                &mut inst_to_index,
                &mut index,
            ));
            ir_text.push('\n');
        }

        let mut asl_text = String::new();
        for node in &self.syntax_list {
            asl_text.push_str(&dump_asl_node(node, &block_to_index, &inst_to_index));
            asl_text.push('\n');
        }

        let _ = (info, type_);
        (ir_text, asl_text)
    }

    /// `DumpProgram`: the two listings in the user's `shader/dumps` directory.
    pub fn dump_program(&self, info: &Info, type_: &str) -> std::io::Result<()> {
        let dump_dir = get_user_path(PathType::ShaderDir).join("dumps");
        std::fs::create_dir_all(&dump_dir)?;
        let (ir_text, asl_text) = self.dump_program_text(info, type_);

        let ir_filename = format!(
            "{}_{:#018x}.{}irprogram.txt",
            info.hw_stage(),
            info.persistent.pgm_hash,
            type_
        );
        std::fs::write(dump_dir.join(ir_filename), ir_text)?;

        let asl_filename = format!(
            "{}_{:#018x}.{}asl.txt",
            info.hw_stage(),
            info.persistent.pgm_hash,
            type_
        );
        std::fs::write(dump_dir.join(asl_filename), asl_text)?;
        Ok(())
    }

    /// The block a handle names, for the callers that look one up by pointer.
    pub fn block_index(&self, block: &BlockRef) -> Option<usize> {
        self.blocks
            .iter()
            .position(|candidate| Rc::ptr_eq(candidate, block))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block};
    use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage};

    #[test]
    fn a_fresh_program_holds_the_info_it_was_given() {
        let info = Rc::new(RefCell::new(Info::new(
            HwStage::Vertex,
            SwStage::Vertex,
            crate::gpu::shader_recompiler::params::ShaderParams::new([0; 16], &[], 7),
        )));
        let program = Program::new(info.clone());
        assert!(program.blocks.is_empty());
        assert!(program.post_order_blocks.is_empty());
        assert!(program.syntax_list.is_empty());
        assert!(program.ins_list.is_empty());
        assert_eq!(program.next_reg_index, 0);
        assert!(Rc::ptr_eq(&program.info, &info));
        // A write through the program reaches the caller's information.
        program.info.borrow_mut().has_discard = true;
        assert!(info.borrow().has_discard);
    }

    #[test]
    fn the_dump_numbers_the_blocks_and_instructions_it_writes() {
        let info = Rc::new(RefCell::new(Info::new(
            HwStage::Vertex,
            SwStage::Vertex,
            crate::gpu::shader_recompiler::params::ShaderParams::new([0; 16], &[], 0xFEED),
        )));
        let mut program = Program::new(info.clone());
        let block = Block::new_ref();
        append_new_inst(&block, Opcode::Prologue, &[]);
        append_new_inst(&block, Opcode::Epilogue, &[]);
        program.blocks.push(block.clone());
        let second = Block::new_ref();
        append_new_inst(&second, Opcode::Barrier, &[]);
        program.blocks.push(second.clone());
        program.syntax_list.push(
            crate::gpu::shader_recompiler::ir::abstract_syntax_list::AbstractSyntaxNode::block(
                block.clone(),
            ),
        );

        let (ir, asl) = program.dump_program_text(&info.borrow(), "");
        assert!(ir.starts_with("Block $0 predecessors"), "{ir}");
        assert!(
            ir.contains("Block $1 predecessors"),
            "the second block is numbered: {ir}"
        );
        assert!(ir.contains("Prologue"), "{ir}");
        assert!(ir.contains("Barrier"), "{ir}");
        assert!(asl.starts_with("Block: $0"), "{asl}");
        assert!(asl.ends_with('\n'), "{asl}");
        assert_eq!(program.block_index(&block), Some(0));
        assert_eq!(program.block_index(&second), Some(1));
        let other = Block::new_ref();
        assert_eq!(program.block_index(&other), None);
    }
}
