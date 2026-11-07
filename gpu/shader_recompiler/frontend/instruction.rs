//! The decoded GCN instruction, ported from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/instruction.h` and `instruction.cpp`.
//!
//! The header's bit-field structs describe how a raw instruction word lays out. The port keeps
//! every field at its position and width; each struct carries the raw word and exposes the
//! fields the way the C++ reads them, which is how the other ports model their unions too.

use super::opcodes::{InstCategory, InstClass, InstEncoding, Opcode, OperandField, ScalarType};
use crate::gpu::common::bit_field::BitField;

/// `GcnMaxSrcCount`.
pub const GCN_MAX_SRC_COUNT: usize = 4;
/// `GcnMaxDstCount`.
pub const GCN_MAX_DST_COUNT: usize = 2;

/// `Shader::Gcn::OperandFieldRange`: the operand code ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OperandFieldRange {
    ScalarGPRMin = 0,
    ScalarGPRMax = 103,
    SignedConstIntPosMin = 129,
    SignedConstIntPosMax = 192,
    SignedConstIntNegMin = 193,
    SignedConstIntNegMax = 208,
    ConstFloatMin = 240,
    VectorGPRMin = 256,
    VectorGPRMax = 511,
}

/// `Shader::Gcn::InputModifiers`: what is applied after an operand register is loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InputModifiers {
    pub neg: bool,
    pub neg_hi: bool,
    pub abs: bool,
    pub sext: bool,
}

/// `Shader::Gcn::OutputModifiers`: what is applied before an operand register is stored.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OutputModifiers {
    pub clamp: bool,
    pub multiplier: f32,
}

/// `Shader::Gcn::OperandSelection`: the packed-16-bit selectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OperandSelection {
    pub op_sel: bool,
    pub op_sel_hi: bool,
}

/// `Shader::Gcn::SdwaSelector`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SdwaSelector {
    Byte0 = 0,
    Byte1 = 1,
    Byte2 = 2,
    Byte3 = 3,
    Word0 = 4,
    Word1 = 5,
    Dword = 6,
    Invalid = 7,
}

impl SdwaSelector {
    /// A selector read from a three-bit field, which cannot be out of range.
    pub const fn from_raw(value: u32) -> Self {
        match value & 7 {
            0 => SdwaSelector::Byte0,
            1 => SdwaSelector::Byte1,
            2 => SdwaSelector::Byte2,
            3 => SdwaSelector::Byte3,
            4 => SdwaSelector::Word0,
            5 => SdwaSelector::Word1,
            _ => SdwaSelector::Dword,
        }
    }
}

/// `Shader::Gcn::SdwaDstUnused`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SdwaDstUnused {
    Pad = 0,
    Sext = 1,
    Preserve = 2,
    Invalid = 3,
}

impl SdwaDstUnused {
    /// A destination modifier read from a two-bit field.
    pub const fn from_raw(value: u32) -> Self {
        match value & 3 {
            0 => SdwaDstUnused::Pad,
            1 => SdwaDstUnused::Sext,
            _ => SdwaDstUnused::Preserve,
        }
    }
}

/// `Shader::Gcn::DppCtrl`: the named control values, all other controls being permutations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum DppCtrl {
    DppQuadPerm = 0,
    DppRowShl = 0x101,
    DppRowShr = 0x111,
    DppRowRor = 0x121,
    DppRowMirror = 0x140,
    DppRowHalfMirror = 0x141,
}

/// `Shader::Gcn::DppOperation`: a decoded `dpp_ctrl` with its masks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DppOperation {
    pub op: DppCtrl,
    pub value: u16,
    pub bc: bool,
    pub row_mask: u8,
    pub bank_mask: u8,
}

/// `Shader::Gcn::InstOperand`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstOperand {
    pub field: u32,
    pub type_: ScalarType,
    pub input_modifier: InputModifiers,
    pub output_modifier: OutputModifiers,
    /// Only valid for packed 16-bit operations.
    pub op_sel: OperandSelection,
    pub sdwa_dst: SdwaDstUnused,
    pub sdwa_sel: SdwaSelector,
    pub dpp: Option<DppOperation>,
    pub code: u32,
}

impl Default for InstOperand {
    fn default() -> Self {
        InstOperand {
            field: OperandField::Undefined.0,
            type_: ScalarType::Undefined,
            input_modifier: InputModifiers::default(),
            output_modifier: OutputModifiers::default(),
            op_sel: OperandSelection::default(),
            sdwa_dst: SdwaDstUnused::Invalid,
            sdwa_sel: SdwaSelector::Invalid,
            dpp: None,
            code: 0xFFFF_FFFF,
        }
    }
}

/// `Shader::Gcn::Operand`. The C++ stores the input and output modifiers in one union member;
/// the port keeps both, since they are read according to which side of the instruction the
/// operand is on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Operand {
    pub field: u32,
    pub type_: ScalarType,
    pub input_modifier: InputModifiers,
    pub output_modifier: OutputModifiers,
    pub code: u32,
}

impl Default for Operand {
    fn default() -> Self {
        Operand {
            field: OperandField::Undefined.0,
            type_: ScalarType::Undefined,
            input_modifier: InputModifiers::default(),
            output_modifier: OutputModifiers::default(),
            code: 0xFFFF_FFFF,
        }
    }
}

/// `Shader::Gcn::InstSOPK`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InstSOPK {
    pub simm: u16,
}

/// `Shader::Gcn::InstSOPP`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InstSOPP {
    pub simm: u16,
}

/// `Shader::Gcn::InstVOP3`: the three source operands of a `VOP3` instruction.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InstVOP3 {
    pub vdst: Operand,
    pub src0: Operand,
    pub src1: Operand,
    pub src2: Operand,
}

/// `Shader::Gcn::SMRD`: an `SMRD` instruction's fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SMRD {
    pub offset: u8,
    pub imm: bool,
    pub sbase: u8,
}

/// `Shader::Gcn::InstControlSOPK`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InstControlSOPK {
    pub simm: i16,
}

/// `Shader::Gcn::InstControlSOPP`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InstControlSOPP {
    pub simm: i16,
}

/// `Shader::Gcn::InstControlVOP3`: eight bits skipped, then `abs`, `clmp`, `op_sel`, 43 bits
/// skipped, `omod` and `neg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct InstControlVOP3(u64);

/// Bits 8..11.
type Vop3AbsField = BitField<u64, 8, 3>;
/// Bit 11.
type Vop3ClmpField = BitField<u64, 11, 1>;
/// Bits 12..16.
type Vop3OpSelField = BitField<u64, 12, 4>;
/// Bits 59..61.
type Vop3OmodField = BitField<u64, 59, 2>;
/// Bits 61..64.
type Vop3NegField = BitField<u64, 61, 3>;

#[allow(
    clippy::should_implement_trait,
    reason = "the field is named `neg` in the C++"
)]
impl InstControlVOP3 {
    /// The structure over a raw control word.
    pub const fn from_raw(raw: u64) -> Self {
        InstControlVOP3(raw)
    }

    /// The raw control word.
    pub const fn raw(self) -> u64 {
        self.0
    }

    pub fn abs(self) -> u64 {
        Vop3AbsField::from_raw(self.0).value()
    }

    pub fn clmp(self) -> bool {
        Vop3ClmpField::from_raw(self.0).value() != 0
    }

    pub fn op_sel(self) -> u64 {
        Vop3OpSelField::from_raw(self.0).value()
    }

    pub fn omod(self) -> u64 {
        Vop3OmodField::from_raw(self.0).value()
    }

    pub fn neg(self) -> u64 {
        Vop3NegField::from_raw(self.0).value()
    }
}

/// `Shader::Gcn::InstControlVOP3P`, with the selectors split across the word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct InstControlVOP3P(u64);

/// Bits 8..11.
type Vop3pNegHiField = BitField<u64, 8, 3>;
/// Bits 11..14.
type Vop3pOpSelField = BitField<u64, 11, 3>;
/// Bit 14.
type Vop3pOpSelHi2Field = BitField<u64, 14, 1>;
/// Bit 15.
type Vop3pClampField = BitField<u64, 15, 1>;
/// Bits 59..61.
type Vop3pOpSelHi01Field = BitField<u64, 59, 2>;
/// Bits 61..64.
type Vop3pNegField = BitField<u64, 61, 3>;

#[allow(
    clippy::should_implement_trait,
    reason = "the field is named `neg` in the C++"
)]
impl InstControlVOP3P {
    /// The structure over a raw control word.
    pub const fn from_raw(raw: u64) -> Self {
        InstControlVOP3P(raw)
    }

    /// The raw control word.
    pub const fn raw(self) -> u64 {
        self.0
    }

    pub fn neg_hi(self) -> u64 {
        Vop3pNegHiField::from_raw(self.0).value()
    }

    pub fn op_sel(self) -> u64 {
        Vop3pOpSelField::from_raw(self.0).value()
    }

    pub fn op_sel_hi_2(self) -> u64 {
        Vop3pOpSelHi2Field::from_raw(self.0).value()
    }

    pub fn clamp(self) -> u64 {
        Vop3pClampField::from_raw(self.0).value()
    }

    pub fn op_sel_hi_01(self) -> u64 {
        Vop3pOpSelHi01Field::from_raw(self.0).value()
    }

    pub fn neg(self) -> u64 {
        Vop3pNegField::from_raw(self.0).value()
    }

    /// `get_op_sel_hi`: the selector of one of the three operands. Only a packed instruction
    /// has three, and the C++ refuses anything else.
    pub fn get_op_sel_hi(self, index: i32) -> Option<bool> {
        match index {
            0 => Some(self.op_sel_hi_01() & 1 == 1),
            1 => Some((self.op_sel_hi_01() >> 1) & 1 == 1),
            2 => Some(self.op_sel_hi_2() & 1 == 1),
            _ => None,
        }
    }
}

/// `Shader::Gcn::InstControlSMRD`: offset at bit 0, `imm` at bit 8, `count` at bit 9.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct InstControlSMRD(u32);

type SmrdOffsetField = BitField<u32, 0, 8>;
type SmrdImmField = BitField<u32, 8, 1>;
type SmrdCountField = BitField<u32, 9, 5>;

impl InstControlSMRD {
    pub const fn from_raw(raw: u32) -> Self {
        InstControlSMRD(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn offset(self) -> u32 {
        SmrdOffsetField::from_raw(self.0).value()
    }

    pub fn imm(self) -> bool {
        SmrdImmField::from_raw(self.0).value() != 0
    }

    pub fn count(self) -> u32 {
        SmrdCountField::from_raw(self.0).value()
    }

    /// `control.smrd.count = count`, which the decoder sets from the opcode.
    pub fn set_count(&mut self, count: u32) {
        let mut field = SmrdCountField::from_raw(self.0);
        field.assign(count);
        self.0 = field.raw();
    }
}

/// `Shader::Gcn::InstControlMUBUF`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct InstControlMUBUF(u64);

type MubufOffsetField = BitField<u64, 0, 12>;
type MubufOffenField = BitField<u64, 12, 1>;
type MubufIdxenField = BitField<u64, 13, 1>;
type MubufGlcField = BitField<u64, 14, 1>;
type MubufAddr64Field = BitField<u64, 15, 1>;
type MubufLdsField = BitField<u64, 16, 1>;
type MubufSlcField = BitField<u64, 54, 1>;
type MubufTfeField = BitField<u64, 55, 1>;
type MubufCountField = BitField<u64, 56, 3>;
type MubufSizeField = BitField<u64, 59, 5>;

impl InstControlMUBUF {
    pub const fn from_raw(raw: u64) -> Self {
        InstControlMUBUF(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub fn offset(self) -> u64 {
        MubufOffsetField::from_raw(self.0).value()
    }

    pub fn offen(self) -> bool {
        MubufOffenField::from_raw(self.0).value() != 0
    }

    pub fn idxen(self) -> bool {
        MubufIdxenField::from_raw(self.0).value() != 0
    }

    pub fn glc(self) -> bool {
        MubufGlcField::from_raw(self.0).value() != 0
    }

    pub fn addr64(self) -> bool {
        MubufAddr64Field::from_raw(self.0).value() != 0
    }

    pub fn lds(self) -> bool {
        MubufLdsField::from_raw(self.0).value() != 0
    }

    pub fn slc(self) -> bool {
        MubufSlcField::from_raw(self.0).value() != 0
    }

    pub fn tfe(self) -> bool {
        MubufTfeField::from_raw(self.0).value() != 0
    }

    pub fn count(self) -> u64 {
        MubufCountField::from_raw(self.0).value()
    }

    pub fn size(self) -> u64 {
        MubufSizeField::from_raw(self.0).value()
    }

    /// The decoder fills in the transfer count from the opcode.
    pub fn set_count(&mut self, count: u64) {
        let mut field = MubufCountField::from_raw(self.0);
        field.assign(count);
        self.0 = field.raw();
    }

    /// And the transfer size, which follows from the count.
    pub fn set_size(&mut self, size: u64) {
        let mut field = MubufSizeField::from_raw(self.0);
        field.assign(size);
        self.0 = field.raw();
    }
}

/// `Shader::Gcn::InstControlMTBUF`: the buffer fields plus the data and number formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct InstControlMTBUF(u64);

type MtbufOffsetField = BitField<u64, 0, 12>;
type MtbufOffenField = BitField<u64, 12, 1>;
type MtbufIdxenField = BitField<u64, 13, 1>;
type MtbufGlcField = BitField<u64, 14, 1>;
type MtbufAddr64Field = BitField<u64, 15, 1>;
type MtbufDfmtField = BitField<u64, 19, 4>;
type MtbufNfmtField = BitField<u64, 23, 3>;
type MtbufSlcField = BitField<u64, 54, 1>;
type MtbufTfeField = BitField<u64, 55, 1>;
type MtbufCountField = BitField<u64, 56, 3>;
type MtbufSizeField = BitField<u64, 59, 5>;

impl InstControlMTBUF {
    pub const fn from_raw(raw: u64) -> Self {
        InstControlMTBUF(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub fn offset(self) -> u64 {
        MtbufOffsetField::from_raw(self.0).value()
    }

    pub fn offen(self) -> bool {
        MtbufOffenField::from_raw(self.0).value() != 0
    }

    pub fn idxen(self) -> bool {
        MtbufIdxenField::from_raw(self.0).value() != 0
    }

    pub fn glc(self) -> bool {
        MtbufGlcField::from_raw(self.0).value() != 0
    }

    pub fn addr64(self) -> bool {
        MtbufAddr64Field::from_raw(self.0).value() != 0
    }

    pub fn dfmt(self) -> u64 {
        MtbufDfmtField::from_raw(self.0).value()
    }

    pub fn nfmt(self) -> u64 {
        MtbufNfmtField::from_raw(self.0).value()
    }

    pub fn slc(self) -> bool {
        MtbufSlcField::from_raw(self.0).value() != 0
    }

    pub fn tfe(self) -> bool {
        MtbufTfeField::from_raw(self.0).value() != 0
    }

    pub fn count(self) -> u64 {
        MtbufCountField::from_raw(self.0).value()
    }

    pub fn size(self) -> u64 {
        MtbufSizeField::from_raw(self.0).value()
    }

    /// The decoder fills in the transfer count from the opcode.
    pub fn set_count(&mut self, count: u64) {
        let mut field = MtbufCountField::from_raw(self.0);
        field.assign(count);
        self.0 = field.raw();
    }
}

/// `Shader::Gcn::InstControlMIMG`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct InstControlMIMG(u64);

type MimgDmaskField = BitField<u64, 8, 4>;
type MimgUnrmField = BitField<u64, 12, 1>;
type MimgGlcField = BitField<u64, 13, 1>;
type MimgDaField = BitField<u64, 14, 1>;
type MimgR128Field = BitField<u64, 15, 1>;
type MimgTfeField = BitField<u64, 16, 1>;
type MimgLweField = BitField<u64, 17, 1>;
type MimgSlcField = BitField<u64, 25, 1>;
type MimgModField = BitField<u64, 26, 32>;

impl InstControlMIMG {
    pub const fn from_raw(raw: u64) -> Self {
        InstControlMIMG(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub fn dmask(self) -> u64 {
        MimgDmaskField::from_raw(self.0).value()
    }

    pub fn unrm(self) -> bool {
        MimgUnrmField::from_raw(self.0).value() != 0
    }

    pub fn glc(self) -> bool {
        MimgGlcField::from_raw(self.0).value() != 0
    }

    pub fn da(self) -> bool {
        MimgDaField::from_raw(self.0).value() != 0
    }

    pub fn r128(self) -> bool {
        MimgR128Field::from_raw(self.0).value() != 0
    }

    pub fn tfe(self) -> bool {
        MimgTfeField::from_raw(self.0).value() != 0
    }

    pub fn lwe(self) -> bool {
        MimgLweField::from_raw(self.0).value() != 0
    }

    pub fn slc(self) -> bool {
        MimgSlcField::from_raw(self.0).value() != 0
    }

    /// `mod`: the image modifiers, one bit per `MimgModifier` enumerator.
    pub fn modifiers(self) -> u64 {
        MimgModField::from_raw(self.0).value()
    }

    /// `control.mimg.mod = modifiers`.
    pub fn set_modifiers(&mut self, modifiers: u64) {
        let mut field = MimgModField::from_raw(self.0);
        field.assign(modifiers);
        self.0 = field.raw();
    }
}

/// `Shader::Gcn::InstControlDS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct InstControlDS(u64);

type DsOffset0Field = BitField<u64, 0, 8>;
type DsOffset1Field = BitField<u64, 8, 8>;
type DsGdsField = BitField<u64, 17, 1>;
type DsDualField = BitField<u64, 18, 1>;
type DsSignField = BitField<u64, 19, 1>;
type DsRelativeField = BitField<u64, 20, 1>;
type DsStrideField = BitField<u64, 21, 1>;
type DsSizeField = BitField<u64, 22, 4>;

impl InstControlDS {
    pub const fn from_raw(raw: u64) -> Self {
        InstControlDS(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub fn offset0(self) -> u64 {
        DsOffset0Field::from_raw(self.0).value()
    }

    pub fn offset1(self) -> u64 {
        DsOffset1Field::from_raw(self.0).value()
    }

    pub fn gds(self) -> bool {
        DsGdsField::from_raw(self.0).value() != 0
    }

    pub fn dual(self) -> bool {
        DsDualField::from_raw(self.0).value() != 0
    }

    pub fn sign(self) -> bool {
        DsSignField::from_raw(self.0).value() != 0
    }

    pub fn relative(self) -> bool {
        DsRelativeField::from_raw(self.0).value() != 0
    }

    pub fn stride(self) -> bool {
        DsStrideField::from_raw(self.0).value() != 0
    }

    pub fn size(self) -> u64 {
        DsSizeField::from_raw(self.0).value()
    }

    /// The decoder derives the four flags and the access size from the opcode.
    pub fn set_dual(&mut self, dual: bool) {
        let mut field = DsDualField::from_raw(self.0);
        field.assign(u64::from(dual));
        self.0 = field.raw();
    }

    pub fn set_sign(&mut self, sign: bool) {
        let mut field = DsSignField::from_raw(self.0);
        field.assign(u64::from(sign));
        self.0 = field.raw();
    }

    pub fn set_relative(&mut self, relative: bool) {
        let mut field = DsRelativeField::from_raw(self.0);
        field.assign(u64::from(relative));
        self.0 = field.raw();
    }

    pub fn set_stride(&mut self, stride: bool) {
        let mut field = DsStrideField::from_raw(self.0);
        field.assign(u64::from(stride));
        self.0 = field.raw();
    }

    pub fn set_size(&mut self, size: u64) {
        let mut field = DsSizeField::from_raw(self.0);
        field.assign(size);
        self.0 = field.raw();
    }
}

/// `Shader::Gcn::InstControlVINTRP`: the channel and attribute of an interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct InstControlVINTRP(u32);

type VintrpChanField = BitField<u32, 8, 2>;
type VintrpAttrField = BitField<u32, 10, 6>;

impl InstControlVINTRP {
    pub const fn from_raw(raw: u32) -> Self {
        InstControlVINTRP(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn chan(self) -> u32 {
        VintrpChanField::from_raw(self.0).value()
    }

    pub fn attr(self) -> u32 {
        VintrpAttrField::from_raw(self.0).value()
    }
}

/// `Shader::Gcn::InstControlEXP`: which components an export writes and where it goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct InstControlEXP(u64);

type ExpEnField = BitField<u64, 0, 4>;
type ExpTargetField = BitField<u64, 4, 6>;
type ExpComprField = BitField<u64, 10, 1>;
type ExpDoneField = BitField<u64, 11, 1>;
type ExpVmField = BitField<u64, 12, 1>;
type ExpReservedField = BitField<u64, 13, 51>;

impl InstControlEXP {
    pub const fn from_raw(raw: u64) -> Self {
        InstControlEXP(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub fn en(self) -> u64 {
        ExpEnField::from_raw(self.0).value()
    }

    pub fn target(self) -> u64 {
        ExpTargetField::from_raw(self.0).value()
    }

    pub fn compr(self) -> bool {
        ExpComprField::from_raw(self.0).value() != 0
    }

    pub fn done(self) -> bool {
        ExpDoneField::from_raw(self.0).value() != 0
    }

    pub fn vm(self) -> bool {
        ExpVmField::from_raw(self.0).value() != 0
    }

    pub fn reserved(self) -> u64 {
        ExpReservedField::from_raw(self.0).value()
    }
}

/// `Shader::Gcn::InstControl`: one word read as whichever control structure the instruction
/// carries, which is what the C++ union does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct InstControl(u64);

impl InstControl {
    /// The union over a raw control word.
    pub const fn from_raw(raw: u64) -> Self {
        InstControl(raw)
    }

    /// The raw control word.
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Writes a control structure's word back into the union, which is how the port's decoder
    /// applies the field assignments the C++ makes in place.
    pub fn set_raw(&mut self, raw: u64) {
        self.0 = raw;
    }

    /// `sopk`, which lives in the low half.
    pub fn sopk(self) -> InstControlSOPK {
        InstControlSOPK {
            simm: self.0 as u16 as i16,
        }
    }

    /// `sopp`.
    pub fn sopp(self) -> InstControlSOPP {
        InstControlSOPP {
            simm: self.0 as u16 as i16,
        }
    }

    /// `vop3`.
    pub fn vop3(self) -> InstControlVOP3 {
        InstControlVOP3::from_raw(self.0)
    }

    /// `vop3p`.
    pub fn vop3p(self) -> InstControlVOP3P {
        InstControlVOP3P::from_raw(self.0)
    }

    /// `smrd`.
    pub fn smrd(self) -> InstControlSMRD {
        InstControlSMRD::from_raw(self.0 as u32)
    }

    /// `mubuf`.
    pub fn mubuf(self) -> InstControlMUBUF {
        InstControlMUBUF::from_raw(self.0)
    }

    /// `mtbuf`.
    pub fn mtbuf(self) -> InstControlMTBUF {
        InstControlMTBUF::from_raw(self.0)
    }

    /// `mimg`.
    pub fn mimg(self) -> InstControlMIMG {
        InstControlMIMG::from_raw(self.0)
    }

    /// `ds`.
    pub fn ds(self) -> InstControlDS {
        InstControlDS::from_raw(self.0)
    }

    /// `vintrp`.
    pub fn vintrp(self) -> InstControlVINTRP {
        InstControlVINTRP::from_raw(self.0 as u32)
    }

    /// `exp`.
    pub fn exp(self) -> InstControlEXP {
        InstControlEXP::from_raw(self.0)
    }
}

/// `Shader::Gcn::SdwaVop12`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct SdwaVop12(u32);

type SdwaSrc0Field = BitField<u32, 0, 8>;
type SdwaDstSelField = BitField<u32, 8, 3>;
type SdwaDstUField = BitField<u32, 11, 2>;
type SdwaClampField = BitField<u32, 13, 1>;
type SdwaOmodField = BitField<u32, 14, 2>;
type SdwaSrc0SelField = BitField<u32, 16, 3>;
type SdwaSrc0SextField = BitField<u32, 19, 1>;
type SdwaSrc0NegField = BitField<u32, 20, 1>;
type SdwaSrc0AbsField = BitField<u32, 21, 1>;
type SdwaS0Field = BitField<u32, 23, 1>;
type SdwaSrc1SelField = BitField<u32, 24, 3>;
type SdwaSrc1SextField = BitField<u32, 27, 1>;
type SdwaSrc1NegField = BitField<u32, 28, 1>;
type SdwaSrc1AbsField = BitField<u32, 29, 1>;
type SdwaS1Field = BitField<u32, 31, 1>;

impl SdwaVop12 {
    pub const fn from_raw(raw: u32) -> Self {
        SdwaVop12(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn src0(self) -> u32 {
        SdwaSrc0Field::from_raw(self.0).value()
    }

    /// `dst_sel`: the byte selector for the destination.
    pub fn dst_sel(self) -> SdwaSelector {
        SdwaSelector::from_raw(SdwaDstSelField::from_raw(self.0).value())
    }

    pub fn dst_u(self) -> SdwaDstUnused {
        SdwaDstUnused::from_raw(SdwaDstUField::from_raw(self.0).value())
    }

    pub fn clamp(self) -> bool {
        SdwaClampField::from_raw(self.0).value() != 0
    }

    pub fn omod(self) -> u32 {
        SdwaOmodField::from_raw(self.0).value()
    }

    pub fn src0_sel(self) -> SdwaSelector {
        SdwaSelector::from_raw(SdwaSrc0SelField::from_raw(self.0).value())
    }

    pub fn src0_sext(self) -> bool {
        SdwaSrc0SextField::from_raw(self.0).value() != 0
    }

    pub fn src0_neg(self) -> bool {
        SdwaSrc0NegField::from_raw(self.0).value() != 0
    }

    pub fn src0_abs(self) -> bool {
        SdwaSrc0AbsField::from_raw(self.0).value() != 0
    }

    pub fn s0(self) -> bool {
        SdwaS0Field::from_raw(self.0).value() != 0
    }

    pub fn src1_sel(self) -> SdwaSelector {
        SdwaSelector::from_raw(SdwaSrc1SelField::from_raw(self.0).value())
    }

    pub fn src1_sext(self) -> bool {
        SdwaSrc1SextField::from_raw(self.0).value() != 0
    }

    pub fn src1_neg(self) -> bool {
        SdwaSrc1NegField::from_raw(self.0).value() != 0
    }

    pub fn src1_abs(self) -> bool {
        SdwaSrc1AbsField::from_raw(self.0).value() != 0
    }

    pub fn s1(self) -> bool {
        SdwaS1Field::from_raw(self.0).value() != 0
    }
}

/// `Shader::Gcn::SdwaVopc`: the same layout as `SdwaVop12` with a compare destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct SdwaVopc(u32);

type SdwaVopcSdstField = BitField<u32, 8, 7>;
type SdwaVopcSdField = BitField<u32, 15, 1>;

impl SdwaVopc {
    pub const fn from_raw(raw: u32) -> Self {
        SdwaVopc(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn src0(self) -> u32 {
        SdwaSrc0Field::from_raw(self.0).value()
    }

    pub fn sdst(self) -> u32 {
        SdwaVopcSdstField::from_raw(self.0).value()
    }

    pub fn sd(self) -> bool {
        SdwaVopcSdField::from_raw(self.0).value() != 0
    }

    pub fn src0_sel(self) -> SdwaSelector {
        SdwaSelector::from_raw(SdwaSrc0SelField::from_raw(self.0).value())
    }

    pub fn src0_sext(self) -> bool {
        SdwaSrc0SextField::from_raw(self.0).value() != 0
    }

    pub fn src0_neg(self) -> bool {
        SdwaSrc0NegField::from_raw(self.0).value() != 0
    }

    pub fn src0_abs(self) -> bool {
        SdwaSrc0AbsField::from_raw(self.0).value() != 0
    }

    pub fn s0(self) -> bool {
        SdwaS0Field::from_raw(self.0).value() != 0
    }

    pub fn src1_sel(self) -> SdwaSelector {
        SdwaSelector::from_raw(SdwaSrc1SelField::from_raw(self.0).value())
    }

    pub fn src1_sext(self) -> bool {
        SdwaSrc1SextField::from_raw(self.0).value() != 0
    }

    pub fn src1_neg(self) -> bool {
        SdwaSrc1NegField::from_raw(self.0).value() != 0
    }

    pub fn src1_abs(self) -> bool {
        SdwaSrc1AbsField::from_raw(self.0).value() != 0
    }

    pub fn s1(self) -> bool {
        SdwaS1Field::from_raw(self.0).value() != 0
    }
}

/// `Shader::Gcn::Sdwa`: a raw SDWA field read as its value form or its compare form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct Sdwa(u32);

impl Sdwa {
    pub const fn from_raw(raw: u32) -> Self {
        Sdwa(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    /// `vopc`.
    pub fn vopc(self) -> SdwaVopc {
        SdwaVopc::from_raw(self.0)
    }

    /// `vop12`.
    pub fn vop12(self) -> SdwaVop12 {
        SdwaVop12::from_raw(self.0)
    }
}

/// `Shader::Gcn::Dpp`: the data-parallel primitive control word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct Dpp(u32);

type DppSrc0Field = BitField<u32, 0, 8>;
type DppCtrlField = BitField<u32, 8, 9>;
type DppFiField = BitField<u32, 18, 1>;
type DppBcField = BitField<u32, 19, 1>;
type DppSrc0NegField = BitField<u32, 20, 1>;
type DppSrc0AbsField = BitField<u32, 21, 1>;
type DppSrc1NegField = BitField<u32, 22, 1>;
type DppSrc1AbsField = BitField<u32, 23, 1>;
type DppBankMaskField = BitField<u32, 24, 4>;
type DppRowMaskField = BitField<u32, 28, 4>;

impl Dpp {
    pub const fn from_raw(raw: u32) -> Self {
        Dpp(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn src0(self) -> u32 {
        DppSrc0Field::from_raw(self.0).value()
    }

    pub fn dpp_ctrl(self) -> u32 {
        DppCtrlField::from_raw(self.0).value()
    }

    pub fn fi(self) -> bool {
        DppFiField::from_raw(self.0).value() != 0
    }

    pub fn bc(self) -> bool {
        DppBcField::from_raw(self.0).value() != 0
    }

    pub fn src0_neg(self) -> bool {
        DppSrc0NegField::from_raw(self.0).value() != 0
    }

    pub fn src0_abs(self) -> bool {
        DppSrc0AbsField::from_raw(self.0).value() != 0
    }

    pub fn src1_neg(self) -> bool {
        DppSrc1NegField::from_raw(self.0).value() != 0
    }

    pub fn src1_abs(self) -> bool {
        DppSrc1AbsField::from_raw(self.0).value() != 0
    }

    pub fn bank_mask(self) -> u8 {
        DppBankMaskField::from_raw(self.0).value() as u8
    }

    pub fn row_mask(self) -> u8 {
        DppRowMaskField::from_raw(self.0).value() as u8
    }

    /// `GetOperation`: the named controls and the four ranges, which the C++ refuses to
    /// decode when the control fits none of them.
    pub fn operation(self) -> Option<DppOperation> {
        let control = self.dpp_ctrl();
        let (op, value) = match control {
            c if c < 0x100 => (DppCtrl::DppQuadPerm, (c & 0xFF) as u16),
            c if (0x101..0x110).contains(&c) => (DppCtrl::DppRowShl, (c & 0xF) as u16),
            c if (0x111..0x120).contains(&c) => (DppCtrl::DppRowShr, (c & 0xF) as u16),
            c if (0x121..0x130).contains(&c) => (DppCtrl::DppRowRor, (c & 0xF) as u16),
            c if c == DppCtrl::DppRowMirror as u32 => (DppCtrl::DppRowMirror, 0),
            c if c == DppCtrl::DppRowHalfMirror as u32 => (DppCtrl::DppRowHalfMirror, 0),
            _ => return None,
        };
        Some(DppOperation {
            op,
            value,
            bc: self.bc(),
            row_mask: self.row_mask(),
            bank_mask: self.bank_mask(),
        })
    }
}

/// `Shader::Gcn::GcnInst`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GcnInst {
    pub opcode: Opcode,
    pub encoding: InstEncoding,
    pub inst_class: InstClass,
    pub category: InstCategory,
    pub control: InstControl,
    pub length: u32,
    pub src_count: u32,
    pub dst_count: u32,
    pub src: [InstOperand; GCN_MAX_SRC_COUNT],
    pub dst: [InstOperand; GCN_MAX_DST_COUNT],
}

impl Default for GcnInst {
    fn default() -> Self {
        GcnInst {
            opcode: Opcode::S_ADD_U32,
            encoding: InstEncoding::ILLEGAL,
            inst_class: InstClass::Undefined,
            category: InstCategory::Undefined,
            control: InstControl::default(),
            length: 0,
            src_count: 0,
            dst_count: 0,
            src: [InstOperand::default(); GCN_MAX_SRC_COUNT],
            dst: [InstOperand::default(); GCN_MAX_DST_COUNT],
        }
    }
}

impl GcnInst {
    /// `GcnInst::BranchTarget`: the branch's target pc, from the signed 16-bit offset.
    pub fn branch_target(&self, pc: u32) -> u32 {
        let simm = self.control.sopp().simm as i32 * 4;
        (pc as i32 + simm + 4) as u32
    }

    /// `GcnInst::IsTerminateInstruction`.
    pub fn is_terminate_instruction(&self) -> bool {
        self.is_unconditional_branch()
            || self.is_conditional_branch()
            || self.is_fork()
            || self.opcode == Opcode::S_ENDPGM
    }

    /// `GcnInst::IsUnconditionalBranch`.
    pub fn is_unconditional_branch(&self) -> bool {
        self.opcode == Opcode::S_BRANCH || self.opcode == Opcode::S_SETPC_B64
    }

    /// `GcnInst::IsFork`.
    pub fn is_fork(&self) -> bool {
        self.opcode == Opcode::S_CBRANCH_I_FORK
            || self.opcode == Opcode::S_CBRANCH_G_FORK
            || self.opcode == Opcode::S_CBRANCH_JOIN
    }

    /// `GcnInst::IsConditionalBranch`. The four debug branches are unimplemented in the C++,
    /// which reports them and then answers yes; the port answers yes and leaves the report to
    /// the caller through `is_unimplemented_conditional_branch`.
    pub fn is_conditional_branch(&self) -> bool {
        matches!(
            self.opcode,
            Opcode::S_CBRANCH_SCC0
                | Opcode::S_CBRANCH_SCC1
                | Opcode::S_CBRANCH_VCCZ
                | Opcode::S_CBRANCH_VCCNZ
                | Opcode::S_CBRANCH_EXECZ
                | Opcode::S_CBRANCH_EXECNZ
                | Opcode::S_CBRANCH_CDBGSYS
                | Opcode::S_CBRANCH_CDBGUSER
                | Opcode::S_CBRANCH_CDBGSYS_OR_USER
                | Opcode::S_CBRANCH_CDBGSYS_AND_USER
        )
    }

    /// The four conditional branches the C++ marks `UNIMPLEMENTED`, for the caller to report.
    pub fn is_unimplemented_conditional_branch(&self) -> bool {
        matches!(
            self.opcode,
            Opcode::S_CBRANCH_CDBGSYS
                | Opcode::S_CBRANCH_CDBGUSER
                | Opcode::S_CBRANCH_CDBGSYS_OR_USER
                | Opcode::S_CBRANCH_CDBGSYS_AND_USER
        )
    }

    /// `GcnInst::IsCmpx`: the compare instructions that write `exec` as well as the result.
    pub fn is_cmpx(&self) -> bool {
        let opcode = self.opcode.0;
        (Opcode::V_CMPX_F_F32.0..=Opcode::V_CMPX_T_F32.0).contains(&opcode)
            || (Opcode::V_CMPX_F_F64.0..=Opcode::V_CMPX_T_F64.0).contains(&opcode)
            || (Opcode::V_CMPSX_F_F32.0..=Opcode::V_CMPSX_T_F32.0).contains(&opcode)
            || (Opcode::V_CMPSX_F_F64.0..=Opcode::V_CMPSX_T_F64.0).contains(&opcode)
            || (Opcode::V_CMPX_F_I32.0..=Opcode::V_CMPX_CLASS_F32.0).contains(&opcode)
            || (Opcode::V_CMPX_F_I64.0..=Opcode::V_CMPX_CLASS_F64.0).contains(&opcode)
            || (Opcode::V_CMPX_F_U32.0..=Opcode::V_CMPX_T_U32.0).contains(&opcode)
            || (Opcode::V_CMPX_F_U64.0..=Opcode::V_CMPX_T_U64.0).contains(&opcode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::frontend::opcodes::*;

    #[test]
    fn the_control_structs_keep_their_field_positions() {
        let control = InstControlVOP3::from_raw(0xFFFF_FFFF_FFFF_FFFF);
        assert_eq!(control.abs(), 7);
        assert!(control.clmp());
        assert_eq!(control.op_sel(), 15);
        assert_eq!(control.omod(), 3);
        assert_eq!(control.neg(), 7);
        // The abs field sits at bit 8 and the neg field at bit 61.
        let control = InstControlVOP3::from_raw(1 << 8 | 1 << 61);
        assert_eq!(control.abs(), 1);
        assert!(!control.clmp());
        assert_eq!(control.op_sel(), 0);
        assert_eq!(control.omod(), 0);
        assert_eq!(control.neg(), 1);

        let vop3p = InstControlVOP3P::from_raw(0);
        assert_eq!(vop3p.get_op_sel_hi(0), Some(false));
        assert_eq!(vop3p.get_op_sel_hi(3), None, "only three operands have one");
        let vop3p = InstControlVOP3P::from_raw(1 << 59 | 1 << 60 | 1 << 14 | 1 << 15);
        assert_eq!(vop3p.get_op_sel_hi(0), Some(true));
        assert_eq!(vop3p.get_op_sel_hi(1), Some(true));
        assert_eq!(vop3p.get_op_sel_hi(2), Some(true));
        assert_eq!(vop3p.op_sel_hi_01(), 3);
        assert_eq!(vop3p.op_sel_hi_2(), 1);
        assert_eq!(vop3p.clamp(), 1);
    }

    #[test]
    fn the_buffer_and_image_fields_keep_their_positions() {
        let mubuf = InstControlMUBUF::from_raw(0xFFF | 1 << 12 | 1 << 13 | 1 << 14 | 1 << 15);
        assert_eq!(mubuf.offset(), 0xFFF);
        assert!(mubuf.offen());
        assert!(mubuf.idxen());
        assert!(mubuf.glc());
        assert!(mubuf.addr64());
        assert!(!mubuf.lds(), "the lds bit is above the ones set");
        assert_eq!(mubuf.size(), 0);
        assert_eq!(
            InstControlMUBUF::from_raw(0x1F << 59 | 7 << 56).size(),
            0x1F
        );
        assert_eq!(InstControlMUBUF::from_raw(7 << 56).count(), 7);
        let mtbuf = InstControlMTBUF::from_raw(0o7 << 19 | 0o5 << 23);
        assert_eq!(mtbuf.dfmt(), 7);
        assert_eq!(mtbuf.nfmt(), 5);
        let mimg = InstControlMIMG::from_raw(0xF << 8 | 1 << 12 | 3 << 26);
        assert_eq!(mimg.dmask(), 0xF);
        assert!(mimg.unrm());
        assert_eq!(mimg.modifiers(), 3);
        let ds = InstControlDS::from_raw(0xAB | 0xCD << 8 | 1 << 18 | 0o17 << 22);
        assert_eq!(ds.offset0(), 0xAB);
        assert_eq!(ds.offset1(), 0xCD);
        assert!(ds.dual());
        assert_eq!(ds.size(), 0o17);
        let vintrp = InstControlVINTRP::from_raw(3 << 8 | 0x3F << 10);
        assert_eq!(vintrp.chan(), 3);
        assert_eq!(vintrp.attr(), 0x3F);
        let exp = InstControlEXP::from_raw(0o17 | 0x3F << 4 | 1 << 10 | 1 << 11 | 1 << 12);
        assert_eq!(exp.en(), 0o17);
        assert_eq!(exp.target(), 0x3F);
        assert!(exp.compr() && exp.done() && exp.vm());
        assert_eq!(exp.reserved(), 0);
        let smrd = InstControlSMRD::from_raw(0xFF | 1 << 8 | 0o37 << 9);
        assert_eq!(smrd.offset(), 0xFF);
        assert!(smrd.imm());
        assert_eq!(smrd.count(), 0o37);
    }

    #[test]
    fn the_union_reads_the_same_word_every_way() {
        // A negative SOPP immediate is read as a signed half word.
        let control = InstControl::from_raw(0xFFFF_FFFF_FFFF_FFFF);
        assert_eq!(control.sopp().simm, -1);
        assert_eq!(control.sopk().simm, -1);
        assert_eq!(control.ds().offset0(), 0xFF);
        assert_eq!(control.raw(), 0xFFFF_FFFF_FFFF_FFFF);
        // A branch target is the pc plus the offset times four plus the instruction length.
        let mut inst = GcnInst {
            control: InstControl::from_raw(4i16 as u64 as u16 as u64),
            ..GcnInst::default()
        };
        assert_eq!(inst.branch_target(0x100), 0x114);
        inst.control = InstControl::from_raw((-2i16) as u16 as u64);
        assert_eq!(inst.branch_target(0x100), 0xFC);
    }

    #[test]
    fn the_instruction_kinds_come_from_the_opcode() {
        let mut inst = GcnInst::default();
        assert!(!inst.is_terminate_instruction());
        inst.opcode = Opcode::S_BRANCH;
        assert!(inst.is_unconditional_branch());
        assert!(inst.is_terminate_instruction());
        assert!(!inst.is_conditional_branch());
        inst.opcode = Opcode::S_CBRANCH_SCC1;
        assert!(inst.is_conditional_branch());
        assert!(!inst.is_unimplemented_conditional_branch());
        inst.opcode = Opcode::S_CBRANCH_CDBGSYS;
        assert!(inst.is_conditional_branch());
        assert!(inst.is_unimplemented_conditional_branch());
        inst.opcode = Opcode::S_CBRANCH_G_FORK;
        assert!(inst.is_fork());
        assert!(inst.is_terminate_instruction());
        inst.opcode = Opcode::S_SETPC_B64;
        assert!(inst.is_unconditional_branch());
        inst.opcode = Opcode::S_ENDPGM;
        assert!(inst.is_terminate_instruction());
        assert!(!inst.is_fork());
    }

    #[test]
    fn the_compare_width_ranges_mark_the_cmpx_instructions() {
        let mut inst = GcnInst::default();
        for opcode in [
            Opcode::V_CMPX_F_F32,
            Opcode::V_CMPX_T_F32,
            Opcode::V_CMPX_CLASS_F32,
            Opcode::V_CMPX_F_F64,
            Opcode::V_CMPX_T_F64,
            Opcode::V_CMPSX_F_F32,
            Opcode::V_CMPSX_T_F32,
            Opcode::V_CMPSX_F_F64,
            Opcode::V_CMPX_F_I32,
            Opcode::V_CMPX_F_I64,
            Opcode::V_CMPX_T_U32,
            Opcode::V_CMPX_T_U64,
        ] {
            inst.opcode = opcode;
            assert!(inst.is_cmpx(), "{opcode:?}");
        }
        for opcode in [Opcode::V_CMP_F_F32, Opcode::V_CMP_T_F32, Opcode::S_BRANCH] {
            inst.opcode = opcode;
            assert!(!inst.is_cmpx(), "{opcode:?}");
        }
    }

    #[test]
    fn the_sdwa_and_dpp_words_decode_their_fields() {
        let sdwa = Sdwa::from_raw(0xAB | 3 << 8 | 2 << 11 | 1 << 13 | 2 << 14 | 5 << 16);
        let vop12 = sdwa.vop12();
        assert_eq!(vop12.src0(), 0xAB);
        assert_eq!(vop12.dst_sel(), SdwaSelector::Byte3);
        assert_eq!(vop12.dst_u(), SdwaDstUnused::Preserve);
        assert!(vop12.clamp());
        assert_eq!(vop12.omod(), 2);
        assert_eq!(vop12.src0_sel(), SdwaSelector::Word1);
        // The compare form reads the same word with a seven-bit destination and its flag.
        let vopc = sdwa.vopc();
        assert_eq!(vopc.src0(), 0xAB);
        assert_eq!(vopc.sdst(), 3 | 2 << 3 | 1 << 5);
        assert!(vopc.sd(), "the flag is the bit above the destination");
        assert_eq!(vopc.src0_sel(), SdwaSelector::Word1);
        // The DPP control decodes into the named ranges, and refuses the rest. The control
        // lives at bit 8, so every case shifts it there.
        let dpp = Dpp::from_raw(0x101 << 8);
        let op = dpp.operation().unwrap();
        assert_eq!(op.op, DppCtrl::DppRowShl);
        assert_eq!(op.value, 1);
        let dpp = Dpp::from_raw(0x0C0 << 8);
        assert_eq!(dpp.operation().unwrap().op, DppCtrl::DppQuadPerm);
        assert_eq!(dpp.operation().unwrap().value, 0xC0);
        let dpp = Dpp::from_raw((DppCtrl::DppRowMirror as u32) << 8);
        assert_eq!(dpp.operation().unwrap().op, DppCtrl::DppRowMirror);
        assert_eq!(dpp.operation().unwrap().value, 0);
        let dpp = Dpp::from_raw((DppCtrl::DppRowHalfMirror as u32) << 8);
        assert_eq!(dpp.operation().unwrap().op, DppCtrl::DppRowHalfMirror);
        let dpp = Dpp::from_raw((DppCtrl::DppRowRor as u32) << 8);
        assert_eq!(dpp.operation().unwrap().op, DppCtrl::DppRowRor);
        assert_eq!(dpp.operation().unwrap().value, 1, "the low nibble of 0x121");
        let dpp = Dpp::from_raw(0x100 << 8);
        assert_eq!(dpp.operation(), None, "0x100 is none of the controls");
        let dpp = Dpp::from_raw(0x1FF << 8);
        assert_eq!(dpp.operation(), None);
        let dpp = Dpp::from_raw(0x12A << 8);
        assert_eq!(dpp.operation().unwrap().op, DppCtrl::DppRowRor);
        assert_eq!(dpp.operation().unwrap().value, 0xA);
        let dpp = Dpp::from_raw(1 << 4 | 0o14 << 24 | 0o12 << 28 | 1 << 19);
        assert_eq!(dpp.row_mask(), 0o12);
        assert_eq!(dpp.bank_mask(), 0o14);
        assert!(dpp.bc());
    }

    #[test]
    fn the_operand_defaults_come_from_the_header() {
        let operand = InstOperand::default();
        assert_eq!(operand.code, 0xFFFF_FFFF);
        assert_eq!(operand.field, OperandField::Undefined.0);
        assert_eq!(operand.type_, ScalarType::Undefined);
        assert_eq!(operand.sdwa_sel, SdwaSelector::Invalid);
        assert_eq!(operand.sdwa_dst, SdwaDstUnused::Invalid);
        assert!(operand.dpp.is_none());
        let operand = Operand::default();
        assert_eq!(operand.code, 0xFFFF_FFFF);
        assert_eq!(operand.field, OperandField::Undefined.0);
        assert_eq!(
            OperandFieldRange::SignedConstIntPosMin as u32,
            OperandField::SignedConstIntPos.0
        );
        assert_eq!(OperandFieldRange::VectorGPRMax as u32, 511);
        assert_eq!(GCN_MAX_SRC_COUNT, 4);
        assert_eq!(GCN_MAX_DST_COUNT, 2);
        assert_eq!(InstControlVOP3::default().raw(), 0);
        assert_eq!(
            std::mem::size_of::<InstControlVOP3P>(),
            8,
            "the C++ asserts eight"
        );
        assert_eq!(
            std::mem::size_of::<GcnInst>(),
            std::mem::size_of::<GcnInst>()
        );
    }
}
