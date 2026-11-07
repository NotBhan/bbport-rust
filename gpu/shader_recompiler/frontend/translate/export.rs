//! Export translation, ported from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/translate/export.cpp`.

use super::translator::Translator;
use crate::gpu::amdgpu::pixel_format::{DataFormat, NumberFormat};
use crate::gpu::amdgpu::regs_shader::ShaderExportFormat;
use crate::gpu::shader_recompiler::frontend::instruction::GcnInst;
use crate::gpu::shader_recompiler::ir::attribute::{is_mrt, is_position, Attribute};
use crate::gpu::shader_recompiler::ir::position::export_position;
use crate::gpu::shader_recompiler::ir::reg::{RegType as _, VectorReg};
use crate::gpu::shader_recompiler::ir::reinterpret::apply_write_number_conversion;
use crate::gpu::shader_recompiler::ir::value::{F32, F32F64};
use crate::gpu::shader_recompiler::runtime_info::{HwInfo, HwStage, SwInfo, SwStage};

/// `NumberFormatCompressed`.
fn number_format_compressed(export_format: ShaderExportFormat) -> NumberFormat {
    match export_format {
        ShaderExportFormat::ABGR_FP16 => NumberFormat::Float,
        ShaderExportFormat::ABGR_UNORM16 => NumberFormat::Unorm,
        ShaderExportFormat::ABGR_SNORM16 => NumberFormat::Snorm,
        ShaderExportFormat::ABGR_UINT16 => NumberFormat::Uint,
        ShaderExportFormat::ABGR_SINT16 => NumberFormat::Sint,
        other => panic!("Unimplemented compressed export format {}", other.raw()),
    }
}

/// `MaskFromExportFormat`.
fn mask_from_export_format(mask: u8, export_format: ShaderExportFormat) -> u32 {
    match export_format {
        ShaderExportFormat::R_32 => (mask & 1) as u32,
        ShaderExportFormat::GR_32 => (mask & 3) as u32,
        ShaderExportFormat::AR_32 => (mask & 9) as u32,
        ShaderExportFormat::ABGR_32 => mask as u32,
        other => panic!("Unimplemented uncompressed export format {}", other.raw()),
    }
}

impl Translator<'_> {
    /// `ExportRenderTarget`.
    fn export_render_target(&mut self, inst: &GcnInst) {
        let exp = inst.control.exp();
        let mrt = Attribute::from_raw(exp.target()).expect("the export target names an attribute");
        self.info.borrow_mut().persistent.mrt_mask |= 1u8 << (mrt as u64 as u8);

        let HwInfo::Fs(fs) = &self.runtime_info.hw else {
            panic!("the fragment stage has fragment info");
        };
        let mut color_buffer_idx = (mrt as u64 as u32) - (Attribute::RenderTarget0 as u64 as u32);
        if fs.dual_source_blending && mrt == Attribute::RenderTarget1 {
            color_buffer_idx = 0;
        }
        let color_buffer = fs.color_buffers[color_buffer_idx as usize];
        if color_buffer.export_format == ShaderExportFormat::Zero || exp.en() == 0 {
            return;
        }

        let mut components: [F32; 4] = std::array::from_fn(|_| F32::empty());
        if exp.compr() {
            let num_format = number_format_compressed(color_buffer.export_format);
            if exp.en() & 1 != 0 {
                let raw = self
                    .ir
                    .get_vector_reg(VectorReg::from_raw(inst.src[0].code));
                let unpacked = self.ir.unpack_2x16(num_format, &raw);
                components[0] = F32::new(self.ir.composite_extract(&unpacked, 0));
                components[1] = F32::new(self.ir.composite_extract(&unpacked, 1));
            }
            if (exp.en() >> 2) & 1 != 0 {
                let raw = self
                    .ir
                    .get_vector_reg(VectorReg::from_raw(inst.src[1].code));
                let unpacked = self.ir.unpack_2x16(num_format, &raw);
                components[2] = F32::new(self.ir.composite_extract(&unpacked, 0));
                components[3] = F32::new(self.ir.composite_extract(&unpacked, 1));
            }
        } else {
            let mut mask = mask_from_export_format(exp.en() as u8, color_buffer.export_format);
            for (index, component) in components.iter_mut().enumerate() {
                if mask & 1 != 0 {
                    let reg = VectorReg::from_raw(inst.src[index].code);
                    *component = self.ir.get_vector_reg_f32(reg);
                }
                mask >>= 1;
            }
        }

        let needs_unorm_fixup = self.profile.needs_unorm_fixup
            && matches!(
                color_buffer.num_format,
                NumberFormat::Unorm | NumberFormat::Snorm | NumberFormat::Srgb
            )
            && matches!(
                color_buffer.data_format,
                DataFormat::Format8 | DataFormat::Format8_8 | DataFormat::Format8_8_8_8
            );

        for index in 0..4u32 {
            let swizzled = components[color_buffer.swizzle.map(index) as usize].clone();
            if swizzled.value().is_empty() {
                continue;
            }
            let mut converted =
                apply_write_number_conversion(&mut self.ir, &swizzled, color_buffer.num_conversion);
            if needs_unorm_fixup {
                let bias = self.ir.imm32_float(1.0 / 127500.0);
                converted = F32::new(
                    self.ir
                        .fp_sub(
                            &F32F64::new(converted.value().clone()),
                            &F32F64::new(bias.value().clone()),
                        )
                        .into_value(),
                );
            }
            self.ir.set_attribute(mrt, &converted, index);
        }
    }

    /// `ExportDepth`.
    fn export_depth(&mut self, inst: &GcnInst) {
        let exp = inst.control.exp();
        if exp.en() == 0 {
            return;
        }
        let HwInfo::Fs(fs) = &self.runtime_info.hw else {
            panic!("the fragment stage has fragment info");
        };

        let mut components: [F32; 4] = std::array::from_fn(|_| F32::empty());
        if exp.compr() {
            let num_format = number_format_compressed(fs.z_export_format);
            if exp.en() & 1 != 0 {
                let raw = self
                    .ir
                    .get_vector_reg(VectorReg::from_raw(inst.src[0].code));
                let unpacked = self.ir.unpack_2x16(num_format, &raw);
                components[0] = F32::new(self.ir.composite_extract(&unpacked, 0));
                components[1] = F32::new(self.ir.composite_extract(&unpacked, 1));
            }
            if (exp.en() >> 2) & 1 != 0 {
                let raw = self
                    .ir
                    .get_vector_reg(VectorReg::from_raw(inst.src[1].code));
                let unpacked = self.ir.unpack_2x16(num_format, &raw);
                components[2] = F32::new(self.ir.composite_extract(&unpacked, 0));
            }
        } else {
            let mut mask =
                mask_from_export_format(exp.en() as u8 & fs.mrtz_mask, fs.z_export_format);
            for (index, component) in components.iter_mut().enumerate() {
                if mask & 1 != 0 {
                    let reg = VectorReg::from_raw(inst.src[index].code);
                    *component = self.ir.get_vector_reg_f32(reg);
                }
                mask >>= 1;
            }
        }

        const MRTZ_BUILTINS: [Attribute; 4] = [
            Attribute::Depth,
            Attribute::StencilRef,
            Attribute::SampleMask,
            Attribute::Null,
        ];
        for (index, component) in components.iter().enumerate() {
            if component.value().is_empty() {
                continue;
            }
            self.ir.set_attribute(MRTZ_BUILTINS[index], component, 0);
        }
    }

    /// `EmitExport`.
    pub(crate) fn emit_export(&mut self, inst: &GcnInst) {
        let exp = inst.control.exp();
        if self.info.borrow().persistent.hw_stage == HwStage::Fragment && exp.vm() {
            let exec = self.ir.get_exec();
            let not_exec = self.ir.logical_not(&exec);
            self.ir.discard_cond(&not_exec);
        }

        let attrib =
            Attribute::from_raw(exp.target()).expect("the export target names an attribute");
        if is_mrt(attrib) {
            return self.export_render_target(inst);
        }
        if attrib == Attribute::Depth {
            return self.export_depth(inst);
        }

        assert!(
            !exp.compr(),
            "Compressed exports only supported for render targets"
        );

        let tess_emulated_primitive = self.info.borrow().persistent.sw_stage == SwStage::Vertex
            && match &self.runtime_info.sw {
                SwInfo::Vs(vs) => vs.tess_emulated_primitive,
                _ => false,
            };

        let mut mask = exp.en() as u32;
        for index in 0..4u32 {
            if mask & 1 == 0 {
                mask >>= 1;
                continue;
            }
            let reg = VectorReg::from_raw(inst.src[index as usize].code);
            let value = self.ir.get_vector_reg_f32(reg);
            if is_position(attrib) {
                match &self.runtime_info.hw {
                    HwInfo::Vs(vs) => export_position(
                        &mut self.ir,
                        vs,
                        tess_emulated_primitive,
                        attrib,
                        index,
                        &value,
                    ),
                    HwInfo::Gs(gs) => export_position(
                        &mut self.ir,
                        gs,
                        tess_emulated_primitive,
                        attrib,
                        index,
                        &value,
                    ),
                    _ => panic!("the stage has no position outputs"),
                }
            } else {
                self.ir.set_attribute(attrib, &value, index);
            }
            mask >>= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::amdgpu::pixel_format::{CompMapping, NumberConversion};
    use crate::gpu::amdgpu::regs_shader::ShaderExportFormat;
    use crate::gpu::shader_recompiler::frontend::opcodes::Opcode;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::Block;
    use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
    use crate::gpu::shader_recompiler::ir::opcodes::Opcode as IrOpcode;
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::profile::Profile;
    use crate::gpu::shader_recompiler::runtime_info::{
        HwFragmentRuntimeInfo, PsColorBuffer, RuntimeInfo,
    };
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn a_render_target_export_writes_each_enabled_component() {
        let info = Rc::new(RefCell::new(Info::new(
            HwStage::Fragment,
            SwStage::Fragment,
            ShaderParams::new([0; 16], &[], 0),
        )));
        let mut runtime = RuntimeInfo::new(HwStage::Fragment, SwStage::Fragment);
        let mut fs = HwFragmentRuntimeInfo::zeroed();
        fs.color_buffers[0] = PsColorBuffer {
            data_format: DataFormat::Format32_32_32_32,
            num_format: NumberFormat::Float,
            num_conversion: NumberConversion::None,
            export_format: ShaderExportFormat::ABGR_32,
            blend_self_scale: false,
            swizzle: CompMapping::IDENTITY,
        };
        runtime.hw = HwInfo::Fs(fs);
        let read = |_| 0u32;
        let mut translator = Translator::new(info, &runtime, &Profile::default(), &read);
        let block = Block::new_ref();
        translator.ir = IrEmitter::new(&block);

        let mut inst = GcnInst {
            opcode: Opcode::EXP,
            ..Default::default()
        };
        inst.control =
            crate::gpu::shader_recompiler::frontend::instruction::InstControl::from_raw(0xF);
        for index in 0..4 {
            inst.src[index].code = index as u32;
        }
        translator.emit_export(&inst);

        let stores: Vec<_> = block
            .borrow()
            .instructions()
            .iter()
            .filter(|inst| inst.borrow().get_opcode() == IrOpcode::SetAttribute)
            .cloned()
            .collect();
        assert_eq!(stores.len(), 4);
        assert!(stores
            .iter()
            .all(|store| store.borrow().arg(0).attribute() == Attribute::RenderTarget0));
    }
}
