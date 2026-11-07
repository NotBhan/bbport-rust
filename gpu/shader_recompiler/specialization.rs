//! The shader permutation key, ported from
//! `legacy/gpu/shadps4/shader_recompiler/specialization.h`.
//!
//! The C++ holds a pointer to `Info` and fetches descriptors while it builds the key; the port
//! shares `Info` and takes the guest-memory reader the descriptor fetches need. Bit-fields are
//! plain fields, since Rust has no anonymous bit-fields.

use crate::gpu::amdgpu::pixel_format::{
    get_number_class, is_integer as number_format_is_integer, CompMapping, CompSwizzle,
    NumberClass, NumberConversion,
};
use crate::gpu::amdgpu::resource::{Buffer, ImageType};
use crate::gpu::shader_recompiler::backend::bindings::Bindings;
use crate::gpu::shader_recompiler::frontend::fetch_shader::{
    parse_fetch_shader, FetchShaderData, InstanceIdType,
};
use crate::gpu::shader_recompiler::frontend::tessellation::TessellationDataConstantBuffer;
use crate::gpu::shader_recompiler::info::Info;
use crate::gpu::shader_recompiler::profile::Profile;
use crate::gpu::shader_recompiler::runtime_info::{HwInfo, RuntimeInfo, SwInfo, SwStage};
use std::cell::RefCell;
use std::rc::Rc;

/// `Shader::VsAttribSpecialization`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VsAttribSpecialization {
    pub divisor: u32,
    pub num_class: NumberClass,
    pub dst_select: CompMapping,
}

impl Default for VsAttribSpecialization {
    fn default() -> Self {
        VsAttribSpecialization {
            divisor: 0,
            num_class: NumberClass::Float,
            dst_select: zero_mapping(),
        }
    }
}

/// A destination swizzle with every component reading zero, which is what a zeroed descriptor
/// maps to.
fn zero_mapping() -> CompMapping {
    CompMapping {
        r: CompSwizzle::Zero,
        g: CompSwizzle::Zero,
        b: CompSwizzle::Zero,
        a: CompSwizzle::Zero,
    }
}

/// `Shader::BufferSpecialization`.
#[derive(Debug, Clone, Copy, Eq)]
pub struct BufferSpecialization {
    pub stride: u32,
    pub is_formatted: bool,
    pub swizzle_enable: bool,
    pub data_format: u32,
    pub num_format: u32,
    pub index_stride: u32,
    pub element_size: u32,
    pub dst_select: CompMapping,
    pub num_conversion: NumberConversion,
}

impl Default for BufferSpecialization {
    fn default() -> Self {
        BufferSpecialization {
            stride: 0,
            is_formatted: false,
            swizzle_enable: false,
            data_format: 0,
            num_format: 0,
            index_stride: 0,
            element_size: 0,
            dst_select: zero_mapping(),
            num_conversion: NumberConversion::None,
        }
    }
}

impl BufferSpecialization {
    /// The header's comparison: format fields only count when formatted, swizzle fields only
    /// when swizzling.
    pub fn is_same(&self, other: &Self) -> bool {
        self.stride == other.stride
            && self.is_formatted == other.is_formatted
            && self.swizzle_enable == other.swizzle_enable
            && (!self.is_formatted
                || (self.data_format == other.data_format
                    && self.num_format == other.num_format
                    && self.dst_select == other.dst_select
                    && self.num_conversion == other.num_conversion))
            && (!self.swizzle_enable
                || (self.index_stride == other.index_stride
                    && self.element_size == other.element_size))
    }
}

impl PartialEq for BufferSpecialization {
    fn eq(&self, other: &Self) -> bool {
        self.is_same(other)
    }
}

/// `Shader::ImageSpecialization`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageSpecialization {
    pub ty: ImageType,
    pub is_integer: bool,
    pub is_storage: bool,
    pub is_cube: bool,
    pub is_srgb: bool,
    pub dst_select: CompMapping,
    pub num_conversion: NumberConversion,
    pub num_bindings: u32,
}

impl Default for ImageSpecialization {
    fn default() -> Self {
        ImageSpecialization {
            ty: ImageType::Color2D,
            is_integer: false,
            is_storage: false,
            is_cube: false,
            is_srgb: false,
            dst_select: zero_mapping(),
            num_conversion: NumberConversion::None,
            num_bindings: 0,
        }
    }
}

/// `Shader::FMaskSpecialization`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FMaskSpecialization {
    pub width: u32,
    pub height: u32,
}

/// `Shader::SamplerSpecialization`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SamplerSpecialization {
    pub force_unnormalized: bool,
    pub force_degamma: bool,
}

/// `Shader::StageSpecialization`: the compatibility key for a shader permutation.
#[derive(Debug, Clone)]
pub struct StageSpecialization {
    pub info: Option<Rc<RefCell<Info>>>,
    pub runtime_info: RuntimeInfo,
    pub bitset: [bool; Self::MAX_STAGE_RESOURCES],
    pub fetch_shader_data: Option<FetchShaderData>,
    pub vs_attribs: Vec<VsAttribSpecialization>,
    pub buffers: Vec<BufferSpecialization>,
    pub images: Vec<ImageSpecialization>,
    pub fmasks: Vec<FMaskSpecialization>,
    pub samplers: Vec<SamplerSpecialization>,
    pub start: Bindings,
    /// The descriptors read through pointers in game memory, kept so the pre-build can translate
    /// the permutation again without the game.
    pub vs_attrib_sharps: Vec<Buffer>,
    pub tess_constants: Option<TessellationDataConstantBuffer>,
}

impl Default for StageSpecialization {
    fn default() -> Self {
        StageSpecialization {
            info: None,
            runtime_info: RuntimeInfo::new(
                crate::gpu::shader_recompiler::runtime_info::HwStage::Fragment,
                SwStage::Fragment,
            ),
            bitset: [false; Self::MAX_STAGE_RESOURCES],
            fetch_shader_data: None,
            vs_attribs: Vec::new(),
            buffers: Vec::new(),
            images: Vec::new(),
            fmasks: Vec::new(),
            samplers: Vec::new(),
            start: Bindings::default(),
            vs_attrib_sharps: Vec::new(),
            tess_constants: None,
        }
    }
}

impl StageSpecialization {
    pub const MAX_STAGE_RESOURCES: usize = 128;

    /// `StageSpecialization(const Info&, RuntimeInfo, const Profile&, Bindings)`.
    pub fn new(
        info: Rc<RefCell<Info>>,
        mut runtime_info: RuntimeInfo,
        _profile: &Profile,
        start: Bindings,
        code: &[u32],
        read: &dyn Fn(u64) -> u32,
    ) -> Self {
        let mut specialization = StageSpecialization {
            info: Some(info.clone()),
            start,
            ..StageSpecialization::default()
        };
        let stage = {
            let info = info.borrow();
            specialization.fetch_shader_data = parse_fetch_shader(&info, code);
            if let Some(fetch_shader_data) = &specialization.fetch_shader_data {
                for attrib in &fetch_shader_data.attributes {
                    specialization
                        .vs_attrib_sharps
                        .push(attrib.read_raw_sharp(&info, read));
                }
            }
            if info.persistent.sw_stage == SwStage::Vertex {
                if let Some(fetch_shader_data) = &specialization.fetch_shader_data {
                    for attrib in &fetch_shader_data.attributes {
                        specialization
                            .vs_attribs
                            .push(VsAttribSpecialization::default());
                        let spec = specialization
                            .vs_attribs
                            .last_mut()
                            .expect("the spec was just pushed");
                        let sharp = attrib.get_sharp(&info, read);
                        if !sharp.is_valid() {
                            continue;
                        }
                        let step_rate = attrib.get_step_rate();
                        if step_rate != InstanceIdType::None {
                            let vs = match &runtime_info.sw {
                                SwInfo::Vs(vs) => vs,
                                _ => unreachable!("a vertex shader has vertex runtime info"),
                            };
                            spec.divisor = match step_rate {
                                InstanceIdType::OverStepRate0 => vs.step_rate_0,
                                InstanceIdType::OverStepRate1 => vs.step_rate_1,
                                _ => 1,
                            };
                        }
                        spec.num_class = get_number_class(sharp.get_number_fmt());
                        spec.dst_select = sharp.dst_select();
                    }
                }
            }

            let mut binding = 0usize;
            for desc in &info.persistent.buffers {
                specialization.buffers.push(BufferSpecialization::default());
                let spec = specialization
                    .buffers
                    .last_mut()
                    .expect("the spec was just pushed");
                let sharp = desc.get_sharp(&info);
                if !sharp.is_valid() {
                    binding += 1;
                    continue;
                }
                specialization.bitset[binding] = true;
                binding += 1;
                spec.stride = sharp.get_stride();
                spec.is_formatted = desc.is_formatted;
                spec.swizzle_enable = sharp.swizzle_enable();
                if spec.is_formatted {
                    spec.data_format = sharp.get_data_fmt().raw();
                    spec.num_format = sharp.get_number_fmt().raw();
                    spec.dst_select = sharp.dst_select();
                    spec.num_conversion = sharp.get_number_conversion();
                }
                if spec.swizzle_enable {
                    spec.index_stride = sharp.index_stride();
                    spec.element_size = sharp.element_size();
                }
            }

            binding = 0;
            for desc in &info.persistent.images {
                specialization.images.push(ImageSpecialization::default());
                let spec = specialization
                    .images
                    .last_mut()
                    .expect("the spec was just pushed");
                let sharp = desc.get_sharp(&info);
                if !sharp.is_valid() {
                    binding += 1;
                    continue;
                }
                specialization.bitset[binding] = true;
                binding += 1;
                spec.ty = sharp.get_view_type(desc.is_array);
                spec.is_integer = number_format_is_integer(sharp.get_number_fmt());
                spec.is_storage = desc.is_written;
                spec.is_cube = sharp.is_cube();
                if spec.is_storage {
                    spec.dst_select = sharp.dst_select();
                } else {
                    spec.is_srgb = sharp.get_number_fmt()
                        == crate::gpu::amdgpu::pixel_format::NumberFormat::Srgb;
                }
                spec.num_conversion = sharp.get_number_conversion();
                spec.num_bindings = desc.num_bindings(&info);
            }

            for desc in &info.persistent.fmasks {
                specialization.fmasks.push(FMaskSpecialization::default());
                let spec = specialization
                    .fmasks
                    .last_mut()
                    .expect("the spec was just pushed");
                let sharp = desc.get_sharp(&info);
                if !sharp.is_valid() {
                    continue;
                }
                spec.width = sharp.width() as u32;
                spec.height = sharp.height() as u32;
            }

            for desc in &info.persistent.samplers {
                specialization
                    .samplers
                    .push(SamplerSpecialization::default());
                let spec = specialization
                    .samplers
                    .last_mut()
                    .expect("the spec was just pushed");
                let sharp = desc.get_sharp(&info);
                if !sharp.is_valid() {
                    continue;
                }
                spec.force_unnormalized = sharp.force_unnormalized() != 0;
                spec.force_degamma = sharp.force_degamma() != 0;
            }

            info.persistent.sw_stage
        };

        if stage == SwStage::TessellationControl || stage == SwStage::TessellationEval {
            let info = info.borrow();
            let mut constants = TessellationDataConstantBuffer::default();
            info.read_tess_constant_buffer(&mut constants, read);
            runtime_info.init_from_tess_constants(&constants);
            specialization.tess_constants = Some(constants);
        }

        specialization
    }

    /// `StageSpecialization::Valid`.
    pub fn valid(&self) -> bool {
        self.info.is_some()
    }

    fn bitset_none(&self) -> bool {
        !self.bitset.iter().any(|bit| *bit)
    }

    /// The first field of the comparison that fails, or `None` when the keys are equal.
    pub fn mismatch(&self, other: &Self) -> Option<&'static str> {
        if !self.valid() {
            return Some("invalid");
        }
        if self.vs_attribs != other.vs_attribs {
            return Some("vs_attribs");
        }
        if self.runtime_info != other.runtime_info {
            let a = &self.runtime_info;
            let b = &other.runtime_info;
            if a.hw_stage != b.hw_stage || a.sw_stage != b.sw_stage {
                return Some("runtime_info.stage");
            }
            if a.props != b.props {
                return Some("runtime_info.props");
            }
            match (&a.hw, &b.hw) {
                (HwInfo::Fs(x), HwInfo::Fs(y)) => {
                    if x.motion_vectors != y.motion_vectors {
                        return Some("fs.motion_vectors");
                    }
                    if x.color_buffers != y.color_buffers {
                        return Some("fs.color_buffers");
                    }
                    if x.num_inputs != y.num_inputs || x.inputs != y.inputs {
                        return Some("fs.inputs");
                    }
                    if x.num_samples != y.num_samples {
                        return Some("fs.num_samples");
                    }
                    if x.mrtz_mask != y.mrtz_mask {
                        return Some("fs.mrtz_mask");
                    }
                    if x.z_export_format != y.z_export_format {
                        return Some("fs.z_export_format");
                    }
                    return Some("fs.other");
                }
                (HwInfo::Vs(x), HwInfo::Vs(y)) => {
                    if x.motion_vectors != y.motion_vectors {
                        return Some("vs.motion_vectors");
                    }
                    if x.num_outputs != y.num_outputs || x.outputs != y.outputs {
                        return Some("vs.outputs");
                    }
                    if x.clip_disable != y.clip_disable {
                        return Some("vs.clip_disable");
                    }
                    if x.user_clip_plane_mask != y.user_clip_plane_mask {
                        return Some("vs.user_clip");
                    }
                    if x.emulate_depth_negative_one_to_one != y.emulate_depth_negative_one_to_one {
                        return Some("vs.depth");
                    }
                    return Some("vs.sw");
                }
                _ => {}
            }
            return Some("runtime_info");
        }
        if self.fetch_shader_data != other.fetch_shader_data {
            return Some("fetch_shader_data");
        }
        if self.fmasks != other.fmasks {
            return Some("fmasks");
        }
        if self.bitset_none() && other.bitset_none() {
            return None;
        }
        if self.start != other.start {
            return Some("start");
        }
        let mut binding = 0usize;
        for index in 0..self.buffers.len() {
            if other.bitset[binding] && self.buffers[index] != other.buffers[index] {
                return Some("buffers");
            }
            binding += 1;
        }
        for index in 0..self.images.len() {
            if other.bitset[binding] && self.images[index] != other.images[index] {
                return Some("images");
            }
            binding += 1;
        }
        for index in 0..self.samplers.len() {
            if self.samplers[index] != other.samplers[index] {
                return Some("samplers");
            }
        }
        None
    }
}

impl PartialEq for StageSpecialization {
    fn eq(&self, other: &Self) -> bool {
        if !self.valid() {
            return false;
        }
        if self.vs_attribs != other.vs_attribs {
            return false;
        }
        if self.runtime_info != other.runtime_info {
            return false;
        }
        if self.fetch_shader_data != other.fetch_shader_data {
            return false;
        }
        if self.fmasks != other.fmasks {
            return false;
        }
        if self.bitset_none() && other.bitset_none() {
            return true;
        }
        if self.start != other.start {
            return false;
        }
        let mut binding = 0usize;
        for index in 0..self.buffers.len() {
            if other.bitset[binding] && self.buffers[index] != other.buffers[index] {
                return false;
            }
            binding += 1;
        }
        for index in 0..self.images.len() {
            if other.bitset[binding] && self.images[index] != other.images[index] {
                return false;
            }
            binding += 1;
        }
        for index in 0..self.samplers.len() {
            if self.samplers[index] != other.samplers[index] {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, RuntimeInfo};

    fn info(stage: HwStage, sw_stage: SwStage) -> Rc<RefCell<Info>> {
        Rc::new(RefCell::new(Info::new(
            stage,
            sw_stage,
            ShaderParams::new([0; 16], &[], 0),
        )))
    }

    #[test]
    fn a_vertex_specialization_without_resources_is_valid_and_compares_by_stage() {
        let read = |_| 0u32;
        let info = info(HwStage::Vertex, SwStage::Vertex);
        let runtime = RuntimeInfo::new(HwStage::Vertex, SwStage::Vertex);
        let spec = StageSpecialization::new(
            info.clone(),
            runtime.clone(),
            &Profile::default(),
            Bindings::default(),
            &[],
            &read,
        );
        assert!(spec.valid());
        assert!(spec.buffers.is_empty());
        assert!(spec.fetch_shader_data.is_none());
        assert_eq!(spec.mismatch(&spec), None);
        assert!(spec == spec);
    }

    #[test]
    fn buffer_specialization_ignores_fields_its_flags_disable() {
        let base = BufferSpecialization {
            stride: 16,
            is_formatted: false,
            swizzle_enable: false,
            data_format: 1,
            num_format: 2,
            index_stride: 1,
            element_size: 2,
            ..BufferSpecialization::default()
        };
        let mut other = base;
        other.data_format = 9;
        other.num_format = 9;
        other.index_stride = 9;
        other.element_size = 9;
        assert_eq!(base, other, "unused fields do not matter");
        other.is_formatted = true;
        assert_ne!(base, other, "once formatted, the format fields count");
        other.is_formatted = false;
        other.swizzle_enable = true;
        assert_ne!(base, other, "once swizzling, the swizzle fields count");
    }

    #[test]
    fn an_invalid_specialization_matches_nothing() {
        let read = |_| 0u32;
        let info = info(HwStage::Vertex, SwStage::Vertex);
        let runtime = RuntimeInfo::new(HwStage::Vertex, SwStage::Vertex);
        let valid = StageSpecialization::new(
            info,
            runtime,
            &Profile::default(),
            Bindings::default(),
            &[],
            &read,
        );
        let invalid = StageSpecialization::default();
        assert!(!invalid.valid());
        assert_eq!(invalid.mismatch(&valid), Some("invalid"));
        assert!(invalid != invalid);
    }
}
