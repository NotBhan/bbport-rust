//! Clip distance attribute injection, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/inject_clip_distance_attributes.cpp`.
//!
//! For a fragment shader that emulates clip distances, the pass reads four components of the
//! next free parameter attribute after the prologue and discards when any is negative. The
//! attribute is reserved by bumping the fragment runtime info's input count.

use crate::gpu::shader_recompiler::ir::attribute::{attr_add, Attribute};
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::value::F32F64;
use crate::gpu::shader_recompiler::runtime_info::{
    HwFragmentRuntimeInfo, HwInfo, RuntimeInfo, SwStage, MAX_EMULATED_CLIP_DISTANCES,
};

fn fragment_info(runtime_info: &mut RuntimeInfo) -> Option<&mut HwFragmentRuntimeInfo> {
    match &mut runtime_info.hw {
        HwInfo::Fs(fs) => Some(fs),
        _ => None,
    }
}

/// `InjectClipDistanceAttributes`.
pub fn inject_clip_distance_attributes(program: &Program, runtime_info: &mut RuntimeInfo) {
    let Some(fs) = fragment_info(runtime_info) else {
        return;
    };
    if !fs.clip_distance_emulation || program.info.borrow().sw_stage() != SwStage::Fragment {
        return;
    }

    let first_block = program
        .blocks
        .first()
        .expect("the program has a first block")
        .clone();
    let instructions = first_block.borrow().instructions.clone();
    let prologue = instructions
        .iter()
        .position(|inst| inst.borrow().get_opcode() == Opcode::Prologue)
        .expect("the first block has a prologue");
    assert!(
        prologue + 1 < instructions.len(),
        "the prologue has a following instruction"
    );

    let mut ir = IrEmitter::at(&first_block, prologue + 2);

    // Not more than four distances are assumed, to save one attribute export slot.
    let attrib = attr_add(Attribute::Param0, fs.num_inputs as i32)
        .expect("Overflow or underflow on register arithmetic");
    for comp in 0..MAX_EMULATED_CLIP_DISTANCES {
        let attr_read = ir.get_attribute(attrib, comp, 0);
        let zero = ir.imm32_float(0.0);
        let cond = ir.fp_less_than(
            &F32F64::new(attr_read.into_value()),
            &F32F64::new(zero.into_value()),
            true,
        );
        ir.discard_cond(&cond);
    }
    fs.num_inputs += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block, BlockRef};
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::HwStage;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn make_program(sw_stage: SwStage, instructions: &[Opcode]) -> (Program, BlockRef) {
        let info = Rc::new(RefCell::new(Info::new(
            HwStage::Fragment,
            sw_stage,
            ShaderParams::new([0; 16], &[], 0),
        )));
        let mut program = Program::new(info);
        let block = Block::new_ref();
        for op in instructions {
            append_new_inst(&block, *op, &[]);
        }
        program.blocks = vec![block.clone()];
        (program, block)
    }

    fn runtime_info(clip_distance_emulation: bool, num_inputs: u32) -> RuntimeInfo {
        let mut runtime_info = RuntimeInfo::new(HwStage::Fragment, SwStage::Fragment);
        let fs = fragment_info(&mut runtime_info).expect("the fragment info");
        fs.clip_distance_emulation = clip_distance_emulation;
        fs.num_inputs = num_inputs;
        runtime_info
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
    fn the_four_components_are_read_and_discarded_after_the_prologue() {
        let (program, block) = make_program(
            SwStage::Fragment,
            &[Opcode::Prologue, Opcode::GetScc, Opcode::Barrier],
        );
        let mut runtime_info = runtime_info(true, 2);
        inject_clip_distance_attributes(&program, &mut runtime_info);

        let ops = opcodes(&block);
        assert_eq!(
            &ops[..6],
            &[
                Opcode::Prologue,
                Opcode::GetScc,
                Opcode::GetAttribute,
                Opcode::FPOrdLessThan32,
                Opcode::DiscardCond,
                Opcode::GetAttribute,
            ]
        );
        assert_eq!(ops.len(), 3 + 3 * MAX_EMULATED_CLIP_DISTANCES as usize);
        assert_eq!(*ops.last().expect("the barrier"), Opcode::Barrier);

        let first_read = block.borrow().instructions[2].clone();
        assert_eq!(
            first_read.borrow().arg(0).attribute(),
            Attribute::Param2,
            "the input count picks the reserved attribute"
        );
        assert_eq!(first_read.borrow().arg(1).u32(), 0);
        assert_eq!(first_read.borrow().arg(2).u32(), 0);

        let second_read = block.borrow().instructions[5].clone();
        assert_eq!(second_read.borrow().arg(1).u32(), 1);

        assert_eq!(
            fragment_info(&mut runtime_info)
                .expect("the fragment info")
                .num_inputs,
            3
        );
    }

    #[test]
    fn the_guards_leave_the_program_alone() {
        let (program, block) = make_program(
            SwStage::Fragment,
            &[Opcode::Prologue, Opcode::GetScc, Opcode::Barrier],
        );
        let mut info = runtime_info(false, 2);
        inject_clip_distance_attributes(&program, &mut info);
        assert_eq!(
            opcodes(&block),
            vec![Opcode::Prologue, Opcode::GetScc, Opcode::Barrier]
        );
        assert_eq!(fragment_info(&mut info).unwrap().num_inputs, 2);

        let (program, block) = make_program(
            SwStage::Vertex,
            &[Opcode::Prologue, Opcode::GetScc, Opcode::Barrier],
        );
        let mut info = runtime_info(true, 2);
        inject_clip_distance_attributes(&program, &mut info);
        assert_eq!(
            opcodes(&block),
            vec![Opcode::Prologue, Opcode::GetScc, Opcode::Barrier]
        );
    }

    #[test]
    fn a_missing_prologue_is_refused() {
        let (program, _block) = make_program(
            SwStage::Fragment,
            &[Opcode::GetScc, Opcode::Barrier, Opcode::Epilogue],
        );
        let mut info = runtime_info(true, 0);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            inject_clip_distance_attributes(&program, &mut info);
        }));
        assert!(result.is_err());
    }
}
