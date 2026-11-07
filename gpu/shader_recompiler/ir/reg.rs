//! Registers and instruction-info words, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/reg.h`.

use super::r#type::Type;
use crate::gpu::amdgpu::pixel_format::{DataFormat, NumberFormat};
use crate::gpu::common::bit_field::BitField;

// The instruction-info words are bit fields over their raw storage, one alias per field.
type TextureIsDepthField = BitField<u32, 0, 1>;
type TextureHasBiasField = BitField<u32, 1, 1>;
type TextureHasLodClampField = BitField<u32, 2, 1>;
type TextureForceLevel0Field = BitField<u32, 3, 1>;
type TextureHasLodField = BitField<u32, 4, 1>;
type TextureHasOffsetField = BitField<u32, 5, 1>;
type TextureGatherCompField = BitField<u32, 6, 2>;
type TextureHasDerivativesField = BitField<u32, 8, 1>;
type TextureIsArrayField = BitField<u32, 9, 1>;
type TextureIsUnnormalizedField = BitField<u32, 10, 1>;
type TextureIsGatherField = BitField<u32, 11, 1>;
type TextureIsR128Field = BitField<u32, 12, 1>;
type TexturePcField = BitField<u32, 16, 16>;
type BufferIndexEnableField = BitField<u64, 0, 1>;
type BufferVoffsetEnableField = BitField<u64, 1, 1>;
type BufferInstOffsetField = BitField<u64, 2, 12>;
type BufferSystemCoherentField = BitField<u64, 14, 1>;
type BufferGloballyCoherentField = BitField<u64, 15, 1>;
type BufferTypedField = BitField<u64, 16, 1>;
type BufferSharpSourceField = BitField<u64, 24, 1>;
type BufferPcField = BitField<u64, 32, 16>;
type BufferFlatbufOffDwField = BitField<u64, 48, 16>;
type InstDataFmtField = BitField<u64, 17, 4>;
type InstNumFmtField = BitField<u64, 21, 3>;

/// `Shader::IR::FloatClassFunc`: the classification bits of a float comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FloatClassFunc(u32);

macro_rules! define_float_classes {
    ($($name:ident = $value:expr),* $(,)?) => {
        impl FloatClassFunc {
            $(
                #[allow(non_upper_case_globals)]
                pub const $name: FloatClassFunc = FloatClassFunc($value);
            )*

            /// Every single-bit class, in bit order.
            pub const ALL: &'static [FloatClassFunc] = &[$(FloatClassFunc::$name),*];
        }
    };
}

define_float_classes! {
    SignalingNan = 1 << 0,
    QuietNan = 1 << 1,
    NegativeInfinity = 1 << 2,
    NegativeNormal = 1 << 3,
    NegativeDenorm = 1 << 4,
    NegativeZero = 1 << 5,
    PositiveZero = 1 << 6,
    PositiveDenorm = 1 << 7,
    PositiveNormal = 1 << 8,
    PositiveInfinity = 1 << 9,
}

impl FloatClassFunc {
    /// The header's named combinations.
    #[allow(non_upper_case_globals)]
    pub const NaN: FloatClassFunc =
        FloatClassFunc(FloatClassFunc::SignalingNan.0 | FloatClassFunc::QuietNan.0);
    #[allow(non_upper_case_globals)]
    pub const Infinity: FloatClassFunc =
        FloatClassFunc(FloatClassFunc::PositiveInfinity.0 | FloatClassFunc::NegativeInfinity.0);
    #[allow(non_upper_case_globals)]
    pub const Negative: FloatClassFunc = FloatClassFunc(
        FloatClassFunc::NegativeInfinity.0
            | FloatClassFunc::NegativeNormal.0
            | FloatClassFunc::NegativeDenorm.0
            | FloatClassFunc::NegativeZero.0,
    );
    #[allow(non_upper_case_globals)]
    pub const Finite: FloatClassFunc = FloatClassFunc(
        FloatClassFunc::NegativeNormal.0
            | FloatClassFunc::NegativeDenorm.0
            | FloatClassFunc::NegativeZero.0
            | FloatClassFunc::PositiveNormal.0
            | FloatClassFunc::PositiveDenorm.0
            | FloatClassFunc::PositiveZero.0,
    );

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// `Shader::IR::TextureInstInfo`: the flags word of an image instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextureInstInfo {
    raw: u32,
}

impl TextureInstInfo {
    /// The `is_depth` field, bits 0..1.
    pub fn is_depth(self) -> u32 {
        TextureIsDepthField::from_raw(self.raw).value()
    }

    pub fn set_is_depth(&mut self, value: u32) {
        let mut field = TextureIsDepthField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `has_bias` field, bits 1..2.
    pub fn has_bias(self) -> u32 {
        TextureHasBiasField::from_raw(self.raw).value()
    }

    pub fn set_has_bias(&mut self, value: u32) {
        let mut field = TextureHasBiasField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `has_lod_clamp` field, bits 2..3.
    pub fn has_lod_clamp(self) -> u32 {
        TextureHasLodClampField::from_raw(self.raw).value()
    }

    pub fn set_has_lod_clamp(&mut self, value: u32) {
        let mut field = TextureHasLodClampField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `force_level0` field, bits 3..4.
    pub fn force_level0(self) -> u32 {
        TextureForceLevel0Field::from_raw(self.raw).value()
    }

    pub fn set_force_level0(&mut self, value: u32) {
        let mut field = TextureForceLevel0Field::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `has_lod` field, bits 4..5.
    pub fn has_lod(self) -> u32 {
        TextureHasLodField::from_raw(self.raw).value()
    }

    pub fn set_has_lod(&mut self, value: u32) {
        let mut field = TextureHasLodField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `has_offset` field, bits 5..6.
    pub fn has_offset(self) -> u32 {
        TextureHasOffsetField::from_raw(self.raw).value()
    }

    pub fn set_has_offset(&mut self, value: u32) {
        let mut field = TextureHasOffsetField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `gather_comp` field, bits 6..8.
    pub fn gather_comp(self) -> u32 {
        TextureGatherCompField::from_raw(self.raw).value()
    }

    pub fn set_gather_comp(&mut self, value: u32) {
        let mut field = TextureGatherCompField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `has_derivatives` field, bits 8..9.
    pub fn has_derivatives(self) -> u32 {
        TextureHasDerivativesField::from_raw(self.raw).value()
    }

    pub fn set_has_derivatives(&mut self, value: u32) {
        let mut field = TextureHasDerivativesField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `is_array` field, bits 9..10.
    pub fn is_array(self) -> u32 {
        TextureIsArrayField::from_raw(self.raw).value()
    }

    pub fn set_is_array(&mut self, value: u32) {
        let mut field = TextureIsArrayField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `is_unnormalized` field, bits 10..11.
    pub fn is_unnormalized(self) -> u32 {
        TextureIsUnnormalizedField::from_raw(self.raw).value()
    }

    pub fn set_is_unnormalized(&mut self, value: u32) {
        let mut field = TextureIsUnnormalizedField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `is_gather` field, bits 11..12.
    pub fn is_gather(self) -> u32 {
        TextureIsGatherField::from_raw(self.raw).value()
    }

    pub fn set_is_gather(&mut self, value: u32) {
        let mut field = TextureIsGatherField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `is_r128` field, bits 12..13.
    pub fn is_r128(self) -> u32 {
        TextureIsR128Field::from_raw(self.raw).value()
    }

    pub fn set_is_r128(&mut self, value: u32) {
        let mut field = TextureIsR128Field::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `pc` field, bits 16..32.
    pub fn pc(self) -> u32 {
        TexturePcField::from_raw(self.raw).value()
    }

    pub fn set_pc(&mut self, value: u32) {
        let mut field = TexturePcField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }
    pub const fn from_raw(raw: u32) -> Self {
        Self { raw }
    }

    pub const fn raw(self) -> u32 {
        self.raw
    }
}

/// `Shader::IR::BufferInstInfo`: the flags word of a buffer instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BufferInstInfo {
    raw: u64,
}

impl BufferInstInfo {
    /// The `index_enable` field, bits 0..1.
    pub fn index_enable(self) -> u64 {
        BufferIndexEnableField::from_raw(self.raw).value()
    }

    pub fn set_index_enable(&mut self, value: u64) {
        let mut field = BufferIndexEnableField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `voffset_enable` field, bits 1..2.
    pub fn voffset_enable(self) -> u64 {
        BufferVoffsetEnableField::from_raw(self.raw).value()
    }

    pub fn set_voffset_enable(&mut self, value: u64) {
        let mut field = BufferVoffsetEnableField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `inst_offset` field, bits 2..14.
    pub fn inst_offset(self) -> u64 {
        BufferInstOffsetField::from_raw(self.raw).value()
    }

    pub fn set_inst_offset(&mut self, value: u64) {
        let mut field = BufferInstOffsetField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `system_coherent` field, bits 14..15.
    pub fn system_coherent(self) -> u64 {
        BufferSystemCoherentField::from_raw(self.raw).value()
    }

    pub fn set_system_coherent(&mut self, value: u64) {
        let mut field = BufferSystemCoherentField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `globally_coherent` field, bits 15..16.
    pub fn globally_coherent(self) -> u64 {
        BufferGloballyCoherentField::from_raw(self.raw).value()
    }

    pub fn set_globally_coherent(&mut self, value: u64) {
        let mut field = BufferGloballyCoherentField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `typed` field, bits 16..17.
    pub fn typed(self) -> u64 {
        BufferTypedField::from_raw(self.raw).value()
    }

    pub fn set_typed(&mut self, value: u64) {
        let mut field = BufferTypedField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `sharp_source` field, bits 24..25.
    pub fn sharp_source(self) -> u64 {
        BufferSharpSourceField::from_raw(self.raw).value()
    }

    pub fn set_sharp_source(&mut self, value: u64) {
        let mut field = BufferSharpSourceField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `pc` field, bits 32..48.
    pub fn pc(self) -> u64 {
        BufferPcField::from_raw(self.raw).value()
    }

    pub fn set_pc(&mut self, value: u64) {
        let mut field = BufferPcField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }

    /// The `flatbuf_off_dw` field, bits 48..64.
    pub fn flatbuf_off_dw(self) -> u64 {
        BufferFlatbufOffDwField::from_raw(self.raw).value()
    }

    pub fn set_flatbuf_off_dw(&mut self, value: u64) {
        let mut field = BufferFlatbufOffDwField::from_raw(self.raw);
        field.assign(value);
        self.raw = field.raw();
    }
    /// The `inst_data_fmt` field as the ported AMD format, `None` when undefined.
    pub fn inst_data_format(self) -> Option<DataFormat> {
        DataFormat::from_raw(InstDataFmtField::from_raw(self.raw).value() as u32)
    }

    /// The `inst_num_fmt` field as the ported AMD format, `None` when undefined.
    pub fn inst_number_format(self) -> Option<NumberFormat> {
        NumberFormat::from_raw(InstNumFmtField::from_raw(self.raw).value() as u32)
    }

    pub fn set_inst_data_format(&mut self, format: DataFormat) {
        let mut field = InstDataFmtField::from_raw(self.raw);
        field.assign(format.raw() as u64);
        self.raw = field.raw();
    }

    pub fn set_inst_number_format(&mut self, format: NumberFormat) {
        let mut field = InstNumFmtField::from_raw(self.raw);
        field.assign(format.raw() as u64);
        self.raw = field.raw();
    }

    pub const fn from_raw(raw: u64) -> Self {
        Self { raw }
    }

    pub const fn raw(self) -> u64 {
        self.raw
    }
}

/// `Shader::IR::ScalarReg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u32)]
pub enum ScalarReg {
    S0 = 0,
    S1 = 1,
    S2 = 2,
    S3 = 3,
    S4 = 4,
    S5 = 5,
    S6 = 6,
    S7 = 7,
    S8 = 8,
    S9 = 9,
    S10 = 10,
    S11 = 11,
    S12 = 12,
    S13 = 13,
    S14 = 14,
    S15 = 15,
    S16 = 16,
    S17 = 17,
    S18 = 18,
    S19 = 19,
    S20 = 20,
    S21 = 21,
    S22 = 22,
    S23 = 23,
    S24 = 24,
    S25 = 25,
    S26 = 26,
    S27 = 27,
    S28 = 28,
    S29 = 29,
    S30 = 30,
    S31 = 31,
    S32 = 32,
    S33 = 33,
    S34 = 34,
    S35 = 35,
    S36 = 36,
    S37 = 37,
    S38 = 38,
    S39 = 39,
    S40 = 40,
    S41 = 41,
    S42 = 42,
    S43 = 43,
    S44 = 44,
    S45 = 45,
    S46 = 46,
    S47 = 47,
    S48 = 48,
    S49 = 49,
    S50 = 50,
    S51 = 51,
    S52 = 52,
    S53 = 53,
    S54 = 54,
    S55 = 55,
    S56 = 56,
    S57 = 57,
    S58 = 58,
    S59 = 59,
    S60 = 60,
    S61 = 61,
    S62 = 62,
    S63 = 63,
    S64 = 64,
    S65 = 65,
    S66 = 66,
    S67 = 67,
    S68 = 68,
    S69 = 69,
    S70 = 70,
    S71 = 71,
    S72 = 72,
    S73 = 73,
    S74 = 74,
    S75 = 75,
    S76 = 76,
    S77 = 77,
    S78 = 78,
    S79 = 79,
    S80 = 80,
    S81 = 81,
    S82 = 82,
    S83 = 83,
    S84 = 84,
    S85 = 85,
    S86 = 86,
    S87 = 87,
    S88 = 88,
    S89 = 89,
    S90 = 90,
    S91 = 91,
    S92 = 92,
    S93 = 93,
    S94 = 94,
    S95 = 95,
    S96 = 96,
    S97 = 97,
    S98 = 98,
    S99 = 99,
    S100 = 100,
    S101 = 101,
    S102 = 102,
    S103 = 103,
    Max = 104,
}

/// `NumScalarRegs`.
pub const NUM_SCALAR_REGS: usize = ScalarReg::Max as usize;

/// `Shader::IR::VectorReg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u32)]
pub enum VectorReg {
    V0 = 0,
    V1 = 1,
    V2 = 2,
    V3 = 3,
    V4 = 4,
    V5 = 5,
    V6 = 6,
    V7 = 7,
    V8 = 8,
    V9 = 9,
    V10 = 10,
    V11 = 11,
    V12 = 12,
    V13 = 13,
    V14 = 14,
    V15 = 15,
    V16 = 16,
    V17 = 17,
    V18 = 18,
    V19 = 19,
    V20 = 20,
    V21 = 21,
    V22 = 22,
    V23 = 23,
    V24 = 24,
    V25 = 25,
    V26 = 26,
    V27 = 27,
    V28 = 28,
    V29 = 29,
    V30 = 30,
    V31 = 31,
    V32 = 32,
    V33 = 33,
    V34 = 34,
    V35 = 35,
    V36 = 36,
    V37 = 37,
    V38 = 38,
    V39 = 39,
    V40 = 40,
    V41 = 41,
    V42 = 42,
    V43 = 43,
    V44 = 44,
    V45 = 45,
    V46 = 46,
    V47 = 47,
    V48 = 48,
    V49 = 49,
    V50 = 50,
    V51 = 51,
    V52 = 52,
    V53 = 53,
    V54 = 54,
    V55 = 55,
    V56 = 56,
    V57 = 57,
    V58 = 58,
    V59 = 59,
    V60 = 60,
    V61 = 61,
    V62 = 62,
    V63 = 63,
    V64 = 64,
    V65 = 65,
    V66 = 66,
    V67 = 67,
    V68 = 68,
    V69 = 69,
    V70 = 70,
    V71 = 71,
    V72 = 72,
    V73 = 73,
    V74 = 74,
    V75 = 75,
    V76 = 76,
    V77 = 77,
    V78 = 78,
    V79 = 79,
    V80 = 80,
    V81 = 81,
    V82 = 82,
    V83 = 83,
    V84 = 84,
    V85 = 85,
    V86 = 86,
    V87 = 87,
    V88 = 88,
    V89 = 89,
    V90 = 90,
    V91 = 91,
    V92 = 92,
    V93 = 93,
    V94 = 94,
    V95 = 95,
    V96 = 96,
    V97 = 97,
    V98 = 98,
    V99 = 99,
    V100 = 100,
    V101 = 101,
    V102 = 102,
    V103 = 103,
    V104 = 104,
    V105 = 105,
    V106 = 106,
    V107 = 107,
    V108 = 108,
    V109 = 109,
    V110 = 110,
    V111 = 111,
    V112 = 112,
    V113 = 113,
    V114 = 114,
    V115 = 115,
    V116 = 116,
    V117 = 117,
    V118 = 118,
    V119 = 119,
    V120 = 120,
    V121 = 121,
    V122 = 122,
    V123 = 123,
    V124 = 124,
    V125 = 125,
    V126 = 126,
    V127 = 127,
    V128 = 128,
    V129 = 129,
    V130 = 130,
    V131 = 131,
    V132 = 132,
    V133 = 133,
    V134 = 134,
    V135 = 135,
    V136 = 136,
    V137 = 137,
    V138 = 138,
    V139 = 139,
    V140 = 140,
    V141 = 141,
    V142 = 142,
    V143 = 143,
    V144 = 144,
    V145 = 145,
    V146 = 146,
    V147 = 147,
    V148 = 148,
    V149 = 149,
    V150 = 150,
    V151 = 151,
    V152 = 152,
    V153 = 153,
    V154 = 154,
    V155 = 155,
    V156 = 156,
    V157 = 157,
    V158 = 158,
    V159 = 159,
    V160 = 160,
    V161 = 161,
    V162 = 162,
    V163 = 163,
    V164 = 164,
    V165 = 165,
    V166 = 166,
    V167 = 167,
    V168 = 168,
    V169 = 169,
    V170 = 170,
    V171 = 171,
    V172 = 172,
    V173 = 173,
    V174 = 174,
    V175 = 175,
    V176 = 176,
    V177 = 177,
    V178 = 178,
    V179 = 179,
    V180 = 180,
    V181 = 181,
    V182 = 182,
    V183 = 183,
    V184 = 184,
    V185 = 185,
    V186 = 186,
    V187 = 187,
    V188 = 188,
    V189 = 189,
    V190 = 190,
    V191 = 191,
    V192 = 192,
    V193 = 193,
    V194 = 194,
    V195 = 195,
    V196 = 196,
    V197 = 197,
    V198 = 198,
    V199 = 199,
    V200 = 200,
    V201 = 201,
    V202 = 202,
    V203 = 203,
    V204 = 204,
    V205 = 205,
    V206 = 206,
    V207 = 207,
    V208 = 208,
    V209 = 209,
    V210 = 210,
    V211 = 211,
    V212 = 212,
    V213 = 213,
    V214 = 214,
    V215 = 215,
    V216 = 216,
    V217 = 217,
    V218 = 218,
    V219 = 219,
    V220 = 220,
    V221 = 221,
    V222 = 222,
    V223 = 223,
    V224 = 224,
    V225 = 225,
    V226 = 226,
    V227 = 227,
    V228 = 228,
    V229 = 229,
    V230 = 230,
    V231 = 231,
    V232 = 232,
    V233 = 233,
    V234 = 234,
    V235 = 235,
    V236 = 236,
    V237 = 237,
    V238 = 238,
    V239 = 239,
    V240 = 240,
    V241 = 241,
    V242 = 242,
    V243 = 243,
    V244 = 244,
    V245 = 245,
    V246 = 246,
    V247 = 247,
    V248 = 248,
    V249 = 249,
    V250 = 250,
    V251 = 251,
    V252 = 252,
    V253 = 253,
    V254 = 254,
    V255 = 255,
    Max = 256,
}

/// `NumVectorRegs`.
pub const NUM_VECTOR_REGS: usize = VectorReg::Max as usize;

/// `Shader::IR::VirtualReg`: a virtual register carries its own type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct VirtualReg {
    /// The first member, the one `RegTag` relies on sharing.
    pub index: u32,
    pub ty: Type,
}

impl VirtualReg {
    /// `VirtualReg(index, type)`, which the C++ asserts is not opaque.
    pub fn new(index: u32, ty: Type) -> Self {
        assert_ne!(ty, Type::Opaque, "a virtual register cannot be opaque");
        Self { index, ty }
    }

    /// `VirtualReg::Key`.
    pub const fn key(self) -> u64 {
        self.index as u64 | (self.ty.bits() as u64) << 32
    }
}

impl PartialOrd for VirtualReg {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VirtualReg {
    /// The C++'s `operator<=>` is on the members; the key orders the same way, index first.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index
            .cmp(&other.index)
            .then_with(|| self.ty.bits().cmp(&other.ty.bits()))
    }
}

/// `RegT`: the register types the arithmetic covers.
pub trait RegType: Copy {
    fn raw(self) -> u32;
    fn from_raw(value: u32) -> Self;
    /// The value of the `Max` sentinel.
    const MAX: u32;
}

macro_rules! impl_reg_type {
    ($t:ty) => {
        impl RegType for $t {
            fn raw(self) -> u32 {
                self as u32
            }

            fn from_raw(value: u32) -> Self {
                assert!(value < Self::MAX, "register out of range");
                unsafe { std::mem::transmute::<u32, $t>(value) }
            }

            const MAX: u32 = <$t>::Max as u32;
        }
    };
}

impl_reg_type!(ScalarReg);
impl_reg_type!(VectorReg);

/// The C++'s `operator+(Reg reg, int num)`, which asserts instead of wrapping.
pub fn reg_add<Reg: RegType>(reg: Reg, num: i32) -> Option<Reg> {
    let result = reg.raw() as i64 + num as i64;
    if result < 0 || result >= Reg::MAX as i64 {
        return None;
    }
    Some(Reg::from_raw(result as u32))
}

/// The C++'s `operator-(Reg reg, int num)`.
pub fn reg_sub<Reg: RegType>(reg: Reg, num: i32) -> Option<Reg> {
    reg_add(reg, -num)
}

/// `RegIndex`.
pub fn reg_index<Reg: RegType>(reg: Reg) -> usize {
    reg.raw() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_register_files_have_the_headers_limits() {
        assert_eq!(NUM_SCALAR_REGS, 104);
        assert_eq!(NUM_VECTOR_REGS, 256);
        assert_eq!(ScalarReg::S0 as u32, 0);
        assert_eq!(ScalarReg::S103 as u32, 103);
        assert_eq!(VectorReg::V255 as u32, 255);
        assert_eq!(reg_index(ScalarReg::S7), 7);
        assert_eq!(reg_index(VectorReg::V200), 200);
        assert_eq!(reg_sub(ScalarReg::S7, 7).map(reg_index), Some(0));
        assert!(reg_sub(ScalarReg::S0, 1).is_none(), "underflow is refused");
        assert!(reg_add(VectorReg::V255, 1).is_none(), "overflow is refused");
        assert_eq!(reg_add(VectorReg::V1, 1), Some(VectorReg::V2));
        assert_eq!(reg_add(ScalarReg::S100, 3).map(reg_index), Some(103));
    }

    #[test]
    fn a_virtual_register_keeps_its_type_in_the_key() {
        let reg = VirtualReg::new(5, Type::F32);
        assert_eq!(reg.index, 5);
        assert_eq!(reg.ty, Type::F32);
        assert_eq!(reg.key(), 5 | (Type::F32.bits() as u64) << 32);
        assert_ne!(VirtualReg::new(5, Type::U32).key(), reg.key());
        assert_eq!(std::mem::size_of::<VirtualReg>(), 8);
        assert_eq!(std::mem::offset_of!(VirtualReg, index), 0);
    }

    #[test]
    fn the_texture_info_fields_sit_where_the_header_puts_them() {
        let mut info = TextureInstInfo::default();
        assert_eq!(info.raw(), 0);
        assert_eq!(info.is_depth(), 0);
        info.set_is_depth(1);
        info.set_gather_comp(0b10);
        info.set_pc(0x1234);
        assert_eq!(info.raw(), 1 | (0b10 << 6) | (0x1234 << 16));
        assert_eq!(info.is_depth(), 1);
        assert_eq!(info.gather_comp(), 0b10);
        assert_eq!(info.pc(), 0x1234);
        assert_eq!(TextureInstInfo::from_raw(info.raw()), info);
        assert_eq!(info.has_bias(), 0, "the neighbours are untouched");
        assert_eq!(info.is_r128(), 0);
        info.set_has_bias(1);
        assert_eq!(info.raw(), 0b11 | (0b10 << 6) | (0x1234 << 16));
        assert_eq!(info.has_derivatives(), 0);
        assert_eq!(info.has_lod_clamp(), 0);
    }

    #[test]
    fn the_buffer_info_carries_the_amd_formats() {
        let mut info = BufferInstInfo::default();
        info.set_index_enable(1);
        info.set_inst_offset(0x123);
        info.set_inst_data_format(DataFormat::Format8_8_8_8);
        info.set_inst_number_format(NumberFormat::Unorm);
        info.set_sharp_source(1);
        info.set_flatbuf_off_dw(0xBEEF);
        assert_eq!(info.index_enable(), 1);
        assert_eq!(info.inst_offset(), 0x123);
        assert_eq!(info.inst_data_format(), Some(DataFormat::Format8_8_8_8));
        assert_eq!(info.inst_number_format(), Some(NumberFormat::Unorm));
        assert_eq!(info.sharp_source(), 1);
        assert_eq!(info.flatbuf_off_dw(), 0xBEEF);
        assert_eq!(info.pc(), 0);
        assert_eq!(BufferInstInfo::from_raw(info.raw()).raw(), info.raw());
        // The field is four bits wide, as the register layout says, so a wider format value
        // keeps only its low bits — the same masking `BitField::FormatValue` does in the C++.
        info.set_inst_data_format(DataFormat::FormatBc7);
        assert_eq!(info.inst_data_format(), Some(DataFormat::Format2_10_10_10));
        // The number-format field is three bits wide.
        info.set_inst_number_format(NumberFormat::Ubscaled);
        assert_eq!(info.inst_number_format(), Some(NumberFormat::Sint));
        assert_eq!(
            info.flatbuf_off_dw(),
            0xBEEF,
            "the upper fields are untouched"
        );
        assert_eq!(info.voffset_enable(), 0);
    }

    #[test]
    fn the_float_classes_compose() {
        assert!(FloatClassFunc::NaN.contains(FloatClassFunc::QuietNan));
        assert!(FloatClassFunc::NaN.contains(FloatClassFunc::SignalingNan));
        assert_eq!(FloatClassFunc::NaN.bits(), 0b11);
        assert!(FloatClassFunc::Infinity.contains(FloatClassFunc::PositiveInfinity));
        assert!(FloatClassFunc::Negative.contains(FloatClassFunc::NegativeZero));
        assert!(!FloatClassFunc::Negative.contains(FloatClassFunc::PositiveZero));
        assert!(FloatClassFunc::Finite.contains(FloatClassFunc::PositiveNormal));
        assert!(!FloatClassFunc::Finite.contains(FloatClassFunc::QuietNan));
        assert!(!FloatClassFunc::PositiveInfinity.is_empty());
        assert!(FloatClassFunc::from_bits(0).is_empty());
        assert_eq!(FloatClassFunc::ALL.len(), 10);
        assert_eq!(
            FloatClassFunc::QuietNan
                .union(FloatClassFunc::PositiveZero)
                .bits(),
            1 << 1 | 1 << 6
        );
        assert_eq!(
            FloatClassFunc::NaN
                .intersection(FloatClassFunc::QuietNan)
                .bits(),
            1 << 1
        );
    }
}
