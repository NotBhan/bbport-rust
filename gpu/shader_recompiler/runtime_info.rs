//! The per-draw runtime information a shader is compiled with, ported from
//! `legacy/gpu/shadps4/shader_recompiler/runtime_info.h`.
//!
//! Two things change shape. The C++'s `SwInfo`/`HwInfo` unions, read through whichever member
//! the stage names, are enums here: the same storage is still only ever filled for one stage,
//! and `HasSameSwInfo`/`HasSameHwInfo` keep comparing exactly the member the stage selects.
//! `RuntimeInfo::Initialize` memsets the whole struct and then applies two fields, which the
//! port reproduces with a `zeroed` per variant rather than the C++'s default member initialisers.
//! `HwGeometryRuntimeInfo::vs_copy` is a `std::span` into the vertex shader's code, which the
//! port owns as a `Vec`.

use crate::gpu::amdgpu::pixel_format::{CompMapping, DataFormat, NumberConversion, NumberFormat};
use crate::gpu::amdgpu::regs_shader::{
    FpDenormMode, FpRoundMode, PsInput as AmdPsInput, ShaderExportFormat,
};
use crate::gpu::amdgpu::regs_vertex::{
    GsOutputPrimitiveType, GsScenario, PrimitiveType, TessellationPartitioning,
    TessellationTopology, TessellationType,
};
use crate::gpu::shader_recompiler::frontend::tessellation::TessellationDataConstantBuffer;
use std::sync::atomic::{AtomicU64, Ordering};

/// `Shader::HwStage`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HwStage {
    Fragment,
    Vertex,
    Geometry,
    Export,
    Hull,
    Local,
    Compute,
}

impl HwStage {
    /// The name the header's formatter prints.
    pub const fn name(self) -> &'static str {
        match self {
            HwStage::Fragment => "fs",
            HwStage::Vertex => "vs",
            HwStage::Geometry => "gs",
            HwStage::Export => "es",
            HwStage::Hull => "hs",
            HwStage::Local => "ls",
            HwStage::Compute => "cs",
        }
    }
}

/// The header's `fmt::formatter<Shader::HwStage>`, which prints the short stage name.
impl std::fmt::Display for HwStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// `Shader::SwStage`: vertex comes after the tessellation stages, since that is the order they
/// are compiled in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwStage {
    Fragment,
    TessellationControl,
    TessellationEval,
    Vertex,
    Geometry,
    Compute,
    NumLogicalStages,
}

/// `Shader::MaxStageTypes`.
pub const MAX_STAGE_TYPES: u32 = SwStage::NumLogicalStages as u32;

/// `Shader::MaxEmulatedClipDistances`.
pub const MAX_EMULATED_CLIP_DISTANCES: u32 = 4;

/// `Shader::StageFromIndex`.
pub fn stage_from_index(index: usize) -> HwStage {
    match index {
        0 => HwStage::Fragment,
        1 => HwStage::Vertex,
        2 => HwStage::Geometry,
        3 => HwStage::Export,
        4 => HwStage::Hull,
        5 => HwStage::Local,
        _ => HwStage::Compute,
    }
}

/// `Shader::Output`: what an export slot declares itself to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Output {
    None,
    PointSize,
    EdgeFlag,
    KillFlag,
    GsCutFlag,
    RenderTargetIndex,
    ViewportIndex,
    CullDist0,
    CullDist1,
    CullDist2,
    CullDist3,
    CullDist4,
    CullDist5,
    CullDist6,
    CullDist7,
    ClipDist0,
    ClipDist1,
    ClipDist2,
    ClipDist3,
    ClipDist4,
    ClipDist5,
    ClipDist6,
    ClipDist7,
}

/// `Shader::OutputMap`.
pub type OutputMap = [Output; 4];

/// `Shader::SwVertexRuntimeInfo`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SwVertexRuntimeInfo {
    pub step_rate_0: u32,
    pub step_rate_1: u32,
    pub vertex_sgpr_offset: u16,
    pub instance_sgpr_offset: u16,
    pub tess_emulated_primitive: bool,
}

/// `Shader::MotionVectors`: the addresses the motion vector pass stores its parameters at. The
/// C++ keeps two mutable statics; the port keeps two atomics and the same accessors.
pub static MOTION_VECTOR_PARAMS_ADDRESS: AtomicU64 = AtomicU64::new(0);
/// The position array's address.
pub static MOTION_VECTOR_POSITIONS_ADDRESS: AtomicU64 = AtomicU64::new(0);

/// The named constants of `Shader::MotionVectors`.
pub struct MotionVectors;

impl MotionVectors {
    /// `CurrentLocation`: the varying the current clip position is written to.
    pub const CURRENT_LOCATION: u32 = 30;
    /// `PreviousLocation`: the varying the previous clip position is read from, with z holding
    /// whether it is valid.
    pub const PREVIOUS_LOCATION: u32 = 31;
    /// `Output`: the color attachment the difference goes to.
    pub const OUTPUT: u32 = 7;
    /// `FlagStore`.
    pub const FLAG_STORE: u32 = 1;
    /// `FlagLoad`.
    pub const FLAG_LOAD: u32 = 2;

    pub fn params_address() -> u64 {
        MOTION_VECTOR_PARAMS_ADDRESS.load(Ordering::Relaxed)
    }

    pub fn set_params_address(address: u64) {
        MOTION_VECTOR_PARAMS_ADDRESS.store(address, Ordering::Relaxed);
    }

    pub fn positions_address() -> u64 {
        MOTION_VECTOR_POSITIONS_ADDRESS.load(Ordering::Relaxed)
    }

    pub fn set_positions_address(address: u64) {
        MOTION_VECTOR_POSITIONS_ADDRESS.store(address, Ordering::Relaxed);
    }
}

/// `Shader::HwLocalRuntimeInfo`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HwLocalRuntimeInfo {
    pub ls_stride: u32,
}

/// `Shader::HwExportRuntimeInfo`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HwExportRuntimeInfo {
    pub vertex_data_size: u32,
}

/// `Shader::HwVertexRuntimeInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HwVertexRuntimeInfo {
    pub num_outputs: u32,
    pub outputs: [OutputMap; 3],
    pub emulate_depth_negative_one_to_one: bool,
    pub clip_disable: bool,
    pub user_clip_plane_mask: u32,
    pub motion_vectors: bool,
}

impl HwVertexRuntimeInfo {
    pub fn zeroed() -> Self {
        HwVertexRuntimeInfo {
            num_outputs: 0,
            outputs: [[Output::None; 4]; 3],
            emulate_depth_negative_one_to_one: false,
            clip_disable: false,
            user_clip_plane_mask: 0,
            motion_vectors: false,
        }
    }
}

/// `Shader::SwTessControlRuntimeInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwTessControlRuntimeInfo {
    pub hs_output_cp_stride: u32,
    pub num_input_control_points: u32,
    pub num_threads: u32,
    pub tess_type: TessellationType,
    pub offchip_lds_enable: bool,
    pub ls_stride: u32,
    pub hs_output_base: u32,
}

impl SwTessControlRuntimeInfo {
    pub fn zeroed() -> Self {
        SwTessControlRuntimeInfo {
            hs_output_cp_stride: 0,
            num_input_control_points: 0,
            num_threads: 0,
            tess_type: TessellationType::Isoline,
            offchip_lds_enable: false,
            ls_stride: 0,
            hs_output_base: 0,
        }
    }

    /// `IsPassthrough`: a non-passthrough shader might also meet these, in which case assuming
    /// passthrough declares a few extra outputs the evaluation shader never reads.
    pub fn is_passthrough(&self) -> bool {
        self.hs_output_base == 0
            && self.ls_stride == self.hs_output_cp_stride
            && self.num_threads == 1
    }

    /// `NumOutputControlPoints`: `hs_output_control_points` holds the number of threads, which
    /// is not the number of output control points; a passthrough shader has the register field
    /// at 1 and takes its input control points instead.
    pub fn num_output_control_points(&self) -> u32 {
        if self.is_passthrough() {
            self.num_input_control_points
        } else {
            self.num_threads
        }
    }
}

/// `Shader::SwTessEvalRuntimeInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwTessEvalRuntimeInfo {
    pub hs_output_cp_stride: u32,
    pub tess_type: TessellationType,
    pub tess_topology: TessellationTopology,
    pub tess_partitioning: TessellationPartitioning,
}

impl SwTessEvalRuntimeInfo {
    pub fn zeroed() -> Self {
        SwTessEvalRuntimeInfo {
            hs_output_cp_stride: 0,
            tess_type: TessellationType::Isoline,
            tess_topology: TessellationTopology::Point,
            tess_partitioning: TessellationPartitioning::Integer,
        }
    }
}

/// `Shader::GsMaxOutputStreams`.
pub const GS_MAX_OUTPUT_STREAMS: usize = 4;

/// `Shader::GsOutputPrimTypes`.
pub type GsOutputPrimTypes = [GsOutputPrimitiveType; GS_MAX_OUTPUT_STREAMS];

/// `Shader::HwGeometryRuntimeInfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HwGeometryRuntimeInfo {
    pub num_outputs: u32,
    pub outputs: [OutputMap; 3],
    pub num_invocations: u32,
    pub output_vertices: u32,
    pub in_vertex_data_size: u32,
    pub out_vertex_data_size: u32,
    pub in_primitive: PrimitiveType,
    pub out_primitive: GsOutputPrimTypes,
    pub mode: GsScenario,
    pub vs_copy: Vec<u32>,
    pub vs_copy_hash: u64,
}

impl HwGeometryRuntimeInfo {
    pub fn zeroed() -> Self {
        HwGeometryRuntimeInfo {
            num_outputs: 0,
            outputs: [[Output::None; 4]; 3],
            num_invocations: 0,
            output_vertices: 0,
            in_vertex_data_size: 0,
            out_vertex_data_size: 0,
            in_primitive: PrimitiveType::None,
            out_primitive: [GsOutputPrimitiveType::PointList; GS_MAX_OUTPUT_STREAMS],
            mode: GsScenario::Off,
            vs_copy: Vec::new(),
            vs_copy_hash: 0,
        }
    }

    /// The header's comparison, which tests whether the two invocation counts are *non-zero*
    /// rather than equal.
    pub fn is_same_as(&self, other: &HwGeometryRuntimeInfo) -> bool {
        self.num_outputs == other.num_outputs
            && self.outputs == other.outputs
            && self.num_invocations != 0
            && other.num_invocations != 0
            && self.output_vertices == other.output_vertices
            && self.in_primitive == other.in_primitive
            && self.out_primitive == other.out_primitive
            && self.vs_copy_hash == other.vs_copy_hash
    }
}

/// `Shader::MrtSwizzle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MrtSwizzle {
    Identity = 0,
    Alt = 1,
    Reverse = 2,
    ReverseAlt = 3,
}

/// `Shader::MaxColorBuffers`.
pub const MAX_COLOR_BUFFERS: usize = 8;

/// `Shader::PsColorBuffer`. The C++ packs the four format fields and the flag into one word;
/// the port keeps them as the fields the compiler reads and writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PsColorBuffer {
    pub data_format: DataFormat,
    pub num_format: NumberFormat,
    pub num_conversion: NumberConversion,
    pub export_format: ShaderExportFormat,
    /// GCN applies blend factors to min/max ops while Vulkan ignores them, so the shader squares
    /// its output instead for the self-scaled pattern.
    pub blend_self_scale: bool,
    pub swizzle: CompMapping,
}

impl PsColorBuffer {
    pub fn zeroed() -> Self {
        PsColorBuffer {
            data_format: DataFormat::FormatInvalid,
            num_format: NumberFormat::Unorm,
            num_conversion: NumberConversion::None,
            export_format: ShaderExportFormat::Zero,
            blend_self_scale: false,
            swizzle: CompMapping::IDENTITY,
        }
    }
}

/// `Shader::HwFragmentRuntimeInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HwFragmentRuntimeInfo {
    pub en_flags: AmdPsInput,
    pub addr_flags: AmdPsInput,
    pub num_inputs: u32,
    pub inputs: [PsInput; 32],
    pub color_buffers: [PsColorBuffer; MAX_COLOR_BUFFERS],
    pub z_export_format: ShaderExportFormat,
    pub num_samples: u8,
    pub mrtz_mask: u8,
    pub front_face_all_bits: bool,
    pub dual_source_blending: bool,
    pub clip_distance_emulation: bool,
    /// bbport: writes the object motion vector to `MotionVectors::OUTPUT`.
    pub motion_vectors: bool,
}

/// `HwFragmentRuntimeInfo::PsInput`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PsInput {
    pub param_index: u8,
    pub is_default: bool,
    pub is_flat: bool,
    pub default_value: u8,
}

impl PsInput {
    pub fn zeroed() -> Self {
        PsInput {
            param_index: 0,
            is_default: false,
            is_flat: false,
            default_value: 0,
        }
    }

    /// `IsDefault`.
    pub fn is_default(&self) -> bool {
        self.is_default && !self.is_flat
    }

    /// `IsPassthrough`.
    pub fn is_passthrough(&self) -> bool {
        self.is_default && self.is_flat
    }
}

impl HwFragmentRuntimeInfo {
    pub fn zeroed() -> Self {
        HwFragmentRuntimeInfo {
            en_flags: AmdPsInput::from_raw(0),
            addr_flags: AmdPsInput::from_raw(0),
            num_inputs: 0,
            inputs: [PsInput::zeroed(); 32],
            color_buffers: [PsColorBuffer::zeroed(); MAX_COLOR_BUFFERS],
            z_export_format: ShaderExportFormat::Zero,
            num_samples: 0,
            mrtz_mask: 0,
            front_face_all_bits: false,
            dual_source_blending: false,
            clip_distance_emulation: false,
            motion_vectors: false,
        }
    }

    /// The header's comparison, which only looks at the inputs the shader reads.
    pub fn is_same_as(&self, other: &HwFragmentRuntimeInfo) -> bool {
        let count = self.num_inputs.min(other.num_inputs) as usize;
        self.color_buffers == other.color_buffers
            && self.en_flags == other.en_flags
            && self.addr_flags == other.addr_flags
            && self.num_inputs == other.num_inputs
            && self.z_export_format == other.z_export_format
            && self.num_samples == other.num_samples
            && self.mrtz_mask == other.mrtz_mask
            && self.front_face_all_bits == other.front_face_all_bits
            && self.dual_source_blending == other.dual_source_blending
            && self.clip_distance_emulation == other.clip_distance_emulation
            && self.motion_vectors == other.motion_vectors
            && self.inputs[..count] == other.inputs[..count]
    }
}

/// `Shader::HwComputeRuntimeInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HwComputeRuntimeInfo {
    pub shared_memory_size: u32,
    pub workgroup_size: [u32; 3],
    pub tgid_enable: [bool; 3],
}

impl HwComputeRuntimeInfo {
    pub fn zeroed() -> Self {
        HwComputeRuntimeInfo {
            shared_memory_size: 0,
            workgroup_size: [0; 3],
            tgid_enable: [false; 3],
        }
    }

    /// The header's comparison, which leaves the shared memory size out.
    pub fn is_same_as(&self, other: &HwComputeRuntimeInfo) -> bool {
        self.workgroup_size == other.workgroup_size && self.tgid_enable == other.tgid_enable
    }
}

/// `RuntimeInfo::SwInfo`: the union the software stages share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwInfo {
    Vs(SwVertexRuntimeInfo),
    Tcs(SwTessControlRuntimeInfo),
    Tes(SwTessEvalRuntimeInfo),
}

/// `RuntimeInfo::HwInfo`: the union the hardware stages share. The geometry stage is by far the
/// largest, as the C++ union's storage is, so clippy's size warning is allowed.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HwInfo {
    Ls(HwLocalRuntimeInfo),
    Es(HwExportRuntimeInfo),
    Gs(HwGeometryRuntimeInfo),
    Vs(HwVertexRuntimeInfo),
    Fs(HwFragmentRuntimeInfo),
    Cs(HwComputeRuntimeInfo),
}

/// `RuntimeInfo::Properties`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Properties {
    pub fp_denorm_mode32: FpDenormMode,
    pub fp_denorm_mode16_64: FpDenormMode,
    pub fp_round_mode32: FpRoundMode,
    pub fp_round_mode16_64: FpRoundMode,
    pub num_user_data: u32,
    pub num_input_vgprs: u32,
    pub num_allocated_vgprs: u32,
}

impl Properties {
    pub fn zeroed() -> Self {
        Properties {
            fp_denorm_mode32: FpDenormMode::InOutFlush,
            fp_denorm_mode16_64: FpDenormMode::InOutFlush,
            fp_round_mode32: FpRoundMode::NearestEven,
            fp_round_mode16_64: FpRoundMode::NearestEven,
            num_user_data: 0,
            num_input_vgprs: 0,
            num_allocated_vgprs: 0,
        }
    }
}

/// `Shader::RuntimeInfo`: what the liverpool registers say about this draw. It can differ for
/// the same shader module, so it is part of the shader key.
#[derive(Debug, Clone)]
pub struct RuntimeInfo {
    pub sw_stage: SwStage,
    pub hw_stage: HwStage,
    pub props: Properties,
    pub sw: SwInfo,
    pub hw: HwInfo,
}

impl RuntimeInfo {
    /// `RuntimeInfo::Initialize`: the C++ memsets the structure and then sets the two stages,
    /// with one sample for a fragment shader.
    pub fn initialize(&mut self, stage: HwStage, l_stage: SwStage) {
        self.props = Properties::zeroed();
        self.sw = SwInfo::Vs(SwVertexRuntimeInfo::default());
        self.hw = match stage {
            HwStage::Local => HwInfo::Ls(HwLocalRuntimeInfo::default()),
            HwStage::Export => HwInfo::Es(HwExportRuntimeInfo::default()),
            HwStage::Geometry => HwInfo::Gs(HwGeometryRuntimeInfo::zeroed()),
            HwStage::Vertex => HwInfo::Vs(HwVertexRuntimeInfo::zeroed()),
            HwStage::Fragment => HwInfo::Fs(HwFragmentRuntimeInfo::zeroed()),
            HwStage::Compute => HwInfo::Cs(HwComputeRuntimeInfo::zeroed()),
            HwStage::Hull => HwInfo::Ls(HwLocalRuntimeInfo::default()),
        };
        self.hw_stage = stage;
        self.sw_stage = l_stage;
        if stage == HwStage::Fragment {
            if let HwInfo::Fs(fs) = &mut self.hw {
                fs.num_samples = 1;
            }
        }
    }

    /// A fresh structure for a stage pair, as `Initialize` would leave it.
    pub fn new(stage: HwStage, l_stage: SwStage) -> Self {
        let mut info = RuntimeInfo {
            sw_stage: l_stage,
            hw_stage: stage,
            props: Properties::zeroed(),
            sw: SwInfo::Vs(SwVertexRuntimeInfo::default()),
            hw: HwInfo::Ls(HwLocalRuntimeInfo::default()),
        };
        info.initialize(stage, l_stage);
        info
    }

    /// `RuntimeInfo::HasSameSwInfo`.
    pub fn has_same_sw_info(&self, other: &RuntimeInfo) -> bool {
        match self.sw_stage {
            SwStage::Vertex => self.sw == other.sw,
            SwStage::TessellationControl => self.sw == other.sw,
            SwStage::TessellationEval => self.sw == other.sw,
            _ => true,
        }
    }

    /// `RuntimeInfo::HasSameHwInfo`: only the member the stage names is compared, and each
    /// hardware stage's own comparison decides which of its fields count.
    pub fn has_same_hw_info(&self, other: &RuntimeInfo) -> bool {
        match (self.hw_stage, (&self.hw, &other.hw)) {
            (HwStage::Local, (HwInfo::Ls(a), HwInfo::Ls(b))) => a == b,
            (HwStage::Export, (HwInfo::Es(a), HwInfo::Es(b))) => a == b,
            (HwStage::Geometry, (HwInfo::Gs(a), HwInfo::Gs(b))) => a.is_same_as(b),
            (HwStage::Vertex, (HwInfo::Vs(a), HwInfo::Vs(b))) => a == b,
            (HwStage::Fragment, (HwInfo::Fs(a), HwInfo::Fs(b))) => a.is_same_as(b),
            (HwStage::Compute, (HwInfo::Cs(a), HwInfo::Cs(b))) => a.is_same_as(b),
            _ => true,
        }
    }

    /// `RuntimeInfo::InitFromTessConstants`.
    pub fn init_from_tess_constants(&mut self, tess_constants: &TessellationDataConstantBuffer) {
        match (&mut self.sw, self.sw_stage) {
            (SwInfo::Tcs(tcs), SwStage::TessellationControl) => {
                tcs.hs_output_cp_stride = tess_constants.hs_cp_stride;
                tcs.ls_stride = tess_constants.ls_stride;
                tcs.hs_output_base = tess_constants.hs_output_base;
            }
            (SwInfo::Tes(tes), SwStage::TessellationEval) => {
                tes.hs_output_cp_stride = tess_constants.hs_cp_stride;
            }
            _ => {}
        }
    }
}

impl PartialEq for RuntimeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.hw_stage == other.hw_stage
            && self.sw_stage == other.sw_stage
            && self.props == other.props
            && self.has_same_sw_info(other)
            && self.has_same_hw_info(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stages_and_outputs_keep_their_numbering() {
        assert_eq!(HwStage::Fragment as u32, 0);
        assert_eq!(HwStage::Compute as u32, 6);
        assert_eq!(SwStage::Fragment as u32, 0);
        assert_eq!(SwStage::TessellationControl as u32, 1);
        assert_eq!(SwStage::TessellationEval as u32, 2);
        assert_eq!(SwStage::Vertex as u32, 3);
        assert_eq!(SwStage::NumLogicalStages as u32, 6);
        assert_eq!(MAX_STAGE_TYPES, 6);
        assert_eq!(MAX_EMULATED_CLIP_DISTANCES, 4);
        assert_eq!(stage_from_index(0), HwStage::Fragment);
        assert_eq!(stage_from_index(4), HwStage::Hull);
        assert_eq!(stage_from_index(6), HwStage::Compute);

        // The formatter's names, in stage order.
        assert_eq!(HwStage::Fragment.name(), "fs");
        assert_eq!(HwStage::Vertex.name(), "vs");
        assert_eq!(HwStage::Geometry.name(), "gs");
        assert_eq!(HwStage::Export.name(), "es");
        assert_eq!(HwStage::Hull.name(), "hs");
        assert_eq!(HwStage::Local.name(), "ls");
        assert_eq!(HwStage::Compute.name(), "cs");
        assert_eq!(HwStage::Compute.to_string(), "cs");
        assert_eq!(format!("{}", HwStage::Hull), "hs");

        // The outputs are numbered in declaration order.
        assert_eq!(Output::None as u8, 0);
        assert_eq!(Output::PointSize as u8, 1);
        assert_eq!(Output::RenderTargetIndex as u8, 5);
        assert_eq!(Output::ViewportIndex as u8, 6);
        assert_eq!(Output::CullDist0 as u8, 7);
        assert_eq!(Output::CullDist7 as u8, 14);
        assert_eq!(Output::ClipDist0 as u8, 15);
        assert_eq!(Output::ClipDist7 as u8, 22);
        assert_eq!(MrtSwizzle::ReverseAlt as u8, 3);
    }

    #[test]
    fn initialization_zeroes_the_structure_and_samples_fragments_once() {
        let mut info = RuntimeInfo::new(HwStage::Fragment, SwStage::Fragment);
        assert_eq!(info.hw_stage, HwStage::Fragment);
        assert_eq!(info.sw_stage, SwStage::Fragment);
        assert_eq!(info.props, Properties::zeroed());
        match info.hw {
            HwInfo::Fs(fs) => {
                assert_eq!(
                    fs.num_samples, 1,
                    "a fragment shader starts with one sample"
                );
                assert_eq!(fs.num_inputs, 0);
                assert_eq!(fs.color_buffers[0].data_format, DataFormat::FormatInvalid);
                assert_eq!(fs.color_buffers[0].swizzle, CompMapping::IDENTITY);
            }
            ref other => panic!("expected a fragment stage, got {other:?}"),
        }

        // Any other stage keeps the sample count at zero, since the C++ memsets it away.
        info.initialize(HwStage::Vertex, SwStage::Vertex);
        match info.hw {
            HwInfo::Vs(vs) => {
                assert_eq!(vs.num_outputs, 0);
                assert!(!vs.clip_disable);
                assert_eq!(vs.outputs[0], [Output::None; 4]);
            }
            ref other => panic!("expected a vertex stage, got {other:?}"),
        }
        info.initialize(HwStage::Compute, SwStage::Compute);
        match info.hw {
            HwInfo::Cs(cs) => {
                assert_eq!(cs.workgroup_size, [0; 3]);
                assert_eq!(cs.tgid_enable, [false; 3]);
                assert_eq!(cs.shared_memory_size, 0);
            }
            ref other => panic!("expected a compute stage, got {other:?}"),
        }
    }

    #[test]
    fn the_stage_decides_which_half_is_compared() {
        // Two fragment stages with the same inputs are equal, and a differing input counts.
        let mut a = RuntimeInfo::new(HwStage::Fragment, SwStage::Fragment);
        let mut b = RuntimeInfo::new(HwStage::Fragment, SwStage::Fragment);
        assert_eq!(a, b);
        if let HwInfo::Fs(fs) = &mut b.hw {
            fs.num_samples = 4;
        }
        assert_ne!(a, b, "the sample count is part of the fragment key");
        if let HwInfo::Fs(fs) = &mut b.hw {
            fs.num_samples = 1;
        }
        assert_eq!(a, b);
        // Only the inputs the shader reads are compared.
        if let HwInfo::Fs(fs) = &mut a.hw {
            fs.num_inputs = 2;
            fs.inputs[0] = PsInput {
                param_index: 1,
                is_default: false,
                is_flat: false,
                default_value: 0,
            };
        }
        if let HwInfo::Fs(fs) = &mut b.hw {
            fs.num_inputs = 2;
            fs.inputs[0] = PsInput {
                param_index: 1,
                is_default: false,
                is_flat: false,
                default_value: 0,
            };
            // An input past the count is not looked at.
            fs.inputs[5] = PsInput {
                param_index: 9,
                is_default: true,
                is_flat: true,
                default_value: 3,
            };
        }
        assert_eq!(a, b, "inputs past num_inputs are ignored");
        if let HwInfo::Fs(fs) = &mut b.hw {
            fs.inputs[1].param_index = 7;
        }
        assert_ne!(a, b);

        // The C++ reads the other structure's union through its own stage's member whatever
        // the other stage holds, which the port's enums cannot do, so two different hardware
        // stages compare equal: there is nothing to compare.
        let vertex = RuntimeInfo::new(HwStage::Vertex, SwStage::Vertex);
        let fragment = RuntimeInfo::new(HwStage::Fragment, SwStage::Fragment);
        assert!(vertex.has_same_hw_info(&fragment));
        assert!(vertex.has_same_sw_info(&fragment));
        assert_ne!(
            vertex.hw_stage, fragment.hw_stage,
            "the stages themselves differ"
        );
    }

    #[test]
    fn the_geometry_and_compute_comparisons_leave_fields_out() {
        let mut a = RuntimeInfo::new(HwStage::Geometry, SwStage::Geometry);
        let mut b = RuntimeInfo::new(HwStage::Geometry, SwStage::Geometry);
        if let HwInfo::Gs(gs) = &mut a.hw {
            gs.num_invocations = 2;
        }
        if let HwInfo::Gs(gs) = &mut b.hw {
            gs.num_invocations = 3;
        }
        assert_eq!(a, b, "the header tests whether the counts are non-zero");
        if let HwInfo::Gs(gs) = &mut b.hw {
            gs.num_invocations = 0;
        }
        assert_ne!(a, b);
        if let HwInfo::Gs(gs) = &mut b.hw {
            gs.num_invocations = 3;
            gs.vs_copy_hash = 0x1234;
        }
        assert_ne!(a, b, "the vertex shader copy's hash is part of the key");
        if let HwInfo::Gs(gs) = &mut b.hw {
            gs.vs_copy_hash = 0;
            gs.in_vertex_data_size = 4;
        }
        assert_eq!(a, b, "the vertex data sizes are not compared");

        let mut a = RuntimeInfo::new(HwStage::Compute, SwStage::Compute);
        let mut b = RuntimeInfo::new(HwStage::Compute, SwStage::Compute);
        if let HwInfo::Cs(cs) = &mut a.hw {
            cs.shared_memory_size = 64;
        }
        assert_eq!(a, b, "the shared memory size is not compared");
        if let HwInfo::Cs(cs) = &mut b.hw {
            cs.workgroup_size = [8, 8, 1];
        }
        assert_ne!(a, b);
        if let HwInfo::Cs(cs) = &mut a.hw {
            cs.workgroup_size = [8, 8, 1];
            cs.tgid_enable = [true, false, false];
        }
        assert_ne!(a, b);
    }

    #[test]
    fn the_tessellation_constants_reach_the_control_and_eval_stages() {
        let constants = TessellationDataConstantBuffer {
            ls_stride: 16,
            hs_cp_stride: 32,
            num_patches: 4,
            hs_output_base: 128,
            ..TessellationDataConstantBuffer::default()
        };
        let mut info = RuntimeInfo::new(HwStage::Hull, SwStage::TessellationControl);
        info.sw = SwInfo::Tcs(SwTessControlRuntimeInfo::zeroed());
        info.init_from_tess_constants(&constants);
        match &info.sw {
            SwInfo::Tcs(tcs) => {
                assert_eq!(tcs.hs_output_cp_stride, 32);
                assert_eq!(tcs.ls_stride, 16);
                assert_eq!(tcs.hs_output_base, 128);
            }
            other => panic!("expected a control stage, got {other:?}"),
        }

        let mut info = RuntimeInfo::new(HwStage::Local, SwStage::TessellationEval);
        info.sw = SwInfo::Tes(SwTessEvalRuntimeInfo::zeroed());
        info.init_from_tess_constants(&constants);
        match &info.sw {
            SwInfo::Tes(tes) => assert_eq!(tes.hs_output_cp_stride, 32),
            other => panic!("expected an evaluation stage, got {other:?}"),
        }

        // A stage without tessellation constants ignores the call.
        let mut info = RuntimeInfo::new(HwStage::Vertex, SwStage::Vertex);
        info.init_from_tess_constants(&constants);
        assert_eq!(info.sw, SwInfo::Vs(SwVertexRuntimeInfo::default()));
    }

    #[test]
    fn the_passthrough_control_points_follow_the_header() {
        let mut tcs = SwTessControlRuntimeInfo::zeroed();
        assert!(
            !tcs.is_passthrough(),
            "a zero thread count is not one, so this is not a passthrough"
        );
        tcs.num_threads = 1;
        assert!(tcs.is_passthrough());
        tcs.num_input_control_points = 3;
        assert_eq!(
            tcs.num_output_control_points(),
            3,
            "the input points stand in"
        );
        tcs.num_threads = 4;
        assert!(!tcs.is_passthrough());
        assert_eq!(tcs.num_output_control_points(), 4);
        assert_eq!(MotionVectors::CURRENT_LOCATION, 30);
        assert_eq!(MotionVectors::PREVIOUS_LOCATION, 31);
        assert_eq!(MotionVectors::OUTPUT, 7);
        assert_eq!(MotionVectors::FLAG_STORE, 1);
        assert_eq!(MotionVectors::FLAG_LOAD, 2);
        MotionVectors::set_params_address(0x1000);
        MotionVectors::set_positions_address(0x2000);
        assert_eq!(MotionVectors::params_address(), 0x1000);
        assert_eq!(MotionVectors::positions_address(), 0x2000);
        MotionVectors::set_params_address(0);
        MotionVectors::set_positions_address(0);
    }

    #[test]
    fn the_fragment_input_helpers_read_their_flags() {
        let input = PsInput {
            param_index: 0,
            is_default: true,
            is_flat: false,
            default_value: 1,
        };
        assert!(input.is_default());
        assert!(!input.is_passthrough());
        let input = PsInput {
            param_index: 0,
            is_default: true,
            is_flat: true,
            default_value: 1,
        };
        assert!(!input.is_default());
        assert!(input.is_passthrough());
        assert_eq!(GS_MAX_OUTPUT_STREAMS, 4);
        assert_eq!(MAX_COLOR_BUFFERS, 8);
    }
}
