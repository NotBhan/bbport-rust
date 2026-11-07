//! Resource patching, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/resource_patching_pass.cpp`.
//!
//! The first pass turns each discovered sharp into a descriptor list entry and replaces the
//! instruction's handle with the binding. The second pass rewrites buffer addresses, image
//! coordinates and data-ring accesses against the descriptors. The C++ `Descriptors` holds
//! references into `Info`; the port moves the four lists out for the duration of the pass and
//! puts them back, because the pass needs `Info` immutably while it appends to them.

use crate::gpu::amdgpu::pixel_format::{is_fmask, remap_data_format, DataFormat, NumberFormat};
use crate::gpu::amdgpu::resource::{Buffer, Image, ImageType, Sampler};
use crate::gpu::shader_recompiler::info::Info;
use crate::gpu::shader_recompiler::ir::basic_block::BlockRef;
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::microinstruction::{
    invalidate, replace_uses_with, set_arg, InstRef, Use,
};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::operand_helper::{
    get_buffer_index_arg, get_buffer_soffset_arg, get_buffer_voffset_arg, load_buffer_args,
};
use crate::gpu::shader_recompiler::ir::passes::resource_pass::{
    is_buffer_instruction, is_buffer_store, is_data_ring_instruction, is_image_atomic_instruction,
    is_image_instruction, ResourceDiscovery, SharpReference,
};
use crate::gpu::shader_recompiler::ir::r#type::Type;
use crate::gpu::shader_recompiler::ir::reinterpret::{
    apply_force_degamma, apply_read_number_conversion_vec4, apply_swizzle,
    apply_write_number_conversion_vec4,
};
use crate::gpu::shader_recompiler::ir::value::{Value, F32, F32F64, U1, U16, U32, U32U64, U64};
use crate::gpu::shader_recompiler::profile::Profile;
use crate::gpu::shader_recompiler::resource::{
    BufferResource, BufferResourceList, BufferType, FMaskResource, FMaskResourceList,
    ImageResource, ImageResourceList, MipStorageFallbackMode, SamplerResource, SamplerResourceList,
    Sharp, SharpFetch, SharpLocation, UNKNOWN_LOCATION,
};
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

fn i_add(ir: &mut IrEmitter, a: &U32, b: &U32) -> U32 {
    U32::new(ir.i_add(&wide(a), &wide(b)).into_value())
}

fn i_mul(ir: &mut IrEmitter, a: &U32, b: &U32) -> U32 {
    U32::new(ir.i_mul(&wide(a), &wide(b)).into_value())
}

fn shift_right(ir: &mut IrEmitter, a: &U32, shift: u32) -> U32 {
    let shift = ir.imm32(shift);
    U32::new(ir.shift_right_logical(&wide(a), &shift).into_value())
}

fn narrow(value: F32F64) -> F32 {
    F32::new(value.into_value())
}

fn widen(value: &F32) -> F32F64 {
    F32F64::new(value.value().clone())
}

fn add<T: Clone>(descriptors: &mut Vec<T>, desc: &T, pred: impl Fn(&T) -> bool) -> u32 {
    if let Some(index) = descriptors.iter().position(pred) {
        return index as u32;
    }
    descriptors.push(desc.clone());
    (descriptors.len() - 1) as u32
}

/// `Descriptors`.
struct Descriptors {
    buffer_resources: BufferResourceList,
    image_resources: ImageResourceList,
    sampler_resources: SamplerResourceList,
    fmask_resources: FMaskResourceList,
}

impl Descriptors {
    fn take(info: &mut Info) -> Self {
        Descriptors {
            buffer_resources: std::mem::take(&mut info.persistent.buffers),
            image_resources: std::mem::take(&mut info.persistent.images),
            sampler_resources: std::mem::take(&mut info.persistent.samplers),
            fmask_resources: std::mem::take(&mut info.persistent.fmasks),
        }
    }

    fn restore(self, info: &mut Info) {
        info.persistent.buffers = self.buffer_resources;
        info.persistent.images = self.image_resources;
        info.persistent.samplers = self.sampler_resources;
        info.persistent.fmasks = self.fmask_resources;
    }

    fn add_buffer(&mut self, desc: BufferResource) -> u32 {
        let index = add(&mut self.buffer_resources, &desc, |existing| {
            desc.sharp_fetch == existing.sharp_fetch
                && desc.post_op == existing.post_op
                && desc.post_op_dw1_mask == existing.post_op_dw1_mask
                && desc.buffer_type == existing.buffer_type
        });
        let buffer = &mut self.buffer_resources[index as usize];
        buffer.used_types = buffer.used_types.union(desc.used_types);
        buffer.is_written |= desc.is_written;
        buffer.is_formatted |= desc.is_formatted;
        index
    }

    fn add_image(&mut self, desc: ImageResource) -> u32 {
        let index = add(&mut self.image_resources, &desc, |existing| {
            desc.sharp_fetch == existing.sharp_fetch
                && desc.is_array == existing.is_array
                && desc.mip_fallback_mode == existing.mip_fallback_mode
                && desc.constant_mip_index == existing.constant_mip_index
                && desc.post_op == existing.post_op
        });
        let image = &mut self.image_resources[index as usize];
        image.is_atomic |= desc.is_atomic;
        image.is_written |= desc.is_written;
        image.needs_native |= desc.needs_native;
        index
    }

    fn add_sampler(&mut self, desc: SamplerResource) -> u32 {
        add(&mut self.sampler_resources, &desc, |existing| {
            desc.sharp_fetch == existing.sharp_fetch
                && desc.post_op == existing.post_op
                && desc.post_op_tsharp_dw3_off == existing.post_op_tsharp_dw3_off
                && desc.is_depth == existing.is_depth
        })
    }

    fn add_fmask(&mut self, desc: FMaskResource) -> u32 {
        add(&mut self.fmask_resources, &desc, |existing| {
            desc.sharp_idx == existing.sharp_idx
        })
    }
}

/// `BufferDataType`.
fn buffer_data_type(inst: &InstRef, profile: &Profile) -> Type {
    match inst.borrow().get_opcode() {
        Opcode::LoadBufferU8 | Opcode::StoreBufferU8 => Type::U8,
        Opcode::LoadBufferU16 | Opcode::StoreBufferU16 => Type::U16,
        Opcode::LoadBufferU64
        | Opcode::StoreBufferU64
        | Opcode::BufferAtomicIAdd64
        | Opcode::BufferAtomicSMax64
        | Opcode::BufferAtomicSMin64
        | Opcode::BufferAtomicUMax64
        | Opcode::BufferAtomicUMin64 => Type::U64,
        Opcode::BufferAtomicFMax32 | Opcode::BufferAtomicFMin32 => {
            if profile.supports_buffer_fp32_atomic_min_max {
                Type::F32
            } else {
                Type::U32
            }
        }
        Opcode::LoadBufferFormatF32 | Opcode::StoreBufferFormatF32 => {
            Type::U32.union(Type::F32).union(Type::U16).union(Type::U8)
        }
        _ => Type::U32,
    }
}

/// `BufferAddressShift`.
fn buffer_address_shift(inst: &InstRef, data_format: DataFormat) -> u32 {
    match inst.borrow().get_opcode() {
        Opcode::LoadBufferU8 | Opcode::StoreBufferU8 => 0,
        Opcode::LoadBufferU16 | Opcode::StoreBufferU16 => 1,
        Opcode::LoadBufferU64
        | Opcode::StoreBufferU64
        | Opcode::BufferAtomicIAdd64
        | Opcode::BufferAtomicSMax64
        | Opcode::BufferAtomicSMin64
        | Opcode::BufferAtomicUMax64
        | Opcode::BufferAtomicUMin64 => 3,
        Opcode::LoadBufferFormatF32 | Opcode::StoreBufferFormatF32 => match data_format {
            DataFormat::Format8 => 0,
            DataFormat::Format8_8 | DataFormat::Format16 => 1,
            DataFormat::Format8_8_8_8
            | DataFormat::Format16_16
            | DataFormat::Format10_11_11
            | DataFormat::Format2_10_10_10
            | DataFormat::Format16_16_16_16
            | DataFormat::Format32
            | DataFormat::Format32_32
            | DataFormat::Format32_32_32
            | DataFormat::Format32_32_32_32 => 2,
            _ => 0,
        },
        Opcode::ReadConstBuffer => 0,
        _ => 2,
    }
}

/// `SharpLocationFromSource`.
pub fn sharp_location_from_source(inst: &InstRef) -> SharpLocation {
    let location = match inst.borrow().get_opcode() {
        Opcode::GetUserData => inst.borrow().arg(0).scalar_reg() as SharpLocation,
        Opcode::ReadConstBuffer => inst.borrow().buffer_info().flatbuf_off_dw() as SharpLocation,
        _ => inst.borrow().flags() as SharpLocation,
    };
    if location == 0 {
        return UNKNOWN_LOCATION;
    }
    location
}

/// `ConstructSharpFetch`.
fn construct_sharp_fetch<T: Sharp>(sharp: &SharpReference) -> SharpFetch<T> {
    let mut fetch = SharpFetch::<T>::default();
    for index in 0..sharp.num_dwords as usize {
        let dword = sharp.dwords[index].clone();
        if dword.is_immediate() {
            fetch.immediates[index] = dword.u32();
        } else {
            fetch.offsets[index] = sharp_location_from_source(&dword.inst());
            fetch.load_mask |= 1 << index;
        }
    }
    fetch
}

/// `PatchBufferSharp`.
fn patch_buffer_sharp(
    resource: &ResourceDiscovery,
    descriptors: &mut Descriptors,
    profile: &Profile,
) {
    let inst = resource.user.clone();
    let opcode = inst.borrow().get_opcode();
    let binding = descriptors.add_buffer(BufferResource {
        sharp_fetch: construct_sharp_fetch::<Buffer>(&resource.sharps[0]),
        used_types: buffer_data_type(&inst, profile),
        buffer_type: BufferType::Guest,
        is_written: is_buffer_store(&inst),
        is_formatted: matches!(
            opcode,
            Opcode::LoadBufferFormatF32 | Opcode::StoreBufferFormatF32
        ),
        post_op: resource.sharps[0].post_op,
        post_op_dw1_mask: resource.sharps[0].post_op_data.dw1_mask,
    });

    let block = inst
        .borrow()
        .parent()
        .expect("the instruction has a parent");
    let index = position(&block, &inst).expect("the instruction is in its block");
    let ir = IrEmitter::at(&block, index);
    let binding = ir.imm32(binding);
    set_arg(&inst, 0, binding.into_value());
}

/// `PatchImageSharp`.
fn patch_image_sharp(
    resource: &ResourceDiscovery,
    info: &Info,
    descriptors: &mut Descriptors,
    profile: &Profile,
) {
    let inst = resource.user.clone();
    let inst_info = inst.borrow().texture_info();
    let opcode = inst.borrow().get_opcode();
    let is_atomic = is_image_atomic_instruction(&inst);
    let is_written = opcode == Opcode::ImageWrite || is_atomic;
    let needs_mip_storage_fallback =
        inst_info.has_lod() != 0 && is_written && !profile.supports_image_load_store_lod;
    let normalized_sample = matches!(
        opcode,
        Opcode::ImageSampleRaw
            | Opcode::ImageSampleImplicitLod
            | Opcode::ImageSampleExplicitLod
            | Opcode::ImageSampleDrefImplicitLod
            | Opcode::ImageSampleDrefExplicitLod
            | Opcode::ImageGather
            | Opcode::ImageGatherDref
            | Opcode::ImageGradient
            | Opcode::ImageQueryLod
    ) && inst_info.has_offset() == 0
        && inst_info.is_unnormalized() == 0;

    let mut image_res = ImageResource {
        sharp_fetch: construct_sharp_fetch::<Image>(&resource.sharps[0]),
        is_depth: inst_info.is_depth() != 0,
        is_atomic,
        is_array: inst_info.is_array() != 0,
        is_written,
        is_r128: inst_info.is_r128() != 0,
        needs_native: !normalized_sample,
        post_op: resource.sharps[0].post_op,
        ..Default::default()
    };

    let image = image_res.get_sharp(info);
    assert_ne!(
        image.get_type(),
        ImageType::Invalid,
        "the image sharp is invalid"
    );

    if needs_mip_storage_fallback {
        let view_type = image.get_view_type(image_res.is_array);
        let body = inst.borrow().arg(1).inst();
        let lod_arg = match view_type {
            ImageType::Color1D => body.borrow().arg(1),
            ImageType::Color1DArray | ImageType::Color2D => body.borrow().arg(2),
            ImageType::Color2DArray | ImageType::Color3D => body.borrow().arg(3),
            _ => panic!("Invalid image type {}", view_type as u64),
        };
        if lod_arg.is_immediate() {
            image_res.mip_fallback_mode = MipStorageFallbackMode::ConstantIndex;
            image_res.constant_mip_index = lod_arg.u32() as u8;
        } else {
            image_res.mip_fallback_mode = MipStorageFallbackMode::DynamicIndex;
        }
    }

    if is_fmask(image.get_data_fmt()) {
        assert!(!is_written, "FMask storage instructions are not supported");
        let block = inst
            .borrow()
            .parent()
            .expect("the instruction has a parent");
        let index = position(&block, &inst).expect("the instruction is in its block");
        let mut ir = IrEmitter::at(&block, index);
        match opcode {
            Opcode::ImageRead | Opcode::ImageSampleRaw => {
                let x = ir.imm32(0x76543210);
                let fmask_x = ir.bit_cast_f32_u32(&x);
                let y = ir.imm32(0xfedcba98);
                let fmask_y = ir.bit_cast_f32_u32(&y);
                let composite = ir.composite_construct(fmask_x, fmask_y);
                replace_uses_with(&inst, composite);
                return;
            }
            Opcode::ImageQueryLod => {
                let one = ir.imm32(1u32);
                replace_uses_with(&inst, one.into_value());
                return;
            }
            Opcode::ImageQueryDimensions => {
                let width = ir.imm32(image.width() as u32);
                let height = ir.imm32(image.height() as u32);
                let one = ir.imm32(1u32);
                let one_2 = ir.imm32(1u32);
                let dims = ir.composite_construct4(width, height, one, one_2);
                replace_uses_with(&inst, dims);
                let sharp_idx = sharp_location_from_source(&resource.sharps[0].dwords[0].inst());
                descriptors.add_fmask(FMaskResource { sharp_idx });
                return;
            }
            _ => panic!("Can't patch fmask instruction {}", opcode.name()),
        }
    }

    let image_binding = descriptors.add_image(image_res);

    let block = inst
        .borrow()
        .parent()
        .expect("the instruction has a parent");
    let index = position(&block, &inst).expect("the instruction is in its block");
    let ir = IrEmitter::at(&block, index);

    if opcode == Opcode::ImageSampleRaw {
        let lod_prod = resource.sharps[1].post_op_data.lod_prod.clone();
        let sampler_binding = descriptors.add_sampler(SamplerResource {
            sharp_fetch: construct_sharp_fetch::<Sampler>(&resource.sharps[1]),
            post_op: resource.sharps[1].post_op,
            post_op_tsharp_dw3_off: if lod_prod.is_empty() {
                UNKNOWN_LOCATION
            } else {
                sharp_location_from_source(&lod_prod.inst())
            },
            is_depth: inst_info.is_depth() != 0,
        });
        let handle = ir.imm32(image_binding | sampler_binding << 16);
        set_arg(&inst, 0, handle.into_value());
    } else {
        let handle = ir.imm32(image_binding);
        set_arg(&inst, 0, handle.into_value());
    }
}

/// `IsMbcntWithExec`.
fn is_mbcnt_with_exec(value: &Value, is_hi: bool, exec: &U1) -> Value {
    if value.is_immediate() {
        return Value::empty();
    }
    let mbcnt = value.inst();
    if mbcnt.borrow().get_opcode() != Opcode::MaskedBitCount32 {
        return Value::empty();
    }
    let mbcnt_arg0 = mbcnt.borrow().arg(0);
    if mbcnt_arg0.is_immediate() || mbcnt.borrow().arg(2).u1() != is_hi {
        return Value::empty();
    }
    let extract = mbcnt_arg0.inst();
    if extract.borrow().get_opcode() != Opcode::CompositeExtractU32x2 {
        return Value::empty();
    }
    let extract_arg0 = extract.borrow().arg(0);
    if extract_arg0.is_immediate() || extract.borrow().arg(1).u32() != u32::from(is_hi) {
        return Value::empty();
    }
    let unpack = extract_arg0.inst();
    if unpack.borrow().get_opcode() != Opcode::UnpackUint2x32 {
        return Value::empty();
    }
    let unpack_arg0 = unpack.borrow().arg(0);
    if unpack_arg0.is_immediate() {
        return Value::empty();
    }
    let ballot = unpack_arg0.inst();
    if ballot.borrow().get_opcode() != Opcode::Ballot {
        return Value::empty();
    }
    if !is_hi && ballot.borrow().arg(0) != exec.value().clone() {
        return Value::empty();
    }
    let result = mbcnt.borrow().arg(1);
    result
}

/// `FindUniqueUser`.
fn find_unique_user(inst: &InstRef, pred: impl Fn(&InstRef) -> bool) -> Use {
    let mut picked: Option<Use> = None;
    for r#use in inst.borrow().uses() {
        if pred(&r#use.user) {
            assert!(picked.is_none(), "expected a single matching user");
            picked = Some(r#use.clone());
        }
    }
    picked.expect("a unique user")
}

/// `RemoveAppendBufferLaneOffset`.
fn remove_append_buffer_lane_offset(vx: &InstRef) {
    let exec = U1::new(vx.borrow().arg(1));
    let uses = vx.borrow().uses().to_vec();
    for mut r#use in uses {
        let user = r#use.user.clone();
        let operand = r#use.operand as usize;
        if user.borrow().get_opcode() == Opcode::MaskedBitCount32 {
            assert_eq!(vx.borrow().get_opcode(), Opcode::DataAppend);
            let vy = user;
            let vx_2 = is_mbcnt_with_exec(&Value::from_inst(vy.clone()), true, &exec);
            assert!(
                !vx_2.is_empty() && vx_2 == Value::from_inst(vx.clone()),
                "the first mask count does not read the append result"
            );
            let found = find_unique_user(&vy, |inst| {
                inst.borrow().get_opcode() == Opcode::MaskedBitCount32
            });
            let idx = found.user;
            let vy_2 = is_mbcnt_with_exec(&Value::from_inst(idx.clone()), false, &exec);
            assert!(
                !vy_2.is_empty() && vy_2 == Value::from_inst(vy.clone()),
                "the second mask count does not read the first one"
            );
            replace_uses_with(&idx, Value::from_inst(vx.clone()));
            continue;
        }
        if !matches!(user.borrow().get_opcode(), Opcode::IAdd32 | Opcode::ISub32) {
            continue;
        }
        let other = user.borrow().arg(1 - operand);
        if other.is_immediate() {
            assert!(
                other.u32() == 1
                    || (other.u32() == u32::MAX && vx.borrow().get_opcode() == Opcode::DataConsume),
                "Unexpected constant offset {} to DataConsume result",
                other.u32()
            );
            r#use = find_unique_user(&user, |inst| {
                matches!(inst.borrow().get_opcode(), Opcode::IAdd32 | Opcode::ISub32)
            });
        }
        let user = r#use.user.clone();
        let operand = r#use.operand as usize;
        let vz = user.borrow().arg(1 - operand);
        let vy = is_mbcnt_with_exec(&vz, false, &exec);
        assert!(
            !vy.is_empty() && !vy.is_immediate(),
            "the lane offset is not a mask count"
        );
        let zero_const = is_mbcnt_with_exec(&vy, true, &exec);
        assert!(
            !zero_const.is_empty() && zero_const.is_immediate() && zero_const.u32() == 0,
            "the high mask count does not start from zero"
        );
        replace_uses_with(&vz.inst(), Value::from_u32(0));
    }
}

/// `PatchGlobalDataShareAccess`.
fn patch_global_data_share_access(inst: &InstRef, descriptors: &mut Descriptors) {
    let binding = descriptors.add_buffer(BufferResource {
        used_types: Type::U32,
        buffer_type: BufferType::GdsBuffer,
        is_written: true,
        ..Default::default()
    });

    let block = inst
        .borrow()
        .parent()
        .expect("the instruction has a parent");
    let index = position(&block, inst).expect("the instruction is in its block");
    let mut ir = IrEmitter::at(&block, index);

    let opcode = inst.borrow().get_opcode();
    if matches!(opcode, Opcode::DataAppend | Opcode::DataConsume) {
        remove_append_buffer_lane_offset(inst);
        let handle = ir.imm32(binding);
        let address = inst.borrow().arg(0);
        let one_handle = handle;
        let one = ir.imm32(1u32);
        let replacement = if opcode == Opcode::DataAppend {
            ir.buffer_atomic_i_add(
                &one_handle.into_value(),
                &address,
                &one.into_value(),
                Default::default(),
            )
        } else {
            ir.buffer_atomic_i_sub(
                &one_handle.into_value(),
                &address,
                &one.into_value(),
                Default::default(),
            )
        };
        replace_uses_with(inst, replacement);
        return;
    }

    let offset = U32::new(inst.borrow().arg(0));
    let address_words = shift_right(&mut ir, &offset, 1);
    let address_dwords = shift_right(&mut ir, &offset, 2);
    let address_qwords = shift_right(&mut ir, &offset, 3);
    let handle = ir.imm32(binding);
    let handle = handle.into_value();
    let words = address_words.into_value();
    let dwords = address_dwords.into_value();
    let qwords = address_qwords.into_value();

    let mut used_type = None;
    match opcode {
        Opcode::SharedAtomicIAdd32 => {
            let value = inst.borrow().arg(1);
            let replacement = ir.buffer_atomic_i_add(&handle, &dwords, &value, Default::default());
            replace_uses_with(inst, replacement);
        }
        Opcode::SharedAtomicIAdd64 => {
            let value = U64::new(inst.borrow().arg(1));
            let replacement =
                ir.buffer_atomic_i_add(&handle, &qwords, &value.into_value(), Default::default());
            replace_uses_with(inst, replacement);
        }
        Opcode::SharedAtomicISub32 => {
            let value = inst.borrow().arg(1);
            let replacement = ir.buffer_atomic_i_sub(&handle, &dwords, &value, Default::default());
            replace_uses_with(inst, replacement);
        }
        Opcode::SharedAtomicSMin32 | Opcode::SharedAtomicUMin32 => {
            let is_signed = opcode == Opcode::SharedAtomicSMin32;
            let value = inst.borrow().arg(1);
            let replacement =
                ir.buffer_atomic_i_min(&handle, &dwords, &value, is_signed, Default::default());
            replace_uses_with(inst, replacement);
        }
        Opcode::SharedAtomicSMax32 | Opcode::SharedAtomicUMax32 => {
            let is_signed = opcode == Opcode::SharedAtomicSMax32;
            let value = inst.borrow().arg(1);
            let replacement =
                ir.buffer_atomic_i_max(&handle, &dwords, &value, is_signed, Default::default());
            replace_uses_with(inst, replacement);
        }
        Opcode::SharedAtomicInc32 => {
            let replacement = ir.buffer_atomic_inc(&handle, &dwords, Default::default());
            replace_uses_with(inst, replacement);
        }
        Opcode::SharedAtomicDec32 => {
            let replacement = ir.buffer_atomic_dec(&handle, &dwords, Default::default());
            replace_uses_with(inst, replacement);
        }
        Opcode::SharedAtomicAnd32 => {
            let value = inst.borrow().arg(1);
            let replacement = ir.buffer_atomic_and(&handle, &dwords, &value, Default::default());
            replace_uses_with(inst, replacement);
        }
        Opcode::SharedAtomicOr32 => {
            let value = inst.borrow().arg(1);
            let replacement = ir.buffer_atomic_or(&handle, &dwords, &value, Default::default());
            replace_uses_with(inst, replacement);
        }
        Opcode::SharedAtomicXor32 => {
            let value = inst.borrow().arg(1);
            let replacement = ir.buffer_atomic_xor(&handle, &dwords, &value, Default::default());
            replace_uses_with(inst, replacement);
        }
        Opcode::SharedAtomicCmpSwap32 => {
            let value = inst.borrow().arg(1);
            let compare = inst.borrow().arg(2);
            let replacement =
                ir.buffer_atomic_cmp_swap(&handle, &dwords, &value, &compare, Default::default());
            replace_uses_with(inst, replacement);
        }
        Opcode::LoadSharedU16 => {
            let replacement = ir.load_buffer_u16(&handle, &words, Default::default());
            replace_uses_with(inst, replacement.into_value());
            used_type = Some(Type::U16);
        }
        Opcode::LoadSharedU32 => {
            let replacement = ir.load_buffer_u32(1, &handle, &dwords, Default::default());
            replace_uses_with(inst, replacement);
        }
        Opcode::LoadSharedU64 => {
            let replacement = ir.load_buffer_u64(&handle, &qwords, Default::default());
            replace_uses_with(inst, replacement.into_value());
            used_type = Some(Type::U64);
        }
        Opcode::WriteSharedU16 => {
            let value = U16::new(inst.borrow().arg(1));
            ir.store_buffer_u16(&handle, &words, &value, Default::default());
            invalidate(inst);
            used_type = Some(Type::U16);
        }
        Opcode::WriteSharedU32 => {
            let value = inst.borrow().arg(1);
            ir.store_buffer_u32(1, &handle, &dwords, &value, Default::default());
            invalidate(inst);
        }
        Opcode::WriteSharedU64 => {
            let value = U64::new(inst.borrow().arg(1));
            ir.store_buffer_u64(&handle, &qwords, &value, Default::default());
            invalidate(inst);
            used_type = Some(Type::U64);
        }
        _ => panic!("Unexpected opcode {}", opcode.name()),
    }

    if let Some(used_type) = used_type {
        let buffer = &mut descriptors.buffer_resources[binding as usize];
        buffer.used_types = buffer.used_types.union(used_type);
    }
}

/// `CalculateBufferAddress`.
fn calculate_buffer_address(
    ir: &mut IrEmitter,
    inst: &InstRef,
    info: &Info,
    buffer: &Buffer,
    stride: u32,
) -> U32 {
    let inst_info = inst.borrow().buffer_info();
    let inst_offset = inst_info.inst_offset() as u32;
    let is_inst_typed = inst_info.inst_data_format().is_some();
    let data_format = if is_inst_typed {
        remap_data_format(inst_info.inst_data_format().expect("the typed format"))
    } else {
        buffer.get_data_fmt()
    };
    let shift = buffer_address_shift(inst, data_format);
    let mask = (1u32 << shift) - 1;
    let soffset = get_buffer_soffset_arg(inst);

    if inst_info.index_enable() != 0
        && inst_info.voffset_enable() == 0
        && soffset.value().is_immediate()
        && !buffer.swizzle_enable()
        && !buffer.add_tid_enable()
        && (stride & mask) == 0
    {
        let total_offset = soffset.value().u32() + inst_offset;
        if total_offset & mask == 0 {
            let index = get_buffer_index_arg(inst);
            let stride_imm = ir.imm32(stride >> shift);
            let scaled = i_mul(ir, &index, &stride_imm);
            let offset_imm = ir.imm32(total_offset >> shift);
            return i_add(ir, &scaled, &offset_imm);
        }
    }

    let mut index = ir.imm32(0u32);
    if inst_info.index_enable() != 0 {
        let vgpr_index = get_buffer_index_arg(inst);
        index = i_add(ir, &index, &vgpr_index);
    }
    if buffer.add_tid_enable() {
        assert_eq!(
            info.sw_stage(),
            crate::gpu::shader_recompiler::runtime_info::SwStage::Compute,
            "Thread ID buffer addressing is not supported outside of compute."
        );
        let thread_id = ir.lane_id();
        index = i_add(ir, &index, &thread_id);
    }

    let mut offset = ir.imm32(inst_offset);
    offset = i_add(ir, &offset, &soffset);
    if inst_info.voffset_enable() != 0 {
        let voffset = get_buffer_voffset_arg(inst);
        offset = i_add(ir, &offset, &voffset);
    }
    let const_stride = ir.imm32(stride);
    let mut buffer_offset;
    if buffer.swizzle_enable() {
        let const_index_stride = ir.imm32(buffer.get_index_stride());
        let const_element_size = ir.imm32(buffer.get_element_size());
        let index_msb = ir.i_div(&index, &const_index_stride, false);
        let index_lsb = ir.i_mod(&index, &const_index_stride, false);
        let offset_msb = ir.i_div(&offset, &const_element_size, false);
        let offset_lsb = ir.i_mod(&offset, &const_element_size, false);
        let index_part = i_mul(ir, &index_msb, &const_stride);
        let offset_part = i_mul(ir, &offset_msb, &const_element_size);
        let sum = i_add(ir, &index_part, &offset_part);
        let msb = i_mul(ir, &sum, &const_index_stride);
        let index_lsb_part = i_mul(ir, &index_lsb, &const_element_size);
        let lsb = i_add(ir, &index_lsb_part, &offset_lsb);
        buffer_offset = i_add(ir, &msb, &lsb);
    } else {
        let scaled = i_mul(ir, &index, &const_stride);
        buffer_offset = i_add(ir, &scaled, &offset);
    }
    if shift != 0 {
        buffer_offset = shift_right(ir, &buffer_offset, shift);
    }
    buffer_offset
}

/// `PatchBufferArgs`.
fn patch_buffer_args(inst: &InstRef, info: &Info, descriptors: &Descriptors) {
    let handle = inst.borrow().arg(0);
    let buffer_res = &descriptors.buffer_resources[handle.u32() as usize];
    let buffer = buffer_res.get_sharp(info);

    if inst.borrow().get_opcode() == Opcode::ReadConstBuffer {
        return;
    }

    let block = inst
        .borrow()
        .parent()
        .expect("the instruction has a parent");
    let index = position(&block, inst).expect("the instruction is in its block");
    let mut ir = IrEmitter::at(&block, index);
    let address = calculate_buffer_address(&mut ir, inst, info, &buffer, buffer.stride() as u32);
    set_arg(inst, load_buffer_args::ADDRESS, address.into_value());
}

/// `FixCubeCoords`.
fn fix_cube_coords(ir: &mut IrEmitter, image: &Image, x: &Value, y: &Value, face: &Value) -> Value {
    if !image.is_cube() {
        return ir.composite_construct3(x.clone(), y.clone(), face.clone());
    }
    let one = ir.imm32_float(1.0);
    let fixed_x = narrow(ir.fp_sub(&widen(&F32::new(x.clone())), &widen(&one)));
    let one = ir.imm32_float(1.0);
    let fixed_y = narrow(ir.fp_sub(&widen(&F32::new(y.clone())), &widen(&one)));
    let eight = ir.imm32_float(8.0);
    let quotient = narrow(ir.fp_div(&widen(&F32::new(face.clone())), &widen(&eight)));
    let quotient = narrow(ir.fp_floor(&widen(&quotient)));
    let minus_two = ir.imm32_float(-2.0);
    let fixed_face = narrow(ir.fp_fma(
        &widen(&quotient),
        &widen(&minus_two),
        &widen(&F32::new(face.clone())),
    ));
    ir.composite_construct3(fixed_x, fixed_y, fixed_face)
}

/// `PatchImageSampleArgs`.
fn patch_image_sample_args(
    inst: &InstRef,
    info: &Info,
    image_res: &ImageResource,
    image: &Image,
    descriptors: &Descriptors,
) {
    let handle = inst.borrow().arg(0);
    let sampler_res = &descriptors.sampler_resources[((handle.u32() >> 16) & 0xFFFF) as usize];
    let sampler = sampler_res.get_sharp(info);

    let block = inst
        .borrow()
        .parent()
        .expect("the instruction has a parent");
    let index = position(&block, inst).expect("the instruction is in its block");
    let mut ir = IrEmitter::at(&block, index);
    let inst_info = inst.borrow().texture_info();
    let view_type = image.get_view_type(image_res.is_array);

    let body_1 = inst.borrow().arg(2).inst();
    let body_2 = inst.borrow().arg(3).inst();
    let body_3 = inst.borrow().arg(4).inst();
    let body_4 = F32::new(inst.borrow().arg(5));
    let get_addr_reg = |index: u32| -> F32 {
        match index {
            0..=3 => F32::new(body_1.borrow().arg(index as usize)),
            4..=7 => F32::new(body_2.borrow().arg((index - 4) as usize)),
            8..=11 => F32::new(body_3.borrow().arg((index - 8) as usize)),
            12 => body_4.clone(),
            _ => panic!("Unreachable"),
        }
    };
    let mut addr_reg = 0u32;

    let offset = if inst_info.has_offset() == 0 {
        Value::empty()
    } else {
        let arg = get_addr_reg(addr_reg).into_value();
        addr_reg += 1;
        let arg = if let Some(inst) = arg.try_inst() {
            assert_eq!(
                inst.borrow().get_opcode(),
                Opcode::BitCastF32U32,
                "the offset is a bit cast"
            );
            inst.borrow().arg(0)
        } else {
            arg
        };
        let read = |ir: &mut IrEmitter, off: u32| -> U32 {
            if arg.is_immediate() {
                let imm = if arg.ty() == Type::F32 {
                    arg.f32().to_bits()
                } else {
                    arg.u32()
                };
                let comp = (imm >> off) & 0x3F;
                ir.imm32((((comp << 26) as i32) >> 26) as u32)
            } else {
                let base = U32::new(arg.clone());
                let off = ir.imm32(off);
                let six = ir.imm32(6u32);
                ir.bit_field_extract(&base, &off, &six, true)
            }
        };
        match view_type {
            ImageType::Color1D | ImageType::Color1DArray => read(&mut ir, 0).into_value(),
            ImageType::Color2D | ImageType::Color2DMsaa | ImageType::Color2DArray => {
                let x = read(&mut ir, 0);
                let y = read(&mut ir, 8);
                ir.composite_construct(x, y)
            }
            ImageType::Color3D => {
                let x = read(&mut ir, 0);
                let y = read(&mut ir, 8);
                let z = read(&mut ir, 16);
                ir.composite_construct3(x, y, z)
            }
            _ => panic!("Unreachable"),
        }
    };

    let bias = if inst_info.has_bias() != 0 {
        let value = get_addr_reg(addr_reg);
        addr_reg += 1;
        value
    } else {
        F32::empty()
    };
    let dref = if inst_info.is_depth() != 0 {
        let value = get_addr_reg(addr_reg);
        addr_reg += 1;
        value
    } else {
        F32::empty()
    };

    let (derivatives_dx, derivatives_dy) = if inst_info.has_derivatives() == 0 {
        (Value::empty(), Value::empty())
    } else {
        match view_type {
            ImageType::Color1D | ImageType::Color1DArray => {
                addr_reg += 2;
                (
                    get_addr_reg(addr_reg - 2).into_value(),
                    get_addr_reg(addr_reg - 1).into_value(),
                )
            }
            ImageType::Color2D | ImageType::Color2DMsaa | ImageType::Color2DArray => {
                addr_reg += 4;
                let dx =
                    ir.composite_construct(get_addr_reg(addr_reg - 4), get_addr_reg(addr_reg - 3));
                let dy =
                    ir.composite_construct(get_addr_reg(addr_reg - 2), get_addr_reg(addr_reg - 1));
                (dx, dy)
            }
            ImageType::Color3D => {
                addr_reg += 6;
                let dx = ir.composite_construct3(
                    get_addr_reg(addr_reg - 6),
                    get_addr_reg(addr_reg - 5),
                    get_addr_reg(addr_reg - 4),
                );
                let dy = ir.composite_construct3(
                    get_addr_reg(addr_reg - 3),
                    get_addr_reg(addr_reg - 2),
                    get_addr_reg(addr_reg - 1),
                );
                (dx, dy)
            }
            _ => panic!("Unreachable"),
        }
    };

    let is_msaa = matches!(
        view_type,
        ImageType::Color2DMsaa | ImageType::Color2DMsaaArray
    );
    let unnormalized = sampler.force_unnormalized() != 0 || inst_info.is_unnormalized() != 0;
    let needs_dimensions = (!is_msaa && unnormalized) || (is_msaa && !unnormalized);
    let dimensions = if needs_dimensions {
        let zero = ir.imm32(0u32);
        let skip_mips = ir.imm1(false);
        ir.image_query_dimension(&handle, &zero, &skip_mips, inst_info)
    } else {
        Value::empty()
    };
    let get_coord = |ir: &mut IrEmitter, coord_idx: u32, dim_idx: u32| -> Value {
        let coord = get_addr_reg(coord_idx);
        if is_msaa {
            if unnormalized {
                return ir.convert_f_to_u(32, &widen(&coord)).into_value();
            }
            let dim_raw = ir.composite_extract(&dimensions, dim_idx as usize);
            let dim = U32::new(dim_raw);
            let dim = narrow(ir.convert_u_to_f(32, 32, &dim.into_value()));
            let scaled = narrow(ir.fp_mul(&widen(&coord), &widen(&dim)));
            return ir.convert_f_to_u(32, &widen(&scaled)).into_value();
        }
        if unnormalized {
            let dim_raw = ir.composite_extract(&dimensions, dim_idx as usize);
            let dim = U32::new(dim_raw);
            let dim = narrow(ir.convert_u_to_f(32, 32, &dim.into_value()));
            return narrow(ir.fp_div(&widen(&coord), &widen(&dim))).into_value();
        }
        coord.into_value()
    };

    let coords = match view_type {
        ImageType::Color1D => {
            addr_reg += 1;
            get_coord(&mut ir, addr_reg - 1, 0)
        }
        ImageType::Color1DArray | ImageType::Color2D | ImageType::Color2DMsaa => {
            addr_reg += 2;
            let x = get_coord(&mut ir, addr_reg - 2, 0);
            let y = get_coord(&mut ir, addr_reg - 1, 1);
            ir.composite_construct(x, y)
        }
        ImageType::Color2DArray => {
            addr_reg += 3;
            let x = get_coord(&mut ir, addr_reg - 3, 0);
            let y = get_coord(&mut ir, addr_reg - 2, 1);
            let face = get_addr_reg(addr_reg - 1).into_value();
            fix_cube_coords(&mut ir, image, &x, &y, &face)
        }
        ImageType::Color3D => {
            addr_reg += 3;
            let x = get_coord(&mut ir, addr_reg - 3, 0);
            let y = get_coord(&mut ir, addr_reg - 2, 1);
            let z = get_coord(&mut ir, addr_reg - 1, 2);
            ir.composite_construct3(x, y, z)
        }
        _ => panic!("Unreachable"),
    };

    assert!(
        inst_info.has_lod() == 0 || inst_info.has_lod_clamp() == 0,
        "both lod and lod clamp are set"
    );
    let explicit_lod = inst_info.has_lod() != 0 || inst_info.force_level0() != 0;
    let lod = if inst_info.has_lod() != 0 {
        let value = get_addr_reg(addr_reg);
        addr_reg += 1;
        value
    } else if inst_info.force_level0() != 0 {
        ir.imm32_float(0.0)
    } else {
        F32::empty()
    };
    let lod_clamp = if inst_info.has_lod_clamp() != 0 {
        get_addr_reg(addr_reg)
    } else {
        F32::empty()
    };

    let texel = if is_msaa {
        let zero = ir.imm32(0u32);
        ir.image_read(&handle, &coords, &U32::empty(), &zero, inst_info)
    } else if inst_info.is_gather() != 0 {
        if inst_info.is_depth() != 0 {
            ir.image_gather_dref(&handle, &coords, &offset, &dref, inst_info)
        } else {
            ir.image_gather(&handle, &coords, &offset, inst_info)
        }
    } else if inst_info.has_derivatives() != 0 {
        ir.image_gradient(
            &handle,
            &coords,
            &derivatives_dx,
            &derivatives_dy,
            &offset,
            &lod_clamp,
            inst_info,
        )
    } else if inst_info.is_depth() != 0 {
        if explicit_lod {
            ir.image_sample_dref_explicit_lod(&handle, &coords, &dref, &lod, &offset, inst_info)
        } else {
            ir.image_sample_dref_implicit_lod(&handle, &coords, &dref, &bias, &offset, inst_info)
        }
    } else if explicit_lod {
        ir.image_sample_explicit_lod(&handle, &coords, &lod, &offset, inst_info)
    } else {
        ir.image_sample_implicit_lod(&handle, &coords, &bias, &offset, inst_info)
    };

    let mut converted =
        apply_read_number_conversion_vec4(&mut ir, &texel, image.get_number_conversion());
    if sampler.force_degamma() != 0 && image.get_number_fmt() != NumberFormat::Srgb {
        converted = apply_force_degamma(&mut ir, &texel);
    }
    replace_uses_with(inst, converted);
}

/// `PatchImageArgs`.
fn patch_image_args(inst: &InstRef, info: &Info, descriptors: &Descriptors) {
    if inst.borrow().get_opcode() == Opcode::ImageQueryDimensions {
        return;
    }

    let image_handle = inst.borrow().arg(0);
    let binding_index = (image_handle.u32() & 0xFFFF) as usize;
    let image_res = &descriptors.image_resources[binding_index];
    let image = image_res.get_sharp(info);

    if inst.borrow().get_opcode() == Opcode::ImageSampleRaw {
        patch_image_sample_args(inst, info, image_res, &image, descriptors);
        return;
    }

    let block = inst
        .borrow()
        .parent()
        .expect("the instruction has a parent");
    let index = position(&block, inst).expect("the instruction is in its block");
    let mut ir = IrEmitter::at(&block, index);
    let mut inst_info = inst.borrow().texture_info();
    let view_type = image.get_view_type(image_res.is_array);

    let body = inst.borrow().arg(1).inst();
    let (coords, arg) = match view_type {
        ImageType::Color1D => (body.borrow().arg(0), body.borrow().arg(1)),
        ImageType::Color1DArray | ImageType::Color2D => (
            ir.composite_construct(body.borrow().arg(0), body.borrow().arg(1)),
            body.borrow().arg(2),
        ),
        ImageType::Color2DMsaa => {
            let mut skip_lod = false;
            if inst_info.has_lod() != 0 {
                let mipid = body.borrow().arg(2);
                if mipid.is_immediate() && mipid.u32() == 0 {
                    inst_info.set_has_lod(0);
                    skip_lod = true;
                } else {
                    panic!("Encountered a _mip instruction with MSAA image, and mipid is non-zero");
                }
            }
            (
                ir.composite_construct(body.borrow().arg(0), body.borrow().arg(1)),
                body.borrow().arg(if skip_lod { 3 } else { 2 }),
            )
        }
        ImageType::Color2DArray | ImageType::Color2DMsaaArray | ImageType::Color3D => (
            ir.composite_construct3(
                body.borrow().arg(0),
                body.borrow().arg(1),
                body.borrow().arg(2),
            ),
            body.borrow().arg(3),
        ),
        _ => panic!("Unknown image type {}", view_type as u64),
    };

    let has_ms = matches!(
        view_type,
        ImageType::Color2DMsaa | ImageType::Color2DMsaaArray
    );
    assert!(
        inst_info.has_lod() == 0 || !has_ms,
        "lod and multisampling are exclusive"
    );
    let lod = if inst_info.has_lod() != 0
        && image_res.mip_fallback_mode != MipStorageFallbackMode::ConstantIndex
    {
        U32::new(arg.clone())
    } else {
        U32::empty()
    };
    let ms = if has_ms { U32::new(arg) } else { U32::empty() };

    let is_storage = image_res.is_written;
    if inst.borrow().get_opcode() == Opcode::ImageRead {
        let mut texel = ir.image_read(&image_handle, &coords, &lod, &ms, inst_info);
        if is_storage {
            texel = apply_swizzle(&mut ir, &texel, image.dst_select());
        }
        let converted =
            apply_read_number_conversion_vec4(&mut ir, &texel, image.get_number_conversion());
        replace_uses_with(inst, converted);
    } else {
        set_arg(inst, 1, coords);
        if inst.borrow().get_opcode() == Opcode::ImageWrite {
            set_arg(inst, 2, lod.into_value());
            set_arg(inst, 3, ms.into_value());

            let mut texel = inst.borrow().arg(4);
            if is_storage {
                texel = apply_swizzle(&mut ir, &texel, image.dst_select().inverse());
            }
            let converted =
                apply_write_number_conversion_vec4(&mut ir, &texel, image.get_number_conversion());
            set_arg(inst, 4, converted);
        }
    }
}

/// `ResourcePatchingPass`.
pub fn resource_patching_pass(info: &mut Info, resources: &[ResourceDiscovery], profile: &Profile) {
    let mut descriptors = Descriptors::take(info);
    for usage in resources {
        let inst = usage.user.clone();
        if is_buffer_instruction(&inst) {
            patch_buffer_sharp(usage, &mut descriptors, profile);
        } else if is_image_instruction(&inst) {
            patch_image_sharp(usage, info, &mut descriptors, profile);
        }
    }

    for usage in resources {
        let inst = usage.user.clone();
        if is_buffer_instruction(&inst) {
            patch_buffer_args(&inst, info, &descriptors);
        } else if is_image_instruction(&inst) {
            patch_image_args(&inst, info, &descriptors);
        } else if is_data_ring_instruction(&inst) {
            patch_global_data_share_access(&inst, &mut descriptors);
        }
    }
    descriptors.restore(info);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block};
    use crate::gpu::shader_recompiler::ir::microinstruction::Inst;
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage};
    use std::cell::RefCell;

    fn info() -> Info {
        Info::new(
            HwStage::Vertex,
            SwStage::Vertex,
            ShaderParams::new([0; 16], &[], 0),
        )
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

    fn sharp(dwords: [u32; 8]) -> SharpReference {
        SharpReference {
            dwords: std::array::from_fn(|index| Value::from_u32(dwords[index])),
            num_dwords: 8,
            ..Default::default()
        }
    }

    fn image_sharp(width: u32, height: u32) -> SharpReference {
        let word1 = (9u64 << 60) | u64::from(width) | (u64::from(height) << 14);
        sharp([0, 0, word1 as u32, (word1 >> 32) as u32, 0, 0, 0, 0])
    }

    fn buffer_instruction(block: &BlockRef, op: Opcode) -> (InstRef, InstRef) {
        let address = append(
            block,
            Opcode::CompositeConstructU32x3,
            &[
                Value::from_u32(0),
                Value::from_u32(0),
                Value::from_u32(0x10),
            ],
        );
        let inst = append(
            block,
            op,
            &[Value::from_u32(0), Value::from_inst(address.clone())],
        );
        (address, inst)
    }

    #[test]
    fn a_buffer_sharp_becomes_a_binding_and_an_address() {
        let block = Block::new_ref();
        let (_address, load) = buffer_instruction(&block, Opcode::LoadBufferU32);
        let mut info = info();
        let resources = vec![ResourceDiscovery {
            user: load.clone(),
            sharps: [sharp([0; 8]), SharpReference::default()],
        }];
        resource_patching_pass(&mut info, &resources, &Profile::default());

        assert_eq!(load.borrow().arg(0).u32(), 0);
        let buffers = &info.persistent.buffers;
        assert_eq!(buffers.len(), 1);
        assert_eq!(buffers[0].buffer_type, BufferType::Guest);
        assert_eq!(buffers[0].used_types, Type::U32);
        assert!(!buffers[0].is_written);

        assert_eq!(
            opcodes(&block),
            vec![
                Opcode::CompositeConstructU32x3,
                Opcode::IAdd32,
                Opcode::IMul32,
                Opcode::IAdd32,
                Opcode::ShiftRightLogical32,
                Opcode::LoadBufferU32,
            ]
        );
        let computed = block.borrow().instructions[4].clone();
        assert!(Rc::ptr_eq(&load.borrow().arg(1).inst(), &computed));
    }

    #[test]
    fn identical_buffers_share_one_binding() {
        let block = Block::new_ref();
        let (_, first) = buffer_instruction(&block, Opcode::LoadBufferU32);
        let address = append(
            &block,
            Opcode::CompositeConstructU32x3,
            &[
                Value::from_u32(0),
                Value::from_u32(0),
                Value::from_u32(0x10),
            ],
        );
        let second = append(
            &block,
            Opcode::LoadBufferU32,
            &[Value::from_u32(0), Value::from_inst(address)],
        );
        let mut info = info();
        let resources = vec![
            ResourceDiscovery {
                user: first.clone(),
                sharps: [sharp([0; 8]), SharpReference::default()],
            },
            ResourceDiscovery {
                user: second.clone(),
                sharps: [sharp([0; 8]), SharpReference::default()],
            },
        ];
        resource_patching_pass(&mut info, &resources, &Profile::default());

        assert_eq!(info.persistent.buffers.len(), 1);
        assert_eq!(first.borrow().arg(0).u32(), 0);
        assert_eq!(second.borrow().arg(0).u32(), 0);
    }

    #[test]
    fn a_data_ring_append_becomes_a_gds_atomic() {
        let block = Block::new_ref();
        let append = append(
            &block,
            Opcode::DataAppend,
            &[Value::from_u32(0), Value::from_u1(true)],
        );
        let mut info = info();
        resource_patching_pass(
            &mut info,
            &[ResourceDiscovery::new(append.clone())],
            &Profile::default(),
        );

        assert_eq!(append.borrow().get_opcode(), Opcode::Void);
        assert_eq!(
            opcodes(&block),
            vec![Opcode::BufferAtomicIAdd32, Opcode::Void]
        );
        assert_eq!(info.persistent.buffers.len(), 1);
        assert_eq!(
            info.persistent.buffers[0].buffer_type,
            BufferType::GdsBuffer
        );
        assert_eq!(info.persistent.buffers[0].used_types, Type::U32);
        assert!(info.persistent.buffers[0].is_written);
    }

    #[test]
    fn a_gds_shared_load_becomes_a_buffer_load() {
        let block = Block::new_ref();
        let load = append(&block, Opcode::LoadSharedU32, &[Value::from_u32(8)]);
        load.borrow_mut().set_flags(1);
        let mut info = info();
        resource_patching_pass(
            &mut info,
            &[ResourceDiscovery::new(load.clone())],
            &Profile::default(),
        );

        assert_eq!(load.borrow().get_opcode(), Opcode::Void);
        assert_eq!(
            opcodes(&block),
            vec![
                Opcode::ShiftRightLogical32,
                Opcode::ShiftRightLogical32,
                Opcode::ShiftRightLogical32,
                Opcode::LoadBufferU32,
                Opcode::Void,
            ]
        );
        assert_eq!(info.persistent.buffers[0].used_types, Type::U32);
    }

    #[test]
    fn a_gds_shared_write_adds_its_width_to_the_buffer_types() {
        let block = Block::new_ref();
        let store = append(
            &block,
            Opcode::WriteSharedU64,
            &[Value::from_u32(8), Value::from_u64(0x1122)],
        );
        store.borrow_mut().set_flags(1);
        let mut info = info();
        resource_patching_pass(
            &mut info,
            &[ResourceDiscovery::new(store.clone())],
            &Profile::default(),
        );

        assert_eq!(store.borrow().get_opcode(), Opcode::Void);
        assert!(info.persistent.buffers[0].used_types.contains(Type::U64));
    }

    #[test]
    fn an_image_sharp_gets_a_binding() {
        let block = Block::new_ref();
        let query = append(
            &block,
            Opcode::ImageQueryDimensions,
            &[
                Value::from_u32(0),
                Value::from_u32(0),
                Value::from_u1(false),
            ],
        );
        let mut info = info();
        let resources = vec![ResourceDiscovery {
            user: query.clone(),
            sharps: [image_sharp(64, 32), SharpReference::default()],
        }];
        resource_patching_pass(&mut info, &resources, &Profile::default());

        assert_eq!(query.borrow().arg(0).u32(), 0);
        assert_eq!(query.borrow().get_opcode(), Opcode::ImageQueryDimensions);
        assert_eq!(info.persistent.images.len(), 1);
        assert!(!info.persistent.images[0].is_written);
        assert!(info.persistent.images[0].needs_native);
    }

    #[test]
    fn an_image_read_rebuilds_its_coordinates() {
        let block = Block::new_ref();
        let body = append(
            &block,
            Opcode::CompositeConstructU32x3,
            &[Value::from_u32(10), Value::from_u32(20), Value::from_u32(0)],
        );
        let read = append(
            &block,
            Opcode::ImageRead,
            &[
                Value::from_u32(0),
                Value::from_inst(body),
                Value::from_u32(0),
                Value::from_u32(0),
            ],
        );
        let mut info = info();
        let resources = vec![ResourceDiscovery {
            user: read.clone(),
            sharps: [image_sharp(64, 32), SharpReference::default()],
        }];
        resource_patching_pass(&mut info, &resources, &Profile::default());

        assert_eq!(read.borrow().get_opcode(), Opcode::Void);
        let instructions = block.borrow().instructions.clone();
        assert_eq!(
            instructions
                .iter()
                .map(|inst| inst.borrow().get_opcode())
                .collect::<Vec<_>>(),
            vec![
                Opcode::CompositeConstructU32x3,
                Opcode::CompositeConstructU32x2,
                Opcode::ImageRead,
                Opcode::Void,
            ]
        );
        let rebuilt = &instructions[2];
        let coords = rebuilt.borrow().arg(1).inst();
        assert_eq!(
            coords.borrow().get_opcode(),
            Opcode::CompositeConstructU32x2
        );
        assert_eq!(coords.borrow().arg(0).u32(), 10);
        assert_eq!(coords.borrow().arg(1).u32(), 20);
        assert_eq!(info.persistent.images.len(), 1);
    }

    #[test]
    fn an_fmask_dimension_query_is_replaced_and_tracked() {
        let block = Block::new_ref();
        let query = append(
            &block,
            Opcode::ImageQueryDimensions,
            &[
                Value::from_u32(0),
                Value::from_u32(0),
                Value::from_u1(false),
            ],
        );
        let mut info = info();
        let word0 = 45u64 << 52;
        let word1 = (9u64 << 60) | 64 | (32 << 14);
        let mut image = SharpReference {
            dwords: std::array::from_fn(|index| {
                Value::from_u32(
                    [
                        word0 as u32,
                        (word0 >> 32) as u32,
                        word1 as u32,
                        (word1 >> 32) as u32,
                        0,
                        0,
                        0,
                        0,
                    ][index],
                )
            }),
            num_dwords: 8,
            ..Default::default()
        };
        let location_source = Rc::new(RefCell::new(Inst::new(Opcode::Barrier, 3)));
        image.dwords[0] = Value::from_inst(location_source);
        info.flattened_ud_buf = vec![0; 8];
        info.flattened_ud_buf[3] = word0 as u32;

        resource_patching_pass(
            &mut info,
            &[ResourceDiscovery {
                user: query.clone(),
                sharps: [image, SharpReference::default()],
            }],
            &Profile::default(),
        );

        assert_eq!(query.borrow().get_opcode(), Opcode::Void);
        assert_eq!(info.persistent.fmasks.len(), 1);
        assert_eq!(info.persistent.fmasks[0].sharp_idx, 3);
        let dims = block
            .borrow()
            .instructions
            .first()
            .cloned()
            .expect("the dims");
        assert_eq!(dims.borrow().get_opcode(), Opcode::CompositeConstructU32x4);
        assert_eq!(dims.borrow().arg(0).u32(), 64);
        assert_eq!(dims.borrow().arg(1).u32(), 32);
        assert_eq!(dims.borrow().arg(2).u32(), 1);
        assert_eq!(dims.borrow().arg(3).u32(), 1);
    }
}
