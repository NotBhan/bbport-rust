//! User clip plane lowering, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/lower_user_clip_planes.cpp`.
//!
//! The fixed-function planes in `PA_CL_UCP_*` become clip distances computed from the final
//! exported position. Their equations arrive through a per-draw buffer, which the pass registers
//! even when no plane is enabled. A shader that exports its own clip distances keeps them.

use crate::gpu::shader_recompiler::ir::attribute::Attribute;
use crate::gpu::shader_recompiler::ir::basic_block::BlockRef;
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::r#type::Type;
use crate::gpu::shader_recompiler::ir::reg::BufferInstInfo;
use crate::gpu::shader_recompiler::ir::value::{Value, F32, F32F64};
use crate::gpu::shader_recompiler::resource::{BufferResource, BufferType};
use crate::gpu::shader_recompiler::runtime_info::{HwInfo, HwStage, Output, RuntimeInfo};

/// `NumUserClipPlanes`.
const NUM_USER_CLIP_PLANES: u32 = 6;

/// `LowerUserClipPlanes`.
pub fn lower_user_clip_planes(program: &Program, runtime_info: &RuntimeInfo) {
    let mut info = program.info.borrow_mut();
    if info.hw_stage() != HwStage::Vertex {
        return;
    }
    let HwInfo::Vs(vs) = &runtime_info.hw else {
        return;
    };

    let binding = info.persistent.buffers.len() as u32;
    info.persistent.buffers.push(BufferResource {
        used_types: Type::F32,
        buffer_type: BufferType::ClipPlanes,
        ..Default::default()
    });

    let enabled_mask = vs.user_clip_plane_mask;
    if enabled_mask == 0 {
        return;
    }
    for index in 0..vs.num_outputs as usize {
        for output in vs.outputs[index] {
            if (output as u32) >= Output::ClipDist0 as u32
                && (output as u32) <= Output::ClipDist7 as u32
            {
                return;
            }
        }
    }

    let mut position = [
        Value::empty(),
        Value::empty(),
        Value::empty(),
        Value::empty(),
    ];
    let mut target: Option<(BlockRef, usize)> = None;
    for block in &program.blocks {
        let instructions = block.borrow().instructions.clone();
        for (index, inst) in instructions.iter().enumerate() {
            if inst.borrow().get_opcode() != Opcode::SetAttribute {
                continue;
            }
            if inst.borrow().arg(0).attribute() != Attribute::Position0 {
                continue;
            }
            let comp = inst.borrow().arg(2).u32() as usize;
            position[comp] = inst.borrow().arg(1);
            target = Some((block.clone(), index + 1));
        }
    }
    if position.iter().any(Value::is_empty) {
        return;
    }
    let (target_block, target_index) = target.expect("a position export");

    let mut ir = IrEmitter::at(&target_block, target_index);
    let handle = ir.imm32(binding).into_value();
    for plane in 0..NUM_USER_CLIP_PLANES {
        if (enabled_mask >> plane) & 1 == 0 {
            continue;
        }
        let address = ir.imm32(plane * 4).into_value();
        let equation = ir.load_buffer_f32(4, &handle, &address, BufferInstInfo::default());
        let first_position = F32::new(position[0].clone());
        let first_coefficient = F32::new(ir.composite_extract(&equation, 0));
        let product = ir.fp_mul(
            &F32F64::new(first_position.into_value()),
            &F32F64::new(first_coefficient.into_value()),
        );
        let mut distance = F32::new(product.into_value());
        for comp in 1..4 {
            let position = F32::new(position[comp].clone());
            let coefficient = F32::new(ir.composite_extract(&equation, comp));
            let fma = ir.fp_fma(
                &F32F64::new(position.into_value()),
                &F32F64::new(coefficient.into_value()),
                &F32F64::new(distance.into_value()),
            );
            distance = F32::new(fma.into_value());
        }
        ir.set_attribute(Attribute::ClipDistance, &distance, plane);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block};
    use crate::gpu::shader_recompiler::ir::microinstruction::InstRef;
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage};
    use std::cell::RefCell;
    use std::rc::Rc;

    fn program(stage: HwStage, block: &BlockRef) -> Program {
        let info = Rc::new(RefCell::new(Info::new(
            stage,
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

    fn export_positions(block: &BlockRef) {
        for comp in 0..4 {
            append(
                block,
                Opcode::SetAttribute,
                &[
                    Value::from_attribute(Attribute::Position0),
                    Value::from_f32(1.0),
                    Value::from_u32(comp),
                ],
            );
        }
    }

    fn runtime_info(mask: u32) -> RuntimeInfo {
        let mut runtime_info = RuntimeInfo::new(HwStage::Vertex, SwStage::Vertex);
        if let HwInfo::Vs(vs) = &mut runtime_info.hw {
            vs.user_clip_plane_mask = mask;
        }
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
    fn an_enabled_plane_is_built_from_the_exported_position() {
        let block = Block::new_ref();
        export_positions(&block);
        append(&block, Opcode::Barrier, &[]);
        let program = program(HwStage::Vertex, &block);
        lower_user_clip_planes(&program, &runtime_info(1));

        let info = program.info.borrow();
        assert_eq!(info.persistent.buffers.len(), 1);
        assert_eq!(
            info.persistent.buffers[0].buffer_type,
            BufferType::ClipPlanes
        );
        assert_eq!(info.persistent.buffers[0].used_types, Type::F32);
        drop(info);

        let ops = opcodes(&block);
        assert_eq!(
            &ops[4..],
            &[
                Opcode::LoadBufferF32x4,
                Opcode::CompositeExtractF32x4,
                Opcode::FPMul32,
                Opcode::CompositeExtractF32x4,
                Opcode::FPFma32,
                Opcode::CompositeExtractF32x4,
                Opcode::FPFma32,
                Opcode::CompositeExtractF32x4,
                Opcode::FPFma32,
                Opcode::SetAttribute,
                Opcode::Barrier,
            ]
        );

        let instructions = block.borrow().instructions.clone();
        let load = &instructions[4];
        assert_eq!(load.borrow().arg(0).u32(), 0, "the buffer binding");
        assert_eq!(load.borrow().arg(1).u32(), 0, "plane 0 at dword 0");
        let set = &instructions[13];
        assert_eq!(set.borrow().arg(0).attribute(), Attribute::ClipDistance);
        assert_eq!(set.borrow().arg(2).u32(), 0);
    }

    #[test]
    fn each_enabled_plane_gets_its_own_equation_and_component() {
        let block = Block::new_ref();
        export_positions(&block);
        lower_user_clip_planes(&program(HwStage::Vertex, &block), &runtime_info(0b11));

        let instructions = block.borrow().instructions.clone();
        assert_eq!(instructions.len(), 4 + 10 * 2);
        assert_eq!(instructions[4].borrow().arg(1).u32(), 0);
        assert_eq!(instructions[14].borrow().arg(1).u32(), 4);
        assert_eq!(
            instructions[23].borrow().arg(0).attribute(),
            Attribute::ClipDistance
        );
        assert_eq!(instructions[23].borrow().arg(2).u32(), 1);
    }

    #[test]
    fn the_guards_register_the_buffer_without_lowering() {
        let block = Block::new_ref();
        export_positions(&block);
        let program_ref = program(HwStage::Vertex, &block);
        lower_user_clip_planes(&program_ref, &runtime_info(0));
        assert_eq!(program_ref.info.borrow().persistent.buffers.len(), 1);
        assert_eq!(opcodes(&block).len(), 4);

        let block = Block::new_ref();
        export_positions(&block);
        let program_ref = program(HwStage::Vertex, &block);
        let mut runtime = runtime_info(1);
        if let HwInfo::Vs(vs) = &mut runtime.hw {
            vs.num_outputs = 1;
            vs.outputs[0][0] = Output::ClipDist3;
        }
        lower_user_clip_planes(&program_ref, &runtime);
        assert_eq!(program_ref.info.borrow().persistent.buffers.len(), 1);
        assert_eq!(opcodes(&block).len(), 4);

        let block = Block::new_ref();
        export_positions(&block);
        let program_ref = program(HwStage::Fragment, &block);
        lower_user_clip_planes(&program_ref, &runtime_info(1));
        assert!(program_ref.info.borrow().persistent.buffers.is_empty());
        assert_eq!(opcodes(&block).len(), 4);
    }

    #[test]
    fn a_missing_position_component_leaves_the_buffer_alone() {
        let block = Block::new_ref();
        append(
            &block,
            Opcode::SetAttribute,
            &[
                Value::from_attribute(Attribute::Position0),
                Value::from_f32(1.0),
                Value::from_u32(0),
            ],
        );
        let program = program(HwStage::Vertex, &block);
        lower_user_clip_planes(&program, &runtime_info(1));
        assert_eq!(program.info.borrow().persistent.buffers.len(), 1);
        assert_eq!(opcodes(&block), vec![Opcode::SetAttribute]);
    }
}
