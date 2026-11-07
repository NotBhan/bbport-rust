//! Vector interpolation, ported from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/translate/vector_interpolation.cpp`.

use super::translator::Translator;
use crate::gpu::shader_recompiler::frontend::instruction::GcnInst;
use crate::gpu::shader_recompiler::frontend::opcodes::Opcode;
use crate::gpu::shader_recompiler::info::{Interpolation, Qualifier};
use crate::gpu::shader_recompiler::ir::attribute::{attr_add, name_of, Attribute};
use crate::gpu::shader_recompiler::ir::value::{F32, F32F64, U32F32};
use crate::gpu::shader_recompiler::runtime_info::HwInfo;

fn get_interpolation(attribute: Attribute) -> Interpolation {
    match attribute {
        Attribute::BaryCoordNoPersp => Interpolation {
            primary: Qualifier::NoPerspective,
            auxiliary: Qualifier::None,
        },
        Attribute::BaryCoordNoPerspCentroid => Interpolation {
            primary: Qualifier::NoPerspective,
            auxiliary: Qualifier::Centroid,
        },
        Attribute::BaryCoordNoPerspSample => Interpolation {
            primary: Qualifier::NoPerspective,
            auxiliary: Qualifier::Sample,
        },
        Attribute::BaryCoordSmooth => Interpolation {
            primary: Qualifier::Smooth,
            auxiliary: Qualifier::None,
        },
        Attribute::BaryCoordSmoothCentroid => Interpolation {
            primary: Qualifier::Smooth,
            auxiliary: Qualifier::Centroid,
        },
        Attribute::BaryCoordSmoothSample => Interpolation {
            primary: Qualifier::Smooth,
            auxiliary: Qualifier::Sample,
        },
        Attribute::BaryCoordPullModel => Interpolation {
            primary: Qualifier::Smooth,
            auxiliary: Qualifier::None,
        },
        other => panic!("Unhandled barycentric attribute {}", name_of(other)),
    }
}

const DEFAULT_VAL_TABLE: [[f32; 4]; 4] = [
    [0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
    [1.0, 1.0, 1.0, 0.0],
    [1.0, 1.0, 1.0, 1.0],
];

fn fp_sub(
    ir: &mut crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter,
    a: &F32,
    b: &F32,
) -> F32 {
    F32::new(
        ir.fp_sub(
            &F32F64::new(a.value().clone()),
            &F32F64::new(b.value().clone()),
        )
        .into_value(),
    )
}

fn fp_fma(
    ir: &mut crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter,
    a: &F32,
    b: &F32,
    c: &F32,
) -> F32 {
    F32::new(
        ir.fp_fma(
            &F32F64::new(a.value().clone()),
            &F32F64::new(b.value().clone()),
            &F32F64::new(c.value().clone()),
        )
        .into_value(),
    )
}

impl Translator<'_> {
    /// `EmitVectorInterpolation`.
    pub(crate) fn emit_vector_interpolation(&mut self, inst: &GcnInst) {
        match inst.opcode {
            Opcode::V_INTERP_P1_F32 => self.v_interp_p1_f32(inst),
            Opcode::V_INTERP_P2_F32 => self.v_interp_p2_f32(inst),
            Opcode::V_INTERP_MOV_F32 => self.v_interp_mov_f32(inst),
            _ => self.log_missing_opcode(inst),
        }
    }

    /// `V_INTERP_P1_F32`.
    fn v_interp_p1_f32(&mut self, inst: &GcnInst) {
        if !self.profile.needs_manual_interpolation {
            return;
        }
        let vintrp = inst.control.vintrp();
        let attr_index = vintrp.attr();
        let HwInfo::Fs(fs) = &self.runtime_info.hw else {
            panic!("fragment interpolation needs fragment info");
        };
        let attr = fs.inputs[attr_index as usize];
        if attr.is_default() {
            return;
        }
        let attrib =
            attr_add(Attribute::Param0, attr_index as i32).expect("the attribute is in range");
        let channel = vintrp.chan();
        let p0 = self.ir.get_attribute(attrib, channel, 0);
        let p1 = self.ir.get_attribute(attrib, channel, 1);
        let i = self.get_src_f32(&inst.src[0]);
        let delta = fp_sub(&mut self.ir, &p1, &p0);
        let result = fp_fma(&mut self.ir, &delta, &i, &p0);
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    /// `V_INTERP_P2_F32`.
    fn v_interp_p2_f32(&mut self, inst: &GcnInst) {
        let vintrp = inst.control.vintrp();
        let attr_index = vintrp.attr();
        let attrib =
            attr_add(Attribute::Param0, attr_index as i32).expect("the attribute is in range");
        let HwInfo::Fs(fs) = &self.runtime_info.hw else {
            panic!("fragment interpolation needs fragment info");
        };
        let attr = fs.inputs[attr_index as usize];
        if attr.is_default() {
            let value = self.ir.imm32_float(
                DEFAULT_VAL_TABLE[attr.default_value as usize][vintrp.chan() as usize],
            );
            self.set_dst(&inst.dst[0], &U32F32::new(value.into_value()));
            return;
        }
        assert!(!attr.is_flat);
        if !self.profile.needs_manual_interpolation {
            let interpolation = get_interpolation(self.vgpr_to_interp[inst.src[0].code as usize]);
            self.info.borrow_mut().fs_interpolation[attr_index as usize] = interpolation;
            let value = self.ir.get_attribute(attrib, vintrp.chan(), 0);
            self.set_dst(&inst.dst[0], &U32F32::new(value.into_value()));
            return;
        }
        let channel = vintrp.chan();
        let p0 = self.ir.get_attribute(attrib, channel, 0);
        let p2 = self.ir.get_attribute(attrib, channel, 2);
        let j = self.get_src_f32(&inst.src[0]);
        let dst = self.get_src_f32(&inst.dst[0]);
        let delta = fp_sub(&mut self.ir, &p2, &p0);
        let result = fp_fma(&mut self.ir, &delta, &j, &dst);
        self.info.borrow_mut().fs_interpolation[attr_index as usize].primary = Qualifier::PerVertex;
        self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
    }

    /// `V_INTERP_MOV_F32`.
    fn v_interp_mov_f32(&mut self, inst: &GcnInst) {
        let vintrp = inst.control.vintrp();
        let attr_index = vintrp.attr();
        let attrib =
            attr_add(Attribute::Param0, attr_index as i32).expect("the attribute is in range");
        let HwInfo::Fs(fs) = &self.runtime_info.hw else {
            panic!("fragment interpolation needs fragment info");
        };
        let attr = fs.inputs[attr_index as usize];
        let src_select = inst.src[0].code;
        assert!(
            src_select < 3,
            "Invalid V_INTERP_MOV_F32 selector {}",
            src_select
        );
        assert!(
            attr.is_flat
                || src_select == 2
                || self.profile.supports_amd_shader_explicit_vertex_parameter
                || self.profile.supports_fragment_shader_barycentric
        );
        if self.profile.supports_amd_shader_explicit_vertex_parameter
            || self.profile.supports_fragment_shader_barycentric
        {
            self.info.borrow_mut().fs_interpolation[attr_index as usize].primary =
                Qualifier::PerVertex;
            let channel = vintrp.chan();
            let mut result = self.ir.get_attribute(attrib, channel, (src_select + 1) % 3);
            if src_select != 2 && !attr.is_passthrough() {
                let p0 = self.ir.get_attribute(attrib, channel, 0);
                result = fp_sub(&mut self.ir, &result, &p0);
            }
            self.set_dst(&inst.dst[0], &U32F32::new(result.into_value()));
        } else {
            self.info.borrow_mut().fs_interpolation[attr_index as usize].primary = Qualifier::Flat;
            let value = self.ir.get_attribute(attrib, vintrp.chan(), 0);
            self.set_dst(&inst.dst[0], &U32F32::new(value.into_value()));
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
    use crate::gpu::shader_recompiler::runtime_info::{
        HwFragmentRuntimeInfo, HwStage, PsInput, RuntimeInfo, SwStage,
    };
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn a_flat_passthrough_input_reads_the_attribute_directly() {
        let info = Rc::new(RefCell::new(Info::new(
            HwStage::Fragment,
            SwStage::Fragment,
            ShaderParams::new([0; 16], &[], 0),
        )));
        let mut runtime = RuntimeInfo::new(HwStage::Fragment, SwStage::Fragment);
        let mut fs = HwFragmentRuntimeInfo::zeroed();
        fs.num_inputs = 1;
        fs.inputs[0] = PsInput {
            param_index: 0,
            is_default: true,
            is_flat: true,
            default_value: 0,
        };
        runtime.hw = HwInfo::Fs(fs);
        let read = |_| 0u32;
        let mut translator = Translator::new(info, &runtime, &Profile::default(), &read);
        let block = Block::new_ref();
        translator.ir = IrEmitter::new(&block);

        let mut inst = GcnInst {
            opcode: Opcode::V_INTERP_MOV_F32,
            ..Default::default()
        };
        inst.control =
            crate::gpu::shader_recompiler::frontend::instruction::InstControl::from_raw(1 << 8);
        inst.src[0].code = 2;
        inst.dst[0] = crate::gpu::shader_recompiler::frontend::instruction::InstOperand {
            field: crate::gpu::shader_recompiler::frontend::opcodes::OperandField::VectorGPR.0,
            code: 4,
            ..Default::default()
        };
        translator.emit_vector_interpolation(&inst);

        assert_eq!(
            translator.info.borrow().fs_interpolation[0].primary,
            Qualifier::Flat
        );
        assert!(block
            .borrow()
            .instructions()
            .iter()
            .any(|inst| inst.borrow().get_opcode() == IrOpcode::GetAttribute));
    }
}
