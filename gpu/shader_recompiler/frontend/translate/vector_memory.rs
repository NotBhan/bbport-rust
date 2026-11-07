//! Vector memory translation, ported from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/translate/vector_memory.cpp`.

use super::translator::{AtomicOp, Translator};
use crate::gpu::amdgpu::pixel_format::{DataFormat, NumberFormat};
use crate::gpu::shader_recompiler::frontend::instruction::{
    GcnInst, InstControlMTBUF, InstControlMUBUF,
};
use crate::gpu::shader_recompiler::frontend::opcodes::{
    ImageResComponent, ImageResFlags, MimgModifier, MimgModifierFlags, Opcode,
};
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::reg::{
    reg_add, BufferInstInfo, RegType as _, ScalarReg, TextureInstInfo, VectorReg,
};
use crate::gpu::shader_recompiler::ir::value::{Value, F32, U1, U16, U32, U32F32, U8, U8U16U32U64};

impl Translator<'_> {
    /// `EmitVectorMemory`.
    pub(crate) fn emit_vector_memory(&mut self, inst: &GcnInst) {
        match inst.opcode {
            Opcode::TBUFFER_LOAD_FORMAT_X => self.buffer_load(1, true, false, inst, 32, false),
            Opcode::TBUFFER_LOAD_FORMAT_XY => self.buffer_load(2, true, false, inst, 32, false),
            Opcode::TBUFFER_LOAD_FORMAT_XYZ => self.buffer_load(3, true, false, inst, 32, false),
            Opcode::TBUFFER_LOAD_FORMAT_XYZW => self.buffer_load(4, true, false, inst, 32, false),
            Opcode::BUFFER_LOAD_FORMAT_X => self.buffer_load(1, false, true, inst, 32, false),
            Opcode::BUFFER_LOAD_FORMAT_XY => self.buffer_load(2, false, true, inst, 32, false),
            Opcode::BUFFER_LOAD_FORMAT_XYZ => self.buffer_load(3, false, true, inst, 32, false),
            Opcode::BUFFER_LOAD_FORMAT_XYZW => self.buffer_load(4, false, true, inst, 32, false),
            Opcode::BUFFER_LOAD_UBYTE => self.buffer_load(1, false, false, inst, 8, false),
            Opcode::BUFFER_LOAD_SBYTE => self.buffer_load(1, false, false, inst, 8, true),
            Opcode::BUFFER_LOAD_USHORT => self.buffer_load(1, false, false, inst, 16, false),
            Opcode::BUFFER_LOAD_SSHORT => self.buffer_load(1, false, false, inst, 16, true),
            Opcode::BUFFER_LOAD_DWORD => self.buffer_load(1, false, false, inst, 32, false),
            Opcode::BUFFER_LOAD_DWORDX2 => self.buffer_load(2, false, false, inst, 32, false),
            Opcode::BUFFER_LOAD_DWORDX3 => self.buffer_load(3, false, false, inst, 32, false),
            Opcode::BUFFER_LOAD_DWORDX4 => self.buffer_load(4, false, false, inst, 32, false),
            Opcode::BUFFER_STORE_FORMAT_X => self.buffer_store(1, false, true, inst, 32),
            Opcode::BUFFER_STORE_FORMAT_XY => self.buffer_store(2, false, true, inst, 32),
            Opcode::BUFFER_STORE_FORMAT_XYZ => self.buffer_store(3, false, true, inst, 32),
            Opcode::BUFFER_STORE_FORMAT_XYZW => self.buffer_store(4, false, true, inst, 32),
            Opcode::TBUFFER_STORE_FORMAT_X => self.buffer_store(1, true, false, inst, 32),
            Opcode::TBUFFER_STORE_FORMAT_XY => self.buffer_store(2, true, false, inst, 32),
            Opcode::TBUFFER_STORE_FORMAT_XYZ => self.buffer_store(3, true, false, inst, 32),
            Opcode::TBUFFER_STORE_FORMAT_XYZW => self.buffer_store(4, true, false, inst, 32),
            Opcode::BUFFER_STORE_BYTE => self.buffer_store(1, false, false, inst, 8),
            Opcode::BUFFER_STORE_SHORT => self.buffer_store(1, false, false, inst, 16),
            Opcode::BUFFER_STORE_DWORD => self.buffer_store(1, false, false, inst, 32),
            Opcode::BUFFER_STORE_DWORDX2 => self.buffer_store(2, false, false, inst, 32),
            Opcode::BUFFER_STORE_DWORDX3 => self.buffer_store(3, false, false, inst, 32),
            Opcode::BUFFER_STORE_DWORDX4 => self.buffer_store(4, false, false, inst, 32),
            Opcode::BUFFER_ATOMIC_ADD => self.buffer_atomic_u32(AtomicOp::Add, inst),
            Opcode::BUFFER_ATOMIC_SWAP => self.buffer_atomic_u32(AtomicOp::Swap, inst),
            Opcode::BUFFER_ATOMIC_CMPSWAP => self.buffer_atomic_u32(AtomicOp::CmpSwap, inst),
            Opcode::BUFFER_ATOMIC_SMIN => self.buffer_atomic_u32(AtomicOp::Smin, inst),
            Opcode::BUFFER_ATOMIC_SMIN_X2 => self.buffer_atomic_u64(AtomicOp::Smin, inst),
            Opcode::BUFFER_ATOMIC_UMIN => self.buffer_atomic_u32(AtomicOp::Umin, inst),
            Opcode::BUFFER_ATOMIC_UMIN_X2 => self.buffer_atomic_u64(AtomicOp::Umin, inst),
            Opcode::BUFFER_ATOMIC_SMAX => self.buffer_atomic_u32(AtomicOp::Smax, inst),
            Opcode::BUFFER_ATOMIC_SMAX_X2 => self.buffer_atomic_u64(AtomicOp::Smax, inst),
            Opcode::BUFFER_ATOMIC_UMAX => self.buffer_atomic_u32(AtomicOp::Umax, inst),
            Opcode::BUFFER_ATOMIC_UMAX_X2 => self.buffer_atomic_u64(AtomicOp::Umax, inst),
            Opcode::BUFFER_ATOMIC_AND => self.buffer_atomic_u32(AtomicOp::And, inst),
            Opcode::BUFFER_ATOMIC_OR => self.buffer_atomic_u32(AtomicOp::Or, inst),
            Opcode::BUFFER_ATOMIC_XOR => self.buffer_atomic_u32(AtomicOp::Xor, inst),
            Opcode::BUFFER_ATOMIC_INC => self.buffer_atomic_u32(AtomicOp::Inc, inst),
            Opcode::BUFFER_ATOMIC_DEC => self.buffer_atomic_u32(AtomicOp::Dec, inst),
            Opcode::BUFFER_ATOMIC_FMIN => self.buffer_atomic_f32(AtomicOp::Fmin, inst),
            Opcode::BUFFER_ATOMIC_FMAX => self.buffer_atomic_f32(AtomicOp::Fmax, inst),
            Opcode::BUFFER_ATOMIC_FCMPSWAP => self.buffer_atomic_f32(AtomicOp::FCmpSwap, inst),
            Opcode::IMAGE_LOAD => self.image_load(false, inst),
            Opcode::IMAGE_LOAD_MIP => self.image_load(true, inst),
            Opcode::IMAGE_STORE => self.image_store(false, inst),
            Opcode::IMAGE_STORE_MIP => self.image_store(true, inst),
            Opcode::IMAGE_GET_RESINFO => self.image_get_resinfo(inst),
            Opcode::IMAGE_ATOMIC_SWAP => self.image_atomic(AtomicOp::Swap, inst),
            Opcode::IMAGE_ATOMIC_CMPSWAP => self.image_atomic(AtomicOp::CmpSwap, inst),
            Opcode::IMAGE_ATOMIC_ADD => self.image_atomic(AtomicOp::Add, inst),
            Opcode::IMAGE_ATOMIC_SMIN => self.image_atomic(AtomicOp::Smin, inst),
            Opcode::IMAGE_ATOMIC_UMIN => self.image_atomic(AtomicOp::Umin, inst),
            Opcode::IMAGE_ATOMIC_FMIN => self.image_atomic(AtomicOp::Fmin, inst),
            Opcode::IMAGE_ATOMIC_SMAX => self.image_atomic(AtomicOp::Smax, inst),
            Opcode::IMAGE_ATOMIC_FMAX => self.image_atomic(AtomicOp::Fmax, inst),
            Opcode::IMAGE_ATOMIC_UMAX => self.image_atomic(AtomicOp::Umax, inst),
            Opcode::IMAGE_ATOMIC_AND => self.image_atomic(AtomicOp::And, inst),
            Opcode::IMAGE_ATOMIC_OR => self.image_atomic(AtomicOp::Or, inst),
            Opcode::IMAGE_ATOMIC_XOR => self.image_atomic(AtomicOp::Xor, inst),
            Opcode::IMAGE_ATOMIC_INC => self.image_atomic(AtomicOp::Inc, inst),
            Opcode::IMAGE_ATOMIC_DEC => self.image_atomic(AtomicOp::Dec, inst),
            Opcode::IMAGE_SAMPLE
            | Opcode::IMAGE_SAMPLE_D
            | Opcode::IMAGE_SAMPLE_L
            | Opcode::IMAGE_SAMPLE_B
            | Opcode::IMAGE_SAMPLE_LZ
            | Opcode::IMAGE_SAMPLE_C
            | Opcode::IMAGE_SAMPLE_C_LZ
            | Opcode::IMAGE_SAMPLE_O
            | Opcode::IMAGE_SAMPLE_D_O
            | Opcode::IMAGE_SAMPLE_L_O
            | Opcode::IMAGE_SAMPLE_B_O
            | Opcode::IMAGE_SAMPLE_LZ_O
            | Opcode::IMAGE_SAMPLE_C_O
            | Opcode::IMAGE_SAMPLE_C_LZ_O
            | Opcode::IMAGE_SAMPLE_CD => self.image_sample(inst),
            Opcode::IMAGE_GATHER4
            | Opcode::IMAGE_GATHER4_L
            | Opcode::IMAGE_GATHER4_LZ
            | Opcode::IMAGE_GATHER4_C
            | Opcode::IMAGE_GATHER4_O
            | Opcode::IMAGE_GATHER4_C_O
            | Opcode::IMAGE_GATHER4_C_LZ
            | Opcode::IMAGE_GATHER4_LZ_O
            | Opcode::IMAGE_GATHER4_C_LZ_O => self.image_gather(inst),
            Opcode::IMAGE_GET_LOD => self.image_get_lod(inst),
            _ => self.log_missing_opcode(inst),
        }
    }

    fn buffer_address(&mut self, inst: &GcnInst) -> (Value, InstControlMUBUF) {
        let mubuf = inst.control.mubuf();
        let vaddr = VectorReg::from_raw(inst.src[0].code);
        let index = if mubuf.idxen() {
            self.ir.get_vector_reg(vaddr)
        } else {
            self.ir.imm32(0)
        };
        let voffset_vgpr = if mubuf.idxen() {
            reg_add(vaddr, 1).expect("the vgprs fit")
        } else {
            vaddr
        };
        let voffset = if mubuf.offen() {
            self.ir.get_vector_reg(voffset_vgpr)
        } else {
            self.ir.imm32(0)
        };
        let soffset = self.get_src(&inst.src[3]);
        let address = self.ir.composite_construct_slice(&[
            index.into_value(),
            voffset.into_value(),
            soffset.into_value(),
        ]);
        (address, mubuf)
    }

    fn buffer_handle(&mut self, sharp: ScalarReg) -> Value {
        let s0 = self.ir.get_scalar_reg(sharp);
        let s1 = self
            .ir
            .get_scalar_reg(ScalarReg::from_raw(sharp as u32 + 1));
        let s2 = self
            .ir
            .get_scalar_reg(ScalarReg::from_raw(sharp as u32 + 2));
        let s3 = self
            .ir
            .get_scalar_reg(ScalarReg::from_raw(sharp as u32 + 3));
        self.ir.composite_construct4(s0, s1, s2, s3)
    }

    fn buffer_info(
        &self,
        mubuf: &InstControlMUBUF,
        typed: bool,
        is_inst_typed: bool,
        mtbuf: Option<InstControlMTBUF>,
    ) -> BufferInstInfo {
        let mut info = BufferInstInfo::from_raw(0);
        info.set_index_enable(u64::from(mubuf.idxen()));
        info.set_voffset_enable(u64::from(mubuf.offen()));
        info.set_inst_offset(mubuf.offset());
        info.set_globally_coherent(u64::from(mubuf.glc()));
        info.set_system_coherent(u64::from(mubuf.slc()));
        info.set_typed(u64::from(typed));
        if is_inst_typed {
            let mtbuf = mtbuf.expect("the typed instruction has MTBUF fields");
            info.set_inst_data_format(
                DataFormat::from_raw(mtbuf.dfmt() as u32).expect("the data format is valid"),
            );
            info.set_inst_number_format(
                NumberFormat::from_raw(mtbuf.nfmt() as u32).expect("the number format is valid"),
            );
        } else {
            info.set_inst_data_format(DataFormat::FormatInvalid);
        }
        info.set_pc(self.pc as u64);
        info
    }

    /// `BUFFER_LOAD`.
    fn buffer_load(
        &mut self,
        num_dwords: u32,
        is_inst_typed: bool,
        is_buffer_typed: bool,
        inst: &GcnInst,
        scalar_width: u32,
        is_signed: bool,
    ) {
        let (address, mubuf) = self.buffer_address(inst);
        let typed = is_inst_typed || is_buffer_typed;
        let info = self.buffer_info(&mubuf, typed, is_inst_typed, Some(inst.control.mtbuf()));
        let sharp = ScalarReg::from_raw(inst.src[2].code * 4);
        let handle = self.buffer_handle(sharp);
        let dst = VectorReg::from_raw(inst.src[1].code);
        if typed {
            let value = self.ir.load_buffer_format(&handle, &address, info);
            for index in 0..num_dwords {
                let component = self.ir.composite_extract(&value, index as usize);
                let reg = reg_add(dst, index as i32).expect("the destination vgprs fit");
                self.ir.set_vector_reg(reg, &U32F32::new(component));
            }
            return;
        }
        let value = match scalar_width {
            8 => {
                let byte = self.ir.load_buffer_u8(&handle, &address, info);
                let raw = U8U16U32U64::new(byte.into_value());
                if is_signed {
                    self.ir.s_convert(32, &raw).into_value()
                } else {
                    self.ir.u_convert(32, &raw).into_value()
                }
            }
            16 => {
                let short = self.ir.load_buffer_u16(&handle, &address, info);
                let raw = U8U16U32U64::new(short.into_value());
                if is_signed {
                    self.ir.s_convert(32, &raw).into_value()
                } else {
                    self.ir.u_convert(32, &raw).into_value()
                }
            }
            32 => self
                .ir
                .load_buffer_u32(num_dwords as i32, &handle, &address, info),
            _ => unreachable!("invalid scalar width"),
        };
        if num_dwords == 1 {
            self.ir.set_vector_reg(dst, &U32F32::new(value));
            return;
        }
        for index in 0..num_dwords {
            let component = self.ir.composite_extract(&value, index as usize);
            let reg = reg_add(dst, index as i32).expect("the destination vgprs fit");
            self.ir.set_vector_reg(reg, &U32F32::new(component));
        }
    }

    /// `BUFFER_STORE`.
    fn buffer_store(
        &mut self,
        num_dwords: u32,
        is_inst_typed: bool,
        is_buffer_typed: bool,
        inst: &GcnInst,
        scalar_width: u32,
    ) {
        let (address, mubuf) = self.buffer_address(inst);
        let typed = is_inst_typed || is_buffer_typed;
        let info = self.buffer_info(&mubuf, typed, is_inst_typed, Some(inst.control.mtbuf()));
        let sharp = ScalarReg::from_raw(inst.src[2].code * 4);
        let handle = self.buffer_handle(sharp);
        let src = VectorReg::from_raw(inst.src[1].code);

        let mut comps: Vec<Value> = Vec::with_capacity(4);
        for index in 0..num_dwords {
            let reg = reg_add(src, index as i32).expect("the source vgprs fit");
            if typed {
                comps.push(self.ir.get_vector_reg_f32(reg).into_value());
            } else {
                comps.push(self.ir.get_vector_reg(reg).into_value());
            }
        }
        if typed {
            for _ in num_dwords..4 {
                comps.push(self.ir.imm32_float(0.0).into_value());
            }
            let value = self.ir.composite_construct_slice(&comps);
            self.ir.store_buffer_format(&handle, &address, &value, info);
        } else {
            let value = if num_dwords == 1 {
                comps[0].clone()
            } else {
                self.ir.composite_construct_slice(&comps)
            };
            let value = if scalar_width != 32 {
                let current = U32::new(value);
                self.ir.u_convert(
                    scalar_width as usize,
                    &U8U16U32U64::new(current.into_value()),
                )
            } else {
                U8U16U32U64::new(value)
            };
            match scalar_width {
                8 => {
                    let data = U8::new(value.into_value());
                    self.ir.store_buffer_u8(&handle, &address, &data, info);
                }
                16 => {
                    let data = U16::new(value.into_value());
                    self.ir.store_buffer_u16(&handle, &address, &data, info);
                }
                32 => {
                    self.ir.store_buffer_u32(
                        num_dwords as i32,
                        &handle,
                        &address,
                        &value.into_value(),
                        info,
                    );
                }
                _ => unreachable!("invalid scalar width"),
            }
        }
    }

    fn buffer_atomic_common(
        &mut self,
        inst: &GcnInst,
    ) -> (Value, Value, BufferInstInfo, u32, bool) {
        let (address, mubuf) = self.buffer_address(inst);
        let info = self.buffer_info(&mubuf, false, false, None);
        let sharp = ScalarReg::from_raw(inst.src[2].code * 4);
        let handle = self.buffer_handle(sharp);
        (handle, address, info, inst.src[1].code, mubuf.glc())
    }

    /// `BUFFER_ATOMIC` for 32-bit integers.
    fn buffer_atomic_u32(&mut self, op: AtomicOp, inst: &GcnInst) {
        let (handle, address, info, vdata, glc) = self.buffer_atomic_common(inst);
        let value = self
            .ir
            .get_vector_reg(VectorReg::from_raw(vdata))
            .into_value();
        let original = match op {
            AtomicOp::Swap => self.ir.buffer_atomic_swap(&handle, &address, &value, info),
            AtomicOp::CmpSwap => {
                let cmp = self
                    .ir
                    .get_vector_reg(VectorReg::from_raw(vdata + 1))
                    .into_value();
                self.ir
                    .buffer_atomic_cmp_swap(&handle, &address, &value, &cmp, info)
            }
            AtomicOp::Add => self.ir.buffer_atomic_i_add(&handle, &address, &value, info),
            AtomicOp::Smin => self
                .ir
                .buffer_atomic_i_min(&handle, &address, &value, true, info),
            AtomicOp::Umin => self
                .ir
                .buffer_atomic_i_min(&handle, &address, &value, false, info),
            AtomicOp::Smax => self
                .ir
                .buffer_atomic_i_max(&handle, &address, &value, true, info),
            AtomicOp::Umax => self
                .ir
                .buffer_atomic_i_max(&handle, &address, &value, false, info),
            AtomicOp::And => self.ir.buffer_atomic_and(&handle, &address, &value, info),
            AtomicOp::Or => self.ir.buffer_atomic_or(&handle, &address, &value, info),
            AtomicOp::Xor => self.ir.buffer_atomic_xor(&handle, &address, &value, info),
            AtomicOp::Inc => self.ir.buffer_atomic_inc(&handle, &address, info),
            AtomicOp::Dec => self.ir.buffer_atomic_dec(&handle, &address, info),
            _ => unreachable!("unsupported 32-bit buffer atomic"),
        };
        if glc {
            let reg = VectorReg::from_raw(vdata);
            self.ir.set_vector_reg(reg, &U32F32::new(original));
        }
    }

    /// `BUFFER_ATOMIC` for 64-bit integers.
    fn buffer_atomic_u64(&mut self, op: AtomicOp, inst: &GcnInst) {
        let (handle, address, info, vdata, glc) = self.buffer_atomic_common(inst);
        let lo = self.ir.get_vector_reg(VectorReg::from_raw(vdata));
        let hi = self.ir.get_vector_reg(VectorReg::from_raw(vdata + 1));
        let composite = self.ir.composite_construct(lo, hi);
        let value = self.ir.pack_uint_2x32(&composite).into_value();
        let original = match op {
            AtomicOp::Add => self.ir.buffer_atomic_i_add(&handle, &address, &value, info),
            AtomicOp::Smin => self
                .ir
                .buffer_atomic_i_min(&handle, &address, &value, true, info),
            AtomicOp::Umin => self
                .ir
                .buffer_atomic_i_min(&handle, &address, &value, false, info),
            AtomicOp::Smax => self
                .ir
                .buffer_atomic_i_max(&handle, &address, &value, true, info),
            AtomicOp::Umax => self
                .ir
                .buffer_atomic_i_max(&handle, &address, &value, false, info),
            _ => unreachable!("unsupported 64-bit buffer atomic"),
        };
        if glc {
            unreachable!("returning 64-bit buffer atomics are not supported");
        }
        let _ = original;
    }

    /// `BUFFER_ATOMIC` for floats.
    fn buffer_atomic_f32(&mut self, op: AtomicOp, inst: &GcnInst) {
        let (handle, address, info, vdata, glc) = self.buffer_atomic_common(inst);
        let value = self
            .ir
            .get_vector_reg_f32(VectorReg::from_raw(vdata))
            .into_value();
        let original = match op {
            AtomicOp::Swap => self.ir.buffer_atomic_swap(&handle, &address, &value, info),
            AtomicOp::CmpSwap => {
                let cmp = self
                    .ir
                    .get_vector_reg(VectorReg::from_raw(vdata + 1))
                    .into_value();
                self.ir
                    .buffer_atomic_cmp_swap(&handle, &address, &value, &cmp, info)
            }
            AtomicOp::FCmpSwap => {
                let cmp = self
                    .ir
                    .get_vector_reg(VectorReg::from_raw(vdata + 1))
                    .into_value();
                self.ir
                    .buffer_atomic_f_cmp_swap(&handle, &address, &value, &cmp, info)
            }
            AtomicOp::Fmin => self.ir.buffer_atomic_f_min(&handle, &address, &value, info),
            AtomicOp::Fmax => self.ir.buffer_atomic_f_max(&handle, &address, &value, info),
            _ => unreachable!("unsupported float buffer atomic"),
        };
        if glc {
            let reg = VectorReg::from_raw(vdata);
            self.ir.set_vector_reg(reg, &U32F32::new(original));
        }
    }

    fn image_handle_pair(&mut self, tsharp: ScalarReg, r128: bool) -> (Value, Value) {
        let t0 = self.ir.get_scalar_reg(tsharp);
        let t1 = self
            .ir
            .get_scalar_reg(ScalarReg::from_raw(tsharp as u32 + 1));
        let t2 = self
            .ir
            .get_scalar_reg(ScalarReg::from_raw(tsharp as u32 + 2));
        let t3 = self
            .ir
            .get_scalar_reg(ScalarReg::from_raw(tsharp as u32 + 3));
        let low = self.ir.composite_construct4(t0, t1, t2, t3);
        let high = if r128 {
            Value::empty()
        } else {
            let t4 = self
                .ir
                .get_scalar_reg(ScalarReg::from_raw(tsharp as u32 + 4));
            let t5 = self
                .ir
                .get_scalar_reg(ScalarReg::from_raw(tsharp as u32 + 5));
            let t6 = self
                .ir
                .get_scalar_reg(ScalarReg::from_raw(tsharp as u32 + 6));
            let t7 = self
                .ir
                .get_scalar_reg(ScalarReg::from_raw(tsharp as u32 + 7));
            self.ir.composite_construct4(t4, t5, t6, t7)
        };
        (low, high)
    }

    fn image_body(&mut self, addr: VectorReg) -> Value {
        let a0 = self.ir.get_vector_reg(addr);
        let a1 = self
            .ir
            .get_vector_reg(reg_add(addr, 1).expect("the vgprs fit"));
        let a2 = self
            .ir
            .get_vector_reg(reg_add(addr, 2).expect("the vgprs fit"));
        let a3 = self
            .ir
            .get_vector_reg(reg_add(addr, 3).expect("the vgprs fit"));
        self.ir.composite_construct4(a0, a1, a2, a3)
    }

    /// `IMAGE_LOAD`.
    fn image_load(&mut self, has_mip: bool, inst: &GcnInst) {
        let mimg = inst.control.mimg();
        let addr = VectorReg::from_raw(inst.src[0].code);
        let mut dst = VectorReg::from_raw(inst.dst[0].code);
        let tsharp = ScalarReg::from_raw(inst.src[2].code * 4);
        let (low, high) = self.image_handle_pair(tsharp, mimg.r128());
        let body = self.image_body(addr);
        let mut info = TextureInstInfo::default();
        info.set_has_lod(u32::from(has_mip));
        info.set_is_array(u32::from(mimg.da()));
        info.set_is_r128(u32::from(mimg.r128()));
        let handle = self.ir.image_handle(&low, &high);
        let texel = self
            .ir
            .image_read(&handle, &body, &U32::empty(), &U32::empty(), info);
        for index in 0..4u64 {
            if (mimg.dmask() >> index) & 1 == 0 {
                continue;
            }
            let value = F32::new(self.ir.composite_extract(&texel, index as usize));
            self.ir
                .set_vector_reg(dst, &U32F32::new(value.into_value()));
            dst = reg_add(dst, 1).expect("the destination vgprs fit");
        }
    }

    /// `IMAGE_STORE`.
    fn image_store(&mut self, has_mip: bool, inst: &GcnInst) {
        let mimg = inst.control.mimg();
        let addr = VectorReg::from_raw(inst.src[0].code);
        let mut data = VectorReg::from_raw(inst.dst[0].code);
        let tsharp = ScalarReg::from_raw(inst.src[2].code * 4);
        let (low, high) = self.image_handle_pair(tsharp, mimg.r128());
        let body = self.image_body(addr);
        let mut info = TextureInstInfo::default();
        info.set_has_lod(u32::from(has_mip));
        info.set_is_array(u32::from(mimg.da()));
        info.set_is_r128(u32::from(mimg.r128()));

        let mut comps: Vec<F32> = Vec::with_capacity(4);
        for index in 0..4u64 {
            if (mimg.dmask() >> index) & 1 == 0 {
                comps.push(self.ir.imm32_float(0.0));
                continue;
            }
            comps.push(self.ir.get_vector_reg_f32(data));
            data = reg_add(data, 1).expect("the source vgprs fit");
        }
        let value = self.ir.composite_construct4(
            comps[0].clone(),
            comps[1].clone(),
            comps[2].clone(),
            comps[3].clone(),
        );
        let handle = self.ir.image_handle(&low, &high);
        self.ir
            .image_write(&handle, &body, &U32::empty(), &U32::empty(), &value, info);
    }

    /// `IMAGE_GET_RESINFO`.
    fn image_get_resinfo(&mut self, inst: &GcnInst) {
        let mimg = inst.control.mimg();
        let mut dst = VectorReg::from_raw(inst.dst[0].code);
        let tsharp = ScalarReg::from_raw(inst.src[2].code * 4);
        let flags = ImageResFlags::from_bits(mimg.dmask() as u32);
        let has_mips = flags.test(ImageResComponent::MipCount);
        let lod = self
            .ir
            .get_vector_reg(VectorReg::from_raw(inst.src[0].code));
        let (low, high) = self.image_handle_pair(tsharp, mimg.r128());
        let mut info = TextureInstInfo::default();
        info.set_is_array(u32::from(mimg.da()));
        info.set_is_r128(u32::from(mimg.r128()));
        let handle = self.ir.image_handle(&low, &high);
        let skip_mips = U1::new(Value::from_u1(has_mips));
        let size = self
            .ir
            .image_query_dimension(&handle, &lod, &skip_mips, info);
        if flags.test(ImageResComponent::Width) {
            let value = U32::new(self.ir.composite_extract(&size, 0));
            self.ir
                .set_vector_reg(dst, &U32F32::new(value.into_value()));
            dst = reg_add(dst, 1).expect("the destination vgprs fit");
        }
        if flags.test(ImageResComponent::Height) {
            let value = U32::new(self.ir.composite_extract(&size, 1));
            self.ir
                .set_vector_reg(dst, &U32F32::new(value.into_value()));
            dst = reg_add(dst, 1).expect("the destination vgprs fit");
        }
        if flags.test(ImageResComponent::Depth) {
            let value = U32::new(self.ir.composite_extract(&size, 2));
            self.ir
                .set_vector_reg(dst, &U32F32::new(value.into_value()));
            dst = reg_add(dst, 1).expect("the destination vgprs fit");
        }
        if has_mips {
            let value = U32::new(self.ir.composite_extract(&size, 3));
            self.ir
                .set_vector_reg(dst, &U32F32::new(value.into_value()));
        }
    }

    /// `IMAGE_ATOMIC`.
    fn image_atomic(&mut self, op: AtomicOp, inst: &GcnInst) {
        let mimg = inst.control.mimg();
        let value_reg = VectorReg::from_raw(inst.dst[0].code);
        let addr = VectorReg::from_raw(inst.src[0].code);
        let tsharp = ScalarReg::from_raw(inst.src[2].code * 4);
        let mut info = TextureInstInfo::default();
        info.set_is_array(u32::from(mimg.da()));
        info.set_is_r128(u32::from(mimg.r128()));
        let value = self.ir.get_vector_reg(value_reg).into_value();
        let (low, high) = self.image_handle_pair(tsharp, mimg.r128());
        let handle = self.ir.image_handle(&low, &high);
        let body = self.image_body(addr);
        let prev = match op {
            AtomicOp::Swap => {
                self.ir
                    .image_atomic_exchange(&handle, &body, &value, TextureInstInfo::default())
            }
            AtomicOp::CmpSwap => {
                let cmp = self
                    .ir
                    .get_vector_reg(reg_add(value_reg, 1).expect("the vgprs fit"))
                    .into_value();
                self.ir
                    .image_atomic_cmp_swap(&handle, &body, &value, &cmp, info)
            }
            AtomicOp::Add => self.ir.image_atomic_i_add(&handle, &body, &value, info),
            AtomicOp::Smin => self.ir.image_atomic_s_min(&handle, &body, &value, info),
            AtomicOp::Umin => self.ir.image_atomic_u_min(&handle, &body, &value, info),
            AtomicOp::Smax => self.ir.image_atomic_s_max(&handle, &body, &value, info),
            AtomicOp::Umax => self.ir.image_atomic_u_max(&handle, &body, &value, info),
            AtomicOp::Fmax => self.ir.image_atomic_f_max(&handle, &body, &value, info),
            AtomicOp::Fmin => self.ir.image_atomic_f_min(&handle, &body, &value, info),
            AtomicOp::And => self.ir.image_atomic_and(&handle, &body, &value, info),
            AtomicOp::Or => self.ir.image_atomic_or(&handle, &body, &value, info),
            AtomicOp::Xor => self.ir.image_atomic_xor(&handle, &body, &value, info),
            AtomicOp::Inc => self.ir.image_atomic_inc(&handle, &body, &value, info),
            AtomicOp::Dec => self.ir.image_atomic_dec(&handle, &body, &value, info),
            _ => unreachable!("unsupported image atomic"),
        };
        if mimg.glc() {
            let prev = U32::new(prev);
            self.ir
                .set_vector_reg(value_reg, &U32F32::new(prev.into_value()));
        }
    }

    /// `IMAGE_SAMPLE`.
    fn image_sample(&mut self, inst: &GcnInst) {
        let mimg = inst.control.mimg();
        let addr = VectorReg::from_raw(inst.src[0].code);
        let mut dst = VectorReg::from_raw(inst.dst[0].code);
        let tsharp = ScalarReg::from_raw(inst.src[2].code * 4);
        let sampler = ScalarReg::from_raw(inst.src[3].code * 4);
        let flags = MimgModifierFlags::from_bits(mimg.modifiers() as u32);
        let texel = emit_image_sample(&mut self.ir, inst, tsharp, sampler, addr, false, self.pc);
        for index in 0..4u64 {
            if (mimg.dmask() >> index) & 1 == 0 {
                continue;
            }
            let value = if flags.test(MimgModifier::Pcf) {
                if index < 3 {
                    F32::new(texel.clone())
                } else {
                    self.ir.imm32_float(1.0)
                }
            } else {
                F32::new(self.ir.composite_extract(&texel, index as usize))
            };
            self.ir
                .set_vector_reg(dst, &U32F32::new(value.into_value()));
            dst = reg_add(dst, 1).expect("the destination vgprs fit");
        }
    }

    /// `IMAGE_GATHER`.
    fn image_gather(&mut self, inst: &GcnInst) {
        let mimg = inst.control.mimg();
        let addr = VectorReg::from_raw(inst.src[0].code);
        let mut dst = VectorReg::from_raw(inst.dst[0].code);
        let tsharp = ScalarReg::from_raw(inst.src[2].code * 4);
        let sampler = ScalarReg::from_raw(inst.src[3].code * 4);
        let flags = MimgModifierFlags::from_bits(mimg.modifiers() as u32);
        assert_eq!(
            mimg.dmask().count_ones(),
            1,
            "Unexpected bits in gather dmask"
        );
        assert!(!flags.test(MimgModifier::Pcf) || mimg.dmask() & 1 != 0);
        let texel = emit_image_sample(&mut self.ir, inst, tsharp, sampler, addr, true, self.pc);
        for index in 0..4usize {
            let value = F32::new(self.ir.composite_extract(&texel, index));
            self.ir
                .set_vector_reg(dst, &U32F32::new(value.into_value()));
            dst = reg_add(dst, 1).expect("the destination vgprs fit");
        }
    }

    /// `IMAGE_GET_LOD`.
    fn image_get_lod(&mut self, inst: &GcnInst) {
        let mimg = inst.control.mimg();
        let mut dst = VectorReg::from_raw(inst.dst[0].code);
        let addr = VectorReg::from_raw(inst.src[0].code);
        let tsharp = ScalarReg::from_raw(inst.src[2].code * 4);
        let mut info = TextureInstInfo::default();
        info.set_is_array(u32::from(mimg.da()));
        info.set_is_r128(u32::from(mimg.r128()));
        let (low, high) = self.image_handle_pair(tsharp, mimg.r128());
        let body = {
            let a0 = self.ir.get_vector_reg_f32(addr);
            let a1 = self
                .ir
                .get_vector_reg_f32(reg_add(addr, 1).expect("the vgprs fit"));
            let a2 = self
                .ir
                .get_vector_reg_f32(reg_add(addr, 2).expect("the vgprs fit"));
            let a3 = self
                .ir
                .get_vector_reg_f32(reg_add(addr, 3).expect("the vgprs fit"));
            self.ir.composite_construct4(a0, a1, a2, a3)
        };
        let handle = self.ir.image_handle(&low, &high);
        let lod = self.ir.image_query_lod(&handle, &body, info);
        if mimg.dmask() & 1 != 0 {
            let value = F32::new(self.ir.composite_extract(&lod, 0));
            self.ir
                .set_vector_reg(dst, &U32F32::new(value.into_value()));
            dst = reg_add(dst, 1).expect("the destination vgprs fit");
        }
        if mimg.dmask() & 2 != 0 {
            let value = F32::new(self.ir.composite_extract(&lod, 1));
            self.ir
                .set_vector_reg(dst, &U32F32::new(value.into_value()));
        }
    }
}

fn emit_image_sample(
    ir: &mut IrEmitter,
    inst: &GcnInst,
    tsharp: ScalarReg,
    sampler: ScalarReg,
    addr: VectorReg,
    gather: bool,
    pc: u32,
) -> Value {
    let mimg = inst.control.mimg();
    let flags = MimgModifierFlags::from_bits(mimg.modifiers() as u32);
    let mut info = TextureInstInfo::default();
    info.set_is_depth(u32::from(flags.test(MimgModifier::Pcf)));
    info.set_has_bias(u32::from(flags.test(MimgModifier::LodBias)));
    info.set_has_lod_clamp(u32::from(flags.test(MimgModifier::LodClamp)));
    info.set_force_level0(u32::from(flags.test(MimgModifier::Level0)));
    info.set_has_offset(u32::from(flags.test(MimgModifier::Offset)));
    info.set_has_lod(u32::from(flags.any(MimgModifier::Lod)));
    info.set_is_array(u32::from(mimg.da()));
    info.set_is_unnormalized(u32::from(mimg.unrm()));
    info.set_is_r128(u32::from(mimg.r128()));
    info.set_pc(pc);
    if gather {
        let width = 64 - mimg.dmask().leading_zeros();
        info.set_gather_comp(width - 1);
        info.set_is_gather(1);
    } else {
        info.set_has_derivatives(u32::from(flags.test(MimgModifier::Derivative)));
    }

    let t0 = ir.get_scalar_reg(tsharp);
    let t1 = ir.get_scalar_reg(ScalarReg::from_raw(tsharp as u32 + 1));
    let t2 = ir.get_scalar_reg(ScalarReg::from_raw(tsharp as u32 + 2));
    let t3 = ir.get_scalar_reg(ScalarReg::from_raw(tsharp as u32 + 3));
    let low = ir.composite_construct4(t0, t1, t2, t3);
    let high = if mimg.r128() {
        Value::empty()
    } else {
        let t4 = ir.get_scalar_reg(ScalarReg::from_raw(tsharp as u32 + 4));
        let t5 = ir.get_scalar_reg(ScalarReg::from_raw(tsharp as u32 + 5));
        let t6 = ir.get_scalar_reg(ScalarReg::from_raw(tsharp as u32 + 6));
        let t7 = ir.get_scalar_reg(ScalarReg::from_raw(tsharp as u32 + 7));
        ir.composite_construct4(t4, t5, t6, t7)
    };
    let image_handle = ir.image_handle(&low, &high);
    let s0 = ir.get_scalar_reg(sampler);
    let s1 = ir.get_scalar_reg(ScalarReg::from_raw(sampler as u32 + 1));
    let s2 = ir.get_scalar_reg(ScalarReg::from_raw(sampler as u32 + 2));
    let s3 = ir.get_scalar_reg(ScalarReg::from_raw(sampler as u32 + 3));
    let sampler_handle = ir.composite_construct4(s0, s1, s2, s3);

    let mut num_addr_regs = 4;
    if flags.test(MimgModifier::Offset) {
        num_addr_regs += 1;
    }
    if flags.test(MimgModifier::LodBias) {
        num_addr_regs += 1;
    }
    if flags.test(MimgModifier::Pcf) {
        num_addr_regs += 1;
    }
    if flags.test(MimgModifier::Derivative) {
        num_addr_regs += 6;
    }
    let get_addr_reg = |ir: &mut IrEmitter, index: i32| -> F32 {
        if index >= num_addr_regs {
            return ir.imm32_float(0.0);
        }
        let reg = reg_add(addr, index).expect("the address vgprs fit");
        ir.get_vector_reg_f32(reg)
    };
    let a0 = get_addr_reg(ir, 0);
    let a1 = get_addr_reg(ir, 1);
    let a2 = get_addr_reg(ir, 2);
    let a3 = get_addr_reg(ir, 3);
    let address1 = ir.composite_construct4(a0, a1, a2, a3);
    let a4 = get_addr_reg(ir, 4);
    let a5 = get_addr_reg(ir, 5);
    let a6 = get_addr_reg(ir, 6);
    let a7 = get_addr_reg(ir, 7);
    let address2 = ir.composite_construct4(a4, a5, a6, a7);
    let a8 = get_addr_reg(ir, 8);
    let a9 = get_addr_reg(ir, 9);
    let a10 = get_addr_reg(ir, 10);
    let a11 = get_addr_reg(ir, 11);
    let address3 = ir.composite_construct4(a8, a9, a10, a11);
    let address4 = get_addr_reg(ir, 12);

    let mut texel = ir.image_sample_raw(
        &image_handle,
        &sampler_handle,
        &address1,
        &address2,
        &address3,
        &address4.into_value(),
        info,
    );
    if flags.test(MimgModifier::Pcf) && !gather {
        texel = ir.composite_extract(&texel, 0);
    }
    texel
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::frontend::instruction::{InstControl, InstOperand};
    use crate::gpu::shader_recompiler::frontend::opcodes::OperandField;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::{Block, BlockRef};
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
    fn a_dword_buffer_load_reads_and_stores_a_vgpr() {
        let read = |_| 0u32;
        let (mut translator, block) = setup(&read);
        let mut inst = GcnInst {
            opcode: Opcode::BUFFER_LOAD_DWORD,
            ..Default::default()
        };
        inst.control = InstControl::from_raw(4);
        inst.src[0] = operand(OperandField::VectorGPR, 0);
        inst.src[1] = operand(OperandField::VectorGPR, 1);
        inst.src[2] = operand(OperandField::ScalarGPR, 0);
        inst.src[3] = operand(OperandField::LiteralConst, 0);
        translator.emit_vector_memory(&inst);

        let ops = opcodes(&block);
        assert!(ops.contains(&IrOpcode::LoadBufferU32));
        let store = block
            .borrow()
            .instructions()
            .iter()
            .find(|inst| inst.borrow().get_opcode() == IrOpcode::SetVectorRegister)
            .expect("the destination store")
            .clone();
        assert_eq!(store.borrow().arg(0).vector_reg(), VectorReg::V1);
    }

    #[test]
    fn a_typed_buffer_store_pads_to_four_components() {
        let read = |_| 0u32;
        let (mut translator, block) = setup(&read);
        let mut inst = GcnInst {
            opcode: Opcode::TBUFFER_STORE_FORMAT_XY,
            ..Default::default()
        };
        inst.control = InstControl::from_raw((13 << 19) | (4 << 23));
        inst.src[0] = operand(OperandField::VectorGPR, 0);
        inst.src[1] = operand(OperandField::VectorGPR, 1);
        inst.src[2] = operand(OperandField::ScalarGPR, 0);
        inst.src[3] = operand(OperandField::LiteralConst, 0);
        translator.emit_vector_memory(&inst);

        let ops = opcodes(&block);
        assert!(ops.contains(&IrOpcode::StoreBufferFormatF32));
        assert!(ops.contains(&IrOpcode::CompositeConstructF32x4));
    }

    #[test]
    fn an_image_load_writes_the_masked_components() {
        let read = |_| 0u32;
        let (mut translator, block) = setup(&read);
        let mut inst = GcnInst {
            opcode: Opcode::IMAGE_LOAD,
            ..Default::default()
        };
        inst.control = InstControl::from_raw(3 << 8);
        inst.src[0] = operand(OperandField::VectorGPR, 0);
        inst.src[2] = operand(OperandField::ScalarGPR, 0);
        inst.dst[0] = operand(OperandField::VectorGPR, 2);
        translator.emit_vector_memory(&inst);

        let ops = opcodes(&block);
        assert!(ops.contains(&IrOpcode::ImageRead));
        assert_eq!(
            ops.iter()
                .filter(|op| **op == IrOpcode::SetVectorRegister)
                .count(),
            2
        );
    }

    #[test]
    fn an_image_sample_uses_the_raw_sample_instruction() {
        let read = |_| 0u32;
        let (mut translator, block) = setup(&read);
        let mut inst = GcnInst {
            opcode: Opcode::IMAGE_SAMPLE,
            ..Default::default()
        };
        inst.control = InstControl::from_raw(0xF << 8);
        inst.src[0] = operand(OperandField::VectorGPR, 0);
        inst.src[2] = operand(OperandField::ScalarGPR, 0);
        inst.src[3] = operand(OperandField::ScalarGPR, 8);
        inst.dst[0] = operand(OperandField::VectorGPR, 4);
        translator.emit_vector_memory(&inst);

        assert!(opcodes(&block).contains(&IrOpcode::ImageSampleRaw));
    }

    #[test]
    fn resinfo_writes_the_requested_components() {
        let read = |_| 0u32;
        let (mut translator, block) = setup(&read);
        let mut inst = GcnInst {
            opcode: Opcode::IMAGE_GET_RESINFO,
            ..Default::default()
        };
        inst.control = InstControl::from_raw(1 << 8);
        inst.src[0] = operand(OperandField::VectorGPR, 0);
        inst.src[2] = operand(OperandField::ScalarGPR, 0);
        inst.dst[0] = operand(OperandField::VectorGPR, 2);
        translator.emit_vector_memory(&inst);

        let ops = opcodes(&block);
        assert!(ops.contains(&IrOpcode::ImageQueryDimensions));
        assert_eq!(
            ops.iter()
                .filter(|op| **op == IrOpcode::SetVectorRegister)
                .count(),
            1
        );
    }
}
