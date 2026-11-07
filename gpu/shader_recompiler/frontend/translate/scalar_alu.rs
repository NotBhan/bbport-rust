//! Scalar ALU translation, ported from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/translate/scalar_alu.cpp`.

use super::translator::{ConditionOp, NegateMode, Translator};
use crate::gpu::shader_recompiler::frontend::instruction::GcnInst;
use crate::gpu::shader_recompiler::frontend::opcodes::{InstEncoding, Opcode};
use crate::gpu::shader_recompiler::ir::value::{U1, U32, U32F32, U32U64, U64, U8U16U32U64};

fn wide32(value: &U32) -> U32U64 {
    U32U64::new(value.value().clone())
}

fn wide64(value: &U64) -> U32U64 {
    U32U64::new(value.value().clone())
}

fn u32_of(value: U32U64) -> U32 {
    U32::new(value.into_value())
}

fn u64_of(value: U32U64) -> U64 {
    U64::new(value.into_value())
}

impl Translator<'_> {
    /// `EmitScalarAlu`.
    pub(crate) fn emit_scalar_alu(&mut self, inst: &GcnInst) {
        if inst.encoding == InstEncoding::SOPC {
            return self.emit_sopc(inst);
        }
        if inst.encoding == InstEncoding::SOPK {
            return self.emit_sopk(inst);
        }
        match inst.opcode {
            Opcode::S_ADD_U32 => self.s_add_u32(inst),
            Opcode::S_SUB_U32 => self.s_sub_u32(inst),
            Opcode::S_ADD_I32 => self.s_add_i32(inst),
            Opcode::S_SUB_I32 => self.s_sub_i32(inst),
            Opcode::S_ADDC_U32 => self.s_addc_u32(inst),
            Opcode::S_SUBB_U32 => self.s_subb_u32(inst),
            Opcode::S_MIN_I32 => self.s_min_u32(true, inst),
            Opcode::S_MIN_U32 => self.s_min_u32(false, inst),
            Opcode::S_MAX_I32 => self.s_max_u32(true, inst),
            Opcode::S_MAX_U32 => self.s_max_u32(false, inst),
            Opcode::S_CSELECT_B32 => self.s_cselect_b32(inst),
            Opcode::S_CSELECT_B64 => self.s_cselect_b64(inst),
            Opcode::S_AND_B32 => self.s_and_b32(NegateMode::None, inst),
            Opcode::S_AND_B64 => self.s_and_b64(NegateMode::None, inst),
            Opcode::S_OR_B32 => self.s_or_b32(inst),
            Opcode::S_OR_B64 => self.s_or_b64(NegateMode::None, false, inst),
            Opcode::S_XOR_B32 => self.s_xor_b32(inst),
            Opcode::S_NOT_B32 => self.s_not_b32(inst),
            Opcode::S_XOR_B64 => self.s_or_b64(NegateMode::None, true, inst),
            Opcode::S_ANDN2_B32 => self.s_and_b32(NegateMode::Src1, inst),
            Opcode::S_ANDN2_B64 => self.s_and_b64(NegateMode::Src1, inst),
            Opcode::S_ORN2_B64 => self.s_or_b64(NegateMode::Src1, false, inst),
            Opcode::S_NAND_B32 => self.s_and_b32(NegateMode::Result, inst),
            Opcode::S_NAND_B64 => self.s_and_b64(NegateMode::Result, inst),
            Opcode::S_NOR_B64 => self.s_or_b64(NegateMode::Result, false, inst),
            Opcode::S_XNOR_B64 => self.s_or_b64(NegateMode::Result, true, inst),
            Opcode::S_LSHL_B32 => self.s_lshl_b32(inst),
            Opcode::S_LSHL_B64 => self.s_lshl_b64(inst),
            Opcode::S_LSHR_B32 => self.s_lshr_b32(inst),
            Opcode::S_ASHR_I32 => self.s_ashr_i32(inst),
            Opcode::S_ASHR_I64 => self.s_ashr_i64(inst),
            Opcode::S_BFM_B32 => self.s_bfm_b32(inst),
            Opcode::S_MUL_I32 => self.s_mul_i32(inst),
            Opcode::S_BFE_I32 => self.s_bfe(inst, true),
            Opcode::S_BFE_U32 => self.s_bfe(inst, false),
            Opcode::S_ABSDIFF_I32 => self.s_absdiff_i32(inst),
            Opcode::S_MOV_B32 => self.s_mov(inst),
            Opcode::S_MOV_B64 => self.s_mov_b64(inst),
            Opcode::S_NOT_B64 => self.s_not_b64(inst),
            Opcode::S_WQM_B64 => {}
            Opcode::S_BREV_B32 => self.s_brev_b32(inst),
            Opcode::S_BCNT1_I32_B32 => self.s_bcnt1_i32_b32(inst),
            Opcode::S_BCNT1_I32_B64 => self.s_bcnt1_i32_b64(inst),
            Opcode::S_FF1_I32_B32 => self.s_ff1_i32_b32(inst),
            Opcode::S_FF1_I32_B64 => self.s_ff1_i32_b64(inst),
            Opcode::S_FLBIT_I32_B32 => self.s_flbit_i32_b32(inst),
            Opcode::S_FLBIT_I32_B64 => self.s_flbit_i32_b64(inst),
            Opcode::S_BITSET0_B32 => self.s_bitset_b32(inst, 0),
            Opcode::S_BITSET1_B32 => self.s_bitset_b32(inst, 1),
            Opcode::S_BITSET0_B64 => self.s_bitset_b64(inst, 0),
            Opcode::S_BITSET1_B64 => self.s_bitset_b64(inst, 1),
            Opcode::S_AND_SAVEEXEC_B64 => self.s_saveexec_b64(NegateMode::None, false, inst),
            Opcode::S_ORN2_SAVEEXEC_B64 => self.s_saveexec_b64(NegateMode::Src1, true, inst),
            Opcode::S_ABS_I32 => self.s_abs_i32(inst),
            _ => self.log_missing_opcode(inst),
        }
    }

    /// `EmitSOPC`.
    fn emit_sopc(&mut self, inst: &GcnInst) {
        match inst.opcode {
            Opcode::S_CMP_EQ_I32 => self.s_cmp(ConditionOp::EQ, true, inst),
            Opcode::S_CMP_LG_I32 => self.s_cmp(ConditionOp::LG, true, inst),
            Opcode::S_CMP_GT_I32 => self.s_cmp(ConditionOp::GT, true, inst),
            Opcode::S_CMP_GE_I32 => self.s_cmp(ConditionOp::GE, true, inst),
            Opcode::S_CMP_LT_I32 => self.s_cmp(ConditionOp::LT, true, inst),
            Opcode::S_CMP_LE_I32 => self.s_cmp(ConditionOp::LE, true, inst),
            Opcode::S_CMP_EQ_U32 => self.s_cmp(ConditionOp::EQ, false, inst),
            Opcode::S_CMP_LG_U32 => self.s_cmp(ConditionOp::LG, false, inst),
            Opcode::S_CMP_GT_U32 => self.s_cmp(ConditionOp::GT, false, inst),
            Opcode::S_CMP_GE_U32 => self.s_cmp(ConditionOp::GE, false, inst),
            Opcode::S_CMP_LT_U32 => self.s_cmp(ConditionOp::LT, false, inst),
            Opcode::S_CMP_LE_U32 => self.s_cmp(ConditionOp::LE, false, inst),
            Opcode::S_BITCMP0_B32 => self.s_bitcmp(false, 32, inst),
            Opcode::S_BITCMP1_B32 => self.s_bitcmp(true, 32, inst),
            Opcode::S_BITCMP0_B64 => self.s_bitcmp(false, 64, inst),
            Opcode::S_BITCMP1_B64 => self.s_bitcmp(true, 64, inst),
            _ => self.log_missing_opcode(inst),
        }
    }

    /// `EmitSOPK`.
    fn emit_sopk(&mut self, inst: &GcnInst) {
        match inst.opcode {
            Opcode::S_MOVK_I32 => self.s_movk(inst, false),
            Opcode::S_CMOVK_I32 => self.s_movk(inst, true),
            Opcode::S_CMPK_EQ_I32 => self.s_cmpk(ConditionOp::EQ, true, inst),
            Opcode::S_CMPK_LG_I32 => self.s_cmpk(ConditionOp::LG, true, inst),
            Opcode::S_CMPK_GT_I32 => self.s_cmpk(ConditionOp::GT, true, inst),
            Opcode::S_CMPK_GE_I32 => self.s_cmpk(ConditionOp::GE, true, inst),
            Opcode::S_CMPK_LT_I32 => self.s_cmpk(ConditionOp::LT, true, inst),
            Opcode::S_CMPK_LE_I32 => self.s_cmpk(ConditionOp::LE, true, inst),
            Opcode::S_CMPK_EQ_U32 => self.s_cmpk(ConditionOp::EQ, false, inst),
            Opcode::S_CMPK_LG_U32 => self.s_cmpk(ConditionOp::LG, false, inst),
            Opcode::S_CMPK_GT_U32 => self.s_cmpk(ConditionOp::GT, false, inst),
            Opcode::S_CMPK_GE_U32 => self.s_cmpk(ConditionOp::GE, false, inst),
            Opcode::S_CMPK_LT_U32 => self.s_cmpk(ConditionOp::LT, false, inst),
            Opcode::S_CMPK_LE_U32 => self.s_cmpk(ConditionOp::LE, false, inst),
            Opcode::S_ADDK_I32 => self.s_addk_i32(inst),
            Opcode::S_MULK_I32 => self.s_mulk_i32(inst),
            _ => self.log_missing_opcode(inst),
        }
    }

    fn s_add_u32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = u32_of(self.ir.i_add(&wide32(&src0), &wide32(&src1)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let scc = self.ir.i_less_than(&wide32(&result), &wide32(&src0), false);
        self.ir.set_scc(&scc);
    }

    fn s_sub_u32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = u32_of(self.ir.i_sub(&wide32(&src0), &wide32(&src1)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let scc = self
            .ir
            .i_greater_than(&wide32(&src1), &wide32(&src0), false);
        self.ir.set_scc(&scc);
    }

    fn s_subb_u32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let scc = self.ir.get_scc();
        let one = self.ir.imm32(1);
        let zero = self.ir.imm32(0);
        let borrow = U32::new(self.ir.select(&scc, &one.into_value(), &zero.into_value()));
        let difference = u32_of(self.ir.i_sub(&wide32(&src0), &wide32(&src1)));
        let result = u32_of(self.ir.i_sub(&wide32(&difference), &wide32(&borrow)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let underflow = self
            .ir
            .i_greater_than(&wide32(&src1), &wide32(&src0), false);
        let borrow_underflow =
            self.ir
                .i_greater_than(&wide32(&borrow), &wide32(&difference), false);
        let scc = self.ir.logical_or(&underflow, &borrow_underflow);
        self.ir.set_scc(&scc);
    }

    fn s_add_i32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = u32_of(self.ir.i_add(&wide32(&src0), &wide32(&src1)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let shift = self.ir.imm32(31);
        let sign0 = u32_of(self.ir.shift_right_logical(&wide32(&src0), &shift));
        let sign1 = u32_of(self.ir.shift_right_logical(&wide32(&src1), &shift));
        let signr = u32_of(self.ir.shift_right_logical(&wide32(&result), &shift));
        let same = self.ir.i_equal(&wide32(&sign0), &wide32(&sign1));
        let differs = self.ir.i_not_equal(&wide32(&sign0), &wide32(&signr));
        let scc = self.ir.logical_and(&same, &differs);
        self.ir.set_scc(&scc);
    }

    fn s_sub_i32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = u32_of(self.ir.i_sub(&wide32(&src0), &wide32(&src1)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let shift = self.ir.imm32(31);
        let sign0 = u32_of(self.ir.shift_right_logical(&wide32(&src0), &shift));
        let sign1 = u32_of(self.ir.shift_right_logical(&wide32(&src1), &shift));
        let signr = u32_of(self.ir.shift_right_logical(&wide32(&result), &shift));
        let differs = self.ir.i_not_equal(&wide32(&sign0), &wide32(&sign1));
        let changed = self.ir.i_not_equal(&wide32(&sign0), &wide32(&signr));
        let scc = self.ir.logical_and(&differs, &changed);
        self.ir.set_scc(&scc);
    }

    fn s_addc_u32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let scc = self.ir.get_scc();
        let one = self.ir.imm32(1);
        let zero = self.ir.imm32(0);
        let carry = U32::new(self.ir.select(&scc, &one.into_value(), &zero.into_value()));
        let result1 = u32_of(self.ir.i_add(&wide32(&src0), &wide32(&src1)));
        let result2 = u32_of(self.ir.i_add(&wide32(&result1), &wide32(&carry)));
        let carry_out1 = self
            .ir
            .i_less_than(&wide32(&result1), &wide32(&src0), false);
        let carry_out2 = self
            .ir
            .i_less_than(&wide32(&result2), &wide32(&result1), false);
        self.set_dst(&inst.dst[0], &U32F32::new(result2.into_value()));
        let scc = self.ir.logical_or(&carry_out1, &carry_out2);
        self.ir.set_scc(&scc);
    }

    fn s_min_u32(&mut self, is_signed: bool, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = self.ir.i_min(&src0, &src1, is_signed);
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let scc = self.ir.i_equal(&wide32(&result), &wide32(&src0));
        self.ir.set_scc(&scc);
    }

    fn s_max_u32(&mut self, is_signed: bool, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = self.ir.i_max(&src0, &src1, is_signed);
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let scc = self.ir.i_equal(&wide32(&result), &wide32(&src0));
        self.ir.set_scc(&scc);
    }

    fn s_cselect_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let scc = self.ir.get_scc();
        let result = self.ir.select(&scc, &src0.into_value(), &src1.into_value());
        self.set_dst(&inst.dst[0], &U32F32::new(result));
    }

    fn s_cselect_b64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64(&inst.src[0]);
        let src1 = self.get_src64(&inst.src[1]);
        let scc = self.ir.get_scc();
        let result = U64::new(self.ir.select(&scc, &src0.into_value(), &src1.into_value()));
        self.set_dst64(&inst.dst[0], result.value().clone());
    }

    fn s_and_b32(&mut self, negate: NegateMode, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let mut src1 = self.get_src(&inst.src[1]);
        if negate == NegateMode::Src1 {
            src1 = u32_of(self.ir.bitwise_not(&wide32(&src1)));
        }
        let mut result = u32_of(self.ir.bitwise_and(&wide32(&src0), &wide32(&src1)));
        if negate == NegateMode::Result {
            result = u32_of(self.ir.bitwise_not(&wide32(&result)));
        }
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let zero = self.ir.imm32(0);
        let scc = self.ir.i_not_equal(&wide32(&result), &wide32(&zero));
        self.ir.set_scc(&scc);
    }

    fn s_and_b64(&mut self, negate: NegateMode, inst: &GcnInst) {
        let src0 = self.get_src64(&inst.src[0]);
        let mut src1 = self.get_src64(&inst.src[1]);
        if negate == NegateMode::Src1 {
            src1 = u64_of(self.ir.bitwise_not(&wide64(&src1)));
        }
        let mut result = u64_of(self.ir.bitwise_and(&wide64(&src0), &wide64(&src1)));
        if negate == NegateMode::Result {
            result = u64_of(self.ir.bitwise_not(&wide64(&result)));
        }
        let scc = self.ir.inverse_ballot(&result);
        self.ir.set_scc(&scc);
        self.set_dst64(&inst.dst[0], result.value().clone());
    }

    fn s_or_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = u32_of(self.ir.bitwise_or(&wide32(&src0), &wide32(&src1)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let zero = self.ir.imm32(0);
        let scc = self.ir.i_not_equal(&wide32(&result), &wide32(&zero));
        self.ir.set_scc(&scc);
    }

    fn s_or_b64(&mut self, negate: NegateMode, is_xor: bool, inst: &GcnInst) {
        let src0 = self.get_src64(&inst.src[0]);
        let mut src1 = self.get_src64(&inst.src[1]);
        if negate == NegateMode::Src1 {
            src1 = u64_of(self.ir.bitwise_not(&wide64(&src1)));
        }
        let mut result = if is_xor {
            u64_of(self.ir.bitwise_xor(&wide64(&src0), &wide64(&src1)))
        } else {
            u64_of(self.ir.bitwise_or(&wide64(&src0), &wide64(&src1)))
        };
        if negate == NegateMode::Result {
            result = u64_of(self.ir.bitwise_not(&wide64(&result)));
        }
        let scc = self.ir.inverse_ballot(&result);
        self.ir.set_scc(&scc);
        self.set_dst64(&inst.dst[0], result.value().clone());
    }

    fn s_xor_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = u32_of(self.ir.bitwise_xor(&wide32(&src0), &wide32(&src1)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let zero = self.ir.imm32(0);
        let scc = self.ir.i_not_equal(&wide32(&result), &wide32(&zero));
        self.ir.set_scc(&scc);
    }

    fn s_not_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let result = u32_of(self.ir.bitwise_not(&wide32(&src0)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let zero = self.ir.imm32(0);
        let scc = self.ir.i_not_equal(&wide32(&result), &wide32(&zero));
        self.ir.set_scc(&scc);
    }

    fn s_lshl_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let mask = self.ir.imm32(0x1F);
        let shift = u32_of(self.ir.bitwise_and(&wide32(&src1), &wide32(&mask)));
        let result = u32_of(self.ir.shift_left_logical(&wide32(&src0), &shift));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let zero = self.ir.imm32(0);
        let scc = self.ir.i_not_equal(&wide32(&result), &wide32(&zero));
        self.ir.set_scc(&scc);
    }

    fn s_lshl_b64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64(&inst.src[0]);
        let src1 = self.get_src64(&inst.src[1]);
        let mask = self.ir.imm64(0x3F);
        let masked = u64_of(self.ir.bitwise_and(&wide64(&src1), &wide64(&mask)));
        let unpacked = self.ir.unpack_uint_2x32(&masked);
        let shift = U32::new(self.ir.composite_extract(&unpacked, 0));
        let result = u64_of(self.ir.shift_left_logical(&wide64(&src0), &shift));
        self.set_dst64(&inst.dst[0], result.value().clone());
        let zero = self.ir.imm64(0);
        let scc = self.ir.i_not_equal(&wide64(&result), &wide64(&zero));
        self.ir.set_scc(&scc);
    }

    fn s_lshr_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let mask = self.ir.imm32(0x1F);
        let shift = u32_of(self.ir.bitwise_and(&wide32(&src1), &wide32(&mask)));
        let result = u32_of(self.ir.shift_right_logical(&wide32(&src0), &shift));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let zero = self.ir.imm32(0);
        let scc = self.ir.i_not_equal(&wide32(&result), &wide32(&zero));
        self.ir.set_scc(&scc);
    }

    fn s_ashr_i32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let mask = self.ir.imm32(0x1F);
        let shift = u32_of(self.ir.bitwise_and(&wide32(&src1), &wide32(&mask)));
        let result = u32_of(self.ir.shift_right_arithmetic(&wide32(&src0), &shift));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let zero = self.ir.imm32(0);
        let scc = self.ir.i_not_equal(&wide32(&result), &wide32(&zero));
        self.ir.set_scc(&scc);
    }

    fn s_ashr_i64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64(&inst.src[0]);
        let src1 = self.get_src64(&inst.src[1]);
        let mask = self.ir.imm64(0x3F);
        let masked = u64_of(self.ir.bitwise_and(&wide64(&src1), &wide64(&mask)));
        let unpacked = self.ir.unpack_uint_2x32(&masked);
        let shift = U32::new(self.ir.composite_extract(&unpacked, 0));
        let result = u64_of(self.ir.shift_right_arithmetic(&wide64(&src0), &shift));
        self.set_dst64(&inst.dst[0], result.value().clone());
        let zero = self.ir.imm64(0);
        let scc = self.ir.i_not_equal(&wide64(&result), &wide64(&zero));
        self.ir.set_scc(&scc);
    }

    fn s_bfm_b32(&mut self, inst: &GcnInst) {
        let mask = self.ir.imm32(0x1F);
        let src0 = self.get_src(&inst.src[0]);
        let src0 = u32_of(self.ir.bitwise_and(&wide32(&src0), &wide32(&mask)));
        let src1 = self.get_src(&inst.src[1]);
        let src1 = u32_of(self.ir.bitwise_and(&wide32(&src1), &wide32(&mask)));
        let one = self.ir.imm32(1);
        let shifted = u32_of(self.ir.shift_left_logical(&wide32(&one), &src0));
        let one = self.ir.imm32(1);
        let mask = u32_of(self.ir.i_sub(&wide32(&shifted), &wide32(&one)));
        let result = u32_of(self.ir.shift_left_logical(&wide32(&mask), &src1));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
    }

    fn s_mul_i32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let result = u32_of(self.ir.i_mul(&wide32(&src0), &wide32(&src1)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
    }

    fn s_bfe(&mut self, inst: &GcnInst, is_signed: bool) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let mask = self.ir.imm32(0x1F);
        let offset = u32_of(self.ir.bitwise_and(&wide32(&src1), &wide32(&mask)));
        let sixteen = self.ir.imm32(16);
        let seven = self.ir.imm32(7);
        let count = self.ir.bit_field_extract(&src1, &sixteen, &seven, false);
        let result = self.ir.bit_field_extract(&src0, &offset, &count, is_signed);
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let zero = self.ir.imm32(0);
        let scc = self.ir.i_not_equal(&wide32(&result), &wide32(&zero));
        self.ir.set_scc(&scc);
    }

    fn s_absdiff_i32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let src1 = self.get_src(&inst.src[1]);
        let difference = u32_of(self.ir.i_sub(&wide32(&src0), &wide32(&src1)));
        let result = self.ir.i_abs(&difference);
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let zero = self.ir.imm32(0);
        let scc = self.ir.i_not_equal(&wide32(&result), &wide32(&zero));
        self.ir.set_scc(&scc);
    }

    fn s_movk(&mut self, inst: &GcnInst, is_conditional: bool) {
        let simm = inst.control.sopk().simm;
        let value = self.ir.imm32_signed(simm as i32);
        let value = if is_conditional {
            let old = self.get_src(&inst.dst[0]);
            let scc = self.ir.get_scc();
            U32::new(self.ir.select(&scc, &value.into_value(), &old.into_value()))
        } else {
            value
        };
        self.set_dst(&inst.dst[0], &U32F32::new(value.into_value()));
    }

    fn s_cmpk(&mut self, cond: ConditionOp, is_signed: bool, inst: &GcnInst) {
        let simm = inst.control.sopk().simm;
        let lhs = self.get_src(&inst.dst[0]);
        let rhs = self.ir.imm32_signed(simm as i32);
        let result = self.compare(cond, &lhs, &rhs, is_signed);
        self.ir.set_scc(&result);
    }

    fn s_addk_i32(&mut self, inst: &GcnInst) {
        let simm = inst.control.sopk().simm;
        let src0 = self.get_src(&inst.dst[0]);
        let src1 = self.ir.imm32_signed(simm as i32);
        let result = u32_of(self.ir.i_add(&wide32(&src0), &wide32(&src1)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let shift = self.ir.imm32(31);
        let sign0 = u32_of(self.ir.shift_right_logical(&wide32(&src0), &shift));
        let sign1 = u32_of(self.ir.shift_right_logical(&wide32(&src1), &shift));
        let signr = u32_of(self.ir.shift_right_logical(&wide32(&result), &shift));
        let same = self.ir.i_equal(&wide32(&sign0), &wide32(&sign1));
        let differs = self.ir.i_not_equal(&wide32(&sign0), &wide32(&signr));
        let scc = self.ir.logical_and(&same, &differs);
        self.ir.set_scc(&scc);
    }

    fn s_mulk_i32(&mut self, inst: &GcnInst) {
        let simm = inst.control.sopk().simm;
        let lhs = self.get_src(&inst.dst[0]);
        let rhs = self.ir.imm32_signed(simm as i32);
        let result = u32_of(self.ir.i_mul(&wide32(&lhs), &wide32(&rhs)));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
    }

    fn s_mov(&mut self, inst: &GcnInst) {
        let value = self.get_src(&inst.src[0]);
        self.set_dst(&inst.dst[0], &U32F32::new(value.into_value()));
    }

    fn s_mov_b64(&mut self, inst: &GcnInst) {
        let value = self.get_src64(&inst.src[0]);
        self.set_dst64(&inst.dst[0], value.into_value());
    }

    fn s_not_b64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64(&inst.src[0]);
        let result = u64_of(self.ir.bitwise_not(&wide64(&src0)));
        let scc = self.ir.inverse_ballot(&result);
        self.ir.set_scc(&scc);
        self.set_dst64(&inst.dst[0], result.value().clone());
    }

    fn s_brev_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let result = self.ir.bit_reverse(&src0);
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
    }

    fn s_bcnt1_i32_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let result = self.ir.bit_count(&wide32(&src0));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let zero = self.ir.imm32(0);
        let scc = self.ir.i_not_equal(&wide32(&result), &wide32(&zero));
        self.ir.set_scc(&scc);
    }

    fn s_bcnt1_i32_b64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64(&inst.src[0]);
        let result = self.ir.bit_count(&wide64(&src0));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let zero = self.ir.imm32(0);
        let scc = self.ir.i_not_equal(&wide32(&result), &wide32(&zero));
        self.ir.set_scc(&scc);
    }

    fn s_ff1_i32_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let result = self.ir.find_i_lsb(&wide32(&src0));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
    }

    fn s_ff1_i32_b64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64(&inst.src[0]);
        let result = self.ir.ballot_find_lsb(&src0);
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
    }

    fn s_flbit_i32_b32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let msb_pos = self.ir.find_u_msb(&wide32(&src0));
        let thirty_one = self.ir.imm32(31);
        let pos_from_left = u32_of(self.ir.i_sub(&wide32(&thirty_one), &wide32(&msb_pos)));
        let zero = self.ir.imm32(0);
        let cond = self.ir.i_not_equal(&wide32(&src0), &wide32(&zero));
        let all_ones = self.ir.imm32(!0u32);
        let result = U32::new(self.ir.select(
            &cond,
            &pos_from_left.into_value(),
            &all_ones.into_value(),
        ));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
    }

    fn s_flbit_i32_b64(&mut self, inst: &GcnInst) {
        let src0 = self.get_src64(&inst.src[0]);
        let msb_pos = self.ir.find_u_msb(&wide64(&src0));
        let sixty_three = self.ir.imm32(63);
        let pos_from_left = u32_of(self.ir.i_sub(&wide32(&sixty_three), &wide32(&msb_pos)));
        let zero = self.ir.imm64(0);
        let cond = self.ir.i_not_equal(&wide64(&src0), &wide64(&zero));
        let all_ones = self.ir.imm32(!0u32);
        let result = U32::new(self.ir.select(
            &cond,
            &pos_from_left.into_value(),
            &all_ones.into_value(),
        ));
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
    }

    fn s_bitset_b32(&mut self, inst: &GcnInst, bit_value: u32) {
        let old_value = self.get_src(&inst.dst[0]);
        let src0 = self.get_src(&inst.src[0]);
        let zero = self.ir.imm32(0);
        let five = self.ir.imm32(5);
        let offset = self.ir.bit_field_extract(&src0, &zero, &five, false);
        let bit = self.ir.imm32(bit_value);
        let one = self.ir.imm32(1);
        let result =
            u32_of(
                self.ir
                    .bit_field_insert(&wide32(&old_value), &wide32(&bit), &offset, &one),
            );
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
    }

    fn s_bitset_b64(&mut self, inst: &GcnInst, bit_value: u32) {
        let old_value = self.get_src64(&inst.dst[0]);
        let src0 = self.get_src(&inst.src[0]);
        let mask = self.ir.imm32(0x3F);
        let offset = u32_of(self.ir.bitwise_and(&wide32(&src0), &wide32(&mask)));
        let bit = self.ir.imm64(bit_value as u64);
        let one = self.ir.imm32(1);
        let result =
            u64_of(
                self.ir
                    .bit_field_insert(&wide64(&old_value), &wide64(&bit), &offset, &one),
            );
        self.set_dst64(&inst.dst[0], result.value().clone());
    }

    fn s_saveexec_b64(&mut self, negate: NegateMode, is_or: bool, inst: &GcnInst) {
        let exec_u1 = self.ir.get_exec();
        let mut exec = self.ir.ballot(&exec_u1);
        let src = self.get_src64(&inst.src[0]);
        self.set_dst64(&inst.dst[0], exec.value().clone());
        if negate == NegateMode::Src1 {
            exec = u64_of(self.ir.bitwise_not(&wide64(&exec)));
        }
        let mut result = if is_or {
            u64_of(self.ir.bitwise_or(&wide64(&exec), &wide64(&src)))
        } else {
            u64_of(self.ir.bitwise_and(&wide64(&exec), &wide64(&src)))
        };
        if negate == NegateMode::Result {
            result = u64_of(self.ir.bitwise_not(&wide64(&result)));
        }
        let result_u1 = self.ir.inverse_ballot(&result);
        self.ir.set_exec(&result_u1);
        self.ir.set_scc(&result_u1);
    }

    fn s_abs_i32(&mut self, inst: &GcnInst) {
        let src0 = self.get_src(&inst.src[0]);
        let result = self.ir.i_abs(&src0);
        self.set_dst(&inst.dst[0], &U32F32::new(result.value().clone()));
        let zero = self.ir.imm32(0);
        let scc = self.ir.i_not_equal(&wide32(&result), &wide32(&zero));
        self.ir.set_scc(&scc);
    }

    fn s_cmp(&mut self, cond: ConditionOp, is_signed: bool, inst: &GcnInst) {
        let lhs = self.get_src(&inst.src[0]);
        let rhs = self.get_src(&inst.src[1]);
        let result = self.compare(cond, &lhs, &rhs, is_signed);
        self.ir.set_scc(&result);
    }

    fn compare(&mut self, cond: ConditionOp, lhs: &U32, rhs: &U32, is_signed: bool) -> U1 {
        match cond {
            ConditionOp::EQ => self.ir.i_equal(&wide32(lhs), &wide32(rhs)),
            ConditionOp::LG => self.ir.i_not_equal(&wide32(lhs), &wide32(rhs)),
            ConditionOp::GT => self
                .ir
                .i_greater_than(&wide32(lhs), &wide32(rhs), is_signed),
            ConditionOp::GE => self
                .ir
                .i_greater_than_equal(&wide32(lhs), &wide32(rhs), is_signed),
            ConditionOp::LT => self.ir.i_less_than(&wide32(lhs), &wide32(rhs), is_signed),
            ConditionOp::LE => self
                .ir
                .i_less_than_equal(&wide32(lhs), &wide32(rhs), is_signed),
            _ => unreachable!("unsupported comparison"),
        }
    }

    fn s_bitcmp(&mut self, compare_mode: bool, bits: u32, inst: &GcnInst) {
        let src1 = self.get_src(&inst.src[1]);
        let mask = self.ir.imm32(if bits == 64 { 0x3f } else { 0x1f });
        let bitpos = u32_of(self.ir.bitwise_and(&wide32(&src1), &wide32(&mask)));
        let bittest = if bits == 64 {
            let src0 = self.get_src64(&inst.src[0]);
            let one = self.ir.imm64(1);
            let shifted = u64_of(self.ir.shift_right_logical(&wide64(&src0), &bitpos));
            let bit = u64_of(self.ir.bitwise_and(&wide64(&shifted), &wide64(&one)));
            U32::new(
                self.ir
                    .u_convert(32, &U8U16U32U64::new(bit.into_value()))
                    .into_value(),
            )
        } else {
            assert_eq!(bits, 32);
            let src0 = self.get_src(&inst.src[0]);
            let one = self.ir.imm32(1);
            let shifted = u32_of(self.ir.shift_right_logical(&wide32(&src0), &bitpos));
            u32_of(self.ir.bitwise_and(&wide32(&shifted), &wide32(&one)))
        };
        let expected = self.ir.imm32(u32::from(compare_mode));
        let result = self.ir.i_equal(&wide32(&bittest), &wide32(&expected));
        self.ir.set_scc(&result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::frontend::instruction::{InstControl, InstOperand};
    use crate::gpu::shader_recompiler::frontend::opcodes::OperandField;
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
    fn an_unsigned_add_sets_the_carry_flag() {
        let read = |_| 0u32;
        let (mut translator, block) = setup(&read);
        let mut inst = GcnInst {
            opcode: Opcode::S_ADD_U32,
            ..Default::default()
        };
        inst.src[0] = operand(OperandField::LiteralConst, 5);
        inst.src[1] = operand(OperandField::LiteralConst, 7);
        inst.dst[0] = operand(OperandField::ScalarGPR, 1);
        translator.s_add_u32(&inst);

        let ops = opcodes(&block);
        assert!(ops.contains(&IrOpcode::IAdd32));
        assert!(ops.contains(&IrOpcode::ULessThan32));
        assert!(ops.contains(&IrOpcode::SetScc));
        assert!(ops.contains(&IrOpcode::SetScalarRegister));
    }

    #[test]
    fn a_64_bit_shift_masks_the_count() {
        let read = |_| 0u32;
        let (mut translator, block) = setup(&read);
        let mut inst = GcnInst {
            opcode: Opcode::S_LSHL_B64,
            ..Default::default()
        };
        inst.src[0] = operand(OperandField::LiteralConst, 1);
        inst.src[1] = operand(OperandField::LiteralConst, 70);
        inst.dst[0] = operand(OperandField::ScalarGPR, 2);
        translator.s_lshl_b64(&inst);

        let ops = opcodes(&block);
        assert!(ops.contains(&IrOpcode::ShiftLeftLogical64));
        assert!(ops.contains(&IrOpcode::BitwiseAnd64));
    }

    #[test]
    fn a_single_bit_set_inserts_one_bit() {
        let read = |_| 0u32;
        let (mut translator, block) = setup(&read);
        let mut inst = GcnInst {
            opcode: Opcode::S_BITSET1_B32,
            ..Default::default()
        };
        inst.src[0] = operand(OperandField::LiteralConst, 3);
        inst.dst[0] = operand(OperandField::ScalarGPR, 2);
        translator.s_bitset_b32(&inst, 1);

        assert!(opcodes(&block).contains(&IrOpcode::BitFieldInsert32));
    }

    #[test]
    fn a_movk_immediate_is_sign_extended() {
        let read = |_| 0u32;
        let (mut translator, block) = setup(&read);
        let mut inst = GcnInst {
            opcode: Opcode::S_MOVK_I32,
            ..Default::default()
        };
        inst.control = InstControl::from_raw(-2i16 as u16 as u64);
        inst.dst[0] = operand(OperandField::ScalarGPR, 2);
        translator.s_movk(&inst, false);

        let store = block
            .borrow()
            .instructions()
            .iter()
            .find(|inst| inst.borrow().get_opcode() == IrOpcode::SetScalarRegister)
            .expect("the destination store")
            .clone();
        assert_eq!(store.borrow().arg(1).u32(), 0xFFFF_FFFE);
    }

    #[test]
    fn saveexec_updates_exec_and_scc() {
        let read = |_| 0u32;
        let (mut translator, block) = setup(&read);
        let mut inst = GcnInst {
            opcode: Opcode::S_AND_SAVEEXEC_B64,
            ..Default::default()
        };
        inst.src[0] = operand(OperandField::LiteralConst, 1);
        inst.dst[0] = operand(OperandField::ScalarGPR, 2);
        translator.s_saveexec_b64(NegateMode::None, false, &inst);

        let ops = opcodes(&block);
        assert!(ops.contains(&IrOpcode::Ballot));
        assert!(ops.contains(&IrOpcode::BitwiseAnd64));
        assert!(ops.contains(&IrOpcode::InverseBallot));
        assert!(ops.contains(&IrOpcode::SetExec));
        assert!(ops.contains(&IrOpcode::SetScc));
    }
}
