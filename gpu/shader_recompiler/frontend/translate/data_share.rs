//! Data-share translation, ported from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/translate/data_share.cpp`. The file also
//! carries the three lane instructions, which are vector operations.

use super::translator::{AtomicOp, Translator};
use crate::gpu::shader_recompiler::frontend::instruction::GcnInst;
use crate::gpu::shader_recompiler::frontend::opcodes::{Opcode, OperandField};
use crate::gpu::shader_recompiler::ir::reg::{reg_add, RegType as _, VectorReg};
use crate::gpu::shader_recompiler::ir::value::{Value, U16, U32, U32F32, U32U64, U64, U8U16U32U64};
use crate::gpu::shader_recompiler::runtime_info::HwStage;

fn wide32(value: &U32) -> U32U64 {
    U32U64::new(value.value().clone())
}

fn wide_value(value: &Value) -> U32U64 {
    U32U64::new(value.clone())
}

impl Translator<'_> {
    /// `EmitDataShare`.
    pub(crate) fn emit_data_share(&mut self, inst: &GcnInst) {
        match inst.opcode {
            Opcode::DS_ADD_U32 => self.ds_op32(inst, AtomicOp::Add, false),
            Opcode::DS_ADD_U64 => self.ds_op64(inst, AtomicOp::Add, false),
            Opcode::DS_SUB_U32 => self.ds_op32(inst, AtomicOp::Sub, false),
            Opcode::DS_INC_U32 => self.ds_op32(inst, AtomicOp::Inc, false),
            Opcode::DS_DEC_U32 => self.ds_op32(inst, AtomicOp::Dec, false),
            Opcode::DS_MIN_I32 => self.ds_op32(inst, AtomicOp::Smin, false),
            Opcode::DS_MAX_I32 => self.ds_op32(inst, AtomicOp::Smax, false),
            Opcode::DS_MIN_U32 => self.ds_op32(inst, AtomicOp::Umin, false),
            Opcode::DS_MAX_U32 => self.ds_op32(inst, AtomicOp::Umax, false),
            Opcode::DS_AND_B32 => self.ds_op32(inst, AtomicOp::And, false),
            Opcode::DS_OR_B32 => self.ds_op32(inst, AtomicOp::Or, false),
            Opcode::DS_XOR_B32 => self.ds_op32(inst, AtomicOp::Xor, false),
            Opcode::DS_WRITE_B32 => self.ds_write(32, false, false, false, inst),
            Opcode::DS_WRITE2_B32 => self.ds_write(32, false, true, false, inst),
            Opcode::DS_WRITE2ST64_B32 => self.ds_write(32, false, true, true, inst),
            Opcode::DS_ADD_RTN_U32 => self.ds_op32(inst, AtomicOp::Add, true),
            Opcode::DS_SUB_RTN_U32 => self.ds_op32(inst, AtomicOp::Sub, true),
            Opcode::DS_MIN_RTN_U32 => self.ds_op32(inst, AtomicOp::Umin, true),
            Opcode::DS_MAX_RTN_U32 => self.ds_op32(inst, AtomicOp::Umax, true),
            Opcode::DS_AND_RTN_B32 => self.ds_op32(inst, AtomicOp::And, true),
            Opcode::DS_OR_RTN_B32 => self.ds_op32(inst, AtomicOp::Or, true),
            Opcode::DS_XOR_RTN_B32 => self.ds_op32(inst, AtomicOp::Xor, true),
            Opcode::DS_CMPST_B32 => self.ds_cmpst(32, false, inst),
            Opcode::DS_CMPST_RTN_B32 => self.ds_cmpst(32, true, inst),
            Opcode::DS_CMPST_B64 => self.ds_cmpst(64, false, inst),
            Opcode::DS_CMPST_RTN_B64 => self.ds_cmpst(64, true, inst),
            Opcode::DS_SWIZZLE_B32 => self.ds_swizzle_b32(inst),
            Opcode::DS_READ_B32 => self.ds_read(32, false, false, false, inst),
            Opcode::DS_READ2_B32 => self.ds_read(32, false, true, false, inst),
            Opcode::DS_READ2ST64_B32 => self.ds_read(32, false, true, true, inst),
            Opcode::DS_READ_U16 => self.ds_read(16, false, false, false, inst),
            Opcode::DS_CONSUME => self.ds_consume(inst),
            Opcode::DS_APPEND => self.ds_append(inst),
            Opcode::DS_WRITE_B16 => self.ds_write(16, false, false, false, inst),
            Opcode::DS_WRITE_B64 => self.ds_write(64, false, false, false, inst),
            Opcode::DS_WRITE2_B64 => self.ds_write(64, false, true, false, inst),
            Opcode::DS_WRITE2ST64_B64 => self.ds_write(64, false, true, true, inst),
            Opcode::DS_READ_B64 => self.ds_read(64, false, false, false, inst),
            Opcode::DS_READ2_B64 => self.ds_read(64, false, true, false, inst),
            Opcode::DS_READ2ST64_B64 => self.ds_read(64, false, true, true, inst),
            _ => self.log_missing_opcode(inst),
        }
    }

    /// `V_READFIRSTLANE_B32`.
    #[allow(dead_code)]
    pub(crate) fn v_readfirstlane_b32(&mut self, inst: &GcnInst) {
        let value = self.get_src(&inst.src[0]);
        let result = self.ir.read_first_lane(&value);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    /// `V_READLANE_B32`.
    #[allow(dead_code)]
    pub(crate) fn v_readlane_b32(&mut self, inst: &GcnInst) {
        assert_eq!(inst.src[0].field, OperandField::VectorGPR.0);
        let value = self.get_src(&inst.src[0]);
        let lane = self.get_src(&inst.src[1]);
        let result = self.ir.read_lane(&value, &lane);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    /// `V_WRITELANE_B32`.
    #[allow(dead_code)]
    pub(crate) fn v_writelane_b32(&mut self, inst: &GcnInst) {
        let dst = VectorReg::from_raw(inst.dst[0].code);
        let value = self.get_src(&inst.src[0]);
        let lane = self.get_src(&inst.src[1]);
        let old_value = self.get_src(&inst.dst[0]);
        let result = self.ir.write_lane(&old_value, &value, &lane);
        self.ir
            .set_vector_reg(dst, &U32F32::new(result.into_value()));
    }

    /// `DS_OP` for 32-bit values.
    fn ds_op32(&mut self, inst: &GcnInst, op: AtomicOp, rtn: bool) {
        let ds = inst.control.ds();
        let is_gds = ds.gds();
        let addr = self.get_src(&inst.src[0]);
        let data = if matches!(op, AtomicOp::Inc | AtomicOp::Dec) {
            U32::new(Value::from_u32(0))
        } else {
            self.get_src(&inst.src[1])
        };
        let offset = self
            .ir
            .imm32(((ds.offset1() as u32) << 8) + ds.offset0() as u32);
        let addr_offset = U32::new(self.ir.i_add(&wide32(&addr), &wide32(&offset)).into_value());
        let original = match op {
            AtomicOp::Add => self
                .ir
                .shared_atomic_i_add(&addr_offset, &wide32(&data), is_gds),
            AtomicOp::Umin => {
                self.ir
                    .shared_atomic_i_min(&addr_offset, &wide32(&data), false, is_gds)
            }
            AtomicOp::Smin => {
                self.ir
                    .shared_atomic_i_min(&addr_offset, &wide32(&data), true, is_gds)
            }
            AtomicOp::Umax => {
                self.ir
                    .shared_atomic_i_max(&addr_offset, &wide32(&data), false, is_gds)
            }
            AtomicOp::Smax => {
                self.ir
                    .shared_atomic_i_max(&addr_offset, &wide32(&data), true, is_gds)
            }
            AtomicOp::And => self
                .ir
                .shared_atomic_and(&addr_offset, &wide32(&data), is_gds),
            AtomicOp::Or => self
                .ir
                .shared_atomic_or(&addr_offset, &wide32(&data), is_gds),
            AtomicOp::Xor => self
                .ir
                .shared_atomic_xor(&addr_offset, &wide32(&data), is_gds),
            AtomicOp::Sub => self
                .ir
                .shared_atomic_i_sub(&addr_offset, &wide32(&data), is_gds),
            AtomicOp::Inc => {
                U32U64::new(self.ir.shared_atomic_inc(&addr_offset, is_gds).into_value())
            }
            AtomicOp::Dec => {
                U32U64::new(self.ir.shared_atomic_dec(&addr_offset, is_gds).into_value())
            }
            _ => unreachable!("unsupported shared atomic"),
        };
        if rtn {
            self.set_dst(&inst.dst[0], &U32F32::new(original.into_value()));
        }
    }

    /// `DS_OP` for 64-bit values.
    fn ds_op64(&mut self, inst: &GcnInst, op: AtomicOp, rtn: bool) {
        let ds = inst.control.ds();
        let is_gds = ds.gds();
        let addr = self.get_src(&inst.src[0]);
        let data = if matches!(op, AtomicOp::Inc | AtomicOp::Dec) {
            U64::new(Value::from_u64(0))
        } else {
            self.get_src64(&inst.src[1])
        };
        let offset = self
            .ir
            .imm32(((ds.offset1() as u32) << 8) + ds.offset0() as u32);
        let addr_offset = U32::new(self.ir.i_add(&wide32(&addr), &wide32(&offset)).into_value());
        let data_wide = wide_value(data.value());
        let original = match op {
            AtomicOp::Add => self
                .ir
                .shared_atomic_i_add(&addr_offset, &data_wide, is_gds),
            AtomicOp::Sub => self
                .ir
                .shared_atomic_i_sub(&addr_offset, &data_wide, is_gds),
            AtomicOp::Inc => U32U64::new(
                self.ir
                    .shared_atomic_inc64(&addr_offset, is_gds)
                    .into_value(),
            ),
            AtomicOp::Dec => U32U64::new(
                self.ir
                    .shared_atomic_dec64(&addr_offset, is_gds)
                    .into_value(),
            ),
            _ => unreachable!("unsupported shared atomic"),
        };
        if rtn {
            self.set_dst64(&inst.dst[0], original.into_value());
        }
    }

    /// `DS_CMPST`.
    fn ds_cmpst(&mut self, bit_size: u32, rtn: bool, inst: &GcnInst) {
        let ds = inst.control.ds();
        let is_gds = ds.gds();
        let addr = self.get_src(&inst.src[0]);
        let offset = self
            .ir
            .imm32(((ds.offset1() as u32) << 8) + ds.offset0() as u32);
        let addr_offset = U32::new(self.ir.i_add(&wide32(&addr), &wide32(&offset)).into_value());
        if bit_size == 64 {
            let cmp_value = self.get_src64(&inst.src[1]);
            let write_value = self.get_src64(&inst.src[2]);
            let original = self.ir.shared_atomic_cmp_swap(
                &addr_offset,
                &wide_value(write_value.value()),
                &wide_value(cmp_value.value()),
                is_gds,
            );
            if rtn {
                self.set_dst64(&inst.dst[0], original.into_value());
            }
        } else {
            let cmp_value = self.get_src(&inst.src[1]);
            let write_value = self.get_src(&inst.src[2]);
            let original = self.ir.shared_atomic_cmp_swap(
                &addr_offset,
                &wide32(&write_value),
                &wide32(&cmp_value),
                is_gds,
            );
            if rtn {
                self.set_dst(&inst.dst[0], &U32F32::new(original.into_value()));
            }
        }
    }

    /// `DS_WRITE`.
    fn ds_write(
        &mut self,
        bit_size: u32,
        _is_signed: bool,
        is_pair: bool,
        stride64: bool,
        inst: &GcnInst,
    ) {
        let ds = inst.control.ds();
        let is_gds = ds.gds();
        let addr = self
            .ir
            .get_vector_reg(VectorReg::from_raw(inst.src[0].code));
        let data0 = inst.src[1].code;
        let data1 = inst.src[2].code;
        let offset = ((ds.offset1() as u32) << 8) + ds.offset0() as u32;
        if self.info.borrow().persistent.hw_stage == HwStage::Fragment {
            assert!(
                !is_pair && bit_size == 32 && offset.is_multiple_of(256),
                "Unexpected shared memory offset alignment: {}",
                offset
            );
            let dst = self.get_scratch_vgpr(offset);
            let value = self.ir.get_vector_reg(VectorReg::from_raw(data0));
            self.ir
                .set_vector_reg(dst, &U32F32::new(value.into_value()));
            return;
        }

        let pair_adj = (if bit_size == 32 { 4 } else { 8 }) * (if stride64 { 64 } else { 1 });
        let write = |translator: &mut Translator<'_>, vreg: u32, offset: u32| {
            let address = translator.ir.imm32(offset);
            let address = U32::new(
                translator
                    .ir
                    .i_add(&wide32(&addr), &wide32(&address))
                    .into_value(),
            );
            if bit_size == 64 {
                let lo = translator.ir.get_vector_reg(VectorReg::from_raw(vreg));
                let hi = translator.ir.get_vector_reg(VectorReg::from_raw(vreg + 1));
                let composite = translator.ir.composite_construct(lo, hi);
                let value = translator.ir.pack_uint_2x32(&composite);
                translator
                    .ir
                    .write_shared(64, &value.into_value(), &address, is_gds);
            } else if bit_size == 32 {
                let value = translator.ir.get_vector_reg(VectorReg::from_raw(vreg));
                translator
                    .ir
                    .write_shared(32, &value.into_value(), &address, is_gds);
            } else {
                let value = translator.ir.get_vector_reg(VectorReg::from_raw(vreg));
                let value = translator
                    .ir
                    .u_convert(16, &U8U16U32U64::new(value.into_value()));
                translator
                    .ir
                    .write_shared(16, &value.into_value(), &address, is_gds);
            }
        };
        if is_pair {
            write(self, data0, ds.offset0() as u32 * pair_adj);
            write(self, data1, ds.offset1() as u32 * pair_adj);
        } else {
            write(self, data0, offset);
        }
    }

    /// `DS_READ`.
    fn ds_read(
        &mut self,
        bit_size: u32,
        is_signed: bool,
        is_pair: bool,
        stride64: bool,
        inst: &GcnInst,
    ) {
        let ds = inst.control.ds();
        let is_gds = ds.gds();
        let addr = self
            .ir
            .get_vector_reg(VectorReg::from_raw(inst.src[0].code));
        let mut dst = VectorReg::from_raw(inst.dst[0].code);
        let offset = ((ds.offset1() as u32) << 8) + ds.offset0() as u32;
        if self.info.borrow().persistent.hw_stage == HwStage::Fragment {
            assert!(
                !is_pair && bit_size == 32 && offset.is_multiple_of(256),
                "Unexpected shared memory offset alignment: {}",
                offset
            );
            let source = self.get_scratch_vgpr(offset);
            let value = self.ir.get_vector_reg(source);
            self.ir
                .set_vector_reg(dst, &U32F32::new(value.into_value()));
            return;
        }

        let pair_adj = (if bit_size == 32 { 4 } else { 8 }) * (if stride64 { 64 } else { 1 });
        let load = |translator: &mut Translator<'_>, dst: &mut VectorReg, offset: u32| {
            let address = translator.ir.imm32(offset);
            let address = U32::new(
                translator
                    .ir
                    .i_add(&wide32(&addr), &wide32(&address))
                    .into_value(),
            );
            let data = translator
                .ir
                .load_shared(bit_size as i32, is_signed, &address, is_gds);
            if bit_size == 64 {
                let vector = translator.ir.unpack_uint_2x32(&U64::new(data));
                let lo = translator.ir.composite_extract(&vector, 0);
                translator.ir.set_vector_reg(*dst, &U32F32::new(lo));
                *dst = reg_add(*dst, 1).expect("the destination registers fit");
                let hi = translator.ir.composite_extract(&vector, 1);
                translator.ir.set_vector_reg(*dst, &U32F32::new(hi));
                *dst = reg_add(*dst, 1).expect("the destination registers fit");
            } else if bit_size == 32 {
                translator.ir.set_vector_reg(*dst, &U32F32::new(data));
                *dst = reg_add(*dst, 1).expect("the destination registers fit");
            } else {
                let value = translator
                    .ir
                    .u_convert(32, &U8U16U32U64::new(U16::new(data).into_value()));
                translator
                    .ir
                    .set_vector_reg(*dst, &U32F32::new(value.into_value()));
                *dst = reg_add(*dst, 1).expect("the destination registers fit");
            }
        };
        if is_pair {
            load(self, &mut dst, ds.offset0() as u32 * pair_adj);
            load(self, &mut dst, ds.offset1() as u32 * pair_adj);
        } else {
            load(self, &mut dst, offset);
        }
    }

    /// `DS_SWIZZLE_B32`.
    fn ds_swizzle_b32(&mut self, inst: &GcnInst) {
        let ds = inst.control.ds();
        let offset0 = ds.offset0() as u8;
        let offset1 = ds.offset1() as u8;
        let src = self.get_src(&inst.src[0]);
        if offset1 & 0x80 != 0 {
            if offset0 == 0x0 || offset0 == 0x55 || offset0 == 0xAA || offset0 == 0xFF {
                let index = self.ir.imm32((offset0 & 3) as u32);
                let result = self.ir.quad_broadcast(&src, &index);
                self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
            } else {
                let lane_id = self.ir.lane_id();
                let three = self.ir.imm32(3);
                let id_in_quad = U32::new(
                    self.ir
                        .bitwise_and(&wide32(&lane_id), &wide32(&three))
                        .into_value(),
                );
                let one = self.ir.imm32(1);
                let base = U32::new(
                    self.ir
                        .shift_left_logical(&wide32(&id_in_quad), &one)
                        .into_value(),
                );
                let offset = self.ir.imm32(offset0 as u32);
                let two = self.ir.imm32(2);
                let sel = self.ir.bit_field_extract(&offset, &base, &two, false);
                let not_three = self.ir.imm32(!3u32);
                let quad_base = U32::new(
                    self.ir
                        .bitwise_and(&wide32(&lane_id), &wide32(&not_three))
                        .into_value(),
                );
                let index = U32::new(
                    self.ir
                        .bitwise_or(&wide32(&quad_base), &wide32(&sel))
                        .into_value(),
                );
                let result = self.ir.shuffle(&src, &index);
                self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
            }
        } else {
            let and_mask = (offset0 & 0x1F) | 0xE0;
            let or_mask = (offset0 >> 5) | ((offset1 & 0x3) << 3);
            let xor_mask = offset1 >> 2;
            if or_mask == 0 && and_mask == 255 {
                let mask = self.ir.imm32(xor_mask as u32);
                let result = self.ir.shuffle_xor(&src, &mask);
                self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
            } else {
                let lane_id = self.ir.lane_id();
                let thirty_one = self.ir.imm32(31);
                let local = U32::new(
                    self.ir
                        .bitwise_and(&wide32(&lane_id), &wide32(&thirty_one))
                        .into_value(),
                );
                let not_thirty_one = self.ir.imm32(!31u32);
                let half = U32::new(
                    self.ir
                        .bitwise_and(&wide32(&lane_id), &wide32(&not_thirty_one))
                        .into_value(),
                );
                let and_imm = self.ir.imm32(and_mask as u32);
                let masked = U32::new(
                    self.ir
                        .bitwise_and(&wide32(&local), &wide32(&and_imm))
                        .into_value(),
                );
                let or_imm = self.ir.imm32(or_mask as u32);
                let ored = U32::new(
                    self.ir
                        .bitwise_or(&wide32(&masked), &wide32(&or_imm))
                        .into_value(),
                );
                let xor_imm = self.ir.imm32(xor_mask as u32);
                let index = U32::new(
                    self.ir
                        .bitwise_xor(&wide32(&ored), &wide32(&xor_imm))
                        .into_value(),
                );
                let index = U32::new(
                    self.ir
                        .bitwise_or(&wide32(&index), &wide32(&half))
                        .into_value(),
                );
                let result = self.ir.shuffle(&src, &index);
                self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
            }
        }
    }

    /// `DS_APPEND`.
    fn ds_append(&mut self, inst: &GcnInst) {
        let ds = inst.control.ds();
        let offset = ((ds.offset1() as u32) << 8) + ds.offset0() as u32;
        let m0 = self.ir.get_m0();
        let sixteen = self.ir.imm32(16);
        let base = self.ir.bit_field_extract(&m0, &sixteen, &sixteen, false);
        let offset = self.ir.imm32(offset);
        let gds_offset = U32::new(self.ir.i_add(&wide32(&base), &wide32(&offset)).into_value());
        let two = self.ir.imm32(2);
        let dword_offset = U32::new(
            self.ir
                .shift_right_logical(&wide32(&gds_offset), &two)
                .into_value(),
        );
        let prev = self.ir.data_append(&dword_offset);
        self.set_dst(&inst.dst[0], &U32F32::new(prev.into_value()));
    }

    /// `DS_CONSUME`.
    fn ds_consume(&mut self, inst: &GcnInst) {
        let ds = inst.control.ds();
        let offset = ((ds.offset1() as u32) << 8) + ds.offset0() as u32;
        let m0 = self.ir.get_m0();
        let sixteen = self.ir.imm32(16);
        let base = self.ir.bit_field_extract(&m0, &sixteen, &sixteen, false);
        let offset = self.ir.imm32(offset);
        let gds_offset = U32::new(self.ir.i_add(&wide32(&base), &wide32(&offset)).into_value());
        let two = self.ir.imm32(2);
        let dword_offset = U32::new(
            self.ir
                .shift_right_logical(&wide32(&gds_offset), &two)
                .into_value(),
        );
        let prev = self.ir.data_consume(&dword_offset);
        self.set_dst(&inst.dst[0], &U32F32::new(prev.into_value()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::Block;
    use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
    use crate::gpu::shader_recompiler::ir::opcodes::Opcode as IrOpcode;
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::profile::Profile;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, RuntimeInfo, SwStage};
    use std::cell::RefCell;
    use std::rc::Rc;

    fn translator<'a>(
        stage: HwStage,
        runtime: &'a RuntimeInfo,
        read: &'a dyn Fn(u64) -> u32,
    ) -> Translator<'a> {
        let info = Rc::new(RefCell::new(Info::new(
            stage,
            SwStage::Fragment,
            ShaderParams::new([0; 16], &[], 0),
        )));
        Translator::new(info, runtime, &Profile::default(), read)
    }

    #[test]
    fn a_fragment_shared_write_goes_to_a_scratch_vgpr() {
        let runtime = RuntimeInfo::new(HwStage::Fragment, SwStage::Fragment);
        let read = |_| 0u32;
        let mut translator = translator(HwStage::Fragment, &runtime, &read);
        translator.next_vgpr_num = 8;
        let block = Block::new_ref();
        translator.ir = IrEmitter::new(&block);

        let mut inst = GcnInst {
            opcode: Opcode::DS_WRITE_B32,
            ..Default::default()
        };
        inst.control =
            crate::gpu::shader_recompiler::frontend::instruction::InstControl::from_raw(1 << 8);
        inst.src[0].code = 0;
        inst.src[1].code = 1;
        translator.emit_data_share(&inst);

        let store = block
            .borrow()
            .instructions()
            .iter()
            .find(|inst| inst.borrow().get_opcode() == IrOpcode::SetVectorRegister)
            .expect("the scratch store")
            .clone();
        assert_eq!(store.borrow().arg(0).vector_reg(), VectorReg::V8);
        assert_eq!(
            store.borrow().arg(1).inst().borrow().get_opcode(),
            IrOpcode::GetVectorRegister
        );
    }

    #[test]
    fn a_returning_atomic_writes_its_result() {
        let runtime = RuntimeInfo::new(HwStage::Compute, SwStage::Compute);
        let read = |_| 0u32;
        let mut translator = translator(HwStage::Compute, &runtime, &read);
        let block = Block::new_ref();
        translator.ir = IrEmitter::new(&block);

        let mut inst = GcnInst {
            opcode: Opcode::DS_ADD_RTN_U32,
            ..Default::default()
        };
        inst.src[0] = crate::gpu::shader_recompiler::frontend::instruction::InstOperand {
            field: crate::gpu::shader_recompiler::frontend::opcodes::OperandField::VectorGPR.0,
            code: 0,
            ..Default::default()
        };
        inst.src[1] = crate::gpu::shader_recompiler::frontend::instruction::InstOperand {
            field: crate::gpu::shader_recompiler::frontend::opcodes::OperandField::VectorGPR.0,
            code: 1,
            ..Default::default()
        };
        inst.dst[0] = crate::gpu::shader_recompiler::frontend::instruction::InstOperand {
            field: crate::gpu::shader_recompiler::frontend::opcodes::OperandField::VectorGPR.0,
            code: 2,
            ..Default::default()
        };
        translator.emit_data_share(&inst);

        let instructions = block.borrow().instructions().to_vec();
        assert!(instructions
            .iter()
            .any(|inst| inst.borrow().get_opcode() == IrOpcode::SharedAtomicIAdd32));
        let store = instructions
            .iter()
            .find(|inst| inst.borrow().get_opcode() == IrOpcode::SetVectorRegister)
            .expect("the result store")
            .clone();
        assert_eq!(
            store.borrow().arg(1).inst().borrow().get_opcode(),
            IrOpcode::SharedAtomicIAdd32
        );
    }

    #[test]
    fn a_broadcast_swizzle_uses_quad_broadcast() {
        let runtime = RuntimeInfo::new(HwStage::Compute, SwStage::Compute);
        let read = |_| 0u32;
        let mut translator = translator(HwStage::Compute, &runtime, &read);
        let block = Block::new_ref();
        translator.ir = IrEmitter::new(&block);

        let mut inst = GcnInst {
            opcode: Opcode::DS_SWIZZLE_B32,
            ..Default::default()
        };
        inst.control = crate::gpu::shader_recompiler::frontend::instruction::InstControl::from_raw(
            (0x80 << 8) | 0x55,
        );
        inst.src[0] = crate::gpu::shader_recompiler::frontend::instruction::InstOperand {
            field: crate::gpu::shader_recompiler::frontend::opcodes::OperandField::VectorGPR.0,
            code: 0,
            ..Default::default()
        };
        inst.dst[0] = crate::gpu::shader_recompiler::frontend::instruction::InstOperand {
            field: crate::gpu::shader_recompiler::frontend::opcodes::OperandField::VectorGPR.0,
            code: 1,
            ..Default::default()
        };
        translator.emit_data_share(&inst);

        assert!(block
            .borrow()
            .instructions()
            .iter()
            .any(|inst| inst.borrow().get_opcode() == IrOpcode::QuadBroadcast));
    }
}
