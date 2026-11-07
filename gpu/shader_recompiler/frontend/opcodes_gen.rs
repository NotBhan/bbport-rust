//! GCN enumerations, generated from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/opcodes.h` by `tools/gen_gcn_opcodes.py`.
//! The header's 34 `enum class ... : u32` tables appear here with the same names, the same
//! enumerators in the same order and the same values.
//!
//! The C++ uses an `enum class`, which allows two enumerators to share a value. A Rust
//! `enum` does not, and three of these tables rely on that (`OpcodeVOP3` and `OpcodeVOPC`
//! alias several instruction encodings, and the unified `Opcode` inherits those aliases),
//! so the port models every table as a `u32` newtype with associated constants. Reading the
//! value is `.0`, and the constants work in `match` patterns.

// The standalone expressions below are the C++'s own: `0 + OP_MAP_SOP2` and
// `0x00000000u << 31` are how the header spells the first entries of a table.
#![allow(non_upper_case_globals)]
#![allow(clippy::identity_op)]

/// `Shader::Gcn::OpcodeSOP2`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeSOP2(pub u32);

impl OpcodeSOP2 {
    pub const S_ADD_U32: OpcodeSOP2 = OpcodeSOP2(0);
    pub const S_SUB_U32: OpcodeSOP2 = OpcodeSOP2(1);
    pub const S_ADD_I32: OpcodeSOP2 = OpcodeSOP2(2);
    pub const S_SUB_I32: OpcodeSOP2 = OpcodeSOP2(3);
    pub const S_ADDC_U32: OpcodeSOP2 = OpcodeSOP2(4);
    pub const S_SUBB_U32: OpcodeSOP2 = OpcodeSOP2(5);
    pub const S_MIN_I32: OpcodeSOP2 = OpcodeSOP2(6);
    pub const S_MIN_U32: OpcodeSOP2 = OpcodeSOP2(7);
    pub const S_MAX_I32: OpcodeSOP2 = OpcodeSOP2(8);
    pub const S_MAX_U32: OpcodeSOP2 = OpcodeSOP2(9);
    pub const S_CSELECT_B32: OpcodeSOP2 = OpcodeSOP2(10);
    pub const S_CSELECT_B64: OpcodeSOP2 = OpcodeSOP2(11);
    pub const S_AND_B32: OpcodeSOP2 = OpcodeSOP2(14);
    pub const S_AND_B64: OpcodeSOP2 = OpcodeSOP2(15);
    pub const S_OR_B32: OpcodeSOP2 = OpcodeSOP2(16);
    pub const S_OR_B64: OpcodeSOP2 = OpcodeSOP2(17);
    pub const S_XOR_B32: OpcodeSOP2 = OpcodeSOP2(18);
    pub const S_XOR_B64: OpcodeSOP2 = OpcodeSOP2(19);
    pub const S_ANDN2_B32: OpcodeSOP2 = OpcodeSOP2(20);
    pub const S_ANDN2_B64: OpcodeSOP2 = OpcodeSOP2(21);
    pub const S_ORN2_B32: OpcodeSOP2 = OpcodeSOP2(22);
    pub const S_ORN2_B64: OpcodeSOP2 = OpcodeSOP2(23);
    pub const S_NAND_B32: OpcodeSOP2 = OpcodeSOP2(24);
    pub const S_NAND_B64: OpcodeSOP2 = OpcodeSOP2(25);
    pub const S_NOR_B32: OpcodeSOP2 = OpcodeSOP2(26);
    pub const S_NOR_B64: OpcodeSOP2 = OpcodeSOP2(27);
    pub const S_XNOR_B32: OpcodeSOP2 = OpcodeSOP2(28);
    pub const S_XNOR_B64: OpcodeSOP2 = OpcodeSOP2(29);
    pub const S_LSHL_B32: OpcodeSOP2 = OpcodeSOP2(30);
    pub const S_LSHL_B64: OpcodeSOP2 = OpcodeSOP2(31);
    pub const S_LSHR_B32: OpcodeSOP2 = OpcodeSOP2(32);
    pub const S_LSHR_B64: OpcodeSOP2 = OpcodeSOP2(33);
    pub const S_ASHR_I32: OpcodeSOP2 = OpcodeSOP2(34);
    pub const S_ASHR_I64: OpcodeSOP2 = OpcodeSOP2(35);
    pub const S_BFM_B32: OpcodeSOP2 = OpcodeSOP2(36);
    pub const S_BFM_B64: OpcodeSOP2 = OpcodeSOP2(37);
    pub const S_MUL_I32: OpcodeSOP2 = OpcodeSOP2(38);
    pub const S_BFE_U32: OpcodeSOP2 = OpcodeSOP2(39);
    pub const S_BFE_I32: OpcodeSOP2 = OpcodeSOP2(40);
    pub const S_BFE_U64: OpcodeSOP2 = OpcodeSOP2(41);
    pub const S_BFE_I64: OpcodeSOP2 = OpcodeSOP2(42);
    pub const S_CBRANCH_G_FORK: OpcodeSOP2 = OpcodeSOP2(43);
    pub const S_ABSDIFF_I32: OpcodeSOP2 = OpcodeSOP2(44);
    pub const OP_RANGE_SOP2: OpcodeSOP2 = OpcodeSOP2(OpcodeSOP2::S_ABSDIFF_I32.0 + 1);
}

/// `Shader::Gcn::OpcodeSOPK`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeSOPK(pub u32);

impl OpcodeSOPK {
    pub const S_MOVK_I32: OpcodeSOPK = OpcodeSOPK(0);
    pub const S_CMOVK_I32: OpcodeSOPK = OpcodeSOPK(2);
    pub const S_CMPK_EQ_I32: OpcodeSOPK = OpcodeSOPK(3);
    pub const S_CMPK_LG_I32: OpcodeSOPK = OpcodeSOPK(4);
    pub const S_CMPK_GT_I32: OpcodeSOPK = OpcodeSOPK(5);
    pub const S_CMPK_GE_I32: OpcodeSOPK = OpcodeSOPK(6);
    pub const S_CMPK_LT_I32: OpcodeSOPK = OpcodeSOPK(7);
    pub const S_CMPK_LE_I32: OpcodeSOPK = OpcodeSOPK(8);
    pub const S_CMPK_EQ_U32: OpcodeSOPK = OpcodeSOPK(9);
    pub const S_CMPK_LG_U32: OpcodeSOPK = OpcodeSOPK(10);
    pub const S_CMPK_GT_U32: OpcodeSOPK = OpcodeSOPK(11);
    pub const S_CMPK_GE_U32: OpcodeSOPK = OpcodeSOPK(12);
    pub const S_CMPK_LT_U32: OpcodeSOPK = OpcodeSOPK(13);
    pub const S_CMPK_LE_U32: OpcodeSOPK = OpcodeSOPK(14);
    pub const S_ADDK_I32: OpcodeSOPK = OpcodeSOPK(15);
    pub const S_MULK_I32: OpcodeSOPK = OpcodeSOPK(16);
    pub const S_CBRANCH_I_FORK: OpcodeSOPK = OpcodeSOPK(17);
    pub const S_GETREG_B32: OpcodeSOPK = OpcodeSOPK(18);
    pub const S_SETREG_B32: OpcodeSOPK = OpcodeSOPK(19);
    pub const S_GETREG_REGRD_B32: OpcodeSOPK = OpcodeSOPK(20);
    pub const S_SETREG_IMM32_B32: OpcodeSOPK = OpcodeSOPK(21);
    pub const OP_RANGE_SOPK: OpcodeSOPK = OpcodeSOPK(OpcodeSOPK::S_SETREG_IMM32_B32.0 + 1);
}

/// `Shader::Gcn::OpcodeSOP1`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeSOP1(pub u32);

impl OpcodeSOP1 {
    pub const S_MOV_B32: OpcodeSOP1 = OpcodeSOP1(3);
    pub const S_MOV_B64: OpcodeSOP1 = OpcodeSOP1(4);
    pub const S_CMOV_B32: OpcodeSOP1 = OpcodeSOP1(5);
    pub const S_CMOV_B64: OpcodeSOP1 = OpcodeSOP1(6);
    pub const S_NOT_B32: OpcodeSOP1 = OpcodeSOP1(7);
    pub const S_NOT_B64: OpcodeSOP1 = OpcodeSOP1(8);
    pub const S_WQM_B32: OpcodeSOP1 = OpcodeSOP1(9);
    pub const S_WQM_B64: OpcodeSOP1 = OpcodeSOP1(10);
    pub const S_BREV_B32: OpcodeSOP1 = OpcodeSOP1(11);
    pub const S_BREV_B64: OpcodeSOP1 = OpcodeSOP1(12);
    pub const S_BCNT0_I32_B32: OpcodeSOP1 = OpcodeSOP1(13);
    pub const S_BCNT0_I32_B64: OpcodeSOP1 = OpcodeSOP1(14);
    pub const S_BCNT1_I32_B32: OpcodeSOP1 = OpcodeSOP1(15);
    pub const S_BCNT1_I32_B64: OpcodeSOP1 = OpcodeSOP1(16);
    pub const S_FF0_I32_B32: OpcodeSOP1 = OpcodeSOP1(17);
    pub const S_FF0_I32_B64: OpcodeSOP1 = OpcodeSOP1(18);
    pub const S_FF1_I32_B32: OpcodeSOP1 = OpcodeSOP1(19);
    pub const S_FF1_I32_B64: OpcodeSOP1 = OpcodeSOP1(20);
    pub const S_FLBIT_I32_B32: OpcodeSOP1 = OpcodeSOP1(21);
    pub const S_FLBIT_I32_B64: OpcodeSOP1 = OpcodeSOP1(22);
    pub const S_FLBIT_I32: OpcodeSOP1 = OpcodeSOP1(23);
    pub const S_FLBIT_I32_I64: OpcodeSOP1 = OpcodeSOP1(24);
    pub const S_SEXT_I32_I8: OpcodeSOP1 = OpcodeSOP1(25);
    pub const S_SEXT_I32_I16: OpcodeSOP1 = OpcodeSOP1(26);
    pub const S_BITSET0_B32: OpcodeSOP1 = OpcodeSOP1(27);
    pub const S_BITSET0_B64: OpcodeSOP1 = OpcodeSOP1(28);
    pub const S_BITSET1_B32: OpcodeSOP1 = OpcodeSOP1(29);
    pub const S_BITSET1_B64: OpcodeSOP1 = OpcodeSOP1(30);
    pub const S_GETPC_B64: OpcodeSOP1 = OpcodeSOP1(31);
    pub const S_SETPC_B64: OpcodeSOP1 = OpcodeSOP1(32);
    pub const S_SWAPPC_B64: OpcodeSOP1 = OpcodeSOP1(33);
    pub const S_RFE_B64: OpcodeSOP1 = OpcodeSOP1(34);
    pub const S_AND_SAVEEXEC_B64: OpcodeSOP1 = OpcodeSOP1(36);
    pub const S_OR_SAVEEXEC_B64: OpcodeSOP1 = OpcodeSOP1(37);
    pub const S_XOR_SAVEEXEC_B64: OpcodeSOP1 = OpcodeSOP1(38);
    pub const S_ANDN2_SAVEEXEC_B64: OpcodeSOP1 = OpcodeSOP1(39);
    pub const S_ORN2_SAVEEXEC_B64: OpcodeSOP1 = OpcodeSOP1(40);
    pub const S_NAND_SAVEEXEC_B64: OpcodeSOP1 = OpcodeSOP1(41);
    pub const S_NOR_SAVEEXEC_B64: OpcodeSOP1 = OpcodeSOP1(42);
    pub const S_XNOR_SAVEEXEC_B64: OpcodeSOP1 = OpcodeSOP1(43);
    pub const S_QUADMASK_B32: OpcodeSOP1 = OpcodeSOP1(44);
    pub const S_QUADMASK_B64: OpcodeSOP1 = OpcodeSOP1(45);
    pub const S_MOVRELS_B32: OpcodeSOP1 = OpcodeSOP1(46);
    pub const S_MOVRELS_B64: OpcodeSOP1 = OpcodeSOP1(47);
    pub const S_MOVRELD_B32: OpcodeSOP1 = OpcodeSOP1(48);
    pub const S_MOVRELD_B64: OpcodeSOP1 = OpcodeSOP1(49);
    pub const S_CBRANCH_JOIN: OpcodeSOP1 = OpcodeSOP1(50);
    pub const S_MOV_REGRD_B32: OpcodeSOP1 = OpcodeSOP1(51);
    pub const S_ABS_I32: OpcodeSOP1 = OpcodeSOP1(52);
    pub const S_MOV_FED_B32: OpcodeSOP1 = OpcodeSOP1(53);
    pub const OP_RANGE_SOP1: OpcodeSOP1 = OpcodeSOP1(OpcodeSOP1::S_MOV_FED_B32.0 + 1);
}

/// `Shader::Gcn::OpcodeSOPC`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeSOPC(pub u32);

impl OpcodeSOPC {
    pub const S_CMP_EQ_I32: OpcodeSOPC = OpcodeSOPC(0);
    pub const S_CMP_LG_I32: OpcodeSOPC = OpcodeSOPC(1);
    pub const S_CMP_GT_I32: OpcodeSOPC = OpcodeSOPC(2);
    pub const S_CMP_GE_I32: OpcodeSOPC = OpcodeSOPC(3);
    pub const S_CMP_LT_I32: OpcodeSOPC = OpcodeSOPC(4);
    pub const S_CMP_LE_I32: OpcodeSOPC = OpcodeSOPC(5);
    pub const S_CMP_EQ_U32: OpcodeSOPC = OpcodeSOPC(6);
    pub const S_CMP_LG_U32: OpcodeSOPC = OpcodeSOPC(7);
    pub const S_CMP_GT_U32: OpcodeSOPC = OpcodeSOPC(8);
    pub const S_CMP_GE_U32: OpcodeSOPC = OpcodeSOPC(9);
    pub const S_CMP_LT_U32: OpcodeSOPC = OpcodeSOPC(10);
    pub const S_CMP_LE_U32: OpcodeSOPC = OpcodeSOPC(11);
    pub const S_BITCMP0_B32: OpcodeSOPC = OpcodeSOPC(12);
    pub const S_BITCMP1_B32: OpcodeSOPC = OpcodeSOPC(13);
    pub const S_BITCMP0_B64: OpcodeSOPC = OpcodeSOPC(14);
    pub const S_BITCMP1_B64: OpcodeSOPC = OpcodeSOPC(15);
    pub const S_SETVSKIP: OpcodeSOPC = OpcodeSOPC(16);
    pub const OP_RANGE_SOPC: OpcodeSOPC = OpcodeSOPC(OpcodeSOPC::S_SETVSKIP.0 + 1);
}

/// `Shader::Gcn::OpcodeSOPP`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeSOPP(pub u32);

impl OpcodeSOPP {
    pub const S_NOP: OpcodeSOPP = OpcodeSOPP(0);
    pub const S_ENDPGM: OpcodeSOPP = OpcodeSOPP(1);
    pub const S_BRANCH: OpcodeSOPP = OpcodeSOPP(2);
    pub const S_CBRANCH_SCC0: OpcodeSOPP = OpcodeSOPP(4);
    pub const S_CBRANCH_SCC1: OpcodeSOPP = OpcodeSOPP(5);
    pub const S_CBRANCH_VCCZ: OpcodeSOPP = OpcodeSOPP(6);
    pub const S_CBRANCH_VCCNZ: OpcodeSOPP = OpcodeSOPP(7);
    pub const S_CBRANCH_EXECZ: OpcodeSOPP = OpcodeSOPP(8);
    pub const S_CBRANCH_EXECNZ: OpcodeSOPP = OpcodeSOPP(9);
    pub const S_BARRIER: OpcodeSOPP = OpcodeSOPP(10);
    pub const S_SETKILL: OpcodeSOPP = OpcodeSOPP(11);
    pub const S_WAITCNT: OpcodeSOPP = OpcodeSOPP(12);
    pub const S_SETHALT: OpcodeSOPP = OpcodeSOPP(13);
    pub const S_SLEEP: OpcodeSOPP = OpcodeSOPP(14);
    pub const S_SETPRIO: OpcodeSOPP = OpcodeSOPP(15);
    pub const S_SENDMSG: OpcodeSOPP = OpcodeSOPP(16);
    pub const S_SENDMSGHALT: OpcodeSOPP = OpcodeSOPP(17);
    pub const S_TRAP: OpcodeSOPP = OpcodeSOPP(18);
    pub const S_ICACHE_INV: OpcodeSOPP = OpcodeSOPP(19);
    pub const S_INCPERFLEVEL: OpcodeSOPP = OpcodeSOPP(20);
    pub const S_DECPERFLEVEL: OpcodeSOPP = OpcodeSOPP(21);
    pub const S_TTRACEDATA: OpcodeSOPP = OpcodeSOPP(22);
    pub const S_CBRANCH_CDBGSYS: OpcodeSOPP = OpcodeSOPP(23);
    pub const S_CBRANCH_CDBGUSER: OpcodeSOPP = OpcodeSOPP(24);
    pub const S_CBRANCH_CDBGSYS_OR_USER: OpcodeSOPP = OpcodeSOPP(25);
    pub const S_CBRANCH_CDBGSYS_AND_USER: OpcodeSOPP = OpcodeSOPP(26);
    pub const OP_RANGE_SOPP: OpcodeSOPP = OpcodeSOPP(OpcodeSOPP::S_CBRANCH_CDBGSYS_AND_USER.0 + 1);
}

/// `Shader::Gcn::OpcodeSMRD`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeSMRD(pub u32);

impl OpcodeSMRD {
    pub const S_LOAD_DWORD: OpcodeSMRD = OpcodeSMRD(0);
    pub const S_LOAD_DWORDX2: OpcodeSMRD = OpcodeSMRD(1);
    pub const S_LOAD_DWORDX4: OpcodeSMRD = OpcodeSMRD(2);
    pub const S_LOAD_DWORDX8: OpcodeSMRD = OpcodeSMRD(3);
    pub const S_LOAD_DWORDX16: OpcodeSMRD = OpcodeSMRD(4);
    pub const S_BUFFER_LOAD_DWORD: OpcodeSMRD = OpcodeSMRD(8);
    pub const S_BUFFER_LOAD_DWORDX2: OpcodeSMRD = OpcodeSMRD(9);
    pub const S_BUFFER_LOAD_DWORDX4: OpcodeSMRD = OpcodeSMRD(10);
    pub const S_BUFFER_LOAD_DWORDX8: OpcodeSMRD = OpcodeSMRD(11);
    pub const S_BUFFER_LOAD_DWORDX16: OpcodeSMRD = OpcodeSMRD(12);
    pub const S_DCACHE_INV_VOL: OpcodeSMRD = OpcodeSMRD(29);
    pub const S_MEMTIME: OpcodeSMRD = OpcodeSMRD(30);
    pub const S_DCACHE_INV: OpcodeSMRD = OpcodeSMRD(31);
    pub const OP_RANGE_SMRD: OpcodeSMRD = OpcodeSMRD(OpcodeSMRD::S_DCACHE_INV.0 + 1);
}

/// `Shader::Gcn::OpcodeVOP2`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeVOP2(pub u32);

impl OpcodeVOP2 {
    pub const V_CNDMASK_B32: OpcodeVOP2 = OpcodeVOP2(0);
    pub const V_READLANE_B32: OpcodeVOP2 = OpcodeVOP2(1);
    pub const V_WRITELANE_B32: OpcodeVOP2 = OpcodeVOP2(2);
    pub const V_ADD_F32: OpcodeVOP2 = OpcodeVOP2(3);
    pub const V_SUB_F32: OpcodeVOP2 = OpcodeVOP2(4);
    pub const V_SUBREV_F32: OpcodeVOP2 = OpcodeVOP2(5);
    pub const V_MAC_LEGACY_F32: OpcodeVOP2 = OpcodeVOP2(6);
    pub const V_MUL_LEGACY_F32: OpcodeVOP2 = OpcodeVOP2(7);
    pub const V_MUL_F32: OpcodeVOP2 = OpcodeVOP2(8);
    pub const V_MUL_I32_I24: OpcodeVOP2 = OpcodeVOP2(9);
    pub const V_MUL_HI_I32_I24: OpcodeVOP2 = OpcodeVOP2(10);
    pub const V_MUL_U32_U24: OpcodeVOP2 = OpcodeVOP2(11);
    pub const V_MUL_HI_U32_U24: OpcodeVOP2 = OpcodeVOP2(12);
    pub const V_MIN_LEGACY_F32: OpcodeVOP2 = OpcodeVOP2(13);
    pub const V_MAX_LEGACY_F32: OpcodeVOP2 = OpcodeVOP2(14);
    pub const V_MIN_F32: OpcodeVOP2 = OpcodeVOP2(15);
    pub const V_MAX_F32: OpcodeVOP2 = OpcodeVOP2(16);
    pub const V_MIN_I32: OpcodeVOP2 = OpcodeVOP2(17);
    pub const V_MAX_I32: OpcodeVOP2 = OpcodeVOP2(18);
    pub const V_MIN_U32: OpcodeVOP2 = OpcodeVOP2(19);
    pub const V_MAX_U32: OpcodeVOP2 = OpcodeVOP2(20);
    pub const V_LSHR_B32: OpcodeVOP2 = OpcodeVOP2(21);
    pub const V_LSHRREV_B32: OpcodeVOP2 = OpcodeVOP2(22);
    pub const V_ASHR_I32: OpcodeVOP2 = OpcodeVOP2(23);
    pub const V_ASHRREV_I32: OpcodeVOP2 = OpcodeVOP2(24);
    pub const V_LSHL_B32: OpcodeVOP2 = OpcodeVOP2(25);
    pub const V_LSHLREV_B32: OpcodeVOP2 = OpcodeVOP2(26);
    pub const V_AND_B32: OpcodeVOP2 = OpcodeVOP2(27);
    pub const V_OR_B32: OpcodeVOP2 = OpcodeVOP2(28);
    pub const V_XOR_B32: OpcodeVOP2 = OpcodeVOP2(29);
    pub const V_BFM_B32: OpcodeVOP2 = OpcodeVOP2(30);
    pub const V_MAC_F32: OpcodeVOP2 = OpcodeVOP2(31);
    pub const V_MADMK_F32: OpcodeVOP2 = OpcodeVOP2(32);
    pub const V_MADAK_F32: OpcodeVOP2 = OpcodeVOP2(33);
    pub const V_BCNT_U32_B32: OpcodeVOP2 = OpcodeVOP2(34);
    pub const V_MBCNT_LO_U32_B32: OpcodeVOP2 = OpcodeVOP2(35);
    pub const V_MBCNT_HI_U32_B32: OpcodeVOP2 = OpcodeVOP2(36);
    pub const V_ADD_I32: OpcodeVOP2 = OpcodeVOP2(37);
    pub const V_SUB_I32: OpcodeVOP2 = OpcodeVOP2(38);
    pub const V_SUBREV_I32: OpcodeVOP2 = OpcodeVOP2(39);
    pub const V_ADDC_U32: OpcodeVOP2 = OpcodeVOP2(40);
    pub const V_SUBB_U32: OpcodeVOP2 = OpcodeVOP2(41);
    pub const V_SUBBREV_U32: OpcodeVOP2 = OpcodeVOP2(42);
    pub const V_LDEXP_F32: OpcodeVOP2 = OpcodeVOP2(43);
    pub const V_CVT_PKACCUM_U8_F32: OpcodeVOP2 = OpcodeVOP2(44);
    pub const V_CVT_PKNORM_I16_F32: OpcodeVOP2 = OpcodeVOP2(45);
    pub const V_CVT_PKNORM_U16_F32: OpcodeVOP2 = OpcodeVOP2(46);
    pub const V_CVT_PKRTZ_F16_F32: OpcodeVOP2 = OpcodeVOP2(47);
    pub const V_CVT_PK_U16_U32: OpcodeVOP2 = OpcodeVOP2(48);
    pub const V_CVT_PK_I16_I32: OpcodeVOP2 = OpcodeVOP2(49);
    pub const V_ADD_F16: OpcodeVOP2 = OpcodeVOP2(50);
    pub const V_SUB_F16: OpcodeVOP2 = OpcodeVOP2(51);
    pub const V_SUBREV_F16: OpcodeVOP2 = OpcodeVOP2(52);
    pub const V_MUL_F16: OpcodeVOP2 = OpcodeVOP2(53);
    pub const V_MAX_F16: OpcodeVOP2 = OpcodeVOP2(57);
    pub const V_MIN_F16: OpcodeVOP2 = OpcodeVOP2(58);
    pub const V_LDEXP_F16: OpcodeVOP2 = OpcodeVOP2(59);
    pub const OP_RANGE_VOP2: OpcodeVOP2 = OpcodeVOP2(OpcodeVOP2::V_LDEXP_F16.0 + 1);
}

/// `Shader::Gcn::OpcodeVOP3`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeVOP3(pub u32);

impl OpcodeVOP3 {
    pub const V_CMP_F_F32: OpcodeVOP3 = OpcodeVOP3(0);
    pub const V_CMP_LT_F32: OpcodeVOP3 = OpcodeVOP3(1);
    pub const V_CMP_EQ_F32: OpcodeVOP3 = OpcodeVOP3(2);
    pub const V_CMP_LE_F32: OpcodeVOP3 = OpcodeVOP3(3);
    pub const V_CMP_GT_F32: OpcodeVOP3 = OpcodeVOP3(4);
    pub const V_CMP_LG_F32: OpcodeVOP3 = OpcodeVOP3(5);
    pub const V_CMP_GE_F32: OpcodeVOP3 = OpcodeVOP3(6);
    pub const V_CMP_O_F32: OpcodeVOP3 = OpcodeVOP3(7);
    pub const V_CMP_U_F32: OpcodeVOP3 = OpcodeVOP3(8);
    pub const V_CMP_NGE_F32: OpcodeVOP3 = OpcodeVOP3(9);
    pub const V_CMP_NLG_F32: OpcodeVOP3 = OpcodeVOP3(10);
    pub const V_CMP_NGT_F32: OpcodeVOP3 = OpcodeVOP3(11);
    pub const V_CMP_NLE_F32: OpcodeVOP3 = OpcodeVOP3(12);
    pub const V_CMP_NEQ_F32: OpcodeVOP3 = OpcodeVOP3(13);
    pub const V_CMP_NLT_F32: OpcodeVOP3 = OpcodeVOP3(14);
    pub const V_CMP_TRU_F32: OpcodeVOP3 = OpcodeVOP3(15);
    pub const V_CMP_T_F32: OpcodeVOP3 = OpcodeVOP3(15);
    pub const V_CMPX_F_F32: OpcodeVOP3 = OpcodeVOP3(16);
    pub const V_CMPX_LT_F32: OpcodeVOP3 = OpcodeVOP3(17);
    pub const V_CMPX_EQ_F32: OpcodeVOP3 = OpcodeVOP3(18);
    pub const V_CMPX_LE_F32: OpcodeVOP3 = OpcodeVOP3(19);
    pub const V_CMPX_GT_F32: OpcodeVOP3 = OpcodeVOP3(20);
    pub const V_CMPX_LG_F32: OpcodeVOP3 = OpcodeVOP3(21);
    pub const V_CMPX_GE_F32: OpcodeVOP3 = OpcodeVOP3(22);
    pub const V_CMPX_O_F32: OpcodeVOP3 = OpcodeVOP3(23);
    pub const V_CMPX_U_F32: OpcodeVOP3 = OpcodeVOP3(24);
    pub const V_CMPX_NGE_F32: OpcodeVOP3 = OpcodeVOP3(25);
    pub const V_CMPX_NLG_F32: OpcodeVOP3 = OpcodeVOP3(26);
    pub const V_CMPX_NGT_F32: OpcodeVOP3 = OpcodeVOP3(27);
    pub const V_CMPX_NLE_F32: OpcodeVOP3 = OpcodeVOP3(28);
    pub const V_CMPX_NEQ_F32: OpcodeVOP3 = OpcodeVOP3(29);
    pub const V_CMPX_NLT_F32: OpcodeVOP3 = OpcodeVOP3(30);
    pub const V_CMPX_TRU_F32: OpcodeVOP3 = OpcodeVOP3(31);
    pub const V_CMPX_T_F32: OpcodeVOP3 = OpcodeVOP3(31);
    pub const V_CMP_F_F64: OpcodeVOP3 = OpcodeVOP3(32);
    pub const V_CMP_LT_F64: OpcodeVOP3 = OpcodeVOP3(33);
    pub const V_CMP_EQ_F64: OpcodeVOP3 = OpcodeVOP3(34);
    pub const V_CMP_LE_F64: OpcodeVOP3 = OpcodeVOP3(35);
    pub const V_CMP_GT_F64: OpcodeVOP3 = OpcodeVOP3(36);
    pub const V_CMP_LG_F64: OpcodeVOP3 = OpcodeVOP3(37);
    pub const V_CMP_GE_F64: OpcodeVOP3 = OpcodeVOP3(38);
    pub const V_CMP_O_F64: OpcodeVOP3 = OpcodeVOP3(39);
    pub const V_CMP_U_F64: OpcodeVOP3 = OpcodeVOP3(40);
    pub const V_CMP_NGE_F64: OpcodeVOP3 = OpcodeVOP3(41);
    pub const V_CMP_NLG_F64: OpcodeVOP3 = OpcodeVOP3(42);
    pub const V_CMP_NGT_F64: OpcodeVOP3 = OpcodeVOP3(43);
    pub const V_CMP_NLE_F64: OpcodeVOP3 = OpcodeVOP3(44);
    pub const V_CMP_NEQ_F64: OpcodeVOP3 = OpcodeVOP3(45);
    pub const V_CMP_NLT_F64: OpcodeVOP3 = OpcodeVOP3(46);
    pub const V_CMP_TRU_F64: OpcodeVOP3 = OpcodeVOP3(47);
    pub const V_CMP_T_F64: OpcodeVOP3 = OpcodeVOP3(47);
    pub const V_CMPX_F_F64: OpcodeVOP3 = OpcodeVOP3(48);
    pub const V_CMPX_LT_F64: OpcodeVOP3 = OpcodeVOP3(49);
    pub const V_CMPX_EQ_F64: OpcodeVOP3 = OpcodeVOP3(50);
    pub const V_CMPX_LE_F64: OpcodeVOP3 = OpcodeVOP3(51);
    pub const V_CMPX_GT_F64: OpcodeVOP3 = OpcodeVOP3(52);
    pub const V_CMPX_LG_F64: OpcodeVOP3 = OpcodeVOP3(53);
    pub const V_CMPX_GE_F64: OpcodeVOP3 = OpcodeVOP3(54);
    pub const V_CMPX_O_F64: OpcodeVOP3 = OpcodeVOP3(55);
    pub const V_CMPX_U_F64: OpcodeVOP3 = OpcodeVOP3(56);
    pub const V_CMPX_NGE_F64: OpcodeVOP3 = OpcodeVOP3(57);
    pub const V_CMPX_NLG_F64: OpcodeVOP3 = OpcodeVOP3(58);
    pub const V_CMPX_NGT_F64: OpcodeVOP3 = OpcodeVOP3(59);
    pub const V_CMPX_NLE_F64: OpcodeVOP3 = OpcodeVOP3(60);
    pub const V_CMPX_NEQ_F64: OpcodeVOP3 = OpcodeVOP3(61);
    pub const V_CMPX_NLT_F64: OpcodeVOP3 = OpcodeVOP3(62);
    pub const V_CMPX_TRU_F64: OpcodeVOP3 = OpcodeVOP3(63);
    pub const V_CMPX_T_F64: OpcodeVOP3 = OpcodeVOP3(63);
    pub const V_CMPS_F_F32: OpcodeVOP3 = OpcodeVOP3(64);
    pub const V_CMPS_LT_F32: OpcodeVOP3 = OpcodeVOP3(65);
    pub const V_CMPS_EQ_F32: OpcodeVOP3 = OpcodeVOP3(66);
    pub const V_CMPS_LE_F32: OpcodeVOP3 = OpcodeVOP3(67);
    pub const V_CMPS_GT_F32: OpcodeVOP3 = OpcodeVOP3(68);
    pub const V_CMPS_LG_F32: OpcodeVOP3 = OpcodeVOP3(69);
    pub const V_CMPS_GE_F32: OpcodeVOP3 = OpcodeVOP3(70);
    pub const V_CMPS_O_F32: OpcodeVOP3 = OpcodeVOP3(71);
    pub const V_CMPS_U_F32: OpcodeVOP3 = OpcodeVOP3(72);
    pub const V_CMPS_NGE_F32: OpcodeVOP3 = OpcodeVOP3(73);
    pub const V_CMPS_NLG_F32: OpcodeVOP3 = OpcodeVOP3(74);
    pub const V_CMPS_NGT_F32: OpcodeVOP3 = OpcodeVOP3(75);
    pub const V_CMPS_NLE_F32: OpcodeVOP3 = OpcodeVOP3(76);
    pub const V_CMPS_NEQ_F32: OpcodeVOP3 = OpcodeVOP3(77);
    pub const V_CMPS_NLT_F32: OpcodeVOP3 = OpcodeVOP3(78);
    pub const V_CMPS_TRU_F32: OpcodeVOP3 = OpcodeVOP3(79);
    pub const V_CMPS_T_F32: OpcodeVOP3 = OpcodeVOP3(79);
    pub const V_CMPSX_F_F32: OpcodeVOP3 = OpcodeVOP3(80);
    pub const V_CMPSX_LT_F32: OpcodeVOP3 = OpcodeVOP3(81);
    pub const V_CMPSX_EQ_F32: OpcodeVOP3 = OpcodeVOP3(82);
    pub const V_CMPSX_LE_F32: OpcodeVOP3 = OpcodeVOP3(83);
    pub const V_CMPSX_GT_F32: OpcodeVOP3 = OpcodeVOP3(84);
    pub const V_CMPSX_LG_F32: OpcodeVOP3 = OpcodeVOP3(85);
    pub const V_CMPSX_GE_F32: OpcodeVOP3 = OpcodeVOP3(86);
    pub const V_CMPSX_O_F32: OpcodeVOP3 = OpcodeVOP3(87);
    pub const V_CMPSX_U_F32: OpcodeVOP3 = OpcodeVOP3(88);
    pub const V_CMPSX_NGE_F32: OpcodeVOP3 = OpcodeVOP3(89);
    pub const V_CMPSX_NLG_F32: OpcodeVOP3 = OpcodeVOP3(90);
    pub const V_CMPSX_NGT_F32: OpcodeVOP3 = OpcodeVOP3(91);
    pub const V_CMPSX_NLE_F32: OpcodeVOP3 = OpcodeVOP3(92);
    pub const V_CMPSX_NEQ_F32: OpcodeVOP3 = OpcodeVOP3(93);
    pub const V_CMPSX_NLT_F32: OpcodeVOP3 = OpcodeVOP3(94);
    pub const V_CMPSX_TRU_F32: OpcodeVOP3 = OpcodeVOP3(95);
    pub const V_CMPSX_T_F32: OpcodeVOP3 = OpcodeVOP3(95);
    pub const V_CMPS_F_F64: OpcodeVOP3 = OpcodeVOP3(96);
    pub const V_CMPS_LT_F64: OpcodeVOP3 = OpcodeVOP3(97);
    pub const V_CMPS_EQ_F64: OpcodeVOP3 = OpcodeVOP3(98);
    pub const V_CMPS_LE_F64: OpcodeVOP3 = OpcodeVOP3(99);
    pub const V_CMPS_GT_F64: OpcodeVOP3 = OpcodeVOP3(100);
    pub const V_CMPS_LG_F64: OpcodeVOP3 = OpcodeVOP3(101);
    pub const V_CMPS_GE_F64: OpcodeVOP3 = OpcodeVOP3(102);
    pub const V_CMPS_O_F64: OpcodeVOP3 = OpcodeVOP3(103);
    pub const V_CMPS_U_F64: OpcodeVOP3 = OpcodeVOP3(104);
    pub const V_CMPS_NGE_F64: OpcodeVOP3 = OpcodeVOP3(105);
    pub const V_CMPS_NLG_F64: OpcodeVOP3 = OpcodeVOP3(106);
    pub const V_CMPS_NGT_F64: OpcodeVOP3 = OpcodeVOP3(107);
    pub const V_CMPS_NLE_F64: OpcodeVOP3 = OpcodeVOP3(108);
    pub const V_CMPS_NEQ_F64: OpcodeVOP3 = OpcodeVOP3(109);
    pub const V_CMPS_NLT_F64: OpcodeVOP3 = OpcodeVOP3(110);
    pub const V_CMPS_TRU_F64: OpcodeVOP3 = OpcodeVOP3(111);
    pub const V_CMPS_T_F64: OpcodeVOP3 = OpcodeVOP3(111);
    pub const V_CMPSX_F_F64: OpcodeVOP3 = OpcodeVOP3(112);
    pub const V_CMPSX_LT_F64: OpcodeVOP3 = OpcodeVOP3(113);
    pub const V_CMPSX_EQ_F64: OpcodeVOP3 = OpcodeVOP3(114);
    pub const V_CMPSX_LE_F64: OpcodeVOP3 = OpcodeVOP3(115);
    pub const V_CMPSX_GT_F64: OpcodeVOP3 = OpcodeVOP3(116);
    pub const V_CMPSX_LG_F64: OpcodeVOP3 = OpcodeVOP3(117);
    pub const V_CMPSX_GE_F64: OpcodeVOP3 = OpcodeVOP3(118);
    pub const V_CMPSX_O_F64: OpcodeVOP3 = OpcodeVOP3(119);
    pub const V_CMPSX_U_F64: OpcodeVOP3 = OpcodeVOP3(120);
    pub const V_CMPSX_NGE_F64: OpcodeVOP3 = OpcodeVOP3(121);
    pub const V_CMPSX_NLG_F64: OpcodeVOP3 = OpcodeVOP3(122);
    pub const V_CMPSX_NGT_F64: OpcodeVOP3 = OpcodeVOP3(123);
    pub const V_CMPSX_NLE_F64: OpcodeVOP3 = OpcodeVOP3(124);
    pub const V_CMPSX_NEQ_F64: OpcodeVOP3 = OpcodeVOP3(125);
    pub const V_CMPSX_NLT_F64: OpcodeVOP3 = OpcodeVOP3(126);
    pub const V_CMPSX_TRU_F64: OpcodeVOP3 = OpcodeVOP3(127);
    pub const V_CMPSX_T_F64: OpcodeVOP3 = OpcodeVOP3(127);
    pub const V_CMP_F_I32: OpcodeVOP3 = OpcodeVOP3(128);
    pub const V_CMP_LT_I32: OpcodeVOP3 = OpcodeVOP3(129);
    pub const V_CMP_EQ_I32: OpcodeVOP3 = OpcodeVOP3(130);
    pub const V_CMP_LE_I32: OpcodeVOP3 = OpcodeVOP3(131);
    pub const V_CMP_GT_I32: OpcodeVOP3 = OpcodeVOP3(132);
    pub const V_CMP_LG_I32: OpcodeVOP3 = OpcodeVOP3(133);
    pub const V_CMP_NE_I32: OpcodeVOP3 = OpcodeVOP3(133);
    pub const V_CMP_GE_I32: OpcodeVOP3 = OpcodeVOP3(134);
    pub const V_CMP_TRU_I32: OpcodeVOP3 = OpcodeVOP3(135);
    pub const V_CMP_T_I32: OpcodeVOP3 = OpcodeVOP3(135);
    pub const V_CMP_CLASS_F32: OpcodeVOP3 = OpcodeVOP3(136);
    pub const V_CMP_LT_I16: OpcodeVOP3 = OpcodeVOP3(137);
    pub const V_CMP_EQ_I16: OpcodeVOP3 = OpcodeVOP3(138);
    pub const V_CMP_LE_I16: OpcodeVOP3 = OpcodeVOP3(139);
    pub const V_CMP_GT_I16: OpcodeVOP3 = OpcodeVOP3(140);
    pub const V_CMP_NE_I16: OpcodeVOP3 = OpcodeVOP3(141);
    pub const V_CMP_GE_I16: OpcodeVOP3 = OpcodeVOP3(142);
    pub const V_CMPX_F_I32: OpcodeVOP3 = OpcodeVOP3(144);
    pub const V_CMPX_LT_I32: OpcodeVOP3 = OpcodeVOP3(145);
    pub const V_CMPX_EQ_I32: OpcodeVOP3 = OpcodeVOP3(146);
    pub const V_CMPX_LE_I32: OpcodeVOP3 = OpcodeVOP3(147);
    pub const V_CMPX_GT_I32: OpcodeVOP3 = OpcodeVOP3(148);
    pub const V_CMPX_LG_I32: OpcodeVOP3 = OpcodeVOP3(149);
    pub const V_CMPX_NE_I32: OpcodeVOP3 = OpcodeVOP3(149);
    pub const V_CMPX_GE_I32: OpcodeVOP3 = OpcodeVOP3(150);
    pub const V_CMPX_TRU_I32: OpcodeVOP3 = OpcodeVOP3(151);
    pub const V_CMPX_T_I32: OpcodeVOP3 = OpcodeVOP3(151);
    pub const V_CMPX_CLASS_F32: OpcodeVOP3 = OpcodeVOP3(152);
    pub const V_CMP_F_I64: OpcodeVOP3 = OpcodeVOP3(160);
    pub const V_CMP_LT_I64: OpcodeVOP3 = OpcodeVOP3(161);
    pub const V_CMP_EQ_I64: OpcodeVOP3 = OpcodeVOP3(162);
    pub const V_CMP_LE_I64: OpcodeVOP3 = OpcodeVOP3(163);
    pub const V_CMP_GT_I64: OpcodeVOP3 = OpcodeVOP3(164);
    pub const V_CMP_LG_I64: OpcodeVOP3 = OpcodeVOP3(165);
    pub const V_CMP_NE_I64: OpcodeVOP3 = OpcodeVOP3(165);
    pub const V_CMP_GE_I64: OpcodeVOP3 = OpcodeVOP3(166);
    pub const V_CMP_TRU_I64: OpcodeVOP3 = OpcodeVOP3(167);
    pub const V_CMP_T_I64: OpcodeVOP3 = OpcodeVOP3(167);
    pub const V_CMP_CLASS_F64: OpcodeVOP3 = OpcodeVOP3(168);
    pub const V_CMP_LT_U16: OpcodeVOP3 = OpcodeVOP3(169);
    pub const V_CMP_EQ_U16: OpcodeVOP3 = OpcodeVOP3(170);
    pub const V_CMP_LE_U16: OpcodeVOP3 = OpcodeVOP3(171);
    pub const V_CMP_GT_U16: OpcodeVOP3 = OpcodeVOP3(172);
    pub const V_CMP_NE_U16: OpcodeVOP3 = OpcodeVOP3(173);
    pub const V_CMP_GE_U16: OpcodeVOP3 = OpcodeVOP3(174);
    pub const V_CMPX_F_I64: OpcodeVOP3 = OpcodeVOP3(176);
    pub const V_CMPX_LT_I64: OpcodeVOP3 = OpcodeVOP3(177);
    pub const V_CMPX_EQ_I64: OpcodeVOP3 = OpcodeVOP3(178);
    pub const V_CMPX_LE_I64: OpcodeVOP3 = OpcodeVOP3(179);
    pub const V_CMPX_GT_I64: OpcodeVOP3 = OpcodeVOP3(180);
    pub const V_CMPX_LG_I64: OpcodeVOP3 = OpcodeVOP3(181);
    pub const V_CMPX_NE_I64: OpcodeVOP3 = OpcodeVOP3(181);
    pub const V_CMPX_GE_I64: OpcodeVOP3 = OpcodeVOP3(182);
    pub const V_CMPX_TRU_I64: OpcodeVOP3 = OpcodeVOP3(183);
    pub const V_CMPX_T_I64: OpcodeVOP3 = OpcodeVOP3(183);
    pub const V_CMPX_CLASS_F64: OpcodeVOP3 = OpcodeVOP3(184);
    pub const V_CMP_F_U32: OpcodeVOP3 = OpcodeVOP3(192);
    pub const V_CMP_LT_U32: OpcodeVOP3 = OpcodeVOP3(193);
    pub const V_CMP_EQ_U32: OpcodeVOP3 = OpcodeVOP3(194);
    pub const V_CMP_LE_U32: OpcodeVOP3 = OpcodeVOP3(195);
    pub const V_CMP_GT_U32: OpcodeVOP3 = OpcodeVOP3(196);
    pub const V_CMP_LG_U32: OpcodeVOP3 = OpcodeVOP3(197);
    pub const V_CMP_NE_U32: OpcodeVOP3 = OpcodeVOP3(197);
    pub const V_CMP_GE_U32: OpcodeVOP3 = OpcodeVOP3(198);
    pub const V_CMP_TRU_U32: OpcodeVOP3 = OpcodeVOP3(199);
    pub const V_CMP_T_U32: OpcodeVOP3 = OpcodeVOP3(199);
    pub const V_CMP_F_F16: OpcodeVOP3 = OpcodeVOP3(200);
    pub const V_CMP_LT_F16: OpcodeVOP3 = OpcodeVOP3(201);
    pub const V_CMP_EQ_F16: OpcodeVOP3 = OpcodeVOP3(202);
    pub const V_CMP_LE_F16: OpcodeVOP3 = OpcodeVOP3(203);
    pub const V_CMP_GT_F16: OpcodeVOP3 = OpcodeVOP3(204);
    pub const V_CMP_LG_F16: OpcodeVOP3 = OpcodeVOP3(205);
    pub const V_CMP_GE_F16: OpcodeVOP3 = OpcodeVOP3(206);
    pub const V_CMP_O_F16: OpcodeVOP3 = OpcodeVOP3(207);
    pub const V_CMPX_F_U32: OpcodeVOP3 = OpcodeVOP3(208);
    pub const V_CMPX_LT_U32: OpcodeVOP3 = OpcodeVOP3(209);
    pub const V_CMPX_EQ_U32: OpcodeVOP3 = OpcodeVOP3(210);
    pub const V_CMPX_LE_U32: OpcodeVOP3 = OpcodeVOP3(211);
    pub const V_CMPX_GT_U32: OpcodeVOP3 = OpcodeVOP3(212);
    pub const V_CMPX_LG_U32: OpcodeVOP3 = OpcodeVOP3(213);
    pub const V_CMPX_NE_U32: OpcodeVOP3 = OpcodeVOP3(213);
    pub const V_CMPX_GE_U32: OpcodeVOP3 = OpcodeVOP3(214);
    pub const V_CMPX_TRU_U32: OpcodeVOP3 = OpcodeVOP3(215);
    pub const V_CMPX_T_U32: OpcodeVOP3 = OpcodeVOP3(215);
    pub const V_CMP_F_U64: OpcodeVOP3 = OpcodeVOP3(224);
    pub const V_CMP_LT_U64: OpcodeVOP3 = OpcodeVOP3(225);
    pub const V_CMP_EQ_U64: OpcodeVOP3 = OpcodeVOP3(226);
    pub const V_CMP_LE_U64: OpcodeVOP3 = OpcodeVOP3(227);
    pub const V_CMP_GT_U64: OpcodeVOP3 = OpcodeVOP3(228);
    pub const V_CMP_LG_U64: OpcodeVOP3 = OpcodeVOP3(229);
    pub const V_CMP_NE_U64: OpcodeVOP3 = OpcodeVOP3(229);
    pub const V_CMP_GE_U64: OpcodeVOP3 = OpcodeVOP3(230);
    pub const V_CMP_TRU_U64: OpcodeVOP3 = OpcodeVOP3(231);
    pub const V_CMP_T_U64: OpcodeVOP3 = OpcodeVOP3(231);
    pub const V_CMPX_F_U64: OpcodeVOP3 = OpcodeVOP3(240);
    pub const V_CMPX_LT_U64: OpcodeVOP3 = OpcodeVOP3(241);
    pub const V_CMPX_EQ_U64: OpcodeVOP3 = OpcodeVOP3(242);
    pub const V_CMPX_LE_U64: OpcodeVOP3 = OpcodeVOP3(243);
    pub const V_CMPX_GT_U64: OpcodeVOP3 = OpcodeVOP3(244);
    pub const V_CMPX_LG_U64: OpcodeVOP3 = OpcodeVOP3(245);
    pub const V_CMPX_NE_U64: OpcodeVOP3 = OpcodeVOP3(245);
    pub const V_CMPX_GE_U64: OpcodeVOP3 = OpcodeVOP3(246);
    pub const V_CMPX_TRU_U64: OpcodeVOP3 = OpcodeVOP3(247);
    pub const V_CMPX_T_U64: OpcodeVOP3 = OpcodeVOP3(247);
    pub const V_CNDMASK_B32: OpcodeVOP3 = OpcodeVOP3(256);
    pub const V_READLANE_B32: OpcodeVOP3 = OpcodeVOP3(257);
    pub const V_WRITELANE_B32: OpcodeVOP3 = OpcodeVOP3(258);
    pub const V_ADD_F32: OpcodeVOP3 = OpcodeVOP3(259);
    pub const V_SUB_F32: OpcodeVOP3 = OpcodeVOP3(260);
    pub const V_SUBREV_F32: OpcodeVOP3 = OpcodeVOP3(261);
    pub const V_MAC_LEGACY_F32: OpcodeVOP3 = OpcodeVOP3(262);
    pub const V_MUL_LEGACY_F32: OpcodeVOP3 = OpcodeVOP3(263);
    pub const V_MUL_F32: OpcodeVOP3 = OpcodeVOP3(264);
    pub const V_MUL_I32_I24: OpcodeVOP3 = OpcodeVOP3(265);
    pub const V_MUL_HI_I32_I24: OpcodeVOP3 = OpcodeVOP3(266);
    pub const V_MUL_U32_U24: OpcodeVOP3 = OpcodeVOP3(267);
    pub const V_MUL_HI_U32_U24: OpcodeVOP3 = OpcodeVOP3(268);
    pub const V_MIN_LEGACY_F32: OpcodeVOP3 = OpcodeVOP3(269);
    pub const V_MAX_LEGACY_F32: OpcodeVOP3 = OpcodeVOP3(270);
    pub const V_MIN_F32: OpcodeVOP3 = OpcodeVOP3(271);
    pub const V_MAX_F32: OpcodeVOP3 = OpcodeVOP3(272);
    pub const V_MIN_I32: OpcodeVOP3 = OpcodeVOP3(273);
    pub const V_MAX_I32: OpcodeVOP3 = OpcodeVOP3(274);
    pub const V_MIN_U32: OpcodeVOP3 = OpcodeVOP3(275);
    pub const V_MAX_U32: OpcodeVOP3 = OpcodeVOP3(276);
    pub const V_LSHR_B32: OpcodeVOP3 = OpcodeVOP3(277);
    pub const V_LSHRREV_B32: OpcodeVOP3 = OpcodeVOP3(278);
    pub const V_ASHR_I32: OpcodeVOP3 = OpcodeVOP3(279);
    pub const V_ASHRREV_I32: OpcodeVOP3 = OpcodeVOP3(280);
    pub const V_LSHL_B32: OpcodeVOP3 = OpcodeVOP3(281);
    pub const V_LSHLREV_B32: OpcodeVOP3 = OpcodeVOP3(282);
    pub const V_AND_B32: OpcodeVOP3 = OpcodeVOP3(283);
    pub const V_OR_B32: OpcodeVOP3 = OpcodeVOP3(284);
    pub const V_XOR_B32: OpcodeVOP3 = OpcodeVOP3(285);
    pub const V_BFM_B32: OpcodeVOP3 = OpcodeVOP3(286);
    pub const V_MAC_F32: OpcodeVOP3 = OpcodeVOP3(287);
    pub const V_MADMK_F32: OpcodeVOP3 = OpcodeVOP3(288);
    pub const V_MADAK_F32: OpcodeVOP3 = OpcodeVOP3(289);
    pub const V_BCNT_U32_B32: OpcodeVOP3 = OpcodeVOP3(290);
    pub const V_MBCNT_LO_U32_B32: OpcodeVOP3 = OpcodeVOP3(291);
    pub const V_MBCNT_HI_U32_B32: OpcodeVOP3 = OpcodeVOP3(292);
    pub const V_ADD_I32: OpcodeVOP3 = OpcodeVOP3(293);
    pub const V_SUB_I32: OpcodeVOP3 = OpcodeVOP3(294);
    pub const V_SUBREV_I32: OpcodeVOP3 = OpcodeVOP3(295);
    pub const V_ADDC_U32: OpcodeVOP3 = OpcodeVOP3(296);
    pub const V_SUBB_U32: OpcodeVOP3 = OpcodeVOP3(297);
    pub const V_SUBBREV_U32: OpcodeVOP3 = OpcodeVOP3(298);
    pub const V_LDEXP_F32: OpcodeVOP3 = OpcodeVOP3(299);
    pub const V_CVT_PKACCUM_U8_F32: OpcodeVOP3 = OpcodeVOP3(300);
    pub const V_CVT_PKNORM_I16_F32: OpcodeVOP3 = OpcodeVOP3(301);
    pub const V_CVT_PKNORM_U16_F32: OpcodeVOP3 = OpcodeVOP3(302);
    pub const V_CVT_PKRTZ_F16_F32: OpcodeVOP3 = OpcodeVOP3(303);
    pub const V_CVT_PK_U16_U32: OpcodeVOP3 = OpcodeVOP3(304);
    pub const V_CVT_PK_I16_I32: OpcodeVOP3 = OpcodeVOP3(305);
    pub const V_ADD_F16: OpcodeVOP3 = OpcodeVOP3(306);
    pub const V_SUB_F16: OpcodeVOP3 = OpcodeVOP3(307);
    pub const V_SUBREV_F16: OpcodeVOP3 = OpcodeVOP3(308);
    pub const V_MUL_F16: OpcodeVOP3 = OpcodeVOP3(309);
    pub const V_MAX_F16: OpcodeVOP3 = OpcodeVOP3(313);
    pub const V_MIN_F16: OpcodeVOP3 = OpcodeVOP3(314);
    pub const V_LDEXP_F16: OpcodeVOP3 = OpcodeVOP3(315);
    pub const V_MAD_LEGACY_F32: OpcodeVOP3 = OpcodeVOP3(320);
    pub const V_MAD_F32: OpcodeVOP3 = OpcodeVOP3(321);
    pub const V_MAD_I32_I24: OpcodeVOP3 = OpcodeVOP3(322);
    pub const V_MAD_U32_U24: OpcodeVOP3 = OpcodeVOP3(323);
    pub const V_CUBEID_F32: OpcodeVOP3 = OpcodeVOP3(324);
    pub const V_CUBESC_F32: OpcodeVOP3 = OpcodeVOP3(325);
    pub const V_CUBETC_F32: OpcodeVOP3 = OpcodeVOP3(326);
    pub const V_CUBEMA_F32: OpcodeVOP3 = OpcodeVOP3(327);
    pub const V_BFE_U32: OpcodeVOP3 = OpcodeVOP3(328);
    pub const V_BFE_I32: OpcodeVOP3 = OpcodeVOP3(329);
    pub const V_BFI_B32: OpcodeVOP3 = OpcodeVOP3(330);
    pub const V_FMA_F32: OpcodeVOP3 = OpcodeVOP3(331);
    pub const V_FMA_F64: OpcodeVOP3 = OpcodeVOP3(332);
    pub const V_LERP_U8: OpcodeVOP3 = OpcodeVOP3(333);
    pub const V_ALIGNBIT_B32: OpcodeVOP3 = OpcodeVOP3(334);
    pub const V_ALIGNBYTE_B32: OpcodeVOP3 = OpcodeVOP3(335);
    pub const V_MULLIT_F32: OpcodeVOP3 = OpcodeVOP3(336);
    pub const V_MIN3_F32: OpcodeVOP3 = OpcodeVOP3(337);
    pub const V_MIN3_I32: OpcodeVOP3 = OpcodeVOP3(338);
    pub const V_MIN3_U32: OpcodeVOP3 = OpcodeVOP3(339);
    pub const V_MAX3_F32: OpcodeVOP3 = OpcodeVOP3(340);
    pub const V_MAX3_I32: OpcodeVOP3 = OpcodeVOP3(341);
    pub const V_MAX3_U32: OpcodeVOP3 = OpcodeVOP3(342);
    pub const V_MED3_F32: OpcodeVOP3 = OpcodeVOP3(343);
    pub const V_MED3_I32: OpcodeVOP3 = OpcodeVOP3(344);
    pub const V_MED3_U32: OpcodeVOP3 = OpcodeVOP3(345);
    pub const V_SAD_U8: OpcodeVOP3 = OpcodeVOP3(346);
    pub const V_SAD_HI_U8: OpcodeVOP3 = OpcodeVOP3(347);
    pub const V_SAD_U16: OpcodeVOP3 = OpcodeVOP3(348);
    pub const V_SAD_U32: OpcodeVOP3 = OpcodeVOP3(349);
    pub const V_CVT_PK_U8_F32: OpcodeVOP3 = OpcodeVOP3(350);
    pub const V_DIV_FIXUP_F32: OpcodeVOP3 = OpcodeVOP3(351);
    pub const V_DIV_FIXUP_F64: OpcodeVOP3 = OpcodeVOP3(352);
    pub const V_LSHL_B64: OpcodeVOP3 = OpcodeVOP3(353);
    pub const V_LSHR_B64: OpcodeVOP3 = OpcodeVOP3(354);
    pub const V_ASHR_I64: OpcodeVOP3 = OpcodeVOP3(355);
    pub const V_ADD_F64: OpcodeVOP3 = OpcodeVOP3(356);
    pub const V_MUL_F64: OpcodeVOP3 = OpcodeVOP3(357);
    pub const V_MIN_F64: OpcodeVOP3 = OpcodeVOP3(358);
    pub const V_MAX_F64: OpcodeVOP3 = OpcodeVOP3(359);
    pub const V_LDEXP_F64: OpcodeVOP3 = OpcodeVOP3(360);
    pub const V_MUL_LO_U32: OpcodeVOP3 = OpcodeVOP3(361);
    pub const V_MUL_HI_U32: OpcodeVOP3 = OpcodeVOP3(362);
    pub const V_MUL_LO_I32: OpcodeVOP3 = OpcodeVOP3(363);
    pub const V_MUL_HI_I32: OpcodeVOP3 = OpcodeVOP3(364);
    pub const V_DIV_SCALE_F32: OpcodeVOP3 = OpcodeVOP3(365);
    pub const V_DIV_SCALE_F64: OpcodeVOP3 = OpcodeVOP3(366);
    pub const V_DIV_FMAS_F32: OpcodeVOP3 = OpcodeVOP3(367);
    pub const V_DIV_FMAS_F64: OpcodeVOP3 = OpcodeVOP3(368);
    pub const V_MSAD_U8: OpcodeVOP3 = OpcodeVOP3(369);
    pub const V_QSAD_U8: OpcodeVOP3 = OpcodeVOP3(370);
    pub const V_QSAD_PK_U16_U8: OpcodeVOP3 = OpcodeVOP3(370);
    pub const V_MQSAD_U8: OpcodeVOP3 = OpcodeVOP3(371);
    pub const V_MQSAD_PK_U16_U8: OpcodeVOP3 = OpcodeVOP3(371);
    pub const V_TRIG_PREOP_F64: OpcodeVOP3 = OpcodeVOP3(372);
    pub const V_MQSAD_U32_U8: OpcodeVOP3 = OpcodeVOP3(373);
    pub const V_MAD_U64_U32: OpcodeVOP3 = OpcodeVOP3(374);
    pub const V_MAD_I64_I32: OpcodeVOP3 = OpcodeVOP3(375);
    pub const V_NOP: OpcodeVOP3 = OpcodeVOP3(384);
    pub const V_MOV_B32: OpcodeVOP3 = OpcodeVOP3(385);
    pub const V_READFIRSTLANE_B32: OpcodeVOP3 = OpcodeVOP3(386);
    pub const V_CVT_I32_F64: OpcodeVOP3 = OpcodeVOP3(387);
    pub const V_CVT_F64_I32: OpcodeVOP3 = OpcodeVOP3(388);
    pub const V_CVT_F32_I32: OpcodeVOP3 = OpcodeVOP3(389);
    pub const V_CVT_F32_U32: OpcodeVOP3 = OpcodeVOP3(390);
    pub const V_CVT_U32_F32: OpcodeVOP3 = OpcodeVOP3(391);
    pub const V_CVT_I32_F32: OpcodeVOP3 = OpcodeVOP3(392);
    pub const V_MOV_FED_B32: OpcodeVOP3 = OpcodeVOP3(393);
    pub const V_CVT_F16_F32: OpcodeVOP3 = OpcodeVOP3(394);
    pub const V_CVT_F32_F16: OpcodeVOP3 = OpcodeVOP3(395);
    pub const V_CVT_RPI_I32_F32: OpcodeVOP3 = OpcodeVOP3(396);
    pub const V_CVT_FLR_I32_F32: OpcodeVOP3 = OpcodeVOP3(397);
    pub const V_CVT_OFF_F32_I4: OpcodeVOP3 = OpcodeVOP3(398);
    pub const V_CVT_F32_F64: OpcodeVOP3 = OpcodeVOP3(399);
    pub const V_CVT_F64_F32: OpcodeVOP3 = OpcodeVOP3(400);
    pub const V_CVT_F32_UBYTE0: OpcodeVOP3 = OpcodeVOP3(401);
    pub const V_CVT_F32_UBYTE1: OpcodeVOP3 = OpcodeVOP3(402);
    pub const V_CVT_F32_UBYTE2: OpcodeVOP3 = OpcodeVOP3(403);
    pub const V_CVT_F32_UBYTE3: OpcodeVOP3 = OpcodeVOP3(404);
    pub const V_CVT_U32_F64: OpcodeVOP3 = OpcodeVOP3(405);
    pub const V_CVT_F64_U32: OpcodeVOP3 = OpcodeVOP3(406);
    pub const V_TRUNC_F64: OpcodeVOP3 = OpcodeVOP3(407);
    pub const V_CEIL_F64: OpcodeVOP3 = OpcodeVOP3(408);
    pub const V_RNDNE_F64: OpcodeVOP3 = OpcodeVOP3(409);
    pub const V_FLOOR_F64: OpcodeVOP3 = OpcodeVOP3(410);
    pub const V_FRACT_F32: OpcodeVOP3 = OpcodeVOP3(416);
    pub const V_TRUNC_F32: OpcodeVOP3 = OpcodeVOP3(417);
    pub const V_CEIL_F32: OpcodeVOP3 = OpcodeVOP3(418);
    pub const V_RNDNE_F32: OpcodeVOP3 = OpcodeVOP3(419);
    pub const V_FLOOR_F32: OpcodeVOP3 = OpcodeVOP3(420);
    pub const V_EXP_F32: OpcodeVOP3 = OpcodeVOP3(421);
    pub const V_LOG_CLAMP_F32: OpcodeVOP3 = OpcodeVOP3(422);
    pub const V_LOG_F32: OpcodeVOP3 = OpcodeVOP3(423);
    pub const V_RCP_CLAMP_F32: OpcodeVOP3 = OpcodeVOP3(424);
    pub const V_RCP_LEGACY_F32: OpcodeVOP3 = OpcodeVOP3(425);
    pub const V_RCP_F32: OpcodeVOP3 = OpcodeVOP3(426);
    pub const V_RCP_IFLAG_F32: OpcodeVOP3 = OpcodeVOP3(427);
    pub const V_RSQ_CLAMP_F32: OpcodeVOP3 = OpcodeVOP3(428);
    pub const V_RSQ_LEGACY_F32: OpcodeVOP3 = OpcodeVOP3(429);
    pub const V_RSQ_F32: OpcodeVOP3 = OpcodeVOP3(430);
    pub const V_RCP_F64: OpcodeVOP3 = OpcodeVOP3(431);
    pub const V_RCP_CLAMP_F64: OpcodeVOP3 = OpcodeVOP3(432);
    pub const V_RSQ_F64: OpcodeVOP3 = OpcodeVOP3(433);
    pub const V_RSQ_CLAMP_F64: OpcodeVOP3 = OpcodeVOP3(434);
    pub const V_SQRT_F32: OpcodeVOP3 = OpcodeVOP3(435);
    pub const V_SQRT_F64: OpcodeVOP3 = OpcodeVOP3(436);
    pub const V_SIN_F32: OpcodeVOP3 = OpcodeVOP3(437);
    pub const V_COS_F32: OpcodeVOP3 = OpcodeVOP3(438);
    pub const V_NOT_B32: OpcodeVOP3 = OpcodeVOP3(439);
    pub const V_BFREV_B32: OpcodeVOP3 = OpcodeVOP3(440);
    pub const V_FFBH_U32: OpcodeVOP3 = OpcodeVOP3(441);
    pub const V_FFBL_B32: OpcodeVOP3 = OpcodeVOP3(442);
    pub const V_FFBH_I32: OpcodeVOP3 = OpcodeVOP3(443);
    pub const V_FREXP_EXP_I32_F64: OpcodeVOP3 = OpcodeVOP3(444);
    pub const V_FREXP_MANT_F64: OpcodeVOP3 = OpcodeVOP3(445);
    pub const V_FRACT_F64: OpcodeVOP3 = OpcodeVOP3(446);
    pub const V_FREXP_EXP_I32_F32: OpcodeVOP3 = OpcodeVOP3(447);
    pub const V_FREXP_MANT_F32: OpcodeVOP3 = OpcodeVOP3(448);
    pub const V_CLREXCP: OpcodeVOP3 = OpcodeVOP3(449);
    pub const V_MOVRELD_B32: OpcodeVOP3 = OpcodeVOP3(450);
    pub const V_MOVRELS_B32: OpcodeVOP3 = OpcodeVOP3(451);
    pub const V_MOVRELSD_B32: OpcodeVOP3 = OpcodeVOP3(452);
    pub const V_LOG_LEGACY_F32: OpcodeVOP3 = OpcodeVOP3(453);
    pub const V_EXP_LEGACY_F32: OpcodeVOP3 = OpcodeVOP3(454);
    pub const V_RCP_F16: OpcodeVOP3 = OpcodeVOP3(468);
    pub const V_SQRT_F16: OpcodeVOP3 = OpcodeVOP3(469);
    pub const V_ADD_NC_U16: OpcodeVOP3 = OpcodeVOP3(771);
    pub const V_LSHRREV_B16: OpcodeVOP3 = OpcodeVOP3(775);
    pub const V_ASHRREV_I16: OpcodeVOP3 = OpcodeVOP3(776);
    pub const V_ADD_NC_I16: OpcodeVOP3 = OpcodeVOP3(781);
    pub const V_SUB_NC_I16: OpcodeVOP3 = OpcodeVOP3(782);
    pub const V_SUB_CO_U32: OpcodeVOP3 = OpcodeVOP3(784);
    pub const V_LSHLREV_B16: OpcodeVOP3 = OpcodeVOP3(788);
    pub const V_MAD_F16: OpcodeVOP3 = OpcodeVOP3(833);
    pub const V_LSHL_ADD_U32: OpcodeVOP3 = OpcodeVOP3(838);
    pub const V_ADD_LSHL_U32: OpcodeVOP3 = OpcodeVOP3(839);
    pub const V_MIN3_F16: OpcodeVOP3 = OpcodeVOP3(849);
    pub const V_MAX3_F16: OpcodeVOP3 = OpcodeVOP3(852);
    pub const V_MED3_F16: OpcodeVOP3 = OpcodeVOP3(855);
    pub const V_MAD_I16: OpcodeVOP3 = OpcodeVOP3(862);
    pub const V_ADD3_U32: OpcodeVOP3 = OpcodeVOP3(877);
    pub const V_LSHL_OR_B32: OpcodeVOP3 = OpcodeVOP3(879);
    pub const V_AND_OR_B32: OpcodeVOP3 = OpcodeVOP3(881);
    pub const V_OR3_B32: OpcodeVOP3 = OpcodeVOP3(882);
    pub const OP_RANGE_VOP3: OpcodeVOP3 = OpcodeVOP3(1024);
}

/// `Shader::Gcn::OpcodeVOP3P`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeVOP3P(pub u32);

impl OpcodeVOP3P {
    pub const V_PK_MAD_I16: OpcodeVOP3P = OpcodeVOP3P(0);
    pub const V_PK_MUL_LO_U16: OpcodeVOP3P = OpcodeVOP3P(1);
    pub const V_PK_ADD_I16: OpcodeVOP3P = OpcodeVOP3P(2);
    pub const V_PK_SUB_I16: OpcodeVOP3P = OpcodeVOP3P(3);
    pub const V_PK_LSHLREV_B16: OpcodeVOP3P = OpcodeVOP3P(4);
    pub const V_PK_LSHRREV_B16: OpcodeVOP3P = OpcodeVOP3P(5);
    pub const V_PK_ASHRREV_I16: OpcodeVOP3P = OpcodeVOP3P(6);
    pub const V_PK_MAX_I16: OpcodeVOP3P = OpcodeVOP3P(7);
    pub const V_PK_MIN_I16: OpcodeVOP3P = OpcodeVOP3P(8);
    pub const V_PK_MAD_U16: OpcodeVOP3P = OpcodeVOP3P(9);
    pub const V_PK_ADD_U16: OpcodeVOP3P = OpcodeVOP3P(10);
    pub const V_PK_SUB_U16: OpcodeVOP3P = OpcodeVOP3P(11);
    pub const V_PK_MAX_U16: OpcodeVOP3P = OpcodeVOP3P(12);
    pub const V_PK_MIN_U16: OpcodeVOP3P = OpcodeVOP3P(13);
    pub const V_PK_FMA_F16: OpcodeVOP3P = OpcodeVOP3P(14);
    pub const V_PK_ADD_F16: OpcodeVOP3P = OpcodeVOP3P(15);
    pub const V_PK_MUL_F16: OpcodeVOP3P = OpcodeVOP3P(16);
    pub const V_PK_MIN_F16: OpcodeVOP3P = OpcodeVOP3P(17);
    pub const V_PK_MAX_F16: OpcodeVOP3P = OpcodeVOP3P(18);
    pub const V_MAD_MIX_F32: OpcodeVOP3P = OpcodeVOP3P(32);
    pub const V_MAD_MIXLO_F16: OpcodeVOP3P = OpcodeVOP3P(33);
    pub const V_MAD_MIXHI_F16: OpcodeVOP3P = OpcodeVOP3P(34);
    pub const OP_RANGE_VOP3P: OpcodeVOP3P = OpcodeVOP3P(OpcodeVOP3P::V_MAD_MIXHI_F16.0 + 1);
}

/// `Shader::Gcn::OpcodeVOP1`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeVOP1(pub u32);

impl OpcodeVOP1 {
    pub const V_NOP: OpcodeVOP1 = OpcodeVOP1(0);
    pub const V_MOV_B32: OpcodeVOP1 = OpcodeVOP1(1);
    pub const V_READFIRSTLANE_B32: OpcodeVOP1 = OpcodeVOP1(2);
    pub const V_CVT_I32_F64: OpcodeVOP1 = OpcodeVOP1(3);
    pub const V_CVT_F64_I32: OpcodeVOP1 = OpcodeVOP1(4);
    pub const V_CVT_F32_I32: OpcodeVOP1 = OpcodeVOP1(5);
    pub const V_CVT_F32_U32: OpcodeVOP1 = OpcodeVOP1(6);
    pub const V_CVT_U32_F32: OpcodeVOP1 = OpcodeVOP1(7);
    pub const V_CVT_I32_F32: OpcodeVOP1 = OpcodeVOP1(8);
    pub const V_MOV_FED_B32: OpcodeVOP1 = OpcodeVOP1(9);
    pub const V_CVT_F16_F32: OpcodeVOP1 = OpcodeVOP1(10);
    pub const V_CVT_F32_F16: OpcodeVOP1 = OpcodeVOP1(11);
    pub const V_CVT_RPI_I32_F32: OpcodeVOP1 = OpcodeVOP1(12);
    pub const V_CVT_FLR_I32_F32: OpcodeVOP1 = OpcodeVOP1(13);
    pub const V_CVT_OFF_F32_I4: OpcodeVOP1 = OpcodeVOP1(14);
    pub const V_CVT_F32_F64: OpcodeVOP1 = OpcodeVOP1(15);
    pub const V_CVT_F64_F32: OpcodeVOP1 = OpcodeVOP1(16);
    pub const V_CVT_F32_UBYTE0: OpcodeVOP1 = OpcodeVOP1(17);
    pub const V_CVT_F32_UBYTE1: OpcodeVOP1 = OpcodeVOP1(18);
    pub const V_CVT_F32_UBYTE2: OpcodeVOP1 = OpcodeVOP1(19);
    pub const V_CVT_F32_UBYTE3: OpcodeVOP1 = OpcodeVOP1(20);
    pub const V_CVT_U32_F64: OpcodeVOP1 = OpcodeVOP1(21);
    pub const V_CVT_F64_U32: OpcodeVOP1 = OpcodeVOP1(22);
    pub const V_TRUNC_F64: OpcodeVOP1 = OpcodeVOP1(23);
    pub const V_CEIL_F64: OpcodeVOP1 = OpcodeVOP1(24);
    pub const V_RNDNE_F64: OpcodeVOP1 = OpcodeVOP1(25);
    pub const V_FLOOR_F64: OpcodeVOP1 = OpcodeVOP1(26);
    pub const V_FRACT_F32: OpcodeVOP1 = OpcodeVOP1(32);
    pub const V_TRUNC_F32: OpcodeVOP1 = OpcodeVOP1(33);
    pub const V_CEIL_F32: OpcodeVOP1 = OpcodeVOP1(34);
    pub const V_RNDNE_F32: OpcodeVOP1 = OpcodeVOP1(35);
    pub const V_FLOOR_F32: OpcodeVOP1 = OpcodeVOP1(36);
    pub const V_EXP_F32: OpcodeVOP1 = OpcodeVOP1(37);
    pub const V_LOG_CLAMP_F32: OpcodeVOP1 = OpcodeVOP1(38);
    pub const V_LOG_F32: OpcodeVOP1 = OpcodeVOP1(39);
    pub const V_RCP_CLAMP_F32: OpcodeVOP1 = OpcodeVOP1(40);
    pub const V_RCP_LEGACY_F32: OpcodeVOP1 = OpcodeVOP1(41);
    pub const V_RCP_F32: OpcodeVOP1 = OpcodeVOP1(42);
    pub const V_RCP_IFLAG_F32: OpcodeVOP1 = OpcodeVOP1(43);
    pub const V_RSQ_CLAMP_F32: OpcodeVOP1 = OpcodeVOP1(44);
    pub const V_RSQ_LEGACY_F32: OpcodeVOP1 = OpcodeVOP1(45);
    pub const V_RSQ_F32: OpcodeVOP1 = OpcodeVOP1(46);
    pub const V_RCP_F64: OpcodeVOP1 = OpcodeVOP1(47);
    pub const V_RCP_CLAMP_F64: OpcodeVOP1 = OpcodeVOP1(48);
    pub const V_RSQ_F64: OpcodeVOP1 = OpcodeVOP1(49);
    pub const V_RSQ_CLAMP_F64: OpcodeVOP1 = OpcodeVOP1(50);
    pub const V_SQRT_F32: OpcodeVOP1 = OpcodeVOP1(51);
    pub const V_SQRT_F64: OpcodeVOP1 = OpcodeVOP1(52);
    pub const V_SIN_F32: OpcodeVOP1 = OpcodeVOP1(53);
    pub const V_COS_F32: OpcodeVOP1 = OpcodeVOP1(54);
    pub const V_NOT_B32: OpcodeVOP1 = OpcodeVOP1(55);
    pub const V_BFREV_B32: OpcodeVOP1 = OpcodeVOP1(56);
    pub const V_FFBH_U32: OpcodeVOP1 = OpcodeVOP1(57);
    pub const V_FFBL_B32: OpcodeVOP1 = OpcodeVOP1(58);
    pub const V_FFBH_I32: OpcodeVOP1 = OpcodeVOP1(59);
    pub const V_FREXP_EXP_I32_F64: OpcodeVOP1 = OpcodeVOP1(60);
    pub const V_FREXP_MANT_F64: OpcodeVOP1 = OpcodeVOP1(61);
    pub const V_FRACT_F64: OpcodeVOP1 = OpcodeVOP1(62);
    pub const V_FREXP_EXP_I32_F32: OpcodeVOP1 = OpcodeVOP1(63);
    pub const V_FREXP_MANT_F32: OpcodeVOP1 = OpcodeVOP1(64);
    pub const V_CLREXCP: OpcodeVOP1 = OpcodeVOP1(65);
    pub const V_MOVRELD_B32: OpcodeVOP1 = OpcodeVOP1(66);
    pub const V_MOVRELS_B32: OpcodeVOP1 = OpcodeVOP1(67);
    pub const V_MOVRELSD_B32: OpcodeVOP1 = OpcodeVOP1(68);
    pub const V_LOG_LEGACY_F32: OpcodeVOP1 = OpcodeVOP1(69);
    pub const V_EXP_LEGACY_F32: OpcodeVOP1 = OpcodeVOP1(70);
    pub const V_RCP_F16: OpcodeVOP1 = OpcodeVOP1(84);
    pub const V_SQRT_F16: OpcodeVOP1 = OpcodeVOP1(85);
    pub const OP_RANGE_VOP1: OpcodeVOP1 = OpcodeVOP1(OpcodeVOP1::V_SQRT_F16.0 + 1);
}

/// `Shader::Gcn::OpcodeVOPC`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeVOPC(pub u32);

impl OpcodeVOPC {
    pub const V_CMP_F_F32: OpcodeVOPC = OpcodeVOPC(0);
    pub const V_CMP_LT_F32: OpcodeVOPC = OpcodeVOPC(1);
    pub const V_CMP_EQ_F32: OpcodeVOPC = OpcodeVOPC(2);
    pub const V_CMP_LE_F32: OpcodeVOPC = OpcodeVOPC(3);
    pub const V_CMP_GT_F32: OpcodeVOPC = OpcodeVOPC(4);
    pub const V_CMP_LG_F32: OpcodeVOPC = OpcodeVOPC(5);
    pub const V_CMP_GE_F32: OpcodeVOPC = OpcodeVOPC(6);
    pub const V_CMP_O_F32: OpcodeVOPC = OpcodeVOPC(7);
    pub const V_CMP_U_F32: OpcodeVOPC = OpcodeVOPC(8);
    pub const V_CMP_NGE_F32: OpcodeVOPC = OpcodeVOPC(9);
    pub const V_CMP_NLG_F32: OpcodeVOPC = OpcodeVOPC(10);
    pub const V_CMP_NGT_F32: OpcodeVOPC = OpcodeVOPC(11);
    pub const V_CMP_NLE_F32: OpcodeVOPC = OpcodeVOPC(12);
    pub const V_CMP_NEQ_F32: OpcodeVOPC = OpcodeVOPC(13);
    pub const V_CMP_NLT_F32: OpcodeVOPC = OpcodeVOPC(14);
    pub const V_CMP_TRU_F32: OpcodeVOPC = OpcodeVOPC(15);
    pub const V_CMP_T_F32: OpcodeVOPC = OpcodeVOPC(15);
    pub const V_CMPX_F_F32: OpcodeVOPC = OpcodeVOPC(16);
    pub const V_CMPX_LT_F32: OpcodeVOPC = OpcodeVOPC(17);
    pub const V_CMPX_EQ_F32: OpcodeVOPC = OpcodeVOPC(18);
    pub const V_CMPX_LE_F32: OpcodeVOPC = OpcodeVOPC(19);
    pub const V_CMPX_GT_F32: OpcodeVOPC = OpcodeVOPC(20);
    pub const V_CMPX_LG_F32: OpcodeVOPC = OpcodeVOPC(21);
    pub const V_CMPX_GE_F32: OpcodeVOPC = OpcodeVOPC(22);
    pub const V_CMPX_O_F32: OpcodeVOPC = OpcodeVOPC(23);
    pub const V_CMPX_U_F32: OpcodeVOPC = OpcodeVOPC(24);
    pub const V_CMPX_NGE_F32: OpcodeVOPC = OpcodeVOPC(25);
    pub const V_CMPX_NLG_F32: OpcodeVOPC = OpcodeVOPC(26);
    pub const V_CMPX_NGT_F32: OpcodeVOPC = OpcodeVOPC(27);
    pub const V_CMPX_NLE_F32: OpcodeVOPC = OpcodeVOPC(28);
    pub const V_CMPX_NEQ_F32: OpcodeVOPC = OpcodeVOPC(29);
    pub const V_CMPX_NLT_F32: OpcodeVOPC = OpcodeVOPC(30);
    pub const V_CMPX_TRU_F32: OpcodeVOPC = OpcodeVOPC(31);
    pub const V_CMPX_T_F32: OpcodeVOPC = OpcodeVOPC(31);
    pub const V_CMP_F_F64: OpcodeVOPC = OpcodeVOPC(32);
    pub const V_CMP_LT_F64: OpcodeVOPC = OpcodeVOPC(33);
    pub const V_CMP_EQ_F64: OpcodeVOPC = OpcodeVOPC(34);
    pub const V_CMP_LE_F64: OpcodeVOPC = OpcodeVOPC(35);
    pub const V_CMP_GT_F64: OpcodeVOPC = OpcodeVOPC(36);
    pub const V_CMP_LG_F64: OpcodeVOPC = OpcodeVOPC(37);
    pub const V_CMP_GE_F64: OpcodeVOPC = OpcodeVOPC(38);
    pub const V_CMP_O_F64: OpcodeVOPC = OpcodeVOPC(39);
    pub const V_CMP_U_F64: OpcodeVOPC = OpcodeVOPC(40);
    pub const V_CMP_NGE_F64: OpcodeVOPC = OpcodeVOPC(41);
    pub const V_CMP_NLG_F64: OpcodeVOPC = OpcodeVOPC(42);
    pub const V_CMP_NGT_F64: OpcodeVOPC = OpcodeVOPC(43);
    pub const V_CMP_NLE_F64: OpcodeVOPC = OpcodeVOPC(44);
    pub const V_CMP_NEQ_F64: OpcodeVOPC = OpcodeVOPC(45);
    pub const V_CMP_NLT_F64: OpcodeVOPC = OpcodeVOPC(46);
    pub const V_CMP_TRU_F64: OpcodeVOPC = OpcodeVOPC(47);
    pub const V_CMP_T_F64: OpcodeVOPC = OpcodeVOPC(47);
    pub const V_CMPX_F_F64: OpcodeVOPC = OpcodeVOPC(48);
    pub const V_CMPX_LT_F64: OpcodeVOPC = OpcodeVOPC(49);
    pub const V_CMPX_EQ_F64: OpcodeVOPC = OpcodeVOPC(50);
    pub const V_CMPX_LE_F64: OpcodeVOPC = OpcodeVOPC(51);
    pub const V_CMPX_GT_F64: OpcodeVOPC = OpcodeVOPC(52);
    pub const V_CMPX_LG_F64: OpcodeVOPC = OpcodeVOPC(53);
    pub const V_CMPX_GE_F64: OpcodeVOPC = OpcodeVOPC(54);
    pub const V_CMPX_O_F64: OpcodeVOPC = OpcodeVOPC(55);
    pub const V_CMPX_U_F64: OpcodeVOPC = OpcodeVOPC(56);
    pub const V_CMPX_NGE_F64: OpcodeVOPC = OpcodeVOPC(57);
    pub const V_CMPX_NLG_F64: OpcodeVOPC = OpcodeVOPC(58);
    pub const V_CMPX_NGT_F64: OpcodeVOPC = OpcodeVOPC(59);
    pub const V_CMPX_NLE_F64: OpcodeVOPC = OpcodeVOPC(60);
    pub const V_CMPX_NEQ_F64: OpcodeVOPC = OpcodeVOPC(61);
    pub const V_CMPX_NLT_F64: OpcodeVOPC = OpcodeVOPC(62);
    pub const V_CMPX_TRU_F64: OpcodeVOPC = OpcodeVOPC(63);
    pub const V_CMPX_T_F64: OpcodeVOPC = OpcodeVOPC(63);
    pub const V_CMPS_F_F32: OpcodeVOPC = OpcodeVOPC(64);
    pub const V_CMPS_LT_F32: OpcodeVOPC = OpcodeVOPC(65);
    pub const V_CMPS_EQ_F32: OpcodeVOPC = OpcodeVOPC(66);
    pub const V_CMPS_LE_F32: OpcodeVOPC = OpcodeVOPC(67);
    pub const V_CMPS_GT_F32: OpcodeVOPC = OpcodeVOPC(68);
    pub const V_CMPS_LG_F32: OpcodeVOPC = OpcodeVOPC(69);
    pub const V_CMPS_GE_F32: OpcodeVOPC = OpcodeVOPC(70);
    pub const V_CMPS_O_F32: OpcodeVOPC = OpcodeVOPC(71);
    pub const V_CMPS_U_F32: OpcodeVOPC = OpcodeVOPC(72);
    pub const V_CMPS_NGE_F32: OpcodeVOPC = OpcodeVOPC(73);
    pub const V_CMPS_NLG_F32: OpcodeVOPC = OpcodeVOPC(74);
    pub const V_CMPS_NGT_F32: OpcodeVOPC = OpcodeVOPC(75);
    pub const V_CMPS_NLE_F32: OpcodeVOPC = OpcodeVOPC(76);
    pub const V_CMPS_NEQ_F32: OpcodeVOPC = OpcodeVOPC(77);
    pub const V_CMPS_NLT_F32: OpcodeVOPC = OpcodeVOPC(78);
    pub const V_CMPS_TRU_F32: OpcodeVOPC = OpcodeVOPC(79);
    pub const V_CMPS_T_F32: OpcodeVOPC = OpcodeVOPC(79);
    pub const V_CMPSX_F_F32: OpcodeVOPC = OpcodeVOPC(80);
    pub const V_CMPSX_LT_F32: OpcodeVOPC = OpcodeVOPC(81);
    pub const V_CMPSX_EQ_F32: OpcodeVOPC = OpcodeVOPC(82);
    pub const V_CMPSX_LE_F32: OpcodeVOPC = OpcodeVOPC(83);
    pub const V_CMPSX_GT_F32: OpcodeVOPC = OpcodeVOPC(84);
    pub const V_CMPSX_LG_F32: OpcodeVOPC = OpcodeVOPC(85);
    pub const V_CMPSX_GE_F32: OpcodeVOPC = OpcodeVOPC(86);
    pub const V_CMPSX_O_F32: OpcodeVOPC = OpcodeVOPC(87);
    pub const V_CMPSX_U_F32: OpcodeVOPC = OpcodeVOPC(88);
    pub const V_CMPSX_NGE_F32: OpcodeVOPC = OpcodeVOPC(89);
    pub const V_CMPSX_NLG_F32: OpcodeVOPC = OpcodeVOPC(90);
    pub const V_CMPSX_NGT_F32: OpcodeVOPC = OpcodeVOPC(91);
    pub const V_CMPSX_NLE_F32: OpcodeVOPC = OpcodeVOPC(92);
    pub const V_CMPSX_NEQ_F32: OpcodeVOPC = OpcodeVOPC(93);
    pub const V_CMPSX_NLT_F32: OpcodeVOPC = OpcodeVOPC(94);
    pub const V_CMPSX_TRU_F32: OpcodeVOPC = OpcodeVOPC(95);
    pub const V_CMPSX_T_F32: OpcodeVOPC = OpcodeVOPC(95);
    pub const V_CMPS_F_F64: OpcodeVOPC = OpcodeVOPC(96);
    pub const V_CMPS_LT_F64: OpcodeVOPC = OpcodeVOPC(97);
    pub const V_CMPS_EQ_F64: OpcodeVOPC = OpcodeVOPC(98);
    pub const V_CMPS_LE_F64: OpcodeVOPC = OpcodeVOPC(99);
    pub const V_CMPS_GT_F64: OpcodeVOPC = OpcodeVOPC(100);
    pub const V_CMPS_LG_F64: OpcodeVOPC = OpcodeVOPC(101);
    pub const V_CMPS_GE_F64: OpcodeVOPC = OpcodeVOPC(102);
    pub const V_CMPS_O_F64: OpcodeVOPC = OpcodeVOPC(103);
    pub const V_CMPS_U_F64: OpcodeVOPC = OpcodeVOPC(104);
    pub const V_CMPS_NGE_F64: OpcodeVOPC = OpcodeVOPC(105);
    pub const V_CMPS_NLG_F64: OpcodeVOPC = OpcodeVOPC(106);
    pub const V_CMPS_NGT_F64: OpcodeVOPC = OpcodeVOPC(107);
    pub const V_CMPS_NLE_F64: OpcodeVOPC = OpcodeVOPC(108);
    pub const V_CMPS_NEQ_F64: OpcodeVOPC = OpcodeVOPC(109);
    pub const V_CMPS_NLT_F64: OpcodeVOPC = OpcodeVOPC(110);
    pub const V_CMPS_TRU_F64: OpcodeVOPC = OpcodeVOPC(111);
    pub const V_CMPS_T_F64: OpcodeVOPC = OpcodeVOPC(111);
    pub const V_CMPSX_F_F64: OpcodeVOPC = OpcodeVOPC(112);
    pub const V_CMPSX_LT_F64: OpcodeVOPC = OpcodeVOPC(113);
    pub const V_CMPSX_EQ_F64: OpcodeVOPC = OpcodeVOPC(114);
    pub const V_CMPSX_LE_F64: OpcodeVOPC = OpcodeVOPC(115);
    pub const V_CMPSX_GT_F64: OpcodeVOPC = OpcodeVOPC(116);
    pub const V_CMPSX_LG_F64: OpcodeVOPC = OpcodeVOPC(117);
    pub const V_CMPSX_GE_F64: OpcodeVOPC = OpcodeVOPC(118);
    pub const V_CMPSX_O_F64: OpcodeVOPC = OpcodeVOPC(119);
    pub const V_CMPSX_U_F64: OpcodeVOPC = OpcodeVOPC(120);
    pub const V_CMPSX_NGE_F64: OpcodeVOPC = OpcodeVOPC(121);
    pub const V_CMPSX_NLG_F64: OpcodeVOPC = OpcodeVOPC(122);
    pub const V_CMPSX_NGT_F64: OpcodeVOPC = OpcodeVOPC(123);
    pub const V_CMPSX_NLE_F64: OpcodeVOPC = OpcodeVOPC(124);
    pub const V_CMPSX_NEQ_F64: OpcodeVOPC = OpcodeVOPC(125);
    pub const V_CMPSX_NLT_F64: OpcodeVOPC = OpcodeVOPC(126);
    pub const V_CMPSX_TRU_F64: OpcodeVOPC = OpcodeVOPC(127);
    pub const V_CMPSX_T_F64: OpcodeVOPC = OpcodeVOPC(127);
    pub const V_CMP_F_I32: OpcodeVOPC = OpcodeVOPC(128);
    pub const V_CMP_LT_I32: OpcodeVOPC = OpcodeVOPC(129);
    pub const V_CMP_EQ_I32: OpcodeVOPC = OpcodeVOPC(130);
    pub const V_CMP_LE_I32: OpcodeVOPC = OpcodeVOPC(131);
    pub const V_CMP_GT_I32: OpcodeVOPC = OpcodeVOPC(132);
    pub const V_CMP_LG_I32: OpcodeVOPC = OpcodeVOPC(133);
    pub const V_CMP_NE_I32: OpcodeVOPC = OpcodeVOPC(133);
    pub const V_CMP_GE_I32: OpcodeVOPC = OpcodeVOPC(134);
    pub const V_CMP_TRU_I32: OpcodeVOPC = OpcodeVOPC(135);
    pub const V_CMP_T_I32: OpcodeVOPC = OpcodeVOPC(135);
    pub const V_CMP_CLASS_F32: OpcodeVOPC = OpcodeVOPC(136);
    pub const V_CMP_LT_I16: OpcodeVOPC = OpcodeVOPC(137);
    pub const V_CMP_EQ_I16: OpcodeVOPC = OpcodeVOPC(138);
    pub const V_CMP_LE_I16: OpcodeVOPC = OpcodeVOPC(139);
    pub const V_CMP_GT_I16: OpcodeVOPC = OpcodeVOPC(140);
    pub const V_CMP_NE_I16: OpcodeVOPC = OpcodeVOPC(141);
    pub const V_CMP_GE_I16: OpcodeVOPC = OpcodeVOPC(142);
    pub const V_CMPX_F_I32: OpcodeVOPC = OpcodeVOPC(144);
    pub const V_CMPX_LT_I32: OpcodeVOPC = OpcodeVOPC(145);
    pub const V_CMPX_EQ_I32: OpcodeVOPC = OpcodeVOPC(146);
    pub const V_CMPX_LE_I32: OpcodeVOPC = OpcodeVOPC(147);
    pub const V_CMPX_GT_I32: OpcodeVOPC = OpcodeVOPC(148);
    pub const V_CMPX_LG_I32: OpcodeVOPC = OpcodeVOPC(149);
    pub const V_CMPX_NE_I32: OpcodeVOPC = OpcodeVOPC(149);
    pub const V_CMPX_GE_I32: OpcodeVOPC = OpcodeVOPC(150);
    pub const V_CMPX_TRU_I32: OpcodeVOPC = OpcodeVOPC(151);
    pub const V_CMPX_T_I32: OpcodeVOPC = OpcodeVOPC(151);
    pub const V_CMPX_CLASS_F32: OpcodeVOPC = OpcodeVOPC(152);
    pub const V_CMP_F_I64: OpcodeVOPC = OpcodeVOPC(160);
    pub const V_CMP_LT_I64: OpcodeVOPC = OpcodeVOPC(161);
    pub const V_CMP_EQ_I64: OpcodeVOPC = OpcodeVOPC(162);
    pub const V_CMP_LE_I64: OpcodeVOPC = OpcodeVOPC(163);
    pub const V_CMP_GT_I64: OpcodeVOPC = OpcodeVOPC(164);
    pub const V_CMP_LG_I64: OpcodeVOPC = OpcodeVOPC(165);
    pub const V_CMP_NE_I64: OpcodeVOPC = OpcodeVOPC(165);
    pub const V_CMP_GE_I64: OpcodeVOPC = OpcodeVOPC(166);
    pub const V_CMP_TRU_I64: OpcodeVOPC = OpcodeVOPC(167);
    pub const V_CMP_T_I64: OpcodeVOPC = OpcodeVOPC(167);
    pub const V_CMP_CLASS_F64: OpcodeVOPC = OpcodeVOPC(168);
    pub const V_CMP_LT_U16: OpcodeVOPC = OpcodeVOPC(169);
    pub const V_CMP_EQ_U16: OpcodeVOPC = OpcodeVOPC(170);
    pub const V_CMP_LE_U16: OpcodeVOPC = OpcodeVOPC(171);
    pub const V_CMP_GT_U16: OpcodeVOPC = OpcodeVOPC(172);
    pub const V_CMP_NE_U16: OpcodeVOPC = OpcodeVOPC(173);
    pub const V_CMP_GE_U16: OpcodeVOPC = OpcodeVOPC(174);
    pub const V_CMPX_F_I64: OpcodeVOPC = OpcodeVOPC(176);
    pub const V_CMPX_LT_I64: OpcodeVOPC = OpcodeVOPC(177);
    pub const V_CMPX_EQ_I64: OpcodeVOPC = OpcodeVOPC(178);
    pub const V_CMPX_LE_I64: OpcodeVOPC = OpcodeVOPC(179);
    pub const V_CMPX_GT_I64: OpcodeVOPC = OpcodeVOPC(180);
    pub const V_CMPX_LG_I64: OpcodeVOPC = OpcodeVOPC(181);
    pub const V_CMPX_NE_I64: OpcodeVOPC = OpcodeVOPC(181);
    pub const V_CMPX_GE_I64: OpcodeVOPC = OpcodeVOPC(182);
    pub const V_CMPX_TRU_I64: OpcodeVOPC = OpcodeVOPC(183);
    pub const V_CMPX_T_I64: OpcodeVOPC = OpcodeVOPC(183);
    pub const V_CMPX_CLASS_F64: OpcodeVOPC = OpcodeVOPC(184);
    pub const V_CMP_F_U32: OpcodeVOPC = OpcodeVOPC(192);
    pub const V_CMP_LT_U32: OpcodeVOPC = OpcodeVOPC(193);
    pub const V_CMP_EQ_U32: OpcodeVOPC = OpcodeVOPC(194);
    pub const V_CMP_LE_U32: OpcodeVOPC = OpcodeVOPC(195);
    pub const V_CMP_GT_U32: OpcodeVOPC = OpcodeVOPC(196);
    pub const V_CMP_LG_U32: OpcodeVOPC = OpcodeVOPC(197);
    pub const V_CMP_NE_U32: OpcodeVOPC = OpcodeVOPC(197);
    pub const V_CMP_GE_U32: OpcodeVOPC = OpcodeVOPC(198);
    pub const V_CMP_TRU_U32: OpcodeVOPC = OpcodeVOPC(199);
    pub const V_CMP_T_U32: OpcodeVOPC = OpcodeVOPC(199);
    pub const V_CMP_F_F16: OpcodeVOPC = OpcodeVOPC(200);
    pub const V_CMP_LT_F16: OpcodeVOPC = OpcodeVOPC(201);
    pub const V_CMP_EQ_F16: OpcodeVOPC = OpcodeVOPC(202);
    pub const V_CMP_LE_F16: OpcodeVOPC = OpcodeVOPC(203);
    pub const V_CMP_GT_F16: OpcodeVOPC = OpcodeVOPC(204);
    pub const V_CMP_LG_F16: OpcodeVOPC = OpcodeVOPC(205);
    pub const V_CMP_GE_F16: OpcodeVOPC = OpcodeVOPC(206);
    pub const V_CMP_O_F16: OpcodeVOPC = OpcodeVOPC(207);
    pub const V_CMPX_F_U32: OpcodeVOPC = OpcodeVOPC(208);
    pub const V_CMPX_LT_U32: OpcodeVOPC = OpcodeVOPC(209);
    pub const V_CMPX_EQ_U32: OpcodeVOPC = OpcodeVOPC(210);
    pub const V_CMPX_LE_U32: OpcodeVOPC = OpcodeVOPC(211);
    pub const V_CMPX_GT_U32: OpcodeVOPC = OpcodeVOPC(212);
    pub const V_CMPX_LG_U32: OpcodeVOPC = OpcodeVOPC(213);
    pub const V_CMPX_NE_U32: OpcodeVOPC = OpcodeVOPC(213);
    pub const V_CMPX_GE_U32: OpcodeVOPC = OpcodeVOPC(214);
    pub const V_CMPX_TRU_U32: OpcodeVOPC = OpcodeVOPC(215);
    pub const V_CMPX_T_U32: OpcodeVOPC = OpcodeVOPC(215);
    pub const V_CMP_F_U64: OpcodeVOPC = OpcodeVOPC(224);
    pub const V_CMP_LT_U64: OpcodeVOPC = OpcodeVOPC(225);
    pub const V_CMP_EQ_U64: OpcodeVOPC = OpcodeVOPC(226);
    pub const V_CMP_LE_U64: OpcodeVOPC = OpcodeVOPC(227);
    pub const V_CMP_GT_U64: OpcodeVOPC = OpcodeVOPC(228);
    pub const V_CMP_LG_U64: OpcodeVOPC = OpcodeVOPC(229);
    pub const V_CMP_NE_U64: OpcodeVOPC = OpcodeVOPC(229);
    pub const V_CMP_GE_U64: OpcodeVOPC = OpcodeVOPC(230);
    pub const V_CMP_TRU_U64: OpcodeVOPC = OpcodeVOPC(231);
    pub const V_CMP_T_U64: OpcodeVOPC = OpcodeVOPC(231);
    pub const V_CMPX_F_U64: OpcodeVOPC = OpcodeVOPC(240);
    pub const V_CMPX_LT_U64: OpcodeVOPC = OpcodeVOPC(241);
    pub const V_CMPX_EQ_U64: OpcodeVOPC = OpcodeVOPC(242);
    pub const V_CMPX_LE_U64: OpcodeVOPC = OpcodeVOPC(243);
    pub const V_CMPX_GT_U64: OpcodeVOPC = OpcodeVOPC(244);
    pub const V_CMPX_LG_U64: OpcodeVOPC = OpcodeVOPC(245);
    pub const V_CMPX_NE_U64: OpcodeVOPC = OpcodeVOPC(245);
    pub const V_CMPX_GE_U64: OpcodeVOPC = OpcodeVOPC(246);
    pub const V_CMPX_TRU_U64: OpcodeVOPC = OpcodeVOPC(247);
    pub const V_CMPX_T_U64: OpcodeVOPC = OpcodeVOPC(247);
    pub const OP_RANGE_VOPC: OpcodeVOPC = OpcodeVOPC(OpcodeVOPC::V_CMPX_T_U64.0 + 1);
}

/// `Shader::Gcn::OpcodeVINTRP`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeVINTRP(pub u32);

impl OpcodeVINTRP {
    pub const V_INTERP_P1_F32: OpcodeVINTRP = OpcodeVINTRP(0);
    pub const V_INTERP_P2_F32: OpcodeVINTRP = OpcodeVINTRP(1);
    pub const V_INTERP_MOV_F32: OpcodeVINTRP = OpcodeVINTRP(2);
    pub const OP_RANGE_VINTRP: OpcodeVINTRP = OpcodeVINTRP(OpcodeVINTRP::V_INTERP_MOV_F32.0 + 1);
}

/// `Shader::Gcn::OpcodeDS`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeDS(pub u32);

impl OpcodeDS {
    pub const DS_ADD_U32: OpcodeDS = OpcodeDS(0);
    pub const DS_SUB_U32: OpcodeDS = OpcodeDS(1);
    pub const DS_RSUB_U32: OpcodeDS = OpcodeDS(2);
    pub const DS_INC_U32: OpcodeDS = OpcodeDS(3);
    pub const DS_DEC_U32: OpcodeDS = OpcodeDS(4);
    pub const DS_MIN_I32: OpcodeDS = OpcodeDS(5);
    pub const DS_MAX_I32: OpcodeDS = OpcodeDS(6);
    pub const DS_MIN_U32: OpcodeDS = OpcodeDS(7);
    pub const DS_MAX_U32: OpcodeDS = OpcodeDS(8);
    pub const DS_AND_B32: OpcodeDS = OpcodeDS(9);
    pub const DS_OR_B32: OpcodeDS = OpcodeDS(10);
    pub const DS_XOR_B32: OpcodeDS = OpcodeDS(11);
    pub const DS_MSKOR_B32: OpcodeDS = OpcodeDS(12);
    pub const DS_WRITE_B32: OpcodeDS = OpcodeDS(13);
    pub const DS_WRITE2_B32: OpcodeDS = OpcodeDS(14);
    pub const DS_WRITE2ST64_B32: OpcodeDS = OpcodeDS(15);
    pub const DS_CMPST_B32: OpcodeDS = OpcodeDS(16);
    pub const DS_CMPST_F32: OpcodeDS = OpcodeDS(17);
    pub const DS_MIN_F32: OpcodeDS = OpcodeDS(18);
    pub const DS_MAX_F32: OpcodeDS = OpcodeDS(19);
    pub const DS_NOP: OpcodeDS = OpcodeDS(20);
    pub const DS_GWS_SEMA_RELEASE_ALL: OpcodeDS = OpcodeDS(24);
    pub const DS_GWS_INIT: OpcodeDS = OpcodeDS(25);
    pub const DS_GWS_SEMA_V: OpcodeDS = OpcodeDS(26);
    pub const DS_GWS_SEMA_BR: OpcodeDS = OpcodeDS(27);
    pub const DS_GWS_SEMA_P: OpcodeDS = OpcodeDS(28);
    pub const DS_GWS_BARRIER: OpcodeDS = OpcodeDS(29);
    pub const DS_WRITE_B8: OpcodeDS = OpcodeDS(30);
    pub const DS_WRITE_B16: OpcodeDS = OpcodeDS(31);
    pub const DS_ADD_RTN_U32: OpcodeDS = OpcodeDS(32);
    pub const DS_SUB_RTN_U32: OpcodeDS = OpcodeDS(33);
    pub const DS_RSUB_RTN_U32: OpcodeDS = OpcodeDS(34);
    pub const DS_INC_RTN_U32: OpcodeDS = OpcodeDS(35);
    pub const DS_DEC_RTN_U32: OpcodeDS = OpcodeDS(36);
    pub const DS_MIN_RTN_I32: OpcodeDS = OpcodeDS(37);
    pub const DS_MAX_RTN_I32: OpcodeDS = OpcodeDS(38);
    pub const DS_MIN_RTN_U32: OpcodeDS = OpcodeDS(39);
    pub const DS_MAX_RTN_U32: OpcodeDS = OpcodeDS(40);
    pub const DS_AND_RTN_B32: OpcodeDS = OpcodeDS(41);
    pub const DS_OR_RTN_B32: OpcodeDS = OpcodeDS(42);
    pub const DS_XOR_RTN_B32: OpcodeDS = OpcodeDS(43);
    pub const DS_MSKOR_RTN_B32: OpcodeDS = OpcodeDS(44);
    pub const DS_WRXCHG_RTN_B32: OpcodeDS = OpcodeDS(45);
    pub const DS_WRXCHG2_RTN_B32: OpcodeDS = OpcodeDS(46);
    pub const DS_WRXCHG2ST64_RTN_B32: OpcodeDS = OpcodeDS(47);
    pub const DS_CMPST_RTN_B32: OpcodeDS = OpcodeDS(48);
    pub const DS_CMPST_RTN_F32: OpcodeDS = OpcodeDS(49);
    pub const DS_MIN_RTN_F32: OpcodeDS = OpcodeDS(50);
    pub const DS_MAX_RTN_F32: OpcodeDS = OpcodeDS(51);
    pub const DS_WRAP_RTN_B32: OpcodeDS = OpcodeDS(52);
    pub const DS_SWIZZLE_B32: OpcodeDS = OpcodeDS(53);
    pub const DS_READ_B32: OpcodeDS = OpcodeDS(54);
    pub const DS_READ2_B32: OpcodeDS = OpcodeDS(55);
    pub const DS_READ2ST64_B32: OpcodeDS = OpcodeDS(56);
    pub const DS_READ_I8: OpcodeDS = OpcodeDS(57);
    pub const DS_READ_U8: OpcodeDS = OpcodeDS(58);
    pub const DS_READ_I16: OpcodeDS = OpcodeDS(59);
    pub const DS_READ_U16: OpcodeDS = OpcodeDS(60);
    pub const DS_CONSUME: OpcodeDS = OpcodeDS(61);
    pub const DS_APPEND: OpcodeDS = OpcodeDS(62);
    pub const DS_ORDERED_COUNT: OpcodeDS = OpcodeDS(63);
    pub const DS_ADD_U64: OpcodeDS = OpcodeDS(64);
    pub const DS_SUB_U64: OpcodeDS = OpcodeDS(65);
    pub const DS_RSUB_U64: OpcodeDS = OpcodeDS(66);
    pub const DS_INC_U64: OpcodeDS = OpcodeDS(67);
    pub const DS_DEC_U64: OpcodeDS = OpcodeDS(68);
    pub const DS_MIN_I64: OpcodeDS = OpcodeDS(69);
    pub const DS_MAX_I64: OpcodeDS = OpcodeDS(70);
    pub const DS_MIN_U64: OpcodeDS = OpcodeDS(71);
    pub const DS_MAX_U64: OpcodeDS = OpcodeDS(72);
    pub const DS_AND_B64: OpcodeDS = OpcodeDS(73);
    pub const DS_OR_B64: OpcodeDS = OpcodeDS(74);
    pub const DS_XOR_B64: OpcodeDS = OpcodeDS(75);
    pub const DS_MSKOR_B64: OpcodeDS = OpcodeDS(76);
    pub const DS_WRITE_B64: OpcodeDS = OpcodeDS(77);
    pub const DS_WRITE2_B64: OpcodeDS = OpcodeDS(78);
    pub const DS_WRITE2ST64_B64: OpcodeDS = OpcodeDS(79);
    pub const DS_CMPST_B64: OpcodeDS = OpcodeDS(80);
    pub const DS_CMPST_F64: OpcodeDS = OpcodeDS(81);
    pub const DS_MIN_F64: OpcodeDS = OpcodeDS(82);
    pub const DS_MAX_F64: OpcodeDS = OpcodeDS(83);
    pub const DS_ADD_RTN_U64: OpcodeDS = OpcodeDS(96);
    pub const DS_SUB_RTN_U64: OpcodeDS = OpcodeDS(97);
    pub const DS_RSUB_RTN_U64: OpcodeDS = OpcodeDS(98);
    pub const DS_INC_RTN_U64: OpcodeDS = OpcodeDS(99);
    pub const DS_DEC_RTN_U64: OpcodeDS = OpcodeDS(100);
    pub const DS_MIN_RTN_I64: OpcodeDS = OpcodeDS(101);
    pub const DS_MAX_RTN_I64: OpcodeDS = OpcodeDS(102);
    pub const DS_MIN_RTN_U64: OpcodeDS = OpcodeDS(103);
    pub const DS_MAX_RTN_U64: OpcodeDS = OpcodeDS(104);
    pub const DS_AND_RTN_B64: OpcodeDS = OpcodeDS(105);
    pub const DS_OR_RTN_B64: OpcodeDS = OpcodeDS(106);
    pub const DS_XOR_RTN_B64: OpcodeDS = OpcodeDS(107);
    pub const DS_MSKOR_RTN_B64: OpcodeDS = OpcodeDS(108);
    pub const DS_WRXCHG_RTN_B64: OpcodeDS = OpcodeDS(109);
    pub const DS_WRXCHG2_RTN_B64: OpcodeDS = OpcodeDS(110);
    pub const DS_WRXCHG2ST64_RTN_B64: OpcodeDS = OpcodeDS(111);
    pub const DS_CMPST_RTN_B64: OpcodeDS = OpcodeDS(112);
    pub const DS_CMPST_RTN_F64: OpcodeDS = OpcodeDS(113);
    pub const DS_MIN_RTN_F64: OpcodeDS = OpcodeDS(114);
    pub const DS_MAX_RTN_F64: OpcodeDS = OpcodeDS(115);
    pub const DS_READ_B64: OpcodeDS = OpcodeDS(118);
    pub const DS_READ2_B64: OpcodeDS = OpcodeDS(119);
    pub const DS_READ2ST64_B64: OpcodeDS = OpcodeDS(120);
    pub const DS_CONDXCHG32_RTN_B64: OpcodeDS = OpcodeDS(126);
    pub const DS_ADD_SRC2_U32: OpcodeDS = OpcodeDS(128);
    pub const DS_SUB_SRC2_U32: OpcodeDS = OpcodeDS(129);
    pub const DS_RSUB_SRC2_U32: OpcodeDS = OpcodeDS(130);
    pub const DS_INC_SRC2_U32: OpcodeDS = OpcodeDS(131);
    pub const DS_DEC_SRC2_U32: OpcodeDS = OpcodeDS(132);
    pub const DS_MIN_SRC2_I32: OpcodeDS = OpcodeDS(133);
    pub const DS_MAX_SRC2_I32: OpcodeDS = OpcodeDS(134);
    pub const DS_MIN_SRC2_U32: OpcodeDS = OpcodeDS(135);
    pub const DS_MAX_SRC2_U32: OpcodeDS = OpcodeDS(136);
    pub const DS_AND_SRC2_B32: OpcodeDS = OpcodeDS(137);
    pub const DS_OR_SRC2_B32: OpcodeDS = OpcodeDS(138);
    pub const DS_XOR_SRC2_B32: OpcodeDS = OpcodeDS(139);
    pub const DS_WRITE_SRC2_B32: OpcodeDS = OpcodeDS(141);
    pub const DS_MIN_SRC2_F32: OpcodeDS = OpcodeDS(146);
    pub const DS_MAX_SRC2_F32: OpcodeDS = OpcodeDS(147);
    pub const DS_ADD_SRC2_U64: OpcodeDS = OpcodeDS(192);
    pub const DS_SUB_SRC2_U64: OpcodeDS = OpcodeDS(193);
    pub const DS_RSUB_SRC2_U64: OpcodeDS = OpcodeDS(194);
    pub const DS_INC_SRC2_U64: OpcodeDS = OpcodeDS(195);
    pub const DS_DEC_SRC2_U64: OpcodeDS = OpcodeDS(196);
    pub const DS_MIN_SRC2_I64: OpcodeDS = OpcodeDS(197);
    pub const DS_MAX_SRC2_I64: OpcodeDS = OpcodeDS(198);
    pub const DS_MIN_SRC2_U64: OpcodeDS = OpcodeDS(199);
    pub const DS_MAX_SRC2_U64: OpcodeDS = OpcodeDS(200);
    pub const DS_AND_SRC2_B64: OpcodeDS = OpcodeDS(201);
    pub const DS_OR_SRC2_B64: OpcodeDS = OpcodeDS(202);
    pub const DS_XOR_SRC2_B64: OpcodeDS = OpcodeDS(203);
    pub const DS_WRITE_SRC2_B64: OpcodeDS = OpcodeDS(205);
    pub const DS_MIN_SRC2_F64: OpcodeDS = OpcodeDS(210);
    pub const DS_MAX_SRC2_F64: OpcodeDS = OpcodeDS(211);
    pub const DS_WRITE_B96: OpcodeDS = OpcodeDS(222);
    pub const DS_WRITE_B128: OpcodeDS = OpcodeDS(223);
    pub const DS_CONDXCHG32_RTN_B128: OpcodeDS = OpcodeDS(253);
    pub const DS_READ_B96: OpcodeDS = OpcodeDS(254);
    pub const DS_READ_B128: OpcodeDS = OpcodeDS(255);
    pub const OP_RANGE_DS: OpcodeDS = OpcodeDS(OpcodeDS::DS_READ_B128.0 + 1);
}

/// `Shader::Gcn::OpcodeMUBUF`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeMUBUF(pub u32);

impl OpcodeMUBUF {
    pub const BUFFER_LOAD_FORMAT_X: OpcodeMUBUF = OpcodeMUBUF(0);
    pub const BUFFER_LOAD_FORMAT_XY: OpcodeMUBUF = OpcodeMUBUF(1);
    pub const BUFFER_LOAD_FORMAT_XYZ: OpcodeMUBUF = OpcodeMUBUF(2);
    pub const BUFFER_LOAD_FORMAT_XYZW: OpcodeMUBUF = OpcodeMUBUF(3);
    pub const BUFFER_STORE_FORMAT_X: OpcodeMUBUF = OpcodeMUBUF(4);
    pub const BUFFER_STORE_FORMAT_XY: OpcodeMUBUF = OpcodeMUBUF(5);
    pub const BUFFER_STORE_FORMAT_XYZ: OpcodeMUBUF = OpcodeMUBUF(6);
    pub const BUFFER_STORE_FORMAT_XYZW: OpcodeMUBUF = OpcodeMUBUF(7);
    pub const BUFFER_LOAD_UBYTE: OpcodeMUBUF = OpcodeMUBUF(8);
    pub const BUFFER_LOAD_SBYTE: OpcodeMUBUF = OpcodeMUBUF(9);
    pub const BUFFER_LOAD_USHORT: OpcodeMUBUF = OpcodeMUBUF(10);
    pub const BUFFER_LOAD_SSHORT: OpcodeMUBUF = OpcodeMUBUF(11);
    pub const BUFFER_LOAD_DWORD: OpcodeMUBUF = OpcodeMUBUF(12);
    pub const BUFFER_LOAD_DWORDX2: OpcodeMUBUF = OpcodeMUBUF(13);
    pub const BUFFER_LOAD_DWORDX4: OpcodeMUBUF = OpcodeMUBUF(14);
    pub const BUFFER_LOAD_DWORDX3: OpcodeMUBUF = OpcodeMUBUF(15);
    pub const BUFFER_STORE_BYTE: OpcodeMUBUF = OpcodeMUBUF(24);
    pub const BUFFER_STORE_SHORT: OpcodeMUBUF = OpcodeMUBUF(26);
    pub const BUFFER_STORE_DWORD: OpcodeMUBUF = OpcodeMUBUF(28);
    pub const BUFFER_STORE_DWORDX2: OpcodeMUBUF = OpcodeMUBUF(29);
    pub const BUFFER_STORE_DWORDX4: OpcodeMUBUF = OpcodeMUBUF(30);
    pub const BUFFER_STORE_DWORDX3: OpcodeMUBUF = OpcodeMUBUF(31);
    pub const BUFFER_ATOMIC_SWAP: OpcodeMUBUF = OpcodeMUBUF(48);
    pub const BUFFER_ATOMIC_CMPSWAP: OpcodeMUBUF = OpcodeMUBUF(49);
    pub const BUFFER_ATOMIC_ADD: OpcodeMUBUF = OpcodeMUBUF(50);
    pub const BUFFER_ATOMIC_SUB: OpcodeMUBUF = OpcodeMUBUF(51);
    pub const BUFFER_ATOMIC_SMIN: OpcodeMUBUF = OpcodeMUBUF(53);
    pub const BUFFER_ATOMIC_UMIN: OpcodeMUBUF = OpcodeMUBUF(54);
    pub const BUFFER_ATOMIC_SMAX: OpcodeMUBUF = OpcodeMUBUF(55);
    pub const BUFFER_ATOMIC_UMAX: OpcodeMUBUF = OpcodeMUBUF(56);
    pub const BUFFER_ATOMIC_AND: OpcodeMUBUF = OpcodeMUBUF(57);
    pub const BUFFER_ATOMIC_OR: OpcodeMUBUF = OpcodeMUBUF(58);
    pub const BUFFER_ATOMIC_XOR: OpcodeMUBUF = OpcodeMUBUF(59);
    pub const BUFFER_ATOMIC_INC: OpcodeMUBUF = OpcodeMUBUF(60);
    pub const BUFFER_ATOMIC_DEC: OpcodeMUBUF = OpcodeMUBUF(61);
    pub const BUFFER_ATOMIC_FCMPSWAP: OpcodeMUBUF = OpcodeMUBUF(62);
    pub const BUFFER_ATOMIC_FMIN: OpcodeMUBUF = OpcodeMUBUF(63);
    pub const BUFFER_ATOMIC_FMAX: OpcodeMUBUF = OpcodeMUBUF(64);
    pub const BUFFER_ATOMIC_SWAP_X2: OpcodeMUBUF = OpcodeMUBUF(80);
    pub const BUFFER_ATOMIC_CMPSWAP_X2: OpcodeMUBUF = OpcodeMUBUF(81);
    pub const BUFFER_ATOMIC_ADD_X2: OpcodeMUBUF = OpcodeMUBUF(82);
    pub const BUFFER_ATOMIC_SUB_X2: OpcodeMUBUF = OpcodeMUBUF(83);
    pub const BUFFER_ATOMIC_SMIN_X2: OpcodeMUBUF = OpcodeMUBUF(85);
    pub const BUFFER_ATOMIC_UMIN_X2: OpcodeMUBUF = OpcodeMUBUF(86);
    pub const BUFFER_ATOMIC_SMAX_X2: OpcodeMUBUF = OpcodeMUBUF(87);
    pub const BUFFER_ATOMIC_UMAX_X2: OpcodeMUBUF = OpcodeMUBUF(88);
    pub const BUFFER_ATOMIC_AND_X2: OpcodeMUBUF = OpcodeMUBUF(89);
    pub const BUFFER_ATOMIC_OR_X2: OpcodeMUBUF = OpcodeMUBUF(90);
    pub const BUFFER_ATOMIC_XOR_X2: OpcodeMUBUF = OpcodeMUBUF(91);
    pub const BUFFER_ATOMIC_INC_X2: OpcodeMUBUF = OpcodeMUBUF(92);
    pub const BUFFER_ATOMIC_DEC_X2: OpcodeMUBUF = OpcodeMUBUF(93);
    pub const BUFFER_ATOMIC_FCMPSWAP_X2: OpcodeMUBUF = OpcodeMUBUF(94);
    pub const BUFFER_ATOMIC_FMIN_X2: OpcodeMUBUF = OpcodeMUBUF(95);
    pub const BUFFER_ATOMIC_FMAX_X2: OpcodeMUBUF = OpcodeMUBUF(96);
    pub const BUFFER_WBINVL1_SC: OpcodeMUBUF = OpcodeMUBUF(112);
    pub const BUFFER_WBINVL1: OpcodeMUBUF = OpcodeMUBUF(113);
    pub const OP_RANGE_MUBUF: OpcodeMUBUF = OpcodeMUBUF(OpcodeMUBUF::BUFFER_WBINVL1.0 + 1);
}

/// `Shader::Gcn::OpcodeMTBUF`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeMTBUF(pub u32);

impl OpcodeMTBUF {
    pub const TBUFFER_LOAD_FORMAT_X: OpcodeMTBUF = OpcodeMTBUF(0);
    pub const TBUFFER_LOAD_FORMAT_XY: OpcodeMTBUF = OpcodeMTBUF(1);
    pub const TBUFFER_LOAD_FORMAT_XYZ: OpcodeMTBUF = OpcodeMTBUF(2);
    pub const TBUFFER_LOAD_FORMAT_XYZW: OpcodeMTBUF = OpcodeMTBUF(3);
    pub const TBUFFER_STORE_FORMAT_X: OpcodeMTBUF = OpcodeMTBUF(4);
    pub const TBUFFER_STORE_FORMAT_XY: OpcodeMTBUF = OpcodeMTBUF(5);
    pub const TBUFFER_STORE_FORMAT_XYZ: OpcodeMTBUF = OpcodeMTBUF(6);
    pub const TBUFFER_STORE_FORMAT_XYZW: OpcodeMTBUF = OpcodeMTBUF(7);
    pub const OP_RANGE_MTBUF: OpcodeMTBUF =
        OpcodeMTBUF(OpcodeMTBUF::TBUFFER_STORE_FORMAT_XYZW.0 + 1);
}

/// `Shader::Gcn::OpcodeMIMG`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeMIMG(pub u32);

impl OpcodeMIMG {
    pub const IMAGE_LOAD: OpcodeMIMG = OpcodeMIMG(0);
    pub const IMAGE_LOAD_MIP: OpcodeMIMG = OpcodeMIMG(1);
    pub const IMAGE_LOAD_PCK: OpcodeMIMG = OpcodeMIMG(2);
    pub const IMAGE_LOAD_PCK_SGN: OpcodeMIMG = OpcodeMIMG(3);
    pub const IMAGE_LOAD_MIP_PCK: OpcodeMIMG = OpcodeMIMG(4);
    pub const IMAGE_LOAD_MIP_PCK_SGN: OpcodeMIMG = OpcodeMIMG(5);
    pub const IMAGE_STORE: OpcodeMIMG = OpcodeMIMG(8);
    pub const IMAGE_STORE_MIP: OpcodeMIMG = OpcodeMIMG(9);
    pub const IMAGE_STORE_PCK: OpcodeMIMG = OpcodeMIMG(10);
    pub const IMAGE_STORE_MIP_PCK: OpcodeMIMG = OpcodeMIMG(11);
    pub const IMAGE_GET_RESINFO: OpcodeMIMG = OpcodeMIMG(14);
    pub const IMAGE_ATOMIC_SWAP: OpcodeMIMG = OpcodeMIMG(15);
    pub const IMAGE_ATOMIC_CMPSWAP: OpcodeMIMG = OpcodeMIMG(16);
    pub const IMAGE_ATOMIC_ADD: OpcodeMIMG = OpcodeMIMG(17);
    pub const IMAGE_ATOMIC_SUB: OpcodeMIMG = OpcodeMIMG(18);
    pub const IMAGE_ATOMIC_SMIN: OpcodeMIMG = OpcodeMIMG(20);
    pub const IMAGE_ATOMIC_UMIN: OpcodeMIMG = OpcodeMIMG(21);
    pub const IMAGE_ATOMIC_SMAX: OpcodeMIMG = OpcodeMIMG(22);
    pub const IMAGE_ATOMIC_UMAX: OpcodeMIMG = OpcodeMIMG(23);
    pub const IMAGE_ATOMIC_AND: OpcodeMIMG = OpcodeMIMG(24);
    pub const IMAGE_ATOMIC_OR: OpcodeMIMG = OpcodeMIMG(25);
    pub const IMAGE_ATOMIC_XOR: OpcodeMIMG = OpcodeMIMG(26);
    pub const IMAGE_ATOMIC_INC: OpcodeMIMG = OpcodeMIMG(27);
    pub const IMAGE_ATOMIC_DEC: OpcodeMIMG = OpcodeMIMG(28);
    pub const IMAGE_ATOMIC_FCMPSWAP: OpcodeMIMG = OpcodeMIMG(29);
    pub const IMAGE_ATOMIC_FMIN: OpcodeMIMG = OpcodeMIMG(30);
    pub const IMAGE_ATOMIC_FMAX: OpcodeMIMG = OpcodeMIMG(31);
    pub const IMAGE_SAMPLE: OpcodeMIMG = OpcodeMIMG(32);
    pub const IMAGE_SAMPLE_CL: OpcodeMIMG = OpcodeMIMG(33);
    pub const IMAGE_SAMPLE_D: OpcodeMIMG = OpcodeMIMG(34);
    pub const IMAGE_SAMPLE_D_CL: OpcodeMIMG = OpcodeMIMG(35);
    pub const IMAGE_SAMPLE_L: OpcodeMIMG = OpcodeMIMG(36);
    pub const IMAGE_SAMPLE_B: OpcodeMIMG = OpcodeMIMG(37);
    pub const IMAGE_SAMPLE_B_CL: OpcodeMIMG = OpcodeMIMG(38);
    pub const IMAGE_SAMPLE_LZ: OpcodeMIMG = OpcodeMIMG(39);
    pub const IMAGE_SAMPLE_C: OpcodeMIMG = OpcodeMIMG(40);
    pub const IMAGE_SAMPLE_C_CL: OpcodeMIMG = OpcodeMIMG(41);
    pub const IMAGE_SAMPLE_C_D: OpcodeMIMG = OpcodeMIMG(42);
    pub const IMAGE_SAMPLE_C_D_CL: OpcodeMIMG = OpcodeMIMG(43);
    pub const IMAGE_SAMPLE_C_L: OpcodeMIMG = OpcodeMIMG(44);
    pub const IMAGE_SAMPLE_C_B: OpcodeMIMG = OpcodeMIMG(45);
    pub const IMAGE_SAMPLE_C_B_CL: OpcodeMIMG = OpcodeMIMG(46);
    pub const IMAGE_SAMPLE_C_LZ: OpcodeMIMG = OpcodeMIMG(47);
    pub const IMAGE_SAMPLE_O: OpcodeMIMG = OpcodeMIMG(48);
    pub const IMAGE_SAMPLE_CL_O: OpcodeMIMG = OpcodeMIMG(49);
    pub const IMAGE_SAMPLE_D_O: OpcodeMIMG = OpcodeMIMG(50);
    pub const IMAGE_SAMPLE_D_CL_O: OpcodeMIMG = OpcodeMIMG(51);
    pub const IMAGE_SAMPLE_L_O: OpcodeMIMG = OpcodeMIMG(52);
    pub const IMAGE_SAMPLE_B_O: OpcodeMIMG = OpcodeMIMG(53);
    pub const IMAGE_SAMPLE_B_CL_O: OpcodeMIMG = OpcodeMIMG(54);
    pub const IMAGE_SAMPLE_LZ_O: OpcodeMIMG = OpcodeMIMG(55);
    pub const IMAGE_SAMPLE_C_O: OpcodeMIMG = OpcodeMIMG(56);
    pub const IMAGE_SAMPLE_C_CL_O: OpcodeMIMG = OpcodeMIMG(57);
    pub const IMAGE_SAMPLE_C_D_O: OpcodeMIMG = OpcodeMIMG(58);
    pub const IMAGE_SAMPLE_C_D_CL_O: OpcodeMIMG = OpcodeMIMG(59);
    pub const IMAGE_SAMPLE_C_L_O: OpcodeMIMG = OpcodeMIMG(60);
    pub const IMAGE_SAMPLE_C_B_O: OpcodeMIMG = OpcodeMIMG(61);
    pub const IMAGE_SAMPLE_C_B_CL_O: OpcodeMIMG = OpcodeMIMG(62);
    pub const IMAGE_SAMPLE_C_LZ_O: OpcodeMIMG = OpcodeMIMG(63);
    pub const IMAGE_GATHER4: OpcodeMIMG = OpcodeMIMG(64);
    pub const IMAGE_GATHER4_CL: OpcodeMIMG = OpcodeMIMG(65);
    pub const IMAGE_GATHER4_L: OpcodeMIMG = OpcodeMIMG(68);
    pub const IMAGE_GATHER4_B: OpcodeMIMG = OpcodeMIMG(69);
    pub const IMAGE_GATHER4_B_CL: OpcodeMIMG = OpcodeMIMG(70);
    pub const IMAGE_GATHER4_LZ: OpcodeMIMG = OpcodeMIMG(71);
    pub const IMAGE_GATHER4_C: OpcodeMIMG = OpcodeMIMG(72);
    pub const IMAGE_GATHER4_C_CL: OpcodeMIMG = OpcodeMIMG(73);
    pub const IMAGE_GATHER4_C_L: OpcodeMIMG = OpcodeMIMG(76);
    pub const IMAGE_GATHER4_C_B: OpcodeMIMG = OpcodeMIMG(77);
    pub const IMAGE_GATHER4_C_B_CL: OpcodeMIMG = OpcodeMIMG(78);
    pub const IMAGE_GATHER4_C_LZ: OpcodeMIMG = OpcodeMIMG(79);
    pub const IMAGE_GATHER4_O: OpcodeMIMG = OpcodeMIMG(80);
    pub const IMAGE_GATHER4_CL_O: OpcodeMIMG = OpcodeMIMG(81);
    pub const IMAGE_GATHER4_L_O: OpcodeMIMG = OpcodeMIMG(84);
    pub const IMAGE_GATHER4_B_O: OpcodeMIMG = OpcodeMIMG(85);
    pub const IMAGE_GATHER4_B_CL_O: OpcodeMIMG = OpcodeMIMG(86);
    pub const IMAGE_GATHER4_LZ_O: OpcodeMIMG = OpcodeMIMG(87);
    pub const IMAGE_GATHER4_C_O: OpcodeMIMG = OpcodeMIMG(88);
    pub const IMAGE_GATHER4_C_CL_O: OpcodeMIMG = OpcodeMIMG(89);
    pub const IMAGE_GATHER4_C_L_O: OpcodeMIMG = OpcodeMIMG(92);
    pub const IMAGE_GATHER4_C_B_O: OpcodeMIMG = OpcodeMIMG(93);
    pub const IMAGE_GATHER4_C_B_CL_O: OpcodeMIMG = OpcodeMIMG(94);
    pub const IMAGE_GATHER4_C_LZ_O: OpcodeMIMG = OpcodeMIMG(95);
    pub const IMAGE_GET_LOD: OpcodeMIMG = OpcodeMIMG(96);
    pub const IMAGE_SAMPLE_CD: OpcodeMIMG = OpcodeMIMG(104);
    pub const IMAGE_SAMPLE_CD_CL: OpcodeMIMG = OpcodeMIMG(105);
    pub const IMAGE_SAMPLE_C_CD: OpcodeMIMG = OpcodeMIMG(106);
    pub const IMAGE_SAMPLE_C_CD_CL: OpcodeMIMG = OpcodeMIMG(107);
    pub const IMAGE_SAMPLE_CD_O: OpcodeMIMG = OpcodeMIMG(108);
    pub const IMAGE_SAMPLE_CD_CL_O: OpcodeMIMG = OpcodeMIMG(109);
    pub const IMAGE_SAMPLE_C_CD_O: OpcodeMIMG = OpcodeMIMG(110);
    pub const IMAGE_SAMPLE_C_CD_CL_O: OpcodeMIMG = OpcodeMIMG(111);
    pub const OP_RANGE_MIMG: OpcodeMIMG = OpcodeMIMG(OpcodeMIMG::IMAGE_SAMPLE_C_CD_CL_O.0 + 1);
}

/// `Shader::Gcn::OpcodeEXP`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeEXP(pub u32);

impl OpcodeEXP {
    pub const EXP: OpcodeEXP = OpcodeEXP(0);
    pub const OP_RANGE_EXP: OpcodeEXP = OpcodeEXP(OpcodeEXP::EXP.0 + 1);
}

/// `Shader::Gcn::OpMapVOP3VOPX`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpMapVOP3VOPX(pub u32);

impl OpMapVOP3VOPX {
    pub const VOP3_TO_VOPC: OpMapVOP3VOPX = OpMapVOP3VOPX(0);
    pub const VOP3_TO_VOP2: OpMapVOP3VOPX = OpMapVOP3VOPX(256);
    pub const VOP3_TO_VOP1: OpMapVOP3VOPX = OpMapVOP3VOPX(384);
}

/// `Shader::Gcn::OpcodeMap`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OpcodeMap(pub u32);

impl OpcodeMap {
    pub const OP_MAP_SOP2: OpcodeMap = OpcodeMap(0);
    pub const OP_MAP_SOPK: OpcodeMap =
        OpcodeMap(OpcodeMap::OP_MAP_SOP2.0 + OpcodeSOP2::OP_RANGE_SOP2.0);
    pub const OP_MAP_SOP1: OpcodeMap =
        OpcodeMap(OpcodeMap::OP_MAP_SOPK.0 + OpcodeSOPK::OP_RANGE_SOPK.0);
    pub const OP_MAP_SOPC: OpcodeMap =
        OpcodeMap(OpcodeMap::OP_MAP_SOP1.0 + OpcodeSOP1::OP_RANGE_SOP1.0);
    pub const OP_MAP_SOPP: OpcodeMap =
        OpcodeMap(OpcodeMap::OP_MAP_SOPC.0 + OpcodeSOPC::OP_RANGE_SOPC.0);
    pub const OP_MAP_VOPC: OpcodeMap =
        OpcodeMap(OpcodeMap::OP_MAP_SOPP.0 + OpcodeSOPP::OP_RANGE_SOPP.0);
    pub const OP_MAP_VOP2: OpcodeMap =
        OpcodeMap(OpcodeMap::OP_MAP_VOPC.0 + OpcodeVOPC::OP_RANGE_VOPC.0);
    pub const OP_MAP_VOP1: OpcodeMap =
        OpcodeMap(OpcodeMap::OP_MAP_VOP2.0 + OpcodeVOP2::OP_RANGE_VOP2.0);
    pub const OP_MAP_VOP3: OpcodeMap =
        OpcodeMap(OpcodeMap::OP_MAP_VOP1.0 + OpcodeVOP1::OP_RANGE_VOP1.0);
    pub const OP_MAP_VINTRP: OpcodeMap =
        OpcodeMap(OpcodeMap::OP_MAP_VOP3.0 + OpcodeVOP3::OP_RANGE_VOP3.0);
    pub const OP_MAP_SMRD: OpcodeMap =
        OpcodeMap(OpcodeMap::OP_MAP_VINTRP.0 + OpcodeVINTRP::OP_RANGE_VINTRP.0);
    pub const OP_MAP_DS: OpcodeMap =
        OpcodeMap(OpcodeMap::OP_MAP_SMRD.0 + OpcodeSMRD::OP_RANGE_SMRD.0);
    pub const OP_MAP_MUBUF: OpcodeMap = OpcodeMap(OpcodeMap::OP_MAP_DS.0 + OpcodeDS::OP_RANGE_DS.0);
    pub const OP_MAP_MTBUF: OpcodeMap =
        OpcodeMap(OpcodeMap::OP_MAP_MUBUF.0 + OpcodeMUBUF::OP_RANGE_MUBUF.0);
    pub const OP_MAP_MIMG: OpcodeMap =
        OpcodeMap(OpcodeMap::OP_MAP_MTBUF.0 + OpcodeMTBUF::OP_RANGE_MTBUF.0);
    pub const OP_MAP_EXP: OpcodeMap =
        OpcodeMap(OpcodeMap::OP_MAP_MIMG.0 + OpcodeMIMG::OP_RANGE_MIMG.0);
    pub const OP_MAP_VOP3P: OpcodeMap =
        OpcodeMap(OpcodeMap::OP_MAP_EXP.0 + OpcodeEXP::OP_RANGE_EXP.0);
}

/// `Shader::Gcn::Opcode`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct Opcode(pub u32);

impl Opcode {
    pub const S_ADD_U32: Opcode = Opcode(0 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_SUB_U32: Opcode = Opcode(1 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_ADD_I32: Opcode = Opcode(2 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_SUB_I32: Opcode = Opcode(3 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_ADDC_U32: Opcode = Opcode(4 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_SUBB_U32: Opcode = Opcode(5 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_MIN_I32: Opcode = Opcode(6 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_MIN_U32: Opcode = Opcode(7 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_MAX_I32: Opcode = Opcode(8 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_MAX_U32: Opcode = Opcode(9 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_CSELECT_B32: Opcode = Opcode(10 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_CSELECT_B64: Opcode = Opcode(11 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_AND_B32: Opcode = Opcode(14 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_AND_B64: Opcode = Opcode(15 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_OR_B32: Opcode = Opcode(16 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_OR_B64: Opcode = Opcode(17 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_XOR_B32: Opcode = Opcode(18 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_XOR_B64: Opcode = Opcode(19 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_ANDN2_B32: Opcode = Opcode(20 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_ANDN2_B64: Opcode = Opcode(21 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_ORN2_B32: Opcode = Opcode(22 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_ORN2_B64: Opcode = Opcode(23 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_NAND_B32: Opcode = Opcode(24 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_NAND_B64: Opcode = Opcode(25 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_NOR_B32: Opcode = Opcode(26 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_NOR_B64: Opcode = Opcode(27 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_XNOR_B32: Opcode = Opcode(28 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_XNOR_B64: Opcode = Opcode(29 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_LSHL_B32: Opcode = Opcode(30 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_LSHL_B64: Opcode = Opcode(31 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_LSHR_B32: Opcode = Opcode(32 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_LSHR_B64: Opcode = Opcode(33 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_ASHR_I32: Opcode = Opcode(34 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_ASHR_I64: Opcode = Opcode(35 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_BFM_B32: Opcode = Opcode(36 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_BFM_B64: Opcode = Opcode(37 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_MUL_I32: Opcode = Opcode(38 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_BFE_U32: Opcode = Opcode(39 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_BFE_I32: Opcode = Opcode(40 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_BFE_U64: Opcode = Opcode(41 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_BFE_I64: Opcode = Opcode(42 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_CBRANCH_G_FORK: Opcode = Opcode(43 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_ABSDIFF_I32: Opcode = Opcode(44 + OpcodeMap::OP_MAP_SOP2.0);
    pub const S_MOVK_I32: Opcode = Opcode(0 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_CMOVK_I32: Opcode = Opcode(2 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_CMPK_EQ_I32: Opcode = Opcode(3 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_CMPK_LG_I32: Opcode = Opcode(4 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_CMPK_GT_I32: Opcode = Opcode(5 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_CMPK_GE_I32: Opcode = Opcode(6 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_CMPK_LT_I32: Opcode = Opcode(7 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_CMPK_LE_I32: Opcode = Opcode(8 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_CMPK_EQ_U32: Opcode = Opcode(9 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_CMPK_LG_U32: Opcode = Opcode(10 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_CMPK_GT_U32: Opcode = Opcode(11 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_CMPK_GE_U32: Opcode = Opcode(12 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_CMPK_LT_U32: Opcode = Opcode(13 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_CMPK_LE_U32: Opcode = Opcode(14 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_ADDK_I32: Opcode = Opcode(15 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_MULK_I32: Opcode = Opcode(16 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_CBRANCH_I_FORK: Opcode = Opcode(17 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_GETREG_B32: Opcode = Opcode(18 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_SETREG_B32: Opcode = Opcode(19 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_GETREG_REGRD_B32: Opcode = Opcode(20 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_SETREG_IMM32_B32: Opcode = Opcode(21 + OpcodeMap::OP_MAP_SOPK.0);
    pub const S_MOV_B32: Opcode = Opcode(3 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_MOV_B64: Opcode = Opcode(4 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_CMOV_B32: Opcode = Opcode(5 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_CMOV_B64: Opcode = Opcode(6 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_NOT_B32: Opcode = Opcode(7 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_NOT_B64: Opcode = Opcode(8 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_WQM_B32: Opcode = Opcode(9 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_WQM_B64: Opcode = Opcode(10 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_BREV_B32: Opcode = Opcode(11 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_BREV_B64: Opcode = Opcode(12 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_BCNT0_I32_B32: Opcode = Opcode(13 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_BCNT0_I32_B64: Opcode = Opcode(14 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_BCNT1_I32_B32: Opcode = Opcode(15 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_BCNT1_I32_B64: Opcode = Opcode(16 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_FF0_I32_B32: Opcode = Opcode(17 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_FF0_I32_B64: Opcode = Opcode(18 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_FF1_I32_B32: Opcode = Opcode(19 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_FF1_I32_B64: Opcode = Opcode(20 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_FLBIT_I32_B32: Opcode = Opcode(21 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_FLBIT_I32_B64: Opcode = Opcode(22 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_FLBIT_I32: Opcode = Opcode(23 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_FLBIT_I32_I64: Opcode = Opcode(24 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_SEXT_I32_I8: Opcode = Opcode(25 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_SEXT_I32_I16: Opcode = Opcode(26 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_BITSET0_B32: Opcode = Opcode(27 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_BITSET0_B64: Opcode = Opcode(28 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_BITSET1_B32: Opcode = Opcode(29 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_BITSET1_B64: Opcode = Opcode(30 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_GETPC_B64: Opcode = Opcode(31 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_SETPC_B64: Opcode = Opcode(32 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_SWAPPC_B64: Opcode = Opcode(33 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_RFE_B64: Opcode = Opcode(34 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_AND_SAVEEXEC_B64: Opcode = Opcode(36 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_OR_SAVEEXEC_B64: Opcode = Opcode(37 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_XOR_SAVEEXEC_B64: Opcode = Opcode(38 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_ANDN2_SAVEEXEC_B64: Opcode = Opcode(39 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_ORN2_SAVEEXEC_B64: Opcode = Opcode(40 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_NAND_SAVEEXEC_B64: Opcode = Opcode(41 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_NOR_SAVEEXEC_B64: Opcode = Opcode(42 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_XNOR_SAVEEXEC_B64: Opcode = Opcode(43 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_QUADMASK_B32: Opcode = Opcode(44 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_QUADMASK_B64: Opcode = Opcode(45 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_MOVRELS_B32: Opcode = Opcode(46 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_MOVRELS_B64: Opcode = Opcode(47 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_MOVRELD_B32: Opcode = Opcode(48 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_MOVRELD_B64: Opcode = Opcode(49 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_CBRANCH_JOIN: Opcode = Opcode(50 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_MOV_REGRD_B32: Opcode = Opcode(51 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_ABS_I32: Opcode = Opcode(52 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_MOV_FED_B32: Opcode = Opcode(53 + OpcodeMap::OP_MAP_SOP1.0);
    pub const S_CMP_EQ_I32: Opcode = Opcode(0 + OpcodeMap::OP_MAP_SOPC.0);
    pub const S_CMP_LG_I32: Opcode = Opcode(1 + OpcodeMap::OP_MAP_SOPC.0);
    pub const S_CMP_GT_I32: Opcode = Opcode(2 + OpcodeMap::OP_MAP_SOPC.0);
    pub const S_CMP_GE_I32: Opcode = Opcode(3 + OpcodeMap::OP_MAP_SOPC.0);
    pub const S_CMP_LT_I32: Opcode = Opcode(4 + OpcodeMap::OP_MAP_SOPC.0);
    pub const S_CMP_LE_I32: Opcode = Opcode(5 + OpcodeMap::OP_MAP_SOPC.0);
    pub const S_CMP_EQ_U32: Opcode = Opcode(6 + OpcodeMap::OP_MAP_SOPC.0);
    pub const S_CMP_LG_U32: Opcode = Opcode(7 + OpcodeMap::OP_MAP_SOPC.0);
    pub const S_CMP_GT_U32: Opcode = Opcode(8 + OpcodeMap::OP_MAP_SOPC.0);
    pub const S_CMP_GE_U32: Opcode = Opcode(9 + OpcodeMap::OP_MAP_SOPC.0);
    pub const S_CMP_LT_U32: Opcode = Opcode(10 + OpcodeMap::OP_MAP_SOPC.0);
    pub const S_CMP_LE_U32: Opcode = Opcode(11 + OpcodeMap::OP_MAP_SOPC.0);
    pub const S_BITCMP0_B32: Opcode = Opcode(12 + OpcodeMap::OP_MAP_SOPC.0);
    pub const S_BITCMP1_B32: Opcode = Opcode(13 + OpcodeMap::OP_MAP_SOPC.0);
    pub const S_BITCMP0_B64: Opcode = Opcode(14 + OpcodeMap::OP_MAP_SOPC.0);
    pub const S_BITCMP1_B64: Opcode = Opcode(15 + OpcodeMap::OP_MAP_SOPC.0);
    pub const S_SETVSKIP: Opcode = Opcode(16 + OpcodeMap::OP_MAP_SOPC.0);
    pub const S_NOP: Opcode = Opcode(0 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_ENDPGM: Opcode = Opcode(1 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_BRANCH: Opcode = Opcode(2 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_CBRANCH_SCC0: Opcode = Opcode(4 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_CBRANCH_SCC1: Opcode = Opcode(5 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_CBRANCH_VCCZ: Opcode = Opcode(6 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_CBRANCH_VCCNZ: Opcode = Opcode(7 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_CBRANCH_EXECZ: Opcode = Opcode(8 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_CBRANCH_EXECNZ: Opcode = Opcode(9 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_BARRIER: Opcode = Opcode(10 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_SETKILL: Opcode = Opcode(11 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_WAITCNT: Opcode = Opcode(12 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_SETHALT: Opcode = Opcode(13 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_SLEEP: Opcode = Opcode(14 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_SETPRIO: Opcode = Opcode(15 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_SENDMSG: Opcode = Opcode(16 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_SENDMSGHALT: Opcode = Opcode(17 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_TRAP: Opcode = Opcode(18 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_ICACHE_INV: Opcode = Opcode(19 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_INCPERFLEVEL: Opcode = Opcode(20 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_DECPERFLEVEL: Opcode = Opcode(21 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_TTRACEDATA: Opcode = Opcode(22 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_CBRANCH_CDBGSYS: Opcode = Opcode(23 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_CBRANCH_CDBGUSER: Opcode = Opcode(24 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_CBRANCH_CDBGSYS_OR_USER: Opcode = Opcode(25 + OpcodeMap::OP_MAP_SOPP.0);
    pub const S_CBRANCH_CDBGSYS_AND_USER: Opcode = Opcode(26 + OpcodeMap::OP_MAP_SOPP.0);
    pub const V_CMP_F_F32: Opcode = Opcode(0 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LT_F32: Opcode = Opcode(1 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_EQ_F32: Opcode = Opcode(2 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LE_F32: Opcode = Opcode(3 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GT_F32: Opcode = Opcode(4 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LG_F32: Opcode = Opcode(5 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GE_F32: Opcode = Opcode(6 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_O_F32: Opcode = Opcode(7 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_U_F32: Opcode = Opcode(8 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NGE_F32: Opcode = Opcode(9 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NLG_F32: Opcode = Opcode(10 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NGT_F32: Opcode = Opcode(11 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NLE_F32: Opcode = Opcode(12 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NEQ_F32: Opcode = Opcode(13 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NLT_F32: Opcode = Opcode(14 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_TRU_F32: Opcode = Opcode(15 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_T_F32: Opcode = Opcode(15 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_F_F32: Opcode = Opcode(16 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_LT_F32: Opcode = Opcode(17 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_EQ_F32: Opcode = Opcode(18 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_LE_F32: Opcode = Opcode(19 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_GT_F32: Opcode = Opcode(20 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_LG_F32: Opcode = Opcode(21 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_GE_F32: Opcode = Opcode(22 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_O_F32: Opcode = Opcode(23 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_U_F32: Opcode = Opcode(24 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_NGE_F32: Opcode = Opcode(25 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_NLG_F32: Opcode = Opcode(26 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_NGT_F32: Opcode = Opcode(27 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_NLE_F32: Opcode = Opcode(28 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_NEQ_F32: Opcode = Opcode(29 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_NLT_F32: Opcode = Opcode(30 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_TRU_F32: Opcode = Opcode(31 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_T_F32: Opcode = Opcode(31 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_F_F64: Opcode = Opcode(32 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LT_F64: Opcode = Opcode(33 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_EQ_F64: Opcode = Opcode(34 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LE_F64: Opcode = Opcode(35 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GT_F64: Opcode = Opcode(36 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LG_F64: Opcode = Opcode(37 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GE_F64: Opcode = Opcode(38 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_O_F64: Opcode = Opcode(39 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_U_F64: Opcode = Opcode(40 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NGE_F64: Opcode = Opcode(41 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NLG_F64: Opcode = Opcode(42 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NGT_F64: Opcode = Opcode(43 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NLE_F64: Opcode = Opcode(44 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NEQ_F64: Opcode = Opcode(45 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NLT_F64: Opcode = Opcode(46 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_TRU_F64: Opcode = Opcode(47 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_T_F64: Opcode = Opcode(47 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_F_F64: Opcode = Opcode(48 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_LT_F64: Opcode = Opcode(49 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_EQ_F64: Opcode = Opcode(50 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_LE_F64: Opcode = Opcode(51 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_GT_F64: Opcode = Opcode(52 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_LG_F64: Opcode = Opcode(53 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_GE_F64: Opcode = Opcode(54 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_O_F64: Opcode = Opcode(55 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_U_F64: Opcode = Opcode(56 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_NGE_F64: Opcode = Opcode(57 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_NLG_F64: Opcode = Opcode(58 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_NGT_F64: Opcode = Opcode(59 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_NLE_F64: Opcode = Opcode(60 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_NEQ_F64: Opcode = Opcode(61 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_NLT_F64: Opcode = Opcode(62 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_TRU_F64: Opcode = Opcode(63 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_T_F64: Opcode = Opcode(63 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_F_F32: Opcode = Opcode(64 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_LT_F32: Opcode = Opcode(65 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_EQ_F32: Opcode = Opcode(66 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_LE_F32: Opcode = Opcode(67 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_GT_F32: Opcode = Opcode(68 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_LG_F32: Opcode = Opcode(69 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_GE_F32: Opcode = Opcode(70 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_O_F32: Opcode = Opcode(71 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_U_F32: Opcode = Opcode(72 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_NGE_F32: Opcode = Opcode(73 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_NLG_F32: Opcode = Opcode(74 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_NGT_F32: Opcode = Opcode(75 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_NLE_F32: Opcode = Opcode(76 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_NEQ_F32: Opcode = Opcode(77 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_NLT_F32: Opcode = Opcode(78 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_TRU_F32: Opcode = Opcode(79 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_T_F32: Opcode = Opcode(79 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_F_F32: Opcode = Opcode(80 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_LT_F32: Opcode = Opcode(81 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_EQ_F32: Opcode = Opcode(82 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_LE_F32: Opcode = Opcode(83 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_GT_F32: Opcode = Opcode(84 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_LG_F32: Opcode = Opcode(85 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_GE_F32: Opcode = Opcode(86 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_O_F32: Opcode = Opcode(87 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_U_F32: Opcode = Opcode(88 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_NGE_F32: Opcode = Opcode(89 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_NLG_F32: Opcode = Opcode(90 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_NGT_F32: Opcode = Opcode(91 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_NLE_F32: Opcode = Opcode(92 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_NEQ_F32: Opcode = Opcode(93 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_NLT_F32: Opcode = Opcode(94 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_TRU_F32: Opcode = Opcode(95 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_T_F32: Opcode = Opcode(95 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_F_F64: Opcode = Opcode(96 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_LT_F64: Opcode = Opcode(97 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_EQ_F64: Opcode = Opcode(98 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_LE_F64: Opcode = Opcode(99 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_GT_F64: Opcode = Opcode(100 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_LG_F64: Opcode = Opcode(101 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_GE_F64: Opcode = Opcode(102 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_O_F64: Opcode = Opcode(103 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_U_F64: Opcode = Opcode(104 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_NGE_F64: Opcode = Opcode(105 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_NLG_F64: Opcode = Opcode(106 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_NGT_F64: Opcode = Opcode(107 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_NLE_F64: Opcode = Opcode(108 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_NEQ_F64: Opcode = Opcode(109 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_NLT_F64: Opcode = Opcode(110 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_TRU_F64: Opcode = Opcode(111 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPS_T_F64: Opcode = Opcode(111 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_F_F64: Opcode = Opcode(112 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_LT_F64: Opcode = Opcode(113 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_EQ_F64: Opcode = Opcode(114 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_LE_F64: Opcode = Opcode(115 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_GT_F64: Opcode = Opcode(116 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_LG_F64: Opcode = Opcode(117 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_GE_F64: Opcode = Opcode(118 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_O_F64: Opcode = Opcode(119 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_U_F64: Opcode = Opcode(120 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_NGE_F64: Opcode = Opcode(121 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_NLG_F64: Opcode = Opcode(122 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_NGT_F64: Opcode = Opcode(123 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_NLE_F64: Opcode = Opcode(124 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_NEQ_F64: Opcode = Opcode(125 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_NLT_F64: Opcode = Opcode(126 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_TRU_F64: Opcode = Opcode(127 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPSX_T_F64: Opcode = Opcode(127 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_F_I32: Opcode = Opcode(128 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LT_I32: Opcode = Opcode(129 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_EQ_I32: Opcode = Opcode(130 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LE_I32: Opcode = Opcode(131 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GT_I32: Opcode = Opcode(132 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NE_I32: Opcode = Opcode(133 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GE_I32: Opcode = Opcode(134 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_TRU_I32: Opcode = Opcode(135 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_T_I32: Opcode = Opcode(135 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_CLASS_F32: Opcode = Opcode(136 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LT_I16: Opcode = Opcode(137 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_EQ_I16: Opcode = Opcode(138 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LE_I16: Opcode = Opcode(139 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GT_I16: Opcode = Opcode(140 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NE_I16: Opcode = Opcode(141 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GE_I16: Opcode = Opcode(142 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_F_I32: Opcode = Opcode(144 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_LT_I32: Opcode = Opcode(145 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_EQ_I32: Opcode = Opcode(146 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_LE_I32: Opcode = Opcode(147 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_GT_I32: Opcode = Opcode(148 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_LG_I32: Opcode = Opcode(149 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_NE_I32: Opcode = Opcode(149 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_GE_I32: Opcode = Opcode(150 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_TRU_I32: Opcode = Opcode(151 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_T_I32: Opcode = Opcode(151 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_CLASS_F32: Opcode = Opcode(152 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_F_I64: Opcode = Opcode(160 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LT_I64: Opcode = Opcode(161 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_EQ_I64: Opcode = Opcode(162 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LE_I64: Opcode = Opcode(163 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GT_I64: Opcode = Opcode(164 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LG_I64: Opcode = Opcode(165 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NE_I64: Opcode = Opcode(165 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GE_I64: Opcode = Opcode(166 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_TRU_I64: Opcode = Opcode(167 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_T_I64: Opcode = Opcode(167 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_CLASS_F64: Opcode = Opcode(168 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LT_U16: Opcode = Opcode(169 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_EQ_U16: Opcode = Opcode(170 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LE_U16: Opcode = Opcode(171 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GT_U16: Opcode = Opcode(172 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NE_U16: Opcode = Opcode(173 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GE_U16: Opcode = Opcode(174 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_F_I64: Opcode = Opcode(176 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_LT_I64: Opcode = Opcode(177 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_EQ_I64: Opcode = Opcode(178 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_LE_I64: Opcode = Opcode(179 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_GT_I64: Opcode = Opcode(180 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_LG_I64: Opcode = Opcode(181 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_NE_I64: Opcode = Opcode(181 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_GE_I64: Opcode = Opcode(182 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_TRU_I64: Opcode = Opcode(183 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_T_I64: Opcode = Opcode(183 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_CLASS_F64: Opcode = Opcode(184 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_F_U32: Opcode = Opcode(192 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LT_U32: Opcode = Opcode(193 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_EQ_U32: Opcode = Opcode(194 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LE_U32: Opcode = Opcode(195 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GT_U32: Opcode = Opcode(196 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NE_U32: Opcode = Opcode(197 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GE_U32: Opcode = Opcode(198 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_TRU_U32: Opcode = Opcode(199 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_T_U32: Opcode = Opcode(199 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_F_F16: Opcode = Opcode(200 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LT_F16: Opcode = Opcode(201 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_EQ_F16: Opcode = Opcode(202 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LE_F16: Opcode = Opcode(203 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GT_F16: Opcode = Opcode(204 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LG_F16: Opcode = Opcode(205 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GE_F16: Opcode = Opcode(206 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_O_F16: Opcode = Opcode(207 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_F_U32: Opcode = Opcode(208 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_LT_U32: Opcode = Opcode(209 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_EQ_U32: Opcode = Opcode(210 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_LE_U32: Opcode = Opcode(211 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_GT_U32: Opcode = Opcode(212 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_NE_U32: Opcode = Opcode(213 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_GE_U32: Opcode = Opcode(214 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_TRU_U32: Opcode = Opcode(215 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_T_U32: Opcode = Opcode(215 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_F_U64: Opcode = Opcode(224 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LT_U64: Opcode = Opcode(225 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_EQ_U64: Opcode = Opcode(226 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LE_U64: Opcode = Opcode(227 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GT_U64: Opcode = Opcode(228 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_LG_U64: Opcode = Opcode(229 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_NE_U64: Opcode = Opcode(229 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_GE_U64: Opcode = Opcode(230 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_TRU_U64: Opcode = Opcode(231 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMP_T_U64: Opcode = Opcode(231 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_F_U64: Opcode = Opcode(240 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_LT_U64: Opcode = Opcode(241 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_EQ_U64: Opcode = Opcode(242 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_LE_U64: Opcode = Opcode(243 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_GT_U64: Opcode = Opcode(244 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_LG_U64: Opcode = Opcode(245 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_NE_U64: Opcode = Opcode(245 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_GE_U64: Opcode = Opcode(246 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_TRU_U64: Opcode = Opcode(247 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CMPX_T_U64: Opcode = Opcode(247 + OpcodeMap::OP_MAP_VOPC.0);
    pub const V_CNDMASK_B32: Opcode = Opcode(0 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_READLANE_B32: Opcode = Opcode(1 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_WRITELANE_B32: Opcode = Opcode(2 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_ADD_F32: Opcode = Opcode(3 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_SUB_F32: Opcode = Opcode(4 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_SUBREV_F32: Opcode = Opcode(5 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MAC_LEGACY_F32: Opcode = Opcode(6 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MUL_LEGACY_F32: Opcode = Opcode(7 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MUL_F32: Opcode = Opcode(8 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MUL_I32_I24: Opcode = Opcode(9 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MUL_HI_I32_I24: Opcode = Opcode(10 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MUL_U32_U24: Opcode = Opcode(11 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MUL_HI_U32_U24: Opcode = Opcode(12 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MIN_LEGACY_F32: Opcode = Opcode(13 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MAX_LEGACY_F32: Opcode = Opcode(14 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MIN_F32: Opcode = Opcode(15 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MAX_F32: Opcode = Opcode(16 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MIN_I32: Opcode = Opcode(17 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MAX_I32: Opcode = Opcode(18 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MIN_U32: Opcode = Opcode(19 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MAX_U32: Opcode = Opcode(20 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_LSHR_B32: Opcode = Opcode(21 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_LSHRREV_B32: Opcode = Opcode(22 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_ASHR_I32: Opcode = Opcode(23 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_ASHRREV_I32: Opcode = Opcode(24 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_LSHL_B32: Opcode = Opcode(25 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_LSHLREV_B32: Opcode = Opcode(26 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_AND_B32: Opcode = Opcode(27 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_OR_B32: Opcode = Opcode(28 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_XOR_B32: Opcode = Opcode(29 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_BFM_B32: Opcode = Opcode(30 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MAC_F32: Opcode = Opcode(31 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MADMK_F32: Opcode = Opcode(32 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MADAK_F32: Opcode = Opcode(33 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_BCNT_U32_B32: Opcode = Opcode(34 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MBCNT_LO_U32_B32: Opcode = Opcode(35 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MBCNT_HI_U32_B32: Opcode = Opcode(36 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_ADD_I32: Opcode = Opcode(37 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_SUB_I32: Opcode = Opcode(38 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_SUBREV_I32: Opcode = Opcode(39 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_ADDC_U32: Opcode = Opcode(40 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_SUBB_U32: Opcode = Opcode(41 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_SUBBREV_U32: Opcode = Opcode(42 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_LDEXP_F32: Opcode = Opcode(43 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_CVT_PKACCUM_U8_F32: Opcode = Opcode(44 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_CVT_PKNORM_I16_F32: Opcode = Opcode(45 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_CVT_PKNORM_U16_F32: Opcode = Opcode(46 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_CVT_PKRTZ_F16_F32: Opcode = Opcode(47 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_CVT_PK_U16_U32: Opcode = Opcode(48 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_CVT_PK_I16_I32: Opcode = Opcode(49 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_ADD_F16: Opcode = Opcode(50 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_SUB_F16: Opcode = Opcode(51 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_SUBREV_F16: Opcode = Opcode(52 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MUL_F16: Opcode = Opcode(53 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MAX_F16: Opcode = Opcode(57 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_MIN_F16: Opcode = Opcode(58 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_LDEXP_F16: Opcode = Opcode(59 + OpcodeMap::OP_MAP_VOP2.0);
    pub const V_NOP: Opcode = Opcode(0 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_MOV_B32: Opcode = Opcode(1 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_READFIRSTLANE_B32: Opcode = Opcode(2 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_I32_F64: Opcode = Opcode(3 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_F64_I32: Opcode = Opcode(4 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_F32_I32: Opcode = Opcode(5 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_F32_U32: Opcode = Opcode(6 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_U32_F32: Opcode = Opcode(7 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_I32_F32: Opcode = Opcode(8 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_MOV_FED_B32: Opcode = Opcode(9 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_F16_F32: Opcode = Opcode(10 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_F32_F16: Opcode = Opcode(11 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_RPI_I32_F32: Opcode = Opcode(12 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_FLR_I32_F32: Opcode = Opcode(13 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_OFF_F32_I4: Opcode = Opcode(14 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_F32_F64: Opcode = Opcode(15 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_F64_F32: Opcode = Opcode(16 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_F32_UBYTE0: Opcode = Opcode(17 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_F32_UBYTE1: Opcode = Opcode(18 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_F32_UBYTE2: Opcode = Opcode(19 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_F32_UBYTE3: Opcode = Opcode(20 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_U32_F64: Opcode = Opcode(21 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CVT_F64_U32: Opcode = Opcode(22 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_TRUNC_F64: Opcode = Opcode(23 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CEIL_F64: Opcode = Opcode(24 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_RNDNE_F64: Opcode = Opcode(25 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_FLOOR_F64: Opcode = Opcode(26 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_FRACT_F32: Opcode = Opcode(32 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_TRUNC_F32: Opcode = Opcode(33 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CEIL_F32: Opcode = Opcode(34 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_RNDNE_F32: Opcode = Opcode(35 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_FLOOR_F32: Opcode = Opcode(36 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_EXP_F32: Opcode = Opcode(37 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_LOG_CLAMP_F32: Opcode = Opcode(38 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_LOG_F32: Opcode = Opcode(39 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_RCP_CLAMP_F32: Opcode = Opcode(40 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_RCP_LEGACY_F32: Opcode = Opcode(41 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_RCP_F32: Opcode = Opcode(42 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_RCP_IFLAG_F32: Opcode = Opcode(43 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_RSQ_CLAMP_F32: Opcode = Opcode(44 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_RSQ_LEGACY_F32: Opcode = Opcode(45 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_RSQ_F32: Opcode = Opcode(46 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_RCP_F64: Opcode = Opcode(47 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_RCP_CLAMP_F64: Opcode = Opcode(48 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_RSQ_F64: Opcode = Opcode(49 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_RSQ_CLAMP_F64: Opcode = Opcode(50 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_SQRT_F32: Opcode = Opcode(51 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_SQRT_F64: Opcode = Opcode(52 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_SIN_F32: Opcode = Opcode(53 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_COS_F32: Opcode = Opcode(54 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_NOT_B32: Opcode = Opcode(55 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_BFREV_B32: Opcode = Opcode(56 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_FFBH_U32: Opcode = Opcode(57 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_FFBL_B32: Opcode = Opcode(58 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_FFBH_I32: Opcode = Opcode(59 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_FREXP_EXP_I32_F64: Opcode = Opcode(60 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_FREXP_MANT_F64: Opcode = Opcode(61 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_FRACT_F64: Opcode = Opcode(62 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_FREXP_EXP_I32_F32: Opcode = Opcode(63 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_FREXP_MANT_F32: Opcode = Opcode(64 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_CLREXCP: Opcode = Opcode(65 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_MOVRELD_B32: Opcode = Opcode(66 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_MOVRELS_B32: Opcode = Opcode(67 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_MOVRELSD_B32: Opcode = Opcode(68 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_LOG_LEGACY_F32: Opcode = Opcode(69 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_EXP_LEGACY_F32: Opcode = Opcode(70 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_RCP_F16: Opcode = Opcode(84 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_SQRT_F16: Opcode = Opcode(85 + OpcodeMap::OP_MAP_VOP1.0);
    pub const V_MAD_LEGACY_F32: Opcode = Opcode(320 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MAD_F32: Opcode = Opcode(321 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MAD_I32_I24: Opcode = Opcode(322 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MAD_U32_U24: Opcode = Opcode(323 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_CUBEID_F32: Opcode = Opcode(324 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_CUBESC_F32: Opcode = Opcode(325 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_CUBETC_F32: Opcode = Opcode(326 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_CUBEMA_F32: Opcode = Opcode(327 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_BFE_U32: Opcode = Opcode(328 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_BFE_I32: Opcode = Opcode(329 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_BFI_B32: Opcode = Opcode(330 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_FMA_F32: Opcode = Opcode(331 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_FMA_F64: Opcode = Opcode(332 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_LERP_U8: Opcode = Opcode(333 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_ALIGNBIT_B32: Opcode = Opcode(334 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_ALIGNBYTE_B32: Opcode = Opcode(335 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MULLIT_F32: Opcode = Opcode(336 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MIN3_F32: Opcode = Opcode(337 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MIN3_I32: Opcode = Opcode(338 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MIN3_U32: Opcode = Opcode(339 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MAX3_F32: Opcode = Opcode(340 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MAX3_I32: Opcode = Opcode(341 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MAX3_U32: Opcode = Opcode(342 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MED3_F32: Opcode = Opcode(343 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MED3_I32: Opcode = Opcode(344 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MED3_U32: Opcode = Opcode(345 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_SAD_U8: Opcode = Opcode(346 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_SAD_HI_U8: Opcode = Opcode(347 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_SAD_U16: Opcode = Opcode(348 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_SAD_U32: Opcode = Opcode(349 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_CVT_PK_U8_F32: Opcode = Opcode(350 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_DIV_FIXUP_F32: Opcode = Opcode(351 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_DIV_FIXUP_F64: Opcode = Opcode(352 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_LSHL_B64: Opcode = Opcode(353 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_LSHR_B64: Opcode = Opcode(354 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_ASHR_I64: Opcode = Opcode(355 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_ADD_F64: Opcode = Opcode(356 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MUL_F64: Opcode = Opcode(357 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MIN_F64: Opcode = Opcode(358 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MAX_F64: Opcode = Opcode(359 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_LDEXP_F64: Opcode = Opcode(360 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MUL_LO_U32: Opcode = Opcode(361 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MUL_HI_U32: Opcode = Opcode(362 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MUL_LO_I32: Opcode = Opcode(363 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MUL_HI_I32: Opcode = Opcode(364 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_DIV_SCALE_F32: Opcode = Opcode(365 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_DIV_SCALE_F64: Opcode = Opcode(366 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_DIV_FMAS_F32: Opcode = Opcode(367 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_DIV_FMAS_F64: Opcode = Opcode(368 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MSAD_U8: Opcode = Opcode(369 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_QSAD_U8: Opcode = Opcode(370 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_QSAD_PK_U16_U8: Opcode = Opcode(370 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MQSAD_U8: Opcode = Opcode(371 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MQSAD_PK_U16_U8: Opcode = Opcode(371 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_TRIG_PREOP_F64: Opcode = Opcode(372 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MQSAD_U32_U8: Opcode = Opcode(373 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MAD_U64_U32: Opcode = Opcode(374 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MAD_I64_I32: Opcode = Opcode(375 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_ADD_NC_U16: Opcode = Opcode(771 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_LSHRREV_B16: Opcode = Opcode(775 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_ASHRREV_I16: Opcode = Opcode(776 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_ADD_NC_I16: Opcode = Opcode(781 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_SUB_NC_I16: Opcode = Opcode(782 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_SUB_CO_U32: Opcode = Opcode(784 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_LSHLREV_B16: Opcode = Opcode(788 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MAD_F16: Opcode = Opcode(833 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_LSHL_ADD_U32: Opcode = Opcode(838 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_ADD_LSHL_U32: Opcode = Opcode(839 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MIN3_F16: Opcode = Opcode(849 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MAX3_F16: Opcode = Opcode(852 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MED3_F16: Opcode = Opcode(855 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_MAD_I16: Opcode = Opcode(862 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_ADD3_U32: Opcode = Opcode(877 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_LSHL_OR_B32: Opcode = Opcode(879 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_AND_OR_B32: Opcode = Opcode(881 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_OR3_B32: Opcode = Opcode(882 + OpcodeMap::OP_MAP_VOP3.0);
    pub const V_INTERP_P1_F32: Opcode = Opcode(0 + OpcodeMap::OP_MAP_VINTRP.0);
    pub const V_INTERP_P2_F32: Opcode = Opcode(1 + OpcodeMap::OP_MAP_VINTRP.0);
    pub const V_INTERP_MOV_F32: Opcode = Opcode(2 + OpcodeMap::OP_MAP_VINTRP.0);
    pub const S_LOAD_DWORD: Opcode = Opcode(0 + OpcodeMap::OP_MAP_SMRD.0);
    pub const S_LOAD_DWORDX2: Opcode = Opcode(1 + OpcodeMap::OP_MAP_SMRD.0);
    pub const S_LOAD_DWORDX4: Opcode = Opcode(2 + OpcodeMap::OP_MAP_SMRD.0);
    pub const S_LOAD_DWORDX8: Opcode = Opcode(3 + OpcodeMap::OP_MAP_SMRD.0);
    pub const S_LOAD_DWORDX16: Opcode = Opcode(4 + OpcodeMap::OP_MAP_SMRD.0);
    pub const S_BUFFER_LOAD_DWORD: Opcode = Opcode(8 + OpcodeMap::OP_MAP_SMRD.0);
    pub const S_BUFFER_LOAD_DWORDX2: Opcode = Opcode(9 + OpcodeMap::OP_MAP_SMRD.0);
    pub const S_BUFFER_LOAD_DWORDX4: Opcode = Opcode(10 + OpcodeMap::OP_MAP_SMRD.0);
    pub const S_BUFFER_LOAD_DWORDX8: Opcode = Opcode(11 + OpcodeMap::OP_MAP_SMRD.0);
    pub const S_BUFFER_LOAD_DWORDX16: Opcode = Opcode(12 + OpcodeMap::OP_MAP_SMRD.0);
    pub const S_MEMTIME: Opcode = Opcode(30 + OpcodeMap::OP_MAP_SMRD.0);
    pub const S_DCACHE_INV: Opcode = Opcode(31 + OpcodeMap::OP_MAP_SMRD.0);
    pub const DS_ADD_U32: Opcode = Opcode(0 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_SUB_U32: Opcode = Opcode(1 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_RSUB_U32: Opcode = Opcode(2 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_INC_U32: Opcode = Opcode(3 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_DEC_U32: Opcode = Opcode(4 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_I32: Opcode = Opcode(5 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_I32: Opcode = Opcode(6 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_U32: Opcode = Opcode(7 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_U32: Opcode = Opcode(8 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_AND_B32: Opcode = Opcode(9 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_OR_B32: Opcode = Opcode(10 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_XOR_B32: Opcode = Opcode(11 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MSKOR_B32: Opcode = Opcode(12 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRITE_B32: Opcode = Opcode(13 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRITE2_B32: Opcode = Opcode(14 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRITE2ST64_B32: Opcode = Opcode(15 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_CMPST_B32: Opcode = Opcode(16 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_CMPST_F32: Opcode = Opcode(17 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_F32: Opcode = Opcode(18 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_F32: Opcode = Opcode(19 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_NOP: Opcode = Opcode(20 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_GWS_SEMA_RELEASE_ALL: Opcode = Opcode(24 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_GWS_INIT: Opcode = Opcode(25 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_GWS_SEMA_V: Opcode = Opcode(26 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_GWS_SEMA_BR: Opcode = Opcode(27 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_GWS_SEMA_P: Opcode = Opcode(28 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_GWS_BARRIER: Opcode = Opcode(29 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRITE_B8: Opcode = Opcode(30 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRITE_B16: Opcode = Opcode(31 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_ADD_RTN_U32: Opcode = Opcode(32 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_SUB_RTN_U32: Opcode = Opcode(33 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_RSUB_RTN_U32: Opcode = Opcode(34 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_INC_RTN_U32: Opcode = Opcode(35 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_DEC_RTN_U32: Opcode = Opcode(36 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_RTN_I32: Opcode = Opcode(37 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_RTN_I32: Opcode = Opcode(38 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_RTN_U32: Opcode = Opcode(39 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_RTN_U32: Opcode = Opcode(40 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_AND_RTN_B32: Opcode = Opcode(41 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_OR_RTN_B32: Opcode = Opcode(42 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_XOR_RTN_B32: Opcode = Opcode(43 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MSKOR_RTN_B32: Opcode = Opcode(44 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRXCHG_RTN_B32: Opcode = Opcode(45 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRXCHG2_RTN_B32: Opcode = Opcode(46 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRXCHG2ST64_RTN_B32: Opcode = Opcode(47 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_CMPST_RTN_B32: Opcode = Opcode(48 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_CMPST_RTN_F32: Opcode = Opcode(49 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_RTN_F32: Opcode = Opcode(50 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_RTN_F32: Opcode = Opcode(51 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRAP_RTN_B32: Opcode = Opcode(52 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_SWIZZLE_B32: Opcode = Opcode(53 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_READ_B32: Opcode = Opcode(54 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_READ2_B32: Opcode = Opcode(55 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_READ2ST64_B32: Opcode = Opcode(56 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_READ_I8: Opcode = Opcode(57 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_READ_U8: Opcode = Opcode(58 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_READ_I16: Opcode = Opcode(59 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_READ_U16: Opcode = Opcode(60 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_CONSUME: Opcode = Opcode(61 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_APPEND: Opcode = Opcode(62 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_ORDERED_COUNT: Opcode = Opcode(63 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_ADD_U64: Opcode = Opcode(64 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_SUB_U64: Opcode = Opcode(65 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_RSUB_U64: Opcode = Opcode(66 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_INC_U64: Opcode = Opcode(67 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_DEC_U64: Opcode = Opcode(68 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_I64: Opcode = Opcode(69 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_I64: Opcode = Opcode(70 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_U64: Opcode = Opcode(71 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_U64: Opcode = Opcode(72 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_AND_B64: Opcode = Opcode(73 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_OR_B64: Opcode = Opcode(74 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_XOR_B64: Opcode = Opcode(75 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MSKOR_B64: Opcode = Opcode(76 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRITE_B64: Opcode = Opcode(77 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRITE2_B64: Opcode = Opcode(78 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRITE2ST64_B64: Opcode = Opcode(79 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_CMPST_B64: Opcode = Opcode(80 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_CMPST_F64: Opcode = Opcode(81 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_F64: Opcode = Opcode(82 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_F64: Opcode = Opcode(83 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_ADD_RTN_U64: Opcode = Opcode(96 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_SUB_RTN_U64: Opcode = Opcode(97 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_RSUB_RTN_U64: Opcode = Opcode(98 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_INC_RTN_U64: Opcode = Opcode(99 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_DEC_RTN_U64: Opcode = Opcode(100 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_RTN_I64: Opcode = Opcode(101 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_RTN_I64: Opcode = Opcode(102 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_RTN_U64: Opcode = Opcode(103 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_RTN_U64: Opcode = Opcode(104 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_AND_RTN_B64: Opcode = Opcode(105 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_OR_RTN_B64: Opcode = Opcode(106 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_XOR_RTN_B64: Opcode = Opcode(107 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MSKOR_RTN_B64: Opcode = Opcode(108 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRXCHG_RTN_B64: Opcode = Opcode(109 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRXCHG2_RTN_B64: Opcode = Opcode(110 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRXCHG2ST64_RTN_B64: Opcode = Opcode(111 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_CMPST_RTN_B64: Opcode = Opcode(112 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_CMPST_RTN_F64: Opcode = Opcode(113 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_RTN_F64: Opcode = Opcode(114 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_RTN_F64: Opcode = Opcode(115 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_READ_B64: Opcode = Opcode(118 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_READ2_B64: Opcode = Opcode(119 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_READ2ST64_B64: Opcode = Opcode(120 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_CONDXCHG32_RTN_B64: Opcode = Opcode(126 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_ADD_SRC2_U32: Opcode = Opcode(128 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_SUB_SRC2_U32: Opcode = Opcode(129 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_RSUB_SRC2_U32: Opcode = Opcode(130 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_INC_SRC2_U32: Opcode = Opcode(131 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_DEC_SRC2_U32: Opcode = Opcode(132 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_SRC2_I32: Opcode = Opcode(133 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_SRC2_I32: Opcode = Opcode(134 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_SRC2_U32: Opcode = Opcode(135 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_SRC2_U32: Opcode = Opcode(136 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_AND_SRC2_B32: Opcode = Opcode(137 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_OR_SRC2_B32: Opcode = Opcode(138 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_XOR_SRC2_B32: Opcode = Opcode(139 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRITE_SRC2_B32: Opcode = Opcode(141 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_SRC2_F32: Opcode = Opcode(146 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_SRC2_F32: Opcode = Opcode(147 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_ADD_SRC2_U64: Opcode = Opcode(192 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_SUB_SRC2_U64: Opcode = Opcode(193 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_RSUB_SRC2_U64: Opcode = Opcode(194 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_INC_SRC2_U64: Opcode = Opcode(195 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_DEC_SRC2_U64: Opcode = Opcode(196 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_SRC2_I64: Opcode = Opcode(197 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_SRC2_I64: Opcode = Opcode(198 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_SRC2_U64: Opcode = Opcode(199 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_SRC2_U64: Opcode = Opcode(200 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_AND_SRC2_B64: Opcode = Opcode(201 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_OR_SRC2_B64: Opcode = Opcode(202 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_XOR_SRC2_B64: Opcode = Opcode(203 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRITE_SRC2_B64: Opcode = Opcode(205 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MIN_SRC2_F64: Opcode = Opcode(210 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_MAX_SRC2_F64: Opcode = Opcode(211 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRITE_B96: Opcode = Opcode(222 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_WRITE_B128: Opcode = Opcode(223 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_CONDXCHG32_RTN_B128: Opcode = Opcode(253 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_READ_B96: Opcode = Opcode(254 + OpcodeMap::OP_MAP_DS.0);
    pub const DS_READ_B128: Opcode = Opcode(255 + OpcodeMap::OP_MAP_DS.0);
    pub const BUFFER_LOAD_FORMAT_X: Opcode = Opcode(0 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_LOAD_FORMAT_XY: Opcode = Opcode(1 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_LOAD_FORMAT_XYZ: Opcode = Opcode(2 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_LOAD_FORMAT_XYZW: Opcode = Opcode(3 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_STORE_FORMAT_X: Opcode = Opcode(4 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_STORE_FORMAT_XY: Opcode = Opcode(5 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_STORE_FORMAT_XYZ: Opcode = Opcode(6 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_STORE_FORMAT_XYZW: Opcode = Opcode(7 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_LOAD_UBYTE: Opcode = Opcode(8 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_LOAD_SBYTE: Opcode = Opcode(9 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_LOAD_USHORT: Opcode = Opcode(10 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_LOAD_SSHORT: Opcode = Opcode(11 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_LOAD_DWORD: Opcode = Opcode(12 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_LOAD_DWORDX2: Opcode = Opcode(13 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_LOAD_DWORDX4: Opcode = Opcode(14 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_LOAD_DWORDX3: Opcode = Opcode(15 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_STORE_BYTE: Opcode = Opcode(24 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_STORE_SHORT: Opcode = Opcode(26 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_STORE_DWORD: Opcode = Opcode(28 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_STORE_DWORDX2: Opcode = Opcode(29 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_STORE_DWORDX4: Opcode = Opcode(30 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_STORE_DWORDX3: Opcode = Opcode(31 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_SWAP: Opcode = Opcode(48 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_CMPSWAP: Opcode = Opcode(49 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_ADD: Opcode = Opcode(50 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_SUB: Opcode = Opcode(51 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_SMIN: Opcode = Opcode(53 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_UMIN: Opcode = Opcode(54 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_SMAX: Opcode = Opcode(55 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_UMAX: Opcode = Opcode(56 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_AND: Opcode = Opcode(57 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_OR: Opcode = Opcode(58 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_XOR: Opcode = Opcode(59 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_INC: Opcode = Opcode(60 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_DEC: Opcode = Opcode(61 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_FCMPSWAP: Opcode = Opcode(62 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_FMIN: Opcode = Opcode(63 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_FMAX: Opcode = Opcode(64 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_SWAP_X2: Opcode = Opcode(80 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_CMPSWAP_X2: Opcode = Opcode(81 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_ADD_X2: Opcode = Opcode(82 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_SUB_X2: Opcode = Opcode(83 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_SMIN_X2: Opcode = Opcode(85 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_UMIN_X2: Opcode = Opcode(86 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_SMAX_X2: Opcode = Opcode(87 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_UMAX_X2: Opcode = Opcode(88 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_AND_X2: Opcode = Opcode(89 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_OR_X2: Opcode = Opcode(90 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_XOR_X2: Opcode = Opcode(91 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_INC_X2: Opcode = Opcode(92 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_DEC_X2: Opcode = Opcode(93 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_FCMPSWAP_X2: Opcode = Opcode(94 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_FMIN_X2: Opcode = Opcode(95 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_ATOMIC_FMAX_X2: Opcode = Opcode(96 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_WBINVL1_SC: Opcode = Opcode(112 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const BUFFER_WBINVL1: Opcode = Opcode(113 + OpcodeMap::OP_MAP_MUBUF.0);
    pub const TBUFFER_LOAD_FORMAT_X: Opcode = Opcode(0 + OpcodeMap::OP_MAP_MTBUF.0);
    pub const TBUFFER_LOAD_FORMAT_XY: Opcode = Opcode(1 + OpcodeMap::OP_MAP_MTBUF.0);
    pub const TBUFFER_LOAD_FORMAT_XYZ: Opcode = Opcode(2 + OpcodeMap::OP_MAP_MTBUF.0);
    pub const TBUFFER_LOAD_FORMAT_XYZW: Opcode = Opcode(3 + OpcodeMap::OP_MAP_MTBUF.0);
    pub const TBUFFER_STORE_FORMAT_X: Opcode = Opcode(4 + OpcodeMap::OP_MAP_MTBUF.0);
    pub const TBUFFER_STORE_FORMAT_XY: Opcode = Opcode(5 + OpcodeMap::OP_MAP_MTBUF.0);
    pub const TBUFFER_STORE_FORMAT_XYZ: Opcode = Opcode(6 + OpcodeMap::OP_MAP_MTBUF.0);
    pub const TBUFFER_STORE_FORMAT_XYZW: Opcode = Opcode(7 + OpcodeMap::OP_MAP_MTBUF.0);
    pub const IMAGE_LOAD: Opcode = Opcode(0 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_LOAD_MIP: Opcode = Opcode(1 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_LOAD_PCK: Opcode = Opcode(2 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_LOAD_PCK_SGN: Opcode = Opcode(3 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_LOAD_MIP_PCK: Opcode = Opcode(4 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_LOAD_MIP_PCK_SGN: Opcode = Opcode(5 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_STORE: Opcode = Opcode(8 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_STORE_MIP: Opcode = Opcode(9 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_STORE_PCK: Opcode = Opcode(10 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_STORE_MIP_PCK: Opcode = Opcode(11 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GET_RESINFO: Opcode = Opcode(14 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_ATOMIC_SWAP: Opcode = Opcode(15 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_ATOMIC_CMPSWAP: Opcode = Opcode(16 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_ATOMIC_ADD: Opcode = Opcode(17 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_ATOMIC_SUB: Opcode = Opcode(18 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_ATOMIC_SMIN: Opcode = Opcode(20 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_ATOMIC_UMIN: Opcode = Opcode(21 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_ATOMIC_SMAX: Opcode = Opcode(22 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_ATOMIC_UMAX: Opcode = Opcode(23 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_ATOMIC_AND: Opcode = Opcode(24 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_ATOMIC_OR: Opcode = Opcode(25 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_ATOMIC_XOR: Opcode = Opcode(26 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_ATOMIC_INC: Opcode = Opcode(27 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_ATOMIC_DEC: Opcode = Opcode(28 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_ATOMIC_FCMPSWAP: Opcode = Opcode(29 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_ATOMIC_FMIN: Opcode = Opcode(30 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_ATOMIC_FMAX: Opcode = Opcode(31 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE: Opcode = Opcode(32 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_CL: Opcode = Opcode(33 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_D: Opcode = Opcode(34 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_D_CL: Opcode = Opcode(35 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_L: Opcode = Opcode(36 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_B: Opcode = Opcode(37 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_B_CL: Opcode = Opcode(38 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_LZ: Opcode = Opcode(39 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C: Opcode = Opcode(40 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_CL: Opcode = Opcode(41 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_D: Opcode = Opcode(42 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_D_CL: Opcode = Opcode(43 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_L: Opcode = Opcode(44 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_B: Opcode = Opcode(45 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_B_CL: Opcode = Opcode(46 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_LZ: Opcode = Opcode(47 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_O: Opcode = Opcode(48 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_CL_O: Opcode = Opcode(49 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_D_O: Opcode = Opcode(50 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_D_CL_O: Opcode = Opcode(51 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_L_O: Opcode = Opcode(52 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_B_O: Opcode = Opcode(53 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_B_CL_O: Opcode = Opcode(54 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_LZ_O: Opcode = Opcode(55 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_O: Opcode = Opcode(56 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_CL_O: Opcode = Opcode(57 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_D_O: Opcode = Opcode(58 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_D_CL_O: Opcode = Opcode(59 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_L_O: Opcode = Opcode(60 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_B_O: Opcode = Opcode(61 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_B_CL_O: Opcode = Opcode(62 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_LZ_O: Opcode = Opcode(63 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4: Opcode = Opcode(64 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_CL: Opcode = Opcode(65 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_L: Opcode = Opcode(68 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_B: Opcode = Opcode(69 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_B_CL: Opcode = Opcode(70 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_LZ: Opcode = Opcode(71 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_C: Opcode = Opcode(72 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_C_CL: Opcode = Opcode(73 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_C_L: Opcode = Opcode(76 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_C_B: Opcode = Opcode(77 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_C_B_CL: Opcode = Opcode(78 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_C_LZ: Opcode = Opcode(79 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_O: Opcode = Opcode(80 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_CL_O: Opcode = Opcode(81 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_L_O: Opcode = Opcode(84 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_B_O: Opcode = Opcode(85 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_B_CL_O: Opcode = Opcode(86 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_LZ_O: Opcode = Opcode(87 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_C_O: Opcode = Opcode(88 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_C_CL_O: Opcode = Opcode(89 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_C_L_O: Opcode = Opcode(92 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_C_B_O: Opcode = Opcode(93 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_C_B_CL_O: Opcode = Opcode(94 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GATHER4_C_LZ_O: Opcode = Opcode(95 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_GET_LOD: Opcode = Opcode(96 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_CD: Opcode = Opcode(104 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_CD_CL: Opcode = Opcode(105 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_CD: Opcode = Opcode(106 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_CD_CL: Opcode = Opcode(107 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_CD_O: Opcode = Opcode(108 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_CD_CL_O: Opcode = Opcode(109 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_CD_O: Opcode = Opcode(110 + OpcodeMap::OP_MAP_MIMG.0);
    pub const IMAGE_SAMPLE_C_CD_CL_O: Opcode = Opcode(111 + OpcodeMap::OP_MAP_MIMG.0);
    pub const EXP: Opcode = Opcode(0 + OpcodeMap::OP_MAP_EXP.0);
    pub const V_PK_MAD_I16: Opcode = Opcode(0 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_MUL_LO_U16: Opcode = Opcode(1 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_ADD_I16: Opcode = Opcode(2 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_SUB_I16: Opcode = Opcode(3 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_LSHLREV_B16: Opcode = Opcode(4 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_LSHRREV_B16: Opcode = Opcode(5 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_ASHRREV_I16: Opcode = Opcode(6 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_MAX_I16: Opcode = Opcode(7 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_MIN_I16: Opcode = Opcode(8 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_MAD_U16: Opcode = Opcode(9 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_ADD_U16: Opcode = Opcode(10 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_SUB_U16: Opcode = Opcode(11 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_MAX_U16: Opcode = Opcode(12 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_MIN_U16: Opcode = Opcode(13 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_FMA_F16: Opcode = Opcode(14 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_ADD_F16: Opcode = Opcode(15 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_MUL_F16: Opcode = Opcode(16 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_MIN_F16: Opcode = Opcode(17 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_PK_MAX_F16: Opcode = Opcode(18 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_MAD_MIX_F32: Opcode = Opcode(32 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_MAD_MIXLO_F16: Opcode = Opcode(33 + OpcodeMap::OP_MAP_VOP3P.0);
    pub const V_MAD_MIXHI_F16: Opcode = Opcode(34 + OpcodeMap::OP_MAP_VOP3P.0);
}

/// `Shader::Gcn::EncodingMask`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct EncodingMask(pub u32);

impl EncodingMask {
    pub const MASK_9bit: EncodingMask = EncodingMask(0x000001FF << 23);
    pub const MASK_7bit: EncodingMask = EncodingMask(0x0000007F << 25);
    pub const MASK_6bit: EncodingMask = EncodingMask(0x0000003F << 26);
    pub const MASK_5bit: EncodingMask = EncodingMask(0x0000001F << 27);
    pub const MASK_4bit: EncodingMask = EncodingMask(0x0000000F << 28);
    pub const MASK_2bit: EncodingMask = EncodingMask(0x00000003 << 30);
    pub const MASK_1bit: EncodingMask = EncodingMask(0x00000001 << 31);
}

/// `Shader::Gcn::InstEncoding`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct InstEncoding(pub u32);

impl InstEncoding {
    pub const SOP1: InstEncoding = InstEncoding(0x0000017D << 23);
    pub const SOPP: InstEncoding = InstEncoding(0x0000017F << 23);
    pub const SOPC: InstEncoding = InstEncoding(0x0000017E << 23);
    pub const VOP1: InstEncoding = InstEncoding(0x0000003F << 25);
    pub const VOPC: InstEncoding = InstEncoding(0x0000003E << 25);
    pub const VOP3: InstEncoding = InstEncoding(0x00000034 << 26);
    pub const EXP: InstEncoding = InstEncoding(0x0000003E << 26);
    pub const VOP3P: InstEncoding = InstEncoding(0x00000033 << 26);
    pub const VINTRP: InstEncoding = InstEncoding(0x00000032 << 26);
    pub const DS: InstEncoding = InstEncoding(0x00000036 << 26);
    pub const MUBUF: InstEncoding = InstEncoding(0x00000038 << 26);
    pub const MTBUF: InstEncoding = InstEncoding(0x0000003A << 26);
    pub const MIMG: InstEncoding = InstEncoding(0x0000003C << 26);
    pub const SMRD: InstEncoding = InstEncoding(0x00000018 << 27);
    pub const SOPK: InstEncoding = InstEncoding(0x0000000B << 28);
    pub const SOP2: InstEncoding = InstEncoding(0x00000002 << 30);
    pub const VOP2: InstEncoding = InstEncoding(0x00000000 << 31);
    pub const ILLEGAL: InstEncoding = InstEncoding(1);
}

/// `Shader::Gcn::InstClass`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct InstClass(pub u32);

impl InstClass {
    pub const Undefined: InstClass = InstClass(0);
    pub const ScalarArith: InstClass = InstClass(1);
    pub const ScalarAbs: InstClass = InstClass(2);
    pub const ScalarMov: InstClass = InstClass(3);
    pub const ScalarMovRel: InstClass = InstClass(4);
    pub const ScalarCmp: InstClass = InstClass(5);
    pub const ScalarSelect: InstClass = InstClass(6);
    pub const ScalarBitLogic: InstClass = InstClass(7);
    pub const ScalarBitManip: InstClass = InstClass(8);
    pub const ScalarBitField: InstClass = InstClass(9);
    pub const ScalarConv: InstClass = InstClass(10);
    pub const ScalarExecMask: InstClass = InstClass(11);
    pub const ScalarQuadMask: InstClass = InstClass(12);
    pub const VectorRegMov: InstClass = InstClass(13);
    pub const VectorMovRel: InstClass = InstClass(14);
    pub const VectorLane: InstClass = InstClass(15);
    pub const VectorBitLogic: InstClass = InstClass(16);
    pub const VectorBitField32: InstClass = InstClass(17);
    pub const VectorThreadMask: InstClass = InstClass(18);
    pub const VectorBitField64: InstClass = InstClass(19);
    pub const VectorFpArith16: InstClass = InstClass(20);
    pub const VectorFpArith32: InstClass = InstClass(21);
    pub const VectorFpRound32: InstClass = InstClass(22);
    pub const VectorFpField32: InstClass = InstClass(23);
    pub const VectorFpTran16: InstClass = InstClass(24);
    pub const VectorFpTran32: InstClass = InstClass(25);
    pub const VectorFpCmp16: InstClass = InstClass(26);
    pub const VectorFpCmp32: InstClass = InstClass(27);
    pub const VectorFpArith64: InstClass = InstClass(28);
    pub const VectorFpRound64: InstClass = InstClass(29);
    pub const VectorFpField64: InstClass = InstClass(30);
    pub const VectorFpTran64: InstClass = InstClass(31);
    pub const VectorFpCmp64: InstClass = InstClass(32);
    pub const VectorIntArith16: InstClass = InstClass(33);
    pub const VectorIntArith32: InstClass = InstClass(34);
    pub const VectorIntArith64: InstClass = InstClass(35);
    pub const VectorIntCmp16: InstClass = InstClass(36);
    pub const VectorIntCmp32: InstClass = InstClass(37);
    pub const VectorIntCmp64: InstClass = InstClass(38);
    pub const VectorConv: InstClass = InstClass(39);
    pub const VectorFpGraph32: InstClass = InstClass(40);
    pub const VectorIntGraph: InstClass = InstClass(41);
    pub const VectorMisc: InstClass = InstClass(42);
    pub const ScalarProgFlow: InstClass = InstClass(43);
    pub const ScalarSync: InstClass = InstClass(44);
    pub const ScalarWait: InstClass = InstClass(45);
    pub const ScalarCache: InstClass = InstClass(46);
    pub const ScalarPrior: InstClass = InstClass(47);
    pub const ScalarRegAccess: InstClass = InstClass(48);
    pub const ScalarMsg: InstClass = InstClass(49);
    pub const ScalarMemRd: InstClass = InstClass(50);
    pub const ScalarMemUt: InstClass = InstClass(51);
    pub const VectorMemBufNoFmt: InstClass = InstClass(52);
    pub const VectorMemBufFmt: InstClass = InstClass(53);
    pub const VectorMemBufAtomic: InstClass = InstClass(54);
    pub const VectorMemImgNoSmp: InstClass = InstClass(55);
    pub const VectorMemImgSmp: InstClass = InstClass(56);
    pub const VectorMemImgUt: InstClass = InstClass(57);
    pub const VectorMemL1Cache: InstClass = InstClass(58);
    pub const DsIdxRd: InstClass = InstClass(59);
    pub const DsIdxWr: InstClass = InstClass(60);
    pub const DsIdxWrXchg: InstClass = InstClass(61);
    pub const DsIdxCondXchg: InstClass = InstClass(62);
    pub const DsIdxWrap: InstClass = InstClass(63);
    pub const DsAtomicArith32: InstClass = InstClass(64);
    pub const DsAtomicArith64: InstClass = InstClass(65);
    pub const DsAtomicMinMax32: InstClass = InstClass(66);
    pub const DsAtomicMinMax64: InstClass = InstClass(67);
    pub const DsAtomicCmpSt32: InstClass = InstClass(68);
    pub const DsAtomicCmpSt64: InstClass = InstClass(69);
    pub const DsAtomicLogic32: InstClass = InstClass(70);
    pub const DsAtomicLogic64: InstClass = InstClass(71);
    pub const DsAppendCon: InstClass = InstClass(72);
    pub const DsDataShareUt: InstClass = InstClass(73);
    pub const DsDataShareMisc: InstClass = InstClass(74);
    pub const GdsSync: InstClass = InstClass(75);
    pub const GdsOrdCnt: InstClass = InstClass(76);
    pub const VectorInterpFpCache: InstClass = InstClass(77);
    pub const Exp: InstClass = InstClass(78);
    pub const DbgProf: InstClass = InstClass(79);
}

/// `Shader::Gcn::InstCategory`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct InstCategory(pub u32);

impl InstCategory {
    pub const Undefined: InstCategory = InstCategory(0);
    pub const ScalarALU: InstCategory = InstCategory(1);
    pub const ScalarMemory: InstCategory = InstCategory(2);
    pub const VectorALU: InstCategory = InstCategory(3);
    pub const VectorMemory: InstCategory = InstCategory(4);
    pub const FlowControl: InstCategory = InstCategory(5);
    pub const DataShare: InstCategory = InstCategory(6);
    pub const VectorInterpolation: InstCategory = InstCategory(7);
    pub const Export: InstCategory = InstCategory(8);
    pub const DebugProfile: InstCategory = InstCategory(9);
}

/// `Shader::Gcn::ScalarType`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct ScalarType(pub u32);

impl ScalarType {
    pub const Undefined: ScalarType = ScalarType(0);
    pub const Any: ScalarType = ScalarType(1);
    pub const Uint16: ScalarType = ScalarType(2);
    pub const Uint32: ScalarType = ScalarType(3);
    pub const Uint64: ScalarType = ScalarType(4);
    pub const Sint16: ScalarType = ScalarType(5);
    pub const Sint32: ScalarType = ScalarType(6);
    pub const Sint64: ScalarType = ScalarType(7);
    pub const Float16: ScalarType = ScalarType(8);
    pub const Float32: ScalarType = ScalarType(9);
    pub const Float64: ScalarType = ScalarType(10);
    pub const Bool: ScalarType = ScalarType(11);
}

/// `Shader::Gcn::DivergentAction`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct DivergentAction(pub u32);

impl DivergentAction {
    pub const Nop: DivergentAction = DivergentAction(0);
    pub const ZeroScalar: DivergentAction = DivergentAction(1);
    pub const Execute: DivergentAction = DivergentAction(2);
}

/// `Shader::Gcn::OperandField`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct OperandField(pub u32);

impl OperandField {
    pub const ScalarGPR: OperandField = OperandField(0);
    pub const VccLo: OperandField = OperandField(106);
    pub const VccHi: OperandField = OperandField(107);
    pub const M0: OperandField = OperandField(124);
    pub const ExecLo: OperandField = OperandField(126);
    pub const ExecHi: OperandField = OperandField(127);
    pub const ConstZero: OperandField = OperandField(128);
    pub const SignedConstIntPos: OperandField = OperandField(129);
    pub const SignedConstIntNeg: OperandField = OperandField(193);
    pub const ConstFloatPos_0_5: OperandField = OperandField(240);
    pub const ConstFloatNeg_0_5: OperandField = OperandField(241);
    pub const ConstFloatPos_1_0: OperandField = OperandField(242);
    pub const ConstFloatNeg_1_0: OperandField = OperandField(243);
    pub const ConstFloatPos_2_0: OperandField = OperandField(244);
    pub const ConstFloatNeg_2_0: OperandField = OperandField(245);
    pub const ConstFloatPos_4_0: OperandField = OperandField(246);
    pub const ConstFloatNeg_4_0: OperandField = OperandField(247);
    pub const Inv2Pi: OperandField = OperandField(248);
    pub const Sdwa: OperandField = OperandField(249);
    pub const Dpp: OperandField = OperandField(250);
    pub const VccZ: OperandField = OperandField(251);
    pub const ExecZ: OperandField = OperandField(252);
    pub const Scc: OperandField = OperandField(253);
    pub const LdsDirect: OperandField = OperandField(254);
    pub const LiteralConst: OperandField = OperandField(255);
    pub const VectorGPR: OperandField = OperandField(256);
    pub const Undefined: OperandField = OperandField(0xFFFFFFFF);
}

/// `Shader::Gcn::SystemValue`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct SystemValue(pub u32);

impl SystemValue {
    pub const None: SystemValue = SystemValue(0);
    pub const Position: SystemValue = SystemValue(1);
    pub const ClipDistance: SystemValue = SystemValue(2);
    pub const CullDistance: SystemValue = SystemValue(3);
    pub const RenderTargetId: SystemValue = SystemValue(4);
    pub const ViewportId: SystemValue = SystemValue(5);
    pub const VertexId: SystemValue = SystemValue(6);
    pub const PrimitiveId: SystemValue = SystemValue(7);
    pub const InstanceId: SystemValue = SystemValue(8);
    pub const IsFrontFace: SystemValue = SystemValue(9);
    pub const SampleIndex: SystemValue = SystemValue(10);
    pub const FinalQuadUeq0EdgeTessFactor: SystemValue = SystemValue(11);
    pub const FinalQuadVeq0EdgeTessFactor: SystemValue = SystemValue(12);
    pub const FinalQuadUeq1EdgeTessFactor: SystemValue = SystemValue(13);
    pub const FinalQuadVeq1EdgeTessFactor: SystemValue = SystemValue(14);
    pub const FinalQuadUInsideTessFactor: SystemValue = SystemValue(15);
    pub const FinalQuadVInsideTessFactor: SystemValue = SystemValue(16);
    pub const FinalTriUeq0EdgeTessFactor: SystemValue = SystemValue(17);
    pub const FinalTriVeq0EdgeTessFactor: SystemValue = SystemValue(18);
    pub const FinalTriWeq0EdgeTessFactor: SystemValue = SystemValue(19);
    pub const FinalTriInsideTessFactor: SystemValue = SystemValue(20);
    pub const FinalLineDetailTessFactor: SystemValue = SystemValue(21);
    pub const FinalLineDensityTessFactor: SystemValue = SystemValue(22);
    pub const GlobalInvocationId: SystemValue = SystemValue(23);
    pub const WorkgroupId: SystemValue = SystemValue(24);
    pub const LocalInvocationId: SystemValue = SystemValue(25);
    pub const LocalInvocationIndex: SystemValue = SystemValue(26);
    pub const Target: SystemValue = SystemValue(64);
    pub const Depth: SystemValue = SystemValue(65);
    pub const Coverage: SystemValue = SystemValue(66);
    pub const DepthGe: SystemValue = SystemValue(67);
    pub const DepthLe: SystemValue = SystemValue(68);
    pub const NumSubgroups: SystemValue = SystemValue(70);
    pub const SubgroupID: SystemValue = SystemValue(71);
    pub const SubgroupSize: SystemValue = SystemValue(72);
    pub const SubgroupInvocationID: SystemValue = SystemValue(73);
    pub const SubgroupEqMask: SystemValue = SystemValue(74);
    pub const SubgroupGeMask: SystemValue = SystemValue(75);
    pub const SubgroupGtMask: SystemValue = SystemValue(76);
    pub const SubgroupLeMask: SystemValue = SystemValue(77);
    pub const SubgroupLtMask: SystemValue = SystemValue(78);
}

/// `Shader::Gcn::InterpolationMode`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct InterpolationMode(pub u32);

impl InterpolationMode {
    pub const Undefined: InterpolationMode = InterpolationMode(0);
    pub const Constant: InterpolationMode = InterpolationMode(1);
    pub const Linear: InterpolationMode = InterpolationMode(2);
    pub const LinearCentroid: InterpolationMode = InterpolationMode(3);
    pub const LinearNoPerspective: InterpolationMode = InterpolationMode(4);
    pub const LinearNoPerspectiveCentroid: InterpolationMode = InterpolationMode(5);
    pub const LinearSample: InterpolationMode = InterpolationMode(6);
    pub const LinearNoPerspectiveSample: InterpolationMode = InterpolationMode(7);
}

/// `Shader::Gcn::MimgModifier`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct MimgModifier(pub u32);

impl MimgModifier {
    pub const Lod: MimgModifier = MimgModifier(0);
    pub const LodBias: MimgModifier = MimgModifier(1);
    pub const LodClamp: MimgModifier = MimgModifier(2);
    pub const Derivative: MimgModifier = MimgModifier(3);
    pub const CoarseDerivative: MimgModifier = MimgModifier(4);
    pub const Level0: MimgModifier = MimgModifier(5);
    pub const Pcf: MimgModifier = MimgModifier(6);
    pub const Offset: MimgModifier = MimgModifier(7);
}

/// `Shader::Gcn::ImageResComponent`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct ImageResComponent(pub u32);

impl ImageResComponent {
    pub const Width: ImageResComponent = ImageResComponent(0);
    pub const Height: ImageResComponent = ImageResComponent(1);
    pub const Depth: ImageResComponent = ImageResComponent(2);
    pub const MipCount: ImageResComponent = ImageResComponent(3);
}

/// `Shader::Gcn::ImageAddrComponent`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct ImageAddrComponent(pub u32);

impl ImageAddrComponent {
    pub const Offsets: ImageAddrComponent = ImageAddrComponent(0);
    pub const Bias: ImageAddrComponent = ImageAddrComponent(1);
    pub const Zpcf: ImageAddrComponent = ImageAddrComponent(2);
    pub const DxDh: ImageAddrComponent = ImageAddrComponent(3);
    pub const DyDh: ImageAddrComponent = ImageAddrComponent(4);
    pub const DzDh: ImageAddrComponent = ImageAddrComponent(5);
    pub const DxDv: ImageAddrComponent = ImageAddrComponent(6);
    pub const DyDv: ImageAddrComponent = ImageAddrComponent(7);
    pub const DzDv: ImageAddrComponent = ImageAddrComponent(8);
    pub const X: ImageAddrComponent = ImageAddrComponent(9);
    pub const Y: ImageAddrComponent = ImageAddrComponent(10);
    pub const Z: ImageAddrComponent = ImageAddrComponent(11);
    pub const Slice: ImageAddrComponent = ImageAddrComponent(12);
    pub const FaceId: ImageAddrComponent = ImageAddrComponent(13);
    pub const FragId: ImageAddrComponent = ImageAddrComponent(14);
    pub const Lod: ImageAddrComponent = ImageAddrComponent(15);
    pub const Clamp: ImageAddrComponent = ImageAddrComponent(16);
}

/// `Shader::Gcn::Message`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct Message(pub u32);

impl Message {
    pub const Interrupt: Message = Message(1);
    pub const Gs: Message = Message(2);
    pub const GsDone: Message = Message(3);
    pub const System: Message = Message(15);
}

/// `Shader::Gcn::GsOp`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct GsOp(pub u32);

impl GsOp {
    pub const Nop: GsOp = GsOp(0);
    pub const Cut: GsOp = GsOp(1);
    pub const Emit: GsOp = GsOp(2);
    pub const EmitCut: GsOp = GsOp(3);
}

/// `magic_enum::enum_name` for a `Opcode`, in declaration order.
pub const OPCODE_NAMES: &[(&str, Opcode)] = &[
    ("S_ADD_U32", Opcode::S_ADD_U32),
    ("S_SUB_U32", Opcode::S_SUB_U32),
    ("S_ADD_I32", Opcode::S_ADD_I32),
    ("S_SUB_I32", Opcode::S_SUB_I32),
    ("S_ADDC_U32", Opcode::S_ADDC_U32),
    ("S_SUBB_U32", Opcode::S_SUBB_U32),
    ("S_MIN_I32", Opcode::S_MIN_I32),
    ("S_MIN_U32", Opcode::S_MIN_U32),
    ("S_MAX_I32", Opcode::S_MAX_I32),
    ("S_MAX_U32", Opcode::S_MAX_U32),
    ("S_CSELECT_B32", Opcode::S_CSELECT_B32),
    ("S_CSELECT_B64", Opcode::S_CSELECT_B64),
    ("S_AND_B32", Opcode::S_AND_B32),
    ("S_AND_B64", Opcode::S_AND_B64),
    ("S_OR_B32", Opcode::S_OR_B32),
    ("S_OR_B64", Opcode::S_OR_B64),
    ("S_XOR_B32", Opcode::S_XOR_B32),
    ("S_XOR_B64", Opcode::S_XOR_B64),
    ("S_ANDN2_B32", Opcode::S_ANDN2_B32),
    ("S_ANDN2_B64", Opcode::S_ANDN2_B64),
    ("S_ORN2_B32", Opcode::S_ORN2_B32),
    ("S_ORN2_B64", Opcode::S_ORN2_B64),
    ("S_NAND_B32", Opcode::S_NAND_B32),
    ("S_NAND_B64", Opcode::S_NAND_B64),
    ("S_NOR_B32", Opcode::S_NOR_B32),
    ("S_NOR_B64", Opcode::S_NOR_B64),
    ("S_XNOR_B32", Opcode::S_XNOR_B32),
    ("S_XNOR_B64", Opcode::S_XNOR_B64),
    ("S_LSHL_B32", Opcode::S_LSHL_B32),
    ("S_LSHL_B64", Opcode::S_LSHL_B64),
    ("S_LSHR_B32", Opcode::S_LSHR_B32),
    ("S_LSHR_B64", Opcode::S_LSHR_B64),
    ("S_ASHR_I32", Opcode::S_ASHR_I32),
    ("S_ASHR_I64", Opcode::S_ASHR_I64),
    ("S_BFM_B32", Opcode::S_BFM_B32),
    ("S_BFM_B64", Opcode::S_BFM_B64),
    ("S_MUL_I32", Opcode::S_MUL_I32),
    ("S_BFE_U32", Opcode::S_BFE_U32),
    ("S_BFE_I32", Opcode::S_BFE_I32),
    ("S_BFE_U64", Opcode::S_BFE_U64),
    ("S_BFE_I64", Opcode::S_BFE_I64),
    ("S_CBRANCH_G_FORK", Opcode::S_CBRANCH_G_FORK),
    ("S_ABSDIFF_I32", Opcode::S_ABSDIFF_I32),
    ("S_MOVK_I32", Opcode::S_MOVK_I32),
    ("S_CMOVK_I32", Opcode::S_CMOVK_I32),
    ("S_CMPK_EQ_I32", Opcode::S_CMPK_EQ_I32),
    ("S_CMPK_LG_I32", Opcode::S_CMPK_LG_I32),
    ("S_CMPK_GT_I32", Opcode::S_CMPK_GT_I32),
    ("S_CMPK_GE_I32", Opcode::S_CMPK_GE_I32),
    ("S_CMPK_LT_I32", Opcode::S_CMPK_LT_I32),
    ("S_CMPK_LE_I32", Opcode::S_CMPK_LE_I32),
    ("S_CMPK_EQ_U32", Opcode::S_CMPK_EQ_U32),
    ("S_CMPK_LG_U32", Opcode::S_CMPK_LG_U32),
    ("S_CMPK_GT_U32", Opcode::S_CMPK_GT_U32),
    ("S_CMPK_GE_U32", Opcode::S_CMPK_GE_U32),
    ("S_CMPK_LT_U32", Opcode::S_CMPK_LT_U32),
    ("S_CMPK_LE_U32", Opcode::S_CMPK_LE_U32),
    ("S_ADDK_I32", Opcode::S_ADDK_I32),
    ("S_MULK_I32", Opcode::S_MULK_I32),
    ("S_CBRANCH_I_FORK", Opcode::S_CBRANCH_I_FORK),
    ("S_GETREG_B32", Opcode::S_GETREG_B32),
    ("S_SETREG_B32", Opcode::S_SETREG_B32),
    ("S_GETREG_REGRD_B32", Opcode::S_GETREG_REGRD_B32),
    ("S_SETREG_IMM32_B32", Opcode::S_SETREG_IMM32_B32),
    ("S_MOV_B32", Opcode::S_MOV_B32),
    ("S_MOV_B64", Opcode::S_MOV_B64),
    ("S_CMOV_B32", Opcode::S_CMOV_B32),
    ("S_CMOV_B64", Opcode::S_CMOV_B64),
    ("S_NOT_B32", Opcode::S_NOT_B32),
    ("S_NOT_B64", Opcode::S_NOT_B64),
    ("S_WQM_B32", Opcode::S_WQM_B32),
    ("S_WQM_B64", Opcode::S_WQM_B64),
    ("S_BREV_B32", Opcode::S_BREV_B32),
    ("S_BREV_B64", Opcode::S_BREV_B64),
    ("S_BCNT0_I32_B32", Opcode::S_BCNT0_I32_B32),
    ("S_BCNT0_I32_B64", Opcode::S_BCNT0_I32_B64),
    ("S_BCNT1_I32_B32", Opcode::S_BCNT1_I32_B32),
    ("S_BCNT1_I32_B64", Opcode::S_BCNT1_I32_B64),
    ("S_FF0_I32_B32", Opcode::S_FF0_I32_B32),
    ("S_FF0_I32_B64", Opcode::S_FF0_I32_B64),
    ("S_FF1_I32_B32", Opcode::S_FF1_I32_B32),
    ("S_FF1_I32_B64", Opcode::S_FF1_I32_B64),
    ("S_FLBIT_I32_B32", Opcode::S_FLBIT_I32_B32),
    ("S_FLBIT_I32_B64", Opcode::S_FLBIT_I32_B64),
    ("S_FLBIT_I32", Opcode::S_FLBIT_I32),
    ("S_FLBIT_I32_I64", Opcode::S_FLBIT_I32_I64),
    ("S_SEXT_I32_I8", Opcode::S_SEXT_I32_I8),
    ("S_SEXT_I32_I16", Opcode::S_SEXT_I32_I16),
    ("S_BITSET0_B32", Opcode::S_BITSET0_B32),
    ("S_BITSET0_B64", Opcode::S_BITSET0_B64),
    ("S_BITSET1_B32", Opcode::S_BITSET1_B32),
    ("S_BITSET1_B64", Opcode::S_BITSET1_B64),
    ("S_GETPC_B64", Opcode::S_GETPC_B64),
    ("S_SETPC_B64", Opcode::S_SETPC_B64),
    ("S_SWAPPC_B64", Opcode::S_SWAPPC_B64),
    ("S_RFE_B64", Opcode::S_RFE_B64),
    ("S_AND_SAVEEXEC_B64", Opcode::S_AND_SAVEEXEC_B64),
    ("S_OR_SAVEEXEC_B64", Opcode::S_OR_SAVEEXEC_B64),
    ("S_XOR_SAVEEXEC_B64", Opcode::S_XOR_SAVEEXEC_B64),
    ("S_ANDN2_SAVEEXEC_B64", Opcode::S_ANDN2_SAVEEXEC_B64),
    ("S_ORN2_SAVEEXEC_B64", Opcode::S_ORN2_SAVEEXEC_B64),
    ("S_NAND_SAVEEXEC_B64", Opcode::S_NAND_SAVEEXEC_B64),
    ("S_NOR_SAVEEXEC_B64", Opcode::S_NOR_SAVEEXEC_B64),
    ("S_XNOR_SAVEEXEC_B64", Opcode::S_XNOR_SAVEEXEC_B64),
    ("S_QUADMASK_B32", Opcode::S_QUADMASK_B32),
    ("S_QUADMASK_B64", Opcode::S_QUADMASK_B64),
    ("S_MOVRELS_B32", Opcode::S_MOVRELS_B32),
    ("S_MOVRELS_B64", Opcode::S_MOVRELS_B64),
    ("S_MOVRELD_B32", Opcode::S_MOVRELD_B32),
    ("S_MOVRELD_B64", Opcode::S_MOVRELD_B64),
    ("S_CBRANCH_JOIN", Opcode::S_CBRANCH_JOIN),
    ("S_MOV_REGRD_B32", Opcode::S_MOV_REGRD_B32),
    ("S_ABS_I32", Opcode::S_ABS_I32),
    ("S_MOV_FED_B32", Opcode::S_MOV_FED_B32),
    ("S_CMP_EQ_I32", Opcode::S_CMP_EQ_I32),
    ("S_CMP_LG_I32", Opcode::S_CMP_LG_I32),
    ("S_CMP_GT_I32", Opcode::S_CMP_GT_I32),
    ("S_CMP_GE_I32", Opcode::S_CMP_GE_I32),
    ("S_CMP_LT_I32", Opcode::S_CMP_LT_I32),
    ("S_CMP_LE_I32", Opcode::S_CMP_LE_I32),
    ("S_CMP_EQ_U32", Opcode::S_CMP_EQ_U32),
    ("S_CMP_LG_U32", Opcode::S_CMP_LG_U32),
    ("S_CMP_GT_U32", Opcode::S_CMP_GT_U32),
    ("S_CMP_GE_U32", Opcode::S_CMP_GE_U32),
    ("S_CMP_LT_U32", Opcode::S_CMP_LT_U32),
    ("S_CMP_LE_U32", Opcode::S_CMP_LE_U32),
    ("S_BITCMP0_B32", Opcode::S_BITCMP0_B32),
    ("S_BITCMP1_B32", Opcode::S_BITCMP1_B32),
    ("S_BITCMP0_B64", Opcode::S_BITCMP0_B64),
    ("S_BITCMP1_B64", Opcode::S_BITCMP1_B64),
    ("S_SETVSKIP", Opcode::S_SETVSKIP),
    ("S_NOP", Opcode::S_NOP),
    ("S_ENDPGM", Opcode::S_ENDPGM),
    ("S_BRANCH", Opcode::S_BRANCH),
    ("S_CBRANCH_SCC0", Opcode::S_CBRANCH_SCC0),
    ("S_CBRANCH_SCC1", Opcode::S_CBRANCH_SCC1),
    ("S_CBRANCH_VCCZ", Opcode::S_CBRANCH_VCCZ),
    ("S_CBRANCH_VCCNZ", Opcode::S_CBRANCH_VCCNZ),
    ("S_CBRANCH_EXECZ", Opcode::S_CBRANCH_EXECZ),
    ("S_CBRANCH_EXECNZ", Opcode::S_CBRANCH_EXECNZ),
    ("S_BARRIER", Opcode::S_BARRIER),
    ("S_SETKILL", Opcode::S_SETKILL),
    ("S_WAITCNT", Opcode::S_WAITCNT),
    ("S_SETHALT", Opcode::S_SETHALT),
    ("S_SLEEP", Opcode::S_SLEEP),
    ("S_SETPRIO", Opcode::S_SETPRIO),
    ("S_SENDMSG", Opcode::S_SENDMSG),
    ("S_SENDMSGHALT", Opcode::S_SENDMSGHALT),
    ("S_TRAP", Opcode::S_TRAP),
    ("S_ICACHE_INV", Opcode::S_ICACHE_INV),
    ("S_INCPERFLEVEL", Opcode::S_INCPERFLEVEL),
    ("S_DECPERFLEVEL", Opcode::S_DECPERFLEVEL),
    ("S_TTRACEDATA", Opcode::S_TTRACEDATA),
    ("S_CBRANCH_CDBGSYS", Opcode::S_CBRANCH_CDBGSYS),
    ("S_CBRANCH_CDBGUSER", Opcode::S_CBRANCH_CDBGUSER),
    (
        "S_CBRANCH_CDBGSYS_OR_USER",
        Opcode::S_CBRANCH_CDBGSYS_OR_USER,
    ),
    (
        "S_CBRANCH_CDBGSYS_AND_USER",
        Opcode::S_CBRANCH_CDBGSYS_AND_USER,
    ),
    ("V_CMP_F_F32", Opcode::V_CMP_F_F32),
    ("V_CMP_LT_F32", Opcode::V_CMP_LT_F32),
    ("V_CMP_EQ_F32", Opcode::V_CMP_EQ_F32),
    ("V_CMP_LE_F32", Opcode::V_CMP_LE_F32),
    ("V_CMP_GT_F32", Opcode::V_CMP_GT_F32),
    ("V_CMP_LG_F32", Opcode::V_CMP_LG_F32),
    ("V_CMP_GE_F32", Opcode::V_CMP_GE_F32),
    ("V_CMP_O_F32", Opcode::V_CMP_O_F32),
    ("V_CMP_U_F32", Opcode::V_CMP_U_F32),
    ("V_CMP_NGE_F32", Opcode::V_CMP_NGE_F32),
    ("V_CMP_NLG_F32", Opcode::V_CMP_NLG_F32),
    ("V_CMP_NGT_F32", Opcode::V_CMP_NGT_F32),
    ("V_CMP_NLE_F32", Opcode::V_CMP_NLE_F32),
    ("V_CMP_NEQ_F32", Opcode::V_CMP_NEQ_F32),
    ("V_CMP_NLT_F32", Opcode::V_CMP_NLT_F32),
    ("V_CMP_TRU_F32", Opcode::V_CMP_TRU_F32),
    ("V_CMP_T_F32", Opcode::V_CMP_T_F32),
    ("V_CMPX_F_F32", Opcode::V_CMPX_F_F32),
    ("V_CMPX_LT_F32", Opcode::V_CMPX_LT_F32),
    ("V_CMPX_EQ_F32", Opcode::V_CMPX_EQ_F32),
    ("V_CMPX_LE_F32", Opcode::V_CMPX_LE_F32),
    ("V_CMPX_GT_F32", Opcode::V_CMPX_GT_F32),
    ("V_CMPX_LG_F32", Opcode::V_CMPX_LG_F32),
    ("V_CMPX_GE_F32", Opcode::V_CMPX_GE_F32),
    ("V_CMPX_O_F32", Opcode::V_CMPX_O_F32),
    ("V_CMPX_U_F32", Opcode::V_CMPX_U_F32),
    ("V_CMPX_NGE_F32", Opcode::V_CMPX_NGE_F32),
    ("V_CMPX_NLG_F32", Opcode::V_CMPX_NLG_F32),
    ("V_CMPX_NGT_F32", Opcode::V_CMPX_NGT_F32),
    ("V_CMPX_NLE_F32", Opcode::V_CMPX_NLE_F32),
    ("V_CMPX_NEQ_F32", Opcode::V_CMPX_NEQ_F32),
    ("V_CMPX_NLT_F32", Opcode::V_CMPX_NLT_F32),
    ("V_CMPX_TRU_F32", Opcode::V_CMPX_TRU_F32),
    ("V_CMPX_T_F32", Opcode::V_CMPX_T_F32),
    ("V_CMP_F_F64", Opcode::V_CMP_F_F64),
    ("V_CMP_LT_F64", Opcode::V_CMP_LT_F64),
    ("V_CMP_EQ_F64", Opcode::V_CMP_EQ_F64),
    ("V_CMP_LE_F64", Opcode::V_CMP_LE_F64),
    ("V_CMP_GT_F64", Opcode::V_CMP_GT_F64),
    ("V_CMP_LG_F64", Opcode::V_CMP_LG_F64),
    ("V_CMP_GE_F64", Opcode::V_CMP_GE_F64),
    ("V_CMP_O_F64", Opcode::V_CMP_O_F64),
    ("V_CMP_U_F64", Opcode::V_CMP_U_F64),
    ("V_CMP_NGE_F64", Opcode::V_CMP_NGE_F64),
    ("V_CMP_NLG_F64", Opcode::V_CMP_NLG_F64),
    ("V_CMP_NGT_F64", Opcode::V_CMP_NGT_F64),
    ("V_CMP_NLE_F64", Opcode::V_CMP_NLE_F64),
    ("V_CMP_NEQ_F64", Opcode::V_CMP_NEQ_F64),
    ("V_CMP_NLT_F64", Opcode::V_CMP_NLT_F64),
    ("V_CMP_TRU_F64", Opcode::V_CMP_TRU_F64),
    ("V_CMP_T_F64", Opcode::V_CMP_T_F64),
    ("V_CMPX_F_F64", Opcode::V_CMPX_F_F64),
    ("V_CMPX_LT_F64", Opcode::V_CMPX_LT_F64),
    ("V_CMPX_EQ_F64", Opcode::V_CMPX_EQ_F64),
    ("V_CMPX_LE_F64", Opcode::V_CMPX_LE_F64),
    ("V_CMPX_GT_F64", Opcode::V_CMPX_GT_F64),
    ("V_CMPX_LG_F64", Opcode::V_CMPX_LG_F64),
    ("V_CMPX_GE_F64", Opcode::V_CMPX_GE_F64),
    ("V_CMPX_O_F64", Opcode::V_CMPX_O_F64),
    ("V_CMPX_U_F64", Opcode::V_CMPX_U_F64),
    ("V_CMPX_NGE_F64", Opcode::V_CMPX_NGE_F64),
    ("V_CMPX_NLG_F64", Opcode::V_CMPX_NLG_F64),
    ("V_CMPX_NGT_F64", Opcode::V_CMPX_NGT_F64),
    ("V_CMPX_NLE_F64", Opcode::V_CMPX_NLE_F64),
    ("V_CMPX_NEQ_F64", Opcode::V_CMPX_NEQ_F64),
    ("V_CMPX_NLT_F64", Opcode::V_CMPX_NLT_F64),
    ("V_CMPX_TRU_F64", Opcode::V_CMPX_TRU_F64),
    ("V_CMPX_T_F64", Opcode::V_CMPX_T_F64),
    ("V_CMPS_F_F32", Opcode::V_CMPS_F_F32),
    ("V_CMPS_LT_F32", Opcode::V_CMPS_LT_F32),
    ("V_CMPS_EQ_F32", Opcode::V_CMPS_EQ_F32),
    ("V_CMPS_LE_F32", Opcode::V_CMPS_LE_F32),
    ("V_CMPS_GT_F32", Opcode::V_CMPS_GT_F32),
    ("V_CMPS_LG_F32", Opcode::V_CMPS_LG_F32),
    ("V_CMPS_GE_F32", Opcode::V_CMPS_GE_F32),
    ("V_CMPS_O_F32", Opcode::V_CMPS_O_F32),
    ("V_CMPS_U_F32", Opcode::V_CMPS_U_F32),
    ("V_CMPS_NGE_F32", Opcode::V_CMPS_NGE_F32),
    ("V_CMPS_NLG_F32", Opcode::V_CMPS_NLG_F32),
    ("V_CMPS_NGT_F32", Opcode::V_CMPS_NGT_F32),
    ("V_CMPS_NLE_F32", Opcode::V_CMPS_NLE_F32),
    ("V_CMPS_NEQ_F32", Opcode::V_CMPS_NEQ_F32),
    ("V_CMPS_NLT_F32", Opcode::V_CMPS_NLT_F32),
    ("V_CMPS_TRU_F32", Opcode::V_CMPS_TRU_F32),
    ("V_CMPS_T_F32", Opcode::V_CMPS_T_F32),
    ("V_CMPSX_F_F32", Opcode::V_CMPSX_F_F32),
    ("V_CMPSX_LT_F32", Opcode::V_CMPSX_LT_F32),
    ("V_CMPSX_EQ_F32", Opcode::V_CMPSX_EQ_F32),
    ("V_CMPSX_LE_F32", Opcode::V_CMPSX_LE_F32),
    ("V_CMPSX_GT_F32", Opcode::V_CMPSX_GT_F32),
    ("V_CMPSX_LG_F32", Opcode::V_CMPSX_LG_F32),
    ("V_CMPSX_GE_F32", Opcode::V_CMPSX_GE_F32),
    ("V_CMPSX_O_F32", Opcode::V_CMPSX_O_F32),
    ("V_CMPSX_U_F32", Opcode::V_CMPSX_U_F32),
    ("V_CMPSX_NGE_F32", Opcode::V_CMPSX_NGE_F32),
    ("V_CMPSX_NLG_F32", Opcode::V_CMPSX_NLG_F32),
    ("V_CMPSX_NGT_F32", Opcode::V_CMPSX_NGT_F32),
    ("V_CMPSX_NLE_F32", Opcode::V_CMPSX_NLE_F32),
    ("V_CMPSX_NEQ_F32", Opcode::V_CMPSX_NEQ_F32),
    ("V_CMPSX_NLT_F32", Opcode::V_CMPSX_NLT_F32),
    ("V_CMPSX_TRU_F32", Opcode::V_CMPSX_TRU_F32),
    ("V_CMPSX_T_F32", Opcode::V_CMPSX_T_F32),
    ("V_CMPS_F_F64", Opcode::V_CMPS_F_F64),
    ("V_CMPS_LT_F64", Opcode::V_CMPS_LT_F64),
    ("V_CMPS_EQ_F64", Opcode::V_CMPS_EQ_F64),
    ("V_CMPS_LE_F64", Opcode::V_CMPS_LE_F64),
    ("V_CMPS_GT_F64", Opcode::V_CMPS_GT_F64),
    ("V_CMPS_LG_F64", Opcode::V_CMPS_LG_F64),
    ("V_CMPS_GE_F64", Opcode::V_CMPS_GE_F64),
    ("V_CMPS_O_F64", Opcode::V_CMPS_O_F64),
    ("V_CMPS_U_F64", Opcode::V_CMPS_U_F64),
    ("V_CMPS_NGE_F64", Opcode::V_CMPS_NGE_F64),
    ("V_CMPS_NLG_F64", Opcode::V_CMPS_NLG_F64),
    ("V_CMPS_NGT_F64", Opcode::V_CMPS_NGT_F64),
    ("V_CMPS_NLE_F64", Opcode::V_CMPS_NLE_F64),
    ("V_CMPS_NEQ_F64", Opcode::V_CMPS_NEQ_F64),
    ("V_CMPS_NLT_F64", Opcode::V_CMPS_NLT_F64),
    ("V_CMPS_TRU_F64", Opcode::V_CMPS_TRU_F64),
    ("V_CMPS_T_F64", Opcode::V_CMPS_T_F64),
    ("V_CMPSX_F_F64", Opcode::V_CMPSX_F_F64),
    ("V_CMPSX_LT_F64", Opcode::V_CMPSX_LT_F64),
    ("V_CMPSX_EQ_F64", Opcode::V_CMPSX_EQ_F64),
    ("V_CMPSX_LE_F64", Opcode::V_CMPSX_LE_F64),
    ("V_CMPSX_GT_F64", Opcode::V_CMPSX_GT_F64),
    ("V_CMPSX_LG_F64", Opcode::V_CMPSX_LG_F64),
    ("V_CMPSX_GE_F64", Opcode::V_CMPSX_GE_F64),
    ("V_CMPSX_O_F64", Opcode::V_CMPSX_O_F64),
    ("V_CMPSX_U_F64", Opcode::V_CMPSX_U_F64),
    ("V_CMPSX_NGE_F64", Opcode::V_CMPSX_NGE_F64),
    ("V_CMPSX_NLG_F64", Opcode::V_CMPSX_NLG_F64),
    ("V_CMPSX_NGT_F64", Opcode::V_CMPSX_NGT_F64),
    ("V_CMPSX_NLE_F64", Opcode::V_CMPSX_NLE_F64),
    ("V_CMPSX_NEQ_F64", Opcode::V_CMPSX_NEQ_F64),
    ("V_CMPSX_NLT_F64", Opcode::V_CMPSX_NLT_F64),
    ("V_CMPSX_TRU_F64", Opcode::V_CMPSX_TRU_F64),
    ("V_CMPSX_T_F64", Opcode::V_CMPSX_T_F64),
    ("V_CMP_F_I32", Opcode::V_CMP_F_I32),
    ("V_CMP_LT_I32", Opcode::V_CMP_LT_I32),
    ("V_CMP_EQ_I32", Opcode::V_CMP_EQ_I32),
    ("V_CMP_LE_I32", Opcode::V_CMP_LE_I32),
    ("V_CMP_GT_I32", Opcode::V_CMP_GT_I32),
    ("V_CMP_NE_I32", Opcode::V_CMP_NE_I32),
    ("V_CMP_GE_I32", Opcode::V_CMP_GE_I32),
    ("V_CMP_TRU_I32", Opcode::V_CMP_TRU_I32),
    ("V_CMP_T_I32", Opcode::V_CMP_T_I32),
    ("V_CMP_CLASS_F32", Opcode::V_CMP_CLASS_F32),
    ("V_CMP_LT_I16", Opcode::V_CMP_LT_I16),
    ("V_CMP_EQ_I16", Opcode::V_CMP_EQ_I16),
    ("V_CMP_LE_I16", Opcode::V_CMP_LE_I16),
    ("V_CMP_GT_I16", Opcode::V_CMP_GT_I16),
    ("V_CMP_NE_I16", Opcode::V_CMP_NE_I16),
    ("V_CMP_GE_I16", Opcode::V_CMP_GE_I16),
    ("V_CMPX_F_I32", Opcode::V_CMPX_F_I32),
    ("V_CMPX_LT_I32", Opcode::V_CMPX_LT_I32),
    ("V_CMPX_EQ_I32", Opcode::V_CMPX_EQ_I32),
    ("V_CMPX_LE_I32", Opcode::V_CMPX_LE_I32),
    ("V_CMPX_GT_I32", Opcode::V_CMPX_GT_I32),
    ("V_CMPX_LG_I32", Opcode::V_CMPX_LG_I32),
    ("V_CMPX_NE_I32", Opcode::V_CMPX_NE_I32),
    ("V_CMPX_GE_I32", Opcode::V_CMPX_GE_I32),
    ("V_CMPX_TRU_I32", Opcode::V_CMPX_TRU_I32),
    ("V_CMPX_T_I32", Opcode::V_CMPX_T_I32),
    ("V_CMPX_CLASS_F32", Opcode::V_CMPX_CLASS_F32),
    ("V_CMP_F_I64", Opcode::V_CMP_F_I64),
    ("V_CMP_LT_I64", Opcode::V_CMP_LT_I64),
    ("V_CMP_EQ_I64", Opcode::V_CMP_EQ_I64),
    ("V_CMP_LE_I64", Opcode::V_CMP_LE_I64),
    ("V_CMP_GT_I64", Opcode::V_CMP_GT_I64),
    ("V_CMP_LG_I64", Opcode::V_CMP_LG_I64),
    ("V_CMP_NE_I64", Opcode::V_CMP_NE_I64),
    ("V_CMP_GE_I64", Opcode::V_CMP_GE_I64),
    ("V_CMP_TRU_I64", Opcode::V_CMP_TRU_I64),
    ("V_CMP_T_I64", Opcode::V_CMP_T_I64),
    ("V_CMP_CLASS_F64", Opcode::V_CMP_CLASS_F64),
    ("V_CMP_LT_U16", Opcode::V_CMP_LT_U16),
    ("V_CMP_EQ_U16", Opcode::V_CMP_EQ_U16),
    ("V_CMP_LE_U16", Opcode::V_CMP_LE_U16),
    ("V_CMP_GT_U16", Opcode::V_CMP_GT_U16),
    ("V_CMP_NE_U16", Opcode::V_CMP_NE_U16),
    ("V_CMP_GE_U16", Opcode::V_CMP_GE_U16),
    ("V_CMPX_F_I64", Opcode::V_CMPX_F_I64),
    ("V_CMPX_LT_I64", Opcode::V_CMPX_LT_I64),
    ("V_CMPX_EQ_I64", Opcode::V_CMPX_EQ_I64),
    ("V_CMPX_LE_I64", Opcode::V_CMPX_LE_I64),
    ("V_CMPX_GT_I64", Opcode::V_CMPX_GT_I64),
    ("V_CMPX_LG_I64", Opcode::V_CMPX_LG_I64),
    ("V_CMPX_NE_I64", Opcode::V_CMPX_NE_I64),
    ("V_CMPX_GE_I64", Opcode::V_CMPX_GE_I64),
    ("V_CMPX_TRU_I64", Opcode::V_CMPX_TRU_I64),
    ("V_CMPX_T_I64", Opcode::V_CMPX_T_I64),
    ("V_CMPX_CLASS_F64", Opcode::V_CMPX_CLASS_F64),
    ("V_CMP_F_U32", Opcode::V_CMP_F_U32),
    ("V_CMP_LT_U32", Opcode::V_CMP_LT_U32),
    ("V_CMP_EQ_U32", Opcode::V_CMP_EQ_U32),
    ("V_CMP_LE_U32", Opcode::V_CMP_LE_U32),
    ("V_CMP_GT_U32", Opcode::V_CMP_GT_U32),
    ("V_CMP_NE_U32", Opcode::V_CMP_NE_U32),
    ("V_CMP_GE_U32", Opcode::V_CMP_GE_U32),
    ("V_CMP_TRU_U32", Opcode::V_CMP_TRU_U32),
    ("V_CMP_T_U32", Opcode::V_CMP_T_U32),
    ("V_CMP_F_F16", Opcode::V_CMP_F_F16),
    ("V_CMP_LT_F16", Opcode::V_CMP_LT_F16),
    ("V_CMP_EQ_F16", Opcode::V_CMP_EQ_F16),
    ("V_CMP_LE_F16", Opcode::V_CMP_LE_F16),
    ("V_CMP_GT_F16", Opcode::V_CMP_GT_F16),
    ("V_CMP_LG_F16", Opcode::V_CMP_LG_F16),
    ("V_CMP_GE_F16", Opcode::V_CMP_GE_F16),
    ("V_CMP_O_F16", Opcode::V_CMP_O_F16),
    ("V_CMPX_F_U32", Opcode::V_CMPX_F_U32),
    ("V_CMPX_LT_U32", Opcode::V_CMPX_LT_U32),
    ("V_CMPX_EQ_U32", Opcode::V_CMPX_EQ_U32),
    ("V_CMPX_LE_U32", Opcode::V_CMPX_LE_U32),
    ("V_CMPX_GT_U32", Opcode::V_CMPX_GT_U32),
    ("V_CMPX_NE_U32", Opcode::V_CMPX_NE_U32),
    ("V_CMPX_GE_U32", Opcode::V_CMPX_GE_U32),
    ("V_CMPX_TRU_U32", Opcode::V_CMPX_TRU_U32),
    ("V_CMPX_T_U32", Opcode::V_CMPX_T_U32),
    ("V_CMP_F_U64", Opcode::V_CMP_F_U64),
    ("V_CMP_LT_U64", Opcode::V_CMP_LT_U64),
    ("V_CMP_EQ_U64", Opcode::V_CMP_EQ_U64),
    ("V_CMP_LE_U64", Opcode::V_CMP_LE_U64),
    ("V_CMP_GT_U64", Opcode::V_CMP_GT_U64),
    ("V_CMP_LG_U64", Opcode::V_CMP_LG_U64),
    ("V_CMP_NE_U64", Opcode::V_CMP_NE_U64),
    ("V_CMP_GE_U64", Opcode::V_CMP_GE_U64),
    ("V_CMP_TRU_U64", Opcode::V_CMP_TRU_U64),
    ("V_CMP_T_U64", Opcode::V_CMP_T_U64),
    ("V_CMPX_F_U64", Opcode::V_CMPX_F_U64),
    ("V_CMPX_LT_U64", Opcode::V_CMPX_LT_U64),
    ("V_CMPX_EQ_U64", Opcode::V_CMPX_EQ_U64),
    ("V_CMPX_LE_U64", Opcode::V_CMPX_LE_U64),
    ("V_CMPX_GT_U64", Opcode::V_CMPX_GT_U64),
    ("V_CMPX_LG_U64", Opcode::V_CMPX_LG_U64),
    ("V_CMPX_NE_U64", Opcode::V_CMPX_NE_U64),
    ("V_CMPX_GE_U64", Opcode::V_CMPX_GE_U64),
    ("V_CMPX_TRU_U64", Opcode::V_CMPX_TRU_U64),
    ("V_CMPX_T_U64", Opcode::V_CMPX_T_U64),
    ("V_CNDMASK_B32", Opcode::V_CNDMASK_B32),
    ("V_READLANE_B32", Opcode::V_READLANE_B32),
    ("V_WRITELANE_B32", Opcode::V_WRITELANE_B32),
    ("V_ADD_F32", Opcode::V_ADD_F32),
    ("V_SUB_F32", Opcode::V_SUB_F32),
    ("V_SUBREV_F32", Opcode::V_SUBREV_F32),
    ("V_MAC_LEGACY_F32", Opcode::V_MAC_LEGACY_F32),
    ("V_MUL_LEGACY_F32", Opcode::V_MUL_LEGACY_F32),
    ("V_MUL_F32", Opcode::V_MUL_F32),
    ("V_MUL_I32_I24", Opcode::V_MUL_I32_I24),
    ("V_MUL_HI_I32_I24", Opcode::V_MUL_HI_I32_I24),
    ("V_MUL_U32_U24", Opcode::V_MUL_U32_U24),
    ("V_MUL_HI_U32_U24", Opcode::V_MUL_HI_U32_U24),
    ("V_MIN_LEGACY_F32", Opcode::V_MIN_LEGACY_F32),
    ("V_MAX_LEGACY_F32", Opcode::V_MAX_LEGACY_F32),
    ("V_MIN_F32", Opcode::V_MIN_F32),
    ("V_MAX_F32", Opcode::V_MAX_F32),
    ("V_MIN_I32", Opcode::V_MIN_I32),
    ("V_MAX_I32", Opcode::V_MAX_I32),
    ("V_MIN_U32", Opcode::V_MIN_U32),
    ("V_MAX_U32", Opcode::V_MAX_U32),
    ("V_LSHR_B32", Opcode::V_LSHR_B32),
    ("V_LSHRREV_B32", Opcode::V_LSHRREV_B32),
    ("V_ASHR_I32", Opcode::V_ASHR_I32),
    ("V_ASHRREV_I32", Opcode::V_ASHRREV_I32),
    ("V_LSHL_B32", Opcode::V_LSHL_B32),
    ("V_LSHLREV_B32", Opcode::V_LSHLREV_B32),
    ("V_AND_B32", Opcode::V_AND_B32),
    ("V_OR_B32", Opcode::V_OR_B32),
    ("V_XOR_B32", Opcode::V_XOR_B32),
    ("V_BFM_B32", Opcode::V_BFM_B32),
    ("V_MAC_F32", Opcode::V_MAC_F32),
    ("V_MADMK_F32", Opcode::V_MADMK_F32),
    ("V_MADAK_F32", Opcode::V_MADAK_F32),
    ("V_BCNT_U32_B32", Opcode::V_BCNT_U32_B32),
    ("V_MBCNT_LO_U32_B32", Opcode::V_MBCNT_LO_U32_B32),
    ("V_MBCNT_HI_U32_B32", Opcode::V_MBCNT_HI_U32_B32),
    ("V_ADD_I32", Opcode::V_ADD_I32),
    ("V_SUB_I32", Opcode::V_SUB_I32),
    ("V_SUBREV_I32", Opcode::V_SUBREV_I32),
    ("V_ADDC_U32", Opcode::V_ADDC_U32),
    ("V_SUBB_U32", Opcode::V_SUBB_U32),
    ("V_SUBBREV_U32", Opcode::V_SUBBREV_U32),
    ("V_LDEXP_F32", Opcode::V_LDEXP_F32),
    ("V_CVT_PKACCUM_U8_F32", Opcode::V_CVT_PKACCUM_U8_F32),
    ("V_CVT_PKNORM_I16_F32", Opcode::V_CVT_PKNORM_I16_F32),
    ("V_CVT_PKNORM_U16_F32", Opcode::V_CVT_PKNORM_U16_F32),
    ("V_CVT_PKRTZ_F16_F32", Opcode::V_CVT_PKRTZ_F16_F32),
    ("V_CVT_PK_U16_U32", Opcode::V_CVT_PK_U16_U32),
    ("V_CVT_PK_I16_I32", Opcode::V_CVT_PK_I16_I32),
    ("V_ADD_F16", Opcode::V_ADD_F16),
    ("V_SUB_F16", Opcode::V_SUB_F16),
    ("V_SUBREV_F16", Opcode::V_SUBREV_F16),
    ("V_MUL_F16", Opcode::V_MUL_F16),
    ("V_MAX_F16", Opcode::V_MAX_F16),
    ("V_MIN_F16", Opcode::V_MIN_F16),
    ("V_LDEXP_F16", Opcode::V_LDEXP_F16),
    ("V_NOP", Opcode::V_NOP),
    ("V_MOV_B32", Opcode::V_MOV_B32),
    ("V_READFIRSTLANE_B32", Opcode::V_READFIRSTLANE_B32),
    ("V_CVT_I32_F64", Opcode::V_CVT_I32_F64),
    ("V_CVT_F64_I32", Opcode::V_CVT_F64_I32),
    ("V_CVT_F32_I32", Opcode::V_CVT_F32_I32),
    ("V_CVT_F32_U32", Opcode::V_CVT_F32_U32),
    ("V_CVT_U32_F32", Opcode::V_CVT_U32_F32),
    ("V_CVT_I32_F32", Opcode::V_CVT_I32_F32),
    ("V_MOV_FED_B32", Opcode::V_MOV_FED_B32),
    ("V_CVT_F16_F32", Opcode::V_CVT_F16_F32),
    ("V_CVT_F32_F16", Opcode::V_CVT_F32_F16),
    ("V_CVT_RPI_I32_F32", Opcode::V_CVT_RPI_I32_F32),
    ("V_CVT_FLR_I32_F32", Opcode::V_CVT_FLR_I32_F32),
    ("V_CVT_OFF_F32_I4", Opcode::V_CVT_OFF_F32_I4),
    ("V_CVT_F32_F64", Opcode::V_CVT_F32_F64),
    ("V_CVT_F64_F32", Opcode::V_CVT_F64_F32),
    ("V_CVT_F32_UBYTE0", Opcode::V_CVT_F32_UBYTE0),
    ("V_CVT_F32_UBYTE1", Opcode::V_CVT_F32_UBYTE1),
    ("V_CVT_F32_UBYTE2", Opcode::V_CVT_F32_UBYTE2),
    ("V_CVT_F32_UBYTE3", Opcode::V_CVT_F32_UBYTE3),
    ("V_CVT_U32_F64", Opcode::V_CVT_U32_F64),
    ("V_CVT_F64_U32", Opcode::V_CVT_F64_U32),
    ("V_TRUNC_F64", Opcode::V_TRUNC_F64),
    ("V_CEIL_F64", Opcode::V_CEIL_F64),
    ("V_RNDNE_F64", Opcode::V_RNDNE_F64),
    ("V_FLOOR_F64", Opcode::V_FLOOR_F64),
    ("V_FRACT_F32", Opcode::V_FRACT_F32),
    ("V_TRUNC_F32", Opcode::V_TRUNC_F32),
    ("V_CEIL_F32", Opcode::V_CEIL_F32),
    ("V_RNDNE_F32", Opcode::V_RNDNE_F32),
    ("V_FLOOR_F32", Opcode::V_FLOOR_F32),
    ("V_EXP_F32", Opcode::V_EXP_F32),
    ("V_LOG_CLAMP_F32", Opcode::V_LOG_CLAMP_F32),
    ("V_LOG_F32", Opcode::V_LOG_F32),
    ("V_RCP_CLAMP_F32", Opcode::V_RCP_CLAMP_F32),
    ("V_RCP_LEGACY_F32", Opcode::V_RCP_LEGACY_F32),
    ("V_RCP_F32", Opcode::V_RCP_F32),
    ("V_RCP_IFLAG_F32", Opcode::V_RCP_IFLAG_F32),
    ("V_RSQ_CLAMP_F32", Opcode::V_RSQ_CLAMP_F32),
    ("V_RSQ_LEGACY_F32", Opcode::V_RSQ_LEGACY_F32),
    ("V_RSQ_F32", Opcode::V_RSQ_F32),
    ("V_RCP_F64", Opcode::V_RCP_F64),
    ("V_RCP_CLAMP_F64", Opcode::V_RCP_CLAMP_F64),
    ("V_RSQ_F64", Opcode::V_RSQ_F64),
    ("V_RSQ_CLAMP_F64", Opcode::V_RSQ_CLAMP_F64),
    ("V_SQRT_F32", Opcode::V_SQRT_F32),
    ("V_SQRT_F64", Opcode::V_SQRT_F64),
    ("V_SIN_F32", Opcode::V_SIN_F32),
    ("V_COS_F32", Opcode::V_COS_F32),
    ("V_NOT_B32", Opcode::V_NOT_B32),
    ("V_BFREV_B32", Opcode::V_BFREV_B32),
    ("V_FFBH_U32", Opcode::V_FFBH_U32),
    ("V_FFBL_B32", Opcode::V_FFBL_B32),
    ("V_FFBH_I32", Opcode::V_FFBH_I32),
    ("V_FREXP_EXP_I32_F64", Opcode::V_FREXP_EXP_I32_F64),
    ("V_FREXP_MANT_F64", Opcode::V_FREXP_MANT_F64),
    ("V_FRACT_F64", Opcode::V_FRACT_F64),
    ("V_FREXP_EXP_I32_F32", Opcode::V_FREXP_EXP_I32_F32),
    ("V_FREXP_MANT_F32", Opcode::V_FREXP_MANT_F32),
    ("V_CLREXCP", Opcode::V_CLREXCP),
    ("V_MOVRELD_B32", Opcode::V_MOVRELD_B32),
    ("V_MOVRELS_B32", Opcode::V_MOVRELS_B32),
    ("V_MOVRELSD_B32", Opcode::V_MOVRELSD_B32),
    ("V_LOG_LEGACY_F32", Opcode::V_LOG_LEGACY_F32),
    ("V_EXP_LEGACY_F32", Opcode::V_EXP_LEGACY_F32),
    ("V_RCP_F16", Opcode::V_RCP_F16),
    ("V_SQRT_F16", Opcode::V_SQRT_F16),
    ("V_MAD_LEGACY_F32", Opcode::V_MAD_LEGACY_F32),
    ("V_MAD_F32", Opcode::V_MAD_F32),
    ("V_MAD_I32_I24", Opcode::V_MAD_I32_I24),
    ("V_MAD_U32_U24", Opcode::V_MAD_U32_U24),
    ("V_CUBEID_F32", Opcode::V_CUBEID_F32),
    ("V_CUBESC_F32", Opcode::V_CUBESC_F32),
    ("V_CUBETC_F32", Opcode::V_CUBETC_F32),
    ("V_CUBEMA_F32", Opcode::V_CUBEMA_F32),
    ("V_BFE_U32", Opcode::V_BFE_U32),
    ("V_BFE_I32", Opcode::V_BFE_I32),
    ("V_BFI_B32", Opcode::V_BFI_B32),
    ("V_FMA_F32", Opcode::V_FMA_F32),
    ("V_FMA_F64", Opcode::V_FMA_F64),
    ("V_LERP_U8", Opcode::V_LERP_U8),
    ("V_ALIGNBIT_B32", Opcode::V_ALIGNBIT_B32),
    ("V_ALIGNBYTE_B32", Opcode::V_ALIGNBYTE_B32),
    ("V_MULLIT_F32", Opcode::V_MULLIT_F32),
    ("V_MIN3_F32", Opcode::V_MIN3_F32),
    ("V_MIN3_I32", Opcode::V_MIN3_I32),
    ("V_MIN3_U32", Opcode::V_MIN3_U32),
    ("V_MAX3_F32", Opcode::V_MAX3_F32),
    ("V_MAX3_I32", Opcode::V_MAX3_I32),
    ("V_MAX3_U32", Opcode::V_MAX3_U32),
    ("V_MED3_F32", Opcode::V_MED3_F32),
    ("V_MED3_I32", Opcode::V_MED3_I32),
    ("V_MED3_U32", Opcode::V_MED3_U32),
    ("V_SAD_U8", Opcode::V_SAD_U8),
    ("V_SAD_HI_U8", Opcode::V_SAD_HI_U8),
    ("V_SAD_U16", Opcode::V_SAD_U16),
    ("V_SAD_U32", Opcode::V_SAD_U32),
    ("V_CVT_PK_U8_F32", Opcode::V_CVT_PK_U8_F32),
    ("V_DIV_FIXUP_F32", Opcode::V_DIV_FIXUP_F32),
    ("V_DIV_FIXUP_F64", Opcode::V_DIV_FIXUP_F64),
    ("V_LSHL_B64", Opcode::V_LSHL_B64),
    ("V_LSHR_B64", Opcode::V_LSHR_B64),
    ("V_ASHR_I64", Opcode::V_ASHR_I64),
    ("V_ADD_F64", Opcode::V_ADD_F64),
    ("V_MUL_F64", Opcode::V_MUL_F64),
    ("V_MIN_F64", Opcode::V_MIN_F64),
    ("V_MAX_F64", Opcode::V_MAX_F64),
    ("V_LDEXP_F64", Opcode::V_LDEXP_F64),
    ("V_MUL_LO_U32", Opcode::V_MUL_LO_U32),
    ("V_MUL_HI_U32", Opcode::V_MUL_HI_U32),
    ("V_MUL_LO_I32", Opcode::V_MUL_LO_I32),
    ("V_MUL_HI_I32", Opcode::V_MUL_HI_I32),
    ("V_DIV_SCALE_F32", Opcode::V_DIV_SCALE_F32),
    ("V_DIV_SCALE_F64", Opcode::V_DIV_SCALE_F64),
    ("V_DIV_FMAS_F32", Opcode::V_DIV_FMAS_F32),
    ("V_DIV_FMAS_F64", Opcode::V_DIV_FMAS_F64),
    ("V_MSAD_U8", Opcode::V_MSAD_U8),
    ("V_QSAD_U8", Opcode::V_QSAD_U8),
    ("V_QSAD_PK_U16_U8", Opcode::V_QSAD_PK_U16_U8),
    ("V_MQSAD_U8", Opcode::V_MQSAD_U8),
    ("V_MQSAD_PK_U16_U8", Opcode::V_MQSAD_PK_U16_U8),
    ("V_TRIG_PREOP_F64", Opcode::V_TRIG_PREOP_F64),
    ("V_MQSAD_U32_U8", Opcode::V_MQSAD_U32_U8),
    ("V_MAD_U64_U32", Opcode::V_MAD_U64_U32),
    ("V_MAD_I64_I32", Opcode::V_MAD_I64_I32),
    ("V_ADD_NC_U16", Opcode::V_ADD_NC_U16),
    ("V_LSHRREV_B16", Opcode::V_LSHRREV_B16),
    ("V_ASHRREV_I16", Opcode::V_ASHRREV_I16),
    ("V_ADD_NC_I16", Opcode::V_ADD_NC_I16),
    ("V_SUB_NC_I16", Opcode::V_SUB_NC_I16),
    ("V_SUB_CO_U32", Opcode::V_SUB_CO_U32),
    ("V_LSHLREV_B16", Opcode::V_LSHLREV_B16),
    ("V_MAD_F16", Opcode::V_MAD_F16),
    ("V_LSHL_ADD_U32", Opcode::V_LSHL_ADD_U32),
    ("V_ADD_LSHL_U32", Opcode::V_ADD_LSHL_U32),
    ("V_MIN3_F16", Opcode::V_MIN3_F16),
    ("V_MAX3_F16", Opcode::V_MAX3_F16),
    ("V_MED3_F16", Opcode::V_MED3_F16),
    ("V_MAD_I16", Opcode::V_MAD_I16),
    ("V_ADD3_U32", Opcode::V_ADD3_U32),
    ("V_LSHL_OR_B32", Opcode::V_LSHL_OR_B32),
    ("V_AND_OR_B32", Opcode::V_AND_OR_B32),
    ("V_OR3_B32", Opcode::V_OR3_B32),
    ("V_INTERP_P1_F32", Opcode::V_INTERP_P1_F32),
    ("V_INTERP_P2_F32", Opcode::V_INTERP_P2_F32),
    ("V_INTERP_MOV_F32", Opcode::V_INTERP_MOV_F32),
    ("S_LOAD_DWORD", Opcode::S_LOAD_DWORD),
    ("S_LOAD_DWORDX2", Opcode::S_LOAD_DWORDX2),
    ("S_LOAD_DWORDX4", Opcode::S_LOAD_DWORDX4),
    ("S_LOAD_DWORDX8", Opcode::S_LOAD_DWORDX8),
    ("S_LOAD_DWORDX16", Opcode::S_LOAD_DWORDX16),
    ("S_BUFFER_LOAD_DWORD", Opcode::S_BUFFER_LOAD_DWORD),
    ("S_BUFFER_LOAD_DWORDX2", Opcode::S_BUFFER_LOAD_DWORDX2),
    ("S_BUFFER_LOAD_DWORDX4", Opcode::S_BUFFER_LOAD_DWORDX4),
    ("S_BUFFER_LOAD_DWORDX8", Opcode::S_BUFFER_LOAD_DWORDX8),
    ("S_BUFFER_LOAD_DWORDX16", Opcode::S_BUFFER_LOAD_DWORDX16),
    ("S_MEMTIME", Opcode::S_MEMTIME),
    ("S_DCACHE_INV", Opcode::S_DCACHE_INV),
    ("DS_ADD_U32", Opcode::DS_ADD_U32),
    ("DS_SUB_U32", Opcode::DS_SUB_U32),
    ("DS_RSUB_U32", Opcode::DS_RSUB_U32),
    ("DS_INC_U32", Opcode::DS_INC_U32),
    ("DS_DEC_U32", Opcode::DS_DEC_U32),
    ("DS_MIN_I32", Opcode::DS_MIN_I32),
    ("DS_MAX_I32", Opcode::DS_MAX_I32),
    ("DS_MIN_U32", Opcode::DS_MIN_U32),
    ("DS_MAX_U32", Opcode::DS_MAX_U32),
    ("DS_AND_B32", Opcode::DS_AND_B32),
    ("DS_OR_B32", Opcode::DS_OR_B32),
    ("DS_XOR_B32", Opcode::DS_XOR_B32),
    ("DS_MSKOR_B32", Opcode::DS_MSKOR_B32),
    ("DS_WRITE_B32", Opcode::DS_WRITE_B32),
    ("DS_WRITE2_B32", Opcode::DS_WRITE2_B32),
    ("DS_WRITE2ST64_B32", Opcode::DS_WRITE2ST64_B32),
    ("DS_CMPST_B32", Opcode::DS_CMPST_B32),
    ("DS_CMPST_F32", Opcode::DS_CMPST_F32),
    ("DS_MIN_F32", Opcode::DS_MIN_F32),
    ("DS_MAX_F32", Opcode::DS_MAX_F32),
    ("DS_NOP", Opcode::DS_NOP),
    ("DS_GWS_SEMA_RELEASE_ALL", Opcode::DS_GWS_SEMA_RELEASE_ALL),
    ("DS_GWS_INIT", Opcode::DS_GWS_INIT),
    ("DS_GWS_SEMA_V", Opcode::DS_GWS_SEMA_V),
    ("DS_GWS_SEMA_BR", Opcode::DS_GWS_SEMA_BR),
    ("DS_GWS_SEMA_P", Opcode::DS_GWS_SEMA_P),
    ("DS_GWS_BARRIER", Opcode::DS_GWS_BARRIER),
    ("DS_WRITE_B8", Opcode::DS_WRITE_B8),
    ("DS_WRITE_B16", Opcode::DS_WRITE_B16),
    ("DS_ADD_RTN_U32", Opcode::DS_ADD_RTN_U32),
    ("DS_SUB_RTN_U32", Opcode::DS_SUB_RTN_U32),
    ("DS_RSUB_RTN_U32", Opcode::DS_RSUB_RTN_U32),
    ("DS_INC_RTN_U32", Opcode::DS_INC_RTN_U32),
    ("DS_DEC_RTN_U32", Opcode::DS_DEC_RTN_U32),
    ("DS_MIN_RTN_I32", Opcode::DS_MIN_RTN_I32),
    ("DS_MAX_RTN_I32", Opcode::DS_MAX_RTN_I32),
    ("DS_MIN_RTN_U32", Opcode::DS_MIN_RTN_U32),
    ("DS_MAX_RTN_U32", Opcode::DS_MAX_RTN_U32),
    ("DS_AND_RTN_B32", Opcode::DS_AND_RTN_B32),
    ("DS_OR_RTN_B32", Opcode::DS_OR_RTN_B32),
    ("DS_XOR_RTN_B32", Opcode::DS_XOR_RTN_B32),
    ("DS_MSKOR_RTN_B32", Opcode::DS_MSKOR_RTN_B32),
    ("DS_WRXCHG_RTN_B32", Opcode::DS_WRXCHG_RTN_B32),
    ("DS_WRXCHG2_RTN_B32", Opcode::DS_WRXCHG2_RTN_B32),
    ("DS_WRXCHG2ST64_RTN_B32", Opcode::DS_WRXCHG2ST64_RTN_B32),
    ("DS_CMPST_RTN_B32", Opcode::DS_CMPST_RTN_B32),
    ("DS_CMPST_RTN_F32", Opcode::DS_CMPST_RTN_F32),
    ("DS_MIN_RTN_F32", Opcode::DS_MIN_RTN_F32),
    ("DS_MAX_RTN_F32", Opcode::DS_MAX_RTN_F32),
    ("DS_WRAP_RTN_B32", Opcode::DS_WRAP_RTN_B32),
    ("DS_SWIZZLE_B32", Opcode::DS_SWIZZLE_B32),
    ("DS_READ_B32", Opcode::DS_READ_B32),
    ("DS_READ2_B32", Opcode::DS_READ2_B32),
    ("DS_READ2ST64_B32", Opcode::DS_READ2ST64_B32),
    ("DS_READ_I8", Opcode::DS_READ_I8),
    ("DS_READ_U8", Opcode::DS_READ_U8),
    ("DS_READ_I16", Opcode::DS_READ_I16),
    ("DS_READ_U16", Opcode::DS_READ_U16),
    ("DS_CONSUME", Opcode::DS_CONSUME),
    ("DS_APPEND", Opcode::DS_APPEND),
    ("DS_ORDERED_COUNT", Opcode::DS_ORDERED_COUNT),
    ("DS_ADD_U64", Opcode::DS_ADD_U64),
    ("DS_SUB_U64", Opcode::DS_SUB_U64),
    ("DS_RSUB_U64", Opcode::DS_RSUB_U64),
    ("DS_INC_U64", Opcode::DS_INC_U64),
    ("DS_DEC_U64", Opcode::DS_DEC_U64),
    ("DS_MIN_I64", Opcode::DS_MIN_I64),
    ("DS_MAX_I64", Opcode::DS_MAX_I64),
    ("DS_MIN_U64", Opcode::DS_MIN_U64),
    ("DS_MAX_U64", Opcode::DS_MAX_U64),
    ("DS_AND_B64", Opcode::DS_AND_B64),
    ("DS_OR_B64", Opcode::DS_OR_B64),
    ("DS_XOR_B64", Opcode::DS_XOR_B64),
    ("DS_MSKOR_B64", Opcode::DS_MSKOR_B64),
    ("DS_WRITE_B64", Opcode::DS_WRITE_B64),
    ("DS_WRITE2_B64", Opcode::DS_WRITE2_B64),
    ("DS_WRITE2ST64_B64", Opcode::DS_WRITE2ST64_B64),
    ("DS_CMPST_B64", Opcode::DS_CMPST_B64),
    ("DS_CMPST_F64", Opcode::DS_CMPST_F64),
    ("DS_MIN_F64", Opcode::DS_MIN_F64),
    ("DS_MAX_F64", Opcode::DS_MAX_F64),
    ("DS_ADD_RTN_U64", Opcode::DS_ADD_RTN_U64),
    ("DS_SUB_RTN_U64", Opcode::DS_SUB_RTN_U64),
    ("DS_RSUB_RTN_U64", Opcode::DS_RSUB_RTN_U64),
    ("DS_INC_RTN_U64", Opcode::DS_INC_RTN_U64),
    ("DS_DEC_RTN_U64", Opcode::DS_DEC_RTN_U64),
    ("DS_MIN_RTN_I64", Opcode::DS_MIN_RTN_I64),
    ("DS_MAX_RTN_I64", Opcode::DS_MAX_RTN_I64),
    ("DS_MIN_RTN_U64", Opcode::DS_MIN_RTN_U64),
    ("DS_MAX_RTN_U64", Opcode::DS_MAX_RTN_U64),
    ("DS_AND_RTN_B64", Opcode::DS_AND_RTN_B64),
    ("DS_OR_RTN_B64", Opcode::DS_OR_RTN_B64),
    ("DS_XOR_RTN_B64", Opcode::DS_XOR_RTN_B64),
    ("DS_MSKOR_RTN_B64", Opcode::DS_MSKOR_RTN_B64),
    ("DS_WRXCHG_RTN_B64", Opcode::DS_WRXCHG_RTN_B64),
    ("DS_WRXCHG2_RTN_B64", Opcode::DS_WRXCHG2_RTN_B64),
    ("DS_WRXCHG2ST64_RTN_B64", Opcode::DS_WRXCHG2ST64_RTN_B64),
    ("DS_CMPST_RTN_B64", Opcode::DS_CMPST_RTN_B64),
    ("DS_CMPST_RTN_F64", Opcode::DS_CMPST_RTN_F64),
    ("DS_MIN_RTN_F64", Opcode::DS_MIN_RTN_F64),
    ("DS_MAX_RTN_F64", Opcode::DS_MAX_RTN_F64),
    ("DS_READ_B64", Opcode::DS_READ_B64),
    ("DS_READ2_B64", Opcode::DS_READ2_B64),
    ("DS_READ2ST64_B64", Opcode::DS_READ2ST64_B64),
    ("DS_CONDXCHG32_RTN_B64", Opcode::DS_CONDXCHG32_RTN_B64),
    ("DS_ADD_SRC2_U32", Opcode::DS_ADD_SRC2_U32),
    ("DS_SUB_SRC2_U32", Opcode::DS_SUB_SRC2_U32),
    ("DS_RSUB_SRC2_U32", Opcode::DS_RSUB_SRC2_U32),
    ("DS_INC_SRC2_U32", Opcode::DS_INC_SRC2_U32),
    ("DS_DEC_SRC2_U32", Opcode::DS_DEC_SRC2_U32),
    ("DS_MIN_SRC2_I32", Opcode::DS_MIN_SRC2_I32),
    ("DS_MAX_SRC2_I32", Opcode::DS_MAX_SRC2_I32),
    ("DS_MIN_SRC2_U32", Opcode::DS_MIN_SRC2_U32),
    ("DS_MAX_SRC2_U32", Opcode::DS_MAX_SRC2_U32),
    ("DS_AND_SRC2_B32", Opcode::DS_AND_SRC2_B32),
    ("DS_OR_SRC2_B32", Opcode::DS_OR_SRC2_B32),
    ("DS_XOR_SRC2_B32", Opcode::DS_XOR_SRC2_B32),
    ("DS_WRITE_SRC2_B32", Opcode::DS_WRITE_SRC2_B32),
    ("DS_MIN_SRC2_F32", Opcode::DS_MIN_SRC2_F32),
    ("DS_MAX_SRC2_F32", Opcode::DS_MAX_SRC2_F32),
    ("DS_ADD_SRC2_U64", Opcode::DS_ADD_SRC2_U64),
    ("DS_SUB_SRC2_U64", Opcode::DS_SUB_SRC2_U64),
    ("DS_RSUB_SRC2_U64", Opcode::DS_RSUB_SRC2_U64),
    ("DS_INC_SRC2_U64", Opcode::DS_INC_SRC2_U64),
    ("DS_DEC_SRC2_U64", Opcode::DS_DEC_SRC2_U64),
    ("DS_MIN_SRC2_I64", Opcode::DS_MIN_SRC2_I64),
    ("DS_MAX_SRC2_I64", Opcode::DS_MAX_SRC2_I64),
    ("DS_MIN_SRC2_U64", Opcode::DS_MIN_SRC2_U64),
    ("DS_MAX_SRC2_U64", Opcode::DS_MAX_SRC2_U64),
    ("DS_AND_SRC2_B64", Opcode::DS_AND_SRC2_B64),
    ("DS_OR_SRC2_B64", Opcode::DS_OR_SRC2_B64),
    ("DS_XOR_SRC2_B64", Opcode::DS_XOR_SRC2_B64),
    ("DS_WRITE_SRC2_B64", Opcode::DS_WRITE_SRC2_B64),
    ("DS_MIN_SRC2_F64", Opcode::DS_MIN_SRC2_F64),
    ("DS_MAX_SRC2_F64", Opcode::DS_MAX_SRC2_F64),
    ("DS_WRITE_B96", Opcode::DS_WRITE_B96),
    ("DS_WRITE_B128", Opcode::DS_WRITE_B128),
    ("DS_CONDXCHG32_RTN_B128", Opcode::DS_CONDXCHG32_RTN_B128),
    ("DS_READ_B96", Opcode::DS_READ_B96),
    ("DS_READ_B128", Opcode::DS_READ_B128),
    ("BUFFER_LOAD_FORMAT_X", Opcode::BUFFER_LOAD_FORMAT_X),
    ("BUFFER_LOAD_FORMAT_XY", Opcode::BUFFER_LOAD_FORMAT_XY),
    ("BUFFER_LOAD_FORMAT_XYZ", Opcode::BUFFER_LOAD_FORMAT_XYZ),
    ("BUFFER_LOAD_FORMAT_XYZW", Opcode::BUFFER_LOAD_FORMAT_XYZW),
    ("BUFFER_STORE_FORMAT_X", Opcode::BUFFER_STORE_FORMAT_X),
    ("BUFFER_STORE_FORMAT_XY", Opcode::BUFFER_STORE_FORMAT_XY),
    ("BUFFER_STORE_FORMAT_XYZ", Opcode::BUFFER_STORE_FORMAT_XYZ),
    ("BUFFER_STORE_FORMAT_XYZW", Opcode::BUFFER_STORE_FORMAT_XYZW),
    ("BUFFER_LOAD_UBYTE", Opcode::BUFFER_LOAD_UBYTE),
    ("BUFFER_LOAD_SBYTE", Opcode::BUFFER_LOAD_SBYTE),
    ("BUFFER_LOAD_USHORT", Opcode::BUFFER_LOAD_USHORT),
    ("BUFFER_LOAD_SSHORT", Opcode::BUFFER_LOAD_SSHORT),
    ("BUFFER_LOAD_DWORD", Opcode::BUFFER_LOAD_DWORD),
    ("BUFFER_LOAD_DWORDX2", Opcode::BUFFER_LOAD_DWORDX2),
    ("BUFFER_LOAD_DWORDX4", Opcode::BUFFER_LOAD_DWORDX4),
    ("BUFFER_LOAD_DWORDX3", Opcode::BUFFER_LOAD_DWORDX3),
    ("BUFFER_STORE_BYTE", Opcode::BUFFER_STORE_BYTE),
    ("BUFFER_STORE_SHORT", Opcode::BUFFER_STORE_SHORT),
    ("BUFFER_STORE_DWORD", Opcode::BUFFER_STORE_DWORD),
    ("BUFFER_STORE_DWORDX2", Opcode::BUFFER_STORE_DWORDX2),
    ("BUFFER_STORE_DWORDX4", Opcode::BUFFER_STORE_DWORDX4),
    ("BUFFER_STORE_DWORDX3", Opcode::BUFFER_STORE_DWORDX3),
    ("BUFFER_ATOMIC_SWAP", Opcode::BUFFER_ATOMIC_SWAP),
    ("BUFFER_ATOMIC_CMPSWAP", Opcode::BUFFER_ATOMIC_CMPSWAP),
    ("BUFFER_ATOMIC_ADD", Opcode::BUFFER_ATOMIC_ADD),
    ("BUFFER_ATOMIC_SUB", Opcode::BUFFER_ATOMIC_SUB),
    ("BUFFER_ATOMIC_SMIN", Opcode::BUFFER_ATOMIC_SMIN),
    ("BUFFER_ATOMIC_UMIN", Opcode::BUFFER_ATOMIC_UMIN),
    ("BUFFER_ATOMIC_SMAX", Opcode::BUFFER_ATOMIC_SMAX),
    ("BUFFER_ATOMIC_UMAX", Opcode::BUFFER_ATOMIC_UMAX),
    ("BUFFER_ATOMIC_AND", Opcode::BUFFER_ATOMIC_AND),
    ("BUFFER_ATOMIC_OR", Opcode::BUFFER_ATOMIC_OR),
    ("BUFFER_ATOMIC_XOR", Opcode::BUFFER_ATOMIC_XOR),
    ("BUFFER_ATOMIC_INC", Opcode::BUFFER_ATOMIC_INC),
    ("BUFFER_ATOMIC_DEC", Opcode::BUFFER_ATOMIC_DEC),
    ("BUFFER_ATOMIC_FCMPSWAP", Opcode::BUFFER_ATOMIC_FCMPSWAP),
    ("BUFFER_ATOMIC_FMIN", Opcode::BUFFER_ATOMIC_FMIN),
    ("BUFFER_ATOMIC_FMAX", Opcode::BUFFER_ATOMIC_FMAX),
    ("BUFFER_ATOMIC_SWAP_X2", Opcode::BUFFER_ATOMIC_SWAP_X2),
    ("BUFFER_ATOMIC_CMPSWAP_X2", Opcode::BUFFER_ATOMIC_CMPSWAP_X2),
    ("BUFFER_ATOMIC_ADD_X2", Opcode::BUFFER_ATOMIC_ADD_X2),
    ("BUFFER_ATOMIC_SUB_X2", Opcode::BUFFER_ATOMIC_SUB_X2),
    ("BUFFER_ATOMIC_SMIN_X2", Opcode::BUFFER_ATOMIC_SMIN_X2),
    ("BUFFER_ATOMIC_UMIN_X2", Opcode::BUFFER_ATOMIC_UMIN_X2),
    ("BUFFER_ATOMIC_SMAX_X2", Opcode::BUFFER_ATOMIC_SMAX_X2),
    ("BUFFER_ATOMIC_UMAX_X2", Opcode::BUFFER_ATOMIC_UMAX_X2),
    ("BUFFER_ATOMIC_AND_X2", Opcode::BUFFER_ATOMIC_AND_X2),
    ("BUFFER_ATOMIC_OR_X2", Opcode::BUFFER_ATOMIC_OR_X2),
    ("BUFFER_ATOMIC_XOR_X2", Opcode::BUFFER_ATOMIC_XOR_X2),
    ("BUFFER_ATOMIC_INC_X2", Opcode::BUFFER_ATOMIC_INC_X2),
    ("BUFFER_ATOMIC_DEC_X2", Opcode::BUFFER_ATOMIC_DEC_X2),
    (
        "BUFFER_ATOMIC_FCMPSWAP_X2",
        Opcode::BUFFER_ATOMIC_FCMPSWAP_X2,
    ),
    ("BUFFER_ATOMIC_FMIN_X2", Opcode::BUFFER_ATOMIC_FMIN_X2),
    ("BUFFER_ATOMIC_FMAX_X2", Opcode::BUFFER_ATOMIC_FMAX_X2),
    ("BUFFER_WBINVL1_SC", Opcode::BUFFER_WBINVL1_SC),
    ("BUFFER_WBINVL1", Opcode::BUFFER_WBINVL1),
    ("TBUFFER_LOAD_FORMAT_X", Opcode::TBUFFER_LOAD_FORMAT_X),
    ("TBUFFER_LOAD_FORMAT_XY", Opcode::TBUFFER_LOAD_FORMAT_XY),
    ("TBUFFER_LOAD_FORMAT_XYZ", Opcode::TBUFFER_LOAD_FORMAT_XYZ),
    ("TBUFFER_LOAD_FORMAT_XYZW", Opcode::TBUFFER_LOAD_FORMAT_XYZW),
    ("TBUFFER_STORE_FORMAT_X", Opcode::TBUFFER_STORE_FORMAT_X),
    ("TBUFFER_STORE_FORMAT_XY", Opcode::TBUFFER_STORE_FORMAT_XY),
    ("TBUFFER_STORE_FORMAT_XYZ", Opcode::TBUFFER_STORE_FORMAT_XYZ),
    (
        "TBUFFER_STORE_FORMAT_XYZW",
        Opcode::TBUFFER_STORE_FORMAT_XYZW,
    ),
    ("IMAGE_LOAD", Opcode::IMAGE_LOAD),
    ("IMAGE_LOAD_MIP", Opcode::IMAGE_LOAD_MIP),
    ("IMAGE_LOAD_PCK", Opcode::IMAGE_LOAD_PCK),
    ("IMAGE_LOAD_PCK_SGN", Opcode::IMAGE_LOAD_PCK_SGN),
    ("IMAGE_LOAD_MIP_PCK", Opcode::IMAGE_LOAD_MIP_PCK),
    ("IMAGE_LOAD_MIP_PCK_SGN", Opcode::IMAGE_LOAD_MIP_PCK_SGN),
    ("IMAGE_STORE", Opcode::IMAGE_STORE),
    ("IMAGE_STORE_MIP", Opcode::IMAGE_STORE_MIP),
    ("IMAGE_STORE_PCK", Opcode::IMAGE_STORE_PCK),
    ("IMAGE_STORE_MIP_PCK", Opcode::IMAGE_STORE_MIP_PCK),
    ("IMAGE_GET_RESINFO", Opcode::IMAGE_GET_RESINFO),
    ("IMAGE_ATOMIC_SWAP", Opcode::IMAGE_ATOMIC_SWAP),
    ("IMAGE_ATOMIC_CMPSWAP", Opcode::IMAGE_ATOMIC_CMPSWAP),
    ("IMAGE_ATOMIC_ADD", Opcode::IMAGE_ATOMIC_ADD),
    ("IMAGE_ATOMIC_SUB", Opcode::IMAGE_ATOMIC_SUB),
    ("IMAGE_ATOMIC_SMIN", Opcode::IMAGE_ATOMIC_SMIN),
    ("IMAGE_ATOMIC_UMIN", Opcode::IMAGE_ATOMIC_UMIN),
    ("IMAGE_ATOMIC_SMAX", Opcode::IMAGE_ATOMIC_SMAX),
    ("IMAGE_ATOMIC_UMAX", Opcode::IMAGE_ATOMIC_UMAX),
    ("IMAGE_ATOMIC_AND", Opcode::IMAGE_ATOMIC_AND),
    ("IMAGE_ATOMIC_OR", Opcode::IMAGE_ATOMIC_OR),
    ("IMAGE_ATOMIC_XOR", Opcode::IMAGE_ATOMIC_XOR),
    ("IMAGE_ATOMIC_INC", Opcode::IMAGE_ATOMIC_INC),
    ("IMAGE_ATOMIC_DEC", Opcode::IMAGE_ATOMIC_DEC),
    ("IMAGE_ATOMIC_FCMPSWAP", Opcode::IMAGE_ATOMIC_FCMPSWAP),
    ("IMAGE_ATOMIC_FMIN", Opcode::IMAGE_ATOMIC_FMIN),
    ("IMAGE_ATOMIC_FMAX", Opcode::IMAGE_ATOMIC_FMAX),
    ("IMAGE_SAMPLE", Opcode::IMAGE_SAMPLE),
    ("IMAGE_SAMPLE_CL", Opcode::IMAGE_SAMPLE_CL),
    ("IMAGE_SAMPLE_D", Opcode::IMAGE_SAMPLE_D),
    ("IMAGE_SAMPLE_D_CL", Opcode::IMAGE_SAMPLE_D_CL),
    ("IMAGE_SAMPLE_L", Opcode::IMAGE_SAMPLE_L),
    ("IMAGE_SAMPLE_B", Opcode::IMAGE_SAMPLE_B),
    ("IMAGE_SAMPLE_B_CL", Opcode::IMAGE_SAMPLE_B_CL),
    ("IMAGE_SAMPLE_LZ", Opcode::IMAGE_SAMPLE_LZ),
    ("IMAGE_SAMPLE_C", Opcode::IMAGE_SAMPLE_C),
    ("IMAGE_SAMPLE_C_CL", Opcode::IMAGE_SAMPLE_C_CL),
    ("IMAGE_SAMPLE_C_D", Opcode::IMAGE_SAMPLE_C_D),
    ("IMAGE_SAMPLE_C_D_CL", Opcode::IMAGE_SAMPLE_C_D_CL),
    ("IMAGE_SAMPLE_C_L", Opcode::IMAGE_SAMPLE_C_L),
    ("IMAGE_SAMPLE_C_B", Opcode::IMAGE_SAMPLE_C_B),
    ("IMAGE_SAMPLE_C_B_CL", Opcode::IMAGE_SAMPLE_C_B_CL),
    ("IMAGE_SAMPLE_C_LZ", Opcode::IMAGE_SAMPLE_C_LZ),
    ("IMAGE_SAMPLE_O", Opcode::IMAGE_SAMPLE_O),
    ("IMAGE_SAMPLE_CL_O", Opcode::IMAGE_SAMPLE_CL_O),
    ("IMAGE_SAMPLE_D_O", Opcode::IMAGE_SAMPLE_D_O),
    ("IMAGE_SAMPLE_D_CL_O", Opcode::IMAGE_SAMPLE_D_CL_O),
    ("IMAGE_SAMPLE_L_O", Opcode::IMAGE_SAMPLE_L_O),
    ("IMAGE_SAMPLE_B_O", Opcode::IMAGE_SAMPLE_B_O),
    ("IMAGE_SAMPLE_B_CL_O", Opcode::IMAGE_SAMPLE_B_CL_O),
    ("IMAGE_SAMPLE_LZ_O", Opcode::IMAGE_SAMPLE_LZ_O),
    ("IMAGE_SAMPLE_C_O", Opcode::IMAGE_SAMPLE_C_O),
    ("IMAGE_SAMPLE_C_CL_O", Opcode::IMAGE_SAMPLE_C_CL_O),
    ("IMAGE_SAMPLE_C_D_O", Opcode::IMAGE_SAMPLE_C_D_O),
    ("IMAGE_SAMPLE_C_D_CL_O", Opcode::IMAGE_SAMPLE_C_D_CL_O),
    ("IMAGE_SAMPLE_C_L_O", Opcode::IMAGE_SAMPLE_C_L_O),
    ("IMAGE_SAMPLE_C_B_O", Opcode::IMAGE_SAMPLE_C_B_O),
    ("IMAGE_SAMPLE_C_B_CL_O", Opcode::IMAGE_SAMPLE_C_B_CL_O),
    ("IMAGE_SAMPLE_C_LZ_O", Opcode::IMAGE_SAMPLE_C_LZ_O),
    ("IMAGE_GATHER4", Opcode::IMAGE_GATHER4),
    ("IMAGE_GATHER4_CL", Opcode::IMAGE_GATHER4_CL),
    ("IMAGE_GATHER4_L", Opcode::IMAGE_GATHER4_L),
    ("IMAGE_GATHER4_B", Opcode::IMAGE_GATHER4_B),
    ("IMAGE_GATHER4_B_CL", Opcode::IMAGE_GATHER4_B_CL),
    ("IMAGE_GATHER4_LZ", Opcode::IMAGE_GATHER4_LZ),
    ("IMAGE_GATHER4_C", Opcode::IMAGE_GATHER4_C),
    ("IMAGE_GATHER4_C_CL", Opcode::IMAGE_GATHER4_C_CL),
    ("IMAGE_GATHER4_C_L", Opcode::IMAGE_GATHER4_C_L),
    ("IMAGE_GATHER4_C_B", Opcode::IMAGE_GATHER4_C_B),
    ("IMAGE_GATHER4_C_B_CL", Opcode::IMAGE_GATHER4_C_B_CL),
    ("IMAGE_GATHER4_C_LZ", Opcode::IMAGE_GATHER4_C_LZ),
    ("IMAGE_GATHER4_O", Opcode::IMAGE_GATHER4_O),
    ("IMAGE_GATHER4_CL_O", Opcode::IMAGE_GATHER4_CL_O),
    ("IMAGE_GATHER4_L_O", Opcode::IMAGE_GATHER4_L_O),
    ("IMAGE_GATHER4_B_O", Opcode::IMAGE_GATHER4_B_O),
    ("IMAGE_GATHER4_B_CL_O", Opcode::IMAGE_GATHER4_B_CL_O),
    ("IMAGE_GATHER4_LZ_O", Opcode::IMAGE_GATHER4_LZ_O),
    ("IMAGE_GATHER4_C_O", Opcode::IMAGE_GATHER4_C_O),
    ("IMAGE_GATHER4_C_CL_O", Opcode::IMAGE_GATHER4_C_CL_O),
    ("IMAGE_GATHER4_C_L_O", Opcode::IMAGE_GATHER4_C_L_O),
    ("IMAGE_GATHER4_C_B_O", Opcode::IMAGE_GATHER4_C_B_O),
    ("IMAGE_GATHER4_C_B_CL_O", Opcode::IMAGE_GATHER4_C_B_CL_O),
    ("IMAGE_GATHER4_C_LZ_O", Opcode::IMAGE_GATHER4_C_LZ_O),
    ("IMAGE_GET_LOD", Opcode::IMAGE_GET_LOD),
    ("IMAGE_SAMPLE_CD", Opcode::IMAGE_SAMPLE_CD),
    ("IMAGE_SAMPLE_CD_CL", Opcode::IMAGE_SAMPLE_CD_CL),
    ("IMAGE_SAMPLE_C_CD", Opcode::IMAGE_SAMPLE_C_CD),
    ("IMAGE_SAMPLE_C_CD_CL", Opcode::IMAGE_SAMPLE_C_CD_CL),
    ("IMAGE_SAMPLE_CD_O", Opcode::IMAGE_SAMPLE_CD_O),
    ("IMAGE_SAMPLE_CD_CL_O", Opcode::IMAGE_SAMPLE_CD_CL_O),
    ("IMAGE_SAMPLE_C_CD_O", Opcode::IMAGE_SAMPLE_C_CD_O),
    ("IMAGE_SAMPLE_C_CD_CL_O", Opcode::IMAGE_SAMPLE_C_CD_CL_O),
    ("EXP", Opcode::EXP),
    ("V_PK_MAD_I16", Opcode::V_PK_MAD_I16),
    ("V_PK_MUL_LO_U16", Opcode::V_PK_MUL_LO_U16),
    ("V_PK_ADD_I16", Opcode::V_PK_ADD_I16),
    ("V_PK_SUB_I16", Opcode::V_PK_SUB_I16),
    ("V_PK_LSHLREV_B16", Opcode::V_PK_LSHLREV_B16),
    ("V_PK_LSHRREV_B16", Opcode::V_PK_LSHRREV_B16),
    ("V_PK_ASHRREV_I16", Opcode::V_PK_ASHRREV_I16),
    ("V_PK_MAX_I16", Opcode::V_PK_MAX_I16),
    ("V_PK_MIN_I16", Opcode::V_PK_MIN_I16),
    ("V_PK_MAD_U16", Opcode::V_PK_MAD_U16),
    ("V_PK_ADD_U16", Opcode::V_PK_ADD_U16),
    ("V_PK_SUB_U16", Opcode::V_PK_SUB_U16),
    ("V_PK_MAX_U16", Opcode::V_PK_MAX_U16),
    ("V_PK_MIN_U16", Opcode::V_PK_MIN_U16),
    ("V_PK_FMA_F16", Opcode::V_PK_FMA_F16),
    ("V_PK_ADD_F16", Opcode::V_PK_ADD_F16),
    ("V_PK_MUL_F16", Opcode::V_PK_MUL_F16),
    ("V_PK_MIN_F16", Opcode::V_PK_MIN_F16),
    ("V_PK_MAX_F16", Opcode::V_PK_MAX_F16),
    ("V_MAD_MIX_F32", Opcode::V_MAD_MIX_F32),
    ("V_MAD_MIXLO_F16", Opcode::V_MAD_MIXLO_F16),
    ("V_MAD_MIXHI_F16", Opcode::V_MAD_MIXHI_F16),
];

/// The first name declared with this `Opcode` value, as `magic_enum` reports it.
pub fn opcode_name(value: Opcode) -> &'static str {
    for (name, named) in OPCODE_NAMES {
        if *named == value {
            return name;
        }
    }
    ""
}

/// `magic_enum::enum_name` for a `InstCategory`, in declaration order.
pub const INST_CATEGORY_NAMES: &[(&str, InstCategory)] = &[
    ("Undefined", InstCategory::Undefined),
    ("ScalarALU", InstCategory::ScalarALU),
    ("ScalarMemory", InstCategory::ScalarMemory),
    ("VectorALU", InstCategory::VectorALU),
    ("VectorMemory", InstCategory::VectorMemory),
    ("FlowControl", InstCategory::FlowControl),
    ("DataShare", InstCategory::DataShare),
    ("VectorInterpolation", InstCategory::VectorInterpolation),
    ("Export", InstCategory::Export),
    ("DebugProfile", InstCategory::DebugProfile),
];

/// The first name declared with this `InstCategory` value, as `magic_enum` reports it.
pub fn inst_category_name(value: InstCategory) -> &'static str {
    for (name, named) in INST_CATEGORY_NAMES {
        if *named == value {
            return name;
        }
    }
    ""
}
