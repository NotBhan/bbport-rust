//! Shader program and export registers, ported from
//! `legacy/gpu/shadps4/video_core/amdgpu/regs_shader.h`. The C++ reads a `BinaryInfo` straight
//! out of the shader's code with `std::bit_cast`; the port lays the same bytes out with a
//! parser, so the signature check and the field widths are what the C++ has.

use crate::gpu::amdgpu::pixel_format::raw_enum;
use crate::gpu::common::bit_field::BitField;
use crate::gpu::shader_recompiler::params::ShaderParams;

raw_enum!(FpRoundMode {
    NearestEven = 0,
    PlusInf = 1,
    MinInf = 2,
    ToZero = 3,
});

raw_enum!(FpDenormMode {
    InOutFlush = 0,
    InAllowOutFlush = 1,
    InFlushOutAllow = 2,
    InOutAllow = 3,
});

raw_enum!(ShaderExportComp {
    None = 0,
    OneComp = 1,
    TwoComp = 2,
    FourCompCompressed = 3,
    FourComp = 4,
});

raw_enum!(ShaderExportFormat {
    Zero = 0,
    R_32 = 1,
    GR_32 = 2,
    AR_32 = 3,
    ABGR_FP16 = 4,
    ABGR_UNORM16 = 5,
    ABGR_SNORM16 = 6,
    ABGR_UINT16 = 7,
    ABGR_SINT16 = 8,
    ABGR_32 = 9,
});

/// `AmdGpu::NUM_USER_DATA`.
pub const NUM_USER_DATA: usize = 16;

/// `AmdGpu::UserData`.
pub type UserData = [u32; NUM_USER_DATA];

/// `AmdGpu::BinaryInfo`: the shader's own header, which sits in its code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryInfo {
    pub signature: [u8; 7],
    pub version: u8,
    pub pssl_or_cg: bool,
    pub cached: bool,
    pub type_: u32,
    pub source_type: u32,
    pub length: u32,
    pub chunk_usage_base_offset_in_dw: u8,
    pub num_input_usage_slots: u8,
    pub is_srt: bool,
    pub is_srt_used_info_valid: bool,
    pub is_extended_usage_info: bool,
    pub reserved2: u32,
    pub reserved3: u8,
    pub shader_hash: u64,
    pub crc32: u32,
}

impl BinaryInfo {
    /// `BinaryInfo::signature_ref`, which spells `OrbShdr`.
    pub const SIGNATURE_REF: [u8; 7] = [0x4f, 0x72, 0x62, 0x53, 0x68, 0x64, 0x72];

    /// The header as it lies in a word stream, which is where the C++'s `bit_cast` reads it.
    pub fn from_words(words: &[u32]) -> BinaryInfo {
        let mut bytes = [0u8; 28];
        for (index, byte) in bytes.iter_mut().enumerate() {
            let word = words[index / 4];
            *byte = (word >> ((index % 4) * 8)) as u8;
        }
        let word = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let flags = bytes[14];
        BinaryInfo {
            signature: bytes[0..7].try_into().unwrap(),
            version: bytes[7],
            pssl_or_cg: word & 1 != 0,
            cached: word & (1 << 1) != 0,
            type_: (word >> 2) & 0xF,
            source_type: (word >> 6) & 0x3,
            length: word >> 8,
            chunk_usage_base_offset_in_dw: bytes[12],
            num_input_usage_slots: bytes[13],
            is_srt: flags & 1 != 0,
            is_srt_used_info_valid: flags & (1 << 1) != 0,
            is_extended_usage_info: flags & (1 << 2) != 0,
            reserved2: u32::from(flags >> 3),
            reserved3: bytes[15],
            shader_hash: u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
            crc32: u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
        }
    }

    /// `BinaryInfo::Valid`.
    pub fn is_valid(&self) -> bool {
        self.signature == Self::SIGNATURE_REF
    }
}

/// The words a header needs, which is what keeps the port's scan in bounds.
const HEADER_WORDS: usize = 7;

/// `AmdGpu::SearchBinaryInfo`: the header for a shader's code, either right after the
/// `s_mov_b32 vcc_hi, #imm` prologue or found by scanning the first 16k words.
pub fn search_binary_info(code: &[u32]) -> BinaryInfo {
    const TOKEN_MOV_VCCHI: u32 = 0xBEEB_03FF;
    if code.first() == Some(&TOKEN_MOV_VCCHI) && code.len() > 1 {
        let offset = (code[1] as usize + 1) * 2;
        if offset + HEADER_WORDS <= code.len() {
            let info = BinaryInfo::from_words(&code[offset..]);
            if info.is_valid() {
                return info;
            }
        }
    }
    // The C++ scans whole words and reads a header at each, past the end of the code if need
    // be; the port stops at the last place a header fits.
    const SEARCH_LIMIT: usize = 0x4000;
    let last = SEARCH_LIMIT.min(code.len().saturating_sub(HEADER_WORDS - 1));
    for index in 0..last {
        let info = BinaryInfo::from_words(&code[index..]);
        if info.is_valid() {
            return info;
        }
    }
    panic!("Shader binary info not found.");
}

/// `AmdGpu::ShaderProgram`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ShaderProgram {
    /// Forty bits wide in the C++.
    pub address: u64,
    pub settings: ShaderProgramSettings,
    pub user_data: UserData,
}

impl ShaderProgram {
    /// `ShaderProgram::Address`: the guest address the program sits at. The C++ returns it as a
    /// pointer; the port has no guest memory to point into, so it stays an address.
    pub fn address(&self) -> u64 {
        (self.address & 0xFF_FFFF_FFFF) << 8
    }

    /// `ShaderProgram::NumVgprs`: each increment allocates four registers, where zero means four.
    pub fn num_vgprs(&self) -> u32 {
        (self.settings.num_vgprs() + 1) * 4
    }
}

/// The anonymous `settings` block of `ShaderProgram`, whose fields span two words.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct ShaderProgramSettings(u64);

type ProgramNumVgprsField = BitField<u64, 0, 6>;
type ProgramNumSgprsField = BitField<u64, 6, 4>;
type ProgramPriorityField = BitField<u64, 10, 2>;
type ProgramRoundMode32Field = BitField<u64, 12, 2>;
type ProgramRoundMode64Field = BitField<u64, 14, 2>;
type ProgramDenormMode32Field = BitField<u64, 16, 2>;
type ProgramDenormMode64Field = BitField<u64, 18, 2>;
type ProgramVgprCompCntField = BitField<u64, 24, 2>;
type ProgramScratchEnField = BitField<u64, 32, 1>;
type ProgramNumUserRegsField = BitField<u64, 33, 5>;
type ProgramOcLdsEnField = BitField<u64, 39, 1>;

impl ShaderProgramSettings {
    pub const fn from_raw(raw: u64) -> Self {
        ShaderProgramSettings(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub fn num_vgprs(self) -> u32 {
        ProgramNumVgprsField::from_raw(self.0).value() as u32
    }

    pub fn num_sgprs(self) -> u32 {
        ProgramNumSgprsField::from_raw(self.0).value() as u32
    }

    pub fn priority(self) -> u32 {
        ProgramPriorityField::from_raw(self.0).value() as u32
    }

    pub fn fp_round_mode32(self) -> FpRoundMode {
        match ProgramRoundMode32Field::from_raw(self.0).value() {
            0 => FpRoundMode::NearestEven,
            1 => FpRoundMode::PlusInf,
            2 => FpRoundMode::MinInf,
            _ => FpRoundMode::ToZero,
        }
    }

    pub fn fp_round_mode64(self) -> FpRoundMode {
        match ProgramRoundMode64Field::from_raw(self.0).value() {
            0 => FpRoundMode::NearestEven,
            1 => FpRoundMode::PlusInf,
            2 => FpRoundMode::MinInf,
            _ => FpRoundMode::ToZero,
        }
    }

    pub fn fp_denorm_mode32(self) -> FpDenormMode {
        match ProgramDenormMode32Field::from_raw(self.0).value() {
            0 => FpDenormMode::InOutFlush,
            1 => FpDenormMode::InAllowOutFlush,
            2 => FpDenormMode::InFlushOutAllow,
            _ => FpDenormMode::InOutAllow,
        }
    }

    pub fn fp_denorm_mode64(self) -> FpDenormMode {
        match ProgramDenormMode64Field::from_raw(self.0).value() {
            0 => FpDenormMode::InOutFlush,
            1 => FpDenormMode::InAllowOutFlush,
            2 => FpDenormMode::InFlushOutAllow,
            _ => FpDenormMode::InOutAllow,
        }
    }

    pub fn vgpr_comp_cnt(self) -> u32 {
        ProgramVgprCompCntField::from_raw(self.0).value() as u32
    }

    pub fn scratch_en(self) -> bool {
        ProgramScratchEnField::from_raw(self.0).value() != 0
    }

    pub fn num_user_regs(self) -> u32 {
        ProgramNumUserRegsField::from_raw(self.0).value() as u32
    }

    pub fn oc_lds_en(self) -> bool {
        ProgramOcLdsEnField::from_raw(self.0).value() != 0
    }
}

/// `AmdGpu::VsOutputConfig`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct VsOutputConfig(u32);

type VsExportCountField = BitField<u32, 1, 5>;
type VsHalfPackField = BitField<u32, 6, 1>;

impl VsOutputConfig {
    pub const fn from_raw(raw: u32) -> Self {
        VsOutputConfig(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn export_count_min_one(self) -> u32 {
        VsExportCountField::from_raw(self.0).value()
    }

    pub fn half_pack(self) -> bool {
        VsHalfPackField::from_raw(self.0).value() != 0
    }

    /// `VsOutputConfig::NumExports`.
    pub fn num_exports(self) -> u32 {
        self.export_count_min_one() + 1
    }
}

/// `AmdGpu::PsInputControl`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct PsInputControl(u32);

type PsInputOffsetField = BitField<u32, 0, 5>;
type PsInputUseDefaultField = BitField<u32, 5, 1>;
type PsInputDefaultValueField = BitField<u32, 7, 2>;
type PsInputFlatShadeField = BitField<u32, 9, 1>;

impl PsInputControl {
    pub const fn from_raw(raw: u32) -> Self {
        PsInputControl(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn input_offset(self) -> u32 {
        PsInputOffsetField::from_raw(self.0).value()
    }

    pub fn use_default(self) -> bool {
        PsInputUseDefaultField::from_raw(self.0).value() != 0
    }

    pub fn default_value(self) -> u32 {
        PsInputDefaultValueField::from_raw(self.0).value()
    }

    pub fn flat_shade(self) -> bool {
        PsInputFlatShadeField::from_raw(self.0).value() != 0
    }
}

/// `AmdGpu::PsInput`: which interpolation modes the fragment shader reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct PsInput(u32);

impl PsInput {
    pub const fn from_raw(raw: u32) -> Self {
        PsInput(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    fn bit(self, index: u32) -> bool {
        self.0 & (1 << index) != 0
    }

    pub fn persp_sample_ena(self) -> bool {
        self.bit(0)
    }

    pub fn persp_center_ena(self) -> bool {
        self.bit(1)
    }

    pub fn persp_centroid_ena(self) -> bool {
        self.bit(2)
    }

    pub fn persp_pull_model_ena(self) -> bool {
        self.bit(3)
    }

    pub fn linear_sample_ena(self) -> bool {
        self.bit(4)
    }

    pub fn linear_center_ena(self) -> bool {
        self.bit(5)
    }

    pub fn linear_centroid_ena(self) -> bool {
        self.bit(6)
    }

    pub fn line_stipple_tex_ena(self) -> bool {
        self.bit(7)
    }

    pub fn pos_x_float_ena(self) -> bool {
        self.bit(8)
    }

    pub fn pos_y_float_ena(self) -> bool {
        self.bit(9)
    }

    pub fn pos_z_float_ena(self) -> bool {
        self.bit(10)
    }

    pub fn pos_w_float_ena(self) -> bool {
        self.bit(11)
    }

    pub fn front_face_ena(self) -> bool {
        self.bit(12)
    }

    pub fn ancillary_ena(self) -> bool {
        self.bit(13)
    }

    pub fn sample_coverage_ena(self) -> bool {
        self.bit(14)
    }

    pub fn pos_fixed_pt_ena(self) -> bool {
        self.bit(15)
    }
}

/// `AmdGpu::BarycentricControl`: one bit, the rest reserved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct BarycentricControl(u32);

type BarycentricFrontFaceAllField = BitField<u32, 24, 1>;

impl BarycentricControl {
    pub const fn from_raw(raw: u32) -> Self {
        BarycentricControl(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn front_face_all_bits(self) -> bool {
        BarycentricFrontFaceAllField::from_raw(self.0).value() != 0
    }
}

/// `AmdGpu::ShaderPosFormat`: four four-bit export formats.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct ShaderPosFormat(u16);

type Pos0Field = BitField<u16, 0, 4>;
type Pos1Field = BitField<u16, 4, 4>;
type Pos2Field = BitField<u16, 8, 4>;
type Pos3Field = BitField<u16, 12, 4>;

impl ShaderPosFormat {
    pub const fn from_raw(raw: u16) -> Self {
        ShaderPosFormat(raw)
    }

    pub const fn raw(self) -> u16 {
        self.0
    }

    fn component(value: u16) -> ShaderExportComp {
        match value {
            0 => ShaderExportComp::None,
            1 => ShaderExportComp::OneComp,
            2 => ShaderExportComp::TwoComp,
            3 => ShaderExportComp::FourCompCompressed,
            _ => ShaderExportComp::FourComp,
        }
    }

    pub fn pos0(self) -> ShaderExportComp {
        Self::component(Pos0Field::from_raw(self.0).value())
    }

    pub fn pos1(self) -> ShaderExportComp {
        Self::component(Pos1Field::from_raw(self.0).value())
    }

    pub fn pos2(self) -> ShaderExportComp {
        Self::component(Pos2Field::from_raw(self.0).value())
    }

    pub fn pos3(self) -> ShaderExportComp {
        Self::component(Pos3Field::from_raw(self.0).value())
    }
}

/// `AmdGpu::ColorExportFormat`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ColorExportFormat {
    pub raw: u32,
}

impl ColorExportFormat {
    /// `ColorExportFormat::GetFormat`: four bits per buffer.
    pub fn get_format(&self, buf_idx: u32) -> ShaderExportFormat {
        let value = (self.raw >> (buf_idx * 4)) & 0xF;
        ShaderExportFormat::from_raw(value).expect("Reachable")
    }
}

/// `AmdGpu::ComputeProgram`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComputeProgram {
    pub dispatch_initiator: u32,
    pub dim_x: u32,
    pub dim_y: u32,
    pub dim_z: u32,
    pub start_x: u32,
    pub start_y: u32,
    pub start_z: u32,
    pub num_thread_x: ProgramThreads,
    pub num_thread_y: ProgramThreads,
    pub num_thread_z: ProgramThreads,
    pub pad0: u32,
    pub max_wave_id: u32,
    /// Forty bits wide in the C++.
    pub address: u64,
    pub pad1: [u32; 4],
    pub settings: ComputeProgramSettings,
    pub pad2: u32,
    pub resource_limits: u32,
    pub pad3: [u32; 42],
    pub user_data: UserData,
}

impl ComputeProgram {
    /// `ComputeProgram::Address`: the guest address, for the reason `ShaderProgram::address`
    /// gives.
    pub fn address(&self) -> u64 {
        (self.address & 0xFF_FFFF_FFFF) << 8
    }

    /// `ComputeProgram::SharedMemSize`: `lds_dwords` counts units of 128 dwords and the result
    /// is in bytes.
    pub fn shared_mem_size(&self) -> u32 {
        self.settings.lds_dwords() * 128 * 4
    }

    /// `ComputeProgram::NumWorkgroups`.
    pub fn num_workgroups(&self) -> u32 {
        self.dim_x * self.dim_y * self.dim_z
    }

    /// `ComputeProgram::IsTgidEnabled`.
    pub fn is_tgid_enabled(&self, i: u32) -> bool {
        (self.settings.tgid_enable() >> i) & 1 != 0
    }
}

impl Default for ComputeProgram {
    fn default() -> Self {
        ComputeProgram {
            dispatch_initiator: 0,
            dim_x: 0,
            dim_y: 0,
            dim_z: 0,
            start_x: 0,
            start_y: 0,
            start_z: 0,
            num_thread_x: ProgramThreads::default(),
            num_thread_y: ProgramThreads::default(),
            num_thread_z: ProgramThreads::default(),
            pad0: 0,
            max_wave_id: 0,
            address: 0,
            pad1: [0; 4],
            settings: ComputeProgramSettings::default(),
            pad2: 0,
            resource_limits: 0,
            pad3: [0; 42],
            user_data: [0; NUM_USER_DATA],
        }
    }
}

/// The `full`/`partial` pair `ComputeProgram` holds per dimension.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProgramThreads {
    pub full: u16,
    pub partial: u16,
}

/// The anonymous `settings` block of `ComputeProgram`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct ComputeProgramSettings(u64);

type ComputeNumVgprsField = BitField<u64, 0, 6>;
type ComputeNumSgprsField = BitField<u64, 6, 4>;
type ComputeRoundMode32Field = BitField<u64, 12, 2>;
type ComputeRoundMode64Field = BitField<u64, 14, 2>;
type ComputeDenormMode32Field = BitField<u64, 16, 2>;
type ComputeDenormMode64Field = BitField<u64, 18, 2>;
type ComputeScratchEnField = BitField<u64, 32, 1>;
type ComputeNumUserRegsField = BitField<u64, 33, 5>;
type ComputeTgidEnableField = BitField<u64, 39, 3>;
type ComputeLdsDwordsField = BitField<u64, 47, 9>;

impl ComputeProgramSettings {
    pub const fn from_raw(raw: u64) -> Self {
        ComputeProgramSettings(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub fn num_vgprs(self) -> u32 {
        ComputeNumVgprsField::from_raw(self.0).value() as u32
    }

    pub fn num_sgprs(self) -> u32 {
        ComputeNumSgprsField::from_raw(self.0).value() as u32
    }

    pub fn fp_round_mode32(self) -> FpRoundMode {
        match ComputeRoundMode32Field::from_raw(self.0).value() {
            0 => FpRoundMode::NearestEven,
            1 => FpRoundMode::PlusInf,
            2 => FpRoundMode::MinInf,
            _ => FpRoundMode::ToZero,
        }
    }

    pub fn fp_round_mode64(self) -> FpRoundMode {
        match ComputeRoundMode64Field::from_raw(self.0).value() {
            0 => FpRoundMode::NearestEven,
            1 => FpRoundMode::PlusInf,
            2 => FpRoundMode::MinInf,
            _ => FpRoundMode::ToZero,
        }
    }

    pub fn fp_denorm_mode32(self) -> FpDenormMode {
        match ComputeDenormMode32Field::from_raw(self.0).value() {
            0 => FpDenormMode::InOutFlush,
            1 => FpDenormMode::InAllowOutFlush,
            2 => FpDenormMode::InFlushOutAllow,
            _ => FpDenormMode::InOutAllow,
        }
    }

    pub fn fp_denorm_mode64(self) -> FpDenormMode {
        match ComputeDenormMode64Field::from_raw(self.0).value() {
            0 => FpDenormMode::InOutFlush,
            1 => FpDenormMode::InAllowOutFlush,
            2 => FpDenormMode::InFlushOutAllow,
            _ => FpDenormMode::InOutAllow,
        }
    }

    pub fn scratch_en(self) -> bool {
        ComputeScratchEnField::from_raw(self.0).value() != 0
    }

    pub fn num_user_regs(self) -> u32 {
        ComputeNumUserRegsField::from_raw(self.0).value() as u32
    }

    pub fn tgid_enable(self) -> u32 {
        ComputeTgidEnableField::from_raw(self.0).value() as u32
    }

    pub fn lds_dwords(self) -> u32 {
        ComputeLdsDwordsField::from_raw(self.0).value() as u32
    }
}

/// `AmdGpu::GetParams`: the parameters a program's code and user data make up.
pub fn get_params<'a>(code: &'a [u32], user_data: UserData) -> ShaderParams<'a> {
    let bininfo = search_binary_info(code);
    let length = bininfo.length as usize / std::mem::size_of::<u32>();
    ShaderParams::new(
        user_data,
        &code[..length.min(code.len())],
        bininfo.shader_hash,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A header with a valid signature, as the words a shader would hold it in.
    fn header_words(length: u32, hash: u64, crc: u32) -> [u32; 7] {
        let mut bytes = [0u8; 28];
        bytes[0..7].copy_from_slice(&BinaryInfo::SIGNATURE_REF);
        bytes[7] = 3;
        // pssl_or_cg, type 5, source_type 1, length in the top 24 bits.
        let word = 1u32 | (5 << 2) | (1 << 6) | (length << 8);
        bytes[8..12].copy_from_slice(&word.to_le_bytes());
        bytes[12] = 4;
        bytes[13] = 9;
        bytes[14] = 0b101;
        bytes[15] = 0x77;
        bytes[16..24].copy_from_slice(&hash.to_le_bytes());
        bytes[24..28].copy_from_slice(&crc.to_le_bytes());
        let mut words = [0u32; 7];
        for (index, word) in words.iter_mut().enumerate() {
            *word = u32::from_le_bytes(bytes[index * 4..index * 4 + 4].try_into().unwrap());
        }
        words
    }

    #[test]
    fn the_binary_header_reads_its_fields() {
        let words = header_words(0x40, 0x1234_5678_9ABC_DEF0, 0xDEAD_BEEF);
        let info = BinaryInfo::from_words(&words);
        assert!(info.is_valid());
        assert_eq!(info.signature, BinaryInfo::SIGNATURE_REF);
        assert_eq!(info.version, 3);
        assert!(info.pssl_or_cg);
        assert!(!info.cached);
        assert_eq!(info.type_, 5);
        assert_eq!(info.source_type, 1);
        assert_eq!(info.length, 0x40);
        assert_eq!(info.chunk_usage_base_offset_in_dw, 4);
        assert_eq!(info.num_input_usage_slots, 9);
        assert!(info.is_srt);
        assert!(!info.is_srt_used_info_valid);
        assert!(info.is_extended_usage_info);
        assert_eq!(info.reserved2, 0);
        assert_eq!(info.reserved3, 0x77);
        assert_eq!(info.shader_hash, 0x1234_5678_9ABC_DEF0);
        assert_eq!(info.crc32, 0xDEAD_BEEF);

        // A word the signature does not start with is not a header.
        let mut words = header_words(1, 2, 3);
        words[0] ^= 1;
        assert!(!BinaryInfo::from_words(&words).is_valid());
    }

    #[test]
    fn the_search_finds_the_header_after_the_prologue_first() {
        // s_mov_b32 vcc_hi, #imm followed by a word count, then the header.
        let header = header_words(8, 0xABCD, 7);
        let mut code = vec![0xBEEB_03FFu32, 1, 0, 0];
        code.extend_from_slice(&header);
        let info = search_binary_info(&code);
        assert_eq!(info.shader_hash, 0xABCD);

        // Without the prologue the scan finds it wherever it lies.
        let mut code = vec![0u32; 6];
        code.extend_from_slice(&header);
        let info = search_binary_info(&code);
        assert_eq!(info.crc32, 7);
    }

    #[test]
    #[should_panic(expected = "Shader binary info not found.")]
    fn a_code_without_a_header_is_refused() {
        let _ = search_binary_info(&[1u32, 2, 3, 4]);
    }

    #[test]
    fn the_program_words_read_their_settings() {
        // num_vgprs 3, num_sgprs 5, priority 1, modess and the bits past the first word.
        let settings = ShaderProgramSettings::from_raw(
            3 | (5 << 6)
                | (1 << 10)
                | (3 << 12)
                | (2 << 14)
                | (1 << 16)
                | (1 << 18)
                | (2 << 24)
                | (1 << 32)
                | (9 << 33)
                | (1 << 39),
        );
        assert_eq!(settings.num_vgprs(), 3);
        assert_eq!(settings.num_sgprs(), 5);
        assert_eq!(settings.priority(), 1);
        assert_eq!(settings.fp_round_mode32(), FpRoundMode::ToZero);
        assert_eq!(settings.fp_round_mode64(), FpRoundMode::MinInf);
        assert_eq!(settings.fp_denorm_mode32(), FpDenormMode::InAllowOutFlush);
        assert_eq!(settings.fp_denorm_mode64(), FpDenormMode::InAllowOutFlush);
        assert_eq!(settings.vgpr_comp_cnt(), 2);
        assert!(settings.scratch_en());
        assert_eq!(settings.num_user_regs(), 9);
        assert!(settings.oc_lds_en());

        let program = ShaderProgram {
            address: 0x1234,
            settings,
            user_data: [0; NUM_USER_DATA],
        };
        assert_eq!(program.num_vgprs(), 16, "(3 + 1) * 4");
        assert_eq!(program.address(), 0x12_3400);
    }

    #[test]
    fn the_compute_program_words_read_their_settings() {
        let settings = ComputeProgramSettings::from_raw(
            2 | (4 << 6) | (1 << 12) | (1 << 32) | (3 << 33) | (0b101 << 39) | (0x20 << 47),
        );
        assert_eq!(settings.num_vgprs(), 2);
        assert_eq!(settings.num_sgprs(), 4);
        assert_eq!(settings.fp_round_mode32(), FpRoundMode::PlusInf);
        assert_eq!(settings.fp_round_mode64(), FpRoundMode::NearestEven);
        assert!(settings.scratch_en());
        assert_eq!(settings.num_user_regs(), 3);
        assert_eq!(settings.tgid_enable(), 0b101);
        assert_eq!(settings.lds_dwords(), 0x20);

        let program = ComputeProgram {
            dim_x: 4,
            dim_y: 3,
            dim_z: 2,
            settings,
            address: 0x10,
            ..ComputeProgram::default()
        };
        assert_eq!(program.num_workgroups(), 24);
        assert_eq!(program.shared_mem_size(), 0x20 * 128 * 4);
        assert!(program.is_tgid_enabled(0));
        assert!(!program.is_tgid_enabled(1));
        assert!(program.is_tgid_enabled(2));
        assert_eq!(program.address(), 0x1000);
    }

    #[test]
    fn the_export_and_input_words_read_their_fields() {
        assert_eq!(VsOutputConfig::from_raw(0).num_exports(), 1);
        let config = VsOutputConfig::from_raw(3 << 1 | 1 << 6);
        assert_eq!(config.num_exports(), 4);
        assert!(config.half_pack());

        let control = PsInputControl::from_raw(7 | 1 << 5 | 2 << 7 | 1 << 9);
        assert_eq!(control.input_offset(), 7);
        assert!(control.use_default());
        assert_eq!(control.default_value(), 2);
        assert!(control.flat_shade());

        let input = PsInput::from_raw(1 | 1 << 4 | 1 << 12 | 1 << 15);
        assert!(input.persp_sample_ena());
        assert!(input.linear_sample_ena());
        assert!(input.front_face_ena());
        assert!(input.pos_fixed_pt_ena());
        assert!(!input.persp_center_ena());
        assert!(!input.pos_w_float_ena());

        assert!(BarycentricControl::from_raw(1 << 24).front_face_all_bits());
        assert!(!BarycentricControl::from_raw(1 << 23).front_face_all_bits());

        let format = ShaderPosFormat::from_raw(1 | 2 << 4 | 3 << 8 | 4 << 12);
        assert_eq!(format.pos0(), ShaderExportComp::OneComp);
        assert_eq!(format.pos1(), ShaderExportComp::TwoComp);
        assert_eq!(format.pos2(), ShaderExportComp::FourCompCompressed);
        assert_eq!(format.pos3(), ShaderExportComp::FourComp);

        let exports = ColorExportFormat {
            raw: 4 | 9 << 4 | 1 << 8,
        };
        assert_eq!(exports.get_format(0), ShaderExportFormat::ABGR_FP16);
        assert_eq!(exports.get_format(1), ShaderExportFormat::ABGR_32);
        assert_eq!(exports.get_format(2), ShaderExportFormat::R_32);
        assert_eq!(exports.get_format(3), ShaderExportFormat::Zero);
    }

    #[test]
    fn the_params_come_from_the_header_length_and_hash() {
        let header = header_words(8, 0xFEED, 1);
        let mut code = vec![0xBEEB_03FFu32, 1, 0, 0];
        code.extend_from_slice(&header);
        code.extend_from_slice(&[0x1111, 0x2222, 0x3333, 0x4444]);
        let params = get_params(&code, [1; NUM_USER_DATA]);
        assert_eq!(params.hash, 0xFEED);
        // The header's length counts the code from its start, the prologue included.
        assert_eq!(params.code.len(), 2, "eight bytes of code");
        assert_eq!(params.code[0], 0xBEEB_03FF);
        assert_eq!(params.user_data[0], 1);
    }
}
