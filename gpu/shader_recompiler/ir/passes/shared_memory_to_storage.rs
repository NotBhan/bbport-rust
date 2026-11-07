//! Shared-memory-to-storage lowering, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/shared_memory_to_storage_pass.cpp`.
//!
//! When the requested shared memory does not fit the host, or the host cannot give the workgroup
//! an explicit layout with mixed access widths, every shared access becomes an access to a
//! storage buffer. The workgroup index selects the workgroup's slice of that buffer.

use crate::gpu::shader_recompiler::ir::attribute::Attribute;
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::microinstruction::{invalidate, replace_uses_with, InstRef};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::r#type::Type;
use crate::gpu::shader_recompiler::ir::reg::BufferInstInfo;
use crate::gpu::shader_recompiler::ir::value::{U16, U32, U32U64, U64};
use crate::gpu::shader_recompiler::profile::Profile;
use crate::gpu::shader_recompiler::resource::{BufferResource, BufferType};
use crate::gpu::shader_recompiler::runtime_info::{HwInfo, HwStage, RuntimeInfo};
use std::rc::Rc;

fn wide(value: &U32) -> U32U64 {
    U32U64::new(value.value().clone())
}

/// `IsSharedAccess`.
fn is_shared_access(inst: &InstRef) -> bool {
    let opcode = inst.borrow().get_opcode();
    matches!(
        opcode,
        Opcode::LoadSharedU16
            | Opcode::LoadSharedU32
            | Opcode::LoadSharedU64
            | Opcode::WriteSharedU16
            | Opcode::WriteSharedU32
            | Opcode::WriteSharedU64
    ) || (opcode >= Opcode::SharedAtomicIAdd32 && opcode <= Opcode::SharedAtomicCmpSwap64)
}

/// `CalculateSharedMemoryTypes`.
fn calculate_shared_memory_types(program: &Program) -> Type {
    let mut used_types = Type::Void;
    for block in &program.blocks {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            if !is_shared_access(inst) {
                continue;
            }
            let used = match inst.borrow().get_opcode() {
                Opcode::LoadSharedU16 | Opcode::WriteSharedU16 => Type::U16,
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
                | Opcode::SharedAtomicCmpSwap32 => Type::U32,
                Opcode::LoadSharedU64
                | Opcode::WriteSharedU64
                | Opcode::SharedAtomicIAdd64
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
                | Opcode::SharedAtomicCmpSwap64 => Type::U64,
                _ => continue,
            };
            used_types = Type::from_bits(used_types.bits() | used.bits());
        }
    }
    used_types
}

/// `SharedMemoryToStoragePass`.
pub fn shared_memory_to_storage_pass(
    program: &Program,
    runtime_info: &RuntimeInfo,
    profile: &Profile,
) {
    if program.info.borrow().persistent.hw_stage != HwStage::Compute {
        return;
    }
    let HwInfo::Cs(compute) = &runtime_info.hw else {
        return;
    };

    let shared_memory_size = compute.shared_memory_size;
    let used_types = calculate_shared_memory_types(program);
    if used_types.is_empty()
        || (shared_memory_size <= profile.max_shared_memory_size
            && (profile.supports_workgroup_explicit_memory_layout
                || used_types.bits().count_ones() == 1))
    {
        return;
    }

    let binding = program.info.borrow().persistent.buffers.len() as u32;
    program
        .info
        .borrow_mut()
        .persistent
        .buffers
        .push(BufferResource {
            used_types,
            buffer_type: BufferType::SharedMemory,
            is_written: true,
            ..Default::default()
        });

    for block in &program.blocks {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            if !is_shared_access(inst) {
                continue;
            }
            let index = block
                .borrow()
                .instructions
                .iter()
                .position(|candidate| Rc::ptr_eq(candidate, inst))
                .expect("the instruction is in its block");
            let mut ir = IrEmitter::at(block, index);
            let info = BufferInstInfo::from_raw(0);
            let handle = ir.imm32(binding).into_value();
            let workgroup = ir.get_attribute_u32(Attribute::WorkgroupIndex, 0);
            let size = ir.imm32(shared_memory_size);
            let slice_offset = U32::new(ir.i_mul(&wide(&workgroup), &wide(&size)).into_value());
            let address = U32::new(
                ir.i_add(&wide(&U32::new(inst.borrow().arg(0))), &wide(&slice_offset))
                    .into_value(),
            );
            let raw_address = address.value().clone();

            let opcode = inst.borrow().get_opcode();
            match opcode {
                Opcode::SharedAtomicIAdd32 | Opcode::SharedAtomicIAdd64 => {
                    let value = inst.borrow().arg(1);
                    let result = ir.buffer_atomic_i_add(&handle, &raw_address, &value, info);
                    replace_uses_with(inst, result);
                }
                Opcode::SharedAtomicISub32 | Opcode::SharedAtomicISub64 => {
                    let value = inst.borrow().arg(1);
                    let result = ir.buffer_atomic_i_sub(&handle, &raw_address, &value, info);
                    replace_uses_with(inst, result);
                }
                Opcode::SharedAtomicSMin32
                | Opcode::SharedAtomicUMin32
                | Opcode::SharedAtomicSMin64
                | Opcode::SharedAtomicUMin64 => {
                    let is_signed = opcode == Opcode::SharedAtomicSMin32
                        || opcode == Opcode::SharedAtomicSMin64;
                    let value = inst.borrow().arg(1);
                    let result =
                        ir.buffer_atomic_i_min(&handle, &raw_address, &value, is_signed, info);
                    replace_uses_with(inst, result);
                }
                Opcode::SharedAtomicSMax32
                | Opcode::SharedAtomicUMax32
                | Opcode::SharedAtomicSMax64
                | Opcode::SharedAtomicUMax64 => {
                    let is_signed = opcode == Opcode::SharedAtomicSMax32
                        || opcode == Opcode::SharedAtomicSMax64;
                    let value = inst.borrow().arg(1);
                    let result =
                        ir.buffer_atomic_i_max(&handle, &raw_address, &value, is_signed, info);
                    replace_uses_with(inst, result);
                }
                Opcode::SharedAtomicInc32 => {
                    let result = ir.buffer_atomic_inc(&handle, &raw_address, info);
                    replace_uses_with(inst, result);
                }
                Opcode::SharedAtomicDec32 => {
                    let result = ir.buffer_atomic_dec(&handle, &raw_address, info);
                    replace_uses_with(inst, result);
                }
                Opcode::SharedAtomicAnd32 | Opcode::SharedAtomicAnd64 => {
                    let value = inst.borrow().arg(1);
                    let result = ir.buffer_atomic_and(&handle, &raw_address, &value, info);
                    replace_uses_with(inst, result);
                }
                Opcode::SharedAtomicOr32 | Opcode::SharedAtomicOr64 => {
                    let value = inst.borrow().arg(1);
                    let result = ir.buffer_atomic_or(&handle, &raw_address, &value, info);
                    replace_uses_with(inst, result);
                }
                Opcode::SharedAtomicXor32 | Opcode::SharedAtomicXor64 => {
                    let value = inst.borrow().arg(1);
                    let result = ir.buffer_atomic_xor(&handle, &raw_address, &value, info);
                    replace_uses_with(inst, result);
                }
                Opcode::SharedAtomicCmpSwap32 | Opcode::SharedAtomicCmpSwap64 => {
                    let value = inst.borrow().arg(1);
                    let compare = inst.borrow().arg(2);
                    let result =
                        ir.buffer_atomic_cmp_swap(&handle, &raw_address, &value, &compare, info);
                    replace_uses_with(inst, result);
                }
                Opcode::LoadSharedU16 => {
                    let result = ir.load_buffer_u16(&handle, &raw_address, info);
                    replace_uses_with(inst, result.into_value());
                }
                Opcode::LoadSharedU32 => {
                    let result = ir.load_buffer_u32(1, &handle, &raw_address, info);
                    replace_uses_with(inst, result);
                }
                Opcode::LoadSharedU64 => {
                    let result = ir.load_buffer_u64(&handle, &raw_address, info);
                    replace_uses_with(inst, result.into_value());
                }
                Opcode::WriteSharedU16 => {
                    let value = U16::new(inst.borrow().arg(1));
                    ir.store_buffer_u16(&handle, &raw_address, &value, info);
                    invalidate(inst);
                }
                Opcode::WriteSharedU32 => {
                    let value = inst.borrow().arg(1);
                    ir.store_buffer_u32(1, &handle, &raw_address, &value, info);
                    invalidate(inst);
                }
                Opcode::WriteSharedU64 => {
                    let value = U64::new(inst.borrow().arg(1));
                    ir.store_buffer_u64(&handle, &raw_address, &value, info);
                    invalidate(inst);
                }
                _ => panic!("Unreachable"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block, BlockRef};
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
        program
    }

    fn runtime(shared_memory_size: u32) -> RuntimeInfo {
        let mut runtime = RuntimeInfo::new(HwStage::Compute, SwStage::Compute);
        runtime.hw = HwInfo::Cs(HwComputeRuntimeInfo {
            shared_memory_size,
            workgroup_size: [64, 1, 1],
            tgid_enable: [false; 3],
        });
        runtime
    }

    fn append(block: &BlockRef, op: Opcode, args: &[Value]) -> InstRef {
        let index = append_new_inst(block, op, args);
        block.borrow().instructions[index].clone()
    }

    #[test]
    fn a_shared_load_becomes_a_storage_buffer_load() {
        let block = Block::new_ref();
        let offset = append(&block, Opcode::LaneId, &[]);
        let load = append(&block, Opcode::LoadSharedU32, &[Value::from_inst(offset)]);
        let sink = append(
            &block,
            Opcode::BitCastF32U32,
            &[Value::from_inst(load.clone())],
        );

        let program = program(&block);
        shared_memory_to_storage_pass(&program, &runtime(65536), &Profile::default());

        assert_eq!(load.borrow().get_opcode(), Opcode::Void);
        assert_eq!(
            sink.borrow().arg(0).inst().borrow().get_opcode(),
            Opcode::LoadBufferU32
        );
        let info = program.info.borrow();
        assert_eq!(info.persistent.buffers.len(), 1);
        assert_eq!(
            info.persistent.buffers[0].buffer_type,
            BufferType::SharedMemory
        );
        assert!(info.persistent.buffers[0].used_types.contains(Type::U32));
    }

    #[test]
    fn mixed_width_accesses_are_recorded() {
        let block = Block::new_ref();
        let offset = Value::from_u32(0);
        append(&block, Opcode::LoadSharedU16, std::slice::from_ref(&offset));
        let load64 = append(&block, Opcode::LoadSharedU64, &[offset]);
        let sink = append(&block, Opcode::ConvertF16F32, &[Value::from_inst(load64)]);

        let program = program(&block);
        shared_memory_to_storage_pass(&program, &runtime(65536), &Profile::default());

        let info = program.info.borrow();
        let used = info.persistent.buffers[0].used_types;
        assert!(used.contains(Type::U16));
        assert!(used.contains(Type::U64));
        let _ = sink;
    }

    #[test]
    fn a_host_that_fits_the_workgroup_keeps_shared_memory() {
        let block = Block::new_ref();
        let offset = Value::from_u32(0);
        let load = append(&block, Opcode::LoadSharedU32, &[offset]);
        let _ = load;

        let program = program(&block);
        let profile = Profile {
            max_shared_memory_size: 65536,
            supports_workgroup_explicit_memory_layout: true,
            ..Profile::default()
        };
        shared_memory_to_storage_pass(&program, &runtime(65536), &profile);

        assert!(program.info.borrow().persistent.buffers.is_empty());
        assert_eq!(
            block.borrow().instructions()[0].borrow().get_opcode(),
            Opcode::LoadSharedU32
        );
    }
}
