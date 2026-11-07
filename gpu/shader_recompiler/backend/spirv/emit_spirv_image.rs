//! Image emitters, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/emit_spirv_image.cpp`.

use super::builder::{Id, ValidId};
use super::emit_context::EmitContext;
use super::spv;
use crate::gpu::amdgpu::resource::ImageType;
use crate::gpu::shader_recompiler::ir::microinstruction::InstRef;
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::reg::TextureInstInfo;
use crate::gpu::shader_recompiler::ir::value::Value;
use crate::gpu::shader_recompiler::resource::MipStorageFallbackMode;

/// `ImageOperands`: the mask and its trailing operands.
struct ImageOperands {
    mask: u32,
    operands: Vec<Id>,
}

impl ImageOperands {
    fn new() -> Self {
        ImageOperands {
            mask: 0,
            operands: Vec::new(),
        }
    }

    fn add(&mut self, new_mask: u32, value: Id) {
        if !ValidId(value) {
            return;
        }
        self.mask |= new_mask;
        self.operands.push(value);
    }

    fn add_pair(&mut self, new_mask: u32, value1: Id, value2: Id) {
        self.mask |= new_mask;
        self.operands.push(value1);
        self.operands.push(value2);
    }

    fn add_offset(&mut self, ctx: &mut EmitContext, offset: &Value, can_use_runtime_offsets: bool) {
        if offset.is_empty() {
            return;
        }
        if offset.is_immediate() {
            let operand = ctx.ConstS32(offset.u32() as i32);
            self.add(spv::ImageOperandsMask::ConstOffset, operand);
            return;
        }
        let inst = offset.inst();
        if inst.borrow().are_all_args_immediates() {
            match inst.borrow().get_opcode() {
                Opcode::CompositeConstructU32x2 => {
                    let values = [
                        inst.borrow().arg(0).u32() as i32,
                        inst.borrow().arg(1).u32() as i32,
                    ];
                    let operand = ctx.ConstS32Slice(&values);
                    self.add(spv::ImageOperandsMask::ConstOffset, operand);
                    return;
                }
                Opcode::CompositeConstructU32x3 => {
                    let values = [
                        inst.borrow().arg(0).u32() as i32,
                        inst.borrow().arg(1).u32() as i32,
                        inst.borrow().arg(2).u32() as i32,
                    ];
                    let operand = ctx.ConstS32Slice(&values);
                    self.add(spv::ImageOperandsMask::ConstOffset, operand);
                    return;
                }
                _ => {}
            }
        }
        if can_use_runtime_offsets {
            let operand = ctx.Def(offset);
            self.add(spv::ImageOperandsMask::Offset, operand);
        }
    }

    fn add_derivatives(&mut self, ctx: &mut EmitContext, dx: Id, dy: Id) {
        let _ = ctx;
        if !ValidId(dx) || !ValidId(dy) {
            return;
        }
        self.add_pair(spv::ImageOperandsMask::Grad, dx, dy);
    }
}

pub fn emit_image_handle(_ctx: &mut EmitContext, _a: Id, _b: Id) -> Id {
    unreachable!("Unreachable instruction")
}

pub fn emit_image_sample_raw(
    _ctx: &mut EmitContext,
    _inst: &InstRef,
    _handle: u32,
    _address1: Id,
    _address2: Id,
    _address3: Id,
    _address4: Id,
) -> Id {
    unreachable!("Unreachable instruction")
}

fn bind_texture(
    ctx: &mut EmitContext,
    handle: u32,
) -> (super::emit_context::TextureDefinition, Id, Id) {
    let types = ctx.type_ids();
    let texture = ctx.images[(handle & 0xFFFF) as usize];
    let image = ctx.OpLoad(texture.image_type, texture.id, &[]);
    let sampler_id = ctx.samplers[(handle >> 16) as usize];
    let sampler = ctx.OpLoad(types.sampler_type, sampler_id, &[]);
    let sampled_image = ctx.OpSampledImage(texture.sampled_type, image, sampler);
    (texture, image, sampled_image)
}

pub fn emit_image_sample_implicit_lod(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    bias: Id,
    offset: &Value,
) -> Id {
    let types = ctx.type_ids();
    let (texture, _image, sampled_image) = bind_texture(ctx, handle);
    let result_type = texture.data_types.get(4);
    let mut operands = ImageOperands::new();
    operands.add(spv::ImageOperandsMask::Bias, bias);
    operands.add_offset(ctx, offset, false);
    let sample = ctx.OpImageSampleImplicitLod(
        result_type,
        sampled_image,
        coords,
        operands.mask,
        &operands.operands,
    );
    if texture.is_integer {
        ctx.OpBitcast(types.F32[4], sample)
    } else {
        sample
    }
}

pub fn emit_image_sample_explicit_lod(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    lod: Id,
    offset: &Value,
) -> Id {
    let types = ctx.type_ids();
    let (texture, _image, sampled_image) = bind_texture(ctx, handle);
    let result_type = texture.data_types.get(4);
    let mut operands = ImageOperands::new();
    operands.add(spv::ImageOperandsMask::Lod, lod);
    operands.add_offset(ctx, offset, false);
    let sample = ctx.OpImageSampleExplicitLod(
        result_type,
        sampled_image,
        coords,
        operands.mask,
        &operands.operands,
    );
    if texture.is_integer {
        ctx.OpBitcast(types.F32[4], sample)
    } else {
        sample
    }
}

pub fn emit_image_sample_dref_implicit_lod(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    dref: Id,
    bias: Id,
    offset: &Value,
) -> Id {
    let types = ctx.type_ids();
    let (texture, _image, sampled_image) = bind_texture(ctx, handle);
    let result_type = texture.data_types.get(1);
    let mut operands = ImageOperands::new();
    operands.add(spv::ImageOperandsMask::Bias, bias);
    operands.add_offset(ctx, offset, false);
    let sample = ctx.OpImageSampleDrefImplicitLod(
        result_type,
        sampled_image,
        coords,
        dref,
        operands.mask,
        &operands.operands,
    );
    let sample_typed = if texture.is_integer {
        ctx.OpBitcast(types.F32[1], sample)
    } else {
        sample
    };
    let zero = types.f32_zero_value;
    ctx.OpCompositeConstruct(types.F32[4], &[sample_typed, zero, zero, zero])
}

#[allow(clippy::too_many_arguments)]
pub fn emit_image_sample_dref_explicit_lod(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    dref: Id,
    lod: Id,
    offset: &Value,
) -> Id {
    let types = ctx.type_ids();
    let (texture, _image, sampled_image) = bind_texture(ctx, handle);
    let result_type = texture.data_types.get(1);
    let mut operands = ImageOperands::new();
    operands.add(spv::ImageOperandsMask::Lod, lod);
    operands.add_offset(ctx, offset, false);
    let sample = ctx.OpImageSampleDrefExplicitLod(
        result_type,
        sampled_image,
        coords,
        dref,
        operands.mask,
        &operands.operands,
    );
    let sample_typed = if texture.is_integer {
        ctx.OpBitcast(types.F32[1], sample)
    } else {
        sample
    };
    let zero = types.f32_zero_value;
    ctx.OpCompositeConstruct(types.F32[4], &[sample_typed, zero, zero, zero])
}

pub fn emit_image_gather(
    ctx: &mut EmitContext,
    inst: &InstRef,
    handle: u32,
    coords: Id,
    offset: &Value,
) -> Id {
    let types = ctx.type_ids();
    let (texture, _image, sampled_image) = bind_texture(ctx, handle);
    let result_type = texture.data_types.get(4);
    let comp = TextureInstInfo::from_raw(inst.borrow().flags_u32()).gather_comp();
    let mut operands = ImageOperands::new();
    operands.add_offset(ctx, offset, true);
    let comp = ctx.ConstU32(comp);
    let texels = ctx.OpImageGather(
        result_type,
        sampled_image,
        coords,
        comp,
        operands.mask,
        &operands.operands,
    );
    if texture.is_integer {
        ctx.OpBitcast(types.F32[4], texels)
    } else {
        texels
    }
}

pub fn emit_image_gather_dref(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    offset: &Value,
    dref: Id,
) -> Id {
    let types = ctx.type_ids();
    let (texture, _image, sampled_image) = bind_texture(ctx, handle);
    let result_type = texture.data_types.get(4);
    let mut operands = ImageOperands::new();
    operands.add_offset(ctx, offset, true);
    let texels = ctx.OpImageDrefGather(
        result_type,
        sampled_image,
        coords,
        dref,
        operands.mask,
        &operands.operands,
    );
    if texture.is_integer {
        ctx.OpBitcast(types.F32[4], texels)
    } else {
        texels
    }
}

pub fn emit_image_query_dimensions(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    lod: Id,
    has_mips: bool,
) -> Id {
    let types = ctx.type_ids();
    let texture = ctx.images[(handle & 0xFFFF) as usize];
    let image = ctx.OpLoad(texture.image_type, texture.id, &[]);
    let info = ctx.info.clone();
    let _sharp =
        info.borrow().persistent.images[(handle & 0xFFFF) as usize].get_sharp(&info.borrow());
    let zero = types.u32_zero_value;
    let mips = if has_mips {
        ctx.OpImageQueryLevels(types.U32[1], image)
    } else {
        zero
    };
    let uses_lod = texture.view_type != ImageType::Color2DMsaa && !texture.is_storage;
    let query = |ctx: &mut EmitContext, ty: Id| -> Id {
        if uses_lod {
            ctx.OpImageQuerySizeLod(ty, image, lod)
        } else {
            ctx.OpImageQuerySize(ty, image)
        }
    };
    match texture.view_type {
        ImageType::Color1D => {
            let width = query(ctx, types.U32[1]);
            ctx.OpCompositeConstruct(types.U32[4], &[width, zero, zero, mips])
        }
        ImageType::Color1DArray | ImageType::Color2D | ImageType::Color2DMsaa => {
            let size = query(ctx, types.U32[2]);
            ctx.OpCompositeConstruct(types.U32[4], &[size, zero, mips])
        }
        ImageType::Color2DArray | ImageType::Color3D => {
            let size = query(ctx, types.U32[3]);
            ctx.OpCompositeConstruct(types.U32[4], &[size, mips])
        }
        _ => unreachable!("SPIR-V Instruction"),
    }
}

pub fn emit_image_query_lod(ctx: &mut EmitContext, _inst: &InstRef, handle: u32, coords: Id) -> Id {
    let types = ctx.type_ids();
    let (_texture, _image, sampled_image) = bind_texture(ctx, handle);
    ctx.OpImageQueryLod(types.F32[2], sampled_image, coords)
}

#[allow(clippy::too_many_arguments)]
pub fn emit_image_gradient(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    derivatives_dx: Id,
    derivatives_dy: Id,
    offset: &Value,
    _lod_clamp: &Value,
) -> Id {
    let types = ctx.type_ids();
    let (texture, _image, sampled_image) = bind_texture(ctx, handle);
    let result_type = texture.data_types.get(4);
    let mut operands = ImageOperands::new();
    operands.add_derivatives(ctx, derivatives_dx, derivatives_dy);
    operands.add_offset(ctx, offset, false);
    let sample = ctx.OpImageSampleExplicitLod(
        result_type,
        sampled_image,
        coords,
        operands.mask,
        &operands.operands,
    );
    if texture.is_integer {
        ctx.OpBitcast(types.F32[4], sample)
    } else {
        sample
    }
}

pub fn emit_image_read(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    lod: Id,
    ms: Id,
) -> Id {
    let types = ctx.type_ids();
    let texture = ctx.images[(handle & 0xFFFF) as usize];
    let color_type = texture.data_types.get(4);
    let mut operands = ImageOperands::new();
    let texel;
    if !texture.is_storage {
        let image = ctx.OpLoad(texture.image_type, texture.id, &[]);
        if texture.view_type == ImageType::Color2DMsaa {
            if ValidId(ms) {
                let sample_count = ctx.OpImageQuerySamples(types.U32[1], image);
                let wrapped = ctx.OpUMod(types.U32[1], ms, sample_count);
                operands.add(spv::ImageOperandsMask::Sample, wrapped);
            }
        } else {
            operands.add(spv::ImageOperandsMask::Lod, lod);
            operands.add(spv::ImageOperandsMask::Sample, ms);
        }
        texel = ctx.OpImageFetch(color_type, image, coords, operands.mask, &operands.operands);
    } else {
        let mut image_ptr = texture.id;
        if ctx.profile.supports_image_load_store_lod {
            operands.add(spv::ImageOperandsMask::Lod, lod);
        } else if ValidId(lod) {
            unreachable!("Unsupported ImageRead with Lod");
        }
        let image = ctx.OpLoad(texture.image_type, image_ptr, &[]);
        texel = ctx.OpImageRead(color_type, image, coords, operands.mask, &operands.operands);
        let _ = &mut image_ptr;
    }
    if texture.is_integer {
        ctx.OpBitcast(types.F32[4], texel)
    } else {
        texel
    }
}

pub fn emit_image_write(
    ctx: &mut EmitContext,
    _inst: &InstRef,
    handle: u32,
    coords: Id,
    lod: Id,
    ms: Id,
    color: Id,
) {
    let texture = ctx.images[(handle & 0xFFFF) as usize];
    let mut image_ptr = texture.id;
    let color_type = texture.data_types.get(4);
    let mut operands = ImageOperands::new();
    operands.add(spv::ImageOperandsMask::Sample, ms);
    if ctx.profile.supports_image_load_store_lod {
        operands.add(spv::ImageOperandsMask::Lod, lod);
    } else if ValidId(lod) {
        assert_eq!(
            texture.mip_fallback_mode,
            MipStorageFallbackMode::DynamicIndex
        );
        let single_image_ptr_type =
            ctx.TypePointer(spv::StorageClass::UniformConstant, texture.image_type);
        image_ptr = ctx.OpAccessChain(single_image_ptr_type, image_ptr, &[lod]);
    }
    let image = ctx.OpLoad(texture.image_type, image_ptr, &[]);
    let texel = if texture.is_integer {
        ctx.OpBitcast(color_type, color)
    } else {
        color
    };
    ctx.OpImageWrite(image, coords, texel, operands.mask, &operands.operands);
}

pub fn emit_cube_face_index(ctx: &mut EmitContext, _inst: &InstRef, cube_coords: Id) -> Id {
    if ctx.profile.supports_native_cube_calc {
        let types = ctx.type_ids();
        ctx.OpCubeFaceIndexAMD(types.F32[1], cube_coords)
    } else {
        unreachable!("SPIR-V Instruction")
    }
}
