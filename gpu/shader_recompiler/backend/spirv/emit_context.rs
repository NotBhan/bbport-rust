//! The SPIR-V emit context, ported from
//! `legacy/gpu/shadps4/shader_recompiler/backend/spirv/spirv_emit_context.h` and its `.cpp`.
//!
//! The C++ context inherits the Sirit module builder. The port holds the local `Module` in a
//! field and dereferences to it, so the emitters keep calling `ctx.Op*` and `ctx.Name` as they
//! do in the original. `Info` is shared through `Rc<RefCell<Info>>`, the port's stand-in for
//! the C++'s `Info&`.

#![allow(non_snake_case)]

use crate::gpu::amdgpu::pixel_format::{get_number_class, is_integer, NumberClass, NumberFormat};
use crate::gpu::amdgpu::regs_vertex::PrimitiveType;
use crate::gpu::amdgpu::resource::{Image, ImageType};
use crate::gpu::common::alignment::align_up_pow2;
use crate::gpu::common::div_ceil::div_ceil_u32;
use crate::gpu::shader_recompiler::backend::bindings::Bindings;
use crate::gpu::shader_recompiler::backend::spirv::builder::{Id, Module, ValidId};
use crate::gpu::shader_recompiler::backend::spirv::spv;
use crate::gpu::shader_recompiler::frontend::fetch_shader::{parse_fetch_shader, InstanceIdType};
use crate::gpu::shader_recompiler::info::{Info, Qualifier, ReadConstType};
use crate::gpu::shader_recompiler::ir::attribute::{Attribute, NUM_PARAMS, NUM_RENDER_TARGETS};
use crate::gpu::shader_recompiler::ir::r#type::Type;
use crate::gpu::shader_recompiler::ir::value::Value;
use crate::gpu::shader_recompiler::profile::Profile;
use crate::gpu::shader_recompiler::resource::{
    BufferType, ImageResource, MipStorageFallbackMode, PushData,
};
use crate::gpu::shader_recompiler::runtime_info::{
    HwInfo, HwStage, MotionVectors, RuntimeInfo, SwInfo, SwStage, MAX_EMULATED_CLIP_DISTANCES,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::rc::Rc;
use std::sync::atomic::Ordering;

/// `Shader::Backend::SPIRV::VectorIds`: one id per vector width, indexed 1 through 4.
#[derive(Debug, Clone, Copy, Default)]
pub struct VectorIds {
    pub ids: [Id; 4],
}

impl VectorIds {
    pub fn get(&self, index: u32) -> Id {
        self.ids[index as usize - 1]
    }

    pub fn set(&mut self, index: u32, value: Id) {
        self.ids[index as usize - 1] = value;
    }
}

impl Index<usize> for VectorIds {
    type Output = Id;

    fn index(&self, index: usize) -> &Id {
        &self.ids[index - 1]
    }
}

impl IndexMut<usize> for VectorIds {
    fn index_mut(&mut self, index: usize) -> &mut Id {
        &mut self.ids[index - 1]
    }
}

/// `EmitterContext::TextureDefinition`.
#[derive(Debug, Clone, Copy)]
pub struct TextureDefinition {
    pub data_types: VectorIds,
    pub id: Id,
    pub sampled_type: Id,
    pub image_type: Id,
    pub view_type: ImageType,
    pub is_integer: bool,
    pub is_storage: bool,
    pub mip_fallback_mode: MipStorageFallbackMode,
}

/// `EmitterContext::PointerType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PointerType {
    U8 = 0,
    U16,
    U32,
    F32,
    U64,
    F64,
    NumAlias,
}

/// `EmitterContext::PointerSize`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PointerSize {
    B8 = 0,
    B16,
    B32,
    B64,
    NumClass,
}

/// `EmitterContext::BufferSpv`.
#[derive(Debug, Clone, Copy, Default)]
pub struct BufferSpv {
    pub id: Id,
    pub pointer_type: Id,
}

/// `EmitterContext::BufferDefinition`.
#[derive(Debug, Clone)]
pub struct BufferDefinition {
    pub binding: u32,
    pub buffer_type: BufferType,
    pub offsets: [Id; PointerSize::NumClass as usize],
    pub aliases: [BufferSpv; PointerType::NumAlias as usize],
}

impl BufferDefinition {
    pub fn alias(&self, alias: PointerType) -> BufferSpv {
        self.aliases[alias as usize]
    }

    pub fn set_alias(&mut self, alias: PointerType, value: BufferSpv) {
        self.aliases[alias as usize] = value;
    }

    pub fn offset(&self, size: PointerSize) -> Id {
        self.offsets[size as usize]
    }

    pub fn set_offset(&mut self, size: PointerSize, value: Id) {
        self.offsets[size as usize] = value;
    }
}

impl Default for BufferDefinition {
    fn default() -> Self {
        BufferDefinition {
            binding: 0,
            buffer_type: BufferType::Guest,
            offsets: [0; PointerSize::NumClass as usize],
            aliases: [BufferSpv::default(); PointerType::NumAlias as usize],
        }
    }
}

/// `EmitterContext::SpirvAttribute`: the C++ union of a single id and the per-vertex array.
#[derive(Debug, Clone, Copy, Default)]
pub struct SpirvAttribute {
    pub id_array: [Id; 3],
    pub pointer_type: Id,
    pub component_type: Id,
    pub num_components: u32,
    pub is_integer: bool,
    pub is_loaded: bool,
    pub is_array: bool,
}

impl SpirvAttribute {
    pub fn id(&self) -> Id {
        self.id_array[0]
    }

    pub fn set_id(&mut self, id: Id) {
        self.id_array[0] = id;
    }
}

fn stage_name(stage: HwStage) -> &'static str {
    match stage {
        HwStage::Vertex => "vs",
        HwStage::Local => "ls",
        HwStage::Export => "es",
        HwStage::Hull => "hs",
        HwStage::Geometry => "gs",
        HwStage::Fragment => "fs",
        HwStage::Compute => "cs",
    }
}

fn num_vertices(ty: PrimitiveType) -> u32 {
    match ty {
        PrimitiveType::PointList => 1,
        PrimitiveType::LineList | PrimitiveType::LineStrip => 2,
        PrimitiveType::TriangleList | PrimitiveType::TriangleStrip | PrimitiveType::RectList => 3,
        PrimitiveType::AdjTriangleList => 6,
        PrimitiveType::AdjLineList => 4,
        _ => unreachable!("invalid geometry input primitive {ty:?}"),
    }
}

/// A snapshot of every id the emitters pass as a type or constant, so those reads do not
/// borrow the whole context while a builder method holds it mutably.
#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
pub(crate) struct TypeIds {
    pub(crate) void_id: Id,
    pub(crate) U8: Id,
    pub(crate) S8: Id,
    pub(crate) U16: Id,
    pub(crate) S16: Id,
    pub(crate) U64: Id,
    pub(crate) F16: VectorIds,
    pub(crate) F32: VectorIds,
    pub(crate) F64: VectorIds,
    pub(crate) S32: VectorIds,
    pub(crate) U32: VectorIds,
    pub(crate) U1: VectorIds,
    pub(crate) full_result_i32x2: Id,
    pub(crate) full_result_u32x2: Id,
    pub(crate) frexp_result_f32: Id,
    pub(crate) frexp_result_f64: Id,
    pub(crate) pi_x2: Id,
    pub(crate) true_value: Id,
    pub(crate) false_value: Id,
    pub(crate) u8_one_value: Id,
    pub(crate) u8_zero_value: Id,
    pub(crate) u16_zero_value: Id,
    pub(crate) u32_one_value: Id,
    pub(crate) u32_zero_value: Id,
    pub(crate) f32_zero_value: Id,
    pub(crate) u64_one_value: Id,
    pub(crate) u64_zero_value: Id,
    pub(crate) shared_u16: Id,
    pub(crate) shared_u32: Id,
    pub(crate) shared_u64: Id,
    pub(crate) input_u32: Id,
    pub(crate) input_f32: Id,
    pub(crate) input_s32: Id,
    pub(crate) output_u32: Id,
    pub(crate) output_f32: Id,
    pub(crate) output_s32: Id,
    pub(crate) gl_in: Id,
    pub(crate) output_position: Id,
    pub(crate) output_point_size: Id,
    pub(crate) output_layer: Id,
    pub(crate) output_viewport_index: Id,
    pub(crate) primitive_id: Id,
    pub(crate) vertex_index: Id,
    pub(crate) instance_id: Id,
    pub(crate) push_data_block: Id,
    pub(crate) motion_out_cur: Id,
    pub(crate) motion_out_prev: Id,
    pub(crate) motion_in_cur: Id,
    pub(crate) motion_in_prev: Id,
    pub(crate) motion_frag_out: Id,
    pub(crate) base_vertex: Id,
    pub(crate) base_instance: Id,
    pub(crate) frag_coord: Id,
    pub(crate) front_facing: Id,
    pub(crate) frag_depth: Id,
    pub(crate) stencil_ref: Id,
    pub(crate) sample_mask: Id,
    pub(crate) sample_mask_in: Id,
    pub(crate) helper_invocation: Id,
    pub(crate) sample_index: Id,
    pub(crate) clip_distances: Id,
    pub(crate) cull_distances: Id,
    pub(crate) patch_vertices: Id,
    pub(crate) output_tess_level_outer: Id,
    pub(crate) output_tess_level_inner: Id,
    pub(crate) tess_coord: Id,
    pub(crate) workgroup_id: Id,
    pub(crate) num_workgroups_id: Id,
    pub(crate) workgroup_index_id: Id,
    pub(crate) local_invocation_id: Id,
    pub(crate) local_invocation_index: Id,
    pub(crate) invocation_id: Id,
    pub(crate) subgroup_local_invocation_id: Id,
    pub(crate) subgroup_lt_mask: Id,
    pub(crate) image_u32: Id,
    pub(crate) image_f32: Id,
    pub(crate) shared_memory_u16: Id,
    pub(crate) shared_memory_u32: Id,
    pub(crate) shared_memory_u64: Id,
    pub(crate) shared_memory_u16_type: Id,
    pub(crate) shared_memory_u32_type: Id,
    pub(crate) shared_memory_u64_type: Id,
    pub(crate) bary_coord: Id,
    pub(crate) bary_coord_smooth: Id,
    pub(crate) bary_coord_smooth_centroid: Id,
    pub(crate) bary_coord_smooth_sample: Id,
    pub(crate) bary_coord_pull_model: Id,
    pub(crate) bary_coord_nopersp: Id,
    pub(crate) bary_coord_nopersp_sample: Id,
    pub(crate) physical_pointer_type_u32: Id,
    pub(crate) sampler_type: Id,
    pub(crate) sampler_pointer_type: Id,
    pub(crate) input_attr_array: Id,
    pub(crate) output_attr_array: Id,
    pub(crate) uf11_to_f32: Id,
    pub(crate) f32_to_uf11: Id,
    pub(crate) uf10_to_f32: Id,
    pub(crate) f32_to_uf10: Id,
    pub(crate) get_bda_pointer: Id,
    pub(crate) read_const: Id,
    pub(crate) read_const_dynamic: Id,
}

impl EmitContext<'_> {
    pub(crate) fn type_ids(&self) -> TypeIds {
        TypeIds {
            void_id: self.void_id,
            U8: self.U8,
            S8: self.S8,
            U16: self.U16,
            S16: self.S16,
            U64: self.U64,
            F16: self.F16,
            F32: self.F32,
            F64: self.F64,
            S32: self.S32,
            U32: self.U32,
            U1: self.U1,
            full_result_i32x2: self.full_result_i32x2,
            full_result_u32x2: self.full_result_u32x2,
            frexp_result_f32: self.frexp_result_f32,
            frexp_result_f64: self.frexp_result_f64,
            pi_x2: self.pi_x2,
            true_value: self.true_value,
            false_value: self.false_value,
            u8_one_value: self.u8_one_value,
            u8_zero_value: self.u8_zero_value,
            u16_zero_value: self.u16_zero_value,
            u32_one_value: self.u32_one_value,
            u32_zero_value: self.u32_zero_value,
            f32_zero_value: self.f32_zero_value,
            u64_one_value: self.u64_one_value,
            u64_zero_value: self.u64_zero_value,
            shared_u16: self.shared_u16,
            shared_u32: self.shared_u32,
            shared_u64: self.shared_u64,
            input_u32: self.input_u32,
            input_f32: self.input_f32,
            input_s32: self.input_s32,
            output_u32: self.output_u32,
            output_f32: self.output_f32,
            output_s32: self.output_s32,
            gl_in: self.gl_in,
            output_position: self.output_position,
            output_point_size: self.output_point_size,
            output_layer: self.output_layer,
            output_viewport_index: self.output_viewport_index,
            primitive_id: self.primitive_id,
            vertex_index: self.vertex_index,
            instance_id: self.instance_id,
            push_data_block: self.push_data_block,
            motion_out_cur: self.motion_out_cur,
            motion_out_prev: self.motion_out_prev,
            motion_in_cur: self.motion_in_cur,
            motion_in_prev: self.motion_in_prev,
            motion_frag_out: self.motion_frag_out,
            base_vertex: self.base_vertex,
            base_instance: self.base_instance,
            frag_coord: self.frag_coord,
            front_facing: self.front_facing,
            frag_depth: self.frag_depth,
            stencil_ref: self.stencil_ref,
            sample_mask: self.sample_mask,
            sample_mask_in: self.sample_mask_in,
            helper_invocation: self.helper_invocation,
            sample_index: self.sample_index,
            clip_distances: self.clip_distances,
            cull_distances: self.cull_distances,
            patch_vertices: self.patch_vertices,
            output_tess_level_outer: self.output_tess_level_outer,
            output_tess_level_inner: self.output_tess_level_inner,
            tess_coord: self.tess_coord,
            workgroup_id: self.workgroup_id,
            num_workgroups_id: self.num_workgroups_id,
            workgroup_index_id: self.workgroup_index_id,
            local_invocation_id: self.local_invocation_id,
            local_invocation_index: self.local_invocation_index,
            invocation_id: self.invocation_id,
            subgroup_local_invocation_id: self.subgroup_local_invocation_id,
            subgroup_lt_mask: self.subgroup_lt_mask,
            image_u32: self.image_u32,
            image_f32: self.image_f32,
            shared_memory_u16: self.shared_memory_u16,
            shared_memory_u32: self.shared_memory_u32,
            shared_memory_u64: self.shared_memory_u64,
            shared_memory_u16_type: self.shared_memory_u16_type,
            shared_memory_u32_type: self.shared_memory_u32_type,
            shared_memory_u64_type: self.shared_memory_u64_type,
            bary_coord: self.bary_coord,
            bary_coord_smooth: self.bary_coord_smooth,
            bary_coord_smooth_centroid: self.bary_coord_smooth_centroid,
            bary_coord_smooth_sample: self.bary_coord_smooth_sample,
            bary_coord_pull_model: self.bary_coord_pull_model,
            bary_coord_nopersp: self.bary_coord_nopersp,
            bary_coord_nopersp_sample: self.bary_coord_nopersp_sample,
            physical_pointer_type_u32: self.physical_pointer_type_u32,
            sampler_type: self.sampler_type,
            sampler_pointer_type: self.sampler_pointer_type,
            input_attr_array: self.input_attr_array,
            output_attr_array: self.output_attr_array,
            uf11_to_f32: self.uf11_to_f32,
            f32_to_uf11: self.f32_to_uf11,
            uf10_to_f32: self.uf10_to_f32,
            f32_to_uf10: self.f32_to_uf10,
            get_bda_pointer: self.get_bda_pointer,
            read_const: self.read_const,
            read_const_dynamic: self.read_const_dynamic,
        }
    }
}

/// `Shader::Backend::SPIRV::EmitContext`.
pub struct EmitContext<'a> {
    pub module: Module,
    pub info: Rc<RefCell<Info>>,
    pub runtime_info: RuntimeInfo,
    pub profile: Profile,
    pub hw_stage: HwStage,
    pub sw_stage: SwStage,
    pub last_label: Id,

    pub void_id: Id,
    pub U8: Id,
    pub S8: Id,
    pub U16: Id,
    pub S16: Id,
    pub U64: Id,
    pub F16: VectorIds,
    pub F32: VectorIds,
    pub F64: VectorIds,
    pub S32: VectorIds,
    pub U32: VectorIds,
    pub U1: VectorIds,

    pub full_result_i32x2: Id,
    pub full_result_u32x2: Id,
    pub frexp_result_f32: Id,
    pub frexp_result_f64: Id,
    pub pi_x2: Id,

    pub true_value: Id,
    pub false_value: Id,
    pub u8_one_value: Id,
    pub u8_zero_value: Id,
    pub u16_zero_value: Id,
    pub u32_one_value: Id,
    pub u32_zero_value: Id,
    pub f32_zero_value: Id,
    pub u64_one_value: Id,
    pub u64_zero_value: Id,

    pub shared_u16: Id,
    pub shared_u32: Id,
    pub shared_u64: Id,

    pub input_u32: Id,
    pub input_f32: Id,
    pub input_s32: Id,
    pub output_u32: Id,
    pub output_f32: Id,
    pub output_s32: Id,

    pub gl_in: Id,

    pub interfaces: Vec<Id>,

    pub output_position: Id,
    pub output_point_size: Id,
    pub output_layer: Id,
    pub output_viewport_index: Id,
    pub primitive_id: Id,
    pub vertex_index: Id,
    pub instance_id: Id,
    pub push_data_block: Id,
    pub motion_out_cur: Id,
    pub motion_out_prev: Id,
    pub motion_in_cur: Id,
    pub motion_in_prev: Id,
    pub motion_frag_out: Id,
    pub base_vertex: Id,
    pub base_instance: Id,
    pub frag_coord: Id,
    pub front_facing: Id,
    pub frag_depth: Id,
    pub stencil_ref: Id,
    pub sample_mask: Id,
    pub sample_mask_in: Id,
    pub helper_invocation: Id,
    pub sample_index: Id,
    pub clip_distances: Id,
    pub cull_distances: Id,

    pub patch_vertices: Id,
    pub output_tess_level_outer: Id,
    pub output_tess_level_inner: Id,
    pub tess_coord: Id,
    pub patches: [Id; 30],

    pub workgroup_id: Id,
    pub num_workgroups_id: Id,
    pub workgroup_index_id: Id,
    pub local_invocation_id: Id,
    pub local_invocation_index: Id,
    pub invocation_id: Id,
    pub subgroup_local_invocation_id: Id,
    pub subgroup_lt_mask: Id,
    pub image_u32: Id,
    pub image_f32: Id,

    pub shared_memory_u16: Id,
    pub shared_memory_u32: Id,
    pub shared_memory_u64: Id,

    pub shared_memory_u16_type: Id,
    pub shared_memory_u32_type: Id,
    pub shared_memory_u64_type: Id,

    pub bary_coord: Id,
    pub bary_coord_smooth: Id,
    pub bary_coord_smooth_centroid: Id,
    pub bary_coord_smooth_sample: Id,
    pub bary_coord_pull_model: Id,
    pub bary_coord_nopersp: Id,
    pub bary_coord_nopersp_sample: Id,

    pub binding: &'a mut Bindings,
    pub fetch_code: &'a [u32],
    pub read: &'a dyn Fn(u64) -> u32,
    pub buf_type_ids: Vec<Id>,
    pub buffers: Vec<BufferDefinition>,
    pub images: Vec<TextureDefinition>,
    pub samplers: Vec<Id>,
    pub first_to_last_label_map: HashMap<u32, Id>,

    pub flatbuf_index: usize,
    pub bda_pagetable_index: usize,
    pub fault_buffer_index: usize,
    pub physical_pointer_type_u32: Id,

    pub sampler_type: Id,
    pub sampler_pointer_type: Id,

    pub input_attr_array: Id,
    pub output_attr_array: Id,
    pub input_params: [SpirvAttribute; NUM_PARAMS],
    pub output_params: [SpirvAttribute; NUM_PARAMS],
    pub frag_outputs: [SpirvAttribute; NUM_RENDER_TARGETS],

    pub uf11_to_f32: Id,
    pub f32_to_uf11: Id,
    pub uf10_to_f32: Id,
    pub f32_to_uf10: Id,

    pub get_bda_pointer: Id,
    pub read_const: Id,
    pub read_const_dynamic: Id,
}

impl Deref for EmitContext<'_> {
    type Target = Module;

    fn deref(&self) -> &Module {
        &self.module
    }
}

impl DerefMut for EmitContext<'_> {
    fn deref_mut(&mut self) -> &mut Module {
        &mut self.module
    }
}

impl<'a> EmitContext<'a> {
    /// `EmitContext(profile, runtime_info, info, binding)`.
    pub fn new(
        profile: &Profile,
        runtime_info: &RuntimeInfo,
        info: Rc<RefCell<Info>>,
        binding: &'a mut Bindings,
        fetch_code: &'a [u32],
        read: &'a dyn Fn(u64) -> u32,
    ) -> Self {
        let hw_stage = info.borrow().hw_stage();
        let sw_stage = info.borrow().sw_stage();
        let mut context = EmitContext {
            module: Module::new(profile.supported_spirv),
            info,
            runtime_info: runtime_info.clone(),
            profile: *profile,
            hw_stage,
            sw_stage,
            last_label: 0,
            void_id: 0,
            U8: 0,
            S8: 0,
            U16: 0,
            S16: 0,
            U64: 0,
            F16: VectorIds::default(),
            F32: VectorIds::default(),
            F64: VectorIds::default(),
            S32: VectorIds::default(),
            U32: VectorIds::default(),
            U1: VectorIds::default(),
            full_result_i32x2: 0,
            full_result_u32x2: 0,
            frexp_result_f32: 0,
            frexp_result_f64: 0,
            pi_x2: 0,
            true_value: 0,
            false_value: 0,
            u8_one_value: 0,
            u8_zero_value: 0,
            u16_zero_value: 0,
            u32_one_value: 0,
            u32_zero_value: 0,
            f32_zero_value: 0,
            u64_one_value: 0,
            u64_zero_value: 0,
            shared_u16: 0,
            shared_u32: 0,
            shared_u64: 0,
            input_u32: 0,
            input_f32: 0,
            input_s32: 0,
            output_u32: 0,
            output_f32: 0,
            output_s32: 0,
            gl_in: 0,
            interfaces: Vec::new(),
            output_position: 0,
            output_point_size: 0,
            output_layer: 0,
            output_viewport_index: 0,
            primitive_id: 0,
            vertex_index: 0,
            instance_id: 0,
            push_data_block: 0,
            motion_out_cur: 0,
            motion_out_prev: 0,
            motion_in_cur: 0,
            motion_in_prev: 0,
            motion_frag_out: 0,
            base_vertex: 0,
            base_instance: 0,
            frag_coord: 0,
            front_facing: 0,
            frag_depth: 0,
            stencil_ref: 0,
            sample_mask: 0,
            sample_mask_in: 0,
            helper_invocation: 0,
            sample_index: 0,
            clip_distances: 0,
            cull_distances: 0,
            patch_vertices: 0,
            output_tess_level_outer: 0,
            output_tess_level_inner: 0,
            tess_coord: 0,
            patches: [0; 30],
            workgroup_id: 0,
            num_workgroups_id: 0,
            workgroup_index_id: 0,
            local_invocation_id: 0,
            local_invocation_index: 0,
            invocation_id: 0,
            subgroup_local_invocation_id: 0,
            subgroup_lt_mask: 0,
            image_u32: 0,
            image_f32: 0,
            shared_memory_u16: 0,
            shared_memory_u32: 0,
            shared_memory_u64: 0,
            shared_memory_u16_type: 0,
            shared_memory_u32_type: 0,
            shared_memory_u64_type: 0,
            bary_coord: 0,
            bary_coord_smooth: 0,
            bary_coord_smooth_centroid: 0,
            bary_coord_smooth_sample: 0,
            bary_coord_pull_model: 0,
            bary_coord_nopersp: 0,
            bary_coord_nopersp_sample: 0,
            binding,
            fetch_code,
            read,
            buf_type_ids: Vec::new(),
            buffers: Vec::new(),
            images: Vec::new(),
            samplers: Vec::new(),
            first_to_last_label_map: HashMap::new(),
            flatbuf_index: 0,
            bda_pagetable_index: 0,
            fault_buffer_index: 0,
            physical_pointer_type_u32: 0,
            sampler_type: 0,
            sampler_pointer_type: 0,
            input_attr_array: 0,
            output_attr_array: 0,
            input_params: [SpirvAttribute::default(); NUM_PARAMS],
            output_params: [SpirvAttribute::default(); NUM_PARAMS],
            frag_outputs: [SpirvAttribute::default(); NUM_RENDER_TARGETS],
            uf11_to_f32: 0,
            f32_to_uf11: 0,
            uf10_to_f32: 0,
            f32_to_uf10: 0,
            get_bda_pointer: 0,
            read_const: 0,
            read_const_dynamic: 0,
        };

        if context.info.borrow().persistent.uses_dma || context.VertexMotion() {
            context.SetMemoryModel(
                spv::AddressingModel::PhysicalStorageBuffer64,
                spv::MemoryModel::GLSL450,
            );
        } else {
            context.SetMemoryModel(spv::AddressingModel::Logical, spv::MemoryModel::GLSL450);
        }
        let hash = context.info.borrow().persistent.pgm_hash;
        context.String(&format!("{hash:#x}"));

        context.AddCapability(spv::Capability::Shader);
        context.DefineArithmeticTypes();
        context.DefineInterfaces();
        context.DefineSharedMemory();
        context.DefineBuffers();
        context.DefineImagesAndSamplers();
        context.DefineFunctions();
        context
    }

    /// `EmitContext::Def`.
    pub fn Def(&mut self, value: &Value) -> Id {
        let types = self.type_ids();
        if !value.is_immediate() {
            return value.inst().borrow().definition_bits();
        }
        match value.ty() {
            Type::Void => 0,
            Type::U1 => {
                if value.u1() {
                    types.true_value
                } else {
                    types.false_value
                }
            }
            Type::U32 => self.ConstU32(value.u32()),
            Type::U64 => {
                let ty = types.U64;
                self.Constant(ty, value.u64())
            }
            Type::F32 => self.ConstF32(value.f32()),
            Type::F64 => {
                let ty = types.F64[1];
                self.Constant(ty, value.f64())
            }
            Type::StringLiteral => self.String(value.string_literal()),
            other => unreachable!("immediate type {other:?}"),
        }
    }

    fn DefineArithmeticTypes(&mut self) {
        let ty = self.TypeVoid();
        self.void_id = self.Name(ty, "void_id");
        let ty = self.TypeBool();
        self.U1[1] = self.Name(ty, "bool_id");
        let ty = self.TypeUInt(8);
        self.U8 = self.Name(ty, "u8_id");
        let ty = self.TypeSInt(8);
        self.S8 = self.Name(ty, "i8_id");
        let ty = self.TypeUInt(16);
        self.U16 = self.Name(ty, "u16_id");
        let ty = self.TypeSInt(16);
        self.S16 = self.Name(ty, "i16_id");
        if self.info.borrow().uses_fp16 {
            let ty = self.TypeFloat(16);
            self.F16[1] = self.Name(ty, "f16_id");
        }
        if self.info.borrow().uses_fp64 {
            let ty = self.TypeFloat(64);
            self.F64[1] = self.Name(ty, "f64_id");
        }
        let ty = self.TypeFloat(32);
        self.F32[1] = self.Name(ty, "f32_id");
        let ty = self.TypeSInt(32);
        self.S32[1] = self.Name(ty, "i32_id");
        let ty = self.TypeUInt(32);
        self.U32[1] = self.Name(ty, "u32_id");
        let ty = self.TypeUInt(64);
        self.U64 = self.Name(ty, "u64_id");

        for index in 2..=4usize {
            if self.info.borrow().uses_fp16 {
                let component = self.F16[1];
                let ty = self.TypeVector(component, index as u32);
                self.F16[index] = self.Name(ty, &format!("f16vec{index}_id"));
            }
            if self.info.borrow().uses_fp64 {
                let component = self.F64[1];
                let ty = self.TypeVector(component, index as u32);
                self.F64[index] = self.Name(ty, &format!("f64vec{index}_id"));
            }
            let f32_component = self.F32[1];
            let ty = self.TypeVector(f32_component, index as u32);
            self.F32[index] = self.Name(ty, &format!("f32vec{index}_id"));
            let s32_component = self.S32[1];
            let ty = self.TypeVector(s32_component, index as u32);
            self.S32[index] = self.Name(ty, &format!("i32vec{index}_id"));
            let u32_component = self.U32[1];
            let ty = self.TypeVector(u32_component, index as u32);
            self.U32[index] = self.Name(ty, &format!("u32vec{index}_id"));
            let u1_component = self.U1[1];
            let ty = self.TypeVector(u1_component, index as u32);
            self.U1[index] = self.Name(ty, &format!("bvec{index}_id"));
        }

        let u1 = self.U1[1];
        self.true_value = self.ConstantTrue(u1);
        self.false_value = self.ConstantFalse(u1);
        let u8 = self.U8;
        self.u8_one_value = self.Constant(u8, 1u32);
        self.u8_zero_value = self.Constant(u8, 0u32);
        let u16 = self.U16;
        self.u16_zero_value = self.Constant(u16, 0u32);
        self.u32_one_value = self.ConstU32(1);
        self.u32_zero_value = self.ConstU32(0);
        self.f32_zero_value = self.ConstF32(0.0);
        let u64_type = self.U64;
        self.u64_one_value = self.Constant(u64_type, 1u64);
        self.u64_zero_value = self.Constant(u64_type, 0u64);

        self.pi_x2 = self.ConstF32(2.0 * std::f32::consts::PI);

        let f32_component = self.F32[1];
        let ty = self.TypePointer(spv::StorageClass::Input, f32_component);
        self.input_f32 = self.Name(ty, "input_f32");
        let u32_component = self.U32[1];
        let ty = self.TypePointer(spv::StorageClass::Input, u32_component);
        self.input_u32 = self.Name(ty, "input_u32");
        let s32_component = self.S32[1];
        let ty = self.TypePointer(spv::StorageClass::Input, s32_component);
        self.input_s32 = self.Name(ty, "input_s32");
        let ty = self.TypePointer(spv::StorageClass::Output, f32_component);
        self.output_f32 = self.Name(ty, "output_f32");
        let ty = self.TypePointer(spv::StorageClass::Output, u32_component);
        self.output_u32 = self.Name(ty, "output_u32");
        let ty = self.TypePointer(spv::StorageClass::Output, s32_component);
        self.output_s32 = self.Name(ty, "output_s32");

        let ty = self.TypeStruct(&[s32_component, s32_component]);
        self.full_result_i32x2 = self.Name(ty, "full_result_i32x2");
        let ty = self.TypeStruct(&[u32_component, u32_component]);
        self.full_result_u32x2 = self.Name(ty, "full_result_u32x2");
        let ty = self.TypeStruct(&[f32_component, s32_component]);
        self.frexp_result_f32 = self.Name(ty, "frexp_result_f32");
        if self.info.borrow().uses_fp64 {
            let f64_component = self.F64[1];
            let ty = self.TypeStruct(&[f64_component, s32_component]);
            self.frexp_result_f64 = self.Name(ty, "frexp_result_f64");
        }
        if self.info.borrow().persistent.uses_dma {
            let u32_component = self.U32[1];
            self.physical_pointer_type_u32 =
                self.TypePointer(spv::StorageClass::PhysicalStorageBuffer, u32_component);
        }
    }

    fn DefineInterfaces(&mut self) {
        self.DefinePushDataBlock();
        self.DefineInputs();
        self.DefineOutputs();
    }

    fn attribute_type(&self, fmt: NumberFormat) -> VectorIds {
        let types = self.type_ids();
        match get_number_class(fmt) {
            NumberClass::Float => types.F32,
            NumberClass::Sint => types.S32,
            NumberClass::Uint => types.U32,
        }
    }

    fn GetAttributeInfo(
        &self,
        fmt: NumberFormat,
        id: Id,
        num_components: u32,
        output: bool,
        loaded: bool,
        array: bool,
    ) -> SpirvAttribute {
        let types = self.type_ids();
        let component_type = self.attribute_type(fmt)[1];
        let mut attribute = SpirvAttribute {
            id_array: [id, 0, 0],
            component_type,
            num_components,
            is_loaded: loaded,
            is_array: array,
            ..SpirvAttribute::default()
        };
        match get_number_class(fmt) {
            NumberClass::Float => {
                attribute.pointer_type = if output {
                    types.output_f32
                } else {
                    types.input_f32
                };
                attribute.is_integer = false;
            }
            NumberClass::Uint => {
                attribute.pointer_type = if output {
                    types.output_u32
                } else {
                    types.input_u32
                };
                attribute.is_integer = true;
            }
            NumberClass::Sint => {
                attribute.pointer_type = if output {
                    types.output_s32
                } else {
                    types.input_s32
                };
                attribute.is_integer = true;
            }
        }
        attribute
    }

    #[allow(dead_code)]
    fn GetBufferSize(&mut self, sharp_idx: u32) -> Id {
        let types = self.type_ids();
        let flatbuf = self.buffers[self.flatbuf_index].clone();
        assert_eq!(flatbuf.buffer_type, BufferType::Flatbuf);
        let alias = flatbuf.alias(PointerType::U32);
        let zero = types.u32_zero_value;
        let index1 = self.ConstU32(sharp_idx + 1);
        let ptr1 = self.OpAccessChain(alias.pointer_type, alias.id, &[zero, index1]);
        let rsrc1 = self.OpLoad(types.U32[1], ptr1, &[]);
        let index2 = self.ConstU32(sharp_idx + 2);
        let ptr2 = self.OpAccessChain(alias.pointer_type, alias.id, &[zero, index2]);
        let rsrc2 = self.OpLoad(types.U32[1], ptr2, &[]);
        let offset = self.ConstU32(16);
        let count = self.ConstU32(14);
        let stride = self.OpBitFieldUExtract(types.U32[1], rsrc1, offset, count);
        let zero = types.u32_zero_value;
        let stride_zero = self.OpIEqual(types.U1[1], stride, zero);
        let stride_size = self.OpIMul(types.U32[1], rsrc2, stride);
        self.OpSelect(types.U32[1], stride_zero, rsrc2, stride_size)
    }

    pub fn DefineBufferProperties(&mut self) {
        let types = self.type_ids();
        if !self.profile.needs_buffer_offsets {
            return;
        }
        for index in 0..self.buffers.len() {
            let buffer = self.buffers[index].clone();
            if buffer.buffer_type != BufferType::Guest {
                continue;
            }
            let desc = self.info.borrow().persistent.buffers[index];
            let binding = buffer.binding;
            let half = PushData::BUF_OFFSET_INDEX + (binding >> 4);
            let component = (binding & 0xf) >> 2;
            let offset_bits = (binding & 0x3) << 3;
            let u32_ty = types.U32[1];
            let push = types.push_data_block;
            let half = self.ConstU32(half);
            let component = self.ConstU32(component);
            let pointer_type = self.TypePointer(spv::StorageClass::PushConstant, u32_ty);
            let ptr = self.OpAccessChain(pointer_type, push, &[half, component]);
            let value = self.OpLoad(types.U32[1], ptr, &[]);
            let offset = self.ConstU32(offset_bits);
            let count = self.ConstU32(8);
            let buf_offset = self.OpBitFieldUExtract(types.U32[1], value, offset, count);
            self.Name(buf_offset, &format!("buf{binding}_off"));
            self.buffers[index].set_offset(PointerSize::B8, buf_offset);
            if desc.used_types.contains(Type::U16) {
                let one = self.ConstU32(1);
                let word = self.OpShiftRightLogical(types.U32[1], buf_offset, one);
                self.Name(word, &format!("buf{binding}_word_off"));
                self.buffers[index].set_offset(PointerSize::B16, word);
            }
            if desc.used_types.contains(Type::U32) {
                let two = self.ConstU32(2);
                let dword = self.OpShiftRightLogical(types.U32[1], buf_offset, two);
                self.Name(dword, &format!("buf{binding}_dword_off"));
                self.buffers[index].set_offset(PointerSize::B32, dword);
            }
            if desc.used_types.contains(Type::U64) {
                let three = self.ConstU32(3);
                let qword = self.OpShiftRightLogical(types.U32[1], buf_offset, three);
                self.Name(qword, &format!("buf{binding}_qword_off"));
                self.buffers[index].set_offset(PointerSize::B64, qword);
            }
        }
    }

    pub fn DefineAmdPerVertexAttribs(&mut self) {
        let types = self.type_ids();
        if !self.profile.supports_amd_shader_explicit_vertex_parameter {
            return;
        }
        let num_inputs = match &self.runtime_info.hw {
            HwInfo::Fs(fs) => fs.num_inputs,
            _ => return,
        };
        for index in 0..num_inputs as usize {
            let input = match &self.runtime_info.hw {
                HwInfo::Fs(fs) => fs.inputs[index],
                _ => unreachable!(),
            };
            if input.is_default()
                || self.info.borrow().fs_interpolation[index].primary != Qualifier::PerVertex
            {
                continue;
            }
            let pointer = self.input_params[index].id();
            let num_components = self.input_params[index].num_components;
            let ty = types.F32[num_components as usize];
            let zero = self.ConstU32(0);
            let one = self.ConstU32(1);
            let two = self.ConstU32(2);
            let first = self.OpInterpolateAtVertexAMD(ty, pointer, zero);
            let second = self.OpInterpolateAtVertexAMD(ty, pointer, one);
            let third = self.OpInterpolateAtVertexAMD(ty, pointer, two);
            let param = &mut self.input_params[index];
            param.id_array = [first, second, third];
            param.is_loaded = true;
        }
    }

    pub fn DefineWorkgroupIndex(&mut self) {
        let types = self.type_ids();
        let workgroup_u32x3 = types.U32[3];
        let workgroup_id = types.workgroup_id;
        let workgroup_id_val = self.OpLoad(workgroup_u32x3, workgroup_id, &[]);
        let workgroup_x = self.OpCompositeExtract(types.U32[1], workgroup_id_val, &[0]);
        let workgroup_y = self.OpCompositeExtract(types.U32[1], workgroup_id_val, &[1]);
        let workgroup_z = self.OpCompositeExtract(types.U32[1], workgroup_id_val, &[2]);
        let num_workgroups_id = types.num_workgroups_id;
        let num_workgroups = self.OpLoad(workgroup_u32x3, num_workgroups_id, &[]);
        let num_workgroups_x = self.OpCompositeExtract(types.U32[1], num_workgroups, &[0]);
        let num_workgroups_y = self.OpCompositeExtract(types.U32[1], num_workgroups, &[1]);
        let y_times_x = self.OpIMul(types.U32[1], workgroup_y, num_workgroups_x);
        let inner = self.OpIAdd(types.U32[1], workgroup_x, y_times_x);
        let x_times_y = self.OpIMul(types.U32[1], num_workgroups_x, num_workgroups_y);
        let z_times = self.OpIMul(types.U32[1], workgroup_z, x_times_y);
        self.workgroup_index_id = self.OpIAdd(types.U32[1], inner, z_times);
        self.Name(types.workgroup_index_id, "workgroup_index");
    }

    fn DefineInput(&mut self, ty: Id, location: Option<u32>, builtin: Option<u32>) -> Id {
        let id = self.DefineVariable(ty, builtin, spv::StorageClass::Input, None);
        if let Some(location) = location {
            self.Decorate(id, spv::Decoration::Location, &[location]);
        }
        id
    }

    fn DefineOutput(&mut self, ty: Id, location: Option<u32>, builtin: Option<u32>) -> Id {
        let id = self.DefineVariable(ty, builtin, spv::StorageClass::Output, None);
        if let Some(location) = location {
            self.Decorate(id, spv::Decoration::Location, &[location]);
        }
        id
    }

    fn DefineVariable(
        &mut self,
        ty: Id,
        builtin: Option<u32>,
        storage_class: u32,
        initializer: Option<Id>,
    ) -> Id {
        let pointer_type = self.TypePointer(storage_class, ty);
        let id = self.AddGlobalVariable(pointer_type, storage_class, initializer);
        if let Some(builtin) = builtin {
            self.Decorate(id, spv::Decoration::BuiltIn, &[builtin]);
        }
        self.interfaces.push(id);
        id
    }

    fn DefineVar(&mut self, ty: Id, storage_class: u32, initializer: Option<Id>) -> Id {
        let pointer_type = self.TypePointer(storage_class, ty);
        self.AddGlobalVariable(pointer_type, storage_class, initializer)
    }

    #[allow(dead_code)]
    fn DefineUniformConst(&mut self, ty: Id, set: u32, binding: u32, readonly: bool) -> Id {
        let id = self.DefineVar(ty, spv::StorageClass::UniformConstant, None);
        self.Decorate(id, spv::Decoration::DescriptorSet, &[set]);
        self.Decorate(id, spv::Decoration::Binding, &[binding]);
        if readonly {
            self.Decorate(id, spv::Decoration::NonWritable, &[]);
        }
        id
    }

    fn TypeArrayLen(&mut self, element_type: Id, length: u32) -> Id {
        let count = self.ConstU32(length);
        self.TypeArray(element_type, count)
    }

    pub fn ConstU32(&mut self, value: u32) -> Id {
        let types = self.type_ids();
        let ty = types.U32[1];
        self.Constant(ty, value)
    }

    pub fn ConstS32(&mut self, value: i32) -> Id {
        let types = self.type_ids();
        let ty = types.S32[1];
        self.Constant(ty, value)
    }

    pub fn ConstS32Slice(&mut self, values: &[i32]) -> Id {
        let types = self.type_ids();
        let ty = if values.len() <= 4 {
            types.S32[values.len()]
        } else {
            let count = self.ConstU32(values.len() as u32);
            self.TypeArray(types.S32[1], count)
        };
        let constituents: Vec<Id> = values.iter().map(|value| self.ConstS32(*value)).collect();
        self.ConstantComposite(ty, &constituents)
    }

    pub fn ConstU32Slice(&mut self, values: &[u32]) -> Id {
        let types = self.type_ids();
        let ty = if values.len() <= 4 {
            types.U32[values.len()]
        } else {
            let count = self.ConstU32(values.len() as u32);
            self.TypeArray(types.U32[1], count)
        };
        let constituents: Vec<Id> = values.iter().map(|value| self.ConstU32(*value)).collect();
        self.ConstantComposite(ty, &constituents)
    }

    pub fn ConstF32(&mut self, value: f32) -> Id {
        let types = self.type_ids();
        let ty = types.F32[1];
        self.Constant(ty, value)
    }

    pub fn AddLabel(&mut self) -> Id {
        let label = self.OpLabel();
        let label = Module::AddLabel(self, label);
        self.last_label = label;
        label
    }

    pub fn AddExistingLabel(&mut self, label: Id) -> Id {
        let label = Module::AddLabel(self, label);
        self.last_label = label;
        label
    }

    pub fn EmitDwordMemoryRead(
        &mut self,
        address: Id,
        fallback: impl FnOnce(&mut Self) -> Id,
    ) -> Id {
        let types = self.type_ids();
        let available_label = self.OpLabel();
        let fallback_label = self.OpLabel();
        let merge_label = self.OpLabel();

        let u64_ty = types.U64;
        let get_bda_pointer = types.get_bda_pointer;
        let addr = self.OpFunctionCall(u64_ty, get_bda_pointer, &[address]);
        let u1 = types.U1[1];
        let zero = types.u64_zero_value;
        let is_available = self.OpINotEqual(u1, addr, zero);
        self.OpSelectionMerge(merge_label, spv::SelectionControlMask::MaskNone);
        self.OpBranchConditional(is_available, available_label, fallback_label);

        self.AddExistingLabel(available_label);
        let pointer = self.OpConvertUToPtr(types.physical_pointer_type_u32, addr);
        let u32_ty = types.U32[1];
        let result = self.OpLoad(u32_ty, pointer, &[spv::MemoryAccessMask::Aligned, 4]);
        self.OpBranch(merge_label);

        self.AddExistingLabel(fallback_label);
        let fallback_result = fallback(self);
        self.OpBranch(merge_label);

        self.AddExistingLabel(merge_label);
        self.OpPhi(
            u32_ty,
            &[fallback_result, fallback_label, result, available_label],
        )
    }

    pub fn EmitSharedMemoryAccess(&mut self, result_type: Id, shared_mem: Id, index: Id) -> Id {
        let types = self.type_ids();
        let shared_types = self.info.borrow().shared_types;
        if shared_types.bits().count_ones() > 1 {
            let zero = types.u32_zero_value;
            return self.OpAccessChain(result_type, shared_mem, &[zero, index]);
        }
        self.OpAccessChain(result_type, shared_mem, &[index])
    }

    pub fn EmitFlatbufferLoad(&mut self, flatbuf_offset: Id) -> Id {
        let types = self.type_ids();
        let flatbuf = self.buffers[self.flatbuf_index].clone();
        assert_eq!(flatbuf.buffer_type, BufferType::Flatbuf);
        let alias = flatbuf.alias(PointerType::U32);
        let zero = types.u32_zero_value;
        let ptr = self.OpAccessChain(alias.pointer_type, alias.id, &[zero, flatbuf_offset]);
        self.OpLoad(types.U32[1], ptr, &[])
    }
}

impl EmitContext<'_> {
    fn DefineInputs(&mut self) {
        let types = self.type_ids();
        if self.info.borrow().uses_lane_id {
            self.subgroup_local_invocation_id = self.DefineVariable(
                types.U32[1],
                Some(spv::BuiltIn::SubgroupLocalInvocationId),
                spv::StorageClass::Input,
                None,
            );
            let id = types.subgroup_local_invocation_id;
            self.Decorate(id, spv::Decoration::Flat, &[]);
        }
        if self.info.borrow().loads.get_any(Attribute::SubgroupLtMask) {
            self.subgroup_lt_mask = self.DefineVariable(
                types.U32[4],
                Some(spv::BuiltIn::SubgroupLtMask),
                spv::StorageClass::Input,
                None,
            );
            let id = types.subgroup_lt_mask;
            self.Decorate(id, spv::Decoration::Flat, &[]);
        }
        match self.sw_stage {
            SwStage::Vertex => self.DefineVertexInputs(),
            SwStage::Fragment => self.DefineFragmentInputs(),
            SwStage::Compute => {
                let loads = self.info.borrow().loads;
                if loads.get_any(Attribute::WorkgroupIndex) || loads.get_any(Attribute::WorkgroupId)
                {
                    self.workgroup_id = self.DefineVariable(
                        types.U32[3],
                        Some(spv::BuiltIn::WorkgroupId),
                        spv::StorageClass::Input,
                        None,
                    );
                }
                if loads.get_any(Attribute::WorkgroupIndex) {
                    self.num_workgroups_id = self.DefineVariable(
                        types.U32[3],
                        Some(spv::BuiltIn::NumWorkgroups),
                        spv::StorageClass::Input,
                        None,
                    );
                }
                if loads.get_any(Attribute::LocalInvocationId) {
                    self.local_invocation_id = self.DefineVariable(
                        types.U32[3],
                        Some(spv::BuiltIn::LocalInvocationId),
                        spv::StorageClass::Input,
                        None,
                    );
                }
                if loads.get(Attribute::LocalInvocationIndex, 0) {
                    self.local_invocation_index = self.DefineVariable(
                        types.U32[1],
                        Some(spv::BuiltIn::LocalInvocationIndex),
                        spv::StorageClass::Input,
                        None,
                    );
                }
            }
            SwStage::Geometry => self.DefineGeometryInputs(),
            SwStage::TessellationControl => self.DefineTessControlInputs(),
            SwStage::TessellationEval => self.DefineTessEvalInputs(),
            _ => unreachable!("invalid logical stage"),
        }
    }

    fn DefineVertexInputs(&mut self) {
        let types = self.type_ids();
        self.vertex_index = self.DefineVariable(
            types.U32[1],
            Some(spv::BuiltIn::VertexIndex),
            spv::StorageClass::Input,
            None,
        );
        self.instance_id = self.DefineVariable(
            types.U32[1],
            Some(spv::BuiltIn::InstanceIndex),
            spv::StorageClass::Input,
            None,
        );
        let loads = self.info.borrow().loads;
        if loads.get(Attribute::BaseVertex, 0) {
            self.base_vertex = self.DefineVariable(
                types.U32[1],
                Some(spv::BuiltIn::BaseVertex),
                spv::StorageClass::Input,
                None,
            );
        }
        if loads.get(Attribute::BaseInstance, 0) {
            self.base_instance = self.DefineVariable(
                types.U32[1],
                Some(spv::BuiltIn::BaseInstance),
                spv::StorageClass::Input,
                None,
            );
        }
        let info = self.info.clone();
        let fetch_shader = parse_fetch_shader(&info.borrow(), self.fetch_code);
        let Some(fetch_shader) = fetch_shader else {
            return;
        };
        for attrib in &fetch_shader.attributes {
            assert!((attrib.semantic as usize) < NUM_PARAMS);
            let sharp = attrib.get_sharp(&info.borrow(), self.read);
            let ty = self.attribute_type(sharp.get_number_fmt())[4];
            let id = self.DefineInput(ty, Some(attrib.semantic as u32), None);
            if attrib.get_step_rate() != InstanceIdType::None {
                self.Name(id, &format!("vs_instance_attr{}", attrib.semantic));
            } else {
                self.Name(id, &format!("vs_in_attr{}", attrib.semantic));
            }
            self.input_params[attrib.semantic as usize] =
                self.GetAttributeInfo(sharp.get_number_fmt(), id, 4, false, false, false);
        }
    }

    fn DefineFragmentInputs(&mut self) {
        let types = self.type_ids();
        let fs = match self.runtime_info.hw {
            HwInfo::Fs(fs) => fs,
            _ => unreachable!("a fragment shader has fragment runtime info"),
        };
        let loads = self.info.borrow().loads;
        if self.FragmentMotion()
            || loads.get_any(Attribute::FragCoord)
            || (loads.get_any(Attribute::BaryCoordPullModel)
                && !self.profile.supports_amd_shader_explicit_vertex_parameter)
        {
            self.frag_coord = self.DefineVariable(
                types.F32[4],
                Some(spv::BuiltIn::FragCoord),
                spv::StorageClass::Input,
                None,
            );
        }
        if loads.get(Attribute::IsFrontFace, 0) {
            self.front_facing = self.DefineVariable(
                types.U1[1],
                Some(spv::BuiltIn::FrontFacing),
                spv::StorageClass::Input,
                None,
            );
        }
        if loads.get_any(Attribute::RenderTargetIndex) {
            self.output_layer = self.DefineVariable(
                types.U32[1],
                Some(spv::BuiltIn::Layer),
                spv::StorageClass::Input,
                None,
            );
            let id = types.output_layer;
            self.Decorate(id, spv::Decoration::Flat, &[]);
        }
        if loads.get(Attribute::SampleIndex, 0) {
            self.sample_index = self.DefineVariable(
                types.U32[1],
                Some(spv::BuiltIn::SampleId),
                spv::StorageClass::Input,
                None,
            );
            let id = types.sample_index;
            self.Decorate(id, spv::Decoration::Flat, &[]);
        }
        if loads.get(Attribute::IsHelperInvocation, 0) {
            self.helper_invocation = self.DefineVariable(
                types.U1[1],
                Some(spv::BuiltIn::HelperInvocation),
                spv::StorageClass::Input,
                None,
            );
        }
        if loads.get(Attribute::SampleMask, 0) {
            let u32_ty = types.U32[1];
            let one = self.ConstU32(1);
            let array = self.TypeArray(u32_ty, one);
            self.sample_mask_in = self.DefineVariable(
                array,
                Some(spv::BuiltIn::SampleMask),
                spv::StorageClass::Input,
                None,
            );
        }
        if loads.get_any(Attribute::BaryCoordSmooth) {
            if self.profile.supports_amd_shader_explicit_vertex_parameter {
                self.bary_coord_smooth = self.DefineVariable(
                    types.F32[2],
                    Some(spv::BuiltIn::BaryCoordSmoothAMD),
                    spv::StorageClass::Input,
                    None,
                );
            } else if self.profile.supports_fragment_shader_barycentric
                && !ValidId(types.bary_coord)
            {
                self.bary_coord = self.DefineVariable(
                    types.F32[3],
                    Some(spv::BuiltIn::BaryCoordKHR),
                    spv::StorageClass::Input,
                    None,
                );
            }
        }
        if loads.get_any(Attribute::BaryCoordPullModel) {
            if self.profile.supports_amd_shader_explicit_vertex_parameter {
                self.bary_coord_pull_model = self.DefineVariable(
                    types.F32[3],
                    Some(spv::BuiltIn::BaryCoordPullModelAMD),
                    spv::StorageClass::Input,
                    None,
                );
            } else if self.profile.supports_fragment_shader_barycentric
                && !ValidId(types.bary_coord)
            {
                self.bary_coord = self.DefineVariable(
                    types.F32[3],
                    Some(spv::BuiltIn::BaryCoordKHR),
                    spv::StorageClass::Input,
                    None,
                );
            }
        }
        if loads.get_any(Attribute::BaryCoordSmoothCentroid) {
            if self.profile.supports_amd_shader_explicit_vertex_parameter {
                self.bary_coord_smooth_centroid = self.DefineVariable(
                    types.F32[2],
                    Some(spv::BuiltIn::BaryCoordSmoothCentroidAMD),
                    spv::StorageClass::Input,
                    None,
                );
            } else if self.profile.supports_fragment_shader_barycentric
                && !ValidId(types.bary_coord)
            {
                self.bary_coord = self.DefineVariable(
                    types.F32[3],
                    Some(spv::BuiltIn::BaryCoordKHR),
                    spv::StorageClass::Input,
                    None,
                );
            }
        }
        if loads.get_any(Attribute::BaryCoordSmoothSample) {
            if self.profile.supports_amd_shader_explicit_vertex_parameter {
                self.bary_coord_smooth_sample = self.DefineVariable(
                    types.F32[2],
                    Some(spv::BuiltIn::BaryCoordSmoothSampleAMD),
                    spv::StorageClass::Input,
                    None,
                );
            } else if self.profile.supports_fragment_shader_barycentric
                && !ValidId(types.bary_coord)
            {
                self.bary_coord = self.DefineVariable(
                    types.F32[3],
                    Some(spv::BuiltIn::BaryCoordKHR),
                    spv::StorageClass::Input,
                    None,
                );
                if !ValidId(types.sample_index) {
                    self.sample_index = self.DefineVariable(
                        types.U32[1],
                        Some(spv::BuiltIn::SampleId),
                        spv::StorageClass::Input,
                        None,
                    );
                    let id = types.sample_index;
                    self.Decorate(id, spv::Decoration::Flat, &[]);
                }
            }
        }
        if loads.get_any(Attribute::BaryCoordNoPersp) {
            if self.profile.supports_amd_shader_explicit_vertex_parameter {
                self.bary_coord_nopersp = self.DefineVariable(
                    types.F32[2],
                    Some(spv::BuiltIn::BaryCoordNoPerspAMD),
                    spv::StorageClass::Input,
                    None,
                );
            } else if self.profile.supports_fragment_shader_barycentric {
                self.bary_coord_nopersp = self.DefineVariable(
                    types.F32[3],
                    Some(spv::BuiltIn::BaryCoordNoPerspKHR),
                    spv::StorageClass::Input,
                    None,
                );
            }
        }
        if loads.get_any(Attribute::BaryCoordNoPerspSample) {
            if self.profile.supports_amd_shader_explicit_vertex_parameter {
                self.bary_coord_nopersp_sample = self.DefineVariable(
                    types.F32[2],
                    Some(spv::BuiltIn::BaryCoordNoPerspSampleAMD),
                    spv::StorageClass::Input,
                    None,
                );
            } else if self.profile.supports_fragment_shader_barycentric {
                self.bary_coord_nopersp_sample = self.DefineVariable(
                    types.F32[3],
                    Some(spv::BuiltIn::BaryCoordNoPerspKHR),
                    spv::StorageClass::Input,
                    None,
                );
            }
        }

        let has_clip_distance_inputs = fs.clip_distance_emulation;
        let num_inputs = fs
            .num_inputs
            .saturating_sub(u32::from(has_clip_distance_inputs));
        for index in 0..num_inputs as usize {
            let input = fs.inputs[index];
            if input.is_default() {
                continue;
            }
            let param = Attribute::from_raw(Attribute::Param0 as u64 + index as u64).unwrap();
            let num_components = self.info.borrow().loads.num_components(param);
            let interpolation = self.info.borrow().fs_interpolation[index];
            let ty = types.F32[num_components as usize];
            let bind_location = input.param_index as u32 + u32::from(has_clip_distance_inputs);
            let attr_id = if interpolation.primary == Qualifier::PerVertex
                && self.profile.supports_fragment_shader_barycentric
            {
                let three = self.ConstU32(3);
                let array = self.TypeArray(ty, three);
                let id = self.DefineInput(array, Some(bind_location), None);
                self.Name(id, &format!("fs_in_attr{index}_p"))
            } else {
                let id = self.DefineInput(ty, Some(bind_location), None);
                self.Name(id, &format!("fs_in_attr{index}"))
            };
            if interpolation.primary == Qualifier::PerVertex {
                let decoration = if self.profile.supports_amd_shader_explicit_vertex_parameter {
                    spv::Decoration::ExplicitInterpAMD
                } else {
                    spv::Decoration::PerVertexKHR
                };
                self.Decorate(attr_id, decoration, &[]);
            } else if interpolation.primary != Qualifier::Smooth {
                let decoration = if interpolation.primary == Qualifier::Flat {
                    spv::Decoration::Flat
                } else {
                    spv::Decoration::NoPerspective
                };
                self.Decorate(attr_id, decoration, &[]);
            }
            if interpolation.auxiliary != Qualifier::None {
                let decoration = if interpolation.auxiliary == Qualifier::Centroid {
                    spv::Decoration::Centroid
                } else {
                    spv::Decoration::Sample
                };
                self.Decorate(attr_id, decoration, &[]);
            }
            self.input_params[index] = self.GetAttributeInfo(
                NumberFormat::Float,
                attr_id,
                num_components,
                false,
                false,
                interpolation.primary == Qualifier::PerVertex,
            );
        }
        if has_clip_distance_inputs {
            let ty = types.F32[MAX_EMULATED_CLIP_DISTANCES as usize];
            let attr_id = self.DefineInput(ty, Some(0), None);
            self.Name(attr_id, "cldist_attr0");
            self.input_params[num_inputs as usize] = self.GetAttributeInfo(
                NumberFormat::Float,
                attr_id,
                MAX_EMULATED_CLIP_DISTANCES,
                false,
                false,
                false,
            );
        }
    }

    fn DefineGeometryInputs(&mut self) {
        let types = self.type_ids();
        self.primitive_id = self.DefineVariable(
            types.U32[1],
            Some(spv::BuiltIn::PrimitiveId),
            spv::StorageClass::Input,
            None,
        );
        let f32x1 = types.F32[1];
        let f32x4 = types.F32[4];
        let one = self.ConstU32(1);
        let clip_array = self.TypeArray(f32x1, one);
        let gl_per_vertex = self.TypeStruct(&[f32x4, f32x1, clip_array]);
        self.Name(gl_per_vertex, "gl_PerVertex");
        self.MemberName(gl_per_vertex, 0, "gl_Position");
        self.MemberName(gl_per_vertex, 1, "gl_PointSize");
        self.MemberName(gl_per_vertex, 2, "gl_ClipDistance");
        self.MemberDecorate(
            gl_per_vertex,
            0,
            spv::Decoration::BuiltIn,
            &[spv::BuiltIn::Position],
        );
        self.MemberDecorate(
            gl_per_vertex,
            1,
            spv::Decoration::BuiltIn,
            &[spv::BuiltIn::PointSize],
        );
        self.MemberDecorate(
            gl_per_vertex,
            2,
            spv::Decoration::BuiltIn,
            &[spv::BuiltIn::ClipDistance],
        );
        self.Decorate(gl_per_vertex, spv::Decoration::Block, &[]);
        let (in_primitive, in_vertex_data_size) = match &self.runtime_info.hw {
            HwInfo::Gs(gs) => (gs.in_primitive, gs.in_vertex_data_size),
            _ => unreachable!("a geometry shader has geometry runtime info"),
        };
        let num_verts_in = num_vertices(in_primitive);
        let count = self.ConstU32(num_verts_in);
        let vertices_in = self.TypeArray(gl_per_vertex, count);
        self.gl_in = self.DefineVar(vertices_in, spv::StorageClass::Input, None);
        let gl_in = self.gl_in;
        self.Name(gl_in, "gl_in");
        self.interfaces.push(gl_in);

        let num_params = in_vertex_data_size / 4 - 1;
        for param_id in 0..num_params {
            let count = self.ConstU32(num_verts_in);
            let ty = self.TypeArray(f32x4, count);
            let id = self.DefineInput(ty, Some(param_id), None);
            self.Name(id, &format!("gs_in_attr{param_id}"));
            self.input_params[param_id as usize] =
                self.GetAttributeInfo(NumberFormat::Float, id, 4, false, false, true);
        }
    }

    fn DefineTessControlInputs(&mut self) {
        let types = self.type_ids();
        self.invocation_id = self.DefineVariable(
            types.U32[1],
            Some(spv::BuiltIn::InvocationId),
            spv::StorageClass::Input,
            None,
        );
        self.patch_vertices = self.DefineVariable(
            types.U32[1],
            Some(spv::BuiltIn::PatchVertices),
            spv::StorageClass::Input,
            None,
        );
        self.primitive_id = self.DefineVariable(
            types.U32[1],
            Some(spv::BuiltIn::PrimitiveId),
            spv::StorageClass::Input,
            None,
        );
        let tcs = match self.runtime_info.sw {
            SwInfo::Tcs(tcs) => tcs,
            _ => unreachable!("a tessellation control shader has control runtime info"),
        };
        let num_attrs = (align_up_pow2(tcs.ls_stride as u64, 16) >> 4) as u32;
        if num_attrs > 0 {
            let f32x4 = types.F32[4];
            let count = self.ConstU32(num_attrs);
            let per_vertex = self.TypeArray(f32x4, count);
            let count = self.ConstU32(32);
            let patch_array = self.TypeArray(per_vertex, count);
            self.input_attr_array = self.DefineInput(patch_array, Some(0), None);
            let id = self.input_attr_array;
            self.Name(id, "in_attrs");
        }
    }

    fn DefineTessEvalInputs(&mut self) {
        let types = self.type_ids();
        self.tess_coord = self.DefineInput(types.F32[3], None, Some(spv::BuiltIn::TessCoord));
        self.primitive_id = self.DefineVariable(
            types.U32[1],
            Some(spv::BuiltIn::PrimitiveId),
            spv::StorageClass::Input,
            None,
        );
        let tes = match self.runtime_info.sw {
            SwInfo::Tes(tes) => tes,
            _ => unreachable!("a tessellation eval shader has eval runtime info"),
        };
        let num_attrs = (align_up_pow2(tes.hs_output_cp_stride as u64, 16) >> 4) as u32;
        if num_attrs > 0 {
            let f32x4 = types.F32[4];
            let count = self.ConstU32(num_attrs);
            let per_vertex = self.TypeArray(f32x4, count);
            let count = self.ConstU32(32);
            let patch_array = self.TypeArray(per_vertex, count);
            self.input_attr_array = self.DefineInput(patch_array, Some(0), None);
            let id = self.input_attr_array;
            self.Name(id, "in_attrs");
        }
        let patch_base_location = num_attrs;
        let uses_patches = self.info.borrow().uses_patches;
        for index in 0..30u32 {
            if uses_patches & (1 << index) == 0 {
                continue;
            }
            let id = self.DefineInput(types.F32[4], Some(patch_base_location + index), None);
            self.Decorate(id, spv::Decoration::Patch, &[]);
            self.Name(id, &format!("patch_in{index}"));
            self.patches[index as usize] = id;
        }
    }

    fn DefineVertexBlock(&mut self) {
        let types = self.type_ids();
        let zeros = [types.f32_zero_value; 8];
        self.output_position = self.DefineVariable(
            types.F32[4],
            Some(spv::BuiltIn::Position),
            spv::StorageClass::Output,
            None,
        );
        let stores = self.info.borrow().stores;
        let needs_clip_distance_emulation = self.sw_stage == SwStage::Vertex
            && self.hw_stage == HwStage::Vertex
            && self.profile.needs_clip_distance_emulation;
        if stores.get_any(Attribute::ClipDistance) && !needs_clip_distance_emulation {
            let f32x1 = types.F32[1];
            let count = self.ConstU32(8);
            let ty = self.TypeArray(f32x1, count);
            let initializer = self.ConstantComposite(ty, &zeros);
            self.clip_distances = self.DefineVariable(
                ty,
                Some(spv::BuiltIn::ClipDistance),
                spv::StorageClass::Output,
                Some(initializer),
            );
        }
        if stores.get_any(Attribute::CullDistance) {
            let f32x1 = types.F32[1];
            let count = self.ConstU32(8);
            let ty = self.TypeArray(f32x1, count);
            let initializer = self.ConstantComposite(ty, &zeros);
            self.cull_distances = self.DefineVariable(
                ty,
                Some(spv::BuiltIn::CullDistance),
                spv::StorageClass::Output,
                Some(initializer),
            );
        }
        if stores.get_any(Attribute::PointSize) {
            self.output_point_size = self.DefineVariable(
                types.F32[1],
                Some(spv::BuiltIn::PointSize),
                spv::StorageClass::Output,
                None,
            );
        }
        if stores.get_any(Attribute::RenderTargetIndex) {
            self.output_layer = self.DefineVariable(
                types.U32[1],
                Some(spv::BuiltIn::Layer),
                spv::StorageClass::Output,
                None,
            );
        }
        if stores.get_any(Attribute::ViewportIndex) {
            self.output_viewport_index = self.DefineVariable(
                types.U32[1],
                Some(spv::BuiltIn::ViewportIndex),
                spv::StorageClass::Output,
                None,
            );
        }
    }

    fn DefineOutputs(&mut self) {
        let types = self.type_ids();
        match self.sw_stage {
            SwStage::Vertex => {
                self.DefineVertexBlock();
                if self.hw_stage == HwStage::Local {
                    let ls = match self.runtime_info.hw {
                        HwInfo::Ls(ls) => ls,
                        _ => unreachable!("a local shader has local runtime info"),
                    };
                    let num_attrs = (align_up_pow2(ls.ls_stride as u64, 16) >> 4) as u32;
                    if num_attrs > 0 {
                        let f32x4 = types.F32[4];
                        let ty = self.TypeArrayLen(f32x4, num_attrs);
                        self.output_attr_array = self.DefineOutput(ty, Some(0), None);
                        let id = self.output_attr_array;
                        self.Name(id, "out_attrs");
                    }
                } else {
                    let stores = self.info.borrow().stores;
                    let needs_clip_distance_emulation = self.hw_stage == HwStage::Vertex
                        && self.profile.needs_clip_distance_emulation
                        && stores.get_any(Attribute::ClipDistance);
                    let mut num_attrs = 0u32;
                    for index in 0..NUM_PARAMS {
                        let param =
                            Attribute::from_raw(Attribute::Param0 as u64 + index as u64).unwrap();
                        if !stores.get_any(param) {
                            continue;
                        }
                        let num_components = stores.num_components(param);
                        let ty = types.F32[num_components as usize];
                        let location = index as u32 + u32::from(needs_clip_distance_emulation);
                        let id = self.DefineOutput(ty, Some(location), None);
                        self.Name(id, &format!("out_attr{index}"));
                        self.output_params[index] = self.GetAttributeInfo(
                            NumberFormat::Float,
                            id,
                            num_components,
                            true,
                            false,
                            false,
                        );
                        num_attrs += 1;
                    }
                    if self.VertexMotion() {
                        self.motion_out_cur = self.DefineOutput(
                            types.F32[4],
                            Some(MotionVectors::CURRENT_LOCATION),
                            None,
                        );
                        self.motion_out_prev = self.DefineOutput(
                            types.F32[4],
                            Some(MotionVectors::PREVIOUS_LOCATION),
                            None,
                        );
                        let id = types.motion_out_cur;
                        self.Name(id, "motion_cur");
                        let id = types.motion_out_prev;
                        self.Name(id, "motion_prev");
                    }
                    if needs_clip_distance_emulation {
                        let ty = types.F32[MAX_EMULATED_CLIP_DISTANCES as usize];
                        self.clip_distances = self.DefineOutput(ty, Some(0), None);
                        self.output_params[num_attrs as usize] = self.GetAttributeInfo(
                            NumberFormat::Float,
                            types.clip_distances,
                            MAX_EMULATED_CLIP_DISTANCES,
                            true,
                            false,
                            false,
                        );
                        let id = types.clip_distances;
                        self.Name(id, "cldist_attr0");
                    }
                }
            }
            SwStage::TessellationControl => {
                if self.info.borrow().stores_tess_level_outer {
                    let f32x1 = types.F32[1];
                    let ty = self.TypeArrayLen(f32x1, 4);
                    self.output_tess_level_outer =
                        self.DefineOutput(ty, None, Some(spv::BuiltIn::TessLevelOuter));
                    let id = types.output_tess_level_outer;
                    self.Decorate(id, spv::Decoration::Patch, &[]);
                }
                if self.info.borrow().stores_tess_level_inner {
                    let f32x1 = types.F32[1];
                    let ty = self.TypeArrayLen(f32x1, 2);
                    self.output_tess_level_inner =
                        self.DefineOutput(ty, None, Some(spv::BuiltIn::TessLevelInner));
                    let id = types.output_tess_level_inner;
                    self.Decorate(id, spv::Decoration::Patch, &[]);
                }
                let tcs = match self.runtime_info.sw {
                    SwInfo::Tcs(tcs) => tcs,
                    _ => unreachable!(),
                };
                let num_attrs = (align_up_pow2(tcs.hs_output_cp_stride as u64, 16) >> 4) as u32;
                if num_attrs > 0 {
                    let f32x4 = types.F32[4];
                    let per_vertex = self.TypeArrayLen(f32x4, num_attrs);
                    let count = self.ConstU32(tcs.num_output_control_points());
                    let patch_array = self.TypeArray(per_vertex, count);
                    self.output_attr_array = self.DefineOutput(patch_array, Some(0), None);
                    self.Name(types.output_attr_array, "out_attrs");
                }
                let patch_base_location = num_attrs;
                let uses_patches = self.info.borrow().uses_patches;
                for index in 0..30u32 {
                    if uses_patches & (1 << index) == 0 {
                        continue;
                    }
                    let id =
                        self.DefineOutput(types.F32[4], Some(patch_base_location + index), None);
                    self.Decorate(id, spv::Decoration::Patch, &[]);
                    self.Name(id, &format!("patch_out{index}"));
                    self.patches[index as usize] = id;
                }
            }
            SwStage::TessellationEval => {
                self.DefineVertexBlock();
                let stores = self.info.borrow().stores;
                for index in 0..NUM_PARAMS {
                    let param =
                        Attribute::from_raw(Attribute::Param0 as u64 + index as u64).unwrap();
                    if !stores.get_any(param) {
                        continue;
                    }
                    let num_components = stores.num_components(param);
                    let ty = types.F32[num_components as usize];
                    let id = self.DefineOutput(ty, Some(index as u32), None);
                    self.Name(id, &format!("out_attr{index}"));
                    self.output_params[index] = self.GetAttributeInfo(
                        NumberFormat::Float,
                        id,
                        num_components,
                        true,
                        false,
                        false,
                    );
                }
            }
            SwStage::Fragment => self.DefineFragmentOutputs(),
            SwStage::Geometry => {
                self.DefineVertexBlock();
                let num_attrs = self.info.borrow().gs_copy_data.num_attrs;
                for attr_id in 0..num_attrs {
                    let id = self.DefineOutput(types.F32[4], Some(attr_id), None);
                    self.Name(id, &format!("out_attr{attr_id}"));
                    self.output_params[attr_id as usize] =
                        self.GetAttributeInfo(NumberFormat::Float, id, 4, true, false, false);
                }
            }
            SwStage::Compute => {}
            _ => unreachable!("invalid logical stage"),
        }
    }

    fn DefineFragmentOutputs(&mut self) {
        let types = self.type_ids();
        let fs = match self.runtime_info.hw {
            HwInfo::Fs(fs) => fs,
            _ => unreachable!(),
        };
        let stores = self.info.borrow().stores;
        if stores.get(Attribute::Depth, 0) {
            self.frag_depth = self.DefineVariable(
                types.F32[1],
                Some(spv::BuiltIn::FragDepth),
                spv::StorageClass::Output,
                None,
            );
        }
        if stores.get(Attribute::SampleMask, 0) {
            let u32_ty = types.U32[1];
            let one = self.ConstU32(1);
            let ty = self.TypeArray(u32_ty, one);
            self.sample_mask = self.DefineVariable(
                ty,
                Some(spv::BuiltIn::SampleMask),
                spv::StorageClass::Output,
                None,
            );
        }
        if stores.get(Attribute::StencilRef, 0) && self.profile.supports_shader_stencil_export {
            self.stencil_ref = self.DefineVariable(
                types.S32[1],
                Some(spv::BuiltIn::FragStencilRefEXT),
                spv::StorageClass::Output,
                None,
            );
        }
        let mut num_render_targets = 0u32;
        for index in 0..NUM_RENDER_TARGETS {
            let mrt = Attribute::from_raw(Attribute::RenderTarget0 as u64 + index as u64).unwrap();
            if !stores.get_any(mrt) {
                continue;
            }
            let num_components = stores.num_components(mrt);
            let num_format = fs.color_buffers[index].num_format;
            let ty = self.attribute_type(num_format)[num_components as usize];
            let id = if fs.dual_source_blending {
                let id = self.DefineOutput(ty, Some(0), None);
                self.Decorate(id, spv::Decoration::Index, &[index as u32]);
                id
            } else {
                self.DefineOutput(ty, Some(index as u32), None)
            };
            self.Name(id, &format!("frag_color{index}"));
            self.frag_outputs[index] =
                self.GetAttributeInfo(num_format, id, num_components, true, false, false);
            num_render_targets += 1;
        }
        if self.FragmentMotion() && !ValidId(self.frag_outputs[MotionVectors::OUTPUT as usize].id())
        {
            self.motion_frag_out =
                self.DefineOutput(types.F32[4], Some(MotionVectors::OUTPUT), None);
            self.motion_in_cur =
                self.DefineInput(types.F32[4], Some(MotionVectors::CURRENT_LOCATION), None);
            self.motion_in_prev =
                self.DefineInput(types.F32[4], Some(MotionVectors::PREVIOUS_LOCATION), None);
            let id = types.motion_frag_out;
            self.Name(id, "motion_vector");
        }
        assert!(
            !fs.dual_source_blending || num_render_targets <= 2,
            "Dual source blending enabled, there must be at most two MRT exports"
        );
    }

    fn DefinePushDataBlock(&mut self) {
        let types = self.type_ids();
        let f32x1 = types.F32[1];
        let u32x1 = types.U32[1];
        let u32x2 = types.U32[2];
        let u32x4 = types.U32[4];
        let struct_type = self.TypeStruct(&[
            f32x1, f32x1, f32x1, f32x1, u32x4, u32x4, u32x4, u32x4, u32x4, u32x4, u32x2, u32x1,
            u32x1,
        ]);
        self.Name(struct_type, "AuxData");
        self.Decorate(struct_type, spv::Decoration::Block, &[]);
        self.MemberName(struct_type, PushData::X_OFFSET_INDEX, "xoffset");
        self.MemberName(struct_type, PushData::Y_OFFSET_INDEX, "yoffset");
        self.MemberName(struct_type, PushData::X_SCALE_INDEX, "xscale");
        self.MemberName(struct_type, PushData::Y_SCALE_INDEX, "yscale");
        for index in 0..4 {
            self.MemberName(
                struct_type,
                PushData::UD_REGS_INDEX + index,
                &format!("ud_regs{index}"),
            );
        }
        for index in 0..3 {
            self.MemberName(
                struct_type,
                PushData::BUF_OFFSET_INDEX + index,
                &format!("buf_offsets{index}"),
            );
        }
        self.MemberDecorate(
            struct_type,
            PushData::X_OFFSET_INDEX,
            spv::Decoration::Offset,
            &[0],
        );
        self.MemberDecorate(
            struct_type,
            PushData::Y_OFFSET_INDEX,
            spv::Decoration::Offset,
            &[4],
        );
        self.MemberDecorate(
            struct_type,
            PushData::X_SCALE_INDEX,
            spv::Decoration::Offset,
            &[8],
        );
        self.MemberDecorate(
            struct_type,
            PushData::Y_SCALE_INDEX,
            spv::Decoration::Offset,
            &[12],
        );
        self.MemberDecorate(
            struct_type,
            PushData::UD_REGS_INDEX,
            spv::Decoration::Offset,
            &[16],
        );
        self.MemberDecorate(
            struct_type,
            PushData::UD_REGS_INDEX + 1,
            spv::Decoration::Offset,
            &[32],
        );
        self.MemberDecorate(
            struct_type,
            PushData::UD_REGS_INDEX + 2,
            spv::Decoration::Offset,
            &[48],
        );
        self.MemberDecorate(
            struct_type,
            PushData::UD_REGS_INDEX + 3,
            spv::Decoration::Offset,
            &[64],
        );
        self.MemberDecorate(
            struct_type,
            PushData::BUF_OFFSET_INDEX,
            spv::Decoration::Offset,
            &[80],
        );
        self.MemberDecorate(
            struct_type,
            PushData::BUF_OFFSET_INDEX + 1,
            spv::Decoration::Offset,
            &[96],
        );
        self.MemberDecorate(
            struct_type,
            PushData::BUF_OFFSET_INDEX + 2,
            spv::Decoration::Offset,
            &[112],
        );
        self.MemberName(struct_type, PushData::MOTION_PARAM_INDEX, "motion_param");
        self.MemberDecorate(
            struct_type,
            PushData::MOTION_PARAM_INDEX,
            spv::Decoration::Offset,
            &[120],
        );
        self.MemberName(struct_type, PushData::SCENE_SIZE_INDEX, "scene_size");
        self.MemberDecorate(
            struct_type,
            PushData::SCENE_SIZE_INDEX,
            spv::Decoration::Offset,
            &[124],
        );
        self.push_data_block = self.DefineVar(struct_type, spv::StorageClass::PushConstant, None);
        self.Name(types.push_data_block, "push_data");
        self.interfaces.push(types.push_data_block);
    }

    fn DefineBuffer(
        &mut self,
        is_written: bool,
        is_coherent: bool,
        elem_shift: u32,
        buffer_type: BufferType,
        data_type: Id,
    ) -> BufferSpv {
        let record_array_type = self.TypeRuntimeArray(data_type);
        let struct_type = self.TypeStruct(&[record_array_type]);
        if !self.buf_type_ids.contains(&record_array_type) {
            self.Decorate(
                record_array_type,
                spv::Decoration::ArrayStride,
                &[1 << elem_shift],
            );
            self.Decorate(struct_type, spv::Decoration::Block, &[]);
            self.MemberName(struct_type, 0, "data");
            self.MemberDecorate(struct_type, 0, spv::Decoration::Offset, &[0]);
            self.buf_type_ids.push(record_array_type);
        }
        const STORAGE_CLASS: u32 = spv::StorageClass::StorageBuffer;
        let struct_pointer_type = self.TypePointer(STORAGE_CLASS, struct_type);
        let pointer_type = self.TypePointer(STORAGE_CLASS, data_type);
        let id = self.AddGlobalVariable(struct_pointer_type, STORAGE_CLASS, None);
        let binding = self.binding.unified;
        self.Decorate(id, spv::Decoration::Binding, &[binding]);
        self.Decorate(id, spv::Decoration::DescriptorSet, &[0]);
        if !is_written {
            self.Decorate(id, spv::Decoration::NonWritable, &[]);
        } else if is_coherent {
            self.Decorate(id, spv::Decoration::Coherent, &[]);
        }
        match buffer_type {
            BufferType::GdsBuffer => self.Name(id, "gds_buffer"),
            BufferType::Flatbuf => self.Name(id, "srt_flatbuf"),
            BufferType::ClipPlanes => self.Name(id, "clip_planes"),
            BufferType::BdaPagetable => self.Name(id, "bda_pagetable"),
            BufferType::FaultBuffer => self.Name(id, "fault_buffer"),
            BufferType::SharedMemory => self.Name(id, "ssbo_shmem"),
            BufferType::Guest => {
                let buffer = self.binding.buffer;
                self.Name(id, &format!("ssbo_{buffer}"))
            }
        };
        self.interfaces.push(id);
        BufferSpv { id, pointer_type }
    }

    fn DefineBuffers(&mut self) {
        let types = self.type_ids();
        let descriptions = self.info.borrow().persistent.buffers.clone();
        for desc in &descriptions {
            let sharp = desc.get_sharp(&self.info.borrow());
            let is_coherent = desc.buffer_type == BufferType::SharedMemory || sharp.mtype() == 3;
            match desc.buffer_type {
                BufferType::Flatbuf => self.flatbuf_index = self.buffers.len(),
                BufferType::BdaPagetable => self.bda_pagetable_index = self.buffers.len(),
                BufferType::FaultBuffer => self.fault_buffer_index = self.buffers.len(),
                _ => {}
            }
            let binding = self.binding.buffer;
            let buffer_type = desc.buffer_type;
            self.buffers.push(BufferDefinition {
                binding,
                buffer_type,
                ..BufferDefinition::default()
            });
            self.binding.buffer += 1;
            let index = self.buffers.len() - 1;
            if desc.used_types.contains(Type::U64) {
                let u64_type = types.U64;
                let alias =
                    self.DefineBuffer(desc.is_written, is_coherent, 3, buffer_type, u64_type);
                self.buffers[index].set_alias(PointerType::U64, alias);
            }
            if desc.used_types.contains(Type::U32) {
                let u32_type = types.U32[1];
                let alias =
                    self.DefineBuffer(desc.is_written, is_coherent, 2, buffer_type, u32_type);
                self.buffers[index].set_alias(PointerType::U32, alias);
            }
            if desc.used_types.contains(Type::F32) {
                let f32_type = types.F32[1];
                let alias =
                    self.DefineBuffer(desc.is_written, is_coherent, 2, buffer_type, f32_type);
                self.buffers[index].set_alias(PointerType::F32, alias);
            }
            if desc.used_types.contains(Type::U16) {
                let u16_type = types.U16;
                let alias =
                    self.DefineBuffer(desc.is_written, is_coherent, 1, buffer_type, u16_type);
                self.buffers[index].set_alias(PointerType::U16, alias);
            }
            if desc.used_types.contains(Type::U8) {
                let u8_type = types.U8;
                let alias =
                    self.DefineBuffer(desc.is_written, is_coherent, 0, buffer_type, u8_type);
                self.buffers[index].set_alias(PointerType::U8, alias);
            }
            self.binding.unified += 1;
        }
    }

    fn DefineImagesAndSamplers(&mut self) {
        let types = self.type_ids();
        let images = self.info.borrow().persistent.images.clone();
        for image_desc in &images {
            let sharp = image_desc.get_sharp(&self.info.borrow());
            let nfmt = sharp.get_number_fmt();
            let is_integer = is_integer(nfmt);
            let is_storage = image_desc.is_written;
            let mip_fallback_mode = image_desc.mip_fallback_mode;
            let data_types = self.attribute_type(nfmt);
            let sampled_type = data_types[1];
            let image_type = self.image_type(image_desc, sampled_type);
            let num_bindings = image_desc.num_bindings(&self.info.borrow());
            let pointee_type = if mip_fallback_mode == MipStorageFallbackMode::DynamicIndex {
                self.TypeArrayLen(image_type, num_bindings)
            } else {
                image_type
            };
            let pointer_type = self.TypePointer(spv::StorageClass::UniformConstant, pointee_type);
            let id = self.AddGlobalVariable(pointer_type, spv::StorageClass::UniformConstant, None);
            let binding = self.binding.unified;
            self.Decorate(id, spv::Decoration::Binding, &[binding]);
            self.binding.unified += num_bindings;
            self.Decorate(id, spv::Decoration::DescriptorSet, &[0]);
            let stage = stage_name(self.hw_stage);
            let index = self.images.len();
            self.Name(id, &format!("{stage}_img{index}"));
            let sampled_image_type = if is_storage {
                sampled_type
            } else {
                self.TypeSampledImage(image_type)
            };
            self.images.push(TextureDefinition {
                data_types,
                id,
                sampled_type: sampled_image_type,
                image_type,
                view_type: sharp.get_view_type(image_desc.is_array),
                is_integer,
                is_storage,
                mip_fallback_mode,
            });
            self.interfaces.push(id);
        }
        if self
            .info
            .borrow()
            .persistent
            .images
            .iter()
            .any(|image| image.is_atomic)
        {
            let u32_type = types.U32[1];
            self.image_u32 = self.TypePointer(spv::StorageClass::Image, u32_type);
            let f32_type = types.F32[1];
            self.image_f32 = self.TypePointer(spv::StorageClass::Image, f32_type);
        }
        if self.info.borrow().persistent.samplers.is_empty() {
            return;
        }
        self.sampler_type = self.TypeSampler();
        self.sampler_pointer_type =
            self.TypePointer(spv::StorageClass::UniformConstant, types.sampler_type);
        let samplers = self.info.borrow().persistent.samplers.clone();
        for _ in &samplers {
            let id = self.AddGlobalVariable(
                types.sampler_pointer_type,
                spv::StorageClass::UniformConstant,
                None,
            );
            let binding = self.binding.unified;
            self.binding.unified += 1;
            self.Decorate(id, spv::Decoration::Binding, &[binding]);
            self.Decorate(id, spv::Decoration::DescriptorSet, &[0]);
            let stage = stage_name(self.hw_stage);
            let index = self.samplers.len();
            self.Name(id, &format!("{stage}_samp{index}"));
            self.samplers.push(id);
            self.interfaces.push(id);
        }
    }

    fn image_type(&mut self, desc: &ImageResource, sampled_type: Id) -> Id {
        let image = desc.get_sharp(&self.info.borrow());
        let format = if desc.is_atomic {
            image_format(&image)
        } else {
            spv::ImageFormat::Unknown
        };
        let ty = image.get_view_type(desc.is_array);
        let sampled = if desc.is_written { 2 } else { 1 };
        match ty {
            ImageType::Color1D => self.TypeImage(
                sampled_type,
                spv::Dim::Dim1D,
                0,
                false,
                false,
                sampled,
                format,
            ),
            ImageType::Color1DArray => self.TypeImage(
                sampled_type,
                spv::Dim::Dim1D,
                0,
                true,
                false,
                sampled,
                format,
            ),
            ImageType::Color2D => self.TypeImage(
                sampled_type,
                spv::Dim::Dim2D,
                0,
                false,
                false,
                sampled,
                format,
            ),
            ImageType::Color2DArray => self.TypeImage(
                sampled_type,
                spv::Dim::Dim2D,
                0,
                true,
                false,
                sampled,
                format,
            ),
            ImageType::Color2DMsaa => self.TypeImage(
                sampled_type,
                spv::Dim::Dim2D,
                0,
                false,
                true,
                sampled,
                format,
            ),
            ImageType::Color3D => self.TypeImage(
                sampled_type,
                spv::Dim::Dim3D,
                0,
                false,
                false,
                sampled,
                format,
            ),
            other => unreachable!("invalid texture type {other:?}"),
        }
    }

    fn DefineSharedMemory(&mut self) {
        let types = self.type_ids();
        let shared_types = self.info.borrow().shared_types;
        let num_types = shared_types.bits().count_ones();
        if num_types == 0 {
            return;
        }
        assert_eq!(self.hw_stage, HwStage::Compute);
        let cs = match self.runtime_info.hw {
            HwInfo::Cs(cs) => cs,
            _ => unreachable!("shared memory belongs to a compute shader"),
        };
        let shared_memory_size =
            cs.shared_memory_size + self.info.borrow().persistent.shared_memory_scratch_size;
        let u16_type = types.U16;
        let (variable, pointer, element_pointer) = self.define_shared_type(
            Type::U16,
            u16_type,
            2,
            "shared_mem_u16",
            shared_memory_size,
            num_types,
        );
        self.shared_memory_u16 = variable;
        self.shared_u16 = pointer;
        self.shared_memory_u16_type = element_pointer;
        let u32_type = types.U32[1];
        let (variable, pointer, element_pointer) = self.define_shared_type(
            Type::U32,
            u32_type,
            4,
            "shared_mem_u32",
            shared_memory_size,
            num_types,
        );
        self.shared_memory_u32 = variable;
        self.shared_u32 = pointer;
        self.shared_memory_u32_type = element_pointer;
        let u64_type = types.U64;
        let (variable, pointer, element_pointer) = self.define_shared_type(
            Type::U64,
            u64_type,
            8,
            "shared_mem_u64",
            shared_memory_size,
            num_types,
        );
        self.shared_memory_u64 = variable;
        self.shared_u64 = pointer;
        self.shared_memory_u64_type = element_pointer;
    }

    #[allow(clippy::too_many_arguments)]
    fn define_shared_type(
        &mut self,
        ty: Type,
        element_type: Id,
        element_size: u32,
        name: &str,
        shared_memory_size: u32,
        num_types: u32,
    ) -> (Id, Id, Id) {
        if !self.info.borrow().shared_types.contains(ty) {
            return (0, 0, 0);
        }
        let num_elements = div_ceil_u32(shared_memory_size, element_size);
        let array_type = self.TypeArrayLen(element_type, num_elements);
        let mem_type = if num_types > 1 {
            let struct_type = self.TypeStruct(&[array_type]);
            self.Decorate(struct_type, spv::Decoration::Block, &[]);
            self.MemberDecorate(struct_type, 0, spv::Decoration::Offset, &[0]);
            struct_type
        } else {
            array_type
        };
        let pointer = self.TypePointer(spv::StorageClass::Workgroup, mem_type);
        let element_pointer = self.TypePointer(spv::StorageClass::Workgroup, element_type);
        let variable = self.AddGlobalVariable(pointer, spv::StorageClass::Workgroup, None);
        self.Name(variable, name);
        self.interfaces.push(variable);
        if num_types > 1 {
            self.Decorate(array_type, spv::Decoration::ArrayStride, &[element_size]);
            self.Decorate(variable, spv::Decoration::Aliased, &[]);
        }
        (variable, pointer, element_pointer)
    }

    fn define_float32_to_ufloat_m5(&mut self, mantissa_bits: u32, name: &str) -> Id {
        let types = self.type_ids();
        let u32_ty = types.U32[1];
        let f32_ty = types.F32[1];
        let s32_ty = types.S32[1];
        let func_type = self.TypeFunction(u32_ty, &[f32_ty]);
        let func = self.OpFunction(u32_ty, spv::FunctionControlMask::MaskNone, func_type);
        let value = self.OpFunctionParameter(f32_ty);
        self.Name(func, name);
        self.AddLabel();

        let raw_value = self.OpBitcast(u32_ty, value);
        let offset = self.ConstU32(23);
        let eight = self.ConstU32(8);
        let exponent_bits = self.OpBitFieldSExtract(u32_ty, raw_value, offset, eight);
        let exponent = self.OpBitcast(s32_ty, exponent_bits);
        let offset = self.ConstU32(31);
        let one = self.ConstU32(1);
        let sign = self.OpBitFieldUExtract(u32_ty, raw_value, offset, one);

        let u1 = types.U1[1];
        let zero = self.ConstU32(0);
        let raw_zero = self.OpIEqual(u1, raw_value, zero);
        let one_value = self.ConstU32(1);
        let sign_one = self.OpIEqual(u1, sign, one_value);
        let is_zero = self.OpLogicalOr(u1, raw_zero, sign_one);
        let is_nan = self.OpIsNan(u1, value);
        let is_inf = self.OpIsInf(u1, value);
        let minus_15 = self.ConstS32(-15);
        let is_denorm = self.OpSLessThanEqual(u1, exponent, minus_15);

        let scale = self.ConstF32((1u32 << (mantissa_bits + 14)) as f32);
        let product = self.OpFMul(f32_ty, value, scale);
        let rounded = self.OpRoundEven(f32_ty, product);
        let denorm_mantissa = self.OpConvertFToU(u32_ty, rounded);
        let shift = self.ConstU32(mantissa_bits);
        let shifted = self.OpShiftRightLogical(u32_ty, denorm_mantissa, shift);
        let zero = self.ConstU32(0);
        let denorm_overflow = self.OpINotEqual(u1, shifted, zero);
        let denorm_constant = self.ConstU32(1u32 << mantissa_bits);
        let denorm = self.OpSelect(u32_ty, denorm_overflow, denorm_constant, denorm_mantissa);

        let mantissa_constant = self.ConstS32(mantissa_bits as i32);
        let exponent_delta = self.OpISub(s32_ty, mantissa_constant, exponent);
        let value_ldexp = self.OpLdexp(f32_ty, value, exponent_delta);
        let rounded = self.OpRoundEven(f32_ty, value_ldexp);
        let norm_mantissa = self.OpConvertFToU(u32_ty, rounded);
        let limit = self.ConstU32(2u32 << mantissa_bits);
        let norm_overflow = self.OpUGreaterThanEqual(u1, norm_mantissa, limit);
        let one = self.ConstU32(1);
        let shifted = self.OpShiftRightLogical(u32_ty, norm_mantissa, one);
        let selected = self.OpSelect(u32_ty, norm_overflow, shifted, norm_mantissa);
        let mask = self.ConstU32((1u32 << mantissa_bits) - 1);
        let norm_final_mantissa = self.OpBitwiseAnd(u32_ty, selected, mask);
        let one = self.ConstS32(1);
        let exponent_plus_one = self.OpIAdd(s32_ty, exponent, one);
        let selected_exponent = self.OpSelect(s32_ty, norm_overflow, exponent_plus_one, exponent);
        let fifteen = self.ConstS32(15);
        let exponent_bits = self.OpIAdd(s32_ty, selected_exponent, fifteen);
        let norm_final_exponent = self.OpBitcast(u32_ty, exponent_bits);
        let offset = self.ConstU32(mantissa_bits);
        let five = self.ConstU32(5);
        let norm = self.OpBitFieldInsert(
            u32_ty,
            norm_final_mantissa,
            norm_final_exponent,
            offset,
            five,
        );

        let zero = self.ConstU32(0);
        let nan_value = self.ConstU32((31u32 << mantissa_bits) | 1);
        let inf_value = self.ConstU32(31u32 << mantissa_bits);
        let special = self.OpSelect(u32_ty, is_denorm, denorm, norm);
        let special = self.OpSelect(u32_ty, is_inf, inf_value, special);
        let special = self.OpSelect(u32_ty, is_nan, nan_value, special);
        let result = self.OpSelect(u32_ty, is_zero, zero, special);
        self.OpReturnValue(result);
        self.OpFunctionEnd();
        func
    }

    fn define_ufloat_m5_to_float32(&mut self, mantissa_bits: u32, name: &str) -> Id {
        let types = self.type_ids();
        let u32_ty = types.U32[1];
        let f32_ty = types.F32[1];
        let func_type = self.TypeFunction(f32_ty, &[u32_ty]);
        let func = self.OpFunction(f32_ty, spv::FunctionControlMask::MaskNone, func_type);
        let value = self.OpFunctionParameter(u32_ty);
        self.Name(func, name);
        self.AddLabel();

        let offset = self.ConstU32(mantissa_bits);
        let five = self.ConstU32(5);
        let exponent = self.OpBitFieldUExtract(u32_ty, value, offset, five);
        let offset = self.ConstU32(0);
        let mantissa_bits_constant = self.ConstU32(mantissa_bits);
        let mantissa = self.OpBitFieldUExtract(u32_ty, value, offset, mantissa_bits_constant);
        let mantissa_f = self.OpConvertUToF(f32_ty, mantissa);
        let u1 = types.U1[1];
        let zero = types.u32_zero_value;
        let mantissa_nonzero = self.OpINotEqual(u1, mantissa, zero);
        let scale = self.ConstF32(1.0 / (1u32 << (14 + mantissa_bits)) as f32);
        let scaled = self.OpFMul(f32_ty, scale, mantissa_f);
        let a = self.OpSelect(f32_ty, mantissa_nonzero, scaled, types.f32_zero_value);
        let sign_bits = self.ConstU32(0x7f80_0000);
        let bits = self.OpBitwiseOr(u32_ty, mantissa, sign_bits);
        let b = self.OpBitcast(f32_ty, bits);
        let fifteen = self.ConstU32(15);
        let exponent_c = self.OpISub(u32_ty, exponent, fifteen);
        let one = types.u32_one_value;
        let negated = self.OpSNegate(u32_ty, exponent_c);
        let shifted = self.OpShiftLeftLogical(u32_ty, one, negated);
        let denominator = self.OpConvertUToF(f32_ty, shifted);
        let one_float = self.ConstF32(1.0);
        let scale_a = self.OpFDiv(f32_ty, one_float, denominator);
        let shifted = self.OpShiftLeftLogical(u32_ty, one, exponent_c);
        let scale_b = self.OpConvertUToF(f32_ty, shifted);
        let negative = self.OpSLessThan(u1, exponent_c, types.u32_zero_value);
        let scale = self.OpSelect(f32_ty, negative, scale_a, scale_b);
        let divisor = self.ConstF32((1u32 << mantissa_bits) as f32);
        let quotient = self.OpFDiv(f32_ty, mantissa_f, divisor);
        let one_float = self.ConstF32(1.0);
        let sum = self.OpFAdd(f32_ty, one_float, quotient);
        let c = self.OpFMul(f32_ty, scale, sum);
        let zero = types.u32_zero_value;
        let exponent_zero = self.OpIEqual(u1, exponent, zero);
        let thirty_one = self.ConstU32(31);
        let exponent_31 = self.OpIEqual(u1, exponent, thirty_one);
        let special = self.OpSelect(f32_ty, exponent_31, b, c);
        let result = self.OpSelect(f32_ty, exponent_zero, a, special);
        self.OpReturnValue(result);
        self.OpFunctionEnd();
        func
    }

    fn define_get_bda_pointer(&mut self) -> Id {
        let types = self.type_ids();
        let u64_ty = types.U64;
        let page_shift_bits = self.profile.sparse_page_shift;
        let page_shift = self.Constant(u64_ty, u64::from(page_shift_bits));
        let page_mask = self.Constant(u64_ty, u64::from((1u32 << page_shift_bits) - 1));
        let func_type = self.TypeFunction(u64_ty, &[u64_ty]);
        let func = self.OpFunction(u64_ty, spv::FunctionControlMask::MaskNone, func_type);
        let address = self.OpFunctionParameter(u64_ty);
        self.Name(func, "get_bda_pointer");
        self.AddLabel();

        let fault_label = self.OpLabel();
        let available_label = self.OpLabel();
        let merge_label = self.OpLabel();

        let page = self.OpShiftRightLogical(u64_ty, address, page_shift);
        let u32_ty = types.U32[1];
        let page32 = self.OpUConvert(u32_ty, page);
        let bda_buffer = self.buffers[self.bda_pagetable_index].clone();
        let alias = bda_buffer.alias(PointerType::U64);
        let zero = types.u32_zero_value;
        let bda_ptr = self.OpAccessChain(alias.pointer_type, alias.id, &[zero, page32]);
        let bda = self.OpLoad(u64_ty, bda_ptr, &[]);

        let u1 = types.U1[1];
        let zero = types.u64_zero_value;
        let is_fault = self.OpIEqual(u1, bda, zero);
        self.OpSelectionMerge(merge_label, spv::SelectionControlMask::MaskNone);
        self.OpBranchConditional(is_fault, fault_label, available_label);

        self.AddExistingLabel(fault_label);
        let fault_buffer = self.buffers[self.fault_buffer_index].clone();
        let alias = fault_buffer.alias(PointerType::U32);
        let five = self.ConstU32(5);
        let page_div32 = self.OpShiftRightLogical(u32_ty, page32, five);
        let thirty_one = self.ConstU32(31);
        let page_mod32 = self.OpBitwiseAnd(u32_ty, page32, thirty_one);
        let one = types.u32_one_value;
        let page_mask_value = self.OpShiftLeftLogical(u32_ty, one, page_mod32);
        let zero = types.u32_zero_value;
        let fault_ptr = self.OpAccessChain(alias.pointer_type, alias.id, &[zero, page_div32]);
        let fault_value = self.OpLoad(u32_ty, fault_ptr, &[]);
        let fault_value_masked = self.OpBitwiseOr(u32_ty, fault_value, page_mask_value);
        self.OpStore(fault_ptr, fault_value_masked, &[]);
        let fallback_result = types.u64_zero_value;
        self.OpBranch(merge_label);

        self.AddExistingLabel(available_label);
        let offset_in_bda = self.OpBitwiseAnd(u64_ty, address, page_mask);
        let addr = self.OpIAdd(u64_ty, bda, offset_in_bda);
        self.OpBranch(merge_label);

        self.AddExistingLabel(merge_label);
        let result = self.OpPhi(
            u64_ty,
            &[addr, available_label, fallback_result, fault_label],
        );
        self.OpReturnValue(result);
        self.OpFunctionEnd();
        func
    }

    fn define_read_const(&mut self, dynamic: bool) -> Id {
        let types = self.type_ids();
        let u32_ty = types.U32[1];
        let u32x2 = types.U32[2];
        let func_type = if !dynamic {
            self.TypeFunction(u32_ty, &[u32x2, u32_ty, u32_ty])
        } else {
            self.TypeFunction(u32_ty, &[u32x2, u32_ty])
        };
        let func = self.OpFunction(u32_ty, spv::FunctionControlMask::MaskNone, func_type);
        let base = self.OpFunctionParameter(u32x2);
        let offset = self.OpFunctionParameter(u32_ty);
        let flatbuf_offset = if !dynamic {
            self.OpFunctionParameter(u32_ty)
        } else {
            0
        };
        self.Name(
            func,
            if dynamic {
                "read_const_dynamic"
            } else {
                "read_const"
            },
        );
        self.AddLabel();

        let u64_ty = types.U64;
        let index = self.OpCompositeExtract(u32_ty, base, &[0]);
        let base_lo = self.OpUConvert(u64_ty, index);
        let index = self.OpCompositeExtract(u32_ty, base, &[1]);
        let base_hi = self.OpUConvert(u64_ty, index);
        let thirty_two = self.ConstU32(32);
        let base_shift = self.OpShiftLeftLogical(u64_ty, base_hi, thirty_two);
        let base_addr = self.OpBitwiseOr(u64_ty, base_lo, base_shift);
        let two = self.ConstU32(2);
        let offset_bytes = self.OpShiftLeftLogical(u32_ty, offset, two);
        let offset64 = self.OpUConvert(u64_ty, offset_bytes);
        let addr = self.OpIAdd(u64_ty, base_addr, offset64);

        let result = if dynamic {
            let zero = types.u32_zero_value;
            self.EmitDwordMemoryRead(addr, |_| zero)
        } else {
            self.EmitDwordMemoryRead(addr, |ctx| ctx.EmitFlatbufferLoad(flatbuf_offset))
        };
        self.OpReturnValue(result);
        self.OpFunctionEnd();
        func
    }

    fn DefineFunctions(&mut self) {
        if self.info.borrow().uses_pack_10_11_11 {
            self.f32_to_uf11 = self.define_float32_to_ufloat_m5(6, "f32_to_uf11");
            self.f32_to_uf10 = self.define_float32_to_ufloat_m5(5, "f32_to_uf10");
        }
        if self.info.borrow().uses_unpack_10_11_11 {
            self.uf11_to_f32 = self.define_ufloat_m5_to_float32(6, "uf11_to_f32");
            self.uf10_to_f32 = self.define_ufloat_m5_to_float32(5, "uf10_to_f32");
        }
        if self.info.borrow().persistent.uses_dma {
            self.get_bda_pointer = self.define_get_bda_pointer();
        }
        let readconst_types = self.info.borrow().readconst_types;
        if readconst_types.has(ReadConstType::IMMEDIATE) {
            self.read_const = self.define_read_const(false);
        }
        if readconst_types.has(ReadConstType::DYNAMIC) {
            self.read_const_dynamic = self.define_read_const(true);
        }
    }

    pub(crate) fn VertexMotion(&self) -> bool {
        self.sw_stage == SwStage::Vertex
            && self.hw_stage == HwStage::Vertex
            && match &self.runtime_info.hw {
                HwInfo::Vs(vs) => vs.motion_vectors
                    && crate::gpu::shader_recompiler::runtime_info::MOTION_VECTOR_POSITIONS_ADDRESS
                        .load(Ordering::Relaxed)
                        != 0,
                _ => false,
            }
    }

    pub(crate) fn FragmentMotion(&self) -> bool {
        self.sw_stage == SwStage::Fragment
            && match &self.runtime_info.hw {
                HwInfo::Fs(fs) => fs.motion_vectors,
                _ => false,
            }
    }
}

fn image_format(image: &Image) -> u32 {
    use crate::gpu::amdgpu::pixel_format::{DataFormat, NumberFormat};
    let data_format = image.get_data_fmt();
    let number_format = image.get_number_fmt();
    match (data_format, number_format) {
        (DataFormat::Format32, NumberFormat::Uint) => spv::ImageFormat::R32ui,
        (DataFormat::Format32, NumberFormat::Sint) => spv::ImageFormat::R32i,
        (DataFormat::Format32, NumberFormat::Float) => spv::ImageFormat::R32f,
        (DataFormat::Format32_32, NumberFormat::Float) => spv::ImageFormat::Rg32f,
        (DataFormat::Format32_32, NumberFormat::Uint) => spv::ImageFormat::Rg32ui,
        (DataFormat::Format32_32_32_32, NumberFormat::Uint) => spv::ImageFormat::Rgba32ui,
        (DataFormat::Format16, NumberFormat::Float) => spv::ImageFormat::R16f,
        (DataFormat::Format16, NumberFormat::Uint) => spv::ImageFormat::R16ui,
        (DataFormat::Format16_16, NumberFormat::Float) => spv::ImageFormat::Rg16f,
        (DataFormat::Format16_16, NumberFormat::Snorm) => spv::ImageFormat::Rg16Snorm,
        (DataFormat::Format8_8, NumberFormat::Unorm) => spv::ImageFormat::Rg8,
        (DataFormat::Format16_16_16_16, NumberFormat::Float) => spv::ImageFormat::Rgba16f,
        (DataFormat::Format16_16_16_16, NumberFormat::Unorm) => spv::ImageFormat::Rgba16,
        (DataFormat::Format8, NumberFormat::Unorm) => spv::ImageFormat::R8,
        (DataFormat::Format8_8_8_8, NumberFormat::Unorm) => spv::ImageFormat::Rgba8,
        (DataFormat::Format8_8_8_8, NumberFormat::Uint) => spv::ImageFormat::Rgba8ui,
        (DataFormat::Format10_11_11, NumberFormat::Float) => spv::ImageFormat::R11fG11fB10f,
        (DataFormat::Format32_32_32_32, NumberFormat::Float) => spv::ImageFormat::Rgba32f,
        _ => unreachable!(
            "Unknown storage format data_format={data_format:?}, num_format={number_format:?}"
        ),
    }
}

#[cfg(test)]
pub(crate) fn test_fragment_context<'a>(
    binding: &'a mut Bindings,
    read: &'a dyn Fn(u64) -> u32,
) -> EmitContext<'a> {
    use crate::gpu::shader_recompiler::params::ShaderParams;
    let info = Rc::new(RefCell::new(Info::new(
        HwStage::Fragment,
        SwStage::Fragment,
        ShaderParams::new([0; 16], &[], 0),
    )));
    let runtime = RuntimeInfo::new(HwStage::Fragment, SwStage::Fragment);
    EmitContext::new(&Profile::default(), &runtime, info, binding, &[], read)
}

#[cfg(test)]
pub(crate) fn test_vertex_context<'a>(
    binding: &'a mut Bindings,
    read: &'a dyn Fn(u64) -> u32,
) -> EmitContext<'a> {
    use crate::gpu::shader_recompiler::params::ShaderParams;
    let info = Rc::new(RefCell::new(Info::new(
        HwStage::Vertex,
        SwStage::Vertex,
        ShaderParams::new([0; 16], &[], 0),
    )));
    let runtime = RuntimeInfo::new(HwStage::Vertex, SwStage::Vertex);
    EmitContext::new(&Profile::default(), &runtime, info, binding, &[], read)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::ir::value::Value;
    use crate::gpu::shader_recompiler::params::ShaderParams;

    fn make_info(hw: HwStage, sw: SwStage) -> Rc<RefCell<Info>> {
        Rc::new(RefCell::new(Info::new(
            hw,
            sw,
            ShaderParams::new([0; 16], &[], 0x1234),
        )))
    }

    fn opcode(word: u32) -> u32 {
        word & 0xffff
    }

    #[test]
    fn a_fragment_context_defines_its_arithmetic_types_and_defs_immediates() {
        let info = make_info(HwStage::Fragment, SwStage::Fragment);
        let runtime = RuntimeInfo::new(HwStage::Fragment, SwStage::Fragment);
        let mut binding = Bindings::default();
        let read = |_| 0u32;
        let mut ctx = EmitContext::new(
            &Profile::default(),
            &runtime,
            info,
            &mut binding,
            &[],
            &read,
        );

        assert_ne!(ctx.U32[1], 0);
        assert_ne!(ctx.F32[1], 0);
        assert_ne!(ctx.void_id, 0);
        let seven = Value::from_u32(7);
        let def = ctx.Def(&seven);
        assert_eq!(def, ctx.ConstU32(7), "an immediate maps to its constant");
        let float = Value::from_f32(0.5);
        assert_eq!(ctx.Def(&float), ctx.ConstF32(0.5));

        let words = ctx.Assemble();
        assert_eq!(
            words[0],
            crate::gpu::shader_recompiler::backend::spirv::builder::MAGIC_NUMBER
        );
        assert!(words
            .iter()
            .any(|word| opcode(*word) == spv::Op::Capability && *word >> 16 == 2));
        assert!(words.iter().any(|word| opcode(*word) == spv::Op::TypeInt));
    }

    #[test]
    fn a_compute_context_with_shared_memory_defines_the_array() {
        let info = make_info(HwStage::Compute, SwStage::Compute);
        info.borrow_mut().shared_types = Type::U32;
        let runtime = RuntimeInfo::new(HwStage::Compute, SwStage::Compute);
        let mut binding = Bindings::default();
        let read = |_| 0u32;
        let mut ctx = EmitContext::new(
            &Profile::default(),
            &runtime,
            info,
            &mut binding,
            &[],
            &read,
        );
        assert_ne!(ctx.shared_memory_u32, 0);
        assert_ne!(ctx.shared_u32, 0);

        // One shared type means the access chain takes the index alone.
        let result_type = ctx.U32[1];
        let shared = ctx.shared_memory_u32;
        let index = ctx.ConstU32(3);
        let access = ctx.EmitSharedMemoryAccess(result_type, shared, index);
        assert_ne!(access, 0);
        let words = ctx.Assemble();
        let access_at = words
            .windows(2)
            .position(|pair| opcode(pair[0]) == spv::Op::AccessChain && pair[1] == result_type)
            .expect("an access chain");
        assert_eq!(words[access_at] >> 16, 5, "type, result, base, one index");
    }
}
