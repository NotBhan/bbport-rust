//! Buffer-format lowering, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/lower_buffer_format_to_raw.cpp`.
//!
//! `LoadBufferFormatF32`/`StoreBufferFormatF32` carry a typed data format, number format and
//! component mapping. This pass replaces each one with raw 8/16/32-bit buffer accesses, unpacking
//! or packing the components and applying the number conversion and destination swizzle by hand.

use crate::gpu::amdgpu::pixel_format::{
    map_number_conversion, num_components, remap_data_format, remap_number_format, remap_swizzle,
    CompMapping, DataFormat, NumberConversion, NumberFormat,
};
use crate::gpu::amdgpu::resource::Buffer;
use crate::gpu::shader_recompiler::info::Info;
use crate::gpu::shader_recompiler::ir::basic_block::BlockRef;
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::microinstruction::{invalidate, replace_uses_with, InstRef};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::reg::BufferInstInfo;
use crate::gpu::shader_recompiler::ir::reinterpret::{
    apply_read_number_conversion, apply_swizzle, apply_write_number_conversion,
};
use crate::gpu::shader_recompiler::ir::value::{Value, F32, U16, U32, U8, U8U16U32U64};
use std::rc::Rc;

/// `FormatInfo`.
struct FormatInfo {
    data_format: DataFormat,
    num_format: NumberFormat,
    swizzle: CompMapping,
    num_conversion: NumberConversion,
    num_components: u32,
}

fn is_buffer_format_load(inst: &InstRef) -> bool {
    inst.borrow().get_opcode() == Opcode::LoadBufferFormatF32
}

fn is_buffer_format_store(inst: &InstRef) -> bool {
    inst.borrow().get_opcode() == Opcode::StoreBufferFormatF32
}

/// `LoadBufferFormat`.
fn load_buffer_format(
    ir: &mut IrEmitter,
    handle: &Value,
    address: &U32,
    info: BufferInstInfo,
    format_info: &FormatInfo,
) -> Value {
    let raw_address = address.value().clone();
    let interpreted = match format_info.data_format {
        DataFormat::FormatInvalid => ir.imm32_float(0.0).into_value(),
        DataFormat::Format8 => {
            let raw = ir.load_buffer_u8(handle, &raw_address, info);
            let converted = ir.u_convert(32, &U8U16U32U64::new(raw.into_value()));
            let raw32 = U32::new(converted.into_value());
            let unpacked = ir.unpack_4x8(format_info.num_format, &raw32);
            ir.composite_extract(&unpacked, 0)
        }
        DataFormat::Format8_8 => {
            let raw = ir.load_buffer_u16(handle, &raw_address, info);
            let converted = ir.u_convert(32, &U8U16U32U64::new(raw.into_value()));
            let raw32 = U32::new(converted.into_value());
            let unpacked = ir.unpack_4x8(format_info.num_format, &raw32);
            let r = ir.composite_extract(&unpacked, 0);
            let g = ir.composite_extract(&unpacked, 1);
            ir.composite_construct(r, g)
        }
        DataFormat::Format8_8_8_8 => {
            let raw = ir.load_buffer_u32(1, handle, &raw_address, info);
            let raw32 = U32::new(raw);
            ir.unpack_4x8(format_info.num_format, &raw32)
        }
        DataFormat::Format16 => {
            let raw = ir.load_buffer_u16(handle, &raw_address, info);
            let converted = ir.u_convert(32, &U8U16U32U64::new(raw.into_value()));
            let raw32 = U32::new(converted.into_value());
            let unpacked = ir.unpack_2x16(format_info.num_format, &raw32);
            ir.composite_extract(&unpacked, 0)
        }
        DataFormat::Format16_16 => {
            let raw = ir.load_buffer_u32(1, handle, &raw_address, info);
            let raw32 = U32::new(raw);
            ir.unpack_2x16(format_info.num_format, &raw32)
        }
        DataFormat::Format10_11_11 => {
            let raw = ir.load_buffer_u32(1, handle, &raw_address, info);
            let raw32 = U32::new(raw);
            ir.unpack_10_11_11(format_info.num_format, &raw32)
        }
        DataFormat::Format2_10_10_10 => {
            let raw = ir.load_buffer_u32(1, handle, &raw_address, info);
            let raw32 = U32::new(raw);
            ir.unpack_2_10_10_10(format_info.num_format, &raw32)
        }
        DataFormat::Format16_16_16_16 => {
            let raw = ir.load_buffer_u32(2, handle, &raw_address, info);
            let low = ir.composite_extract(&raw, 0);
            let high = ir.composite_extract(&raw, 1);
            let low_unpacked = ir.unpack_2x16(format_info.num_format, &U32::new(low));
            let high_unpacked = ir.unpack_2x16(format_info.num_format, &U32::new(high));
            ir.composite_construct(low_unpacked, high_unpacked)
        }
        DataFormat::Format32
        | DataFormat::Format32_32
        | DataFormat::Format32_32_32
        | DataFormat::Format32_32_32_32 => {
            assert!(matches!(
                format_info.num_format,
                NumberFormat::Uint | NumberFormat::Sint | NumberFormat::Float
            ));
            ir.load_buffer_f32(
                format_info.num_components as i32,
                handle,
                &raw_address,
                info,
            )
        }
        other => panic!("Unsupported buffer data format: {}", other),
    };

    let mut components: Vec<Value> = Vec::with_capacity(4);
    for index in 0..4u32 {
        if index < format_info.num_components {
            let component = if format_info.num_components == 1 {
                interpreted.clone()
            } else {
                ir.composite_extract(&interpreted, index as usize)
            };
            let converted =
                apply_read_number_conversion(ir, &F32::new(component), format_info.num_conversion);
            components.push(converted.into_value());
        } else {
            components.push(ir.imm32_float(0.0).into_value());
        }
    }
    let composite = ir.composite_construct_slice(&components);
    apply_swizzle(ir, &composite, format_info.swizzle)
}

/// `StoreBufferFormat`.
fn store_buffer_format(
    ir: &mut IrEmitter,
    handle: &Value,
    address: &U32,
    value: &Value,
    info: BufferInstInfo,
    format_info: &FormatInfo,
) {
    let raw_address = address.value().clone();
    let swizzled = apply_swizzle(ir, value, format_info.swizzle.inverse());
    let mut components: Vec<Value> = Vec::with_capacity(format_info.num_components as usize);
    for index in 0..format_info.num_components {
        let component = ir.composite_extract(&swizzled, index as usize);
        let converted =
            apply_write_number_conversion(ir, &F32::new(component), format_info.num_conversion);
        components.push(converted.into_value());
    }
    let real_value = if components.len() == 1 {
        components[0].clone()
    } else {
        ir.composite_construct_slice(&components)
    };

    match format_info.data_format {
        DataFormat::FormatInvalid => {}
        DataFormat::Format8 => {
            let zero1 = ir.imm32_float(0.0);
            let zero2 = ir.imm32_float(0.0);
            let zero3 = ir.imm32_float(0.0);
            let vector = ir.composite_construct4(real_value, zero1, zero2, zero3);
            let packed = ir.pack_4x8(format_info.num_format, &vector);
            let converted = ir.u_convert(8, &U8U16U32U64::new(packed.into_value()));
            let data = U8::new(converted.into_value());
            ir.store_buffer_u8(handle, &raw_address, &data, info);
        }
        DataFormat::Format8_8 => {
            let r = ir.composite_extract(&real_value, 0);
            let g = ir.composite_extract(&real_value, 1);
            let zero1 = ir.imm32_float(0.0);
            let zero2 = ir.imm32_float(0.0);
            let vector = ir.composite_construct4(r, g, zero1, zero2);
            let packed = ir.pack_4x8(format_info.num_format, &vector);
            let converted = ir.u_convert(16, &U8U16U32U64::new(packed.into_value()));
            let data = U16::new(converted.into_value());
            ir.store_buffer_u16(handle, &raw_address, &data, info);
        }
        DataFormat::Format8_8_8_8 => {
            let packed = ir.pack_4x8(format_info.num_format, &real_value);
            ir.store_buffer_u32(1, handle, &raw_address, &packed.into_value(), info);
        }
        DataFormat::Format16 => {
            let zero = ir.imm32_float(0.0);
            let vector = ir.composite_construct(real_value, zero);
            let packed = ir.pack_2x16(format_info.num_format, &vector);
            let converted = ir.u_convert(16, &U8U16U32U64::new(packed.into_value()));
            let data = U16::new(converted.into_value());
            ir.store_buffer_u16(handle, &raw_address, &data, info);
        }
        DataFormat::Format16_16 => {
            let packed = ir.pack_2x16(format_info.num_format, &real_value);
            ir.store_buffer_u32(1, handle, &raw_address, &packed.into_value(), info);
        }
        DataFormat::Format10_11_11 => {
            let packed = ir.pack_10_11_11(format_info.num_format, &real_value);
            ir.store_buffer_u32(1, handle, &raw_address, &packed.into_value(), info);
        }
        DataFormat::Format2_10_10_10 => {
            let packed = ir.pack_2_10_10_10(format_info.num_format, &real_value);
            ir.store_buffer_u32(1, handle, &raw_address, &packed.into_value(), info);
        }
        DataFormat::Format16_16_16_16 => {
            let r = ir.composite_extract(&real_value, 0);
            let g = ir.composite_extract(&real_value, 1);
            let b = ir.composite_extract(&real_value, 2);
            let a = ir.composite_extract(&real_value, 3);
            let low = ir.composite_construct(r, g);
            let high = ir.composite_construct(b, a);
            let low_packed = ir.pack_2x16(format_info.num_format, &low);
            let high_packed = ir.pack_2x16(format_info.num_format, &high);
            let packed = ir.composite_construct(low_packed, high_packed);
            ir.store_buffer_u32(2, handle, &raw_address, &packed, info);
        }
        DataFormat::Format32
        | DataFormat::Format32_32
        | DataFormat::Format32_32_32
        | DataFormat::Format32_32_32_32 => {
            assert!(matches!(
                format_info.num_format,
                NumberFormat::Uint | NumberFormat::Sint | NumberFormat::Float
            ));
            ir.store_buffer_f32(
                format_info.num_components as i32,
                handle,
                &raw_address,
                &real_value,
                info,
            );
        }
        other => panic!("Unsupported buffer data format: {}", other),
    }
}

/// `LowerBufferFormatInst`.
fn lower_buffer_format_inst(block: &BlockRef, inst: &InstRef, info: &Info) {
    let index = block
        .borrow()
        .instructions
        .iter()
        .position(|candidate| Rc::ptr_eq(candidate, inst))
        .expect("the instruction is in its block");
    let mut ir = IrEmitter::at(block, index);
    let flags = inst.borrow().buffer_info();
    let handle = inst.borrow().arg(0);
    let address = U32::new(inst.borrow().arg(1));
    let desc = info.persistent.buffers[handle.u32() as usize];
    let buffer: Buffer = desc.get_sharp(info);

    let inst_data_format = flags.inst_data_format();
    let is_inst_typed = inst_data_format.is_some_and(|format| format != DataFormat::FormatInvalid);
    let data_format = if is_inst_typed {
        remap_data_format(inst_data_format.expect("the typed format"))
    } else {
        buffer.get_data_fmt()
    };
    let format_info = FormatInfo {
        data_format,
        num_format: if is_inst_typed {
            remap_number_format(
                flags.inst_number_format().unwrap_or(NumberFormat::Unorm),
                data_format,
            )
        } else {
            buffer.get_number_fmt()
        },
        swizzle: if is_inst_typed {
            remap_swizzle(
                inst_data_format.expect("the typed format"),
                CompMapping::IDENTITY,
            )
        } else {
            buffer.dst_select()
        },
        num_conversion: if is_inst_typed {
            map_number_conversion(
                flags.inst_number_format().unwrap_or(NumberFormat::Unorm),
                inst_data_format.expect("the typed format"),
            )
        } else {
            buffer.get_number_conversion()
        },
        num_components: num_components(data_format).unwrap_or(0),
    };

    if data_format == DataFormat::FormatInvalid {
        if is_buffer_format_load(inst) {
            let zero0 = ir.imm32_float(0.0);
            let zero1 = ir.imm32_float(0.0);
            let zero2 = ir.imm32_float(0.0);
            let zero3 = ir.imm32_float(0.0);
            let zero = ir.composite_construct4(zero0, zero1, zero2, zero3);
            replace_uses_with(inst, zero);
        } else {
            invalidate(inst);
        }
        return;
    }

    if is_buffer_format_load(inst) {
        let interpreted = load_buffer_format(&mut ir, &handle, &address, flags, &format_info);
        replace_uses_with(inst, interpreted);
    } else if is_buffer_format_store(inst) {
        let value = inst.borrow().arg(2);
        store_buffer_format(&mut ir, &handle, &address, &value, flags, &format_info);
        invalidate(inst);
    }
}

/// `LowerBufferFormatToRaw`.
pub fn lower_buffer_format_to_raw(program: &Program) {
    let info = program.info.clone();
    for block in &program.blocks {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            if is_buffer_format_load(inst) || is_buffer_format_store(inst) {
                let info = info.borrow();
                lower_buffer_format_inst(block, inst, &info);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::amdgpu::pixel_format::CompSwizzle;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block};
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::resource::BufferResource;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage};
    use std::cell::RefCell;

    fn buffer(format: DataFormat, number_format: NumberFormat) -> Buffer {
        let mut buffer = Buffer::default();
        buffer.set_base_address(1);
        buffer.set_data_format(format.raw());
        buffer.set_num_format(number_format.raw());
        buffer.word2 |= (CompSwizzle::Red as u32)
            | ((CompSwizzle::Green as u32) << 3)
            | ((CompSwizzle::Blue as u32) << 6)
            | ((CompSwizzle::Alpha as u32) << 9);
        buffer
    }

    fn program(block: &BlockRef, buffer: Buffer) -> Program {
        let info = Rc::new(RefCell::new(Info::new(
            HwStage::Vertex,
            SwStage::Vertex,
            ShaderParams::new([0; 16], &[], 0),
        )));
        let mut resource = BufferResource::default();
        resource.sharp_fetch.immediates[..4].copy_from_slice(&buffer.dwords());
        info.borrow_mut().persistent.buffers.push(resource);
        let mut program = Program::new(info);
        program.blocks = vec![block.clone()];
        program
    }

    fn append(block: &BlockRef, op: Opcode, args: &[Value]) -> InstRef {
        let index = append_new_inst(block, op, args);
        block.borrow().instructions[index].clone()
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
    fn a_formatted_load_becomes_a_raw_load_and_an_unpack() {
        let block = Block::new_ref();
        let handle = Value::from_u32(0);
        let address = append(&block, Opcode::LaneId, &[]);
        let load = append(
            &block,
            Opcode::LoadBufferFormatF32,
            &[handle.clone(), Value::from_inst(address)],
        );
        let sink = append(
            &block,
            Opcode::ConvertF16F32,
            &[Value::from_inst(load.clone())],
        );

        lower_buffer_format_to_raw(&program(
            &block,
            buffer(DataFormat::Format8_8_8_8, NumberFormat::Unorm),
        ));

        assert_eq!(load.borrow().get_opcode(), Opcode::Void);
        let result = sink.borrow().arg(0).inst();
        assert_eq!(result.borrow().get_opcode(), Opcode::CompositeShuffleF32x4);
        let ops = opcodes(&block);
        assert!(ops.contains(&Opcode::LoadBufferU32));
        assert!(ops.contains(&Opcode::UnpackUnorm4x8));
        assert!(ops.contains(&Opcode::CompositeExtractF32x4));
    }

    #[test]
    fn an_invalid_format_loads_a_zero_vector() {
        let block = Block::new_ref();
        let handle = Value::from_u32(0);
        let address = append(&block, Opcode::LaneId, &[]);
        let load = append(
            &block,
            Opcode::LoadBufferFormatF32,
            &[handle.clone(), Value::from_inst(address)],
        );
        let sink = append(
            &block,
            Opcode::ConvertF16F32,
            &[Value::from_inst(load.clone())],
        );

        lower_buffer_format_to_raw(&program(&block, Buffer::default()));

        assert_eq!(load.borrow().get_opcode(), Opcode::Void);
        let result = sink.borrow().arg(0).inst();
        assert_eq!(
            result.borrow().get_opcode(),
            Opcode::CompositeConstructF32x4
        );
        for index in 0..4 {
            assert_eq!(result.borrow().arg(index).f32(), 0.0);
        }
    }

    #[test]
    fn a_formatted_store_becomes_a_pack_and_a_raw_store() {
        let block = Block::new_ref();
        let handle = Value::from_u32(0);
        let address = append(&block, Opcode::LaneId, &[]);
        let value = append(
            &block,
            Opcode::CompositeConstructF32x4,
            &[
                Value::from_f32(1.0),
                Value::from_f32(2.0),
                Value::from_f32(3.0),
                Value::from_f32(4.0),
            ],
        );
        let store = append(
            &block,
            Opcode::StoreBufferFormatF32,
            &[
                handle.clone(),
                Value::from_inst(address),
                Value::from_inst(value),
            ],
        );

        lower_buffer_format_to_raw(&program(
            &block,
            buffer(DataFormat::Format8_8_8_8, NumberFormat::Unorm),
        ));

        assert_eq!(store.borrow().get_opcode(), Opcode::Void);
        let ops = opcodes(&block);
        assert!(ops.contains(&Opcode::PackUnorm4x8));
        assert!(ops.contains(&Opcode::StoreBufferU32));
    }
}
