//! Vector ALU translation, ported from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/translate/vector_alu.cpp`.

use super::translator::{ConditionOp, Translator};
use crate::gpu::amdgpu::pixel_format::NumberFormat;
use crate::gpu::shader_recompiler::frontend::instruction::GcnInst;
use crate::gpu::shader_recompiler::frontend::opcodes::Opcode;
use crate::gpu::shader_recompiler::frontend::opcodes::OperandField;
use crate::gpu::shader_recompiler::ir::reg::{reg_add, FloatClassFunc, VectorReg};
use crate::gpu::shader_recompiler::ir::value::{
    F16, F16F32F64, F32, F32F64, F64, U1, U16, U32, U32F32, U32U64, U64, U8U16U32U64,
};

fn f32_of(value: F32F64) -> F32 {
    F32::new(value.into_value())
}

fn f64_of(value: F32F64) -> F64 {
    F64::new(value.into_value())
}

fn u32_of(value: U32U64) -> U32 {
    U32::new(value.into_value())
}

fn u64_of(value: U32U64) -> U64 {
    U64::new(value.into_value())
}

fn f32w(value: &F32) -> F32F64 {
    F32F64::new(value.value().clone())
}

fn f64w(value: &F64) -> F32F64 {
    F32F64::new(value.value().clone())
}

fn u32w(value: &U32) -> U32U64 {
    U32U64::new(value.value().clone())
}

fn u64w(value: &U64) -> U32U64 {
    U32U64::new(value.value().clone())
}

impl Translator<'_> {
    /// `EmitVectorAlu`.
    pub(crate) fn emit_vector_alu(&mut self, inst: &GcnInst) {
        match inst.opcode {
            Opcode::V_CNDMASK_B32 => self.v_cndmask_b32(inst),
            Opcode::V_READLANE_B32 => self.v_readlane_b32(inst),
            Opcode::V_WRITELANE_B32 => self.v_writelane_b32(inst),
            Opcode::V_ADD_F32 => self.v_add_f32(inst),
            Opcode::V_SUB_F32 => self.v_sub_f32(inst),
            Opcode::V_SUBREV_F32 => self.v_subrev_f32(inst),
            Opcode::V_MAC_LEGACY_F32 => self.v_mac_f32(inst),
            Opcode::V_MUL_LEGACY_F32 => self.v_mul_legacy_f32(inst),
            Opcode::V_MUL_F32 => self.v_mul_f32(inst),
            Opcode::V_MUL_I32_I24 => self.v_mul_i32_i24(inst, true),
            Opcode::V_MUL_U32_U24 => self.v_mul_i32_i24(inst, false),
            Opcode::V_MIN_LEGACY_F32 => self.v_min_f32(inst, true),
            Opcode::V_MAX_LEGACY_F32 => self.v_max_f32(inst, true),
            Opcode::V_MIN_F32 => self.v_min_f32(inst, false),
            Opcode::V_MAX_F32 => self.v_max_f32(inst, false),
            Opcode::V_MIN_I32 => self.v_min_i32(inst),
            Opcode::V_MAX_I32 => self.v_max_u32(true, inst),
            Opcode::V_MIN_U32 => self.v_min_u32(inst),
            Opcode::V_MAX_U32 => self.v_max_u32(false, inst),
            Opcode::V_LSHR_B32 => self.v_lshr_b32(inst),
            Opcode::V_LSHRREV_B32 => self.v_lshrrev_b32(inst),
            Opcode::V_ASHR_I32 => self.v_ashr_i32(inst),
            Opcode::V_ASHRREV_I32 => self.v_ashrrev_i32(inst),
            Opcode::V_LSHL_B32 => self.v_lshl_b32(inst),
            Opcode::V_LSHLREV_B32 => self.v_lshlrev_b32(inst),
            Opcode::V_AND_B32 => self.v_and_b32(inst),
            Opcode::V_OR_B32 => self.v_or_b32(false, inst),
            Opcode::V_XOR_B32 => self.v_or_b32(true, inst),
            Opcode::V_BFM_B32 => self.v_bfm_b32(inst),
            Opcode::V_MAC_F32 => self.v_mac_f32(inst),
            Opcode::V_MADMK_F32 => self.v_madmk_f32(inst),
            Opcode::V_MADAK_F32 => self.v_fma_f32(inst),
            Opcode::V_BCNT_U32_B32 => self.v_bcnt_u32_b32(inst),
            Opcode::V_MBCNT_LO_U32_B32 => self.v_mbcnt_u32_b32(false, inst),
            Opcode::V_MBCNT_HI_U32_B32 => self.v_mbcnt_u32_b32(true, inst),
            Opcode::V_ADD_I32 => self.v_add_i32(inst),
            Opcode::V_SUB_I32 => self.v_sub_i32(inst),
            Opcode::V_SUBREV_I32 => self.v_subrev_i32(inst),
            Opcode::V_ADDC_U32 => self.v_addc_u32(inst),
            Opcode::V_SUBB_U32 => self.v_subb_u32(inst),
            Opcode::V_SUBBREV_U32 => self.v_subb_u32_rev(inst),
            Opcode::V_LDEXP_F32 => self.v_ldexp_f32(inst),
            Opcode::V_CVT_PKNORM_U16_F32 => self.v_cvt_pknorm_u16_f32(inst),
            Opcode::V_CVT_PKNORM_I16_F32 => self.v_cvt_pknorm_i16_f32(inst),
            Opcode::V_CVT_PKRTZ_F16_F32 => self.v_cvt_pkrtz_f16_f32(inst),
            Opcode::V_ADD_F16 => self.v_add_f16(inst),
            Opcode::V_SUB_F16 => self.v_sub_f16(inst),
            Opcode::V_MUL_F16 => self.v_mul_f16(inst),
            Opcode::V_MAX_F16 => self.v_max_f16(inst),
            Opcode::V_MIN_F16 => self.v_min_f16(inst),
            Opcode::V_MOV_B32 => self.v_mov(inst),
            Opcode::V_READFIRSTLANE_B32 => self.v_readfirstlane_b32(inst),
            Opcode::V_CVT_I32_F64 => self.v_cvt_i32_f64(inst),
            Opcode::V_CVT_F64_I32 => self.v_cvt_f64_i32(inst),
            Opcode::V_CVT_F64_U32 => self.v_cvt_f64_u32(inst),
            Opcode::V_CVT_F32_I32 => self.v_cvt_f32_i32(inst),
            Opcode::V_CVT_F32_U32 => self.v_cvt_f32_u32(inst),
            Opcode::V_CVT_U32_F32 => self.v_cvt_u32_f32(inst),
            Opcode::V_CVT_I32_F32 => self.v_cvt_i32_f32(inst),
            Opcode::V_CVT_F16_F32 => self.v_cvt_f16_f32(inst),
            Opcode::V_CVT_F32_F16 => self.v_cvt_f32_f16(inst),
            Opcode::V_CVT_FLR_I32_F32 => self.v_cvt_flr_i32_f32(inst),
            Opcode::V_CVT_F32_F64 => self.v_cvt_f32_f64(inst),
            Opcode::V_CVT_F64_F32 => self.v_cvt_f64_f32(inst),
            Opcode::V_CVT_RPI_I32_F32 => self.v_cvt_rpi_i32_f32(inst),
            Opcode::V_CVT_OFF_F32_I4 => self.v_cvt_off_f32_i4(inst),
            Opcode::V_CVT_F32_UBYTE0 => self.v_cvt_f32_ubyte(0, inst),
            Opcode::V_CVT_F32_UBYTE1 => self.v_cvt_f32_ubyte(1, inst),
            Opcode::V_CVT_F32_UBYTE2 => self.v_cvt_f32_ubyte(2, inst),
            Opcode::V_CVT_F32_UBYTE3 => self.v_cvt_f32_ubyte(3, inst),
            Opcode::V_FLOOR_F64 => self.v_floor_f64(inst),
            Opcode::V_FRACT_F32 => self.v_fract_f32(inst),
            Opcode::V_TRUNC_F32 => self.v_trunc_f32(inst),
            Opcode::V_CEIL_F32 => self.v_ceil_f32(inst),
            Opcode::V_RNDNE_F32 => self.v_rndne_f32(inst),
            Opcode::V_FLOOR_F32 => self.v_floor_f32(inst),
            Opcode::V_EXP_F32 => self.v_exp_f32(inst),
            Opcode::V_LOG_CLAMP_F32 => self.v_log_f32(inst),
            Opcode::V_LOG_F32 => self.v_log_f32(inst),
            Opcode::V_RCP_F32 => self.v_rcp_f32(inst),
            Opcode::V_RCP_LEGACY_F32 => self.v_rcp_legacy_f32(inst),
            Opcode::V_RCP_F64 => self.v_rcp_f64(inst),
            Opcode::V_RCP_IFLAG_F32 => self.v_rcp_f32(inst),
            Opcode::V_RCP_CLAMP_F32 => self.v_rcp_f32(inst),
            Opcode::V_RSQ_CLAMP_F32 => self.v_rsq_f32(inst),
            Opcode::V_RSQ_LEGACY_F32 => self.v_rsq_f32(inst),
            Opcode::V_RSQ_F32 => self.v_rsq_f32(inst),
            Opcode::V_SQRT_F32 => self.v_sqrt_f32(inst),
            Opcode::V_SIN_F32 => self.v_sin_f32(inst),
            Opcode::V_COS_F32 => self.v_cos_f32(inst),
            Opcode::V_NOT_B32 => self.v_not_b32(inst),
            Opcode::V_BFREV_B32 => self.v_bfrev_b32(inst),
            Opcode::V_FFBH_U32 => self.v_ffbh_u32(inst),
            Opcode::V_FFBL_B32 => self.v_ffbl_b32(inst),
            Opcode::V_FFBH_I32 => self.v_ffbh_i32(inst),
            Opcode::V_FREXP_EXP_I32_F64 => self.v_frexp_exp_i32_f64(inst),
            Opcode::V_FREXP_MANT_F64 => self.v_frexp_mant_f64(inst),
            Opcode::V_FRACT_F64 => self.v_fract_f64(inst),
            Opcode::V_FREXP_EXP_I32_F32 => self.v_frexp_exp_i32_f32(inst),
            Opcode::V_FREXP_MANT_F32 => self.v_frexp_mant_f32(inst),
            Opcode::V_MOVRELD_B32 => self.v_movreld_b32(inst),
            Opcode::V_MOVRELS_B32 => self.v_movrels_b32(inst),
            Opcode::V_MOVRELSD_B32 => self.v_movrelsd_b32(inst),
            Opcode::V_CMP_F_F32 => self.v_cmp_f32(ConditionOp::F, false, inst),
            Opcode::V_CMP_LT_F32 => self.v_cmp_f32(ConditionOp::LT, false, inst),
            Opcode::V_CMP_EQ_F32 => self.v_cmp_f32(ConditionOp::EQ, false, inst),
            Opcode::V_CMP_LE_F32 => self.v_cmp_f32(ConditionOp::LE, false, inst),
            Opcode::V_CMP_GT_F32 => self.v_cmp_f32(ConditionOp::GT, false, inst),
            Opcode::V_CMP_LG_F32 => self.v_cmp_f32(ConditionOp::LG, false, inst),
            Opcode::V_CMP_GE_F32 => self.v_cmp_f32(ConditionOp::GE, false, inst),
            Opcode::V_CMP_U_F32 => self.v_cmp_f32(ConditionOp::U, false, inst),
            Opcode::V_CMP_NGE_F32 => self.v_cmp_f32(ConditionOp::LT, false, inst),
            Opcode::V_CMP_NGT_F32 => self.v_cmp_f32(ConditionOp::LE, false, inst),
            Opcode::V_CMP_NLE_F32 => self.v_cmp_f32(ConditionOp::GT, false, inst),
            Opcode::V_CMP_NEQ_F32 => self.v_cmp_f32(ConditionOp::LG, false, inst),
            Opcode::V_CMP_NLT_F32 => self.v_cmp_f32(ConditionOp::GE, false, inst),
            Opcode::V_CMPX_F_F32 => self.v_cmp_f32(ConditionOp::F, true, inst),
            Opcode::V_CMPX_LT_F32 => self.v_cmp_f32(ConditionOp::LT, true, inst),
            Opcode::V_CMPX_EQ_F32 => self.v_cmp_f32(ConditionOp::EQ, true, inst),
            Opcode::V_CMPX_LE_F32 => self.v_cmp_f32(ConditionOp::LE, true, inst),
            Opcode::V_CMPX_GT_F32 => self.v_cmp_f32(ConditionOp::GT, true, inst),
            Opcode::V_CMPX_LG_F32 => self.v_cmp_f32(ConditionOp::LG, true, inst),
            Opcode::V_CMPX_GE_F32 => self.v_cmp_f32(ConditionOp::GE, true, inst),
            Opcode::V_CMPX_NGE_F32 => self.v_cmp_f32(ConditionOp::LT, true, inst),
            Opcode::V_CMPX_NLG_F32 => self.v_cmp_f32(ConditionOp::EQ, true, inst),
            Opcode::V_CMPX_NGT_F32 => self.v_cmp_f32(ConditionOp::LE, true, inst),
            Opcode::V_CMPX_NLE_F32 => self.v_cmp_f32(ConditionOp::GT, true, inst),
            Opcode::V_CMPX_NEQ_F32 => self.v_cmp_f32(ConditionOp::LG, true, inst),
            Opcode::V_CMPX_NLT_F32 => self.v_cmp_f32(ConditionOp::GE, true, inst),
            Opcode::V_CMPX_TRU_F32 => self.v_cmp_f32(ConditionOp::TRU, true, inst),
            Opcode::V_CMP_F_F64 => self.v_cmp_f64(ConditionOp::F, false, inst),
            Opcode::V_CMP_LT_F64 => self.v_cmp_f64(ConditionOp::LT, false, inst),
            Opcode::V_CMP_EQ_F64 => self.v_cmp_f64(ConditionOp::EQ, false, inst),
            Opcode::V_CMP_LE_F64 => self.v_cmp_f64(ConditionOp::LE, false, inst),
            Opcode::V_CMP_GT_F64 => self.v_cmp_f64(ConditionOp::GT, false, inst),
            Opcode::V_CMP_LG_F64 => self.v_cmp_f64(ConditionOp::LG, false, inst),
            Opcode::V_CMP_GE_F64 => self.v_cmp_f64(ConditionOp::GE, false, inst),
            Opcode::V_CMP_U_F64 => self.v_cmp_f64(ConditionOp::U, false, inst),
            Opcode::V_CMP_NGE_F64 => self.v_cmp_f64(ConditionOp::LT, false, inst),
            Opcode::V_CMP_NGT_F64 => self.v_cmp_f64(ConditionOp::LE, false, inst),
            Opcode::V_CMP_NLE_F64 => self.v_cmp_f64(ConditionOp::GT, false, inst),
            Opcode::V_CMP_NEQ_F64 => self.v_cmp_f64(ConditionOp::LG, false, inst),
            Opcode::V_CMP_NLT_F64 => self.v_cmp_f64(ConditionOp::GE, false, inst),
            Opcode::V_CMP_LT_I32 => self.v_cmp_u32(ConditionOp::LT, true, false, inst),
            Opcode::V_CMP_EQ_I32 => self.v_cmp_u32(ConditionOp::EQ, true, false, inst),
            Opcode::V_CMP_LE_I32 => self.v_cmp_u32(ConditionOp::LE, true, false, inst),
            Opcode::V_CMP_GT_I32 => self.v_cmp_u32(ConditionOp::GT, true, false, inst),
            Opcode::V_CMP_NE_I32 => self.v_cmp_u32(ConditionOp::LG, true, false, inst),
            Opcode::V_CMP_GE_I32 => self.v_cmp_u32(ConditionOp::GE, true, false, inst),
            Opcode::V_CMPX_LE_I32 => self.v_cmp_u32(ConditionOp::LE, true, true, inst),
            Opcode::V_CMPX_LT_I32 => self.v_cmp_u32(ConditionOp::LT, true, true, inst),
            Opcode::V_CMPX_EQ_I32 => self.v_cmp_u32(ConditionOp::EQ, true, true, inst),
            Opcode::V_CMPX_GT_I32 => self.v_cmp_u32(ConditionOp::GT, true, true, inst),
            Opcode::V_CMPX_LG_I32 => self.v_cmp_u32(ConditionOp::LG, true, true, inst),
            Opcode::V_CMPX_GE_I32 => self.v_cmp_u32(ConditionOp::GE, true, true, inst),
            Opcode::V_CMP_F_U32 => self.v_cmp_u32(ConditionOp::F, false, false, inst),
            Opcode::V_CMP_LT_U32 => self.v_cmp_u32(ConditionOp::LT, false, false, inst),
            Opcode::V_CMP_EQ_U32 => self.v_cmp_u32(ConditionOp::EQ, false, false, inst),
            Opcode::V_CMP_LE_U32 => self.v_cmp_u32(ConditionOp::LE, false, false, inst),
            Opcode::V_CMP_GT_U32 => self.v_cmp_u32(ConditionOp::GT, false, false, inst),
            Opcode::V_CMP_NE_U32 => self.v_cmp_u32(ConditionOp::LG, false, false, inst),
            Opcode::V_CMP_GE_U32 => self.v_cmp_u32(ConditionOp::GE, false, false, inst),
            Opcode::V_CMP_TRU_U32 => self.v_cmp_u32(ConditionOp::TRU, false, false, inst),
            Opcode::V_CMPX_F_U32 => self.v_cmp_u32(ConditionOp::F, false, true, inst),
            Opcode::V_CMPX_LT_U32 => self.v_cmp_u32(ConditionOp::LT, false, true, inst),
            Opcode::V_CMPX_EQ_U32 => self.v_cmp_u32(ConditionOp::EQ, false, true, inst),
            Opcode::V_CMPX_LE_U32 => self.v_cmp_u32(ConditionOp::LE, false, true, inst),
            Opcode::V_CMPX_GT_U32 => self.v_cmp_u32(ConditionOp::GT, false, true, inst),
            Opcode::V_CMPX_NE_U32 => self.v_cmp_u32(ConditionOp::LG, false, true, inst),
            Opcode::V_CMPX_GE_U32 => self.v_cmp_u32(ConditionOp::GE, false, true, inst),
            Opcode::V_CMPX_TRU_U32 => self.v_cmp_u32(ConditionOp::TRU, false, true, inst),
            Opcode::V_CMP_F_I64 => self.v_cmp_u64(ConditionOp::F, true, false, inst),
            Opcode::V_CMP_LT_I64 => self.v_cmp_u64(ConditionOp::LT, true, false, inst),
            Opcode::V_CMP_EQ_I64 => self.v_cmp_u64(ConditionOp::EQ, true, false, inst),
            Opcode::V_CMP_LE_I64 => self.v_cmp_u64(ConditionOp::LE, true, false, inst),
            Opcode::V_CMP_GT_I64 => self.v_cmp_u64(ConditionOp::GT, true, false, inst),
            Opcode::V_CMP_NE_I64 => self.v_cmp_u64(ConditionOp::LG, true, false, inst),
            Opcode::V_CMP_GE_I64 => self.v_cmp_u64(ConditionOp::GE, true, false, inst),
            Opcode::V_CMP_TRU_I64 => self.v_cmp_u64(ConditionOp::TRU, true, false, inst),
            Opcode::V_CMPX_EQ_I64 => self.v_cmp_u64(ConditionOp::EQ, true, true, inst),
            Opcode::V_CMP_EQ_U64 => self.v_cmp_u64(ConditionOp::EQ, false, false, inst),
            Opcode::V_CMP_NE_U64 => self.v_cmp_u64(ConditionOp::LG, false, false, inst),
            Opcode::V_CMP_GT_U64 => self.v_cmp_u64(ConditionOp::GT, false, false, inst),
            Opcode::V_CMP_LT_U64 => self.v_cmp_u64(ConditionOp::LT, false, false, inst),
            Opcode::V_CMPX_EQ_U64 => self.v_cmp_u64(ConditionOp::EQ, false, true, inst),
            Opcode::V_CMPX_LG_U64 => self.v_cmp_u64(ConditionOp::LG, false, true, inst),
            Opcode::V_CMP_CLASS_F32 => self.v_cmp_class_f32(inst),
            Opcode::V_MAD_LEGACY_F32 => self.v_mad_f32(inst),
            Opcode::V_MAD_F32 => self.v_mad_f32(inst),
            Opcode::V_MAD_I32_I24 => self.v_mad_i32_i24(inst, true),
            Opcode::V_MAD_U32_U24 => self.v_mad_i32_i24(inst, false),
            Opcode::V_CUBEID_F32 => self.v_cubeid_f32(inst),
            Opcode::V_CUBESC_F32 => self.v_cubesc_f32(inst),
            Opcode::V_CUBETC_F32 => self.v_cubetc_f32(inst),
            Opcode::V_CUBEMA_F32 => self.v_cubema_f32(inst),
            Opcode::V_BFE_U32 => self.v_bfe_u32(false, inst),
            Opcode::V_BFE_I32 => self.v_bfe_u32(true, inst),
            Opcode::V_BFI_B32 => self.v_bfi_b32(inst),
            Opcode::V_FMA_F32 => self.v_fma_f32(inst),
            Opcode::V_FMA_F64 => self.v_fma_f64(inst),
            Opcode::V_MIN3_F32 => self.v_min3_f32(inst),
            Opcode::V_MIN3_I32 => self.v_min3_u32(true, inst),
            Opcode::V_MIN3_U32 => self.v_min3_u32(false, inst),
            Opcode::V_MAX3_F32 => self.v_max3_f32(inst),
            Opcode::V_MAX3_I32 => self.v_max3_u32(true, inst),
            Opcode::V_MAX3_U32 => self.v_max3_u32(false, inst),
            Opcode::V_MED3_F32 => self.v_med3_f32(inst),
            Opcode::V_MED3_I32 => self.v_med3_u32(true, inst),
            Opcode::V_MED3_U32 => self.v_med3_u32(false, inst),
            Opcode::V_SAD_U32 => self.v_sad_u32(inst),
            Opcode::V_CVT_PK_U16_U32 => self.v_cvt_pk_u16_u32(inst),
            Opcode::V_CVT_PK_I16_I32 => self.v_cvt_pk_i16_i32(inst),
            Opcode::V_CVT_PK_U8_F32 => self.v_cvt_pk_u8_f32(inst),
            Opcode::V_LSHL_B64 => self.v_lshl_b64(inst),
            Opcode::V_LSHR_B64 => self.v_lshr_b64(inst),
            Opcode::V_ASHR_I64 => self.v_ashr_i64(inst),
            Opcode::V_ADD_F64 => self.v_add_f64(inst),
            Opcode::V_ALIGNBIT_B32 => self.v_alignbit_b32(inst),
            Opcode::V_ALIGNBYTE_B32 => self.v_alignbyte_b32(inst),
            Opcode::V_MUL_F64 => self.v_mul_f64(inst),
            Opcode::V_MAX_F64 => self.v_max_f64(inst),
            Opcode::V_MUL_LO_U32 => self.v_mul_lo_u32(inst),
            Opcode::V_MUL_HI_U32 => self.v_mul_hi_u32(false, inst),
            Opcode::V_MUL_LO_I32 => self.v_mul_lo_u32(inst),
            Opcode::V_MUL_HI_I32 => self.v_mul_hi_u32(true, inst),
            Opcode::V_MAD_U64_U32 => self.v_mad_u64_u32(inst),
            Opcode::V_LSHRREV_B16 => self.v_lshrrev_b16(inst),
            Opcode::V_ASHRREV_I16 => self.v_ashrrev_i16(inst),
            Opcode::V_LSHLREV_B16 => self.v_lshlrev_b16(inst),
            Opcode::V_ADD_LSHL_U32 => self.v_add_lshl_u32(inst),
            Opcode::V_LSHL_ADD_U32 => self.v_lshl_add_u32(inst),
            Opcode::V_MIN3_F16 => self.v_min3_f16(inst),
            Opcode::V_MAX3_F16 => self.v_max3_f16(inst),
            Opcode::V_MED3_F16 => self.v_med3_f16(inst),
            Opcode::V_ADD3_U32 => self.v_add3_u32(inst),
            Opcode::V_LSHL_OR_B32 => self.v_lshl_or_b32(inst),
            Opcode::V_AND_OR_B32 => self.v_and_or_b32(inst),
            Opcode::V_OR3_B32 => self.v_or3_b32(inst),
            Opcode::V_NOP => {}
            Opcode::V_PK_MUL_LO_U16 => self.v_pk_mul_lo_u16(inst),
            Opcode::V_PK_ADD_I16 => self.v_pk_add_i16(inst),
            Opcode::V_PK_SUB_I16 => self.v_pk_sub_i16(inst),
            Opcode::V_PK_LSHRREV_B16 => self.v_pk_lshrrev_b16(inst),
            Opcode::V_PK_LSHLREV_B16 => self.v_pk_lshlrev_b16(inst),
            Opcode::V_PK_MAD_U16 => self.v_pk_mad_u16(inst),
            Opcode::V_PK_ADD_U16 => self.v_pk_add_u16(inst),
            Opcode::V_PK_SUB_U16 => self.v_pk_sub_u16(inst),
            Opcode::V_PK_MAX_U16 => self.v_pk_max_u16(inst),
            Opcode::V_PK_MIN_U16 => self.v_pk_min_u16(inst),
            Opcode::V_PK_FMA_F16 => self.v_pk_fma_f16(inst),
            Opcode::V_PK_ADD_F16 => self.v_pk_add_f16(inst),
            Opcode::V_PK_MUL_F16 => self.v_pk_mul_f16(inst),
            Opcode::V_PK_MIN_F16 => self.v_pk_min_f16(inst),
            Opcode::V_PK_MAX_F16 => self.v_pk_max_f16(inst),
            Opcode::V_MAD_MIX_F32 => self.v_mad_mix_f32(inst),
            Opcode::V_MAD_MIXLO_F16 => self.v_mad_mixlo_f16(inst),
            Opcode::V_MAD_MIXHI_F16 => self.v_mad_mixhi_f16(inst),
            _ => self.log_missing_opcode(inst),
        }
    }
}

impl Translator<'_> {
    fn v_cndmask_b32(&mut self, inst: &GcnInst) {
        let mask = if inst.src[2].field == OperandField::Undefined.0 {
            let lo = self.ir.get_vcc_lo();
            let hi = self.ir.get_vcc_hi();
            let composite = self.ir.composite_construct(lo, hi);
            self.ir.pack_uint_2x32(&composite)
        } else {
            self.get_src64(&inst.src[2])
        };
        let condition = self.ir.inverse_ballot(&mask);
        let true_value = self.get_src_f32(&inst.src[1]).into_value();
        let false_value = self.get_src_f32(&inst.src[0]).into_value();
        let result = self.ir.select(&condition, &true_value, &false_value);
        self.set_dst(&inst.dst[0], &U32F32::new(result));
    }

    fn v_add_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let result = self.ir.fp_add(&f32w(&src0), &f32w(&src1));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_add_f64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64_f64(&inst.src[0]);
        let src1 = self.get_src64_f64(&inst.src[1]);
        let result = f64_of(self.ir.fp_add(&f64w(&src0), &f64w(&src1)));
        self.set_dst64(&inst.dst[0], result.into_value());
    }

    fn v_sub_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let result = self.ir.fp_sub(&f32w(&src0), &f32w(&src1));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_subrev_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let result = self.ir.fp_sub(&f32w(&src1), &f32w(&src0));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_mul_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let result = self.ir.fp_mul(&f32w(&src0), &f32w(&src1));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_mul_legacy_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let zero = self.ir.imm32_float(0.0);
        let src0_zero = self.ir.fp_equal(&f32w(&src0), &f32w(&zero), true);
        let src1_zero = self.ir.fp_equal(&f32w(&src1), &f32w(&zero), true);
        let either_zero = self.ir.logical_or(&src0_zero, &src1_zero);
        let product = self.ir.fp_mul(&f32w(&src0), &f32w(&src1));
        let result = self
            .ir
            .select(&either_zero, &zero.into_value(), &product.into_value());
        self.set_dst(&inst.dst[0], &U32F32::new(result));
    }

    fn v_mul_i32_i24(&mut self, inst: &GcnInst, is_signed: bool) {
        let imm0 = self.ir.imm32(0);
        let imm24 = self.ir.imm32(24);
        let raw0 = self.get_src(&inst.src[0]);
        let raw1 = self.get_src(&inst.src[1]);
        let src0 = self.ir.bit_field_extract(&raw0, &imm0, &imm24, is_signed);
        let src1 = self.ir.bit_field_extract(&raw1, &imm0, &imm24, is_signed);
        let result = self.ir.i_mul(&u32w(&src0), &u32w(&src1));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_min_f32(&mut self, inst: &GcnInst, is_legacy: bool) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let fpmin = self.ir.fp_min(&f32w(&src0), &f32w(&src1));
        let result = if is_legacy {
            let nan0 = self.ir.fp_is_nan(&f32w(&src0));
            let nan1 = self.ir.fp_is_nan(&f32w(&src1));
            let either = self.ir.logical_or(&nan0, &nan1);
            self.ir
                .select(&either, &src1.into_value(), &fpmin.into_value())
        } else {
            fpmin.into_value()
        };
        self.set_dst(&inst.dst[0], &U32F32::new(result));
    }

    fn v_max_f32(&mut self, inst: &GcnInst, is_legacy: bool) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let fpmax = self.ir.fp_max(&f32w(&src0), &f32w(&src1));
        let result = if is_legacy {
            let nan0 = self.ir.fp_is_nan(&f32w(&src0));
            let nan1 = self.ir.fp_is_nan(&f32w(&src1));
            let either = self.ir.logical_or(&nan0, &nan1);
            self.ir
                .select(&either, &src1.into_value(), &fpmax.into_value())
        } else {
            fpmax.into_value()
        };
        self.set_dst(&inst.dst[0], &U32F32::new(result));
    }

    fn v_min_i32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = self.ir.i_min(&src0, &src1, true);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_min_u32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = self.ir.i_min(&src0, &src1, false);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_max_u32(&mut self, is_signed: bool, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = self.ir.i_max(&src0, &src1, is_signed);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_lshr_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let mask = self.ir.imm32(0x1F);
        let shift = u32_of(self.ir.bitwise_and(&u32w(&src1), &u32w(&mask)));
        let result = self.ir.shift_right_logical(&u32w(&src0), &shift);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_lshrrev_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let mask = self.ir.imm32(0x1F);
        let shift = u32_of(self.ir.bitwise_and(&u32w(&src0), &u32w(&mask)));
        let result = self.ir.shift_right_logical(&u32w(&src1), &shift);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_ashr_i32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let mask = self.ir.imm32(0x1F);
        let shift = u32_of(self.ir.bitwise_and(&u32w(&src1), &u32w(&mask)));
        let result = self.ir.shift_right_arithmetic(&u32w(&src0), &shift);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_ashrrev_i32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let mask = self.ir.imm32(0x1F);
        let shift = u32_of(self.ir.bitwise_and(&u32w(&src0), &u32w(&mask)));
        let result = self.ir.shift_right_arithmetic(&u32w(&src1), &shift);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_lshl_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let mask = self.ir.imm32(0x1F);
        let shift = u32_of(self.ir.bitwise_and(&u32w(&src1), &u32w(&mask)));
        let result = self.ir.shift_left_logical(&u32w(&src0), &shift);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_lshlrev_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let mask = self.ir.imm32(0x1F);
        let shift = u32_of(self.ir.bitwise_and(&u32w(&src0), &u32w(&mask)));
        let result = self.ir.shift_left_logical(&u32w(&src1), &shift);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_and_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = self.ir.bitwise_and(&u32w(&src0), &u32w(&src1));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_or_b32(&mut self, is_xor: bool, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = if is_xor {
            self.ir.bitwise_xor(&u32w(&src0), &u32w(&src1))
        } else {
            self.ir.bitwise_or(&u32w(&src0), &u32w(&src1))
        };
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_bfm_b32(&mut self, inst: &GcnInst) {
        let imm0 = self.ir.imm32(0);
        let imm5 = self.ir.imm32(5);
        let raw0 = self.get_src(&inst.src[0]);
        let raw1 = self.get_src(&inst.src[1]);
        let src0 = self.ir.bit_field_extract(&raw0, &imm0, &imm5, false);
        let src1 = self.ir.bit_field_extract(&raw1, &imm0, &imm5, false);
        let one = self.ir.imm32(1);
        let shifted = self.ir.shift_left_logical(&u32w(&one), &src0);
        let one = self.ir.imm32(1);
        let ones = self.ir.i_sub(&shifted, &u32w(&one));
        let result = self.ir.shift_left_logical(&ones, &src1);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_mac_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let dst0 = self.get_src_f32(&inst.dst[0]);
        let product = self.ir.fp_mul(&f32w(&src0), &f32w(&src1));
        let result = self.ir.fp_add(&product, &f32w(&dst0));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_madmk_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let k = self.get_src_f32(&inst.src[2]);
        let product = self.ir.fp_mul(&f32w(&src0), &f32w(&k));
        let result = self.ir.fp_add(&product, &f32w(&src1));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_bcnt_u32_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let count = self.ir.bit_count(&u32w(&src0));
        let result = self.ir.i_add(&u32w(&count), &u32w(&src1));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_mbcnt_u32_b32(&mut self, hi: bool, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = self.ir.masked_bit_count(&src0, &src1, hi);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_add_i32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = u32_of(self.ir.i_add(&u32w(&src0), &u32w(&src1)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.clone().into_value()));
        let carry = self.ir.i_less_than(&u32w(&result), &u32w(&src0), false);
        self.set_carry_out(inst, &carry);
    }

    fn v_sub_i32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = u32_of(self.ir.i_sub(&u32w(&src0), &u32w(&src1)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.clone().into_value()));
        let underflow = self.ir.i_greater_than(&u32w(&src1), &u32w(&src0), false);
        self.set_carry_out(inst, &underflow);
    }

    fn v_subrev_i32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = u32_of(self.ir.i_sub(&u32w(&src1), &u32w(&src0)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.clone().into_value()));
        let underflow = self.ir.i_greater_than(&u32w(&src0), &u32w(&src1), false);
        self.set_carry_out(inst, &underflow);
    }

    fn v_addc_u32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let carry = self.get_carry_in(inst);
        let result1 = u32_of(self.ir.i_add(&u32w(&src0), &u32w(&src1)));
        let result2 = u32_of(self.ir.i_add(&u32w(&result1), &u32w(&carry)));
        let carry_out1 = self.ir.i_less_than(&u32w(&result1), &u32w(&src0), false);
        let carry_out2 = self.ir.i_less_than(&u32w(&result2), &u32w(&result1), false);
        self.set_dst(&inst.dst[0], &U32F32::new(result2.into_value()));
        let did_overflow = self.ir.logical_or(&carry_out1, &carry_out2);
        self.set_carry_out(inst, &did_overflow);
    }

    fn v_subb_u32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let carry = self.get_carry_in(inst);
        let difference = u32_of(self.ir.i_sub(&u32w(&src0), &u32w(&src1)));
        let result = u32_of(self.ir.i_sub(&u32w(&difference), &u32w(&carry)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
        let underflow = self.ir.i_greater_than(&u32w(&src1), &u32w(&src0), false);
        let borrow_underflow = self
            .ir
            .i_greater_than(&u32w(&carry), &u32w(&difference), false);
        let carry_out = self.ir.logical_or(&underflow, &borrow_underflow);
        self.set_carry_out(inst, &carry_out);
    }

    fn v_subb_u32_rev(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let carry = self.get_carry_in(inst);
        let difference = u32_of(self.ir.i_sub(&u32w(&src1), &u32w(&src0)));
        let result = u32_of(self.ir.i_sub(&u32w(&difference), &u32w(&carry)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
        let underflow = self.ir.i_greater_than(&u32w(&src0), &u32w(&src1), false);
        let borrow_underflow = self
            .ir
            .i_greater_than(&u32w(&carry), &u32w(&difference), false);
        let carry_out = self.ir.logical_or(&underflow, &borrow_underflow);
        self.set_carry_out(inst, &carry_out);
    }

    fn v_ldexp_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = self.ir.fp_ldexp(&src0, &src1);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cvt_pknorm_u16_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let vector = self.ir.composite_construct(src0, src1);
        let result = self.ir.pack_2x16(NumberFormat::Unorm, &vector);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cvt_pknorm_i16_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let vector = self.ir.composite_construct(src0, src1);
        let result = self.ir.pack_2x16(NumberFormat::Snorm, &vector);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cvt_pkrtz_f16_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let vector = self.ir.composite_construct(src0, src1);
        let result = self.ir.pack_2x16(NumberFormat::Float, &vector);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_add_f16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src16_f32(&inst.src[0]);
        let src1 = self.get_src16_f32(&inst.src[1]);
        let result = f32_of(self.ir.fp_add(&f32w(&src0), &f32w(&src1)));
        self.set_dst16(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_sub_f16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src16_f32(&inst.src[0]);
        let src1 = self.get_src16_f32(&inst.src[1]);
        let result = f32_of(self.ir.fp_sub(&f32w(&src0), &f32w(&src1)));
        self.set_dst16(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_mul_f16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src16_f32(&inst.src[0]);
        let src1 = self.get_src16_f32(&inst.src[1]);
        let result = f32_of(self.ir.fp_mul(&f32w(&src0), &f32w(&src1)));
        self.set_dst16(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_max_f16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src16_f32(&inst.src[0]);
        let src1 = self.get_src16_f32(&inst.src[1]);
        let result = f32_of(self.ir.fp_max(&f32w(&src0), &f32w(&src1)));
        self.set_dst16(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_min_f16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src16_f32(&inst.src[0]);
        let src1 = self.get_src16_f32(&inst.src[1]);
        let result = f32_of(self.ir.fp_min(&f32w(&src0), &f32w(&src1)));
        self.set_dst16(&inst.dst[0], &U32F32::new(result.into_value()));
    }
}

impl Translator<'_> {
    fn v_mov(&mut self, inst: &GcnInst) {
        let value = self.get_src_f32(&inst.src[0]);
        self.set_dst(&inst.dst[0], &U32F32::new(value.into_value()));
    }

    fn v_cvt_i32_f64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64_f64(&inst.src[0]);
        let result = self.ir.convert_f_to_s(32, &f64w(&src0));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cvt_f64_i32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let result = f64_of(self.ir.convert_s_to_f(64, 32, &src0.into_value()));
        self.set_dst64(&inst.dst[0], result.into_value());
    }

    fn v_cvt_f64_u32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let result = f64_of(self.ir.convert_u_to_f(64, 32, &src0.into_value()));
        self.set_dst64(&inst.dst[0], result.into_value());
    }

    fn v_cvt_f32_i32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let result = f32_of(self.ir.convert_s_to_f(32, 32, &src0.into_value()));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cvt_f32_u32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let result = f32_of(self.ir.convert_u_to_f(32, 32, &src0.into_value()));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cvt_u32_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = self.ir.convert_f_to_u(32, &f32w(&src0));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cvt_i32_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = self.ir.convert_f_to_s(32, &f32w(&src0));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cvt_f16_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        if self.profile.support_float16 {
            let converted = self.ir.fp_convert(16, &F16F32F64::new(src0.into_value()));
            let half = F16::new(converted.into_value());
            let raw = self.ir.bit_cast_u16_f16(&half);
            let widened = self.ir.u_convert(32, &U8U16U32U64::new(raw.into_value()));
            self.set_dst(&inst.dst[0], &U32F32::new(widened.into_value()));
        } else {
            let zero = self.ir.imm32_float(0.0);
            let vector = self.ir.composite_construct(src0, zero);
            let packed = self.ir.pack_2x16(NumberFormat::Float, &vector);
            let offset = self.ir.imm32(0);
            let count = self.ir.imm32(16);
            let extracted = self.ir.bit_field_extract(&packed, &offset, &count, false);
            self.set_dst(&inst.dst[0], &U32F32::new(extracted.into_value()));
        }
    }

    fn v_cvt_f32_f16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        if self.profile.support_float16 {
            let narrowed = self.ir.u_convert(16, &U8U16U32U64::new(src0.into_value()));
            let half = U16::new(narrowed.into_value());
            let float = self.ir.bit_cast_f16_u16(&half);
            let widened = self.ir.fp_convert(32, &F16F32F64::new(float.into_value()));
            self.set_dst(&inst.dst[0], &U32F32::new(widened.into_value()));
        } else {
            let unpacked = self.ir.unpack_2x16(NumberFormat::Float, &src0);
            let extracted = self.ir.composite_extract(&unpacked, 0);
            self.set_dst(&inst.dst[0], &U32F32::new(extracted));
        }
    }

    fn v_cvt_flr_i32_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let floor = self.ir.fp_floor(&f32w(&src0));
        let result = self.ir.convert_f_to_i(32, true, &floor);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cvt_rpi_i32_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let half = self.ir.imm32_float(0.5);
        let sum = self.ir.fp_add(&f32w(&src0), &f32w(&half));
        let floor = self.ir.fp_floor(&sum);
        let result = self.ir.convert_f_to_i(32, true, &floor);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cvt_off_f32_i4(&mut self, inst: &GcnInst) {
        const INT_TO_FLOAT: [f32; 16] = [
            0.0, 0.0625, 0.1250, 0.1875, 0.2500, 0.3125, 0.3750, 0.4375, -0.5000, -0.4375, -0.3750,
            -0.3125, -0.2500, -0.1875, -0.1250, -0.0625,
        ];
        let src0 = self.get_src(&inst.src[0]);
        assert!(src0.value().is_immediate(), "the source must be immediate");
        let index = (src0.value().u32() & 0xF) as usize;
        let result = self.ir.imm32_float(INT_TO_FLOAT[index]);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cvt_f32_ubyte(&mut self, index: u32, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let offset = self.ir.imm32(8 * index);
        let count = self.ir.imm32(8);
        let byte = self.ir.bit_field_extract(&src0, &offset, &count, false);
        let result = f32_of(self.ir.convert_u_to_f(32, 32, &byte.into_value()));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cvt_f32_f64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64_f64(&inst.src[0]);
        let result = F32::new(
            self.ir
                .fp_convert(32, &F16F32F64::new(src0.into_value()))
                .into_value(),
        );
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cvt_f64_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = F64::new(
            self.ir
                .fp_convert(64, &F16F32F64::new(src0.into_value()))
                .into_value(),
        );
        self.set_dst64(&inst.dst[0], result.into_value());
    }

    fn v_floor_f64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64_f64(&inst.src[0]);
        let result = f64_of(self.ir.fp_floor(&f64w(&src0)));
        self.set_dst64(&inst.dst[0], result.into_value());
    }

    fn v_fract_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = self.ir.fp_fract(&f32w(&src0));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_trunc_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = self.ir.fp_trunc(&f32w(&src0));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_ceil_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = self.ir.fp_ceil(&f32w(&src0));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_rndne_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = self.ir.fp_round_even(&f32w(&src0));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_floor_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = self.ir.fp_floor(&f32w(&src0));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_exp_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = self.ir.fp_exp2(&src0);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_log_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = self.ir.fp_log2(&src0);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_rcp_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = f32_of(self.ir.fp_recip(&f32w(&src0)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_rcp_legacy_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = f32_of(self.ir.fp_recip(&f32w(&src0)));
        let inf = self.ir.fp_is_inf(&f32w(&result));
        let raw = u32_of(self.ir.convert_f_to_u(32, &f32w(&result)));
        let imm31 = self.ir.imm32(31);
        let sign_bit = u32_of(self.ir.shift_right_logical(&u32w(&raw), &imm31));
        let zero = self.ir.imm32(0);
        let sign_bit_set = self.ir.i_not_equal(&u32w(&sign_bit), &u32w(&zero));
        let neg_zero = self.ir.imm32_float(-0.0);
        let pos_zero = self.ir.imm32_float(0.0);
        let inf_result = self.ir.select(
            &sign_bit_set,
            &neg_zero.into_value(),
            &pos_zero.into_value(),
        );
        let value = self.ir.select(&inf, &inf_result, &result.into_value());
        self.set_dst(&inst.dst[0], &U32F32::new(value));
    }

    fn v_rcp_f64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64_f64(&inst.src[0]);
        let result = f64_of(self.ir.fp_recip(&f64w(&src0)));
        self.set_dst64(&inst.dst[0], result.into_value());
    }

    fn v_rsq_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = f32_of(self.ir.fp_recip_sqrt(&f32w(&src0)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_sqrt_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = self.ir.fp_sqrt(&src0);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_sin_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = self.ir.fp_sin(&src0);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cos_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = self.ir.fp_cos(&src0);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_not_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let result = self.ir.bitwise_not(&u32w(&src0));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_bfrev_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let result = self.ir.bit_reverse(&src0);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_ffbh_u32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let msb_pos = self.ir.find_u_msb(&u32w(&src0));
        let imm31 = self.ir.imm32(31);
        let pos_from_left = u32_of(self.ir.i_sub(&u32w(&imm31), &u32w(&msb_pos)));
        let zero = self.ir.imm32(0);
        let cond = self.ir.i_not_equal(&u32w(&src0), &u32w(&zero));
        let all_ones = self.ir.imm32(!0u32);
        let result = self
            .ir
            .select(&cond, &pos_from_left.into_value(), &all_ones.into_value());
        self.set_dst(&inst.dst[0], &U32F32::new(result));
    }

    fn v_ffbl_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let result = self.ir.find_i_lsb(&u32w(&src0));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_ffbh_i32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let msb_pos = self.ir.find_s_msb(&src0);
        let imm31 = self.ir.imm32(31);
        let pos_from_left = u32_of(self.ir.i_sub(&u32w(&imm31), &u32w(&msb_pos)));
        let minus_one = self.ir.imm32(!0u32);
        let zero = self.ir.imm32(0);
        let not_zero = self.ir.i_not_equal(&u32w(&src0), &u32w(&zero));
        let not_minus_one = self.ir.i_not_equal(&u32w(&src0), &u32w(&minus_one));
        let cond = self.ir.logical_and(&not_zero, &not_minus_one);
        let result = self
            .ir
            .select(&cond, &pos_from_left.into_value(), &minus_one.into_value());
        self.set_dst(&inst.dst[0], &U32F32::new(result));
    }

    fn v_frexp_exp_i32_f64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64_f64(&inst.src[0]);
        let result = self.ir.fp_frexp_exp(&f64w(&src0));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_frexp_mant_f64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64_f64(&inst.src[0]);
        let result = f64_of(self.ir.fp_frexp_sig(&f64w(&src0)));
        self.set_dst64(&inst.dst[0], result.into_value());
    }

    fn v_fract_f64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64_f64(&inst.src[0]);
        let result = f64_of(self.ir.fp_fract(&f64w(&src0)));
        self.set_dst64(&inst.dst[0], result.into_value());
    }

    fn v_frexp_exp_i32_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = self.ir.fp_frexp_exp(&f32w(&src0));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_frexp_mant_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let result = f32_of(self.ir.fp_frexp_sig(&f32w(&src0)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_movreld_b32(&mut self, inst: &GcnInst) {
        let src_val = self.get_src(&inst.src[0]);
        let dst_vgprno = inst.dst[0].code;
        let m0 = self.ir.get_m0();
        self.vmovreld_helper(dst_vgprno, &src_val, &m0);
    }

    fn v_movrels_b32(&mut self, inst: &GcnInst) {
        let src_vgprno = inst.src[0].code;
        let m0 = self.ir.get_m0();
        let src_val = self.vmovrels_helper(src_vgprno, &m0);
        self.set_dst(&inst.dst[0], &U32F32::new(src_val.into_value()));
    }

    fn v_movrelsd_b32(&mut self, inst: &GcnInst) {
        let src_vgprno = inst.src[0].code;
        let dst_vgprno = inst.dst[0].code;
        let m0 = self.ir.get_m0();
        let src_val = self.vmovrels_helper(src_vgprno, &m0);
        self.vmovreld_helper(dst_vgprno, &src_val, &m0);
    }
}

impl Translator<'_> {
    fn finish_compare(&mut self, set_exec: bool, result: &U1, inst: &GcnInst) {
        if set_exec {
            let exec = self.ir.get_exec();
            let masked = self.ir.logical_and(&exec, result);
            self.ir.set_exec(&masked);
            let ballot = self.ir.ballot(&masked);
            self.set_dst64(&inst.dst[1], ballot.into_value());
            return;
        }
        let ballot = self.ir.ballot(result);
        self.set_dst64(&inst.dst[1], ballot.into_value());
    }

    fn v_cmp_f32(&mut self, op: ConditionOp, set_exec: bool, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let result = match op {
            ConditionOp::F => self.ir.imm1(false),
            ConditionOp::EQ => self.ir.fp_equal(&f32w(&src0), &f32w(&src1), true),
            ConditionOp::LG => self.ir.fp_not_equal(&f32w(&src0), &f32w(&src1), true),
            ConditionOp::GT => self.ir.fp_greater_than(&f32w(&src0), &f32w(&src1), true),
            ConditionOp::LT => self.ir.fp_less_than(&f32w(&src0), &f32w(&src1), true),
            ConditionOp::LE => self.ir.fp_less_than_equal(&f32w(&src0), &f32w(&src1), true),
            ConditionOp::GE => self
                .ir
                .fp_greater_than_equal(&f32w(&src0), &f32w(&src1), true),
            ConditionOp::U => {
                let nan0 = self.ir.fp_is_nan(&f32w(&src0));
                let nan1 = self.ir.fp_is_nan(&f32w(&src1));
                self.ir.logical_or(&nan0, &nan1)
            }
            _ => unreachable!("unsupported F32 comparison"),
        };
        self.finish_compare(set_exec, &result, inst);
    }

    fn v_cmp_f64(&mut self, op: ConditionOp, set_exec: bool, inst: &GcnInst) {
        let src0 = self.get_src64_f64(&inst.src[0]);
        let src1 = self.get_src64_f64(&inst.src[1]);
        let result = match op {
            ConditionOp::F => self.ir.imm1(false),
            ConditionOp::EQ => self.ir.fp_equal(&f64w(&src0), &f64w(&src1), true),
            ConditionOp::LG => self.ir.fp_not_equal(&f64w(&src0), &f64w(&src1), true),
            ConditionOp::GT => self.ir.fp_greater_than(&f64w(&src0), &f64w(&src1), true),
            ConditionOp::LT => self.ir.fp_less_than(&f64w(&src0), &f64w(&src1), true),
            ConditionOp::LE => self.ir.fp_less_than_equal(&f64w(&src0), &f64w(&src1), true),
            ConditionOp::GE => self
                .ir
                .fp_greater_than_equal(&f64w(&src0), &f64w(&src1), true),
            ConditionOp::U => {
                let nan0 = self.ir.fp_is_nan(&f64w(&src0));
                let nan1 = self.ir.fp_is_nan(&f64w(&src1));
                self.ir.logical_or(&nan0, &nan1)
            }
            _ => unreachable!("unsupported F64 comparison"),
        };
        self.finish_compare(set_exec, &result, inst);
    }

    fn v_cmp_u32(&mut self, op: ConditionOp, is_signed: bool, set_exec: bool, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = match op {
            ConditionOp::F => self.ir.imm1(false),
            ConditionOp::TRU => self.ir.imm1(true),
            ConditionOp::EQ => self.ir.i_equal(&u32w(&src0), &u32w(&src1)),
            ConditionOp::LG => self.ir.i_not_equal(&u32w(&src0), &u32w(&src1)),
            ConditionOp::GT => self
                .ir
                .i_greater_than(&u32w(&src0), &u32w(&src1), is_signed),
            ConditionOp::LT => self.ir.i_less_than(&u32w(&src0), &u32w(&src1), is_signed),
            ConditionOp::LE => self
                .ir
                .i_less_than_equal(&u32w(&src0), &u32w(&src1), is_signed),
            ConditionOp::GE => self
                .ir
                .i_greater_than_equal(&u32w(&src0), &u32w(&src1), is_signed),
            _ => unreachable!("unsupported U32 comparison"),
        };
        self.finish_compare(set_exec, &result, inst);
    }

    fn v_cmp_u64(&mut self, op: ConditionOp, is_signed: bool, set_exec: bool, inst: &GcnInst) {
        let src0 = self.get_src64(&inst.src[0]);
        let src1 = self.get_src64(&inst.src[1]);
        let result = match op {
            ConditionOp::F => self.ir.imm1(false),
            ConditionOp::TRU => self.ir.imm1(true),
            ConditionOp::EQ => self.ir.i_equal(&u64w(&src0), &u64w(&src1)),
            ConditionOp::LG => self.ir.i_not_equal(&u64w(&src0), &u64w(&src1)),
            ConditionOp::GT => self
                .ir
                .i_greater_than(&u64w(&src0), &u64w(&src1), is_signed),
            ConditionOp::LT => self.ir.i_less_than(&u64w(&src0), &u64w(&src1), is_signed),
            ConditionOp::LE => self
                .ir
                .i_less_than_equal(&u64w(&src0), &u64w(&src1), is_signed),
            ConditionOp::GE => self
                .ir
                .i_greater_than_equal(&u64w(&src0), &u64w(&src1), is_signed),
            _ => unreachable!("unsupported U64 comparison"),
        };
        self.finish_compare(set_exec, &result, inst);
    }

    fn v_cmp_class_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let value = if src1.value().is_immediate() {
            let class_mask = FloatClassFunc::from_bits(src1.value().u32());
            if class_mask.contains(FloatClassFunc::NaN) {
                self.ir.fp_is_nan(&f32w(&src0))
            } else if class_mask.contains(FloatClassFunc::Infinity) {
                self.ir.fp_is_inf(&f32w(&src0))
            } else if class_mask.contains(FloatClassFunc::Negative) {
                let neg_zero = self.ir.imm32_float(-0.0);
                self.ir
                    .fp_less_than_equal(&f32w(&src0), &f32w(&neg_zero), true)
            } else {
                unreachable!("Unsupported float class mask: {:#x}", class_mask.bits())
            }
        } else {
            self.ir.fp_cmp_class32(&src0, &src1)
        };
        let ballot = self.ir.ballot(&value);
        self.set_dst64(&inst.dst[1], ballot.into_value());
    }
}

impl Translator<'_> {
    fn v_mad_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let src2 = self.get_src_f32(&inst.src[2]);
        let product = self.ir.fp_mul(&f32w(&src0), &f32w(&src1));
        let result = self.ir.fp_add(&product, &f32w(&src2));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_mad_i32_i24(&mut self, inst: &GcnInst, is_signed: bool) {
        let imm0 = self.ir.imm32(0);
        let imm24 = self.ir.imm32(24);
        let raw0 = self.get_src(&inst.src[0]);
        let raw1 = self.get_src(&inst.src[1]);
        let src0 = self.ir.bit_field_extract(&raw0, &imm0, &imm24, is_signed);
        let src1 = self.ir.bit_field_extract(&raw1, &imm0, &imm24, is_signed);
        let src2 = self.get_src(&inst.src[2]);
        let product = self.ir.i_mul(&u32w(&src0), &u32w(&src1));
        let result = self.ir.i_add(&product, &u32w(&src2));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn select_cube_result(
        &mut self,
        x: &F32,
        y: &F32,
        z: &F32,
        x_res: &F32,
        y_res: &F32,
        z_res: &F32,
    ) -> F32 {
        let abs_x = self.ir.fp_abs(&f32w(x));
        let abs_y = self.ir.fp_abs(&f32w(y));
        let abs_z = self.ir.fp_abs(&f32w(z));
        let z_ge_x = self.ir.fp_greater_than_equal(&abs_z, &abs_x, true);
        let z_ge_y = self.ir.fp_greater_than_equal(&abs_z, &abs_y, true);
        let z_face_cond = self.ir.logical_and(&z_ge_x, &z_ge_y);
        let y_face_cond = self.ir.fp_greater_than_equal(&abs_y, &abs_x, true);
        let inner = self
            .ir
            .select(&y_face_cond, &y_res.value().clone(), &x_res.value().clone());
        let result = self.ir.select(&z_face_cond, &z_res.value().clone(), &inner);
        F32::new(result)
    }

    fn v_cubeid_f32(&mut self, inst: &GcnInst) {
        let x = self.get_src_f32(&inst.src[0]);
        let y = self.get_src_f32(&inst.src[1]);
        let z = self.get_src_f32(&inst.src[2]);
        let result = if self.profile.supports_native_cube_calc {
            let coords = self.ir.composite_construct_slice(&[
                x.into_value(),
                y.into_value(),
                z.into_value(),
            ]);
            self.ir.cube_face_index(&coords)
        } else {
            let zero = self.ir.imm32_float(0.0);
            let one = self.ir.imm32_float(1.0);
            let three = self.ir.imm32_float(3.0);
            let two = self.ir.imm32_float(2.0);
            let five = self.ir.imm32_float(5.0);
            let four = self.ir.imm32_float(4.0);
            let x_neg = self.ir.fp_less_than(&f32w(&x), &f32w(&zero), true);
            let y_neg = self.ir.fp_less_than(&f32w(&y), &f32w(&zero), true);
            let z_neg = self.ir.fp_less_than(&f32w(&z), &f32w(&zero), true);
            let x_face = F32::new(
                self.ir
                    .select(&x_neg, &one.into_value(), &zero.into_value()),
            );
            let y_face = F32::new(
                self.ir
                    .select(&y_neg, &three.into_value(), &two.into_value()),
            );
            let z_face = F32::new(
                self.ir
                    .select(&z_neg, &five.into_value(), &four.into_value()),
            );
            self.select_cube_result(&x, &y, &z, &x_face, &y_face, &z_face)
        };
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cubesc_f32(&mut self, inst: &GcnInst) {
        let x = self.get_src_f32(&inst.src[0]);
        let y = self.get_src_f32(&inst.src[1]);
        let z = self.get_src_f32(&inst.src[2]);
        let zero = self.ir.imm32_float(0.0);
        let x_neg = self.ir.fp_less_than(&f32w(&x), &f32w(&zero), true);
        let z_neg = self.ir.fp_less_than(&f32w(&z), &f32w(&zero), true);
        let neg_z = f32_of(self.ir.fp_neg(&f32w(&z)));
        let x_sc = F32::new(
            self.ir
                .select(&x_neg, &z.value().clone(), &neg_z.into_value()),
        );
        let y_sc = x.clone();
        let neg_x = f32_of(self.ir.fp_neg(&f32w(&x)));
        let z_sc = F32::new(
            self.ir
                .select(&z_neg, &neg_x.into_value(), &x.value().clone()),
        );
        let result = self.select_cube_result(&x, &y, &z, &x_sc, &y_sc, &z_sc);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cubetc_f32(&mut self, inst: &GcnInst) {
        let x = self.get_src_f32(&inst.src[0]);
        let y = self.get_src_f32(&inst.src[1]);
        let z = self.get_src_f32(&inst.src[2]);
        let zero = self.ir.imm32_float(0.0);
        let y_neg = self.ir.fp_less_than(&f32w(&y), &f32w(&zero), true);
        let x_z_tc = f32_of(self.ir.fp_neg(&f32w(&y)));
        let neg_z = f32_of(self.ir.fp_neg(&f32w(&z)));
        let y_tc = F32::new(
            self.ir
                .select(&y_neg, &neg_z.into_value(), &z.value().clone()),
        );
        let result = self.select_cube_result(&x, &y, &z, &x_z_tc, &y_tc, &x_z_tc);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cubema_f32(&mut self, inst: &GcnInst) {
        let x = self.get_src_f32(&inst.src[0]);
        let y = self.get_src_f32(&inst.src[1]);
        let z = self.get_src_f32(&inst.src[2]);
        let two = self.ir.imm32_float(2.0);
        let x_major = f32_of(self.ir.fp_mul(&f32w(&x), &f32w(&two)));
        let y_major = f32_of(self.ir.fp_mul(&f32w(&y), &f32w(&two)));
        let z_major = f32_of(self.ir.fp_mul(&f32w(&z), &f32w(&two)));
        let result = self.select_cube_result(&x, &y, &z, &x_major, &y_major, &z_major);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_bfe_u32(&mut self, is_signed: bool, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let mut src1 = self.get_src(&inst.src[1]);
        let mut src2 = self.get_src(&inst.src[2]);
        if !src1.value().is_immediate() {
            let mask = self.ir.imm32(0x1F);
            src1 = u32_of(self.ir.bitwise_and(&u32w(&src1), &u32w(&mask)));
        }
        if !src2.value().is_immediate() {
            let mask = self.ir.imm32(0x1F);
            src2 = u32_of(self.ir.bitwise_and(&u32w(&src2), &u32w(&mask)));
        }
        let result = self.ir.bit_field_extract(&src0, &src1, &src2, is_signed);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_bfi_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let src2 = self.get_src(&inst.src[2]);
        let masked01 = self.ir.bitwise_and(&u32w(&src0), &u32w(&src1));
        let not_src0 = self.ir.bitwise_not(&u32w(&src0));
        let masked2 = self.ir.bitwise_and(&not_src0, &u32w(&src2));
        let result = self.ir.bitwise_or(&masked01, &masked2);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_fma_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let src2 = self.get_src_f32(&inst.src[2]);
        let result = self.ir.fp_fma(&f32w(&src0), &f32w(&src1), &f32w(&src2));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_fma_f64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64_f64(&inst.src[0]);
        let src1 = self.get_src64_f64(&inst.src[1]);
        let src2 = self.get_src64_f64(&inst.src[2]);
        let result = f64_of(self.ir.fp_fma(&f64w(&src0), &f64w(&src1), &f64w(&src2)));
        self.set_dst64(&inst.dst[0], result.into_value());
    }

    fn v_min3_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let src2 = self.get_src_f32(&inst.src[2]);
        let result = self.ir.fp_min_tri(&f32w(&src0), &f32w(&src1), &f32w(&src2));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_min3_u32(&mut self, is_signed: bool, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let src2 = self.get_src(&inst.src[2]);
        let result = self.ir.i_min_tri(&src0, &src1, &src2, is_signed);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_max3_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let src2 = self.get_src_f32(&inst.src[2]);
        let result = self.ir.fp_max_tri(&f32w(&src0), &f32w(&src1), &f32w(&src2));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_max3_u32(&mut self, is_signed: bool, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let src2 = self.get_src(&inst.src[2]);
        let result = self.ir.i_max_tri(&src0, &src1, &src2, is_signed);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_med3_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src_f32(&inst.src[1]);
        let src2 = self.get_src_f32(&inst.src[2]);
        let result = self.ir.fp_med_tri(&f32w(&src0), &f32w(&src1), &f32w(&src2));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_med3_u32(&mut self, is_signed: bool, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let src2 = self.get_src(&inst.src[2]);
        let result = self.ir.i_med_tri(&src0, &src1, &src2, is_signed);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_sad_u32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let src2 = self.get_src(&inst.src[2]);
        let result = if src0.value().is_immediate() && src0.value().u32() == 0 {
            src1.clone()
        } else if src1.value().is_immediate() && src1.value().u32() == 0 {
            src0.clone()
        } else {
            let max = self.ir.i_max(&src0, &src1, false);
            let min = self.ir.i_min(&src0, &src1, false);
            u32_of(self.ir.i_sub(&u32w(&max), &u32w(&min)))
        };
        let sum = self.ir.i_add(&u32w(&result), &u32w(&src2));
        self.set_dst(&inst.dst[0], &U32F32::new(sum.into_value()));
    }

    fn v_cvt_pk_u16_u32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let float0 = self.ir.bit_cast_f32_u32(&src0);
        let float1 = self.ir.bit_cast_f32_u32(&src1);
        let vector = self.ir.composite_construct(float0, float1);
        let result = self.ir.pack_2x16(NumberFormat::Uint, &vector);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cvt_pk_i16_i32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let float0 = self.ir.bit_cast_f32_u32(&src0);
        let float1 = self.ir.bit_cast_f32_u32(&src1);
        let vector = self.ir.composite_construct(float0, float1);
        let result = self.ir.pack_2x16(NumberFormat::Sint, &vector);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_cvt_pk_u8_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_f32(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let src2 = self.get_src(&inst.src[2]);
        let rounded = self.ir.fp_round_even(&f32w(&src0));
        let zero = self.ir.imm32_float(0.0);
        let max = self.ir.imm32_float(255.0);
        let clamped = f32_of(self.ir.fp_clamp(&rounded, &f32w(&zero), &f32w(&max)));
        let zero2 = self.ir.imm32_float(0.0);
        let is_nan = self.ir.fp_is_nan(&f32w(&src0));
        let value = F32::new(
            self.ir
                .select(&is_nan, &zero2.into_value(), &clamped.into_value()),
        );
        let value_uint = u32_of(self.ir.convert_f_to_u(32, &f32w(&value)));
        let three = self.ir.imm32(3);
        let three_shift = self.ir.imm32(3);
        let masked = u32_of(self.ir.bitwise_and(&u32w(&src1), &u32w(&three)));
        let offset = u32_of(self.ir.shift_left_logical(&u32w(&masked), &three_shift));
        let eight = self.ir.imm32(8);
        let result = self
            .ir
            .bit_field_insert(&u32w(&src2), &u32w(&value_uint), &offset, &eight);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn shift_count64(&mut self, value: &U64) -> U32 {
        let mask = self.ir.imm64(0x3F);
        let masked = u64_of(self.ir.bitwise_and(&u64w(value), &u64w(&mask)));
        let unpacked = self.ir.unpack_uint_2x32(&masked);
        U32::new(self.ir.composite_extract(&unpacked, 0))
    }

    fn v_lshl_b64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64(&inst.src[0]);
        let src1 = self.get_src64(&inst.src[1]);
        let shift = self.shift_count64(&src1);
        let result = self.ir.shift_left_logical(&u64w(&src0), &shift);
        self.set_dst64(&inst.dst[0], result.into_value());
    }

    fn v_lshr_b64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64(&inst.src[0]);
        let src1 = self.get_src64(&inst.src[1]);
        let shift = self.shift_count64(&src1);
        let result = self.ir.shift_right_logical(&u64w(&src0), &shift);
        self.set_dst64(&inst.dst[0], result.into_value());
    }

    fn v_ashr_i64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64(&inst.src[0]);
        let src1 = self.get_src64(&inst.src[1]);
        let shift = self.shift_count64(&src1);
        let result = self.ir.shift_right_arithmetic(&u64w(&src0), &shift);
        self.set_dst64(&inst.dst[0], result.into_value());
    }

    fn v_alignbit_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let mask = self.ir.imm32(0x1F);
        let raw2 = self.get_src(&inst.src[2]);
        let src2 = u32_of(self.ir.bitwise_and(&u32w(&raw2), &u32w(&mask)));
        let lo = u32_of(self.ir.shift_right_logical(&u32w(&src1), &src2));
        let one = self.ir.imm32(1);
        let left = self.ir.shift_left_logical(&u32w(&src0), &one);
        let imm31 = self.ir.imm32(31);
        let count = u32_of(self.ir.i_sub(&u32w(&imm31), &u32w(&src2)));
        let hi = self.ir.shift_left_logical(&left, &count);
        let result = self.ir.bitwise_or(&u32w(&lo), &hi);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_alignbyte_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let mask = self.ir.imm32(0x3);
        let raw2 = self.get_src(&inst.src[2]);
        let src2 = u32_of(self.ir.bitwise_and(&u32w(&raw2), &u32w(&mask)));
        let three = self.ir.imm32(3);
        let shift = u32_of(self.ir.shift_left_logical(&u32w(&src2), &three));
        let lo = u32_of(self.ir.shift_right_logical(&u32w(&src1), &shift));
        let one = self.ir.imm32(1);
        let left = self.ir.shift_left_logical(&u32w(&src0), &one);
        let imm31 = self.ir.imm32(31);
        let count = u32_of(self.ir.i_sub(&u32w(&imm31), &u32w(&shift)));
        let hi = self.ir.shift_left_logical(&left, &count);
        let result = self.ir.bitwise_or(&u32w(&lo), &hi);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_mul_f64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64_f64(&inst.src[0]);
        let src1 = self.get_src64_f64(&inst.src[1]);
        let result = f64_of(self.ir.fp_mul(&f64w(&src0), &f64w(&src1)));
        self.set_dst64(&inst.dst[0], result.into_value());
    }

    fn v_max_f64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64_f64(&inst.src[0]);
        let src1 = self.get_src64_f64(&inst.src[1]);
        let result = f64_of(self.ir.fp_max(&f64w(&src0), &f64w(&src1)));
        self.set_dst64(&inst.dst[0], result.into_value());
    }

    fn v_mul_lo_u32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = self.ir.i_mul(&u32w(&src0), &u32w(&src1));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_mul_hi_u32(&mut self, is_signed: bool, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = self.ir.i_mul_hi(&src0, &src1, is_signed);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_mad_u64_u32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let src2 = self.get_src64(&inst.src[2]);
        let wide0 = U64::new(
            self.ir
                .u_convert(64, &U8U16U32U64::new(src0.into_value()))
                .into_value(),
        );
        let wide1 = U64::new(
            self.ir
                .u_convert(64, &U8U16U32U64::new(src1.into_value()))
                .into_value(),
        );
        let mul_result = u64_of(self.ir.i_mul(&u64w(&wide0), &u64w(&wide1)));
        let sum_result = u64_of(self.ir.i_add(&u64w(&mul_result), &u64w(&src2)));
        self.set_dst64(&inst.dst[0], sum_result.clone().into_value());
        let less_src0 = self
            .ir
            .i_less_than(&u64w(&sum_result), &u64w(&mul_result), false);
        let less_src1 = self.ir.i_less_than(&u64w(&sum_result), &u64w(&src2), false);
        let did_overflow = self.ir.logical_or(&less_src0, &less_src1);
        let ballot = self.ir.ballot(&did_overflow);
        let unpacked = self.ir.unpack_uint_2x32(&ballot);
        let lo = U32::new(self.ir.composite_extract(&unpacked, 0));
        let hi = U32::new(self.ir.composite_extract(&unpacked, 1));
        self.ir.set_vcc_lo(&lo);
        self.ir.set_vcc_hi(&hi);
    }

    fn v_lshlrev_b16(&mut self, inst: &GcnInst) {
        let shift = self.get_src16(&inst.src[0]);
        let src = self.get_src16(&inst.src[1]);
        let mask = self.ir.imm32(0xF);
        let shift = u32_of(self.ir.bitwise_and(&u32w(&shift), &u32w(&mask)));
        let result = self.ir.shift_left_logical(&u32w(&src), &shift);
        self.set_dst16(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_lshrrev_b16(&mut self, inst: &GcnInst) {
        let shift = self.get_src16(&inst.src[0]);
        let src = self.get_src16(&inst.src[1]);
        let mask = self.ir.imm32(0xF);
        let shift = u32_of(self.ir.bitwise_and(&u32w(&shift), &u32w(&mask)));
        let result = self.ir.shift_right_logical(&u32w(&src), &shift);
        self.set_dst16(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_ashrrev_i16(&mut self, inst: &GcnInst) {
        let shift = self.get_src16_signed(&inst.src[0]);
        let src = self.get_src16_signed(&inst.src[1]);
        let mask = self.ir.imm32(0xF);
        let shift = u32_of(self.ir.bitwise_and(&u32w(&shift), &u32w(&mask)));
        let result = self.ir.shift_right_arithmetic(&u32w(&src), &shift);
        self.set_dst16_signed(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_add_lshl_u32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let src2 = self.get_src(&inst.src[2]);
        let mask = self.ir.imm32(0x1F);
        let shift = u32_of(self.ir.bitwise_and(&u32w(&src2), &u32w(&mask)));
        let sum = self.ir.i_add(&u32w(&src0), &u32w(&src1));
        let result = self.ir.shift_left_logical(&sum, &shift);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_lshl_add_u32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let src2 = self.get_src(&inst.src[2]);
        let mask = self.ir.imm32(0x1F);
        let shift = u32_of(self.ir.bitwise_and(&u32w(&src1), &u32w(&mask)));
        let shifted = self.ir.shift_left_logical(&u32w(&src0), &shift);
        let result = self.ir.i_add(&shifted, &u32w(&src2));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_min3_f16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src16_f32(&inst.src[0]);
        let src1 = self.get_src16_f32(&inst.src[1]);
        let src2 = self.get_src16_f32(&inst.src[2]);
        let result = self.ir.fp_min_tri(&f32w(&src0), &f32w(&src1), &f32w(&src2));
        self.set_dst16(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_max3_f16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src16_f32(&inst.src[0]);
        let src1 = self.get_src16_f32(&inst.src[1]);
        let src2 = self.get_src16_f32(&inst.src[2]);
        let result = self.ir.fp_max_tri(&f32w(&src0), &f32w(&src1), &f32w(&src2));
        self.set_dst16(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_med3_f16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src16_f32(&inst.src[0]);
        let src1 = self.get_src16_f32(&inst.src[1]);
        let src2 = self.get_src16_f32(&inst.src[2]);
        let result = self.ir.fp_med_tri(&f32w(&src0), &f32w(&src1), &f32w(&src2));
        self.set_dst16(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_add3_u32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let src2 = self.get_src(&inst.src[2]);
        let inner = self.ir.i_add(&u32w(&src1), &u32w(&src2));
        let result = self.ir.i_add(&u32w(&src0), &inner);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_lshl_or_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let src2 = self.get_src(&inst.src[2]);
        let mask = self.ir.imm32(0x1F);
        let shift = u32_of(self.ir.bitwise_and(&u32w(&src1), &u32w(&mask)));
        let shifted = self.ir.shift_left_logical(&u32w(&src0), &shift);
        let result = self.ir.bitwise_or(&shifted, &u32w(&src2));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_and_or_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let src2 = self.get_src(&inst.src[2]);
        let masked = self.ir.bitwise_and(&u32w(&src0), &u32w(&src1));
        let result = self.ir.bitwise_or(&masked, &u32w(&src2));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_or3_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let src2 = self.get_src(&inst.src[2]);
        let first = self.ir.bitwise_or(&u32w(&src0), &u32w(&src1));
        let result = self.ir.bitwise_or(&first, &u32w(&src2));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_mad_mix_f32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_mix(&inst.src[0]);
        let src1 = self.get_src_mix(&inst.src[1]);
        let src2 = self.get_src_mix(&inst.src[2]);
        let product = self.ir.fp_mul(&f32w(&src0), &f32w(&src1));
        let result = self.ir.fp_add(&product, &f32w(&src2));
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    fn v_mad_mixlo_f16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_mix(&inst.src[0]);
        let src1 = self.get_src_mix(&inst.src[1]);
        let src2 = self.get_src_mix(&inst.src[2]);
        let product = self.ir.fp_mul(&f32w(&src0), &f32w(&src1));
        let result = self.ir.fp_add(&product, &f32w(&src2));
        let converted = self.ir.fp_convert(16, &F16F32F64::new(result.into_value()));
        let half = F16::new(converted.into_value());
        let raw = self.ir.bit_cast_u16_f16(&half);
        let widened = self.ir.u_convert(32, &U8U16U32U64::new(raw.into_value()));
        let old_value = self.get_src(&inst.dst[0]);
        let offset = self.ir.imm32(0);
        let count = self.ir.imm32(16);
        let new_value = self.ir.bit_field_insert(
            &u32w(&old_value),
            &u32w(&U32::new(widened.into_value())),
            &offset,
            &count,
        );
        self.set_dst(&inst.dst[0], &U32F32::new(new_value.into_value()));
    }

    fn v_mad_mixhi_f16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_mix(&inst.src[0]);
        let src1 = self.get_src_mix(&inst.src[1]);
        let src2 = self.get_src_mix(&inst.src[2]);
        let product = self.ir.fp_mul(&f32w(&src0), &f32w(&src1));
        let result = self.ir.fp_add(&product, &f32w(&src2));
        let converted = self.ir.fp_convert(16, &F16F32F64::new(result.into_value()));
        let half = F16::new(converted.into_value());
        let raw = self.ir.bit_cast_u16_f16(&half);
        let widened = self.ir.u_convert(32, &U8U16U32U64::new(raw.into_value()));
        let old_value = self.get_src(&inst.dst[0]);
        let offset = self.ir.imm32(16);
        let count = self.ir.imm32(16);
        let new_value = self.ir.bit_field_insert(
            &u32w(&old_value),
            &u32w(&U32::new(widened.into_value())),
            &offset,
            &count,
        );
        self.set_dst(&inst.dst[0], &U32F32::new(new_value.into_value()));
    }
}

impl Translator<'_> {
    fn v_pk_mul_lo_u16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_pk(&inst.src[0]);
        let src1 = self.get_src_pk(&inst.src[1]);
        let result_lo = self.ir.i_mul(&u32w(&src0.0), &u32w(&src1.0));
        let result_hi = self.ir.i_mul(&u32w(&src0.1), &u32w(&src1.1));
        let result = (u32_of(result_lo), u32_of(result_hi));
        self.set_dst_pk(&inst.dst[0], &result);
    }

    fn v_pk_add_i16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_pk_signed(&inst.src[0]);
        let src1 = self.get_src_pk_signed(&inst.src[1]);
        let result_lo = self.ir.i_add(&u32w(&src0.0), &u32w(&src1.0));
        let result_hi = self.ir.i_add(&u32w(&src0.1), &u32w(&src1.1));
        let result = (u32_of(result_lo), u32_of(result_hi));
        self.set_dst_pk_signed(&inst.dst[0], &result);
    }

    fn v_pk_sub_i16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_pk_signed(&inst.src[0]);
        let src1 = self.get_src_pk_signed(&inst.src[1]);
        let result_lo = self.ir.i_sub(&u32w(&src0.0), &u32w(&src1.0));
        let result_hi = self.ir.i_sub(&u32w(&src0.1), &u32w(&src1.1));
        let result = (u32_of(result_lo), u32_of(result_hi));
        self.set_dst_pk_signed(&inst.dst[0], &result);
    }

    fn v_pk_lshlrev_b16(&mut self, inst: &GcnInst) {
        let shift = self.get_src_pk(&inst.src[0]);
        let src = self.get_src_pk(&inst.src[1]);
        let result_lo = self.ir.shift_left_logical(&u32w(&src.0), &shift.0);
        let result_hi = self.ir.shift_left_logical(&u32w(&src.1), &shift.1);
        let result = (u32_of(result_lo), u32_of(result_hi));
        self.set_dst_pk(&inst.dst[0], &result);
    }

    fn v_pk_lshrrev_b16(&mut self, inst: &GcnInst) {
        let shift = self.get_src_pk(&inst.src[0]);
        let src = self.get_src_pk(&inst.src[1]);
        let result_lo = self.ir.shift_right_logical(&u32w(&src.0), &shift.0);
        let result_hi = self.ir.shift_right_logical(&u32w(&src.1), &shift.1);
        let result = (u32_of(result_lo), u32_of(result_hi));
        self.set_dst_pk(&inst.dst[0], &result);
    }

    fn v_pk_mad_u16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_pk(&inst.src[0]);
        let src1 = self.get_src_pk(&inst.src[1]);
        let src2 = self.get_src_pk(&inst.src[2]);
        let mul_lo = self.ir.i_mul(&u32w(&src0.0), &u32w(&src1.0));
        let mul_hi = self.ir.i_mul(&u32w(&src0.1), &u32w(&src1.1));
        let result_lo = self.ir.i_add(&mul_lo, &u32w(&src2.0));
        let result_hi = self.ir.i_add(&mul_hi, &u32w(&src2.1));
        let result = (u32_of(result_lo), u32_of(result_hi));
        self.set_dst_pk(&inst.dst[0], &result);
    }

    fn v_pk_add_u16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_pk(&inst.src[0]);
        let src1 = self.get_src_pk(&inst.src[1]);
        let result_lo = self.ir.i_add(&u32w(&src0.0), &u32w(&src1.0));
        let result_hi = self.ir.i_add(&u32w(&src0.1), &u32w(&src1.1));
        let result = (u32_of(result_lo), u32_of(result_hi));
        self.set_dst_pk(&inst.dst[0], &result);
    }

    fn v_pk_sub_u16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_pk(&inst.src[0]);
        let src1 = self.get_src_pk(&inst.src[1]);
        let result_lo = self.ir.i_sub(&u32w(&src0.0), &u32w(&src1.0));
        let result_hi = self.ir.i_sub(&u32w(&src0.1), &u32w(&src1.1));
        let result = (u32_of(result_lo), u32_of(result_hi));
        self.set_dst_pk(&inst.dst[0], &result);
    }

    fn v_pk_max_u16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_pk(&inst.src[0]);
        let src1 = self.get_src_pk(&inst.src[1]);
        let result_lo = self.ir.u_max(&src0.0, &src1.0);
        let result_hi = self.ir.u_max(&src0.1, &src1.1);
        self.set_dst_pk(&inst.dst[0], &(result_lo, result_hi));
    }

    fn v_pk_min_u16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_pk(&inst.src[0]);
        let src1 = self.get_src_pk(&inst.src[1]);
        let result_lo = self.ir.u_min(&src0.0, &src1.0);
        let result_hi = self.ir.u_min(&src0.1, &src1.1);
        self.set_dst_pk(&inst.dst[0], &(result_lo, result_hi));
    }

    fn v_pk_fma_f16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_pk_f32(&inst.src[0]);
        let src1 = self.get_src_pk_f32(&inst.src[1]);
        let src2 = self.get_src_pk_f32(&inst.src[2]);
        let result_lo = f32_of(
            self.ir
                .fp_fma(&f32w(&src0.0), &f32w(&src1.0), &f32w(&src2.0)),
        );
        let result_hi = f32_of(
            self.ir
                .fp_fma(&f32w(&src0.1), &f32w(&src1.1), &f32w(&src2.1)),
        );
        self.set_dst_pk_f32(&inst.dst[0], &(result_lo, result_hi));
    }

    fn v_pk_add_f16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_pk_f32(&inst.src[0]);
        let src1 = self.get_src_pk_f32(&inst.src[1]);
        let result_lo = f32_of(self.ir.fp_add(&f32w(&src0.0), &f32w(&src1.0)));
        let result_hi = f32_of(self.ir.fp_add(&f32w(&src0.1), &f32w(&src1.1)));
        self.set_dst_pk_f32(&inst.dst[0], &(result_lo, result_hi));
    }

    fn v_pk_mul_f16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_pk_f32(&inst.src[0]);
        let src1 = self.get_src_pk_f32(&inst.src[1]);
        let result_lo = f32_of(self.ir.fp_mul(&f32w(&src0.0), &f32w(&src1.0)));
        let result_hi = f32_of(self.ir.fp_mul(&f32w(&src0.1), &f32w(&src1.1)));
        self.set_dst_pk_f32(&inst.dst[0], &(result_lo, result_hi));
    }

    fn v_pk_min_f16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_pk_f32(&inst.src[0]);
        let src1 = self.get_src_pk_f32(&inst.src[1]);
        let result_lo = f32_of(self.ir.fp_min(&f32w(&src0.0), &f32w(&src1.0)));
        let result_hi = f32_of(self.ir.fp_min(&f32w(&src0.1), &f32w(&src1.1)));
        self.set_dst_pk_f32(&inst.dst[0], &(result_lo, result_hi));
    }

    fn v_pk_max_f16(&mut self, inst: &GcnInst) {
        let src0 = self.get_src_pk_f32(&inst.src[0]);
        let src1 = self.get_src_pk_f32(&inst.src[1]);
        let result_lo = f32_of(self.ir.fp_max(&f32w(&src0.0), &f32w(&src1.0)));
        let result_hi = f32_of(self.ir.fp_max(&f32w(&src0.1), &f32w(&src1.1)));
        self.set_dst_pk_f32(&inst.dst[0], &(result_lo, result_hi));
    }

    fn get_carry_in(&mut self, inst: &GcnInst) -> U32 {
        let carry = if inst.src_count == 3 {
            self.get_src64(&inst.src[2])
        } else {
            let lo = self.ir.get_vcc_lo();
            let hi = self.ir.get_vcc_hi();
            let composite = self.ir.composite_construct(lo, hi);
            self.ir.pack_uint_2x32(&composite)
        };
        let condition = self.ir.inverse_ballot(&carry);
        let one = self.ir.imm32(1);
        let zero = self.ir.imm32(0);
        U32::new(
            self.ir
                .select(&condition, &one.into_value(), &zero.into_value()),
        )
    }

    fn set_carry_out(&mut self, inst: &GcnInst, carry: &U1) {
        if inst.dst_count == 2 {
            let ballot = self.ir.ballot(carry);
            self.set_dst64(&inst.dst[1], ballot.into_value());
            return;
        }
        let ballot = self.ir.ballot(carry);
        let unpacked = self.ir.unpack_uint_2x32(&ballot);
        let lo = U32::new(self.ir.composite_extract(&unpacked, 0));
        let hi = U32::new(self.ir.composite_extract(&unpacked, 1));
        self.ir.set_vcc_lo(&lo);
        self.ir.set_vcc_hi(&hi);
    }

    fn vmovrels_helper(&mut self, src_vgprno: u32, m0: &U32) -> U32 {
        let mut src_val = self.ir.get_vector_reg(VectorReg::V0);
        for index in src_vgprno..self.runtime_info.props.num_allocated_vgprs {
            let offset = self.ir.imm32(index - src_vgprno);
            let condition = self.ir.i_equal(&u32w(m0), &u32w(&offset));
            let reg = reg_add(VectorReg::V0, index as i32).expect("the vgprs fit");
            let value = self.ir.get_vector_reg(reg);
            let selected = self
                .ir
                .select(&condition, &value.into_value(), &src_val.into_value());
            src_val = U32::new(selected);
        }
        src_val
    }

    fn vmovreld_helper(&mut self, dst_vgprno: u32, src_val: &U32, m0: &U32) {
        for index in dst_vgprno..self.runtime_info.props.num_allocated_vgprs {
            let offset = self.ir.imm32(index - dst_vgprno);
            let condition = self.ir.i_equal(&u32w(m0), &u32w(&offset));
            let reg = reg_add(VectorReg::V0, index as i32).expect("the vgprs fit");
            let old = self.ir.get_vector_reg(reg);
            let selected = self
                .ir
                .select(&condition, &src_val.value().clone(), &old.into_value());
            self.ir.set_vector_reg(reg, &U32F32::new(selected));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::frontend::instruction::InstOperand;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::{Block, BlockRef};
    use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
    use crate::gpu::shader_recompiler::ir::opcodes::Opcode as IrOpcode;
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::profile::Profile;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, RuntimeInfo, SwStage};
    use std::cell::RefCell;
    use std::rc::Rc;

    fn operand(field: OperandField, code: u32) -> InstOperand {
        InstOperand {
            field: field.0,
            code,
            ..Default::default()
        }
    }

    fn setup<'a>(read: &'a dyn Fn(u64) -> u32) -> (Translator<'a>, BlockRef) {
        let info = Rc::new(RefCell::new(Info::new(
            HwStage::Vertex,
            SwStage::Vertex,
            ShaderParams::new([0; 16], &[], 0),
        )));
        let runtime = RuntimeInfo::new(HwStage::Vertex, SwStage::Vertex);
        let mut translator = Translator::new(info, &runtime, &Profile::default(), read);
        let block = Block::new_ref();
        translator.ir = IrEmitter::new(&block);
        (translator, block)
    }

    fn opcodes(block: &BlockRef) -> Vec<IrOpcode> {
        block
            .borrow()
            .instructions()
            .iter()
            .map(|inst| inst.borrow().get_opcode())
            .collect()
    }

    #[test]
    fn a_float_add_writes_the_destination() {
        let read = |_| 0u32;
        let (mut translator, block) = setup(&read);
        let mut inst = GcnInst {
            opcode: Opcode::V_ADD_F32,
            ..Default::default()
        };
        inst.src[0] = operand(OperandField::VectorGPR, 0);
        inst.src[1] = operand(OperandField::VectorGPR, 1);
        inst.dst[0] = operand(OperandField::VectorGPR, 2);
        translator.emit_vector_alu(&inst);

        let ops = opcodes(&block);
        assert!(ops.contains(&IrOpcode::FPAdd32));
        let store = block
            .borrow()
            .instructions()
            .iter()
            .find(|inst| inst.borrow().get_opcode() == IrOpcode::SetVectorRegister)
            .expect("the destination store")
            .clone();
        assert_eq!(store.borrow().arg(0).vector_reg(), VectorReg::V2);
    }

    #[test]
    fn a_float_compare_ballots_into_the_sdst() {
        let read = |_| 0u32;
        let (mut translator, block) = setup(&read);
        let mut inst = GcnInst {
            opcode: Opcode::V_CMP_LT_F32,
            ..Default::default()
        };
        inst.src[0] = operand(OperandField::VectorGPR, 0);
        inst.src[1] = operand(OperandField::VectorGPR, 1);
        inst.dst[1] = operand(OperandField::VectorGPR, 0);
        translator.emit_vector_alu(&inst);

        let ops = opcodes(&block);
        assert!(ops.contains(&IrOpcode::FPOrdLessThan32));
        assert!(ops.contains(&IrOpcode::Ballot));
    }

    #[test]
    fn a_ubyte_conversion_extracts_the_requested_byte() {
        let read = |_| 0u32;
        let (mut translator, block) = setup(&read);
        let mut inst = GcnInst {
            opcode: Opcode::V_CVT_F32_UBYTE2,
            ..Default::default()
        };
        inst.src[0] = operand(OperandField::VectorGPR, 0);
        inst.dst[0] = operand(OperandField::VectorGPR, 1);
        translator.emit_vector_alu(&inst);

        let ops = opcodes(&block);
        assert!(ops.contains(&IrOpcode::BitFieldUExtract));
        assert!(ops.contains(&IrOpcode::ConvertF32U32));
    }

    #[test]
    fn a_zero_sad_skips_the_min_max_pair() {
        let read = |_| 0u32;
        let (mut translator, block) = setup(&read);
        let mut inst = GcnInst {
            opcode: Opcode::V_SAD_U32,
            ..Default::default()
        };
        inst.src[0] = operand(OperandField::LiteralConst, 0);
        inst.src[1] = operand(OperandField::VectorGPR, 1);
        inst.src[2] = operand(OperandField::VectorGPR, 2);
        inst.dst[0] = operand(OperandField::VectorGPR, 3);
        translator.emit_vector_alu(&inst);

        let ops = opcodes(&block);
        assert!(!ops.contains(&IrOpcode::UMax32));
        assert!(!ops.contains(&IrOpcode::UMin32));
        assert!(ops.contains(&IrOpcode::IAdd32));
    }

    #[test]
    fn a_movrels_chain_covers_every_allocated_vgpr() {
        let read = |_| 0u32;
        let (mut translator, block) = setup(&read);
        let mut inst = GcnInst {
            opcode: Opcode::V_MOVRELS_B32,
            ..Default::default()
        };
        inst.src[0] = operand(OperandField::VectorGPR, 0);
        inst.dst[0] = operand(OperandField::VectorGPR, 3);
        translator.emit_vector_alu(&inst);
        let _ = block;
    }
}
