//! The GCN translator core, ported from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/translate/translate.{h,cpp}`.
//!
//! The C++ class keeps one emitter for the block being translated plus the register maps and the
//! stage's runtime information. The port keeps the same state; the category emitters live in
//! sibling modules and are methods on this type.
//!
//! `EmitFetch`'s shader dump is not ported: the port's pre-build translates from a stored fetch
//! shader, with no guest process to dump from.

use crate::gpu::shader_recompiler::frontend::fetch_shader::{
    get_fetch_shader_code, parse_fetch_shader, FetchShaderData,
};
use crate::gpu::shader_recompiler::frontend::instruction::{
    GcnInst, InstOperand, OperandFieldRange,
};
use crate::gpu::shader_recompiler::frontend::opcodes::{InstCategory, OperandField};
use crate::gpu::shader_recompiler::info::Info;
use crate::gpu::shader_recompiler::ir::attribute::{attr_add, Attribute};
use crate::gpu::shader_recompiler::ir::basic_block::BlockRef;
use crate::gpu::shader_recompiler::ir::condition::Condition;
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::r#type::Type;
use crate::gpu::shader_recompiler::ir::reg::{reg_add, RegType as _, ScalarReg, VectorReg};
use crate::gpu::shader_recompiler::ir::reinterpret::{
    apply_read_number_conversion_vec4, apply_swizzle,
};
use crate::gpu::shader_recompiler::ir::value::{
    Value, F16, F16F32F64, F32, F32F64, F64, U1, U32, U32F32, U32U64, U64, U8U16U32U64,
};
use crate::gpu::shader_recompiler::profile::Profile;
use crate::gpu::shader_recompiler::runtime_info::{HwInfo, HwStage, RuntimeInfo, SwInfo, SwStage};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// `Shader::Gcn::ConditionOp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ConditionOp {
    F,
    EQ,
    LG,
    GT,
    GE,
    LT,
    LE,
    TRU,
    U,
}

/// `Shader::Gcn::AtomicOp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AtomicOp {
    Swap,
    CmpSwap,
    Add,
    Sub,
    Smin,
    Umin,
    Smax,
    Umax,
    And,
    Or,
    Xor,
    Inc,
    Dec,
    FCmpSwap,
    Fmin,
    Fmax,
}

/// `Shader::Gcn::NegateMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NegateMode {
    None,
    Src1,
    Result,
}

/// `MaxInterpVgpr`.
pub const MAX_INTERP_VGPR: usize = 16;

fn wide32(value: &U32) -> U32U64 {
    U32U64::new(value.value().clone())
}

fn fp_abs(ir: &mut IrEmitter, value: &F32) -> F32 {
    F32::new(ir.fp_abs(&F32F64::new(value.value().clone())).into_value())
}

fn fp_neg(ir: &mut IrEmitter, value: &F32) -> F32 {
    F32::new(ir.fp_neg(&F32F64::new(value.value().clone())).into_value())
}

fn fp_mul(ir: &mut IrEmitter, a: &F32, b: &F32) -> F32 {
    F32::new(
        ir.fp_mul(
            &F32F64::new(a.value().clone()),
            &F32F64::new(b.value().clone()),
        )
        .into_value(),
    )
}

fn fp_saturate(ir: &mut IrEmitter, value: &F32) -> F32 {
    F32::new(
        ir.fp_saturate(&F32F64::new(value.value().clone()))
            .into_value(),
    )
}

fn fp_abs64(ir: &mut IrEmitter, value: &F64) -> F64 {
    F64::new(ir.fp_abs(&F32F64::new(value.value().clone())).into_value())
}

fn fp_neg64(ir: &mut IrEmitter, value: &F64) -> F64 {
    F64::new(ir.fp_neg(&F32F64::new(value.value().clone())).into_value())
}

fn fp_mul64(ir: &mut IrEmitter, a: &F64, b: &F64) -> F64 {
    F64::new(
        ir.fp_mul(
            &F32F64::new(a.value().clone()),
            &F32F64::new(b.value().clone()),
        )
        .into_value(),
    )
}

fn fp_saturate64(ir: &mut IrEmitter, value: &F64) -> F64 {
    F64::new(
        ir.fp_saturate(&F32F64::new(value.value().clone()))
            .into_value(),
    )
}

/// `IterateBarycentrics`: the fragment-stage barycentric user data and the vreg each lands in.
pub fn iterate_barycentrics(runtime_info: &RuntimeInfo) -> (VectorReg, Vec<(u32, Attribute, u32)>) {
    let mut entries = Vec::new();
    if runtime_info.hw_stage != HwStage::Fragment {
        return (VectorReg::V0, entries);
    }
    let HwInfo::Fs(fs) = &runtime_info.hw else {
        return (VectorReg::V0, entries);
    };
    let flags = fs.addr_flags;
    let mut dst = 0u32;
    let mut add = |dst: &mut u32, attribute: Attribute, comp: u32| {
        entries.push((*dst, attribute, comp));
        *dst += 1;
    };
    if flags.persp_sample_ena() {
        add(&mut dst, Attribute::BaryCoordSmoothSample, 0);
        add(&mut dst, Attribute::BaryCoordSmoothSample, 1);
    }
    if flags.persp_center_ena() {
        add(&mut dst, Attribute::BaryCoordSmooth, 0);
        add(&mut dst, Attribute::BaryCoordSmooth, 1);
    }
    if flags.persp_centroid_ena() {
        add(&mut dst, Attribute::BaryCoordSmoothCentroid, 0);
        add(&mut dst, Attribute::BaryCoordSmoothCentroid, 1);
    }
    if flags.persp_pull_model_ena() {
        add(&mut dst, Attribute::BaryCoordPullModel, 0);
        add(&mut dst, Attribute::BaryCoordPullModel, 1);
        add(&mut dst, Attribute::BaryCoordPullModel, 2);
    }
    if flags.linear_sample_ena() {
        add(&mut dst, Attribute::BaryCoordNoPerspSample, 0);
        add(&mut dst, Attribute::BaryCoordNoPerspSample, 1);
    }
    if flags.linear_center_ena() {
        add(&mut dst, Attribute::BaryCoordNoPersp, 0);
        add(&mut dst, Attribute::BaryCoordNoPersp, 1);
    }
    if flags.linear_centroid_ena() {
        add(&mut dst, Attribute::BaryCoordNoPerspCentroid, 0);
        add(&mut dst, Attribute::BaryCoordNoPerspCentroid, 1);
    }
    if flags.line_stipple_tex_ena() {
        dst += 1;
    }
    (VectorReg::from_raw(dst), entries)
}

/// `UdRegFromShOffset`.
fn ud_reg_from_sh_offset(sgpr_offset: u16, hw_stage: HwStage) -> i32 {
    const INDIRECT_SGPR_OFFSETS: [u32; 6] = [0, 0x4c, 0, 0xcc, 0, 0x14c];
    if sgpr_offset == 0 {
        return -1;
    }
    let ud_reg = (sgpr_offset as u32).wrapping_sub(INDIRECT_SGPR_OFFSETS[hw_stage as usize]);
    assert!(ud_reg < 16, "Out of bounds indirect SGPR copy");
    ud_reg as i32
}

/// `Shader::Gcn::Translator`.
pub struct Translator<'a> {
    pub(crate) ir: IrEmitter,
    pub(crate) info: Rc<RefCell<Info>>,
    pub(crate) runtime_info: RuntimeInfo,
    pub(crate) profile: Profile,
    pub(crate) next_vgpr_num: u32,
    pub(crate) vgpr_map: HashMap<u32, VectorReg>,
    /// Read by the interpolation emitter, which is still pending.
    #[allow(dead_code)]
    pub(crate) vgpr_to_interp: [Attribute; MAX_INTERP_VGPR],
    pub(crate) fetch_data: Option<FetchShaderData>,
    pub(crate) pc: u32,
    pub(crate) read: &'a dyn Fn(u64) -> u32,
}

impl<'a> Translator<'a> {
    /// `Translator(Info&, const RuntimeInfo&, const Profile&)`. The C++ leaves the emitter
    /// unbound until a block is translated; the port starts it on a throwaway block.
    pub fn new(
        info: Rc<RefCell<Info>>,
        runtime_info: &RuntimeInfo,
        profile: &Profile,
        read: &'a dyn Fn(u64) -> u32,
    ) -> Self {
        let next_vgpr_num = runtime_info.props.num_allocated_vgprs;
        let (_, entries) = iterate_barycentrics(runtime_info);
        let mut vgpr_to_interp = [Attribute::default(); MAX_INTERP_VGPR];
        for (vreg, attribute, _) in entries {
            vgpr_to_interp[vreg as usize] = attribute;
        }
        Translator {
            ir: IrEmitter::new(&crate::gpu::shader_recompiler::ir::basic_block::Block::new_ref()),
            info,
            runtime_info: runtime_info.clone(),
            profile: *profile,
            next_vgpr_num,
            vgpr_map: HashMap::new(),
            vgpr_to_interp,
            fetch_data: None,
            pc: 0,
            read,
        }
    }

    /// `GetScratchVgpr`.
    pub fn get_scratch_vgpr(&mut self, offset: u32) -> VectorReg {
        if let Some(reg) = self.vgpr_map.get(&offset) {
            return *reg;
        }
        assert!(self.next_vgpr_num < 256, "Out of VGPRs");
        let reg = VectorReg::from_raw(self.next_vgpr_num);
        self.next_vgpr_num += 1;
        self.vgpr_map.insert(offset, reg);
        reg
    }

    /// `EmitPrologue`.
    pub fn emit_prologue(&mut self, first_block: &BlockRef) {
        self.ir = IrEmitter::at(first_block, 0);
        self.ir.prologue();
        self.ir.set_exec(&U1::new(Value::from_u1(true)));

        let mut dst_sreg = ScalarReg::S0;
        for _ in 0..self.runtime_info.props.num_user_data {
            let value = self.ir.get_user_data(dst_sreg);
            self.ir
                .set_scalar_reg(dst_sreg, &U32F32::new(value.into_value()));
            dst_sreg = reg_add(dst_sreg, 1).expect("the user data registers fit");
        }

        let mut dst_vreg = VectorReg::V0;
        let stage = self.info.borrow().persistent.sw_stage;
        match stage {
            SwStage::Vertex => {
                let vs = match &self.runtime_info.sw {
                    SwInfo::Vs(vs) => *vs,
                    _ => panic!("the vertex stage has vertex info"),
                };
                let hw_stage = self.runtime_info.hw_stage;
                let base_vertex_sgpr = ud_reg_from_sh_offset(vs.vertex_sgpr_offset, hw_stage);
                if base_vertex_sgpr != -1 {
                    let value = self.ir.get_attribute_u32(Attribute::BaseVertex, 0);
                    let reg = ScalarReg::from_raw(base_vertex_sgpr as u32);
                    self.ir
                        .set_scalar_reg(reg, &U32F32::new(value.into_value()));
                }
                let base_instance_sgpr = ud_reg_from_sh_offset(vs.instance_sgpr_offset, hw_stage);
                if base_instance_sgpr != -1 {
                    let value = self.ir.get_attribute_u32(Attribute::BaseInstance, 0);
                    let reg = ScalarReg::from_raw(base_instance_sgpr as u32);
                    self.ir
                        .set_scalar_reg(reg, &U32F32::new(value.into_value()));
                }

                let mut vertex_id = self.ir.get_attribute_u32(Attribute::VertexId, 0);
                if base_vertex_sgpr != -1 {
                    if self
                        .fetch_data
                        .as_ref()
                        .is_none_or(|data| data.vertex_offset_sgpr == -1)
                    {
                        let base = self.ir.get_attribute_u32(Attribute::BaseVertex, 0);
                        vertex_id = U32::new(
                            self.ir
                                .i_sub(&wide32(&vertex_id), &wide32(&base))
                                .into_value(),
                        );
                    } else {
                        assert_eq!(
                            self.fetch_data.as_ref().unwrap().vertex_offset_sgpr,
                            base_vertex_sgpr as i8,
                            "Fetch shader in indirect draw uses wrong base vertex"
                        );
                    }
                }
                self.ir
                    .set_vector_reg(dst_vreg, &U32F32::new(vertex_id.into_value()));
                dst_vreg = reg_add(dst_vreg, 1).expect("the input vgprs fit");

                if self.runtime_info.hw_stage == HwStage::Local {
                    if self.runtime_info.props.num_input_vgprs > 0 {
                        self.ir
                            .set_vector_reg(dst_vreg, &U32F32::new(Value::from_u32(0)));
                        dst_vreg = reg_add(dst_vreg, 1).expect("the input vgprs fit");
                    }
                    if self.runtime_info.props.num_input_vgprs > 1 {
                        dst_vreg = reg_add(dst_vreg, 1).expect("the input vgprs fit");
                    }
                } else {
                    if self.runtime_info.props.num_input_vgprs > 0 {
                        if vs.step_rate_0 != 0 {
                            let instance = self.ir.get_attribute_u32(Attribute::InstanceId, 0);
                            let rate = self.ir.imm32(vs.step_rate_0);
                            let divided = self.ir.i_div(&instance, &rate, false);
                            self.ir
                                .set_vector_reg(dst_vreg, &U32F32::new(divided.into_value()));
                        } else {
                            self.ir
                                .set_vector_reg(dst_vreg, &U32F32::new(Value::from_u32(0)));
                        }
                        dst_vreg = reg_add(dst_vreg, 1).expect("the input vgprs fit");
                    }
                    if self.runtime_info.props.num_input_vgprs > 1 {
                        if vs.step_rate_1 != 0 {
                            let instance = self.ir.get_attribute_u32(Attribute::InstanceId, 0);
                            let rate = self.ir.imm32(vs.step_rate_1);
                            let divided = self.ir.i_div(&instance, &rate, false);
                            self.ir
                                .set_vector_reg(dst_vreg, &U32F32::new(divided.into_value()));
                        } else {
                            self.ir
                                .set_vector_reg(dst_vreg, &U32F32::new(Value::from_u32(0)));
                        }
                        dst_vreg = reg_add(dst_vreg, 1).expect("the input vgprs fit");
                    }
                }

                if self.runtime_info.props.num_input_vgprs > 2 {
                    let mut instance_id = self.ir.get_attribute_u32(Attribute::InstanceId, 0);
                    if base_instance_sgpr != -1 {
                        if self
                            .fetch_data
                            .as_ref()
                            .is_none_or(|data| data.instance_offset_sgpr == -1)
                        {
                            let base = self.ir.get_attribute_u32(Attribute::BaseInstance, 0);
                            instance_id = U32::new(
                                self.ir
                                    .i_sub(&wide32(&instance_id), &wide32(&base))
                                    .into_value(),
                            );
                        } else {
                            assert_eq!(
                                self.fetch_data.as_ref().unwrap().instance_offset_sgpr,
                                base_instance_sgpr as i8,
                                "Fetch shader in indirect draw uses wrong base instance"
                            );
                        }
                    }
                    self.ir
                        .set_vector_reg(dst_vreg, &U32F32::new(instance_id.into_value()));
                }
            }
            SwStage::Fragment => {
                let HwInfo::Fs(fs) = &self.runtime_info.hw else {
                    panic!("the fragment stage has fragment info");
                };
                let fs = *fs;
                let (end, entries) = iterate_barycentrics(&self.runtime_info);
                if self.profile.supports_amd_shader_explicit_vertex_parameter
                    || self.profile.supports_fragment_shader_barycentric
                {
                    for (vreg, attribute, comp) in entries {
                        let value = self.ir.get_attribute(attribute, comp, 0);
                        self.ir.set_vector_reg(
                            VectorReg::from_raw(vreg),
                            &U32F32::new(value.into_value()),
                        );
                    }
                }
                dst_vreg = end;
                let addr_flags = fs.addr_flags;
                let en_flags = fs.en_flags;
                for comp in 0..4u32 {
                    let enabled = match comp {
                        0 => addr_flags.pos_x_float_ena(),
                        1 => addr_flags.pos_y_float_ena(),
                        2 => addr_flags.pos_z_float_ena(),
                        _ => addr_flags.pos_w_float_ena(),
                    };
                    if !enabled {
                        continue;
                    }
                    let present = match comp {
                        0 => en_flags.pos_x_float_ena(),
                        1 => en_flags.pos_y_float_ena(),
                        2 => en_flags.pos_z_float_ena(),
                        _ => en_flags.pos_w_float_ena(),
                    };
                    let value = if present {
                        let coord = self.ir.get_attribute(Attribute::FragCoord, comp, 0);
                        if comp == 3 {
                            F32::new(ir_fp_recip(&mut self.ir, &coord).into_value())
                        } else {
                            coord
                        }
                    } else {
                        self.ir.imm32_float(0.0)
                    };
                    self.ir
                        .set_vector_reg(dst_vreg, &U32F32::new(value.into_value()));
                    dst_vreg = reg_add(dst_vreg, 1).expect("the input vgprs fit");
                }
                if addr_flags.front_face_ena() {
                    if en_flags.front_face_ena() {
                        let front_face = self.ir.get_attribute_u1(Attribute::IsFrontFace, 0);
                        if fs.front_face_all_bits {
                            let one = self.ir.imm32(1);
                            let zero = self.ir.imm32(0);
                            let selected =
                                self.ir
                                    .select(&front_face, &one.into_value(), &zero.into_value());
                            self.ir.set_vector_reg(dst_vreg, &U32F32::new(selected));
                        } else {
                            let one = self.ir.imm32_float(1.0);
                            let minus_one = self.ir.imm32_float(-1.0);
                            let selected = self.ir.select(
                                &front_face,
                                &one.into_value(),
                                &minus_one.into_value(),
                            );
                            self.ir.set_vector_reg(dst_vreg, &U32F32::new(selected));
                        }
                    } else {
                        self.ir
                            .set_vector_reg(dst_vreg, &U32F32::new(Value::from_u32(0)));
                    }
                    dst_vreg = reg_add(dst_vreg, 1).expect("the input vgprs fit");
                }
                if addr_flags.ancillary_ena() {
                    if en_flags.ancillary_ena() {
                        let value = self.ir.get_attribute_u32(Attribute::PackedAncillary, 0);
                        self.ir
                            .set_vector_reg(dst_vreg, &U32F32::new(value.into_value()));
                    } else {
                        self.ir
                            .set_vector_reg(dst_vreg, &U32F32::new(Value::from_u32(0)));
                    }
                    dst_vreg = reg_add(dst_vreg, 1).expect("the input vgprs fit");
                }
                if addr_flags.sample_coverage_ena() {
                    if en_flags.sample_coverage_ena() {
                        let num_samples = fs.num_samples.clamp(1, 16);
                        let mut coverage = self.ir.imm32(1);
                        if num_samples > 1 {
                            let valid_samples = (1u32 << num_samples) - 1;
                            let mask = self.ir.get_attribute_u32(Attribute::SampleMask, 0);
                            let valid = self.ir.imm32(valid_samples);
                            coverage = U32::new(
                                self.ir
                                    .bitwise_and(&wide32(&mask), &wide32(&valid))
                                    .into_value(),
                            );
                        }
                        let is_helper = self.ir.get_attribute_u1(Attribute::IsHelperInvocation, 0);
                        let zero = self.ir.imm32(0);
                        let selected =
                            self.ir
                                .select(&is_helper, &zero.into_value(), &coverage.into_value());
                        self.ir.set_vector_reg(dst_vreg, &U32F32::new(selected));
                    } else {
                        self.ir
                            .set_vector_reg(dst_vreg, &U32F32::new(Value::from_u32(0)));
                    }
                }
            }
            SwStage::TessellationControl => {
                let value = self.ir.get_attribute_u32(Attribute::PrimitiveId, 0);
                self.ir
                    .set_vector_reg(VectorReg::V0, &U32F32::new(value.into_value()));
                let value = self
                    .ir
                    .get_attribute_u32(Attribute::PackedHullInvocationInfo, 0);
                self.ir
                    .set_vector_reg(VectorReg::V1, &U32F32::new(value.into_value()));
                let tcs = match &self.runtime_info.sw {
                    SwInfo::Tcs(tcs) => *tcs,
                    _ => panic!("the tessellation-control stage has its info"),
                };
                if tcs.offchip_lds_enable {
                    let value = self.ir.get_attribute_u32(Attribute::OffChipLdsBase, 0);
                    self.ir
                        .set_scalar_reg(dst_sreg, &U32F32::new(value.into_value()));
                    dst_sreg = reg_add(dst_sreg, 1).expect("the scalar registers fit");
                }
                let value = self
                    .ir
                    .get_attribute_u32(Attribute::TessFactorsBufferBase, 0);
                self.ir
                    .set_scalar_reg(dst_sreg, &U32F32::new(value.into_value()));
            }
            SwStage::TessellationEval => {
                let value = self
                    .ir
                    .get_attribute(Attribute::TessellationEvaluationPointU, 0, 0);
                self.ir
                    .set_vector_reg(VectorReg::V0, &U32F32::new(value.into_value()));
                let value = self
                    .ir
                    .get_attribute(Attribute::TessellationEvaluationPointV, 0, 0);
                self.ir
                    .set_vector_reg(VectorReg::V1, &U32F32::new(value.into_value()));
                self.ir
                    .set_vector_reg(VectorReg::V2, &U32F32::new(Value::from_u32(0)));
                let value = self.ir.get_attribute_u32(Attribute::PrimitiveId, 0);
                self.ir
                    .set_vector_reg(VectorReg::V3, &U32F32::new(value.into_value()));
            }
            SwStage::Compute => {
                for comp in 0..3u32 {
                    let value = self
                        .ir
                        .get_attribute_u32(Attribute::LocalInvocationId, comp);
                    self.ir
                        .set_vector_reg(dst_vreg, &U32F32::new(value.into_value()));
                    dst_vreg = reg_add(dst_vreg, 1).expect("the input vgprs fit");
                }
                let HwInfo::Cs(cs) = &self.runtime_info.hw else {
                    panic!("the compute stage has compute info");
                };
                for comp in 0..3usize {
                    if cs.tgid_enable[comp] {
                        let value = self
                            .ir
                            .get_attribute_u32(Attribute::WorkgroupId, comp as u32);
                        self.ir
                            .set_scalar_reg(dst_sreg, &U32F32::new(value.into_value()));
                        dst_sreg = reg_add(dst_sreg, 1).expect("the scalar registers fit");
                    }
                }
            }
            SwStage::Geometry => {
                let HwInfo::Gs(gs) = &self.runtime_info.hw else {
                    panic!("the geometry stage has geometry info");
                };
                use crate::gpu::amdgpu::regs_vertex::PrimitiveType;
                match gs.in_primitive {
                    PrimitiveType::AdjTriangleList | PrimitiveType::AdjTriangleStrip => {
                        self.ir
                            .set_vector_reg(VectorReg::V6, &U32F32::new(Value::from_u32(5)));
                        self.ir
                            .set_vector_reg(VectorReg::V5, &U32F32::new(Value::from_u32(4)));
                        self.ir
                            .set_vector_reg(VectorReg::V4, &U32F32::new(Value::from_u32(3)));
                        self.ir
                            .set_vector_reg(VectorReg::V3, &U32F32::new(Value::from_u32(2)));
                        self.ir
                            .set_vector_reg(VectorReg::V1, &U32F32::new(Value::from_u32(1)));
                        self.ir
                            .set_vector_reg(VectorReg::V0, &U32F32::new(Value::from_u32(0)));
                    }
                    PrimitiveType::AdjLineList | PrimitiveType::AdjLineStrip => {
                        self.ir
                            .set_vector_reg(VectorReg::V4, &U32F32::new(Value::from_u32(3)));
                        self.ir
                            .set_vector_reg(VectorReg::V3, &U32F32::new(Value::from_u32(2)));
                        self.ir
                            .set_vector_reg(VectorReg::V1, &U32F32::new(Value::from_u32(1)));
                        self.ir
                            .set_vector_reg(VectorReg::V0, &U32F32::new(Value::from_u32(0)));
                    }
                    PrimitiveType::TriangleList
                    | PrimitiveType::TriangleStrip
                    | PrimitiveType::RectList => {
                        self.ir
                            .set_vector_reg(VectorReg::V3, &U32F32::new(Value::from_u32(2)));
                        self.ir
                            .set_vector_reg(VectorReg::V1, &U32F32::new(Value::from_u32(1)));
                        self.ir
                            .set_vector_reg(VectorReg::V0, &U32F32::new(Value::from_u32(0)));
                    }
                    PrimitiveType::LineList | PrimitiveType::LineStrip => {
                        self.ir
                            .set_vector_reg(VectorReg::V1, &U32F32::new(Value::from_u32(1)));
                        self.ir
                            .set_vector_reg(VectorReg::V0, &U32F32::new(Value::from_u32(0)));
                    }
                    _ => {
                        self.ir
                            .set_vector_reg(VectorReg::V0, &U32F32::new(Value::from_u32(0)));
                    }
                }
                let value = self.ir.get_attribute_u32(Attribute::PrimitiveId, 0);
                self.ir
                    .set_vector_reg(VectorReg::V2, &U32F32::new(value.into_value()));
            }
            SwStage::NumLogicalStages => unreachable!("unknown shader stage"),
        }
    }

    /// `GetSrc` for the unsigned 32-bit instantiation.
    pub fn get_src(&mut self, operand: &InstOperand) -> U32 {
        let value = self.get_src_raw(operand, false);
        let value = U32::new(value);
        let value = if operand.input_modifier.abs {
            let mask = self.ir.imm32(0x7fff_ffff);
            U32::new(
                self.ir
                    .bitwise_and(&wide32(&value), &wide32(&mask))
                    .into_value(),
            )
        } else {
            value
        };
        if operand.input_modifier.neg {
            let mask = self.ir.imm32(0x8000_0000);
            U32::new(
                self.ir
                    .bitwise_xor(&wide32(&value), &wide32(&mask))
                    .into_value(),
            )
        } else {
            value
        }
    }

    /// `GetSrc` for the float instantiation.
    pub fn get_src_f32(&mut self, operand: &InstOperand) -> F32 {
        let value = F32::new(self.get_src_raw(operand, true));
        let value = if operand.input_modifier.abs {
            fp_abs(&mut self.ir, &value)
        } else {
            value
        };
        if operand.input_modifier.neg {
            fp_neg(&mut self.ir, &value)
        } else {
            value
        }
    }

    fn get_src_raw(&mut self, operand: &InstOperand, is_float: bool) -> Value {
        let field = operand.field;
        let imm = |ir: &mut IrEmitter, bits: u32| -> Value {
            if is_float {
                Value::from_f32(f32::from_bits(bits))
            } else {
                let _ = ir;
                Value::from_u32(bits)
            }
        };
        if field == OperandField::ScalarGPR.0 {
            let reg = ScalarReg::from_raw(operand.code);
            return if is_float {
                self.ir.get_scalar_reg_f32(reg).into_value()
            } else {
                self.ir.get_scalar_reg(reg).into_value()
            };
        }
        if field == OperandField::VectorGPR.0 {
            let reg = VectorReg::from_raw(operand.code);
            return if is_float {
                self.ir.get_vector_reg_f32(reg).into_value()
            } else {
                self.ir.get_vector_reg(reg).into_value()
            };
        }
        if field == OperandField::ConstZero.0 {
            return imm(&mut self.ir, 0);
        }
        if field == OperandField::SignedConstIntPos.0 {
            let bits = operand.code - OperandFieldRange::SignedConstIntPosMin as u32 + 1;
            return imm(&mut self.ir, bits);
        }
        if field == OperandField::SignedConstIntNeg.0 {
            let bits = (-(operand.code as i32) + OperandFieldRange::SignedConstIntNegMin as i32 - 1)
                as u32;
            return imm(&mut self.ir, bits);
        }
        if field == OperandField::LiteralConst.0 {
            return imm(&mut self.ir, operand.code);
        }
        let float_constant = match field {
            f if f == OperandField::ConstFloatPos_1_0.0 => Some(1.0),
            f if f == OperandField::ConstFloatPos_0_5.0 => Some(0.5),
            f if f == OperandField::ConstFloatPos_2_0.0 => Some(2.0),
            f if f == OperandField::ConstFloatPos_4_0.0 => Some(4.0),
            f if f == OperandField::ConstFloatNeg_0_5.0 => Some(-0.5),
            f if f == OperandField::ConstFloatNeg_1_0.0 => Some(-1.0),
            f if f == OperandField::ConstFloatNeg_2_0.0 => Some(-2.0),
            f if f == OperandField::ConstFloatNeg_4_0.0 => Some(-4.0),
            f if f == OperandField::Inv2Pi.0 => Some(1.0f32 / (2.0f32 * std::f32::consts::PI)),
            _ => None,
        };
        if let Some(value) = float_constant {
            return imm(&mut self.ir, value.to_bits());
        }
        assert_ne!(field, OperandField::Sdwa.0, "unhandled SDWA");
        assert_ne!(field, OperandField::Dpp.0, "unhandled DPP");
        if field == OperandField::VccLo.0
            || field == OperandField::VccHi.0
            || field == OperandField::M0.0
        {
            let value = if field == OperandField::VccLo.0 {
                self.ir.get_vcc_lo()
            } else if field == OperandField::VccHi.0 {
                self.ir.get_vcc_hi()
            } else {
                self.ir.get_m0()
            };
            return if is_float {
                self.ir.bit_cast_f32_u32(&value).into_value()
            } else {
                value.into_value()
            };
        }
        if field == OperandField::Scc.0 {
            assert!(!is_float);
            let scc = self.ir.get_scc();
            let one = self.ir.imm32(1);
            let zero = self.ir.imm32(0);
            let selected = self.ir.select(&scc, &one.into_value(), &zero.into_value());
            return selected;
        }
        if field == OperandField::ExecLo.0 || field == OperandField::ExecHi.0 {
            let exec = self.ir.get_exec();
            let ballot = self.ir.ballot(&exec);
            let unpacked = self.ir.unpack_uint_2x32(&ballot);
            let index = usize::from(field == OperandField::ExecHi.0);
            let value = U32::new(self.ir.composite_extract(&unpacked, index));
            return if is_float {
                self.ir.bit_cast_f32_u32(&value).into_value()
            } else {
                value.into_value()
            };
        }
        panic!("Unexpected operand: {}", field);
    }

    /// `GetSrc16` for unsigned integers.
    pub fn get_src16(&mut self, operand: &InstOperand) -> U32 {
        let value = self.get_src16_raw(operand, false, false);
        let value = U32::new(value);
        let value = if operand.input_modifier.abs {
            let mask = self.ir.imm32(0x7fff_ffff);
            U32::new(
                self.ir
                    .bitwise_and(&wide32(&value), &wide32(&mask))
                    .into_value(),
            )
        } else {
            value
        };
        if operand.input_modifier.neg {
            let mask = self.ir.imm32(0x8000_0000);
            U32::new(
                self.ir
                    .bitwise_xor(&wide32(&value), &wide32(&mask))
                    .into_value(),
            )
        } else {
            value
        }
    }

    /// `GetSrc16` for signed integers.
    pub fn get_src16_signed(&mut self, operand: &InstOperand) -> U32 {
        let value = self.get_src16_raw(operand, false, true);
        let value = U32::new(value);
        let value = if operand.input_modifier.abs {
            let mask = self.ir.imm32(0x7fff_ffff);
            U32::new(
                self.ir
                    .bitwise_and(&wide32(&value), &wide32(&mask))
                    .into_value(),
            )
        } else {
            value
        };
        if operand.input_modifier.neg {
            let mask = self.ir.imm32(0x8000_0000);
            U32::new(
                self.ir
                    .bitwise_xor(&wide32(&value), &wide32(&mask))
                    .into_value(),
            )
        } else {
            value
        }
    }

    /// `GetSrc16` for floats.
    pub fn get_src16_f32(&mut self, operand: &InstOperand) -> F32 {
        let value = F32::new(self.get_src16_raw(operand, true, false));
        let value = if operand.input_modifier.abs {
            fp_abs(&mut self.ir, &value)
        } else {
            value
        };
        if operand.input_modifier.neg {
            fp_neg(&mut self.ir, &value)
        } else {
            value
        }
    }

    fn get_src16_raw(&mut self, operand: &InstOperand, is_float: bool, is_signed: bool) -> Value {
        let field = operand.field;
        let number_format = if is_float {
            crate::gpu::amdgpu::pixel_format::NumberFormat::Float
        } else {
            crate::gpu::amdgpu::pixel_format::NumberFormat::Uint
        };
        let op_sel = operand.op_sel.op_sel;
        let cast = |ir: &mut IrEmitter, value: &F32| -> Value {
            if is_float {
                value.clone().into_value()
            } else {
                let bits = ir.bit_cast_u32_f32(value);
                let off = ir.imm32(0);
                let count = ir.imm32(16);
                ir.bit_field_extract(&bits, &off, &count, is_signed)
                    .into_value()
            }
        };
        let unpack = |ir: &mut IrEmitter, value: &U32| -> F32 {
            let unpacked = ir.unpack_2x16(number_format, value);
            F32::new(ir.composite_extract(&unpacked, usize::from(op_sel)))
        };
        if field == OperandField::ScalarGPR.0 || field == OperandField::VectorGPR.0 {
            let raw = if field == OperandField::ScalarGPR.0 {
                let reg = ScalarReg::from_raw(operand.code);
                if is_float {
                    let f = self.ir.get_scalar_reg_f32(reg);
                    self.ir.bit_cast_u32_f32(&f)
                } else {
                    self.ir.get_scalar_reg(reg)
                }
            } else {
                let reg = VectorReg::from_raw(operand.code);
                if is_float {
                    let f = self.ir.get_vector_reg_f32(reg);
                    self.ir.bit_cast_u32_f32(&f)
                } else {
                    self.ir.get_vector_reg(reg)
                }
            };
            let component = unpack(&mut self.ir, &raw);
            return cast(&mut self.ir, &component);
        }
        let imm = |bits: u32| -> Value {
            if is_float {
                Value::from_f32(f32::from_bits(bits))
            } else {
                Value::from_u32(bits)
            }
        };
        let literal = match field {
            f if f == OperandField::ConstZero.0 => Some(0u32),
            f if f == OperandField::SignedConstIntPos.0 => {
                Some(operand.code - OperandFieldRange::SignedConstIntPosMin as u32 + 1)
            }
            f if f == OperandField::SignedConstIntNeg.0 => Some(
                (-(operand.code as i32) + OperandFieldRange::SignedConstIntNegMin as i32 - 1)
                    as u32,
            ),
            f if f == OperandField::LiteralConst.0 => Some(operand.code),
            f if f == OperandField::ConstFloatPos_1_0.0 => Some(1.0f32.to_bits()),
            f if f == OperandField::ConstFloatPos_0_5.0 => Some(0.5f32.to_bits()),
            f if f == OperandField::ConstFloatPos_2_0.0 => Some(2.0f32.to_bits()),
            f if f == OperandField::ConstFloatPos_4_0.0 => Some(4.0f32.to_bits()),
            f if f == OperandField::ConstFloatNeg_0_5.0 => Some((-0.5f32).to_bits()),
            f if f == OperandField::ConstFloatNeg_1_0.0 => Some((-1.0f32).to_bits()),
            f if f == OperandField::ConstFloatNeg_2_0.0 => Some((-2.0f32).to_bits()),
            f if f == OperandField::ConstFloatNeg_4_0.0 => Some((-4.0f32).to_bits()),
            f if f == OperandField::Inv2Pi.0 => {
                Some((1.0f32 / (2.0f32 * std::f32::consts::PI)).to_bits())
            }
            _ => None,
        };
        if let Some(bits) = literal {
            return imm(bits);
        }
        if field == OperandField::Sdwa.0 || field == OperandField::Dpp.0 {
            return imm(0);
        }
        if field == OperandField::VccLo.0 {
            let raw = self.ir.get_vcc_lo();
            if is_float {
                let unpacked = self.ir.unpack_2x16(number_format, &raw);
                let component = F32::new(self.ir.composite_extract(&unpacked, usize::from(op_sel)));
                return component.into_value();
            }
            let component = unpack(&mut self.ir, &raw);
            return cast(&mut self.ir, &component);
        }
        panic!("Unexpected operand: {}", field);
    }

    /// `GetSrcMix`: the mixed-precision source used by `V_MAD_MIX*`.
    pub fn get_src_mix(&mut self, operand: &InstOperand) -> F32 {
        let field = operand.field;
        let op_sel = operand.op_sel.op_sel;
        let op_sel_hi = operand.op_sel.op_sel_hi;
        let value = if field == OperandField::ScalarGPR.0 || field == OperandField::VectorGPR.0 {
            if !op_sel_hi {
                if field == OperandField::ScalarGPR.0 {
                    let reg = ScalarReg::from_raw(operand.code);
                    self.ir.get_scalar_reg_f32(reg)
                } else {
                    let reg = VectorReg::from_raw(operand.code);
                    self.ir.get_vector_reg_f32(reg)
                }
            } else {
                let raw = if field == OperandField::ScalarGPR.0 {
                    let reg = ScalarReg::from_raw(operand.code);
                    self.ir.get_scalar_reg(reg)
                } else {
                    let reg = VectorReg::from_raw(operand.code);
                    self.ir.get_vector_reg(reg)
                };
                let unpacked = self
                    .ir
                    .unpack_2x16(crate::gpu::amdgpu::pixel_format::NumberFormat::Float, &raw);
                F32::new(self.ir.composite_extract(&unpacked, usize::from(op_sel)))
            }
        } else {
            let bits = if field == OperandField::ConstZero.0 {
                Some(0u32)
            } else if field == OperandField::SignedConstIntPos.0 {
                Some(operand.code - OperandFieldRange::SignedConstIntPosMin as u32 + 1)
            } else if field == OperandField::SignedConstIntNeg.0 {
                Some(
                    (-(operand.code as i32) + OperandFieldRange::SignedConstIntNegMin as i32 - 1)
                        as u32,
                )
            } else if field == OperandField::LiteralConst.0 {
                Some(operand.code)
            } else if field == OperandField::ConstFloatPos_1_0.0 {
                Some(1.0f32.to_bits())
            } else if field == OperandField::ConstFloatPos_0_5.0 {
                Some(0.5f32.to_bits())
            } else if field == OperandField::ConstFloatPos_2_0.0 {
                Some(2.0f32.to_bits())
            } else if field == OperandField::ConstFloatPos_4_0.0 {
                Some(4.0f32.to_bits())
            } else if field == OperandField::ConstFloatNeg_0_5.0 {
                Some((-0.5f32).to_bits())
            } else if field == OperandField::ConstFloatNeg_1_0.0 {
                Some((-1.0f32).to_bits())
            } else if field == OperandField::ConstFloatNeg_2_0.0 {
                Some((-2.0f32).to_bits())
            } else if field == OperandField::ConstFloatNeg_4_0.0 {
                Some((-4.0f32).to_bits())
            } else if field == OperandField::VccLo.0 {
                let raw = self.ir.get_vcc_lo();
                if !op_sel_hi {
                    let value = F32::new(self.ir.bit_cast_f32_u32(&raw).into_value());
                    return finish_mix(self, value, operand);
                }
                let unpacked = self
                    .ir
                    .unpack_2x16(crate::gpu::amdgpu::pixel_format::NumberFormat::Float, &raw);
                let value = F32::new(self.ir.composite_extract(&unpacked, usize::from(op_sel)));
                return finish_mix(self, value, operand);
            } else {
                None
            };
            match bits {
                Some(bits) => F32::new(Value::from_f32(f32::from_bits(bits))),
                None => panic!("Unexpected operand: {}", field),
            }
        };
        let value = if operand.input_modifier.neg_hi {
            fp_abs(&mut self.ir, &value)
        } else {
            value
        };
        if operand.input_modifier.neg {
            fp_neg(&mut self.ir, &value)
        } else {
            value
        }
    }

    /// `GetSrc64` for unsigned integers.
    pub fn get_src64(&mut self, operand: &InstOperand) -> U64 {
        let raw = self.get_src64_raw(operand, false);
        let value = U64::new(raw);
        let value = if operand.input_modifier.abs {
            let unpacked = self.ir.unpack_uint_2x32(&value);
            let lo = U32::new(self.ir.composite_extract(&unpacked, 0));
            let hi = U32::new(self.ir.composite_extract(&unpacked, 1));
            let mask = self.ir.imm32(0x7fff_ffff);
            let hi = U32::new(
                self.ir
                    .bitwise_and(&wide32(&hi), &wide32(&mask))
                    .into_value(),
            );
            let composite = self.ir.composite_construct(lo, hi);
            U64::new(self.ir.pack_uint_2x32(&composite).into_value())
        } else {
            value
        };
        if operand.input_modifier.neg {
            let unpacked = self.ir.unpack_uint_2x32(&value);
            let lo = U32::new(self.ir.composite_extract(&unpacked, 0));
            let hi = U32::new(self.ir.composite_extract(&unpacked, 1));
            let mask = self.ir.imm32(0x8000_0000);
            let hi = U32::new(
                self.ir
                    .bitwise_xor(&wide32(&hi), &wide32(&mask))
                    .into_value(),
            );
            let composite = self.ir.composite_construct(lo, hi);
            U64::new(self.ir.pack_uint_2x32(&composite).into_value())
        } else {
            value
        }
    }

    /// `GetSrc64` for floats.
    pub fn get_src64_f64(&mut self, operand: &InstOperand) -> F64 {
        let value = F64::new(self.get_src64_raw(operand, true));
        let value = if operand.input_modifier.abs {
            fp_abs64(&mut self.ir, &value)
        } else {
            value
        };
        if operand.input_modifier.neg {
            fp_neg64(&mut self.ir, &value)
        } else {
            value
        }
    }

    fn get_src64_raw(&mut self, operand: &InstOperand, is_float: bool) -> Value {
        let field = operand.field;
        let imm = |bits: u64| -> Value {
            if is_float {
                Value::from_f64(f64::from_bits(bits))
            } else {
                Value::from_u64(bits)
            }
        };
        if field == OperandField::ScalarGPR.0 || field == OperandField::VectorGPR.0 {
            let (lo, hi) = if field == OperandField::ScalarGPR.0 {
                let lo = self.ir.get_scalar_reg(ScalarReg::from_raw(operand.code));
                let hi = self
                    .ir
                    .get_scalar_reg(ScalarReg::from_raw(operand.code + 1));
                (lo, hi)
            } else {
                let lo = self.ir.get_vector_reg(VectorReg::from_raw(operand.code));
                let hi = self
                    .ir
                    .get_vector_reg(VectorReg::from_raw(operand.code + 1));
                (lo, hi)
            };
            let composite = self.ir.composite_construct(lo, hi);
            return if is_float {
                self.ir.pack_double_2x32(&composite).into_value()
            } else {
                self.ir.pack_uint_2x32(&composite).into_value()
            };
        }
        let bits = if field == OperandField::ConstZero.0 {
            Some(0u64)
        } else if field == OperandField::SignedConstIntPos.0 {
            Some((operand.code as i64 - OperandFieldRange::SignedConstIntPosMin as i64 + 1) as u64)
        } else if field == OperandField::SignedConstIntNeg.0 {
            Some(
                (-(operand.code as i64) + OperandFieldRange::SignedConstIntNegMin as i64 - 1)
                    as u64,
            )
        } else if field == OperandField::LiteralConst.0 {
            if is_float {
                Some((operand.code as u64) << 32)
            } else {
                Some(operand.code as u64)
            }
        } else if field == OperandField::ConstFloatPos_1_0.0 {
            Some(1.0f64.to_bits())
        } else if field == OperandField::ConstFloatPos_0_5.0 {
            Some(0.5f64.to_bits())
        } else if field == OperandField::ConstFloatPos_2_0.0 {
            Some(2.0f64.to_bits())
        } else if field == OperandField::ConstFloatPos_4_0.0 {
            Some(4.0f64.to_bits())
        } else if field == OperandField::ConstFloatNeg_0_5.0 {
            Some((-0.5f64).to_bits())
        } else if field == OperandField::ConstFloatNeg_1_0.0 {
            Some((-1.0f64).to_bits())
        } else if field == OperandField::ConstFloatNeg_2_0.0 {
            Some((-2.0f64).to_bits())
        } else if field == OperandField::ConstFloatNeg_4_0.0 {
            Some((-4.0f64).to_bits())
        } else if field == OperandField::VccLo.0 {
            let lo = self.ir.get_vcc_lo();
            let hi = self.ir.get_vcc_hi();
            let composite = self.ir.composite_construct(lo, hi);
            return if is_float {
                self.ir.pack_double_2x32(&composite).into_value()
            } else {
                self.ir.pack_uint_2x32(&composite).into_value()
            };
        } else if field == OperandField::ExecLo.0 {
            assert!(!is_float);
            let exec = self.ir.get_exec();
            return self.ir.ballot(&exec).into_value();
        } else {
            None
        };
        match bits {
            Some(bits) => imm(bits),
            None => panic!("Unexpected operand: {}", field),
        }
    }

    /// `GetSrcPk` for unsigned integers.
    pub fn get_src_pk(&mut self, operand: &InstOperand) -> (U32, U32) {
        let (first, second) = self.get_src_pk_raw(operand, false, false);
        (U32::new(first), U32::new(second))
    }

    /// `GetSrcPk` for signed integers.
    pub fn get_src_pk_signed(&mut self, operand: &InstOperand) -> (U32, U32) {
        let (first, second) = self.get_src_pk_raw(operand, false, true);
        (U32::new(first), U32::new(second))
    }

    /// `GetSrcPk` for floats.
    pub fn get_src_pk_f32(&mut self, operand: &InstOperand) -> (F32, F32) {
        let (first, second) = self.get_src_pk_raw(operand, true, false);
        (F32::new(first), F32::new(second))
    }

    fn get_src_pk_raw(
        &mut self,
        operand: &InstOperand,
        is_float: bool,
        is_signed: bool,
    ) -> (Value, Value) {
        use crate::gpu::amdgpu::pixel_format::NumberFormat;
        let field = operand.field;
        let number_format = if is_float {
            NumberFormat::Float
        } else {
            NumberFormat::Uint
        };
        let op_sel = operand.op_sel.op_sel;
        let op_sel_hi = operand.op_sel.op_sel_hi;
        let imm = |bits: u32| -> (Value, Value) {
            let value = if is_float {
                Value::from_f32(f32::from_bits(bits))
            } else {
                Value::from_u32(bits)
            };
            let zero = if is_float {
                Value::from_f32(0.0)
            } else {
                Value::from_u32(0)
            };
            (
                if op_sel { zero.clone() } else { value.clone() },
                if op_sel_hi { zero } else { value },
            )
        };
        let cast = |ir: &mut IrEmitter, value: &F32| -> Value {
            if is_float {
                value.clone().into_value()
            } else {
                let bits = ir.bit_cast_u32_f32(value);
                let off = ir.imm32(0);
                let count = ir.imm32(16);
                ir.bit_field_extract(&bits, &off, &count, is_signed)
                    .into_value()
            }
        };
        let extract = |ir: &mut IrEmitter, raw: &U32| -> (Value, Value) {
            let unpacked = ir.unpack_2x16(number_format, raw);
            let first = F32::new(ir.composite_extract(&unpacked, usize::from(op_sel)));
            let second = F32::new(ir.composite_extract(&unpacked, usize::from(op_sel_hi)));
            (cast(ir, &first), cast(ir, &second))
        };
        let mut pair = if field == OperandField::ScalarGPR.0 {
            let reg = ScalarReg::from_raw(operand.code);
            let raw = self.ir.get_scalar_reg(reg);
            extract(&mut self.ir, &raw)
        } else if field == OperandField::VectorGPR.0 {
            let reg = VectorReg::from_raw(operand.code);
            let raw = self.ir.get_vector_reg(reg);
            extract(&mut self.ir, &raw)
        } else if field == OperandField::ConstZero.0 {
            imm(0)
        } else if field == OperandField::SignedConstIntPos.0 {
            imm(operand.code - OperandFieldRange::SignedConstIntPosMin as u32 + 1)
        } else if field == OperandField::SignedConstIntNeg.0 {
            imm(
                (-((operand.code as i32) - OperandFieldRange::SignedConstIntNegMin as i32 + 1))
                    as u32,
            )
        } else if field == OperandField::LiteralConst.0 {
            imm(operand.code)
        } else if field == OperandField::ConstFloatPos_1_0.0 {
            imm(1.0f32.to_bits())
        } else if field == OperandField::ConstFloatPos_0_5.0 {
            imm(0.5f32.to_bits())
        } else if field == OperandField::ConstFloatPos_2_0.0 {
            imm(2.0f32.to_bits())
        } else if field == OperandField::ConstFloatPos_4_0.0 {
            imm(4.0f32.to_bits())
        } else if field == OperandField::ConstFloatNeg_0_5.0 {
            imm((-0.5f32).to_bits())
        } else if field == OperandField::ConstFloatNeg_1_0.0 {
            imm((-1.0f32).to_bits())
        } else if field == OperandField::ConstFloatNeg_2_0.0 {
            imm((-2.0f32).to_bits())
        } else if field == OperandField::ConstFloatNeg_4_0.0 {
            imm((-4.0f32).to_bits())
        } else if field == OperandField::Inv2Pi.0 {
            imm((1.0f32 / (2.0f32 * std::f32::consts::PI)).to_bits())
        } else if field == OperandField::VccLo.0 {
            let raw = self.ir.get_vcc_lo();
            extract(&mut self.ir, &raw)
        } else {
            panic!("Unexpected operand: {}", field);
        };
        if is_float {
            if operand.input_modifier.neg {
                pair.0 = fp_neg(&mut self.ir, &F32::new(pair.0.clone())).into_value();
            }
            if operand.input_modifier.neg_hi {
                pair.1 = fp_neg(&mut self.ir, &F32::new(pair.1.clone())).into_value();
            }
        } else {
            if operand.input_modifier.neg {
                pair.0 =
                    U32::new(self.ir.i_neg(&U32U64::new(pair.0.clone())).into_value()).into_value();
            }
            if operand.input_modifier.neg_hi {
                pair.1 =
                    U32::new(self.ir.i_neg(&U32U64::new(pair.1.clone())).into_value()).into_value();
            }
        }
        pair
    }

    /// `SetDst`.
    pub fn set_dst(&mut self, operand: &InstOperand, value: &U32F32) {
        let mut result = value.clone();
        if value.ty() == Type::F32 {
            if operand.output_modifier.multiplier != 0.0 {
                let current = F32::new(result.into_value());
                let multiplier = self.ir.imm32_float(operand.output_modifier.multiplier);
                result = U32F32::new(fp_mul(&mut self.ir, &current, &multiplier).into_value());
            }
            if operand.output_modifier.clamp {
                let current = F32::new(result.into_value());
                result = U32F32::new(fp_saturate(&mut self.ir, &current).into_value());
            }
        }
        let field = operand.field;
        if field == OperandField::ScalarGPR.0 {
            let reg = ScalarReg::from_raw(operand.code);
            self.ir.set_scalar_reg(reg, &result);
        } else if field == OperandField::VectorGPR.0 {
            let reg = VectorReg::from_raw(operand.code);
            self.ir.set_vector_reg(reg, &result);
        } else if field == OperandField::VccLo.0 {
            self.ir.set_vcc_lo(&U32::new(result.into_value()));
        } else if field == OperandField::VccHi.0 {
            self.ir.set_vcc_hi(&U32::new(result.into_value()));
        } else if field == OperandField::M0.0 {
            self.ir.set_m0(&U32::new(result.into_value()));
        } else {
            panic!("Unknown field {}", field);
        }
    }

    /// `SetDst16` for unsigned integers.
    pub fn set_dst16(&mut self, operand: &InstOperand, value: &U32F32) {
        self.set_dst16_raw(operand, value, false);
    }

    /// `SetDst16` for signed integers.
    pub fn set_dst16_signed(&mut self, operand: &InstOperand, value: &U32F32) {
        self.set_dst16_raw(operand, value, true);
    }

    fn set_dst16_raw(&mut self, operand: &InstOperand, value: &U32F32, is_signed: bool) {
        let mut result = value.clone();
        if value.ty() == Type::F32 {
            if operand.output_modifier.multiplier != 0.0 {
                let current = F32::new(result.into_value());
                let multiplier = self.ir.imm32_float(operand.output_modifier.multiplier);
                result = U32F32::new(fp_mul(&mut self.ir, &current, &multiplier).into_value());
            }
            if operand.output_modifier.clamp {
                let current = F32::new(result.into_value());
                result = U32F32::new(fp_saturate(&mut self.ir, &current).into_value());
            }
        } else if operand.output_modifier.clamp {
            let current = U32::new(result.into_value());
            if is_signed {
                let lower = self.ir.imm32_signed(-32768);
                let upper = self.ir.imm32(32767);
                result = U32F32::new(self.ir.s_clamp(&current, &lower, &upper).into_value());
            } else {
                let upper = self.ir.imm32(0xffff);
                result = U32F32::new(self.ir.u_min(&current, &upper).into_value());
            }
        }

        let result = if result.ty() == Type::F32 {
            let value = F32::new(result.into_value());
            let half = F16::new(
                self.ir
                    .fp_convert(16, &F16F32F64::new(value.into_value()))
                    .into_value(),
            );
            let bits = self.ir.bit_cast_u16_f16(&half);
            U32::new(
                self.ir
                    .u_convert(32, &U8U16U32U64::new(bits.into_value()))
                    .into_value(),
            )
        } else {
            U32::new(result.into_value())
        };
        let op_sel = operand.op_sel.op_sel;
        let field = operand.field;
        if field == OperandField::ScalarGPR.0 || field == OperandField::VectorGPR.0 {
            let prev = if field == OperandField::ScalarGPR.0 {
                let reg = ScalarReg::from_raw(operand.code);
                self.ir.get_scalar_reg(reg)
            } else {
                let reg = VectorReg::from_raw(operand.code);
                self.ir.get_vector_reg(reg)
            };
            let offset = self.ir.imm32(if op_sel { 16 } else { 0 });
            let count = self.ir.imm32(16);
            let new_dst =
                self.ir
                    .bit_field_insert(&wide32(&prev), &wide32(&result), &offset, &count);
            let new_dst = U32::new(new_dst.into_value());
            if field == OperandField::ScalarGPR.0 {
                let reg = ScalarReg::from_raw(operand.code);
                self.ir
                    .set_scalar_reg(reg, &U32F32::new(new_dst.into_value()));
            } else {
                let reg = VectorReg::from_raw(operand.code);
                self.ir
                    .set_vector_reg(reg, &U32F32::new(new_dst.into_value()));
            }
        } else {
            panic!("Unexpected operand: {}", field);
        }
    }

    /// `SetDst64`.
    pub fn set_dst64(&mut self, operand: &InstOperand, value_raw: Value) {
        let is_float = value_raw.ty() == Type::F64 || value_raw.ty() == Type::F32;
        let mut value = value_raw;
        if is_float {
            if operand.output_modifier.multiplier != 0.0 {
                if value.ty() == Type::F64 {
                    let current = F64::new(value);
                    let multiplier = self
                        .ir
                        .imm64_float(operand.output_modifier.multiplier as f64);
                    value = fp_mul64(&mut self.ir, &current, &multiplier).into_value();
                } else {
                    let current = F32::new(value);
                    let multiplier = self.ir.imm32_float(operand.output_modifier.multiplier);
                    value = fp_mul(&mut self.ir, &current, &multiplier).into_value();
                }
            }
            if operand.output_modifier.clamp {
                if value.ty() == Type::F64 {
                    let current = F64::new(value);
                    value = fp_saturate64(&mut self.ir, &current).into_value();
                } else {
                    let current = F32::new(value);
                    value = fp_saturate(&mut self.ir, &current).into_value();
                }
            }
        }
        let split = |ir: &mut IrEmitter, value: &Value| -> (U32, U32) {
            let unpacked = if is_float {
                ir.unpack_double_2x32(&F64::new(value.clone()))
            } else {
                ir.unpack_uint_2x32(&U64::new(value.clone()))
            };
            (
                U32::new(ir.composite_extract(&unpacked, 0)),
                U32::new(ir.composite_extract(&unpacked, 1)),
            )
        };
        let field = operand.field;
        if field == OperandField::ScalarGPR.0 {
            let (lo, hi) = split(&mut self.ir, &value);
            let hi_reg = ScalarReg::from_raw(operand.code + 1);
            let lo_reg = ScalarReg::from_raw(operand.code);
            self.ir
                .set_scalar_reg(hi_reg, &U32F32::new(hi.into_value()));
            self.ir
                .set_scalar_reg(lo_reg, &U32F32::new(lo.into_value()));
        } else if field == OperandField::VectorGPR.0 {
            let (lo, hi) = split(&mut self.ir, &value);
            let hi_reg = VectorReg::from_raw(operand.code + 1);
            let lo_reg = VectorReg::from_raw(operand.code);
            self.ir
                .set_vector_reg(hi_reg, &U32F32::new(hi.into_value()));
            self.ir
                .set_vector_reg(lo_reg, &U32F32::new(lo.into_value()));
        } else if field == OperandField::VccLo.0 {
            let (lo, hi) = split(&mut self.ir, &value);
            self.ir.set_vcc_lo(&lo);
            self.ir.set_vcc_hi(&hi);
        } else if field == OperandField::ExecLo.0 {
            let exec = self.ir.inverse_ballot(&U64::new(value));
            self.ir.set_exec(&exec);
        } else {
            panic!("Unexpected operand: {}", field);
        }
    }

    /// `SetDstPk` for unsigned integers.
    pub fn set_dst_pk(&mut self, operand: &InstOperand, value: &(U32, U32)) {
        self.set_dst_pk_raw(operand, value, false, false);
    }

    /// `SetDstPk` for signed integers.
    pub fn set_dst_pk_signed(&mut self, operand: &InstOperand, value: &(U32, U32)) {
        self.set_dst_pk_raw(operand, value, false, true);
    }

    /// `SetDstPk` for floats.
    pub fn set_dst_pk_f32(&mut self, operand: &InstOperand, value: &(F32, F32)) {
        self.set_dst_pk_f32_raw(operand, value);
    }

    fn set_dst_pk_raw(
        &mut self,
        operand: &InstOperand,
        value: &(U32, U32),
        is_float: bool,
        is_signed: bool,
    ) {
        assert!(!is_float);
        let mut first = value.0.clone();
        let mut second = value.1.clone();
        if operand.output_modifier.clamp {
            if is_signed {
                let lower = self.ir.imm32_signed(-32768);
                let upper = self.ir.imm32(32767);
                first = self.ir.s_clamp(&first, &lower, &upper);
                let lower = self.ir.imm32_signed(-32768);
                let upper = self.ir.imm32(32767);
                second = self.ir.s_clamp(&second, &lower, &upper);
            } else {
                let upper = self.ir.imm32(0xffff);
                first = self.ir.u_min(&first, &upper);
                let upper = self.ir.imm32(0xffff);
                second = self.ir.u_min(&second, &upper);
            }
        }
        let first = self.ir.bit_cast_f32_u32(&first);
        let second = self.ir.bit_cast_f32_u32(&second);
        let vector = self.ir.composite_construct(first, second);
        let packed = self.ir.pack_2x16(
            crate::gpu::amdgpu::pixel_format::NumberFormat::Uint,
            &vector,
        );
        self.set_dst(operand, &U32F32::new(packed.into_value()));
    }

    fn set_dst_pk_f32_raw(&mut self, operand: &InstOperand, value: &(F32, F32)) {
        let mut first = value.0.clone();
        let mut second = value.1.clone();
        if operand.output_modifier.clamp {
            first = fp_saturate(&mut self.ir, &first);
            second = fp_saturate(&mut self.ir, &second);
        }
        let vector = self.ir.composite_construct(first, second);
        let packed = self.ir.pack_2x16(
            crate::gpu::amdgpu::pixel_format::NumberFormat::Float,
            &vector,
        );
        self.set_dst(operand, &U32F32::new(packed.into_value()));
    }

    /// `EmitFetch`.
    pub fn emit_fetch(&mut self, inst: &GcnInst) {
        let code_sgpr_base = inst.src[0].code;
        {
            let mut info = self.info.borrow_mut();
            info.persistent.has_fetch_shader = true;
            info.persistent.fetch_shader_sgpr_base = code_sgpr_base;
        }
        let address = {
            let info = self.info.borrow();
            get_fetch_shader_code(&info, code_sgpr_base)
        };
        let mut code = Vec::with_capacity(256);
        for index in 0..256u64 {
            code.push((self.read)(address + index * 4));
        }
        self.fetch_data = {
            let info = self.info.borrow();
            parse_fetch_shader(&info, &code)
        };
        assert!(self.fetch_data.is_some());
        let attributes = self
            .fetch_data
            .as_ref()
            .expect("the fetch shader parsed")
            .attributes
            .clone();

        for attrib in attributes {
            let attribute = attr_add(Attribute::Param0, attrib.semantic as i32)
                .expect("the attribute is in range");
            let dst_reg = VectorReg::from_raw(attrib.dest_vgpr as u32);
            let buffer = {
                let info = self.info.borrow();
                attrib.get_sharp(&info, self.read)
            };
            let x = self.ir.get_attribute(attribute, 0, 0);
            let y = self.ir.get_attribute(attribute, 1, 0);
            let z = self.ir.get_attribute(attribute, 2, 0);
            let w = self.ir.get_attribute(attribute, 3, 0);
            let values = self.ir.composite_construct4(x, y, z, w);
            let converted = apply_read_number_conversion_vec4(
                &mut self.ir,
                &values,
                buffer.get_number_conversion(),
            );
            let swizzled = apply_swizzle(&mut self.ir, &converted, buffer.dst_select());
            for index in 0..4 {
                let value = self.ir.composite_extract(&swizzled, index);
                let reg = reg_add(dst_reg, index as i32).expect("the attribute vgprs fit");
                self.ir.set_vector_reg(reg, &U32F32::new(value));
            }
        }
    }

    /// `LogMissingOpcode`.
    pub fn log_missing_opcode(&mut self, _inst: &GcnInst) {
        self.info.borrow_mut().translation_failed = true;
    }

    /// `Translate`.
    pub fn translate(
        &mut self,
        block: &BlockRef,
        start_pc: u32,
        cond: Condition,
        inst_list: &[GcnInst],
    ) {
        if inst_list.is_empty() {
            return;
        }
        self.ir = IrEmitter::at(block, 0);
        self.pc = start_pc;
        for inst in inst_list {
            self.pc += inst.length;
            if inst.opcode == crate::gpu::shader_recompiler::frontend::opcodes::Opcode::S_SWAPPC_B64
            {
                let stage = self.info.borrow().persistent.hw_stage;
                assert!(
                    stage == HwStage::Vertex || stage == HwStage::Export || stage == HwStage::Local
                );
                self.emit_fetch(inst);
                continue;
            }
            self.translate_instruction(inst);
        }
        if cond != Condition::True && cond != Condition::False {
            let value = self.ir.condition(cond);
            block.borrow_mut().branch_cond = self.ir.condition_ref(&value);
        }
    }

    /// `TranslateInstruction`.
    pub fn translate_instruction(&mut self, inst: &GcnInst) {
        match inst.category {
            InstCategory::DataShare => self.emit_data_share(inst),
            InstCategory::VectorInterpolation => self.emit_vector_interpolation(inst),
            InstCategory::ScalarMemory => self.emit_scalar_memory(inst),
            InstCategory::VectorMemory => self.emit_vector_memory(inst),
            InstCategory::Export => self.emit_export(inst),
            InstCategory::FlowControl => self.emit_flow_control(inst),
            InstCategory::ScalarALU => self.emit_scalar_alu(inst),
            InstCategory::VectorALU => self.emit_vector_alu(inst),
            InstCategory::DebugProfile => {}
            _ => unreachable!("unknown instruction category"),
        }
    }
}

fn finish_mix(translator: &mut Translator<'_>, value: F32, operand: &InstOperand) -> F32 {
    let value = if operand.input_modifier.neg_hi {
        fp_abs(&mut translator.ir, &value)
    } else {
        value
    };
    if operand.input_modifier.neg {
        fp_neg(&mut translator.ir, &value)
    } else {
        value
    }
}

fn ir_fp_recip(ir: &mut IrEmitter, value: &F32) -> F32 {
    F32::new(
        ir.fp_recip(&F32F64::new(value.value().clone()))
            .into_value(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::amdgpu::regs_shader::PsInput;
    use crate::gpu::shader_recompiler::frontend::instruction::InstOperand;
    use crate::gpu::shader_recompiler::ir::basic_block::Block;
    use crate::gpu::shader_recompiler::ir::opcodes::Opcode as IrOpcode;
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{
        HwComputeRuntimeInfo, HwFragmentRuntimeInfo,
    };

    fn info(stage: HwStage, sw_stage: SwStage) -> Rc<RefCell<Info>> {
        Rc::new(RefCell::new(Info::new(
            stage,
            sw_stage,
            ShaderParams::new([0; 16], &[], 0),
        )))
    }

    fn operand(field: OperandField, code: u32) -> InstOperand {
        InstOperand {
            field: field.0,
            code,
            ..Default::default()
        }
    }

    fn translator<'a>(
        info: Rc<RefCell<Info>>,
        runtime: &'a RuntimeInfo,
        read: &'a dyn Fn(u64) -> u32,
    ) -> Translator<'a> {
        Translator::new(info, runtime, &Profile::default(), read)
    }

    #[test]
    fn immediate_sources_keep_their_bit_patterns() {
        let info = info(HwStage::Vertex, SwStage::Vertex);
        let runtime = RuntimeInfo::new(HwStage::Vertex, SwStage::Vertex);
        let read = |_| 0u32;
        let mut translator = translator(info, &runtime, &read);
        translator.ir = IrEmitter::new(&Block::new_ref());

        let value = translator.get_src(&operand(OperandField::LiteralConst, 0x1234));
        assert_eq!(value.value().u32(), 0x1234);
        let value = translator.get_src_f32(&operand(OperandField::LiteralConst, 1.5f32.to_bits()));
        assert_eq!(value.value().f32(), 1.5);
        let value = translator.get_src_f32(&operand(OperandField::ConstFloatNeg_1_0, 0));
        assert_eq!(value.value().f32(), -1.0);
    }

    #[test]
    fn source_modifiers_apply_to_the_typed_value() {
        let info = info(HwStage::Vertex, SwStage::Vertex);
        let runtime = RuntimeInfo::new(HwStage::Vertex, SwStage::Vertex);
        let read = |_| 0u32;
        let mut translator = translator(info, &runtime, &read);
        translator.ir = IrEmitter::new(&Block::new_ref());
        let mut negated = operand(OperandField::LiteralConst, 5);
        negated.input_modifier.neg = true;
        let value = translator.get_src(&negated);
        assert_eq!(
            value.value().inst().borrow().get_opcode(),
            IrOpcode::BitwiseXor32
        );
    }

    #[test]
    fn a_sixteen_bit_source_unpacks_the_selected_half() {
        let info = info(HwStage::Vertex, SwStage::Vertex);
        let runtime = RuntimeInfo::new(HwStage::Vertex, SwStage::Vertex);
        let read = |_| 0u32;
        let mut translator = translator(info, &runtime, &read);
        translator.ir = IrEmitter::new(&Block::new_ref());

        let mut selected = operand(OperandField::ConstZero, 0);
        selected.op_sel.op_sel = true;
        let value = translator.get_src16(&selected);
        assert_eq!(value.value().u32(), 0);
    }

    #[test]
    fn a_64_bit_source_packs_the_register_pair() {
        let info = info(HwStage::Vertex, SwStage::Vertex);
        let runtime = RuntimeInfo::new(HwStage::Vertex, SwStage::Vertex);
        let read = |_| 0u32;
        let mut translator = translator(info, &runtime, &read);
        translator.ir = IrEmitter::new(&Block::new_ref());
        let value = translator.get_src64(&operand(OperandField::ScalarGPR, 2));
        assert_eq!(
            value.value().inst().borrow().get_opcode(),
            IrOpcode::PackUint2x32
        );
    }

    #[test]
    fn setting_a_scalar_destination_writes_the_register() {
        let info = info(HwStage::Vertex, SwStage::Vertex);
        let runtime = RuntimeInfo::new(HwStage::Vertex, SwStage::Vertex);
        let read = |_| 0u32;
        let mut translator = translator(info, &runtime, &read);
        let block = Block::new_ref();
        translator.ir = IrEmitter::new(&block);
        translator.set_dst(
            &operand(OperandField::ScalarGPR, 5),
            &U32F32::new(Value::from_u32(7)),
        );
        let stored = block.borrow().instructions()[0].clone();
        assert_eq!(stored.borrow().get_opcode(), IrOpcode::SetScalarRegister);
        assert_eq!(stored.borrow().arg(0).scalar_reg(), ScalarReg::S5);
        assert_eq!(stored.borrow().arg(1).u32(), 7);
    }

    #[test]
    fn a_float_destination_applies_the_output_modifiers() {
        let info = info(HwStage::Vertex, SwStage::Vertex);
        let runtime = RuntimeInfo::new(HwStage::Vertex, SwStage::Vertex);
        let read = |_| 0u32;
        let mut translator = translator(info, &runtime, &read);
        let block = Block::new_ref();
        translator.ir = IrEmitter::new(&block);
        let mut dst = operand(OperandField::VectorGPR, 3);
        dst.output_modifier.multiplier = 2.0;
        dst.output_modifier.clamp = true;
        translator.set_dst(&dst, &U32F32::new(Value::from_f32(0.5)));
        let ops: Vec<IrOpcode> = block
            .borrow()
            .instructions()
            .iter()
            .map(|inst| inst.borrow().get_opcode())
            .collect();
        assert_eq!(
            ops,
            vec![
                IrOpcode::FPMul32,
                IrOpcode::FPSaturate32,
                IrOpcode::BitCastU32F32,
                IrOpcode::SetVectorRegister
            ]
        );
    }

    #[test]
    fn the_compute_prologue_loads_the_workgroup_ids() {
        let info = info(HwStage::Compute, SwStage::Compute);
        info.borrow_mut().persistent.hw_stage = HwStage::Compute;
        let mut runtime = RuntimeInfo::new(HwStage::Compute, SwStage::Compute);
        runtime.hw = HwInfo::Cs(HwComputeRuntimeInfo {
            shared_memory_size: 0,
            workgroup_size: [4, 1, 1],
            tgid_enable: [true, false, false],
        });
        runtime.props.num_user_data = 2;
        runtime.props.num_allocated_vgprs = 8;
        let read = |_| 0u32;
        let mut translator = translator(info, &runtime, &read);
        let block = Block::new_ref();
        translator.emit_prologue(&block);
        let ops: Vec<IrOpcode> = block
            .borrow()
            .instructions()
            .iter()
            .map(|inst| inst.borrow().get_opcode())
            .collect();
        assert!(ops.contains(&IrOpcode::Prologue));
        assert!(ops.contains(&IrOpcode::SetExec));
        assert_eq!(
            ops.iter()
                .filter(|op| **op == IrOpcode::SetVectorRegister)
                .count(),
            3
        );
        assert!(ops.contains(&IrOpcode::GetAttributeU32));
    }

    #[test]
    fn the_fragment_prologue_counts_the_enabled_barycentrics() {
        let info = info(HwStage::Fragment, SwStage::Fragment);
        let mut runtime = RuntimeInfo::new(HwStage::Fragment, SwStage::Fragment);
        let mut fs = HwFragmentRuntimeInfo::zeroed();
        fs.addr_flags = PsInput::from_raw(1 << 1);
        runtime.hw = HwInfo::Fs(fs);
        let read = |_| 0u32;
        let translator = translator(info, &runtime, &read);
        assert_eq!(translator.vgpr_to_interp[0], Attribute::BaryCoordSmooth);
        let (end, entries) = iterate_barycentrics(&runtime);
        assert_eq!(entries.len(), 2);
        assert_eq!(end, VectorReg::V2);
    }
}
