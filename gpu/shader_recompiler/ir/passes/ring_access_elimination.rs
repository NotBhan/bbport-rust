//! Ring-access elimination, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/ring_access_elimination.cpp`.
//!
//! Geometry shaders read and write their outputs through the ring buffer API. For a Local shader
//! the shared-memory writes become attribute stores, and for an Export/Geometry shader the
//! coherent buffer accesses become `SetAttribute`/`GetAttribute` (or a real position export).

use crate::gpu::shader_recompiler::frontend::copy_shader::{parse_copy_shader, CopyShaderData};
use crate::gpu::shader_recompiler::ir::attribute::{attr_add, is_position, Attribute};
use crate::gpu::shader_recompiler::ir::basic_block::BlockRef;
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::microinstruction::{
    clear_args, invalidate, set_arg, InstRef,
};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::operand_helper::{
    get_buffer_index_arg, get_buffer_soffset_arg, get_buffer_voffset_arg,
};
use crate::gpu::shader_recompiler::ir::position::export_position;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::value::{Value, U32, U64};
use crate::gpu::shader_recompiler::runtime_info::{HwInfo, HwStage, RuntimeInfo};
use std::rc::Rc;

fn position(block: &BlockRef, inst: &InstRef) -> Option<usize> {
    block
        .borrow()
        .instructions
        .iter()
        .position(|candidate| Rc::ptr_eq(candidate, inst))
}

/// `RingAccessElimination`.
pub fn ring_access_elimination(program: &Program, runtime_info: &RuntimeInfo) {
    let stage = program.info.borrow().persistent.hw_stage;
    match stage {
        HwStage::Local => {
            for block in &program.blocks {
                let instructions = block.borrow().instructions.clone();
                for inst in &instructions {
                    let opcode = inst.borrow().get_opcode();
                    if opcode != Opcode::WriteSharedU64 && opcode != Opcode::WriteSharedU32 {
                        continue;
                    }
                    let is_composite = opcode == Opcode::WriteSharedU64;
                    let num_components = if opcode == Opcode::WriteSharedU32 {
                        1
                    } else {
                        2
                    };
                    assert!(inst.borrow().arg(0).is_immediate());

                    let mut offset = inst.borrow().arg(0).u32();
                    let data = if is_composite {
                        let wide = U64::new(inst.borrow().arg(1));
                        let index = position(block, inst).expect("the instruction is in its block");
                        let mut ir = IrEmitter::at(block, index);
                        ir.unpack_uint_2x32(&wide)
                    } else {
                        inst.borrow().arg(1)
                    };
                    let index = position(block, inst).expect("the instruction is in its block");
                    let mut ir = IrEmitter::at(block, index);
                    for component in 0..num_components {
                        let attrib = attr_add(Attribute::Param0, (offset / 16) as i32)
                            .expect("the attribute is in range");
                        let comp = (offset / 4) % 4;
                        let value = if is_composite {
                            ir.composite_extract(&data, component as usize)
                        } else {
                            data.clone()
                        };
                        let value = U32::new(value);
                        let as_float = ir.bit_cast_f32_u32(&value);
                        ir.set_attribute(attrib, &as_float, comp);
                        offset += 4;
                    }
                    invalidate(inst);
                }
            }
        }
        HwStage::Export => {
            let HwInfo::Es(export_info) = &runtime_info.hw else {
                return;
            };
            for block in &program.blocks {
                let instructions = block.borrow().instructions.clone();
                for inst in &instructions {
                    if inst.borrow().get_opcode() != Opcode::StoreBufferU32 {
                        continue;
                    }
                    let flags = inst.borrow().buffer_info();
                    if flags.system_coherent() == 0 || flags.globally_coherent() == 0 {
                        continue;
                    }

                    let offset = flags.inst_offset() as u32;
                    assert!(offset < export_info.vertex_data_size * 4);
                    let index = position(block, inst).expect("the instruction is in its block");
                    let mut ir = IrEmitter::at(block, index);
                    let data = ir.bit_cast_f32_u32(&U32::new(inst.borrow().arg(2)));
                    let attrib = if offset < 16 {
                        Attribute::Position0
                    } else {
                        attr_add(Attribute::Param0, (offset / 16 - 1) as i32)
                            .expect("the attribute is in range")
                    };
                    let comp = (offset / 4) % 4;

                    inst.borrow_mut().replace_opcode(Opcode::SetAttribute);
                    clear_args(inst);
                    set_arg(inst, 0, Value::from_attribute(attrib));
                    set_arg(inst, 1, data.into_value());
                    set_arg(inst, 2, Value::from_u32(comp));
                }
            }
        }
        HwStage::Geometry => {
            let HwInfo::Gs(gs_info) = &runtime_info.hw else {
                return;
            };
            let gs_copy: CopyShaderData = parse_copy_shader(&gs_info.vs_copy);
            program.info.borrow_mut().gs_copy_data = gs_copy.clone();

            let mut dwords_per_vertex = gs_info.out_vertex_data_size;
            if gs_copy.num_comps != 0 && gs_copy.num_comps > dwords_per_vertex {
                dwords_per_vertex = gs_copy.num_comps;
            }

            for block in &program.blocks {
                let instructions = block.borrow().instructions.clone();
                for inst in &instructions {
                    let opcode = inst.borrow().get_opcode();
                    match opcode {
                        Opcode::LoadBufferU32 => {
                            let flags = inst.borrow().buffer_info();
                            if flags.system_coherent() == 0 || flags.globally_coherent() == 0 {
                                continue;
                            }

                            let address = if flags.index_enable() != 0 {
                                get_buffer_index_arg(inst)
                            } else {
                                get_buffer_voffset_arg(inst)
                            };
                            let vertex_id = address.value().u32() >> 2;
                            let soffset = get_buffer_soffset_arg(inst);
                            let bucket = soffset.value().u32() / 256;
                            let attrib = if bucket < 4 {
                                Attribute::Position0
                            } else {
                                attr_add(Attribute::Param0, (bucket / 4 - 1) as i32)
                                    .expect("the attribute is in range")
                            };
                            let comp = bucket % 4;

                            let index =
                                position(block, inst).expect("the instruction is in its block");
                            let mut ir = IrEmitter::at(block, index);
                            let attr_value = ir.get_attribute(attrib, comp, vertex_id);
                            inst.borrow_mut().replace_opcode(Opcode::BitCastU32F32);
                            clear_args(inst);
                            set_arg(inst, 0, attr_value.into_value());
                        }
                        Opcode::StoreBufferU32 => {
                            let flags = inst.borrow().buffer_info();
                            if flags.system_coherent() == 0 || flags.globally_coherent() == 0 {
                                continue;
                            }

                            let offset = flags.inst_offset() as u32;
                            let index =
                                position(block, inst).expect("the instruction is in its block");
                            let mut ir = IrEmitter::at(block, index);
                            let data = ir.bit_cast_f32_u32(&U32::new(inst.borrow().arg(2)));
                            let comp_ofs = gs_copy.output_vertices * 4;
                            let output_size = gs_info.output_vertices * dwords_per_vertex * 4;
                            let vc_read_ofs = (((offset / comp_ofs) * comp_ofs) % output_size) * 16;
                            let &(attr, comp) = gs_copy
                                .attr_map
                                .get(&vc_read_ofs)
                                .expect("the copy shader maps the offset");

                            invalidate(inst);
                            if is_position(attr) {
                                export_position(&mut ir, gs_info, false, attr, comp, &data);
                            } else {
                                ir.set_attribute(attr, &data, comp);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block};
    use crate::gpu::shader_recompiler::ir::reg::BufferInstInfo;
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{HwExportRuntimeInfo, SwStage};
    use std::cell::RefCell;

    struct Stages {
        program: Program,
        runtime: RuntimeInfo,
    }

    fn stages(stage: HwStage, block: &BlockRef) -> Stages {
        let info = Rc::new(RefCell::new(Info::new(
            stage,
            SwStage::Vertex,
            ShaderParams::new([0; 16], &[], 0),
        )));
        let mut program = Program::new(info);
        program.blocks = vec![block.clone()];
        let mut runtime = RuntimeInfo::new(stage, SwStage::Vertex);
        if stage == HwStage::Export {
            runtime.hw = HwInfo::Es(HwExportRuntimeInfo {
                vertex_data_size: 8,
            });
        }
        Stages { program, runtime }
    }

    fn append(block: &BlockRef, op: Opcode, args: &[Value], flags: u64) -> InstRef {
        let index = append_new_inst(block, op, args);
        let inst = block.borrow().instructions[index].clone();
        inst.borrow_mut().set_flags(flags);
        inst
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
    fn a_local_shared_write_becomes_an_attribute_store() {
        let block = Block::new_ref();
        let data = append(&block, Opcode::LaneId, &[], 0);
        let write = append(
            &block,
            Opcode::WriteSharedU32,
            &[Value::from_u32(32), Value::from_inst(data)],
            0,
        );

        let stages = stages(HwStage::Local, &block);
        ring_access_elimination(&stages.program, &stages.runtime);

        assert_eq!(write.borrow().get_opcode(), Opcode::Void);
        let ops = opcodes(&block);
        assert!(ops.contains(&Opcode::SetAttribute));
        let store = block
            .borrow()
            .instructions()
            .iter()
            .find(|inst| inst.borrow().get_opcode() == Opcode::SetAttribute)
            .expect("the attribute store")
            .clone();
        assert!(matches!(
            store.borrow().arg(0).attribute(),
            Attribute::Param2
        ));
        assert_eq!(store.borrow().arg(2).u32(), 0);
    }

    #[test]
    fn an_export_buffer_store_becomes_an_attribute_store() {
        let block = Block::new_ref();
        let handle = Value::from_u32(0);
        let address = Value::from_u32(0);
        let data = append(&block, Opcode::LaneId, &[], 0);
        let mut flags = BufferInstInfo::from_raw(0);
        flags.set_system_coherent(1);
        flags.set_globally_coherent(1);
        flags.set_inst_offset(20);
        let store = append(
            &block,
            Opcode::StoreBufferU32,
            &[handle, address, Value::from_inst(data)],
            flags.raw(),
        );

        let stages = stages(HwStage::Export, &block);
        ring_access_elimination(&stages.program, &stages.runtime);

        assert_eq!(store.borrow().get_opcode(), Opcode::SetAttribute);
        assert_eq!(store.borrow().arg(0).attribute(), Attribute::Param0);
        assert_eq!(store.borrow().arg(2).u32(), 1);
    }
}
