//! Shared memory simplification, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/shared_memory_simplify_pass.cpp`.
//!
//! When the device cannot lay out workgroup memory explicitly, 16- and 64-bit shared loads and
//! stores are lowered to 32-bit ones unless an atomic of the same width needs the real access.
//! `Requires16BitSharedAtomic` is still a stub in the C++, so only the 64-bit atomics matter.

use crate::gpu::shader_recompiler::ir::basic_block::BlockRef;
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::microinstruction::{invalidate, replace_uses_with, InstRef};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::r#type::Type;
use crate::gpu::shader_recompiler::ir::value::{U16, U32, U32U64, U64, U8U16U32U64};
use crate::gpu::shader_recompiler::profile::Profile;
use crate::gpu::shader_recompiler::runtime_info::HwStage;
use std::rc::Rc;

fn position(block: &BlockRef, inst: &InstRef) -> Option<usize> {
    block
        .borrow()
        .instructions
        .iter()
        .position(|candidate| Rc::ptr_eq(candidate, inst))
}

fn wide(value: &U32) -> U32U64 {
    U32U64::new(value.value().clone())
}

/// `Requires16BitSharedAtomic`: nothing is required yet in the C++ either.
fn requires_16bit_shared_atomic(_inst: &InstRef) -> bool {
    false
}

/// `Requires64BitSharedAtomic`.
fn requires_64bit_shared_atomic(inst: &InstRef) -> bool {
    matches!(
        inst.borrow().get_opcode(),
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
            | Opcode::SharedAtomicCmpSwap64
    )
}

/// `IsNon32BitSharedLoadStore`.
fn is_non_32bit_shared_load_store(inst: &InstRef) -> bool {
    matches!(
        inst.borrow().get_opcode(),
        Opcode::LoadSharedU16
            | Opcode::LoadSharedU64
            | Opcode::WriteSharedU16
            | Opcode::WriteSharedU64
    )
}

/// `CalculateSpecialSharedAtomicTypes`.
pub fn calculate_special_shared_atomic_types(program: &Program) -> Type {
    let mut extra_atomic_types = Type::Void;
    for block in &program.blocks {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            if requires_16bit_shared_atomic(inst) {
                extra_atomic_types = extra_atomic_types.union(Type::U16);
            }
            if requires_64bit_shared_atomic(inst) {
                extra_atomic_types = extra_atomic_types.union(Type::U64);
            }
        }
    }
    extra_atomic_types
}

/// `SharedMemorySimplifyPass`.
pub fn shared_memory_simplify_pass(program: &Program, profile: &Profile) {
    if program.info.borrow().hw_stage() != HwStage::Compute
        || profile.supports_workgroup_explicit_memory_layout
    {
        return;
    }

    let atomic_types = calculate_special_shared_atomic_types(program);
    let needs_u16 = !atomic_types.intersection(Type::U16).is_empty();
    let needs_u64 = !atomic_types.intersection(Type::U64).is_empty();
    if needs_u16 && needs_u64 {
        return;
    }

    for block in &program.blocks {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            if !is_non_32bit_shared_load_store(inst) {
                continue;
            }

            let opcode = inst.borrow().get_opcode();
            let index = position(block, inst).expect("the instruction is in its block");
            let mut ir = IrEmitter::at(block, index);
            let offset = U32::new(inst.borrow().arg(0));

            if !needs_u16 {
                match opcode {
                    Opcode::LoadSharedU16 => {
                        let mask = ir.imm32(!3u32);
                        let dword_offset =
                            U32::new(ir.bitwise_and(&wide(&offset), &wide(&mask)).into_value());
                        let dword_value = U32::new(ir.load_shared(32, false, &dword_offset, false));
                        let mask = ir.imm32(2u32);
                        let shifted = ir.bitwise_and(&wide(&offset), &wide(&mask));
                        let eight = ir.imm32(8u32);
                        let bit_offset = U32::new(ir.i_mul(&shifted, &wide(&eight)).into_value());
                        let count = ir.imm32(16u32);
                        let value = ir.bit_field_extract(&dword_value, &bit_offset, &count, false);
                        let converted = ir.u_convert(16, &U8U16U32U64::new(value.into_value()));
                        replace_uses_with(inst, converted.into_value());
                        continue;
                    }
                    Opcode::WriteSharedU16 => {
                        let value = U16::new(inst.borrow().arg(1));
                        let converted = ir.u_convert(32, &U8U16U32U64::new(value.into_value()));
                        let value = U32::new(converted.into_value());
                        let mask = ir.imm32(2u32);
                        let shifted = ir.bitwise_and(&wide(&offset), &wide(&mask));
                        let eight = ir.imm32(8u32);
                        let bit_offset = U32::new(ir.i_mul(&shifted, &wide(&eight)).into_value());
                        let mask = ir.imm32(!3u32);
                        let dword_offset =
                            U32::new(ir.bitwise_and(&wide(&offset), &wide(&mask)).into_value());
                        let combined = U32::new(
                            ir.bitwise_and(&wide(&offset), &wide(&dword_offset))
                                .into_value(),
                        );
                        let dword_value = U32::new(ir.load_shared(32, false, &combined, false));
                        let count = ir.imm32(16u32);
                        let new_dword_value = U32::new(
                            ir.bit_field_insert(
                                &wide(&dword_value),
                                &wide(&value),
                                &bit_offset,
                                &count,
                            )
                            .into_value(),
                        );
                        let new_value = new_dword_value.into_value();
                        ir.write_shared(32, &new_value, &dword_offset, false);
                        invalidate(inst);
                        continue;
                    }
                    _ => {}
                }
            }

            if !needs_u64 {
                match opcode {
                    Opcode::LoadSharedU64 => {
                        let value0 = ir.load_shared(32, false, &offset, false);
                        let four = ir.imm32(4u32);
                        let address = U32::new(ir.i_add(&wide(&offset), &wide(&four)).into_value());
                        let value1 = ir.load_shared(32, false, &address, false);
                        let composite = ir.composite_construct(value0, value1);
                        let packed = ir.pack_uint_2x32(&composite);
                        replace_uses_with(inst, packed.into_value());
                        continue;
                    }
                    Opcode::WriteSharedU64 => {
                        let packed = U64::new(inst.borrow().arg(1));
                        let value = ir.unpack_uint_2x32(&packed);
                        let value0 = ir.composite_extract(&value, 0);
                        let value1 = ir.composite_extract(&value, 1);
                        ir.write_shared(32, &value0, &offset, false);
                        let four = ir.imm32(4u32);
                        let address = U32::new(ir.i_add(&wide(&offset), &wide(&four)).into_value());
                        ir.write_shared(32, &value1, &address, false);
                        invalidate(inst);
                        continue;
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block};
    use crate::gpu::shader_recompiler::ir::value::Value;
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::SwStage;
    use std::cell::RefCell;

    fn program(stage: HwStage, block: &BlockRef) -> Program {
        let info = Rc::new(RefCell::new(Info::new(
            stage,
            SwStage::Compute,
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

    fn sink(block: &BlockRef, value: &InstRef) -> InstRef {
        append(
            block,
            Opcode::ConvertF16F32,
            &[Value::from_inst(value.clone())],
        )
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
    fn a_16bit_load_becomes_a_masked_dword_load() {
        let block = Block::new_ref();
        let load = append(&block, Opcode::LoadSharedU16, &[Value::from_u32(6)]);
        let load_sink = sink(&block, &load);

        shared_memory_simplify_pass(&program(HwStage::Compute, &block), &Profile::default());

        assert_eq!(
            opcodes(&block),
            vec![
                Opcode::BitwiseAnd32,
                Opcode::LoadSharedU32,
                Opcode::BitwiseAnd32,
                Opcode::IMul32,
                Opcode::BitFieldUExtract,
                Opcode::ConvertU16U32,
                Opcode::Void,
                Opcode::ConvertF16F32,
            ]
        );
        let converted = load_sink.borrow().arg(0);
        assert_eq!(
            converted.inst().borrow().get_opcode(),
            Opcode::ConvertU16U32
        );
    }

    #[test]
    fn a_16bit_store_is_a_read_modify_write() {
        let block = Block::new_ref();
        let store = append(
            &block,
            Opcode::WriteSharedU16,
            &[Value::from_u32(6), Value::from_u16(0x1234)],
        );

        shared_memory_simplify_pass(&program(HwStage::Compute, &block), &Profile::default());

        let ops = opcodes(&block);
        assert_eq!(
            &ops[..7],
            &[
                Opcode::ConvertU32U16,
                Opcode::BitwiseAnd32,
                Opcode::IMul32,
                Opcode::BitwiseAnd32,
                Opcode::BitwiseAnd32,
                Opcode::LoadSharedU32,
                Opcode::BitFieldInsert32,
            ]
        );
        assert_eq!(&ops[7..], &[Opcode::WriteSharedU32, Opcode::Void]);
        assert_eq!(store.borrow().get_opcode(), Opcode::Void);
        let write = block.borrow().instructions[7].clone();
        assert_eq!(write.borrow().get_opcode(), Opcode::WriteSharedU32);
    }

    #[test]
    fn a_64bit_load_becomes_two_dword_loads_and_a_pack() {
        let block = Block::new_ref();
        let load = append(&block, Opcode::LoadSharedU64, &[Value::from_u32(4)]);
        let load_sink = sink(&block, &load);

        shared_memory_simplify_pass(&program(HwStage::Compute, &block), &Profile::default());

        assert_eq!(
            opcodes(&block),
            vec![
                Opcode::LoadSharedU32,
                Opcode::IAdd32,
                Opcode::LoadSharedU32,
                Opcode::CompositeConstructU32x2,
                Opcode::PackUint2x32,
                Opcode::Void,
                Opcode::ConvertF16F32,
            ]
        );
        assert_eq!(
            load_sink.borrow().arg(0).inst().borrow().get_opcode(),
            Opcode::PackUint2x32
        );
    }

    #[test]
    fn a_64bit_store_becomes_two_dword_stores_and_an_unpack() {
        let block = Block::new_ref();
        append(
            &block,
            Opcode::WriteSharedU64,
            &[Value::from_u32(4), Value::from_u64(0x1122_3344_5566_7788)],
        );

        shared_memory_simplify_pass(&program(HwStage::Compute, &block), &Profile::default());

        assert_eq!(
            opcodes(&block),
            vec![
                Opcode::UnpackUint2x32,
                Opcode::CompositeExtractU32x2,
                Opcode::CompositeExtractU32x2,
                Opcode::WriteSharedU32,
                Opcode::IAdd32,
                Opcode::WriteSharedU32,
                Opcode::Void,
            ]
        );
    }

    #[test]
    fn the_guards_leave_64bit_shared_accesses_alone() {
        let block = Block::new_ref();
        append(
            &block,
            Opcode::SharedAtomicIAdd64,
            &[Value::from_u32(0), Value::from_u64(1)],
        );
        append(&block, Opcode::LoadSharedU64, &[Value::from_u32(4)]);

        shared_memory_simplify_pass(&program(HwStage::Compute, &block), &Profile::default());
        assert_eq!(
            opcodes(&block),
            vec![Opcode::SharedAtomicIAdd64, Opcode::LoadSharedU64]
        );

        let block = Block::new_ref();
        append(&block, Opcode::LoadSharedU16, &[Value::from_u32(0)]);
        let profile = Profile {
            supports_workgroup_explicit_memory_layout: true,
            ..Default::default()
        };
        shared_memory_simplify_pass(&program(HwStage::Compute, &block), &profile);
        assert_eq!(opcodes(&block), vec![Opcode::LoadSharedU16]);

        let block = Block::new_ref();
        append(&block, Opcode::LoadSharedU16, &[Value::from_u32(0)]);
        shared_memory_simplify_pass(&program(HwStage::Vertex, &block), &Profile::default());
        assert_eq!(opcodes(&block), vec![Opcode::LoadSharedU16]);
    }

    #[test]
    fn the_special_atomic_widths_come_from_the_64bit_atomics() {
        let block = Block::new_ref();
        append(
            &block,
            Opcode::SharedAtomicIAdd64,
            &[Value::from_u32(0), Value::from_u64(1)],
        );
        assert_eq!(
            calculate_special_shared_atomic_types(&program(HwStage::Compute, &block)),
            Type::U64
        );

        let block = Block::new_ref();
        append(&block, Opcode::LoadSharedU16, &[Value::from_u32(0)]);
        assert!(
            calculate_special_shared_atomic_types(&program(HwStage::Compute, &block)).is_empty()
        );
    }
}
