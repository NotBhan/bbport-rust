//! Scalar flow control, ported from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/translate/scalar_flow.cpp`.

use super::translator::Translator;
use crate::gpu::shader_recompiler::frontend::instruction::GcnInst;
use crate::gpu::shader_recompiler::frontend::opcodes::{GsOp, Message, Opcode, SendMsgSimm};
use crate::gpu::shader_recompiler::ir::reg::{RegType as _, ScalarReg};
use crate::gpu::shader_recompiler::ir::value::U32F32;

impl Translator<'_> {
    /// `EmitFlowControl`.
    pub(crate) fn emit_flow_control(&mut self, inst: &GcnInst) {
        match inst.opcode {
            Opcode::S_BARRIER => self.ir.barrier(),
            Opcode::S_TTRACEDATA | Opcode::S_SETPRIO | Opcode::S_TRAP => {}
            Opcode::S_GETPC_B64 => self.s_getpc_b64(inst),
            Opcode::S_SETPC_B64
            | Opcode::S_WAITCNT
            | Opcode::S_NOP
            | Opcode::S_ENDPGM
            | Opcode::S_CBRANCH_EXECZ
            | Opcode::S_CBRANCH_SCC0
            | Opcode::S_CBRANCH_SCC1
            | Opcode::S_CBRANCH_VCCNZ
            | Opcode::S_CBRANCH_VCCZ
            | Opcode::S_CBRANCH_EXECNZ
            | Opcode::S_BRANCH => {}
            Opcode::S_SENDMSG => self.s_sendmsg(inst),
            _ => unreachable!("unknown flow-control opcode"),
        }
    }

    /// `S_GETPC_B64`.
    fn s_getpc_b64(&mut self, inst: &GcnInst) {
        let dst = ScalarReg::from_raw(inst.dst[0].code);
        let pc = self.ir.imm32(self.pc);
        let value = self.ir.get_pc_lo(&pc);
        self.ir
            .set_scalar_reg(dst, &U32F32::new(value.into_value()));
        let next = ScalarReg::from_raw(inst.dst[0].code + 1);
        let zero = self.ir.imm32(0);
        self.ir
            .set_scalar_reg(next, &U32F32::new(zero.into_value()));
    }

    /// `S_SENDMSG`.
    fn s_sendmsg(&mut self, inst: &GcnInst) {
        let simm = SendMsgSimm::from_raw(inst.control.sopp().simm as u16 as u32);
        match simm.msg() {
            Message::Gs => match simm.op() {
                GsOp::Nop => {}
                GsOp::Cut => self.ir.emit_primitive(),
                GsOp::Emit => self.ir.emit_vertex(),
                _ => unreachable!("unknown geometry operation"),
            },
            Message::GsDone => {}
            _ => unreachable!("unknown send-message"),
        }
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
    fn getpc_writes_the_current_pc_and_a_zero_high_word() {
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
        translator.pc = 0x1234;

        let mut inst = GcnInst {
            opcode: Opcode::S_GETPC_B64,
            ..Default::default()
        };
        inst.dst[0].code = 2;
        translator.emit_flow_control(&inst);

        let stores: Vec<_> = block
            .borrow()
            .instructions()
            .iter()
            .filter(|inst| inst.borrow().get_opcode() == IrOpcode::SetScalarRegister)
            .cloned()
            .collect();
        assert_eq!(stores.len(), 2);
        assert_eq!(stores[0].borrow().arg(0).scalar_reg(), ScalarReg::S2);
        let pc = stores[0].borrow().arg(1).inst();
        assert_eq!(pc.borrow().get_opcode(), IrOpcode::GetPcLo);
        assert_eq!(pc.borrow().arg(0).u32(), 0x1234);
        assert_eq!(stores[1].borrow().arg(0).scalar_reg(), ScalarReg::S3);
        assert_eq!(stores[1].borrow().arg(1).u32(), 0);
    }
}
