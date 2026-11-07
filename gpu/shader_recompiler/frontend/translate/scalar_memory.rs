//! Scalar memory access, ported from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/translate/scalar_memory.cpp`.

use super::translator::Translator;
use crate::gpu::shader_recompiler::frontend::instruction::GcnInst;
use crate::gpu::shader_recompiler::frontend::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::reg::{BufferInstInfo, RegType as _, ScalarReg};
use crate::gpu::shader_recompiler::ir::value::{U32, U32F32, U32U64};

const SQ_SRC_LITERAL: u32 = 0xFF;

fn wide(value: &U32) -> U32U64 {
    U32U64::new(value.value().clone())
}

impl Translator<'_> {
    /// `EmitScalarMemory`.
    pub(crate) fn emit_scalar_memory(&mut self, inst: &GcnInst) {
        match inst.opcode {
            Opcode::S_LOAD_DWORD => self.s_load_dword(1, inst),
            Opcode::S_LOAD_DWORDX2 => self.s_load_dword(2, inst),
            Opcode::S_LOAD_DWORDX4 => self.s_load_dword(4, inst),
            Opcode::S_LOAD_DWORDX8 => self.s_load_dword(8, inst),
            Opcode::S_LOAD_DWORDX16 => self.s_load_dword(16, inst),
            Opcode::S_BUFFER_LOAD_DWORD => self.s_buffer_load_dword(1, inst),
            Opcode::S_BUFFER_LOAD_DWORDX2 => self.s_buffer_load_dword(2, inst),
            Opcode::S_BUFFER_LOAD_DWORDX4 => self.s_buffer_load_dword(4, inst),
            Opcode::S_BUFFER_LOAD_DWORDX8 => self.s_buffer_load_dword(8, inst),
            Opcode::S_BUFFER_LOAD_DWORDX16 => self.s_buffer_load_dword(16, inst),
            Opcode::S_MEMTIME => self.s_memtime(inst),
            _ => self.log_missing_opcode(inst),
        }
    }

    /// `S_LOAD_DWORD`.
    fn s_load_dword(&mut self, num_dwords: u32, inst: &GcnInst) {
        let smrd = inst.control.smrd();
        let sbase = ScalarReg::from_raw(inst.src[0].code * 2);
        let dword_offset = if smrd.imm() {
            self.ir.imm32(smrd.offset())
        } else if smrd.offset() == SQ_SRC_LITERAL {
            self.ir.imm32(inst.src[1].code)
        } else {
            let offset = self.ir.get_scalar_reg(ScalarReg::from_raw(smrd.offset()));
            let two = self.ir.imm32(2);
            U32::new(
                self.ir
                    .shift_right_logical(&wide(&offset), &two)
                    .into_value(),
            )
        };
        let lo = self.ir.get_scalar_reg(sbase);
        let hi = self
            .ir
            .get_scalar_reg(ScalarReg::from_raw(sbase as u32 + 1));
        let base = self.ir.composite_construct(lo, hi);
        for index in 0..num_dwords {
            let i = self.ir.imm32(index);
            let offset = U32::new(self.ir.i_add(&wide(&dword_offset), &wide(&i)).into_value());
            let value = self.ir.read_const(&base, &offset);
            let dst = ScalarReg::from_raw(inst.dst[0].code + index);
            self.ir
                .set_scalar_reg(dst, &U32F32::new(value.into_value()));
        }
    }

    /// `S_BUFFER_LOAD_DWORD`.
    fn s_buffer_load_dword(&mut self, num_dwords: u32, inst: &GcnInst) {
        let smrd = inst.control.smrd();
        let sbase = ScalarReg::from_raw(inst.src[0].code * 2);
        let dword_offset = if smrd.imm() {
            self.ir.imm32(smrd.offset())
        } else if smrd.offset() == SQ_SRC_LITERAL {
            self.ir.imm32(inst.src[1].code)
        } else {
            let offset = self.ir.get_scalar_reg(ScalarReg::from_raw(smrd.offset()));
            let two = self.ir.imm32(2);
            U32::new(
                self.ir
                    .shift_right_logical(&wide(&offset), &two)
                    .into_value(),
            )
        };

        let mut buffer_info = BufferInstInfo::from_raw(0);
        buffer_info.set_pc(self.pc as u64);

        let s0 = self.ir.get_scalar_reg(sbase);
        let s1 = self
            .ir
            .get_scalar_reg(ScalarReg::from_raw(sbase as u32 + 1));
        let s2 = self
            .ir
            .get_scalar_reg(ScalarReg::from_raw(sbase as u32 + 2));
        let s3 = self
            .ir
            .get_scalar_reg(ScalarReg::from_raw(sbase as u32 + 3));
        let vsharp = self.ir.composite_construct4(s0, s1, s2, s3);
        for index in 0..num_dwords {
            let i = self.ir.imm32(index);
            let offset = U32::new(self.ir.i_add(&wide(&dword_offset), &wide(&i)).into_value());
            let value = self.ir.read_const_buffer(&vsharp, &offset, buffer_info);
            let dst = ScalarReg::from_raw(inst.dst[0].code + index);
            self.ir
                .set_scalar_reg(dst, &U32F32::new(value.into_value()));
        }
    }

    /// `S_MEMTIME`.
    fn s_memtime(&mut self, inst: &GcnInst) {
        let value = self.ir.memtime();
        self.set_dst64(&inst.dst[0], value.into_value());
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

    #[test]
    fn an_immediate_offset_scalar_load_becomes_read_const() {
        let info = Rc::new(RefCell::new(Info::new(
            HwStage::Vertex,
            SwStage::Vertex,
            ShaderParams::new([0; 16], &[], 0),
        )));
        let runtime = RuntimeInfo::new(HwStage::Vertex, SwStage::Vertex);
        let read = |_| 0u32;
        let mut translator = Translator::new(info, &runtime, &Profile::default(), &read);
        let block = Block::new_ref();
        translator.ir = IrEmitter::new(&block);

        let mut inst = GcnInst {
            opcode: Opcode::S_LOAD_DWORD,
            ..Default::default()
        };
        inst.control = crate::gpu::shader_recompiler::frontend::instruction::InstControl::from_raw(
            5 | (1 << 8),
        );
        inst.src[0].code = 0;
        inst.dst[0].code = 3;
        translator.emit_scalar_memory(&inst);

        let instructions = block.borrow().instructions().to_vec();
        assert!(instructions
            .iter()
            .any(|inst| inst.borrow().get_opcode() == IrOpcode::ReadConst));
        let store = instructions
            .iter()
            .find(|inst| inst.borrow().get_opcode() == IrOpcode::SetScalarRegister)
            .expect("the destination store")
            .clone();
        assert_eq!(store.borrow().arg(0).scalar_reg(), ScalarReg::S3);
        assert_eq!(
            store.borrow().arg(1).inst().borrow().get_opcode(),
            IrOpcode::ReadConst
        );
    }
}
