//! Shader info collection, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/shader_info_collection_pass.cpp`.
//!
//! Walks the post-order blocks and records which attributes the shader loads and stores, which
//! user data registers it reads, which patches and shared-memory widths it uses, and the
//! feature flags the backend and the resource binder need. The DMA gate reads the ported
//! emulator setting, whose default matches the C++.

use crate::gpu::shader_recompiler::info::{Info, ReadConstType};
use crate::gpu::shader_recompiler::ir::microinstruction::InstRef;
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::patch::Patch;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::r#type::Type;
use crate::gpu::shader_recompiler::profile::Profile;
use crate::gpu::shader_recompiler::resource::{BufferResource, BufferType};
use crate::gpu::shim::core::emulator_settings::emulator_settings;

fn add_shared_type(info: &mut Info, ty: Type) {
    info.shared_types = Type::from_bits(info.shared_types.bits() | ty.bits());
}

/// `Visit`.
fn visit(info: &mut Info, inst: &InstRef) {
    let opcode = inst.borrow().get_opcode();
    match opcode {
        Opcode::GetAttribute | Opcode::GetAttributeU1 | Opcode::GetAttributeU32 => {
            let attribute = inst.borrow().arg(0).attribute();
            let comp = inst.borrow().arg(1).u32();
            info.loads.set(attribute, comp);
        }
        Opcode::SetAttribute => {
            let attribute = inst.borrow().arg(0).attribute();
            let comp = inst.borrow().arg(2).u32();
            info.stores.set(attribute, comp);
        }
        Opcode::GetUserData => {
            info.persistent
                .ud_mask
                .set(inst.borrow().arg(0).scalar_reg());
        }
        Opcode::SetPatch => {
            let patch = inst.borrow().arg(0).patch();
            if patch <= Patch::TESSELLATION_LOD_BOTTOM {
                info.stores_tess_level_outer = true;
            } else if patch <= Patch::TESSELLATION_LOD_INTERIOR_V {
                info.stores_tess_level_inner = true;
            } else {
                info.uses_patches |= 1 << patch.generic_index();
            }
        }
        Opcode::GetPatch => {
            let patch = inst.borrow().arg(0).patch();
            info.uses_patches |= 1 << patch.generic_index();
        }
        Opcode::LoadSharedU16 | Opcode::WriteSharedU16 => {
            add_shared_type(info, Type::U16);
        }
        Opcode::LoadSharedU32
        | Opcode::WriteSharedU32
        | Opcode::SharedAtomicIAdd32
        | Opcode::SharedAtomicISub32
        | Opcode::SharedAtomicSMin32
        | Opcode::SharedAtomicUMin32
        | Opcode::SharedAtomicSMax32
        | Opcode::SharedAtomicUMax32
        | Opcode::SharedAtomicInc32
        | Opcode::SharedAtomicDec32
        | Opcode::SharedAtomicAnd32
        | Opcode::SharedAtomicOr32
        | Opcode::SharedAtomicXor32
        | Opcode::SharedAtomicCmpSwap32 => {
            add_shared_type(info, Type::U32);
        }
        Opcode::SharedAtomicIAdd64
        | Opcode::SharedAtomicISub64
        | Opcode::SharedAtomicSMin64
        | Opcode::SharedAtomicUMin64
        | Opcode::SharedAtomicSMax64
        | Opcode::SharedAtomicUMax64
        | Opcode::SharedAtomicInc64
        | Opcode::SharedAtomicDec64
        | Opcode::SharedAtomicAnd64
        | Opcode::SharedAtomicOr64
        | Opcode::SharedAtomicXor64
        | Opcode::SharedAtomicCmpSwap64 => {
            info.uses_shared_int64_atomics = true;
            add_shared_type(info, Type::U64);
        }
        Opcode::LoadSharedU64 | Opcode::WriteSharedU64 => {
            add_shared_type(info, Type::U64);
        }
        Opcode::ConvertF16F32
        | Opcode::ConvertF32F16
        | Opcode::BitCastU16F16
        | Opcode::BitCastF16U16 => {
            info.uses_fp16 = true;
        }
        Opcode::PackDouble2x32 | Opcode::UnpackDouble2x32 => {
            info.uses_fp64 = true;
        }
        Opcode::ImageWrite => {
            info.has_storage_images = true;
        }
        Opcode::QuadBroadcast => {
            info.uses_group_quad = true;
        }
        Opcode::Shuffle | Opcode::ShuffleXor => {
            info.uses_group_shuffle = true;
        }
        Opcode::ReadLane | Opcode::ReadFirstLane | Opcode::WriteLane => {
            info.uses_group_ballot = true;
        }
        Opcode::Discard | Opcode::DiscardCond => {
            info.has_discard = true;
        }
        Opcode::BitwiseXor32 => {
            info.persistent.has_bitwise_xor = true;
        }
        Opcode::ImageGather | Opcode::ImageGatherDref => {
            info.has_image_gather = true;
        }
        Opcode::ImageQueryDimensions | Opcode::ImageQueryLod => {
            info.has_image_query = true;
        }
        Opcode::ImageAtomicFMax32 | Opcode::ImageAtomicFMin32 => {
            info.uses_image_atomic_float_min_max = true;
        }
        Opcode::BufferAtomicFMax32 | Opcode::BufferAtomicFMin32 => {
            info.uses_buffer_atomic_float_min_max = true;
        }
        Opcode::BufferAtomicIAdd64
        | Opcode::BufferAtomicSMax64
        | Opcode::BufferAtomicSMin64
        | Opcode::BufferAtomicUMax64
        | Opcode::BufferAtomicUMin64 => {
            info.uses_buffer_int64_atomics = true;
        }
        Opcode::DataAppend
        | Opcode::DataConsume
        | Opcode::Ballot
        | Opcode::InverseBallot
        | Opcode::BallotFindLsb => {
            info.uses_group_ballot = true;
            info.uses_lane_id = true;
        }
        Opcode::LaneId => {
            info.uses_lane_id = true;
        }
        Opcode::Memtime => {
            info.uses_shader_clock = true;
        }
        Opcode::ReadConst => {
            if !info.has_readconst {
                info.persistent.buffers.push(BufferResource {
                    used_types: Type::U32,
                    buffer_type: BufferType::Flatbuf,
                    ..Default::default()
                });
                info.has_readconst = true;
            }
            if inst.borrow().flags() as u32 == 0 {
                info.readconst_types.add(ReadConstType::IMMEDIATE);
                info.readconst_types.add(ReadConstType::DYNAMIC);
                info.persistent.uses_dma = true;
            }
        }
        Opcode::PackUfloat10_11_11 => {
            info.uses_pack_10_11_11 = true;
        }
        Opcode::UnpackUfloat10_11_11 => {
            info.uses_unpack_10_11_11 = true;
        }
        _ => {}
    }
}

/// `CollectShaderInfoPass`.
pub fn collect_shader_info_pass(program: &Program, _profile: &Profile) {
    let info = program.info.clone();
    for block in &program.post_order_blocks {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            visit(&mut info.borrow_mut(), inst);
        }
    }

    let mut info = info.borrow_mut();
    if !emulator_settings().is_direct_memory_access_enabled() {
        info.persistent.uses_dma = false;
        info.readconst_types = ReadConstType::NONE;
    }

    if info.persistent.uses_dma {
        info.persistent.buffers.push(BufferResource {
            used_types: Type::U64,
            buffer_type: BufferType::BdaPagetable,
            is_written: true,
            ..Default::default()
        });
        info.persistent.buffers.push(BufferResource {
            used_types: Type::U32,
            buffer_type: BufferType::FaultBuffer,
            is_written: true,
            ..Default::default()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::ir::attribute::Attribute;
    use crate::gpu::shader_recompiler::ir::basic_block::BlockRef;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block};
    use crate::gpu::shader_recompiler::ir::reg::ScalarReg;
    use crate::gpu::shader_recompiler::ir::value::Value;
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage};
    use std::cell::RefCell;
    use std::rc::Rc;

    fn program(block: &BlockRef) -> Program {
        let info = Rc::new(RefCell::new(Info::new(
            HwStage::Vertex,
            SwStage::Vertex,
            ShaderParams::new([0; 16], &[], 0),
        )));
        let mut program = Program::new(info);
        program.post_order_blocks = vec![block.clone()];
        program
    }

    fn append(block: &BlockRef, op: Opcode, args: &[Value]) -> InstRef {
        let index = append_new_inst(block, op, args);
        block.borrow().instructions[index].clone()
    }

    #[test]
    fn attribute_and_user_data_accesses_are_recorded() {
        let block = Block::new_ref();
        append(
            &block,
            Opcode::GetAttribute,
            &[
                Value::from_attribute(Attribute::Param0),
                Value::from_u32(1),
                Value::from_u32(0),
            ],
        );
        append(
            &block,
            Opcode::SetAttribute,
            &[
                Value::from_attribute(Attribute::Param1),
                Value::from_f32(1.0),
                Value::from_u32(2),
            ],
        );
        append(
            &block,
            Opcode::GetUserData,
            &[Value::from_scalar_reg(ScalarReg::S4)],
        );

        let program = program(&block);
        collect_shader_info_pass(&program, &Profile::default());

        let info = program.info.borrow();
        assert!(info.loads.get(Attribute::Param0, 1));
        assert!(info.stores.get(Attribute::Param1, 2));
        assert!(info.persistent.ud_mask.mask & (1 << 4) != 0);
    }

    #[test]
    fn patches_and_shared_widths_are_recorded() {
        let block = Block::new_ref();
        append(
            &block,
            Opcode::SetPatch,
            &[
                Value::from_patch(Patch::TESSELLATION_LOD_LEFT),
                Value::from_f32(1.0),
            ],
        );
        append(
            &block,
            Opcode::SetPatch,
            &[
                Value::from_patch(Patch::TESSELLATION_LOD_INTERIOR_U),
                Value::from_f32(1.0),
            ],
        );
        append(
            &block,
            Opcode::GetPatch,
            &[Value::from_patch(Patch::COMPONENT0)],
        );
        append(&block, Opcode::LoadSharedU16, &[Value::from_u32(0)]);
        append(
            &block,
            Opcode::SharedAtomicIAdd64,
            &[Value::from_u32(0), Value::from_u64(1)],
        );

        let program = program(&block);
        collect_shader_info_pass(&program, &Profile::default());

        let info = program.info.borrow();
        assert!(info.stores_tess_level_outer);
        assert!(info.stores_tess_level_inner);
        assert_eq!(info.uses_patches, 1);
        assert!(info.shared_types.contains(Type::U16));
        assert!(info.shared_types.contains(Type::U64));
        assert!(info.uses_shared_int64_atomics);
    }

    #[test]
    fn the_first_read_const_adds_a_flatbuf_binding() {
        let block = Block::new_ref();
        append(
            &block,
            Opcode::ReadConst,
            &[Value::from_u32(0), Value::from_u32(0)],
        );

        let program = program(&block);
        collect_shader_info_pass(&program, &Profile::default());

        let info = program.info.borrow();
        assert!(info.has_readconst);
        assert_eq!(info.persistent.buffers.len(), 1);
        assert_eq!(info.persistent.buffers[0].buffer_type, BufferType::Flatbuf);
    }
}
