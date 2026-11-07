//! The GCN instruction decoder, ported from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/decode.h` and `decode.cpp`. The format tables
//! live in `format_gen` (generated from `format.cpp`); this module is the decoder itself, which
//! turns a word of code into a `GcnInst`.
//!
//! `decode.h` also declares `GetEncodingLength` and `DecodeOpcode`, and defines neither anywhere
//! in the C++ tree, so the port has no equivalent; `GcnDecodeContext::getEncodingLength` is the
//! one that exists. `decode.cpp`'s `LOG_WARNING`/`LOG_ERROR` calls are dropped with the rest of
//! the logging, and where the C++ assigns a small member of the control union (leaving the rest
//! of the union as the cleared instruction left it) the port writes the same word.

// The decoder keeps `decode.cpp`'s shape: switches that test one value, ranges written as two
// comparisons, and a lower bound that is zero where the C++ still names it. Those are the only
// things the lints below complain about, so they are allowed for the module.
#![allow(
    clippy::manual_range_contains,
    clippy::absurd_extreme_comparisons,
    clippy::match_like_matches_macro,
    clippy::collapsible_match
)]

use super::format_gen::*;
use super::instruction::{Dpp, GcnInst, InstControl, OperandFieldRange, Sdwa};
use super::opcodes::OperandField;
use super::opcodes::*;

/// `Shader::Gcn::bit::extract`: the bits `[fst, lst]` of a value.
pub fn extract(value: u64, lst: u32, fst: u32) -> u64 {
    let width = lst - fst + 1;
    let mask = if width >= 64 {
        u64::MAX
    } else {
        !(u64::MAX << width)
    };
    (value >> fst) & mask
}

/// `Shader::Gcn::InstFormat`: what the tables say about an opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstFormat {
    pub inst_class: InstClass,
    pub inst_category: InstCategory,
    pub src_count: u32,
    pub dst_count: u32,
    pub src_type: ScalarType,
    pub dst_type: ScalarType,
}

impl InstFormat {
    /// An entry the C++ writes as `{}`.
    pub const EMPTY: InstFormat = InstFormat {
        inst_class: InstClass::Undefined,
        inst_category: InstCategory::Undefined,
        src_count: 0,
        dst_count: 0,
        src_type: ScalarType::Undefined,
        dst_type: ScalarType::Undefined,
    };
}

impl Default for InstFormat {
    fn default() -> Self {
        InstFormat::EMPTY
    }
}

/// `Shader::Gcn::InstructionFormat`: the table for an encoding, indexed by the encoding's own
/// opcode.
pub fn instruction_format(encoding: InstEncoding, opcode: u32) -> InstFormat {
    let opcode = opcode as usize;
    match encoding {
        InstEncoding::SOP1 => INSTRUCTION_FORMAT_SOP1[opcode],
        InstEncoding::SOPP => INSTRUCTION_FORMAT_SOPP[opcode],
        InstEncoding::SOPC => INSTRUCTION_FORMAT_SOPC[opcode],
        InstEncoding::VOP1 => INSTRUCTION_FORMAT_VOP1[opcode],
        InstEncoding::VOPC => INSTRUCTION_FORMAT_VOPC[opcode],
        InstEncoding::VOP3 => INSTRUCTION_FORMAT_VOP3[opcode],
        InstEncoding::VOP3P => INSTRUCTION_FORMAT_VOP3P[opcode],
        InstEncoding::EXP => INSTRUCTION_FORMAT_EXP[opcode],
        InstEncoding::VINTRP => INSTRUCTION_FORMAT_VINTRP[opcode],
        InstEncoding::DS => INSTRUCTION_FORMAT_DS[opcode],
        InstEncoding::MUBUF => INSTRUCTION_FORMAT_MUBUF[opcode],
        InstEncoding::MTBUF => INSTRUCTION_FORMAT_MTBUF[opcode],
        InstEncoding::MIMG => INSTRUCTION_FORMAT_MIMG[opcode],
        InstEncoding::SMRD => INSTRUCTION_FORMAT_SMRD[opcode],
        InstEncoding::SOPK => INSTRUCTION_FORMAT_SOPK[opcode],
        InstEncoding::SOP2 => INSTRUCTION_FORMAT_SOP2[opcode],
        InstEncoding::VOP2 => INSTRUCTION_FORMAT_VOP2[opcode],
        _ => panic!("Unreachable"),
    }
}

/// `GetInstructionEncoding`: which encoding a token belongs to, tried from the widest mask down.
pub fn get_instruction_encoding(token: u32) -> InstEncoding {
    let mut encoding = InstEncoding(token & EncodingMask::MASK_9bit.0);
    if matches!(
        encoding,
        InstEncoding::SOP1 | InstEncoding::SOPP | InstEncoding::SOPC
    ) {
        return encoding;
    }

    encoding = InstEncoding(token & EncodingMask::MASK_7bit.0);
    if matches!(encoding, InstEncoding::VOP1 | InstEncoding::VOPC) {
        return encoding;
    }

    encoding = InstEncoding(token & EncodingMask::MASK_6bit.0);
    if matches!(
        encoding,
        InstEncoding::VOP3
            | InstEncoding::VOP3P
            | InstEncoding::EXP
            | InstEncoding::VINTRP
            | InstEncoding::DS
            | InstEncoding::MUBUF
            | InstEncoding::MTBUF
            | InstEncoding::MIMG
    ) {
        return encoding;
    }

    encoding = InstEncoding(token & EncodingMask::MASK_5bit.0);
    if matches!(encoding, InstEncoding::SMRD) {
        return encoding;
    }

    encoding = InstEncoding(token & EncodingMask::MASK_4bit.0);
    if matches!(encoding, InstEncoding::SOPK) {
        return encoding;
    }

    encoding = InstEncoding(token & EncodingMask::MASK_2bit.0);
    if matches!(encoding, InstEncoding::SOP2) {
        return encoding;
    }

    encoding = InstEncoding(token & EncodingMask::MASK_1bit.0);
    if matches!(encoding, InstEncoding::VOP2) {
        return encoding;
    }

    InstEncoding::ILLEGAL
}

/// `HasAdditionalLiteral`: the two forms that carry their literal in the following word.
pub fn has_additional_literal(encoding: InstEncoding, opcode: Opcode) -> bool {
    match encoding {
        InstEncoding::SOPK => opcode == Opcode::S_SETREG_IMM32_B32,
        InstEncoding::VOP2 => opcode == Opcode::V_MADMK_F32 || opcode == Opcode::V_MADAK_F32,
        _ => false,
    }
}

/// `IsVop3BEncoding`: the arithmetic that writes a second scalar destination.
pub fn is_vop3b_encoding(opcode: Opcode) -> bool {
    opcode == Opcode::V_ADD_I32
        || opcode == Opcode::V_ADDC_U32
        || opcode == Opcode::V_SUB_I32
        || opcode == Opcode::V_SUBB_U32
        || opcode == Opcode::V_SUBREV_I32
        || opcode == Opcode::V_SUBBREV_U32
        || opcode == Opcode::V_DIV_SCALE_F32
        || opcode == Opcode::V_DIV_SCALE_F64
        || opcode == Opcode::V_MAD_U64_U32
        || opcode == Opcode::V_MAD_I64_I32
}

/// `Shader::Gcn::GcnCodeSlice`: a cursor over the code words. The C++ holds pointers; the port
/// holds the slice and an index, and `position` is that index.
#[derive(Debug, Clone, Copy)]
pub struct GcnCodeSlice<'a> {
    words: &'a [u32],
    index: usize,
}

impl<'a> GcnCodeSlice<'a> {
    /// `GcnCodeSlice(const u32* ptr, const u32* end)`.
    pub fn new(words: &'a [u32]) -> Self {
        GcnCodeSlice { words, index: 0 }
    }

    /// `at`.
    pub fn at(&self, id: u32) -> u32 {
        self.words[self.index + id as usize]
    }

    /// `readu32`.
    pub fn read_u32(&mut self) -> u32 {
        let value = self.words[self.index];
        self.index += 1;
        value
    }

    /// `readu64`: the C++ reads the two words as one little-endian value.
    pub fn read_u64(&mut self) -> u64 {
        let low = self.words[self.index] as u64;
        let high = self.words[self.index + 1] as u64;
        self.index += 2;
        low | (high << 32)
    }

    /// `atEnd`.
    pub fn at_end(&self) -> bool {
        self.index >= self.words.len()
    }

    /// The header's `position`, which the fetch shader parse cache uses.
    pub fn position(&self) -> usize {
        self.index
    }
}

/// `Shader::Gcn::GcnDecodeContext`.
#[derive(Debug, Clone, Copy, Default)]
pub struct GcnDecodeContext {
    instruction: GcnInst,
}

impl GcnDecodeContext {
    /// A decoder, which the C++ leaves uninitialised until the first instruction.
    pub fn new() -> Self {
        GcnDecodeContext::default()
    }

    /// `decodeInstruction`.
    pub fn decode_instruction(&mut self, code: &mut GcnCodeSlice) -> GcnInst {
        let token = code.at(0);

        let encoding = get_instruction_encoding(token);
        assert!(
            encoding != InstEncoding::ILLEGAL,
            "illegal encoding: {token:#x}"
        );
        let encoding_len = self.get_encoding_length(encoding);

        // Clear the instruction
        self.instruction = GcnInst::default();

        // Decode
        if encoding_len == std::mem::size_of::<u32>() as u32 {
            self.decode_instruction32(encoding, code);
        } else {
            self.decode_instruction64(encoding, code);
        }

        // Update instruction meta info.
        self.update_instruction_meta(encoding);

        // Detect literal constant. Only 32 bits instructions may have literal constant.
        // Note: Literal constant decode must be performed after meta info updated.
        if encoding_len == std::mem::size_of::<u32>() as u32 {
            self.decode_literal_constant(encoding, code);
            self.decode_sub_dword_addressing(encoding, code);
            self.decode_data_parallel_primitive(encoding, code);
        }

        self.repair_operand_type();
        self.instruction
    }

    /// `getEncodingLength`.
    fn get_encoding_length(&self, encoding: InstEncoding) -> u32 {
        match encoding {
            InstEncoding::SOP1
            | InstEncoding::SOPP
            | InstEncoding::SOPC
            | InstEncoding::SOPK
            | InstEncoding::SOP2
            | InstEncoding::VOP1
            | InstEncoding::VOPC
            | InstEncoding::VOP2
            | InstEncoding::SMRD
            | InstEncoding::VINTRP => std::mem::size_of::<u32>() as u32,

            InstEncoding::VOP3
            | InstEncoding::VOP3P
            | InstEncoding::MUBUF
            | InstEncoding::MTBUF
            | InstEncoding::MIMG
            | InstEncoding::DS
            | InstEncoding::EXP => std::mem::size_of::<u64>() as u32,

            _ => 0,
        }
    }

    /// `getOpMapOffset`.
    fn get_op_map_offset(&self, encoding: InstEncoding) -> u32 {
        match encoding {
            InstEncoding::SOP1 => OpcodeMap::OP_MAP_SOP1.0,
            InstEncoding::SOPP => OpcodeMap::OP_MAP_SOPP.0,
            InstEncoding::SOPC => OpcodeMap::OP_MAP_SOPC.0,
            InstEncoding::VOP1 => OpcodeMap::OP_MAP_VOP1.0,
            InstEncoding::VOPC => OpcodeMap::OP_MAP_VOPC.0,
            InstEncoding::VOP3 => OpcodeMap::OP_MAP_VOP3.0,
            InstEncoding::VOP3P => OpcodeMap::OP_MAP_VOP3P.0,
            InstEncoding::EXP => OpcodeMap::OP_MAP_EXP.0,
            InstEncoding::VINTRP => OpcodeMap::OP_MAP_VINTRP.0,
            InstEncoding::DS => OpcodeMap::OP_MAP_DS.0,
            InstEncoding::MUBUF => OpcodeMap::OP_MAP_MUBUF.0,
            InstEncoding::MTBUF => OpcodeMap::OP_MAP_MTBUF.0,
            InstEncoding::MIMG => OpcodeMap::OP_MAP_MIMG.0,
            InstEncoding::SMRD => OpcodeMap::OP_MAP_SMRD.0,
            InstEncoding::SOPK => OpcodeMap::OP_MAP_SOPK.0,
            InstEncoding::SOP2 => OpcodeMap::OP_MAP_SOP2.0,
            InstEncoding::VOP2 => OpcodeMap::OP_MAP_VOP2.0,
            _ => 0,
        }
    }

    /// `mapEncodingOp`: from the unified opcode back to the one this encoding carries.
    fn map_encoding_op(&self, encoding: InstEncoding, opcode: Opcode) -> u32 {
        // Map from uniform opcode to encoding specific opcode.
        if encoding == InstEncoding::VOP3 {
            if opcode.0 >= Opcode::V_CMP_F_F32.0 && opcode.0 <= Opcode::V_CMPX_T_U64.0 {
                let op = opcode.0 - OpcodeMap::OP_MAP_VOPC.0;
                op + OpMapVOP3VOPX::VOP3_TO_VOPC.0
            } else if opcode.0 >= Opcode::V_CNDMASK_B32.0 && opcode.0 <= Opcode::V_LDEXP_F16.0 {
                let op = opcode.0 - OpcodeMap::OP_MAP_VOP2.0;
                op + OpMapVOP3VOPX::VOP3_TO_VOP2.0
            } else if opcode.0 >= Opcode::V_NOP.0 && opcode.0 <= Opcode::V_SQRT_F16.0 {
                let op = opcode.0 - OpcodeMap::OP_MAP_VOP1.0;
                op + OpMapVOP3VOPX::VOP3_TO_VOP1.0
            } else {
                opcode.0 - OpcodeMap::OP_MAP_VOP3.0
            }
        } else {
            let map_offset = self.get_op_map_offset(encoding);
            opcode.0 - map_offset
        }
    }

    /// `updateInstructionMeta`.
    fn update_instruction_meta(&mut self, encoding: InstEncoding) {
        let encoding_op = self.map_encoding_op(encoding, self.instruction.opcode);
        let inst_format = instruction_format(encoding, encoding_op);

        assert!(
            inst_format.src_type != ScalarType::Undefined
                && inst_format.dst_type != ScalarType::Undefined,
            "Instruction format table incomplete for opcode {} ({}, encoding = {:#x})",
            opcode_name(self.instruction.opcode),
            self.instruction.opcode.0,
            encoding.0
        );

        self.instruction.inst_class = inst_format.inst_class;
        self.instruction.category = inst_format.inst_category;
        self.instruction.encoding = encoding;
        self.instruction.src_count = inst_format.src_count;
        self.instruction.length = self.get_encoding_length(encoding);

        // Update src operand scalar type.
        for index in 0..self.instruction.src_count as usize {
            let src = &mut self.instruction.src[index];
            // Only update uninitialized numeric type.
            if src.type_ == ScalarType::Undefined {
                src.type_ = inst_format.src_type;
            }
        }

        // Update dst operand scalar type.
        match self.instruction.dst_count {
            2 => {
                if self.instruction.dst[1].type_ == ScalarType::Undefined {
                    // Only VOP3B has an additional sdst operand,
                    // and it must be Uint64
                    self.instruction.dst[1].type_ = ScalarType::Uint64;
                }
                if self.instruction.dst[0].type_ == ScalarType::Undefined {
                    self.instruction.dst[0].type_ = inst_format.dst_type;
                }
            }
            1 => {
                if self.instruction.dst[0].type_ == ScalarType::Undefined {
                    self.instruction.dst[0].type_ = inst_format.dst_type;
                }
            }
            _ => {}
        }
    }

    /// `repairOperandType`.
    fn repair_operand_type(&mut self) {
        // Some instructions' operand type is not uniform,
        // it's best to change the instruction table's format and fix them there,
        // but it's a hard work.
        // We fix them here.
        match self.instruction.opcode {
            Opcode::V_MAD_U64_U32 => self.instruction.src[2].type_ = ScalarType::Uint64,
            Opcode::V_MAD_I64_I32 => self.instruction.src[2].type_ = ScalarType::Sint64,
            Opcode::V_ADDC_U32 => self.instruction.src[2].type_ = ScalarType::Uint64,
            Opcode::IMAGE_GATHER4_C | Opcode::IMAGE_GATHER4_C_O => {
                self.instruction.src[0].type_ = ScalarType::Any
            }
            _ => {}
        }
    }

    /// `getOperandField`.
    fn get_operand_field(&self, code: u32) -> OperandField {
        if code >= OperandFieldRange::ScalarGPRMin as u32
            && code <= OperandFieldRange::ScalarGPRMax as u32
        {
            OperandField::ScalarGPR
        } else if code >= OperandFieldRange::SignedConstIntPosMin as u32
            && code <= OperandFieldRange::SignedConstIntPosMax as u32
        {
            OperandField::SignedConstIntPos
        } else if code >= OperandFieldRange::SignedConstIntNegMin as u32
            && code <= OperandFieldRange::SignedConstIntNegMax as u32
        {
            OperandField::SignedConstIntNeg
        } else if code >= OperandFieldRange::VectorGPRMin as u32
            && code <= OperandFieldRange::VectorGPRMax as u32
        {
            OperandField::VectorGPR
        } else {
            OperandField(code)
        }
    }

    /// `decodeInstruction32`.
    fn decode_instruction32(&mut self, encoding: InstEncoding, code: &mut GcnCodeSlice) {
        let hex_instruction = code.read_u32();
        match encoding {
            InstEncoding::SOP1 => self.decode_instruction_sop1(hex_instruction),
            InstEncoding::SOPP => self.decode_instruction_sopp(hex_instruction),
            InstEncoding::SOPC => self.decode_instruction_sopc(hex_instruction),
            InstEncoding::SOPK => self.decode_instruction_sopk(hex_instruction),
            InstEncoding::SOP2 => self.decode_instruction_sop2(hex_instruction),
            InstEncoding::VOP1 => self.decode_instruction_vop1(hex_instruction),
            InstEncoding::VOPC => self.decode_instruction_vopc(hex_instruction),
            InstEncoding::VOP2 => self.decode_instruction_vop2(hex_instruction),
            InstEncoding::SMRD => self.decode_instruction_smrd(hex_instruction),
            InstEncoding::VINTRP => self.decode_instruction_vintrp(hex_instruction),
            _ => {}
        }
    }

    /// `decodeInstruction64`.
    fn decode_instruction64(&mut self, encoding: InstEncoding, code: &mut GcnCodeSlice) {
        let hex_instruction = code.read_u64();
        match encoding {
            InstEncoding::VOP3 => self.decode_instruction_vop3(hex_instruction),
            InstEncoding::VOP3P => self.decode_instruction_vop3p(hex_instruction),
            InstEncoding::MUBUF => self.decode_instruction_mubuf(hex_instruction),
            InstEncoding::MTBUF => self.decode_instruction_mtbuf(hex_instruction),
            InstEncoding::MIMG => self.decode_instruction_mimg(hex_instruction),
            InstEncoding::DS => self.decode_instruction_ds(hex_instruction),
            InstEncoding::EXP => self.decode_instruction_exp(hex_instruction),
            _ => {}
        }
    }

    /// `decodeLiteralConstant`.
    fn decode_literal_constant(&mut self, encoding: InstEncoding, code: &mut GcnCodeSlice) {
        if has_additional_literal(encoding, self.instruction.opcode) {
            let encoding_op = self.map_encoding_op(encoding, self.instruction.opcode);
            let inst_format = instruction_format(encoding, encoding_op);
            let index = self.instruction.src_count as usize;
            self.instruction.src[index].field = OperandField::LiteralConst.0;
            self.instruction.src[index].type_ = inst_format.src_type;
            self.instruction.src[index].code = code.read_u32();
            self.instruction.src_count += 1;
            self.instruction.length += std::mem::size_of::<u32>() as u32;
            return;
        }

        // Find if the instruction contains a literal constant
        let literal = self
            .instruction
            .src
            .iter()
            .position(|src| src.field == OperandField::LiteralConst.0);
        if let Some(index) = literal {
            self.instruction.src[index].code = code.read_u32();
            self.instruction.length += std::mem::size_of::<u32>() as u32;
        }
    }

    /// `decodeSubDwordAddressing`.
    fn decode_sub_dword_addressing(&mut self, encoding: InstEncoding, code: &mut GcnCodeSlice) {
        // Find if the instruction contains SDWA (it's legal only as src0)
        if self.instruction.src[0].field == OperandField::Sdwa.0 {
            self.instruction.src[0].code = code.read_u32();
            self.instruction.length += std::mem::size_of::<u32>() as u32;

            if encoding == InstEncoding::VOPC {
                let sdwa = Sdwa::from_raw(self.instruction.src[0].code).vopc();

                self.instruction.src[0].field = if !sdwa.s0() {
                    OperandField::VectorGPR.0
                } else {
                    self.get_operand_field(sdwa.src0()).0
                };
                self.instruction.src[0].code = sdwa.src0();
                self.instruction.src[0].sdwa_sel = sdwa.src0_sel();

                self.instruction.src[0].input_modifier.neg = sdwa.src0_neg();
                self.instruction.src[0].input_modifier.abs = sdwa.src0_abs();
                self.instruction.src[0].input_modifier.sext = sdwa.src0_sext();

                self.instruction.src[1].field = if !sdwa.s1() {
                    OperandField::VectorGPR.0
                } else {
                    self.get_operand_field(self.instruction.src[1].code).0
                };
                self.instruction.src[1].sdwa_sel = sdwa.src1_sel();

                self.instruction.src[1].input_modifier.neg = sdwa.src1_neg();
                self.instruction.src[1].input_modifier.abs = sdwa.src1_abs();
                self.instruction.src[1].input_modifier.sext = sdwa.src1_sext();

                self.instruction.dst[0].field = if sdwa.sd() {
                    OperandField::ScalarGPR.0
                } else {
                    OperandField::VccLo.0
                };
                self.instruction.dst[0].code = sdwa.sdst();
            } else if encoding == InstEncoding::VOP1 || encoding == InstEncoding::VOP2 {
                let sdwa = Sdwa::from_raw(self.instruction.src[0].code).vop12();

                self.instruction.src[0].field = if !sdwa.s0() {
                    OperandField::VectorGPR.0
                } else {
                    self.get_operand_field(sdwa.src0()).0
                };
                self.instruction.src[0].code = sdwa.src0();
                self.instruction.src[0].sdwa_sel = sdwa.src0_sel();

                self.instruction.src[0].input_modifier.neg = sdwa.src0_neg();
                self.instruction.src[0].input_modifier.abs = sdwa.src0_abs();
                self.instruction.src[0].input_modifier.sext = sdwa.src0_sext();

                self.instruction.src[1].field = if !sdwa.s1() {
                    OperandField::VectorGPR.0
                } else {
                    self.get_operand_field(self.instruction.src[1].code).0
                };
                self.instruction.src[1].sdwa_sel = sdwa.src1_sel();

                self.instruction.src[1].input_modifier.neg = sdwa.src1_neg();
                self.instruction.src[1].input_modifier.abs = sdwa.src1_abs();
                self.instruction.src[1].input_modifier.sext = sdwa.src1_sext();

                self.instruction.dst[0].sdwa_sel = sdwa.dst_sel();
                self.instruction.dst[0].sdwa_dst = sdwa.dst_u();
                self.instruction.dst[0].output_modifier.clamp = sdwa.clamp();

                self.instruction.dst[0].output_modifier.multiplier = match sdwa.omod() {
                    0 => 0.0,
                    1 => 2.0,
                    2 => 4.0,
                    3 => 0.5,
                    _ => 0.0,
                };
            } else {
                // The C++ logs that SDWA outside VOP1/VOP2/VOPC is illegal and carries on.
            }
        }
    }

    /// `decodeDataParallelPrimitive`.
    fn decode_data_parallel_primitive(&mut self, _encoding: InstEncoding, code: &mut GcnCodeSlice) {
        // Find if the instruction contains DPP
        if self.instruction.src[0].field == OperandField::Dpp.0 {
            self.instruction.src[0].code = code.read_u32();
            self.instruction.length += std::mem::size_of::<u32>() as u32;

            let dpp = Dpp::from_raw(self.instruction.src[0].code);

            self.instruction.src[0].field = OperandField::VectorGPR.0;
            self.instruction.src[0].code = dpp.src0();

            if dpp.src0_abs() {
                self.instruction.src[0].input_modifier.abs = true;
            }
            if dpp.src0_neg() {
                self.instruction.src[0].input_modifier.neg = true;
            }
            if dpp.src1_abs() {
                self.instruction.src[1].input_modifier.abs = true;
            }
            if dpp.src1_neg() {
                self.instruction.src[1].input_modifier.neg = true;
            }

            // The C++ reports every DPP operation as unhandled before storing it.
            let op = dpp.operation().expect("malformed dpp_ctrl");
            self.instruction.src[0].dpp = Some(op);
        }
    }

    /// `decodeInstructionSOP1`.
    fn decode_instruction_sop1(&mut self, hex_instruction: u32) {
        let ssrc0 = extract(hex_instruction as u64, 7, 0) as u32;
        let op = extract(hex_instruction as u64, 15, 8) as u32;
        let sdst = extract(hex_instruction as u64, 22, 16) as u32;

        self.instruction.opcode = Opcode(op + OpcodeMap::OP_MAP_SOP1.0);

        self.instruction.src[0].field = self.get_operand_field(ssrc0).0;
        self.instruction.src[0].code = ssrc0;
        self.instruction.dst[0].field = self.get_operand_field(sdst).0;
        self.instruction.dst[0].code = sdst;
        self.instruction.dst_count = 1;
    }

    /// `decodeInstructionSOPP`.
    fn decode_instruction_sopp(&mut self, hex_instruction: u32) {
        let op = extract(hex_instruction as u64, 22, 16) as u32;

        self.instruction.opcode = Opcode(op + OpcodeMap::OP_MAP_SOPP.0);

        // The union member is a signed half word, so only the low 16 bits of the token go in.
        self.instruction.control = InstControl::from_raw(hex_instruction as u16 as u64);
    }

    /// `decodeInstructionSOPC`.
    fn decode_instruction_sopc(&mut self, hex_instruction: u32) {
        let ssrc0 = extract(hex_instruction as u64, 7, 0) as u32;
        let ssrc1 = extract(hex_instruction as u64, 15, 8) as u32;
        let op = extract(hex_instruction as u64, 22, 16) as u32;

        self.instruction.opcode = Opcode(op + OpcodeMap::OP_MAP_SOPC.0);

        self.instruction.src[0].field = self.get_operand_field(ssrc0).0;
        self.instruction.src[0].code = ssrc0;
        self.instruction.src[1].field = self.get_operand_field(ssrc1).0;
        self.instruction.src[1].code = ssrc1;
    }

    /// `decodeInstructionSOPK`.
    fn decode_instruction_sopk(&mut self, hex_instruction: u32) {
        let sdst = extract(hex_instruction as u64, 22, 16) as u32;
        let op = extract(hex_instruction as u64, 27, 23) as u32;

        self.instruction.opcode = Opcode(op + OpcodeMap::OP_MAP_SOPK.0);

        self.instruction.dst[0].field = self.get_operand_field(sdst).0;
        self.instruction.dst[0].code = sdst;
        self.instruction.dst_count = 1;

        self.instruction.control = InstControl::from_raw(hex_instruction as u16 as u64);
    }

    /// `decodeInstructionSOP2`.
    fn decode_instruction_sop2(&mut self, hex_instruction: u32) {
        let ssrc0 = extract(hex_instruction as u64, 7, 0) as u32;
        let ssrc1 = extract(hex_instruction as u64, 15, 8) as u32;
        let sdst = extract(hex_instruction as u64, 22, 16) as u32;
        let op = extract(hex_instruction as u64, 29, 23) as u32;

        self.instruction.opcode = Opcode(op + OpcodeMap::OP_MAP_SOP2.0);

        self.instruction.src[0].field = self.get_operand_field(ssrc0).0;
        self.instruction.src[0].code = ssrc0;
        self.instruction.src[1].field = self.get_operand_field(ssrc1).0;
        self.instruction.src[1].code = ssrc1;
        self.instruction.dst[0].field = self.get_operand_field(sdst).0;
        self.instruction.dst[0].code = sdst;
        self.instruction.dst_count = 1;
    }

    /// `decodeInstructionVOP1`.
    fn decode_instruction_vop1(&mut self, hex_instruction: u32) {
        let src0 = extract(hex_instruction as u64, 8, 0) as u32;
        let op = extract(hex_instruction as u64, 16, 9) as u32;
        let vdst = extract(hex_instruction as u64, 24, 17) as u32;

        self.instruction.opcode = Opcode(op + OpcodeMap::OP_MAP_VOP1.0);

        self.instruction.src[0].field = self.get_operand_field(src0).0;
        self.instruction.src[0].code = if self.instruction.src[0].field == OperandField::VectorGPR.0
        {
            src0 - OperandFieldRange::VectorGPRMin as u32
        } else {
            src0
        };
        self.instruction.dst[0].field = OperandField::VectorGPR.0;
        self.instruction.dst[0].code = vdst;
        self.instruction.dst_count = 1;

        if OpcodeVOP1(op) == OpcodeVOP1::V_READFIRSTLANE_B32 {
            self.instruction.dst[0].field = self.get_operand_field(vdst).0;
            self.instruction.dst[0].type_ = ScalarType::Uint32;
        }
    }

    /// `decodeInstructionVOPC`.
    fn decode_instruction_vopc(&mut self, hex_instruction: u32) {
        let src0 = extract(hex_instruction as u64, 8, 0) as u32;
        let vsrc1 = extract(hex_instruction as u64, 16, 9) as u32;
        let op = extract(hex_instruction as u64, 24, 17) as u32;

        self.instruction.opcode = Opcode(op + OpcodeMap::OP_MAP_VOPC.0);

        self.instruction.src[0].field = self.get_operand_field(src0).0;
        self.instruction.src[0].code = if self.instruction.src[0].field == OperandField::VectorGPR.0
        {
            src0 - OperandFieldRange::VectorGPRMin as u32
        } else {
            src0
        };
        self.instruction.src[1].field = OperandField::VectorGPR.0;
        self.instruction.src[1].code = vsrc1;
        // VOPC dst is forced to VCC.
        // In order to be unified with VOP3 encoding,
        // we store it to dst[1]
        self.instruction.dst[1].field = OperandField::VccLo.0;
        self.instruction.dst[1].type_ = ScalarType::Uint64;
        self.instruction.dst[1].code = OperandField::VccLo.0;
    }

    /// `decodeInstructionVOP2`.
    fn decode_instruction_vop2(&mut self, hex_instruction: u32) {
        let src0 = extract(hex_instruction as u64, 8, 0) as u32;
        let vsrc1 = extract(hex_instruction as u64, 16, 9) as u32;
        let vdst = extract(hex_instruction as u64, 24, 17) as u32;
        let op = extract(hex_instruction as u64, 30, 25) as u32;

        self.instruction.opcode = Opcode(op + OpcodeMap::OP_MAP_VOP2.0);

        self.instruction.src[0].field = self.get_operand_field(src0).0;
        self.instruction.src[0].code = if self.instruction.src[0].field == OperandField::VectorGPR.0
        {
            src0 - OperandFieldRange::VectorGPRMin as u32
        } else {
            src0
        };
        self.instruction.src[1].field = OperandField::VectorGPR.0;
        self.instruction.src[1].code = vsrc1;
        self.instruction.dst[0].field = OperandField::VectorGPR.0;
        self.instruction.dst[0].code = vdst;
        self.instruction.dst_count = 1;

        if OpcodeVOP2(op) == OpcodeVOP2::V_READLANE_B32 {
            // vsrc1 is scalar for lane instructions
            self.instruction.src[1].field = self.get_operand_field(vsrc1).0;
            // dst is sgpr
            self.instruction.dst[0].field = self.get_operand_field(vdst).0;
            self.instruction.dst[0].type_ = ScalarType::Uint32;
        } else if OpcodeVOP2(op) == OpcodeVOP2::V_WRITELANE_B32 {
            self.instruction.src[1].field = self.get_operand_field(vsrc1).0;
            // dst is vgpr, as normal
        } else if is_vop3b_encoding(self.instruction.opcode) {
            self.instruction.dst[1].field = OperandField::VccLo.0;
            self.instruction.dst[1].type_ = ScalarType::Uint64;
            self.instruction.dst[1].code = OperandField::VccLo.0;
        }
    }

    /// `decodeInstructionSMRD`.
    fn decode_instruction_smrd(&mut self, hex_instruction: u32) {
        let sbase = extract(hex_instruction as u64, 14, 9) as u32;
        let sdst = extract(hex_instruction as u64, 21, 15) as u32;
        let op = extract(hex_instruction as u64, 26, 22) as u32;

        self.instruction.opcode = Opcode(op + OpcodeMap::OP_MAP_SMRD.0);

        self.instruction.src[0].field = OperandField::ScalarGPR.0;
        self.instruction.src[0].code = sbase;
        self.instruction.dst[0].field = OperandField::ScalarGPR.0;
        self.instruction.dst[0].code = sdst;
        self.instruction.dst_count = 1;

        let mut smrd = InstControl::from_raw(hex_instruction as u64).smrd();

        if op <= OpcodeSMRD::S_LOAD_DWORDX16.0 {
            smrd.set_count(1 << op);
        } else if op >= OpcodeSMRD::S_BUFFER_LOAD_DWORD.0
            && op <= OpcodeSMRD::S_BUFFER_LOAD_DWORDX16.0
        {
            smrd.set_count(1 << (op - 8));
        }
        self.instruction.control = InstControl::from_raw(smrd.raw() as u64);

        if !smrd.imm() {
            let code = smrd.offset();
            self.instruction.src[1].field = self.get_operand_field(code).0;
            self.instruction.src[1].type_ = ScalarType::Uint32;
            self.instruction.src[1].code = code;
        }
    }

    /// `decodeInstructionVINTRP`.
    fn decode_instruction_vintrp(&mut self, hex_instruction: u32) {
        let vsrc = extract(hex_instruction as u64, 7, 0) as u32;
        let op = extract(hex_instruction as u64, 17, 16) as u32;
        let vdst = extract(hex_instruction as u64, 25, 18) as u32;

        self.instruction.opcode = Opcode(op + OpcodeMap::OP_MAP_VINTRP.0);

        self.instruction.src[0].field = OperandField::VectorGPR.0;
        self.instruction.src[0].code = vsrc;
        self.instruction.dst[0].field = OperandField::VectorGPR.0;
        self.instruction.dst[0].code = vdst;
        self.instruction.dst_count = 1;

        self.instruction.control = InstControl::from_raw(hex_instruction as u64);
    }

    /// `decodeInstructionVOP3`.
    fn decode_instruction_vop3(&mut self, hex_instruction: u64) {
        let vdst = extract(hex_instruction, 7, 0) as u32;
        let sdst = extract(hex_instruction, 14, 8) as u32; // For VOP3B
        let op = extract(hex_instruction, 25, 17) as u32;
        let op_msb = extract(hex_instruction, 16, 16) as u32;
        let op = op + op_msb * (1 << 9);
        let src0 = extract(hex_instruction, 40, 32) as u32;
        let src1 = extract(hex_instruction, 49, 41) as u32;
        let src2 = extract(hex_instruction, 58, 50) as u32;

        if op >= OpcodeVOP3::V_CMP_F_F32.0 && op <= OpcodeVOP3::V_CMPX_T_U64.0 {
            // Map from VOP3 to VOPC
            let vopc_op = op - OpMapVOP3VOPX::VOP3_TO_VOPC.0;
            self.instruction.opcode = Opcode(vopc_op + OpcodeMap::OP_MAP_VOPC.0);
        } else if op >= OpcodeVOP3::V_CNDMASK_B32.0 && op <= OpcodeVOP3::V_LDEXP_F16.0 {
            // Map from VOP3 to VOP2
            let vop2_op = op - OpMapVOP3VOPX::VOP3_TO_VOP2.0;
            self.instruction.opcode = Opcode(vop2_op + OpcodeMap::OP_MAP_VOP2.0);
        } else if op >= OpcodeVOP3::V_NOP.0 && op <= OpcodeVOP3::V_SQRT_F16.0 {
            // Map from VOP3 to VOP1
            let vop1_op = op - OpMapVOP3VOPX::VOP3_TO_VOP1.0;
            self.instruction.opcode = Opcode(vop1_op + OpcodeMap::OP_MAP_VOP1.0);
        } else {
            // VOP3 encoding, do not map.
            self.instruction.opcode = Opcode(op + OpcodeMap::OP_MAP_VOP3.0);
        }

        for (operand, code) in [(0usize, src0), (1usize, src1), (2usize, src2)] {
            self.instruction.src[operand].field = self.get_operand_field(code).0;
            self.instruction.src[operand].code =
                if self.instruction.src[operand].field == OperandField::VectorGPR.0 {
                    code - OperandFieldRange::VectorGPRMin as u32
                } else {
                    code
                };
        }
        self.instruction.dst[0].field = OperandField::VectorGPR.0;
        self.instruction.dst[0].code = vdst;

        let vop3_op = OpcodeVOP3(op);
        if is_vop3b_encoding(self.instruction.opcode) {
            self.instruction.dst[1].field = self.get_operand_field(sdst).0;
            self.instruction.dst[1].type_ = ScalarType::Uint64;
            self.instruction.dst[1].code = sdst;
        } else if vop3_op >= OpcodeVOP3::V_CMP_F_F32 && vop3_op <= OpcodeVOP3::V_CMPX_T_U64 {
            self.instruction.dst[1].field = self.get_operand_field(vdst).0;
            self.instruction.dst[1].type_ = ScalarType::Uint64;
            self.instruction.dst[1].code = vdst;
        } else if vop3_op == OpcodeVOP3::V_READLANE_B32
            || vop3_op == OpcodeVOP3::V_READFIRSTLANE_B32
        {
            self.instruction.dst[0].field = self.get_operand_field(vdst).0;
            self.instruction.dst[0].type_ = ScalarType::Uint32;
            // WRITELANE can be decoded like other VOP3's
        }

        if op >= OpcodeVOP3::V_ADD_I32.0 && op <= OpcodeVOP3::V_DIV_SCALE_F64.0 {
            // VOP3B has a sdst operand.
            self.instruction.dst_count = 2;
        } else {
            self.instruction.dst_count = 1;
        }

        self.instruction.control = InstControl::from_raw(hex_instruction);

        // update input modifier
        let control = self.instruction.control.vop3();

        // update output modifier
        if !is_vop3b_encoding(self.instruction.opcode) {
            for index in 0..3u32 {
                if control.abs() & (1 << index) != 0 {
                    self.instruction.src[index as usize].input_modifier.abs = true;
                }
            }
            self.instruction.dst[0].output_modifier.clamp = control.clmp();
        }
        for index in 0..3u32 {
            if control.neg() & (1 << index) != 0 {
                self.instruction.src[index as usize].input_modifier.neg = true;
            }

            if control.op_sel() & (1 << index) != 0 {
                self.instruction.src[index as usize].op_sel.op_sel = true;
            }
        }

        self.instruction.dst[0].op_sel.op_sel = control.op_sel() & (1 << 3) != 0;

        self.instruction.dst[0].output_modifier.multiplier = match control.omod() {
            0 => 0.0,
            1 => 2.0,
            2 => 4.0,
            3 => 0.5,
            _ => 0.0,
        };
    }

    /// `decodeInstructionVOP3P`.
    fn decode_instruction_vop3p(&mut self, hex_instruction: u64) {
        let vdst = extract(hex_instruction, 7, 0) as u32;
        let op = extract(hex_instruction, 22, 16) as u32;
        let src0 = extract(hex_instruction, 40, 32) as u32;
        let src1 = extract(hex_instruction, 49, 41) as u32;
        let src2 = extract(hex_instruction, 58, 50) as u32;

        self.instruction.opcode = Opcode(op + OpcodeMap::OP_MAP_VOP3P.0);

        for (operand, code) in [(0usize, src0), (1usize, src1), (2usize, src2)] {
            self.instruction.src[operand].field = self.get_operand_field(code).0;
            self.instruction.src[operand].code =
                if self.instruction.src[operand].field == OperandField::VectorGPR.0 {
                    code - OperandFieldRange::VectorGPRMin as u32
                } else {
                    code
                };
        }
        self.instruction.dst[0].field = OperandField::VectorGPR.0;
        self.instruction.dst[0].code = vdst;

        self.instruction.control = InstControl::from_raw(hex_instruction);

        // update input modifier
        let control = self.instruction.control.vop3p();
        for index in 0..3u32 {
            if control.neg() & (1 << index) != 0 {
                self.instruction.src[index as usize].input_modifier.neg = true;
            }

            if control.neg_hi() & (1 << index) != 0 {
                self.instruction.src[index as usize].input_modifier.neg_hi = true;
            }

            if control.op_sel() & (1 << index) != 0 {
                self.instruction.src[index as usize].op_sel.op_sel = true;
            }

            if control.get_op_sel_hi(index as i32) == Some(true) {
                self.instruction.src[index as usize].op_sel.op_sel_hi = true;
            }
        }

        // update output modifier
        self.instruction.dst[0].output_modifier.clamp = control.clamp() != 0;
    }

    /// `decodeInstructionMUBUF`.
    fn decode_instruction_mubuf(&mut self, hex_instruction: u64) {
        let op = extract(hex_instruction, 24, 18) as u32;
        let vaddr = extract(hex_instruction, 39, 32) as u32;
        let vdata = extract(hex_instruction, 47, 40) as u32;
        let srsrc = extract(hex_instruction, 52, 48) as u32;
        let soffset = extract(hex_instruction, 63, 56) as u32;

        self.instruction.opcode = Opcode(op + OpcodeMap::OP_MAP_MUBUF.0);

        self.instruction.src[0].field = OperandField::VectorGPR.0;
        self.instruction.src[0].code = vaddr;
        self.instruction.src[1].field = OperandField::VectorGPR.0;
        self.instruction.src[1].code = vdata;
        self.instruction.src[2].field = OperandField::ScalarGPR.0;
        self.instruction.src[2].code = srsrc;
        self.instruction.src[3].field = self.get_operand_field(soffset).0;
        self.instruction.src[3].code = soffset;

        let mut mubuf = InstControl::from_raw(hex_instruction).mubuf();

        let dword = std::mem::size_of::<u32>() as u64;
        if op >= OpcodeMUBUF::BUFFER_LOAD_FORMAT_X.0 && op <= OpcodeMUBUF::BUFFER_LOAD_FORMAT_XYZW.0
        {
            mubuf.set_count((op + 1) as u64);
            mubuf.set_size(((op + 1) as u64) * dword);
        } else if op >= OpcodeMUBUF::BUFFER_STORE_FORMAT_X.0
            && op <= OpcodeMUBUF::BUFFER_STORE_FORMAT_XYZW.0
        {
            mubuf.set_count((op - 3) as u64);
            mubuf.set_size(((op - 3) as u64) * dword);
        } else if op >= OpcodeMUBUF::BUFFER_LOAD_DWORD.0 && op <= OpcodeMUBUF::BUFFER_LOAD_DWORDX3.0
        {
            let count = if op == OpcodeMUBUF::BUFFER_LOAD_DWORDX3.0 {
                3
            } else {
                1u32 << (op - 12)
            };
            mubuf.set_count(count as u64);
            mubuf.set_size(count as u64 * dword);
        } else if op >= OpcodeMUBUF::BUFFER_STORE_DWORD.0
            && op <= OpcodeMUBUF::BUFFER_STORE_DWORDX3.0
        {
            let count = if op == OpcodeMUBUF::BUFFER_STORE_DWORDX3.0 {
                3
            } else {
                1u32 << (op - 28)
            };
            mubuf.set_count(count as u64);
            mubuf.set_size(count as u64 * dword);
        } else if op >= OpcodeMUBUF::BUFFER_LOAD_UBYTE.0 && op <= OpcodeMUBUF::BUFFER_LOAD_SSHORT.0
        {
            mubuf.set_count(1);
            if op >= OpcodeMUBUF::BUFFER_LOAD_UBYTE.0 && op <= OpcodeMUBUF::BUFFER_LOAD_SBYTE.0 {
                mubuf.set_size(1);
            } else {
                mubuf.set_size(2);
            }
        } else if op >= OpcodeMUBUF::BUFFER_STORE_BYTE.0 && op <= OpcodeMUBUF::BUFFER_STORE_SHORT.0
        {
            mubuf.set_count(1);
            if op == OpcodeMUBUF::BUFFER_STORE_BYTE.0 {
                mubuf.set_size(1);
            } else {
                mubuf.set_size(2);
            }
        } else if op >= OpcodeMUBUF::BUFFER_ATOMIC_SWAP.0 && op <= OpcodeMUBUF::BUFFER_ATOMIC_FMAX.0
        {
            mubuf.set_count(1);
            mubuf.set_size(dword);
        } else if op >= OpcodeMUBUF::BUFFER_ATOMIC_SWAP_X2.0
            && op <= OpcodeMUBUF::BUFFER_ATOMIC_FMAX_X2.0
        {
            mubuf.set_count(2);
            mubuf.set_size(dword * 2);
        }
        self.instruction.control = InstControl::from_raw(mubuf.raw());
    }

    /// `decodeInstructionMTBUF`.
    fn decode_instruction_mtbuf(&mut self, hex_instruction: u64) {
        let op = extract(hex_instruction, 18, 16) as u32;
        let vaddr = extract(hex_instruction, 39, 32) as u32;
        let vdata = extract(hex_instruction, 47, 40) as u32;
        let srsrc = extract(hex_instruction, 52, 48) as u32;
        let soffset = extract(hex_instruction, 63, 56) as u32;

        self.instruction.opcode = Opcode(op + OpcodeMap::OP_MAP_MTBUF.0);

        self.instruction.src[0].field = OperandField::VectorGPR.0;
        self.instruction.src[0].code = vaddr;
        self.instruction.src[1].field = OperandField::VectorGPR.0;
        self.instruction.src[1].code = vdata;
        self.instruction.src[2].field = OperandField::ScalarGPR.0;
        self.instruction.src[2].code = srsrc;
        self.instruction.src[3].field = self.get_operand_field(soffset).0;
        self.instruction.src[3].code = soffset;

        let mut mtbuf = InstControl::from_raw(hex_instruction).mtbuf();

        if op >= OpcodeMTBUF::TBUFFER_LOAD_FORMAT_X.0
            && op <= OpcodeMTBUF::TBUFFER_LOAD_FORMAT_XYZW.0
        {
            mtbuf.set_count((op + 1) as u64);
        } else if op >= OpcodeMTBUF::TBUFFER_STORE_FORMAT_X.0
            && op <= OpcodeMTBUF::TBUFFER_STORE_FORMAT_XYZW.0
        {
            mtbuf.set_count((op - 3) as u64);
        }
        self.instruction.control = InstControl::from_raw(mtbuf.raw());
    }

    /// `getMimgModifier`: the modifier bits an image instruction implies.
    fn get_mimg_modifier(&self, opcode: Opcode) -> u32 {
        let mut flags = MimgModifierFlags::default();

        match opcode {
            Opcode::IMAGE_SAMPLE => {}
            Opcode::IMAGE_SAMPLE_CL => flags.set(MimgModifier::LodClamp),
            Opcode::IMAGE_SAMPLE_D => flags.set(MimgModifier::Derivative),
            Opcode::IMAGE_SAMPLE_D_CL => {
                flags.set_all(&[MimgModifier::Derivative, MimgModifier::LodClamp])
            }
            Opcode::IMAGE_SAMPLE_L => flags.set(MimgModifier::Lod),
            Opcode::IMAGE_SAMPLE_B => flags.set(MimgModifier::LodBias),
            Opcode::IMAGE_SAMPLE_B_CL => {
                flags.set_all(&[MimgModifier::LodBias, MimgModifier::LodClamp])
            }
            Opcode::IMAGE_SAMPLE_LZ => flags.set(MimgModifier::Level0),
            Opcode::IMAGE_SAMPLE_C => flags.set(MimgModifier::Pcf),
            Opcode::IMAGE_SAMPLE_C_CL => {
                flags.set_all(&[MimgModifier::Pcf, MimgModifier::LodClamp])
            }
            Opcode::IMAGE_SAMPLE_C_D => {
                flags.set_all(&[MimgModifier::Pcf, MimgModifier::Derivative])
            }
            Opcode::IMAGE_SAMPLE_C_D_CL => flags.set_all(&[
                MimgModifier::Pcf,
                MimgModifier::Derivative,
                MimgModifier::LodClamp,
            ]),
            Opcode::IMAGE_SAMPLE_C_L => flags.set_all(&[MimgModifier::Pcf, MimgModifier::Lod]),
            Opcode::IMAGE_SAMPLE_C_B => flags.set_all(&[MimgModifier::Pcf, MimgModifier::LodBias]),
            Opcode::IMAGE_SAMPLE_C_B_CL => flags.set_all(&[
                MimgModifier::Pcf,
                MimgModifier::LodBias,
                MimgModifier::LodClamp,
            ]),
            Opcode::IMAGE_SAMPLE_C_LZ => flags.set_all(&[MimgModifier::Pcf, MimgModifier::Level0]),
            Opcode::IMAGE_SAMPLE_O => flags.set(MimgModifier::Offset),
            Opcode::IMAGE_SAMPLE_CL_O => {
                flags.set_all(&[MimgModifier::LodClamp, MimgModifier::Offset])
            }
            Opcode::IMAGE_SAMPLE_D_O => {
                flags.set_all(&[MimgModifier::Derivative, MimgModifier::Offset])
            }
            Opcode::IMAGE_SAMPLE_D_CL_O => flags.set_all(&[
                MimgModifier::Derivative,
                MimgModifier::LodClamp,
                MimgModifier::Offset,
            ]),
            Opcode::IMAGE_SAMPLE_L_O => flags.set_all(&[MimgModifier::Lod, MimgModifier::Offset]),
            Opcode::IMAGE_SAMPLE_B_O => {
                flags.set_all(&[MimgModifier::LodBias, MimgModifier::Offset])
            }
            Opcode::IMAGE_SAMPLE_B_CL_O => flags.set_all(&[
                MimgModifier::LodBias,
                MimgModifier::LodClamp,
                MimgModifier::Offset,
            ]),
            Opcode::IMAGE_SAMPLE_LZ_O => {
                flags.set_all(&[MimgModifier::Level0, MimgModifier::Offset])
            }
            Opcode::IMAGE_SAMPLE_C_O => flags.set_all(&[MimgModifier::Pcf, MimgModifier::Offset]),
            Opcode::IMAGE_SAMPLE_C_CL_O => flags.set_all(&[
                MimgModifier::Pcf,
                MimgModifier::LodClamp,
                MimgModifier::Offset,
            ]),
            Opcode::IMAGE_SAMPLE_C_D_O => flags.set_all(&[
                MimgModifier::Pcf,
                MimgModifier::Derivative,
                MimgModifier::Offset,
            ]),
            Opcode::IMAGE_SAMPLE_C_D_CL_O => flags.set_all(&[
                MimgModifier::Pcf,
                MimgModifier::Derivative,
                MimgModifier::LodClamp,
                MimgModifier::Offset,
            ]),
            Opcode::IMAGE_SAMPLE_C_L_O => {
                flags.set_all(&[MimgModifier::Pcf, MimgModifier::Lod, MimgModifier::Offset])
            }
            Opcode::IMAGE_SAMPLE_C_B_O => flags.set_all(&[
                MimgModifier::Pcf,
                MimgModifier::LodBias,
                MimgModifier::Offset,
            ]),
            Opcode::IMAGE_SAMPLE_C_B_CL_O => flags.set_all(&[
                MimgModifier::Pcf,
                MimgModifier::LodBias,
                MimgModifier::LodClamp,
                MimgModifier::Offset,
            ]),
            Opcode::IMAGE_SAMPLE_C_LZ_O => flags.set_all(&[
                MimgModifier::Pcf,
                MimgModifier::Level0,
                MimgModifier::Offset,
            ]),
            Opcode::IMAGE_GATHER4 => {}
            Opcode::IMAGE_GATHER4_CL => flags.set(MimgModifier::LodClamp),
            Opcode::IMAGE_GATHER4_L => flags.set(MimgModifier::Lod),
            Opcode::IMAGE_GATHER4_B => flags.set(MimgModifier::LodBias),
            Opcode::IMAGE_GATHER4_B_CL => {
                flags.set_all(&[MimgModifier::LodBias, MimgModifier::LodClamp])
            }
            Opcode::IMAGE_GATHER4_LZ => flags.set(MimgModifier::Level0),
            Opcode::IMAGE_GATHER4_C => flags.set(MimgModifier::Pcf),
            Opcode::IMAGE_GATHER4_C_CL => {
                flags.set_all(&[MimgModifier::Pcf, MimgModifier::LodClamp])
            }
            Opcode::IMAGE_GATHER4_C_L => flags.set_all(&[MimgModifier::Pcf, MimgModifier::Lod]),
            Opcode::IMAGE_GATHER4_C_B => flags.set_all(&[MimgModifier::Pcf, MimgModifier::LodBias]),
            Opcode::IMAGE_GATHER4_C_B_CL => flags.set_all(&[
                MimgModifier::Pcf,
                MimgModifier::LodBias,
                MimgModifier::LodClamp,
            ]),
            Opcode::IMAGE_GATHER4_C_LZ => flags.set_all(&[MimgModifier::Pcf, MimgModifier::Level0]),
            Opcode::IMAGE_GATHER4_O => flags.set(MimgModifier::Offset),
            Opcode::IMAGE_GATHER4_CL_O => {
                flags.set_all(&[MimgModifier::LodClamp, MimgModifier::Offset])
            }
            Opcode::IMAGE_GATHER4_L_O => flags.set_all(&[MimgModifier::Lod, MimgModifier::Offset]),
            Opcode::IMAGE_GATHER4_B_O => {
                flags.set_all(&[MimgModifier::LodBias, MimgModifier::Offset])
            }
            Opcode::IMAGE_GATHER4_B_CL_O => flags.set_all(&[
                MimgModifier::LodBias,
                MimgModifier::LodClamp,
                MimgModifier::Offset,
            ]),
            Opcode::IMAGE_GATHER4_LZ_O => {
                flags.set_all(&[MimgModifier::Level0, MimgModifier::Offset])
            }
            Opcode::IMAGE_GATHER4_C_O => flags.set_all(&[MimgModifier::Pcf, MimgModifier::Offset]),
            Opcode::IMAGE_GATHER4_C_CL_O => flags.set_all(&[
                MimgModifier::Pcf,
                MimgModifier::LodClamp,
                MimgModifier::Offset,
            ]),
            Opcode::IMAGE_GATHER4_C_L_O => {
                flags.set_all(&[MimgModifier::Pcf, MimgModifier::Lod, MimgModifier::Offset])
            }
            Opcode::IMAGE_GATHER4_C_B_O => flags.set_all(&[
                MimgModifier::Pcf,
                MimgModifier::LodBias,
                MimgModifier::Offset,
            ]),
            Opcode::IMAGE_GATHER4_C_B_CL_O => flags.set_all(&[
                MimgModifier::Pcf,
                MimgModifier::LodBias,
                MimgModifier::LodClamp,
                MimgModifier::Offset,
            ]),
            Opcode::IMAGE_GATHER4_C_LZ_O => flags.set_all(&[
                MimgModifier::Pcf,
                MimgModifier::Level0,
                MimgModifier::Offset,
            ]),
            Opcode::IMAGE_SAMPLE_CD => flags.set(MimgModifier::CoarseDerivative),
            Opcode::IMAGE_SAMPLE_CD_CL => {
                flags.set_all(&[MimgModifier::CoarseDerivative, MimgModifier::LodClamp])
            }
            Opcode::IMAGE_SAMPLE_C_CD => {
                flags.set_all(&[MimgModifier::Pcf, MimgModifier::CoarseDerivative])
            }
            Opcode::IMAGE_SAMPLE_C_CD_CL => flags.set_all(&[
                MimgModifier::Pcf,
                MimgModifier::CoarseDerivative,
                MimgModifier::LodClamp,
            ]),
            Opcode::IMAGE_SAMPLE_CD_O => {
                flags.set_all(&[MimgModifier::CoarseDerivative, MimgModifier::Offset])
            }
            Opcode::IMAGE_SAMPLE_CD_CL_O => flags.set_all(&[
                MimgModifier::CoarseDerivative,
                MimgModifier::LodClamp,
                MimgModifier::Offset,
            ]),
            Opcode::IMAGE_SAMPLE_C_CD_O => flags.set_all(&[
                MimgModifier::Pcf,
                MimgModifier::CoarseDerivative,
                MimgModifier::Offset,
            ]),
            Opcode::IMAGE_SAMPLE_C_CD_CL_O => flags.set_all(&[
                MimgModifier::Pcf,
                MimgModifier::CoarseDerivative,
                MimgModifier::LodClamp,
                MimgModifier::Offset,
            ]),
            _ => {}
        }

        flags.raw()
    }

    /// `decodeInstructionMIMG`.
    fn decode_instruction_mimg(&mut self, hex_instruction: u64) {
        let op = extract(hex_instruction, 24, 18) as u32;
        let vaddr = extract(hex_instruction, 39, 32) as u32;
        let vdata = extract(hex_instruction, 47, 40) as u32;
        let srsrc = extract(hex_instruction, 52, 48) as u32;
        let ssamp = extract(hex_instruction, 57, 53) as u32;

        self.instruction.opcode = Opcode(op + OpcodeMap::OP_MAP_MIMG.0);

        self.instruction.src[0].field = OperandField::VectorGPR.0;
        self.instruction.src[0].code = vaddr;
        self.instruction.src[2].field = OperandField::ScalarGPR.0;
        self.instruction.src[2].code = srsrc;
        self.instruction.src[3].field = OperandField::ScalarGPR.0;
        self.instruction.src[3].code = ssamp;
        self.instruction.dst[0].field = OperandField::VectorGPR.0;
        self.instruction.dst[0].code = vdata;

        let mut mimg = InstControl::from_raw(hex_instruction).mimg();
        mimg.set_modifiers(self.get_mimg_modifier(self.instruction.opcode) as u64);
        self.instruction.control = InstControl::from_raw(mimg.raw());
    }

    /// `decodeInstructionDS`.
    fn decode_instruction_ds(&mut self, hex_instruction: u64) {
        let op = extract(hex_instruction, 25, 18) as u32;
        let addr = extract(hex_instruction, 39, 32) as u32;
        let data0 = extract(hex_instruction, 47, 40) as u32;
        let data1 = extract(hex_instruction, 55, 48) as u32;
        let vdst = extract(hex_instruction, 63, 56) as u32;

        self.instruction.opcode = Opcode(op + OpcodeMap::OP_MAP_DS.0);

        self.instruction.src[0].field = OperandField::VectorGPR.0;
        self.instruction.src[0].code = addr;
        self.instruction.src[1].field = OperandField::VectorGPR.0;
        self.instruction.src[1].code = data0;
        self.instruction.src[2].field = OperandField::VectorGPR.0;
        self.instruction.src[2].code = data1;
        self.instruction.dst[0].field = OperandField::VectorGPR.0;
        self.instruction.dst[0].code = vdst;
        self.instruction.dst_count = 1;

        let mut ds = InstControl::from_raw(hex_instruction).ds();

        let inst_format = instruction_format(InstEncoding::DS, op);

        let opcode = OpcodeDS(op);
        ds.set_dual(
            opcode == OpcodeDS::DS_WRITE2_B32
                || opcode == OpcodeDS::DS_WRXCHG2_RTN_B32
                || opcode == OpcodeDS::DS_READ2_B32
                || opcode == OpcodeDS::DS_WRITE2_B64
                || opcode == OpcodeDS::DS_WRXCHG2_RTN_B64
                || opcode == OpcodeDS::DS_READ2_B64,
        );

        ds.set_sign(inst_format.src_type == ScalarType::Sint32);

        ds.set_relative(opcode >= OpcodeDS::DS_ADD_SRC2_U32 && opcode <= OpcodeDS::DS_MAX_SRC2_F64);

        ds.set_stride(
            opcode == OpcodeDS::DS_WRITE2ST64_B32
                || opcode == OpcodeDS::DS_WRXCHG2ST64_RTN_B32
                || opcode == OpcodeDS::DS_READ2ST64_B32
                || opcode == OpcodeDS::DS_WRITE2ST64_B64
                || opcode == OpcodeDS::DS_WRXCHG2ST64_RTN_B64
                || opcode == OpcodeDS::DS_READ2ST64_B64,
        );

        if opcode == OpcodeDS::DS_WRITE_B8
            || opcode == OpcodeDS::DS_READ_I8
            || opcode == OpcodeDS::DS_READ_U8
        {
            ds.set_size(1);
        } else if opcode == OpcodeDS::DS_WRITE_B16
            || opcode == OpcodeDS::DS_READ_I16
            || opcode == OpcodeDS::DS_READ_U16
        {
            ds.set_size(2);
        } else if inst_format.src_type == ScalarType::Sint32
            || inst_format.src_type == ScalarType::Uint32
        {
            ds.set_size(4);
        } else if inst_format.src_type == ScalarType::Sint64
            || inst_format.src_type == ScalarType::Uint64
        {
            ds.set_size(8);
        } else {
            ds.set_size(0);
        }
        self.instruction.control = InstControl::from_raw(ds.raw());
    }

    /// `decodeInstructionEXP`.
    fn decode_instruction_exp(&mut self, hex_instruction: u64) {
        let vsrc0 = extract(hex_instruction, 39, 32) as u32;
        let vsrc1 = extract(hex_instruction, 47, 40) as u32;
        let vsrc2 = extract(hex_instruction, 55, 48) as u32;
        let vsrc3 = extract(hex_instruction, 63, 56) as u32;

        self.instruction.opcode = Opcode::EXP;

        self.instruction.src[0].field = OperandField::VectorGPR.0;
        self.instruction.src[0].code = vsrc0;
        self.instruction.src[1].field = OperandField::VectorGPR.0;
        self.instruction.src[1].code = vsrc1;
        self.instruction.src[2].field = OperandField::VectorGPR.0;
        self.instruction.src[2].code = vsrc2;
        self.instruction.src[3].field = OperandField::VectorGPR.0;
        self.instruction.src[3].code = vsrc3;

        self.instruction.control = InstControl::from_raw(hex_instruction);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::frontend::instruction::{
        DppCtrl, SdwaDstUnused, SdwaSelector,
    };

    /// The encoding bits of an encoding, as a 32-bit token's top bits.
    fn encoding_bits(encoding: InstEncoding) -> u32 {
        encoding.0
    }

    /// Decodes one 32-bit instruction.
    fn decode32(words: &[u32]) -> GcnInst {
        let mut code = GcnCodeSlice::new(words);
        GcnDecodeContext::new().decode_instruction(&mut code)
    }

    /// Decodes one 64-bit instruction.
    fn decode64(low: u32, high: u32) -> GcnInst {
        let words = [low, high];
        let mut code = GcnCodeSlice::new(&words);
        GcnDecodeContext::new().decode_instruction(&mut code)
    }

    /// Splits a 64-bit word into the two the slice would hold.
    fn words_of(value: u64) -> (u32, u32) {
        (value as u32, (value >> 32) as u32)
    }

    #[test]
    fn the_encoding_comes_from_the_top_bits() {
        assert_eq!(
            get_instruction_encoding(encoding_bits(InstEncoding::SOP1)),
            InstEncoding::SOP1
        );
        assert_eq!(
            get_instruction_encoding(encoding_bits(InstEncoding::SOPP)),
            InstEncoding::SOPP
        );
        assert_eq!(
            get_instruction_encoding(encoding_bits(InstEncoding::SOPC)),
            InstEncoding::SOPC
        );
        assert_eq!(
            get_instruction_encoding(encoding_bits(InstEncoding::VOP1)),
            InstEncoding::VOP1
        );
        assert_eq!(
            get_instruction_encoding(encoding_bits(InstEncoding::VOPC)),
            InstEncoding::VOPC
        );
        for encoding in [
            InstEncoding::VOP3,
            InstEncoding::VOP3P,
            InstEncoding::EXP,
            InstEncoding::VINTRP,
            InstEncoding::DS,
            InstEncoding::MUBUF,
            InstEncoding::MTBUF,
            InstEncoding::MIMG,
            InstEncoding::SMRD,
            InstEncoding::SOPK,
            InstEncoding::SOP2,
        ] {
            assert_eq!(get_instruction_encoding(encoding_bits(encoding)), encoding);
        }
        // VOP2 is the fallback for a token whose top bit is clear.
        assert_eq!(get_instruction_encoding(0), InstEncoding::VOP2);
        // A token that matches no encoding is illegal.
        assert_eq!(get_instruction_encoding(0xD400_0000), InstEncoding::ILLEGAL);
        assert_eq!(get_instruction_encoding(0x0001_0000), InstEncoding::VOP2);
    }

    #[test]
    fn the_format_tables_are_indexed_by_encoding_opcode() {
        assert_eq!(
            instruction_format(InstEncoding::SOP2, OpcodeSOP2::S_ADD_U32.0),
            InstFormat {
                inst_class: InstClass::ScalarArith,
                inst_category: InstCategory::ScalarALU,
                src_count: 2,
                dst_count: 1,
                src_type: ScalarType::Uint32,
                dst_type: ScalarType::Uint32,
            }
        );
        // SOP1 index 3 is S_MOV_B32, which has one source.
        assert_eq!(
            instruction_format(InstEncoding::SOP1, OpcodeSOP1::S_MOV_B32.0),
            InstFormat {
                inst_class: InstClass::ScalarMov,
                inst_category: InstCategory::ScalarALU,
                src_count: 1,
                dst_count: 1,
                src_type: ScalarType::Uint32,
                dst_type: ScalarType::Uint32,
            }
        );
        // The holes in a table are empty entries: SOP2 has none between 11 and 14.
        assert_eq!(
            instruction_format(InstEncoding::SOP2, 12),
            InstFormat::EMPTY,
            "index 12 is a gap"
        );
        // A VOP3 entry three operands wide.
        let vop3 = instruction_format(
            InstEncoding::VOP3,
            Opcode::V_MAD_F32.0 - OpcodeMap::OP_MAP_VOP3.0,
        );
        assert_eq!(vop3.inst_class, InstClass::VectorFpArith32);
        assert_eq!(vop3.inst_category, InstCategory::VectorALU);
        assert_eq!(vop3.src_count, 3);
        assert_eq!(vop3.dst_count, 1);
        assert_eq!(vop3.src_type, ScalarType::Float32);
        assert_eq!(vop3.dst_type, ScalarType::Float32);
        // The last entry of the largest table is reachable.
        assert_ne!(
            instruction_format(InstEncoding::DS, 255),
            InstFormat::EMPTY,
            "the da table is full"
        );
        assert_eq!(INSTRUCTION_FORMAT_VOP3.len(), 883);
        assert_eq!(INSTRUCTION_FORMAT_DS.len(), 256);
        assert_eq!(INSTRUCTION_FORMAT_EXP.len(), 1);
    }

    #[test]
    fn an_sop1_instruction_decodes_its_two_scalars() {
        // s_mov_b32 s1, s2
        let token =
            encoding_bits(InstEncoding::SOP1) | (OpcodeSOP1::S_MOV_B32.0 << 8) | (1 << 16) | 2;
        let inst = decode32(&[token]);
        assert_eq!(inst.opcode, Opcode::S_MOV_B32);
        assert_eq!(inst.encoding, InstEncoding::SOP1);
        assert_eq!(inst.inst_class, InstClass::ScalarMov);
        assert_eq!(inst.category, InstCategory::ScalarALU);
        assert_eq!(inst.length, 4);
        assert_eq!(inst.src_count, 1);
        assert_eq!(inst.dst_count, 1);
        assert_eq!(inst.src[0].field, OperandField::ScalarGPR.0);
        assert_eq!(inst.src[0].code, 2);
        assert_eq!(inst.src[0].type_, ScalarType::Uint32);
        assert_eq!(inst.dst[0].field, OperandField::ScalarGPR.0);
        assert_eq!(inst.dst[0].code, 1);
        assert_eq!(inst.dst[0].type_, ScalarType::Uint32);

        // The operand is one byte wide, so a code in none of the ranges stays a plain code:
        // 120 sits between the scalar range and the positive integer constants.
        let token =
            encoding_bits(InstEncoding::SOP1) | (OpcodeSOP1::S_MOV_B32.0 << 8) | (1 << 16) | 120;
        let inst = decode32(&[token]);
        assert_eq!(inst.src[0].field, 120);
        assert_eq!(inst.src[0].code, 120);
    }

    #[test]
    fn an_sopp_branch_decodes_its_offset() {
        let token = encoding_bits(InstEncoding::SOPP) | (OpcodeSOPP::S_BRANCH.0 << 16) | 2;
        let inst = decode32(&[token]);
        assert_eq!(inst.opcode, Opcode::S_BRANCH);
        assert_eq!(inst.length, 4);
        assert_eq!(inst.category, InstCategory::FlowControl);
        assert_eq!(inst.control.sopp().simm, 2);
        assert_eq!(inst.branch_target(0x100), 0x10C);
        // A negative offset walks backwards, and the union holds it signed.
        let token = encoding_bits(InstEncoding::SOPP)
            | (OpcodeSOPP::S_BRANCH.0 << 16)
            | ((-3i16) as u16 as u32);
        let inst = decode32(&[token]);
        assert_eq!(inst.control.sopp().simm, -3);
        assert_eq!(inst.branch_target(0x100), 0xF8);
    }

    #[test]
    fn a_vop1_renumbers_its_vector_operand() {
        // v_mov_b32 v7, v3
        let token = encoding_bits(InstEncoding::VOP1)
            | (OperandFieldRange::VectorGPRMin as u32 + 3)
            | (OpcodeVOP1::V_MOV_B32.0 << 9)
            | (7 << 17);
        let inst = decode32(&[token]);
        assert_eq!(inst.opcode, Opcode::V_MOV_B32);
        assert_eq!(inst.length, 4);
        assert_eq!(inst.src[0].field, OperandField::VectorGPR.0);
        assert_eq!(inst.src[0].code, 3, "the register number, not the code");
        assert_eq!(inst.dst[0].field, OperandField::VectorGPR.0);
        assert_eq!(inst.dst[0].code, 7);
        assert_eq!(inst.src_count, 1);
        assert_eq!(inst.category, InstCategory::VectorALU);
    }

    #[test]
    fn a_vopc_writes_vcc_as_its_second_destination() {
        let token = encoding_bits(InstEncoding::VOPC)
            | (OperandFieldRange::VectorGPRMin as u32 + 1)
            | (2 << 9)
            | (OpcodeVOPC::V_CMP_F_F32.0 << 17);
        let inst = decode32(&[token]);
        assert_eq!(inst.opcode, Opcode::V_CMP_F_F32);
        assert_eq!(inst.src[0].code, 1);
        assert_eq!(inst.src[1].field, OperandField::VectorGPR.0);
        assert_eq!(inst.src[1].code, 2);
        assert_eq!(inst.dst[1].field, OperandField::VccLo.0);
        assert_eq!(inst.dst[1].code, OperandField::VccLo.0);
        assert_eq!(inst.dst[1].type_, ScalarType::Uint64);
    }

    #[test]
    fn a_vop3_instruction_decodes_its_modifiers() {
        // v_mad_f32 v5, -|v1|, v2, v3 with omod 1 and op_sel bits 0 and 3.
        let op = (Opcode::V_MAD_F32.0 - OpcodeMap::OP_MAP_VOP3.0) as u64;
        let word = encoding_bits(InstEncoding::VOP3) as u64
            | (op << 17)
            | 5
            | (257u64 << 32)
            | (258u64 << 41)
            | (259u64 << 50)
            | (1 << 8)
            | (1 << 12)
            | (1 << 15)
            | (1 << 59)
            | (1 << 61);
        let (low, high) = words_of(word);
        let inst = decode64(low, high);
        assert_eq!(inst.opcode, Opcode::V_MAD_F32);
        assert_eq!(inst.length, 8);
        assert_eq!(inst.inst_class, InstClass::VectorFpArith32);
        assert_eq!(inst.src_count, 3);
        // The C++ decides the destination count from the *encoding* opcode's range, which
        // spans V_ADD_I32 to V_DIV_SCALE_F64 and therefore covers V_MAD_F32 as well.
        assert!(
            !is_vop3b_encoding(inst.opcode),
            "but it is not one of the ten"
        );
        assert_eq!(inst.dst_count, 2);
        assert_eq!(inst.src[0].field, OperandField::VectorGPR.0);
        assert_eq!(inst.src[0].code, 1);
        assert_eq!(inst.src[1].code, 2);
        assert_eq!(inst.src[2].code, 3);
        assert!(inst.src[0].input_modifier.abs, "the abs bit is src0's");
        assert!(inst.src[0].input_modifier.neg, "the neg bit is src0's");
        assert!(!inst.src[1].input_modifier.neg);
        assert!(inst.src[0].op_sel.op_sel, "op_sel bit 0 belongs to src0");
        assert!(
            inst.dst[0].op_sel.op_sel,
            "op_sel bit 3 marks the destination"
        );
        assert_eq!(inst.dst[0].output_modifier.multiplier, 2.0, "omod 1");
        assert!(!inst.dst[0].output_modifier.clamp);
        assert_eq!(inst.dst[0].code, 5);

        // The whole word is kept as the control, so the fields read back.
        let control = inst.control.vop3();
        assert_eq!(control.abs(), 1);
        assert_eq!(control.op_sel(), 1 << 3 | 1);
        assert_eq!(control.omod(), 1);
        assert_eq!(control.neg(), 1);
    }

    #[test]
    fn a_vop3_compare_becomes_a_vopc_and_writes_the_scalar_destination() {
        let op = OpcodeVOP3::V_CMP_F_F32.0 as u64;
        let word = encoding_bits(InstEncoding::VOP3) as u64 | (op << 17) | 5 | (256u64 << 32);
        let (low, high) = words_of(word);
        let inst = decode64(low, high);
        assert_eq!(inst.opcode, Opcode::V_CMP_F_F32, "mapped onto VOPC");
        assert_eq!(inst.dst[1].field, OperandField::ScalarGPR.0);
        assert_eq!(inst.dst[1].code, 5);
        assert_eq!(inst.dst[1].type_, ScalarType::Uint64);
        assert_eq!(inst.dst_count, 1);
    }

    #[test]
    fn a_vop3b_form_keeps_its_scalar_destination() {
        // v_add_i32 is one of the ten VOP3B forms.
        let op = OpcodeVOP3::V_ADD_I32.0 as u64;
        let sdst = 9u64;
        let word = encoding_bits(InstEncoding::VOP3) as u64 | (op << 17) | 5 | (sdst << 8);
        let (low, high) = words_of(word);
        let inst = decode64(low, high);
        assert_eq!(inst.opcode, Opcode::V_ADD_I32);
        assert!(is_vop3b_encoding(inst.opcode));
        assert_eq!(inst.dst_count, 2);
        assert_eq!(inst.dst[1].field, OperandField::ScalarGPR.0);
        assert_eq!(inst.dst[1].code, 9);
        assert_eq!(inst.dst[1].type_, ScalarType::Uint64);
    }

    #[test]
    fn a_vop3p_instruction_decodes_both_selectors() {
        let op = OpcodeVOP3P::V_PK_ADD_F16.0 as u64;
        let word = encoding_bits(InstEncoding::VOP3P) as u64
            | (op << 16)
            | 4
            | (257u64 << 32)
            | (1 << 8)
            | (1 << 11)
            | (1 << 14)
            | (1 << 15)
            | (1 << 59)
            | (1 << 61);
        let (low, high) = words_of(word);
        let inst = decode64(low, high);
        assert_eq!(inst.opcode, Opcode::V_PK_ADD_F16);
        assert!(inst.src[0].input_modifier.neg);
        assert!(
            inst.src[0].input_modifier.neg_hi,
            "neg_hi covers the high halves"
        );
        assert!(inst.src[0].op_sel.op_sel);
        assert!(inst.src[0].op_sel.op_sel_hi);
        assert!(inst.dst[0].output_modifier.clamp);
        assert_eq!(inst.src[0].code, 1);
    }

    #[test]
    fn a_smrd_load_counts_its_dwords() {
        let token = encoding_bits(InstEncoding::SMRD)
            | (4 << 9)
            | (6 << 15)
            | (OpcodeSMRD::S_LOAD_DWORD.0 << 22);
        let inst = decode32(&[token]);
        assert_eq!(inst.opcode, Opcode::S_LOAD_DWORD);
        assert_eq!(inst.length, 4);
        assert_eq!(inst.dst_count, 1);
        assert_eq!(inst.src[0].field, OperandField::ScalarGPR.0);
        assert_eq!(inst.src[0].code, 4);
        assert_eq!(inst.dst[0].code, 6);
        assert_eq!(inst.control.smrd().count(), 1);
        assert!(!inst.control.smrd().imm());
        // With no immediate the offset word becomes the second source.
        assert_eq!(inst.src[1].field, OperandField::ScalarGPR.0);
        assert_eq!(inst.src[1].code, 0);
        assert_eq!(inst.src[1].type_, ScalarType::Uint32);

        // S_LOAD_DWORDX4 asks for four dwords.
        let token = encoding_bits(InstEncoding::SMRD) | (OpcodeSMRD::S_LOAD_DWORDX4.0 << 22);
        let inst = decode32(&[token]);
        assert_eq!(inst.opcode, Opcode::S_LOAD_DWORDX4);
        assert_eq!(inst.control.smrd().count(), 4);

        // An immediate offset leaves the second source undefined.
        let token = encoding_bits(InstEncoding::SMRD)
            | (1 << 8)
            | 0x40
            | (OpcodeSMRD::S_LOAD_DWORDX4.0 << 22);
        let inst = decode32(&[token]);
        assert!(inst.control.smrd().imm());
        assert_eq!(inst.control.smrd().offset(), 0x40);
        assert_eq!(inst.src[1].field, OperandField::Undefined.0);
    }

    #[test]
    fn a_mubuf_load_sets_its_transfer_size() {
        let word = encoding_bits(InstEncoding::MUBUF) as u64
            | ((OpcodeMUBUF::BUFFER_LOAD_DWORDX2.0 as u64) << 18)
            | (1u64 << 32)
            | (2u64 << 40)
            | (3u64 << 48)
            | (4u64 << 56);
        let (low, high) = words_of(word);
        let inst = decode64(low, high);
        assert_eq!(inst.opcode, Opcode::BUFFER_LOAD_DWORDX2);
        assert_eq!(inst.length, 8);
        assert_eq!(inst.control.mubuf().count(), 2);
        assert_eq!(inst.control.mubuf().size(), 8);
        assert_eq!(inst.src[0].field, OperandField::VectorGPR.0);
        assert_eq!(inst.src[0].code, 1, "vaddr");
        assert_eq!(inst.src[1].code, 2, "vdata");
        assert_eq!(inst.src[2].field, OperandField::ScalarGPR.0);
        assert_eq!(inst.src[2].code, 3, "srsrc");
        assert_eq!(inst.src[3].field, OperandField::ScalarGPR.0);
        assert_eq!(inst.src[3].code, 4, "soffset");

        // A format load takes one dword per enabled component.
        let word = encoding_bits(InstEncoding::MUBUF) as u64
            | ((OpcodeMUBUF::BUFFER_LOAD_FORMAT_XYZW.0 as u64) << 18);
        let (low, high) = words_of(word);
        let inst = decode64(low, high);
        assert_eq!(inst.opcode, Opcode::BUFFER_LOAD_FORMAT_XYZW);
        assert_eq!(inst.control.mubuf().count(), 4);
        assert_eq!(inst.control.mubuf().size(), 16);

        // A byte load is one byte, a short one two.
        for (opcode, size) in [
            (OpcodeMUBUF::BUFFER_LOAD_UBYTE, 1u64),
            (OpcodeMUBUF::BUFFER_LOAD_SSHORT, 2u64),
            (OpcodeMUBUF::BUFFER_STORE_BYTE, 1u64),
            (OpcodeMUBUF::BUFFER_STORE_SHORT, 2u64),
        ] {
            let word = encoding_bits(InstEncoding::MUBUF) as u64 | ((opcode.0 as u64) << 18);
            let (low, high) = words_of(word);
            let inst = decode64(low, high);
            assert_eq!(inst.control.mubuf().count(), 1);
            assert_eq!(inst.control.mubuf().size(), size, "{opcode:?}");
        }
    }

    #[test]
    fn a_mtbuf_load_counts_its_components() {
        let word = encoding_bits(InstEncoding::MTBUF) as u64
            | ((OpcodeMTBUF::TBUFFER_LOAD_FORMAT_XY.0 as u64) << 16)
            | (7u64 << 19)
            | (4u64 << 23);
        let (low, high) = words_of(word);
        let inst = decode64(low, high);
        assert_eq!(inst.opcode, Opcode::TBUFFER_LOAD_FORMAT_XY);
        assert_eq!(inst.control.mtbuf().count(), 2);
        assert_eq!(inst.control.mtbuf().dfmt(), 7);
        assert_eq!(inst.control.mtbuf().nfmt(), 4);
    }

    #[test]
    fn a_ds_instruction_sets_its_flags_from_the_opcode() {
        let word = encoding_bits(InstEncoding::DS) as u64
            | ((OpcodeDS::DS_WRITE2_B32.0 as u64) << 18)
            | (5u64 << 32)
            | (6u64 << 40)
            | (7u64 << 48)
            | (8u64 << 56);
        let (low, high) = words_of(word);
        let inst = decode64(low, high);
        assert_eq!(inst.opcode, Opcode::DS_WRITE2_B32);
        assert_eq!(inst.length, 8);
        assert_eq!(inst.dst_count, 1);
        assert!(inst.control.ds().dual(), "a two-address write");
        assert!(!inst.control.ds().sign());
        assert!(!inst.control.ds().relative());
        assert!(!inst.control.ds().stride());
        assert_eq!(inst.control.ds().size(), 4, "a 32-bit source");
        assert_eq!(inst.src[0].code, 5);
        assert_eq!(inst.src[1].code, 6);
        assert_eq!(inst.src[2].code, 7);
        assert_eq!(inst.dst[0].code, 8);

        // The narrow forms take their size from the opcode.
        for (opcode, size) in [
            (OpcodeDS::DS_WRITE_B8, 1u64),
            (OpcodeDS::DS_READ_I16, 2u64),
            (OpcodeDS::DS_READ_U16, 2u64),
        ] {
            let word = encoding_bits(InstEncoding::DS) as u64 | ((opcode.0 as u64) << 18);
            let (low, high) = words_of(word);
            let inst = decode64(low, high);
            assert_eq!(inst.control.ds().size(), size, "{opcode:?}");
            assert!(!inst.control.ds().dual());
        }

        // The strided two-address forms set the stride flag.
        let word =
            encoding_bits(InstEncoding::DS) as u64 | ((OpcodeDS::DS_WRITE2ST64_B32.0 as u64) << 18);
        let (low, high) = words_of(word);
        let inst = decode64(low, high);
        assert!(inst.control.ds().stride());
        assert!(
            !inst.control.ds().dual(),
            "the strided forms are not in the dual set the C++ lists"
        );
    }

    #[test]
    fn an_exp_writes_its_export_control() {
        let word = encoding_bits(InstEncoding::EXP) as u64
            | (1u64 << 32)
            | (2u64 << 40)
            | (3u64 << 48)
            | (4u64 << 56)
            | 0o17
            | (4 << 4)
            | (1 << 11);
        let (low, high) = words_of(word);
        let inst = decode64(low, high);
        assert_eq!(inst.opcode, Opcode::EXP);
        assert_eq!(inst.category, InstCategory::Export);
        assert_eq!(inst.inst_class, InstClass::Exp);
        assert_eq!(inst.src_count, 4);
        assert_eq!(inst.src[0].code, 1);
        assert_eq!(inst.src[1].code, 2);
        assert_eq!(inst.src[2].code, 3);
        assert_eq!(inst.src[3].code, 4);
        assert_eq!(inst.control.exp().en(), 0o17);
        assert_eq!(inst.control.exp().target(), 4);
        assert!(inst.control.exp().done());
        assert!(!inst.control.exp().compr());
        assert_eq!(inst.src[0].type_, ScalarType::Float32);
    }

    #[test]
    fn an_mimg_load_carries_its_modifiers() {
        let word = encoding_bits(InstEncoding::MIMG) as u64
            | ((OpcodeMIMG::IMAGE_SAMPLE_CL.0 as u64) << 18)
            | (1u64 << 32)
            | (2u64 << 40)
            | (3u64 << 48)
            | (4u64 << 53);
        let (low, high) = words_of(word);
        let inst = decode64(low, high);
        assert_eq!(inst.opcode, Opcode::IMAGE_SAMPLE_CL);
        assert_eq!(inst.src[0].code, 1, "vaddr");
        assert_eq!(inst.src[2].field, OperandField::ScalarGPR.0);
        assert_eq!(inst.src[2].code, 3, "srsrc");
        assert_eq!(inst.src[3].code, 4, "ssamp");
        assert_eq!(inst.dst[0].code, 2, "vdata");
        assert_eq!(
            inst.control.mimg().modifiers(),
            1u64 << MimgModifier::LodClamp.0,
            "IMAGE_SAMPLE_CL asks for a LOD clamp"
        );

        // An image instruction with no modifier leaves the field clear.
        let word =
            encoding_bits(InstEncoding::MIMG) as u64 | ((OpcodeMIMG::IMAGE_SAMPLE.0 as u64) << 18);
        let (low, high) = words_of(word);
        let inst = decode64(low, high);
        assert_eq!(inst.opcode, Opcode::IMAGE_SAMPLE);
        assert_eq!(inst.control.mimg().modifiers(), 0);

        // The sample with an offset example carries one bit.
        let word = encoding_bits(InstEncoding::MIMG) as u64
            | ((OpcodeMIMG::IMAGE_GATHER4_CL_O.0 as u64) << 18);
        let (low, high) = words_of(word);
        let inst = decode64(low, high);
        let expected = (1 << MimgModifier::LodClamp.0) | (1 << MimgModifier::Offset.0);
        assert_eq!(inst.control.mimg().modifiers(), expected as u64);
    }

    #[test]
    fn a_vintrp_decodes_its_channel_and_attribute() {
        let token = encoding_bits(InstEncoding::VINTRP)
            | 9
            | ((OpcodeVINTRP::V_INTERP_P1_F32.0) << 16)
            | (11 << 18)
            | (3 << 8);
        let inst = decode32(&[token]);
        assert_eq!(inst.opcode, Opcode::V_INTERP_P1_F32);
        assert_eq!(inst.src[0].field, OperandField::VectorGPR.0);
        assert_eq!(inst.src[0].code, 9);
        assert_eq!(inst.dst[0].code, 11);
        assert_eq!(inst.control.vintrp().chan(), 3);
    }

    #[test]
    fn a_dpp_operation_is_decoded_from_the_second_word() {
        // v_mov_b32 with a DPP source: the selector is in src0's operand field.
        let token = encoding_bits(InstEncoding::VOP1)
            | OperandField::Dpp.0
            | (OpcodeVOP1::V_MOV_B32.0 << 9)
            | (7 << 17);
        let dpp_word = 9u32 | (0x101 << 8) | (1 << 19) | (0o12 << 28) | (0o14 << 24);
        let inst = decode32(&[token, dpp_word]);
        assert_eq!(
            inst.length, 8,
            "the control word is part of the instruction"
        );
        assert_eq!(inst.src[0].field, OperandField::VectorGPR.0);
        assert_eq!(inst.src[0].code, 9);
        let op = inst.src[0].dpp.expect("the DPP operation is attached");
        assert_eq!(op.op, DppCtrl::DppRowShl);
        assert_eq!(op.value, 1);
        assert!(op.bc);
        assert_eq!(op.row_mask, 0o12);
        assert_eq!(op.bank_mask, 0o14);
    }

    #[test]
    fn an_sdwa_word_is_unpacked_into_the_operands() {
        // A VOP2 with the SDWA selector in src0's field: the s0 and s1 bits say whether each
        // source is a vector register or a scalar one.
        let token = encoding_bits(InstEncoding::VOP2)
            | OperandField::Sdwa.0
            | (OpcodeVOP2::V_MUL_F32.0 << 25)
            | (7 << 9)
            | (9 << 17);
        let sdwa_word = 5u32
            | (3 << 8)
            | (2 << 11)
            | (1 << 13)
            | (1 << 14)
            | (6 << 16)
            | (1 << 19)
            | (1 << 20)
            | (1 << 21)
            | (1 << 24)
            | (1 << 28)
            | (1 << 29);
        let inst = decode32(&[token, sdwa_word]);
        assert_eq!(inst.opcode, Opcode::V_MUL_F32);
        assert_eq!(inst.length, 8);
        assert_eq!(inst.src[0].field, OperandField::VectorGPR.0);
        assert_eq!(inst.src[0].code, 5);
        assert_eq!(inst.src[0].sdwa_sel, SdwaSelector::Dword);
        assert!(inst.src[0].input_modifier.neg);
        assert!(inst.src[0].input_modifier.abs);
        assert!(inst.src[0].input_modifier.sext);
        assert_eq!(inst.src[1].field, OperandField::VectorGPR.0);
        assert_eq!(inst.src[1].code, 7, "vsrc1 as VOP2 decoded it");
        assert_eq!(inst.src[1].sdwa_sel, SdwaSelector::Byte1);
        assert!(inst.src[1].input_modifier.neg);
        assert!(inst.src[1].input_modifier.abs);
        assert_eq!(inst.dst[0].sdwa_sel, SdwaSelector::Byte3);
        assert_eq!(inst.dst[0].sdwa_dst, SdwaDstUnused::Preserve);
        assert!(inst.dst[0].output_modifier.clamp);
        assert_eq!(inst.dst[0].output_modifier.multiplier, 2.0, "omod 1");

        // A scalar source takes its field from the code the SDWA word carries.
        let sdwa_word = sdwa_word | (1 << 23);
        let inst = decode32(&[token, sdwa_word]);
        assert_eq!(inst.src[0].field, OperandField::ScalarGPR.0, "s0 is set");
        assert_eq!(inst.src[0].code, 5);
    }

    #[test]
    fn a_literal_constant_comes_from_the_word_after_the_instruction() {
        // s_mov_b32 s1, 0xDEADBEEF: the literal operand field makes the next word a constant.
        let token = encoding_bits(InstEncoding::SOP1)
            | (OpcodeSOP1::S_MOV_B32.0 << 8)
            | (1 << 16)
            | OperandField::LiteralConst.0;
        let inst = decode32(&[token, 0xDEAD_BEEF]);
        assert_eq!(inst.length, 8);
        assert_eq!(inst.src[0].field, OperandField::LiteralConst.0);
        assert_eq!(inst.src[0].code, 0xDEAD_BEEF);
        assert_eq!(inst.src[0].type_, ScalarType::Uint32);
        assert_eq!(inst.src_count, 1);

        // s_setreg_imm32_b32 carries its literal as an extra source.
        let token = encoding_bits(InstEncoding::SOPK)
            | (OpcodeSOPK::S_SETREG_IMM32_B32.0 << 23)
            | (1 << 16);
        assert!(has_additional_literal(
            InstEncoding::SOPK,
            Opcode::S_SETREG_IMM32_B32
        ));
        let table = instruction_format(InstEncoding::SOPK, OpcodeSOPK::S_SETREG_IMM32_B32.0);
        let inst = decode32(&[token, 0x1234_5678]);
        assert_eq!(inst.opcode, Opcode::S_SETREG_IMM32_B32);
        assert_eq!(inst.length, 8);
        assert_eq!(inst.src_count, table.src_count + 1);
        assert_eq!(
            inst.src[table.src_count as usize].field,
            OperandField::LiteralConst.0
        );
        assert_eq!(inst.src[table.src_count as usize].code, 0x1234_5678);

        // An instruction with no literal stays one word long.
        let token =
            encoding_bits(InstEncoding::SOP1) | (OpcodeSOP1::S_MOV_B32.0 << 8) | (1 << 16) | 2;
        let inst = decode32(&[token]);
        assert_eq!(inst.length, 4);
    }

    #[test]
    fn the_fetch_shader_slice_reads_both_widths() {
        let words = [0x1111_1111u32, 0x2222_2222, 0x3333_3333];
        let mut code = GcnCodeSlice::new(&words);
        assert_eq!(code.position(), 0);
        assert_eq!(code.at(0), 0x1111_1111);
        assert_eq!(code.at(2), 0x3333_3333);
        assert!(!code.at_end());
        assert_eq!(code.read_u32(), 0x1111_1111);
        assert_eq!(code.position(), 1);
        assert_eq!(code.read_u64(), 0x3333_3333_2222_2222);
        assert_eq!(code.position(), 3);
        assert!(code.at_end());
    }

    #[test]
    #[should_panic(expected = "Instruction format table incomplete")]
    fn an_opcode_missing_from_the_tables_is_reported() {
        // SOP2 has a gap at index 12, which the C++ asserts on.
        let token = encoding_bits(InstEncoding::SOP2) | (12 << 23);
        let inst = decode32(&[token]);
        let _ = inst;
    }

    #[test]
    #[should_panic(expected = "illegal encoding")]
    fn an_illegal_encoding_is_reported() {
        let inst = decode32(&[0xD400_0000]);
        let _ = inst;
    }
}
