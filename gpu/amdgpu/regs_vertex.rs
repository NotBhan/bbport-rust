//! Vertex, primitive and tessellation registers, ported from
//! `legacy/gpu/shadps4/video_core/amdgpu/regs_vertex.h`. Every word the C++ describes with a
//! bit-field union is a raw `u32` here with the fields as accessors, and every plain field keeps
//! its name and width.

use crate::gpu::amdgpu::pixel_format::raw_enum;
use crate::gpu::common::bit_field::BitField;

raw_enum!(PrimitiveType {
    None = 0,
    PointList = 1,
    LineList = 2,
    LineStrip = 3,
    TriangleList = 4,
    TriangleFan = 5,
    TriangleStrip = 6,
    PatchPrimitive = 9,
    AdjLineList = 10,
    AdjLineStrip = 11,
    AdjTriangleList = 12,
    AdjTriangleStrip = 13,
    RectList = 17,
    LineLoop = 18,
    QuadList = 19,
    QuadStrip = 20,
    Polygon = 21,
});

raw_enum!(IndexType {
    Index16 = 0,
    Index32 = 1,
});

raw_enum!(IndexSwapMode {
    None = 0,
    Swap16 = 1,
    Swap32 = 2,
    SwapWord = 3,
});

raw_enum!(GsOutputPrimitiveType {
    PointList = 0,
    LineStrip = 1,
    TriangleStrip = 2,
});

raw_enum!(GsScenario {
    Off = 0,
    ScenarioA = 1,
    ScenarioB = 2,
    ScenarioG = 3,
    ScenarioC = 4,
});

raw_enum!(TessellationType {
    Isoline = 0,
    Triangle = 1,
    Quad = 2,
});

raw_enum!(TessellationPartitioning {
    Integer = 0,
    Pow2 = 1,
    FracOdd = 2,
    FracEven = 3,
});

raw_enum!(TessellationTopology {
    Point = 0,
    Line = 1,
    TriangleCw = 2,
    TriangleCcw = 3,
});

/// `AmdGpu::IndexBufferBase`: the high byte first, then the low word, which the address is
/// assembled from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexBufferBase {
    /// Eight bits wide in the C++.
    pub base_addr_hi: u32,
    pub base_addr_lo: u32,
}

impl IndexBufferBase {
    /// `IndexBufferBase::Address`: `(lo & ~1) | hi << 32`.
    pub fn address(&self) -> u64 {
        ((self.base_addr_lo & !1) as u64) | ((self.base_addr_hi as u64 & 0xFF) << 32)
    }
}

/// `AmdGpu::IndexBufferType`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct IndexBufferType(u32);

type IndexTypeField = BitField<u32, 0, 2>;
type IndexSwapModeField = BitField<u32, 2, 2>;

impl IndexBufferType {
    pub const fn from_raw(raw: u32) -> Self {
        IndexBufferType(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn index_type(self) -> IndexType {
        match IndexTypeField::from_raw(self.0).value() {
            0 => IndexType::Index16,
            _ => IndexType::Index32,
        }
    }

    pub fn swap_mode(self) -> IndexSwapMode {
        match IndexSwapModeField::from_raw(self.0).value() {
            0 => IndexSwapMode::None,
            1 => IndexSwapMode::Swap16,
            2 => IndexSwapMode::Swap32,
            _ => IndexSwapMode::SwapWord,
        }
    }
}

/// `AmdGpu::VgtNumInstances`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VgtNumInstances {
    pub num_instances: u32,
}

impl VgtNumInstances {
    /// `NumInstances`: zero means one.
    pub fn num_instances_or_one(&self) -> u32 {
        if self.num_instances == 0 {
            1
        } else {
            self.num_instances
        }
    }
}

/// `AmdGpu::PolygonOffset`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PolygonOffset {
    pub depth_bias: f32,
    pub front_scale: f32,
    pub front_offset: f32,
    pub back_scale: f32,
    pub back_offset: f32,
}

/// `AmdGpu::Address`: a guest address held as the top 24 bits of a word.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Address {
    pub address: u32,
}

impl Address {
    pub fn get_address(&self) -> u64 {
        (self.address as u64) << 8
    }
}

/// `AmdGpu::ShaderStageEnable`: the `VGT_STAGES` encodings the register can hold, plus the
/// enable bits themselves.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct ShaderStageEnable(u32);

/// `ShaderStageEnable::VgtStages`.
impl ShaderStageEnable {
    /// Always enabled.
    pub const VS: u32 = 0x0;
    pub const ES_GS: u32 = 0xB0;
    pub const LS_HS: u32 = 0x45;
    pub const LS_HS_ES_GS: u32 = 0xAD;
}

type VgtLsEnField = BitField<u32, 0, 2>;
type VgtHsEnField = BitField<u32, 2, 1>;
type VgtEsEnField = BitField<u32, 3, 2>;
type VgtGsEnField = BitField<u32, 5, 1>;
type VgtVsEnField = BitField<u32, 6, 2>;
type VgtDynamicHsField = BitField<u32, 8, 1>;

impl ShaderStageEnable {
    pub const fn from_raw(raw: u32) -> Self {
        ShaderStageEnable(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn ls_en(self) -> u32 {
        VgtLsEnField::from_raw(self.0).value()
    }

    pub fn hs_en(self) -> bool {
        VgtHsEnField::from_raw(self.0).value() != 0
    }

    pub fn es_en(self) -> u32 {
        VgtEsEnField::from_raw(self.0).value()
    }

    pub fn gs_en(self) -> bool {
        VgtGsEnField::from_raw(self.0).value() != 0
    }

    pub fn vs_en(self) -> u32 {
        VgtVsEnField::from_raw(self.0).value()
    }

    pub fn dynamic_hs(self) -> bool {
        VgtDynamicHsField::from_raw(self.0).value() != 0
    }

    /// `IsStageEnabled`: stages 0 and 1 are always on, the rest follow their own bit.
    pub fn is_stage_enabled(self, stage: u32) -> bool {
        match stage {
            0 | 1 => true,
            2 => self.gs_en(),
            3 => self.es_en() != 0,
            4 => self.hs_en(),
            5 => self.ls_en() != 0,
            _ => panic!("Unreachable"),
        }
    }
}

/// `AmdGpu::GsInstances`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct GsInstances(u32);

type GsInstancesEnableField = BitField<u32, 0, 2>;
type GsInstancesCountField = BitField<u32, 2, 6>;

impl GsInstances {
    pub const fn from_raw(raw: u32) -> Self {
        GsInstances(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn enable(self) -> u32 {
        GsInstancesEnableField::from_raw(self.0).value()
    }

    pub fn count(self) -> u32 {
        GsInstancesCountField::from_raw(self.0).value()
    }

    /// `IsEnabled`.
    pub fn is_enabled(self) -> bool {
        self.enable() != 0 && self.count() > 0
    }
}

/// `AmdGpu::RingOffset`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct RingOffset(u32);

type RingOffsetField = BitField<u32, 0, 15>;

impl RingOffset {
    pub const fn from_raw(raw: u32) -> Self {
        RingOffset(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn offset(self) -> u32 {
        RingOffsetField::from_raw(self.0).value()
    }

    /// `GetOffset`.
    pub fn get_offset(self) -> u32 {
        self.offset()
    }
}

/// `AmdGpu::GsOutPrimitiveType`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct GsOutPrimitiveType(u32);

type GsOutPrim0Field = BitField<u32, 0, 6>;
type GsOutPrim1Field = BitField<u32, 6, 6>;
type GsOutPrim2Field = BitField<u32, 12, 6>;
type GsOutPrim3Field = BitField<u32, 18, 6>;
type GsOutUniqueTypeField = BitField<u32, 27, 1>;

impl GsOutPrimitiveType {
    pub const fn from_raw(raw: u32) -> Self {
        GsOutPrimitiveType(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    fn primitive_of(value: u32) -> GsOutputPrimitiveType {
        match value {
            0 => GsOutputPrimitiveType::PointList,
            1 => GsOutputPrimitiveType::LineStrip,
            _ => GsOutputPrimitiveType::TriangleStrip,
        }
    }

    pub fn outprim_type(self) -> GsOutputPrimitiveType {
        Self::primitive_of(GsOutPrim0Field::from_raw(self.0).value())
    }

    pub fn outprim_type1(self) -> GsOutputPrimitiveType {
        Self::primitive_of(GsOutPrim1Field::from_raw(self.0).value())
    }

    pub fn outprim_type2(self) -> GsOutputPrimitiveType {
        Self::primitive_of(GsOutPrim2Field::from_raw(self.0).value())
    }

    pub fn outprim_type3(self) -> GsOutputPrimitiveType {
        Self::primitive_of(GsOutPrim3Field::from_raw(self.0).value())
    }

    pub fn unique_type_per_stream(self) -> bool {
        GsOutUniqueTypeField::from_raw(self.0).value() != 0
    }

    /// `GetPrimitiveType`: the first stream's type unless the register gives each its own.
    pub fn get_primitive_type(self, stream: u32) -> GsOutputPrimitiveType {
        if !self.unique_type_per_stream() {
            return self.outprim_type();
        }
        match stream {
            0 => self.outprim_type(),
            1 => self.outprim_type1(),
            2 => self.outprim_type2(),
            3 => self.outprim_type3(),
            _ => panic!("Unreachable"),
        }
    }
}

/// `AmdGpu::GsMode`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct GsMode(u32);

type GsModeScenarioField = BitField<u32, 0, 3>;
type GsModeCutModeField = BitField<u32, 4, 2>;
type GsModeGsCPackEnField = BitField<u32, 10, 1>;
type GsModeEsPassthruField = BitField<u32, 13, 1>;
type GsModeComputeModeField = BitField<u32, 14, 1>;
type GsModeFastComputeField = BitField<u32, 15, 1>;
type GsModeElementInfoField = BitField<u32, 16, 1>;
type GsModePartialThdField = BitField<u32, 17, 1>;
type GsModeSuppressCutsField = BitField<u32, 18, 1>;
type GsModeEsWriteOptField = BitField<u32, 19, 1>;
type GsModeGsWriteOptField = BitField<u32, 20, 1>;
type GsModeOnchipField = BitField<u32, 21, 2>;

impl GsMode {
    pub const fn from_raw(raw: u32) -> Self {
        GsMode(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn mode(self) -> GsScenario {
        match GsModeScenarioField::from_raw(self.0).value() {
            0 => GsScenario::Off,
            1 => GsScenario::ScenarioA,
            2 => GsScenario::ScenarioB,
            3 => GsScenario::ScenarioG,
            _ => GsScenario::ScenarioC,
        }
    }

    pub fn cut_mode(self) -> u32 {
        GsModeCutModeField::from_raw(self.0).value()
    }

    pub fn gs_c_pack_en(self) -> bool {
        GsModeGsCPackEnField::from_raw(self.0).value() != 0
    }

    pub fn es_passthru(self) -> bool {
        GsModeEsPassthruField::from_raw(self.0).value() != 0
    }

    pub fn compute_mode(self) -> bool {
        GsModeComputeModeField::from_raw(self.0).value() != 0
    }

    pub fn fast_compute_mode(self) -> bool {
        GsModeFastComputeField::from_raw(self.0).value() != 0
    }

    pub fn element_info_en(self) -> bool {
        GsModeElementInfoField::from_raw(self.0).value() != 0
    }

    pub fn partial_thd_at_eoi(self) -> bool {
        GsModePartialThdField::from_raw(self.0).value() != 0
    }

    pub fn suppress_cuts(self) -> bool {
        GsModeSuppressCutsField::from_raw(self.0).value() != 0
    }

    pub fn es_write_optimize(self) -> bool {
        GsModeEsWriteOptField::from_raw(self.0).value() != 0
    }

    pub fn gs_write_optimize(self) -> bool {
        GsModeGsWriteOptField::from_raw(self.0).value() != 0
    }

    pub fn onchip(self) -> u32 {
        GsModeOnchipField::from_raw(self.0).value()
    }
}

/// `AmdGpu::GsOnchip`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct GsOnchip(u32);

type GsOnchipEsVertsField = BitField<u32, 0, 11>;
type GsOnchipGsPrimsField = BitField<u32, 11, 11>;

impl GsOnchip {
    pub const fn from_raw(raw: u32) -> Self {
        GsOnchip(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn es_verts_per_subgroup(self) -> u32 {
        GsOnchipEsVertsField::from_raw(self.0).value()
    }

    pub fn gs_prims_per_subgroup(self) -> u32 {
        GsOnchipGsPrimsField::from_raw(self.0).value()
    }
}

/// `AmdGpu::StreamOutControl`: one bit, the rest reserved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct StreamOutControl(u32);

type StreamOutControlDoneField = BitField<u32, 0, 1>;

impl StreamOutControl {
    pub const fn from_raw(raw: u32) -> Self {
        StreamOutControl(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn offset_update_done(self) -> bool {
        StreamOutControlDoneField::from_raw(self.0).value() != 0
    }
}

/// `AmdGpu::StreamOutConfig`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct StreamOutConfig(u32);

type StreamOut0EnField = BitField<u32, 0, 1>;
type StreamOut1EnField = BitField<u32, 1, 1>;
type StreamOut2EnField = BitField<u32, 2, 1>;
type StreamOut3EnField = BitField<u32, 3, 1>;
type StreamOutRastStreamField = BitField<u32, 4, 3>;
type StreamOutRastMaskField = BitField<u32, 8, 4>;
type StreamOutUseMaskField = BitField<u32, 31, 1>;

impl StreamOutConfig {
    pub const fn from_raw(raw: u32) -> Self {
        StreamOutConfig(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn streamout_0_en(self) -> bool {
        StreamOut0EnField::from_raw(self.0).value() != 0
    }

    pub fn streamout_1_en(self) -> bool {
        StreamOut1EnField::from_raw(self.0).value() != 0
    }

    pub fn streamout_2_en(self) -> bool {
        StreamOut2EnField::from_raw(self.0).value() != 0
    }

    pub fn streamout_3_en(self) -> bool {
        StreamOut3EnField::from_raw(self.0).value() != 0
    }

    pub fn rast_stream(self) -> u32 {
        StreamOutRastStreamField::from_raw(self.0).value()
    }

    pub fn rast_stream_mask(self) -> u32 {
        StreamOutRastMaskField::from_raw(self.0).value()
    }

    pub fn use_rast_stream_mask(self) -> bool {
        StreamOutUseMaskField::from_raw(self.0).value() != 0
    }
}

/// `AmdGpu::StreamOutBufferConfig`: four nibbles, one per stream.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct StreamOutBufferConfig(u32);

type StreamOutBuf0Field = BitField<u32, 0, 4>;
type StreamOutBuf1Field = BitField<u32, 4, 4>;
type StreamOutBuf2Field = BitField<u32, 8, 4>;
type StreamOutBuf3Field = BitField<u32, 12, 4>;

impl StreamOutBufferConfig {
    pub const fn from_raw(raw: u32) -> Self {
        StreamOutBufferConfig(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn stream_0_buf_en(self) -> u32 {
        StreamOutBuf0Field::from_raw(self.0).value()
    }

    pub fn stream_1_buf_en(self) -> u32 {
        StreamOutBuf1Field::from_raw(self.0).value()
    }

    pub fn stream_2_buf_en(self) -> u32 {
        StreamOutBuf2Field::from_raw(self.0).value()
    }

    pub fn stream_3_buf_en(self) -> u32 {
        StreamOutBuf3Field::from_raw(self.0).value()
    }
}

/// `AmdGpu::LsHsConfig`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct LsHsConfig(u32);

type LsHsNumPatchesField = BitField<u32, 0, 8>;
type LsHsInputCpField = BitField<u32, 8, 6>;
type LsHsOutputCpField = BitField<u32, 14, 6>;

impl LsHsConfig {
    pub const fn from_raw(raw: u32) -> Self {
        LsHsConfig(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn num_patches(self) -> u32 {
        LsHsNumPatchesField::from_raw(self.0).value()
    }

    pub fn hs_input_control_points(self) -> u32 {
        LsHsInputCpField::from_raw(self.0).value()
    }

    pub fn hs_output_control_points(self) -> u32 {
        LsHsOutputCpField::from_raw(self.0).value()
    }
}

/// `AmdGpu::TessellationConfig`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct TessellationConfig(u32);

type TessTypeField = BitField<u32, 0, 2>;
type TessPartitioningField = BitField<u32, 2, 3>;
type TessTopologyField = BitField<u32, 5, 3>;

impl TessellationConfig {
    pub const fn from_raw(raw: u32) -> Self {
        TessellationConfig(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn type_(self) -> TessellationType {
        match TessTypeField::from_raw(self.0).value() {
            0 => TessellationType::Isoline,
            1 => TessellationType::Triangle,
            _ => TessellationType::Quad,
        }
    }

    pub fn partitioning(self) -> TessellationPartitioning {
        match TessPartitioningField::from_raw(self.0).value() {
            0 => TessellationPartitioning::Integer,
            1 => TessellationPartitioning::Pow2,
            2 => TessellationPartitioning::FracOdd,
            _ => TessellationPartitioning::FracEven,
        }
    }

    pub fn topology(self) -> TessellationTopology {
        match TessTopologyField::from_raw(self.0).value() {
            0 => TessellationTopology::Point,
            1 => TessellationTopology::Line,
            2 => TessellationTopology::TriangleCw,
            _ => TessellationTopology::TriangleCcw,
        }
    }
}

/// `AmdGpu::TessFactorMemoryBase`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TessFactorMemoryBase {
    pub base: u32,
}

impl TessFactorMemoryBase {
    pub fn memory_base(&self) -> u64 {
        (self.base as u64) << 8
    }
}

/// `AmdGpu::TessFactorClamp`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TessFactorClamp {
    pub hs_max_tess: f32,
    pub hs_min_tess: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_enumerations_keep_their_values() {
        assert_eq!(PrimitiveType::TriangleList.raw(), 4);
        assert_eq!(PrimitiveType::PatchPrimitive.raw(), 9);
        assert_eq!(PrimitiveType::Polygon.raw(), 21);
        assert_eq!(PrimitiveType::from_raw(7), None, "the gap at 7 has no name");
        assert_eq!(IndexType::Index16.raw(), 0);
        assert_eq!(IndexSwapMode::SwapWord.raw(), 3);
        assert_eq!(GsOutputPrimitiveType::LineStrip.raw(), 1);
        assert_eq!(GsScenario::ScenarioG.raw(), 3);
        assert_eq!(TessellationType::Quad.raw(), 2);
        assert_eq!(TessellationPartitioning::FracEven.raw(), 3);
        assert_eq!(TessellationTopology::TriangleCcw.raw(), 3);
    }

    #[test]
    fn the_address_forms_shift_into_place() {
        let base = IndexBufferBase {
            base_addr_hi: 0x12,
            base_addr_lo: 0x1234_5679,
        };
        assert_eq!(base.address(), 0x12_1234_5678, "the low bit is dropped");
        let address = Address {
            address: 0x0012_3456,
        };
        assert_eq!(address.get_address(), 0x1234_5600);
        let factors = TessFactorMemoryBase { base: 0x10 };
        assert_eq!(factors.memory_base(), 0x1000);
    }

    #[test]
    fn the_shader_stage_enable_reads_its_bits() {
        let stages = ShaderStageEnable::from_raw(0);
        assert!(stages.is_stage_enabled(0));
        assert!(stages.is_stage_enabled(1));
        assert!(!stages.is_stage_enabled(2));
        assert!(!stages.is_stage_enabled(3));
        assert!(!stages.is_stage_enabled(4));
        assert!(!stages.is_stage_enabled(5));
        // hs_en is bit 2, es_en bits 3..5, gs_en bit 5, ls_en bits 0..2.
        let stages = ShaderStageEnable::from_raw(1 << 2 | 3 << 3 | 1 << 5 | 2);
        assert!(stages.hs_en());
        assert_eq!(stages.es_en(), 3);
        assert!(stages.gs_en());
        assert_eq!(stages.ls_en(), 2);
        assert!(stages.is_stage_enabled(4));
        assert!(stages.is_stage_enabled(5));
        assert!(!stages.dynamic_hs());
        // The register's encoded stages are named constants.
        assert_eq!(ShaderStageEnable::VS, 0);
        assert_eq!(ShaderStageEnable::LS_HS_ES_GS, 0xAD);
        assert!(ShaderStageEnable::from_raw(ShaderStageEnable::ES_GS).gs_en());
    }

    #[test]
    fn the_instance_and_primitive_helpers_follow_the_header() {
        let instances = GsInstances::from_raw(1 | 4 << 2);
        assert_eq!(instances.enable(), 1);
        assert_eq!(instances.count(), 4);
        assert!(instances.is_enabled());
        assert!(!GsInstances::from_raw(0).is_enabled());
        assert!(
            !GsInstances::from_raw(1).is_enabled(),
            "a zero count is off"
        );

        // One type for all streams.
        let primitive = GsOutPrimitiveType::from_raw(2);
        for stream in 0..4 {
            assert_eq!(
                primitive.get_primitive_type(stream),
                GsOutputPrimitiveType::TriangleStrip
            );
        }
        // A type per stream.
        let primitive = GsOutPrimitiveType::from_raw(1 << 6 | 2 << 12 | 3 << 18 | 1 << 27);
        assert!(primitive.unique_type_per_stream());
        assert_eq!(
            primitive.get_primitive_type(0),
            GsOutputPrimitiveType::PointList
        );
        assert_eq!(
            primitive.get_primitive_type(1),
            GsOutputPrimitiveType::LineStrip
        );
        assert_eq!(
            primitive.get_primitive_type(2),
            GsOutputPrimitiveType::TriangleStrip
        );
        assert_eq!(
            primitive.get_primitive_type(3),
            GsOutputPrimitiveType::TriangleStrip
        );

        let instances = VgtNumInstances { num_instances: 0 };
        assert_eq!(instances.num_instances_or_one(), 1);
        assert_eq!(
            VgtNumInstances { num_instances: 5 }.num_instances_or_one(),
            5
        );
    }

    #[test]
    fn the_gs_mode_and_stream_out_words_read_their_fields() {
        let mode = GsMode::from_raw(3 | 2 << 4 | 1 << 10 | 1 << 13 | 1 << 18 | 2 << 21);
        assert_eq!(mode.mode(), GsScenario::ScenarioG);
        assert_eq!(mode.cut_mode(), 2);
        assert!(mode.gs_c_pack_en());
        assert!(mode.es_passthru());
        assert!(!mode.compute_mode());
        assert!(mode.suppress_cuts());
        assert_eq!(mode.onchip(), 2);

        let onchip = GsOnchip::from_raw(7 | 9 << 11);
        assert_eq!(onchip.es_verts_per_subgroup(), 7);
        assert_eq!(onchip.gs_prims_per_subgroup(), 9);

        let config = StreamOutConfig::from_raw(1 | 1 << 3 | 5 << 4 | 3 << 8 | 1 << 31);
        assert!(config.streamout_0_en());
        assert!(!config.streamout_1_en());
        assert!(config.streamout_3_en());
        assert_eq!(config.rast_stream(), 5);
        assert_eq!(config.rast_stream_mask(), 3);
        assert!(config.use_rast_stream_mask());
        assert!(StreamOutControl::from_raw(1).offset_update_done());
        assert!(!StreamOutControl::from_raw(2).offset_update_done());

        let buffers = StreamOutBufferConfig::from_raw(1 | 2 << 4 | 3 << 8 | 4 << 12);
        assert_eq!(buffers.stream_0_buf_en(), 1);
        assert_eq!(buffers.stream_1_buf_en(), 2);
        assert_eq!(buffers.stream_2_buf_en(), 3);
        assert_eq!(buffers.stream_3_buf_en(), 4);
    }

    #[test]
    fn the_tessellation_words_read_their_fields() {
        let config = LsHsConfig::from_raw(4 | 3 << 8 | 5 << 14);
        assert_eq!(config.num_patches(), 4);
        assert_eq!(config.hs_input_control_points(), 3);
        assert_eq!(config.hs_output_control_points(), 5);
        let tess = TessellationConfig::from_raw(2 | 1 << 2 | 2 << 5);
        assert_eq!(tess.type_(), TessellationType::Quad);
        assert_eq!(tess.partitioning(), TessellationPartitioning::Pow2);
        assert_eq!(tess.topology(), TessellationTopology::TriangleCw);
        let clamp = TessFactorClamp {
            hs_max_tess: 8.0,
            hs_min_tess: 1.0,
        };
        assert_eq!(clamp.hs_max_tess, 8.0);
        let offset = RingOffset::from_raw(0x1234_5678);
        assert_eq!(offset.get_offset(), 0x5678 & 0x7FFF);
        assert_eq!(
            IndexBufferType::from_raw(1 | 2 << 2).index_type(),
            IndexType::Index32
        );
        assert_eq!(
            IndexBufferType::from_raw(2 << 2).swap_mode(),
            IndexSwapMode::Swap32
        );
        let polygon = PolygonOffset {
            depth_bias: 0.5,
            front_scale: 1.0,
            front_offset: 0.0,
            back_scale: 1.0,
            back_offset: 0.0,
        };
        assert_eq!(polygon.depth_bias, 0.5);
    }
}
