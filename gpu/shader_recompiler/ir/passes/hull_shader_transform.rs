//! Hull and domain shader transforms, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/hull_shader_transform.cpp`.
//!
//! Tessellation shaders pass control points and patch constants through LDS. The pass turns
//! those LDS accesses into tessellation attribute reads and writes, and the tessellation factor
//! stores into patch writes. The long comment in the C++ explains the LDS layout; the port keeps
//! the same address arithmetic and the counter-walk that tells input, output and patch-constant
//! regions apart.

use crate::gpu::amdgpu::regs_vertex::TessellationType;
use crate::gpu::shader_recompiler::frontend::tessellation::{
    TessConstantAttribute, TessellationDataConstantBuffer,
};
use crate::gpu::shader_recompiler::info::Info;
use crate::gpu::shader_recompiler::ir::attribute::Attribute;
use crate::gpu::shader_recompiler::ir::basic_block::BlockRef;
use crate::gpu::shader_recompiler::ir::breadth_first_search::breadth_first_search_value;
use crate::gpu::shader_recompiler::ir::ir_emitter::IrEmitter;
use crate::gpu::shader_recompiler::ir::microinstruction::{
    invalidate, replace_uses_with, InstRef, Use,
};
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::operand_helper::{
    get_buffer_soffset_arg, store_buffer_args,
};
use crate::gpu::shader_recompiler::ir::passes::constant_propogation::constant_propagation_pass;
use crate::gpu::shader_recompiler::ir::patch::Patch;
use crate::gpu::shader_recompiler::ir::pattern_matching::{
    attribute as p_attribute, ignore as p_ignore, imm as p_imm, inst as p_inst, u32_val as p_u32,
    value as p_value,
};
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::reg::ScalarReg;
use crate::gpu::shader_recompiler::ir::value::{Value, F32, U32, U32U64, U64};
use crate::gpu::shader_recompiler::runtime_info::{
    RuntimeInfo, SwInfo, SwStage, SwTessControlRuntimeInfo,
};
use std::collections::HashMap;
use std::rc::Rc;

fn position(block: &BlockRef, inst: &InstRef) -> Option<usize> {
    block
        .borrow()
        .instructions
        .iter()
        .position(|candidate| Rc::ptr_eq(candidate, inst))
}

fn key(inst: &InstRef) -> usize {
    Rc::as_ptr(inst) as usize
}

fn wide(value: &U32) -> U32U64 {
    U32U64::new(value.value().clone())
}

fn i_add(ir: &mut IrEmitter, a: &U32, b: &U32) -> U32 {
    U32::new(ir.i_add(&wide(a), &wide(b)).into_value())
}

fn shift_right(ir: &mut IrEmitter, value: &U32, shift: u32) -> U32 {
    let shift = ir.imm32(shift);
    U32::new(ir.shift_right_logical(&wide(value), &shift).into_value())
}

fn bitwise_and(ir: &mut IrEmitter, a: &U32, b: &U32) -> U32 {
    U32::new(ir.bitwise_and(&wide(a), &wide(b)).into_value())
}

fn tcs_info(runtime_info: &RuntimeInfo) -> &SwTessControlRuntimeInfo {
    match &runtime_info.sw {
        SwInfo::Tcs(tcs) => tcs,
        _ => panic!("the runtime info is not for a tessellation control shader"),
    }
}

fn hs_cp_stride(runtime_info: &RuntimeInfo) -> u32 {
    match &runtime_info.sw {
        SwInfo::Tcs(tcs) => tcs.hs_output_cp_stride,
        SwInfo::Tes(tes) => tes.hs_output_cp_stride,
        _ => panic!("the runtime info is not for a tessellation shader"),
    }
}

fn tcs_passthrough(runtime_info: &RuntimeInfo) -> bool {
    match &runtime_info.sw {
        SwInfo::Tcs(tcs) => tcs.is_passthrough(),
        _ => false,
    }
}

struct TessSharpLocation {
    ptr_base: ScalarReg,
    dword_off: u32,
}

/// `FindTessConstantSharp`.
fn find_tess_constant_sharp(read_const_buffer: &InstRef) -> Option<TessSharpLocation> {
    let handle = read_const_buffer.borrow().arg(0);

    let mut dword = Value::empty();
    let matched = p_inst(
        Opcode::CompositeConstructU32x4,
        vec![
            p_inst(Opcode::GetUserData, vec![p_imm(&mut dword)]),
            p_ignore(),
            p_ignore(),
            p_ignore(),
        ],
    )
    .matches(&handle);
    if matched {
        return Some(TessSharpLocation {
            ptr_base: ScalarReg::Max,
            dword_off: dword.scalar_reg() as u32,
        });
    }

    let mut ptr_base = Value::empty();
    let mut dword = Value::empty();
    let matched = p_inst(
        Opcode::CompositeConstructU32x4,
        vec![
            p_inst(
                Opcode::ReadConst,
                vec![
                    p_inst(
                        Opcode::CompositeConstructU32x2,
                        vec![
                            p_inst(Opcode::GetUserData, vec![p_imm(&mut ptr_base)]),
                            p_ignore(),
                        ],
                    ),
                    p_imm(&mut dword),
                ],
            ),
            p_ignore(),
            p_ignore(),
            p_ignore(),
        ],
    )
    .matches(&handle);
    if matched {
        return Some(TessSharpLocation {
            ptr_base: ptr_base.scalar_reg(),
            dword_off: dword.u32(),
        });
    }
    None
}

/// `TessConstantUseWalker`: increments a counter on the LDS instructions an address feeds.
struct TessConstantUseWalker {
    phi_infos: HashMap<usize, PhiInfo>,
    seq_num: u32,
}

struct PhiInfo {
    seq_num: u32,
    unique_edge: u16,
}

impl TessConstantUseWalker {
    fn new() -> Self {
        TessConstantUseWalker {
            phi_infos: HashMap::new(),
            seq_num: 1,
        }
    }

    /// `WalkUsersOfTessConstant`.
    fn walk_users_of_tess_constant(
        &mut self,
        read_const_buffer: &InstRef,
        attribute: TessConstantAttribute,
    ) {
        let inc = match attribute {
            TessConstantAttribute::HsNumPatch | TessConstantAttribute::HsOutputBase => 1,
            TessConstantAttribute::PatchConstBase => 2,
            _ => panic!("Unreachable"),
        };
        let uses = read_const_buffer.borrow().uses().to_vec();
        for r#use in uses {
            self.walk_users_of_tess_constant_helper(&r#use, inc, false);
        }
        self.seq_num += 1;
    }

    fn walk_users_of_tess_constant_helper(
        &mut self,
        r#use: &Use,
        inc: u32,
        mut propagate_error: bool,
    ) {
        let user = r#use.user.clone();
        let opcode = user.borrow().get_opcode();
        match opcode {
            Opcode::LoadSharedU32
            | Opcode::LoadSharedU64
            | Opcode::WriteSharedU32
            | Opcode::WriteSharedU64 => {
                if r#use.operand == 0 {
                    let counter = user.borrow().flags_u32();
                    user.borrow_mut().set_flags(u64::from(counter + inc));
                    assert!(
                        !propagate_error,
                        "LDS instruction {:p} accesses ambiguous attribute type",
                        Rc::as_ptr(&user)
                    );
                    return;
                }
            }
            Opcode::Phi => {
                let id = key(&user);
                match self.phi_infos.get(&id) {
                    None => {
                        self.phi_infos.insert(
                            id,
                            PhiInfo {
                                seq_num: self.seq_num,
                                unique_edge: r#use.operand as u16,
                            },
                        );
                    }
                    Some(info) if info.seq_num < self.seq_num => {
                        let info = self.phi_infos.get_mut(&id).expect("the phi info");
                        info.seq_num = self.seq_num;
                        if info.unique_edge != r#use.operand as u16 {
                            propagate_error = true;
                        }
                    }
                    Some(info) => {
                        assert_eq!(info.seq_num, self.seq_num);
                        return;
                    }
                }
            }
            _ => {}
        }
        let uses = user.borrow().uses().to_vec();
        for child in uses {
            self.walk_users_of_tess_constant_helper(&child, inc, propagate_error);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttributeRegion {
    InputCp,
    OutputCp,
    PatchConst,
}

/// `GetAttributeRegionKind`.
fn attribute_region_kind(
    inst: &InstRef,
    info: &Info,
    runtime_info: &RuntimeInfo,
) -> AttributeRegion {
    let count = inst.borrow().flags_u32();
    if count == 0 {
        return AttributeRegion::InputCp;
    }
    if info.sw_stage() == SwStage::TessellationControl && tcs_passthrough(runtime_info) {
        assert!(count <= 1);
        return AttributeRegion::PatchConst;
    }
    assert!(count <= 2);
    match count {
        1 => AttributeRegion::OutputCp,
        2 => AttributeRegion::PatchConst,
        _ => panic!("Unreachable"),
    }
}

/// `IsDivisibleByStride`.
fn is_divisible_by_stride(term: &Value, stride: u32) -> bool {
    if p_u32(stride).matches(term) {
        return true;
    }
    let mut a = Value::empty();
    let mut matched = p_inst(
        Opcode::BitFieldUExtract,
        vec![p_value(&mut a), p_u32(0), p_u32(24)],
    )
    .matches(term);
    if !matched {
        matched = p_inst(
            Opcode::BitFieldSExtract,
            vec![p_value(&mut a), p_u32(0), p_u32(24)],
        )
        .matches(term);
    }
    if matched {
        return is_divisible_by_stride(&a, stride);
    }
    let mut a = Value::empty();
    let mut b = Value::empty();
    let matched = p_inst(Opcode::IMul32, vec![p_value(&mut a), p_value(&mut b)]).matches(term);
    if matched {
        return is_divisible_by_stride(&a, stride) || is_divisible_by_stride(&b, stride);
    }
    false
}

/// `TryOptimizeAddendInModulo`.
fn try_optimize_addend_in_modulo(addend: &Value, stride: u32, addends: &mut Vec<U32>) -> bool {
    let mut a = Value::empty();
    let mut b = Value::empty();
    let matched = p_inst(Opcode::IAdd32, vec![p_value(&mut a), p_value(&mut b)]).matches(addend);
    if matched {
        let mut ret = try_optimize_addend_in_modulo(&a, stride, addends);
        ret |= try_optimize_addend_in_modulo(&b, stride, addends);
        return ret;
    }
    if !is_divisible_by_stride(addend, stride) {
        addends.push(U32::new(addend.clone()));
        return false;
    }
    true
}

/// `TryOptimizeAddressModulo`.
fn try_optimize_address_modulo(ir: &mut IrEmitter, addr: &U32, stride: u32) -> U32 {
    let mut addends = Vec::new();
    if try_optimize_addend_in_modulo(addr.value(), stride, &mut addends) {
        let mut result = ir.imm32(0u32);
        for addend in addends {
            result = i_add(ir, &result, &addend);
        }
        result
    } else {
        addr.clone()
    }
}

/// `ReadTessControlPointAttribute`.
fn read_tess_control_point_attribute(
    ir: &mut IrEmitter,
    addr: &U32,
    stride: u32,
    off_dw: u32,
    is_output_read_in_tcs: bool,
) -> F32 {
    let mut addr = addr.clone();
    if off_dw > 0 {
        let off = ir.imm32(off_dw);
        addr = i_add(ir, &addr, &off);
    }
    let stride_imm = ir.imm32(stride);
    let control_point_index = ir.i_div(&addr, &stride_imm, false);
    let opt_addr = try_optimize_address_modulo(ir, &addr, stride);
    let stride_imm = ir.imm32(stride);
    let offset = ir.i_mod(&opt_addr, &stride_imm, false);
    let attr_index = shift_right(ir, &offset, 4);
    let mask = ir.imm32(0xFu32);
    let masked = bitwise_and(ir, &offset, &mask);
    let comp_index = shift_right(ir, &masked, 2);
    if is_output_read_in_tcs {
        ir.read_tcs_generic_output_attribute(&control_point_index, &attr_index, &comp_index)
    } else {
        ir.get_tess_generic_attribute(&control_point_index, &attr_index, &comp_index)
    }
}

/// `InitTessConstants`.
fn init_tess_constants(
    sharp_ptr_base: ScalarReg,
    sharp_dword_offset: i32,
    info: &mut Info,
    runtime_info: &mut RuntimeInfo,
    tess_constants: &mut TessellationDataConstantBuffer,
    read: &dyn Fn(u64) -> u32,
) {
    info.persistent.tess_consts_ptr_base = sharp_ptr_base;
    info.persistent.tess_consts_dword_offset = sharp_dword_offset;
    info.read_tess_constant_buffer(tess_constants, read);
    runtime_info.init_from_tess_constants(tess_constants);
}

/// `GetValue`.
fn factor_value(ir: &mut IrEmitter, data: &Value) -> F32 {
    if let Some(inst) = data.try_inst() {
        if inst.borrow().get_opcode() == Opcode::BitCastU32F32 {
            let arg = inst.borrow().arg(0);
            return F32::new(arg);
        }
    }
    let value = U32::new(data.clone());
    ir.bit_cast_f32_u32(&value)
}

/// `get_factor_attr`.
fn factor_attribute(runtime_info: &RuntimeInfo, gcn_factor_idx: u32) -> Patch {
    match tcs_info(runtime_info).tess_type {
        TessellationType::Isoline => {
            assert!(gcn_factor_idx < 2);
            Patch::factor(gcn_factor_idx)
        }
        TessellationType::Triangle => {
            assert!(gcn_factor_idx < 4);
            if gcn_factor_idx == 3 {
                Patch::TESSELLATION_LOD_INTERIOR_U
            } else {
                Patch::factor(gcn_factor_idx)
            }
        }
        TessellationType::Quad => {
            assert!(gcn_factor_idx < 6);
            Patch::factor(gcn_factor_idx)
        }
    }
}

/// `SetOutput`.
fn set_output(
    ir: &mut IrEmitter,
    mut addr: U32,
    value: &U32,
    output_kind: AttributeRegion,
    off_dw: u32,
    runtime_info: &RuntimeInfo,
) {
    let data_component = ir.bit_cast_f32_u32(value);
    if output_kind == AttributeRegion::OutputCp {
        if off_dw > 0 {
            let off = ir.imm32(off_dw);
            addr = i_add(ir, &addr, &off);
        }
        let stride = hs_cp_stride(runtime_info);
        let opt_addr = try_optimize_address_modulo(ir, &addr, stride);
        let stride_imm = ir.imm32(stride);
        let offset = ir.i_mod(&opt_addr, &stride_imm, false);
        let attr_index = shift_right(ir, &offset, 4);
        let mask = ir.imm32(0xFu32);
        let masked = bitwise_and(ir, &offset, &mask);
        let comp_index = shift_right(ir, &masked, 2);
        ir.set_tcs_generic_attribute(&data_component, &attr_index, &comp_index);
    } else {
        assert_eq!(output_kind, AttributeRegion::PatchConst);
        assert!(addr.value().is_immediate(), "patch addr non imm");
        ir.set_patch(
            Patch::generic((addr.value().u32() >> 2) + off_dw),
            &data_component,
        );
    }
}

/// `HullShaderTransform`.
pub fn hull_shader_transform(program: &Program, runtime_info: &RuntimeInfo) {
    let info = program.info.borrow();

    for block in &program.blocks {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            let opcode = inst.borrow().get_opcode();
            match opcode {
                Opcode::StoreBufferU32
                | Opcode::StoreBufferU32x2
                | Opcode::StoreBufferU32x3
                | Opcode::StoreBufferU32x4 => {
                    let soffset = get_buffer_soffset_arg(inst);
                    let matched = p_inst(
                        Opcode::GetAttributeU32,
                        vec![p_attribute(Attribute::TessFactorsBufferBase), p_ignore()],
                    )
                    .matches(soffset.value());
                    if !matched {
                        continue;
                    }

                    let buffer_info = inst.borrow().buffer_info();
                    let index = position(block, inst).expect("the instruction is in its block");
                    let mut ir = IrEmitter::at(block, index);

                    let address = inst.borrow().arg(store_buffer_args::ADDRESS);
                    let mut voffset = Value::empty();
                    let success = p_inst(
                        Opcode::CompositeConstructU32x3,
                        vec![p_u32(0), p_imm(&mut voffset), p_ignore()],
                    )
                    .matches(&address);
                    assert!(success, "unhandled pattern in tess factor store");

                    let gcn_factor_idx = (buffer_info.inst_offset() as u32 + voffset.u32()) >> 2;
                    let data = inst.borrow().arg(store_buffer_args::DATA);
                    let num_dwords = opcode as u32 - Opcode::StoreBufferU32 as u32 + 1;

                    invalidate(inst);
                    if num_dwords == 1 {
                        let value = factor_value(&mut ir, &data);
                        ir.set_patch(factor_attribute(runtime_info, gcn_factor_idx), &value);
                        continue;
                    }
                    let composite = data.inst();
                    assert!(matches!(
                        composite.borrow().get_opcode(),
                        Opcode::CompositeConstructU32x2
                            | Opcode::CompositeConstructU32x3
                            | Opcode::CompositeConstructU32x4
                    ));
                    for i in 0..num_dwords {
                        let element = composite.borrow().arg(i as usize);
                        let value = factor_value(&mut ir, &element);
                        ir.set_patch(factor_attribute(runtime_info, gcn_factor_idx + i), &value);
                    }
                }
                Opcode::WriteSharedU32 | Opcode::WriteSharedU64 => {
                    let index = position(block, inst).expect("the instruction is in its block");
                    let mut ir = IrEmitter::at(block, index);
                    let num_dwords = if opcode == Opcode::WriteSharedU32 {
                        1
                    } else {
                        2
                    };
                    let addr = U32::new(inst.borrow().arg(0));
                    let data = if num_dwords == 2 {
                        let packed = U64::new(inst.borrow().arg(1));
                        ir.unpack_uint_2x32(&packed)
                    } else {
                        inst.borrow().arg(1)
                    };
                    let region = attribute_region_kind(inst, &info, runtime_info);
                    if num_dwords == 1 {
                        let value = U32::new(data);
                        set_output(&mut ir, addr, &value, region, 0, runtime_info);
                    } else {
                        for i in 0..num_dwords {
                            let component = ir.composite_extract(&data, i as usize);
                            let value = U32::new(component);
                            set_output(&mut ir, addr.clone(), &value, region, i, runtime_info);
                        }
                    }
                    invalidate(inst);
                }
                Opcode::LoadSharedU32 | Opcode::LoadSharedU64 => {
                    let index = position(block, inst).expect("the instruction is in its block");
                    let mut ir = IrEmitter::at(block, index);
                    let addr = U32::new(inst.borrow().arg(0));
                    let region = attribute_region_kind(inst, &info, runtime_info);
                    let num_dwords = if opcode == Opcode::LoadSharedU32 {
                        1
                    } else {
                        2
                    };
                    assert!(
                        matches!(region, AttributeRegion::InputCp | AttributeRegion::OutputCp),
                        "Unhandled read of patchconst attribute in hull shader"
                    );
                    let is_tcs_output_read = region == AttributeRegion::OutputCp;
                    let stride = if is_tcs_output_read {
                        hs_cp_stride(runtime_info)
                    } else {
                        tcs_info(runtime_info).ls_stride
                    };
                    if num_dwords == 1 {
                        let attr_read = read_tess_control_point_attribute(
                            &mut ir,
                            &addr,
                            stride,
                            0,
                            is_tcs_output_read,
                        );
                        let value = ir.bit_cast_u32_f32(&attr_read);
                        replace_uses_with(inst, value.into_value());
                    } else {
                        let mut components = Vec::new();
                        for i in 0..num_dwords {
                            let component = read_tess_control_point_attribute(
                                &mut ir,
                                &addr,
                                stride,
                                i,
                                is_tcs_output_read,
                            );
                            components.push(ir.bit_cast_u32_f32(&component).into_value());
                        }
                        let composite =
                            ir.composite_construct(components[0].clone(), components[1].clone());
                        let packed = ir.pack_uint_2x32(&composite);
                        replace_uses_with(inst, packed.into_value());
                    }
                }
                _ => {}
            }
        }
    }

    if tcs_passthrough(runtime_info) {
        let entry = program
            .blocks
            .first()
            .expect("the program has a first block")
            .clone();
        let instructions = entry.borrow().instructions.clone();
        let prologue = instructions
            .iter()
            .position(|inst| inst.borrow().get_opcode() == Opcode::Prologue)
            .expect("the first block has a prologue");
        assert!(prologue + 1 < instructions.len());
        let mut ir = IrEmitter::at(&entry, prologue + 2);

        let num_attributes = (tcs_info(runtime_info).ls_stride + 15) >> 4;
        let invocation_id = ir.get_attribute_u32(Attribute::InvocationId, 0);
        for attr_no in 0..num_attributes {
            for comp in 0..4 {
                let attr_index = ir.imm32(attr_no);
                let comp_index = ir.imm32(comp);
                let attr_read =
                    ir.get_tess_generic_attribute(&invocation_id, &attr_index, &comp_index);
                let attr_index = ir.imm32(attr_no);
                let comp_index = ir.imm32(comp);
                ir.set_tcs_generic_attribute(&attr_read, &attr_index, &comp_index);
            }
        }
    }
}

/// `DomainShaderTransform`.
pub fn domain_shader_transform(program: &Program, runtime_info: &RuntimeInfo) {
    let info = program.info.borrow();

    for block in &program.blocks {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            let opcode = inst.borrow().get_opcode();
            match opcode {
                Opcode::LoadSharedU32 | Opcode::LoadSharedU64 => {
                    let index = position(block, inst).expect("the instruction is in its block");
                    let mut ir = IrEmitter::at(block, index);
                    let addr = U32::new(inst.borrow().arg(0));
                    let region = attribute_region_kind(inst, &info, runtime_info);
                    let num_dwords = if opcode == Opcode::LoadSharedU32 {
                        1
                    } else {
                        2
                    };
                    let get_input = |ir: &mut IrEmitter, addr: &U32, off_dw: u32| -> F32 {
                        if region == AttributeRegion::OutputCp {
                            read_tess_control_point_attribute(
                                ir,
                                addr,
                                hs_cp_stride(runtime_info),
                                off_dw,
                                false,
                            )
                        } else {
                            assert_eq!(region, AttributeRegion::PatchConst);
                            ir.get_patch(Patch::generic((addr.value().u32() >> 2) + off_dw))
                        }
                    };
                    if num_dwords == 1 {
                        let attr_read = get_input(&mut ir, &addr, 0);
                        let value = ir.bit_cast_u32_f32(&attr_read);
                        replace_uses_with(inst, value.into_value());
                    } else {
                        let mut components = Vec::new();
                        for i in 0..num_dwords {
                            let component = get_input(&mut ir, &addr, i);
                            components.push(ir.bit_cast_u32_f32(&component).into_value());
                        }
                        let composite =
                            ir.composite_construct(components[0].clone(), components[1].clone());
                        let packed = ir.pack_uint_2x32(&composite);
                        replace_uses_with(inst, packed.into_value());
                    }
                }
                _ => {}
            }
        }
    }
}

/// `TessellationPreprocess`.
pub fn tessellation_preprocess(
    program: &Program,
    runtime_info: &mut RuntimeInfo,
    read: &dyn Fn(u64) -> u32,
) {
    let mut tess_constants = TessellationDataConstantBuffer::default();
    let mut info = program.info.borrow_mut();

    'find: for block in &program.blocks {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            let opcode = inst.borrow().get_opcode();
            if !matches!(
                opcode,
                Opcode::LoadSharedU32
                    | Opcode::LoadSharedU64
                    | Opcode::WriteSharedU32
                    | Opcode::WriteSharedU64
            ) {
                continue;
            }
            let addr = inst.borrow().arg(0);
            let found = breadth_first_search_value(&addr, |candidate| {
                if candidate.borrow().get_opcode() == Opcode::ReadConstBuffer {
                    Some(candidate.clone())
                } else {
                    None
                }
            });
            if let Some(read_const_buffer) = found {
                let location = find_tess_constant_sharp(&read_const_buffer)
                    .expect("Failed to match tess constant sharp");
                if info.persistent.tess_consts_dword_offset >= 0 {
                    assert!(
                        location.dword_off as i32 == info.persistent.tess_consts_dword_offset
                            && location.ptr_base == info.persistent.tess_consts_ptr_base,
                        "TessConstants V# is ambiguous"
                    );
                }
                init_tess_constants(
                    location.ptr_base,
                    location.dword_off as i32,
                    &mut info,
                    runtime_info,
                    &mut tess_constants,
                    read,
                );
                break 'find;
            }
        }
    }

    assert!(info.persistent.tess_consts_dword_offset >= 0);

    let mut walker = TessConstantUseWalker::new();
    for block in &program.blocks {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            if inst.borrow().get_opcode() != Opcode::ReadConstBuffer {
                continue;
            }
            let Some(location) = find_tess_constant_sharp(inst) else {
                continue;
            };
            if location.ptr_base != info.persistent.tess_consts_ptr_base
                || location.dword_off != info.persistent.tess_consts_dword_offset as u32
            {
                continue;
            }
            let index = inst.borrow().arg(1);
            assert!(
                index.is_immediate(),
                "Tessellation constant read with dynamic index"
            );
            let off_dw = index.u32();
            assert!(
                off_dw <= TessConstantAttribute::FirstEdgeTessFactorIndex.index(),
                "the tessellation attribute is in range"
            );
            let attribute =
                TessConstantAttribute::from_index(off_dw).expect("a tessellation attribute");
            match attribute {
                TessConstantAttribute::LsStride => {
                    assert_eq!(info.sw_stage(), SwStage::TessellationControl);
                    replace_uses_with(inst, Value::from_u32(tess_constants.ls_stride));
                }
                TessConstantAttribute::HsCpStride => {
                    replace_uses_with(inst, Value::from_u32(tess_constants.hs_cp_stride));
                }
                TessConstantAttribute::HsNumPatch
                | TessConstantAttribute::HsOutputBase
                | TessConstantAttribute::PatchConstBase => {
                    walker.walk_users_of_tess_constant(inst, attribute);
                    replace_uses_with(inst, Value::from_u32(0));
                }
                TessConstantAttribute::PatchConstSize
                | TessConstantAttribute::PatchOutputSize
                | TessConstantAttribute::OffChipTessellationFactorThreshold
                | TessConstantAttribute::FirstEdgeTessFactorIndex => {}
            }
        }
    }

    if info.sw_stage() == SwStage::TessellationControl {
        let passthrough = tcs_passthrough(runtime_info);
        for block in &program.blocks {
            let instructions = block.borrow().instructions.clone();
            for (index, inst) in instructions.iter().enumerate() {
                let value = Value::from_inst(inst.clone());
                let patch_id = p_inst(
                    Opcode::BitFieldUExtract,
                    vec![
                        p_inst(
                            Opcode::GetAttributeU32,
                            vec![p_attribute(Attribute::PackedHullInvocationInfo), p_ignore()],
                        ),
                        p_u32(0),
                        p_u32(8),
                    ],
                )
                .matches(&value);
                if patch_id {
                    replace_uses_with(inst, Value::from_u32(0));
                    continue;
                }
                let invocation = p_inst(
                    Opcode::BitFieldUExtract,
                    vec![
                        p_inst(
                            Opcode::GetAttributeU32,
                            vec![p_attribute(Attribute::PackedHullInvocationInfo), p_ignore()],
                        ),
                        p_u32(8),
                        p_u32(5),
                    ],
                )
                .matches(&value);
                if invocation {
                    let mut ir = IrEmitter::at(block, index);
                    let replacement = if passthrough {
                        ir.imm32(0u32).into_value()
                    } else {
                        ir.get_attribute_u32(Attribute::InvocationId, 0)
                            .into_value()
                    };
                    replace_uses_with(inst, replacement);
                }
            }
        }
    }

    drop(info);
    constant_propagation_pass(&program.post_order_blocks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block};
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage, SwTessEvalRuntimeInfo};
    use std::cell::RefCell;

    fn make_program(stage: HwStage, sw_stage: SwStage, block: &BlockRef) -> Program {
        let info = Rc::new(RefCell::new(Info::new(
            stage,
            sw_stage,
            ShaderParams::new([0; 16], &[], 0),
        )));
        let mut program = Program::new(info);
        program.blocks = vec![block.clone()];
        program.post_order_blocks = vec![block.clone()];
        program
    }

    fn tcs_runtime() -> RuntimeInfo {
        let mut runtime_info = RuntimeInfo::new(HwStage::Hull, SwStage::TessellationControl);
        let mut tcs = SwTessControlRuntimeInfo::zeroed();
        tcs.tess_type = TessellationType::Triangle;
        tcs.ls_stride = 0x20;
        tcs.hs_output_cp_stride = 0x10;
        runtime_info.sw = SwInfo::Tcs(tcs);
        runtime_info
    }

    fn domain_runtime() -> RuntimeInfo {
        let mut runtime_info = RuntimeInfo::new(HwStage::Local, SwStage::TessellationEval);
        let mut tes = SwTessEvalRuntimeInfo::zeroed();
        tes.hs_output_cp_stride = 0x10;
        runtime_info.sw = SwInfo::Tes(tes);
        runtime_info
    }

    fn append(block: &BlockRef, op: Opcode, args: &[Value]) -> InstRef {
        let index = append_new_inst(block, op, args);
        block.borrow().instructions[index].clone()
    }

    fn opcodes(block: &BlockRef) -> Vec<Opcode> {
        block
            .borrow()
            .instructions()
            .iter()
            .map(|inst| inst.borrow().get_opcode())
            .collect()
    }

    #[test]
    fn a_tess_factor_store_becomes_a_patch_write() {
        let block = Block::new_ref();
        let soffset = append(
            &block,
            Opcode::GetAttributeU32,
            &[
                Value::from_attribute(Attribute::TessFactorsBufferBase),
                Value::from_u32(0),
            ],
        );
        let address = append(
            &block,
            Opcode::CompositeConstructU32x3,
            &[
                Value::from_u32(0),
                Value::from_u32(12),
                Value::from_inst(soffset),
            ],
        );
        let store = append(
            &block,
            Opcode::StoreBufferU32,
            &[
                Value::from_u32(0),
                Value::from_inst(address),
                Value::from_u32(0x3F80_0000),
            ],
        );
        let program = make_program(HwStage::Hull, SwStage::TessellationControl, &block);
        hull_shader_transform(&program, &tcs_runtime());

        assert_eq!(store.borrow().get_opcode(), Opcode::Void);
        let ops = opcodes(&block);
        assert_eq!(
            &ops[ops.len() - 3..],
            &[Opcode::BitCastF32U32, Opcode::SetPatch, Opcode::Void]
        );
        let set_patch = block.borrow().instructions[ops.len() - 2].clone();
        assert_eq!(
            set_patch.borrow().arg(0).patch(),
            Patch::TESSELLATION_LOD_INTERIOR_U,
            "triangle factor 3 is the interior factor"
        );
    }

    #[test]
    fn a_shared_load_becomes_a_tessellation_attribute_read() {
        let block = Block::new_ref();
        let load = append(&block, Opcode::LoadSharedU32, &[Value::from_u32(0x20)]);
        let program = make_program(HwStage::Hull, SwStage::TessellationControl, &block);
        hull_shader_transform(&program, &tcs_runtime());

        assert_eq!(load.borrow().get_opcode(), Opcode::Void);
        assert_eq!(
            opcodes(&block),
            vec![
                Opcode::UDiv32,
                Opcode::UMod32,
                Opcode::ShiftRightLogical32,
                Opcode::BitwiseAnd32,
                Opcode::ShiftRightLogical32,
                Opcode::GetTessGenericAttribute,
                Opcode::BitCastU32F32,
                Opcode::Void,
            ]
        );

        let block = Block::new_ref();
        let load = append(&block, Opcode::LoadSharedU32, &[Value::from_u32(0x10)]);
        load.borrow_mut().set_flags(1);
        let program = make_program(HwStage::Hull, SwStage::TessellationControl, &block);
        hull_shader_transform(&program, &tcs_runtime());
        assert!(opcodes(&block).contains(&Opcode::ReadTcsGenericOuputAttribute));
    }

    #[test]
    fn a_domain_load_reads_output_points_or_patch_constants() {
        let block = Block::new_ref();
        let output = append(&block, Opcode::LoadSharedU32, &[Value::from_u32(0x10)]);
        output.borrow_mut().set_flags(1);
        let patch = append(&block, Opcode::LoadSharedU32, &[Value::from_u32(0x40)]);
        patch.borrow_mut().set_flags(2);
        let program = make_program(HwStage::Local, SwStage::TessellationEval, &block);
        domain_shader_transform(&program, &domain_runtime());

        assert_eq!(output.borrow().get_opcode(), Opcode::Void);
        assert_eq!(patch.borrow().get_opcode(), Opcode::Void);
        assert!(opcodes(&block).contains(&Opcode::GetTessGenericAttribute));
        assert!(opcodes(&block).contains(&Opcode::GetPatch));
    }

    #[test]
    fn preprocessing_reads_the_tessellation_constants_and_strips_ls_stride() {
        let block = Block::new_ref();
        let ud = append(
            &block,
            Opcode::GetUserData,
            &[Value::from_scalar_reg(ScalarReg::S0)],
        );
        let ud_hi = append(
            &block,
            Opcode::GetUserData,
            &[Value::from_scalar_reg(ScalarReg::S1)],
        );
        let pair = append(
            &block,
            Opcode::CompositeConstructU32x2,
            &[Value::from_inst(ud), Value::from_inst(ud_hi)],
        );
        let base = append(
            &block,
            Opcode::ReadConst,
            &[Value::from_inst(pair), Value::from_u32(0)],
        );
        let handle = append(
            &block,
            Opcode::CompositeConstructU32x4,
            &[
                Value::from_inst(base),
                Value::from_u32(0),
                Value::from_u32(0),
                Value::from_u32(0),
            ],
        );
        let read_const_buffer = append(
            &block,
            Opcode::ReadConstBuffer,
            &[Value::from_inst(handle), Value::from_u32(0)],
        );
        let address = append(
            &block,
            Opcode::IAdd32,
            &[
                Value::from_inst(read_const_buffer.clone()),
                Value::from_u32(0),
            ],
        );
        let shared = append(&block, Opcode::LoadSharedU32, &[Value::from_inst(address)]);
        let invocation = append(
            &block,
            Opcode::GetAttributeU32,
            &[
                Value::from_attribute(Attribute::PackedHullInvocationInfo),
                Value::from_u32(0),
            ],
        );
        let patch_id = append(
            &block,
            Opcode::BitFieldUExtract,
            &[
                Value::from_inst(invocation),
                Value::from_u32(0),
                Value::from_u32(8),
            ],
        );
        let sink = append(
            &block,
            Opcode::ConvertF16F32,
            &[Value::from_inst(patch_id.clone())],
        );

        let program = make_program(HwStage::Hull, SwStage::TessellationControl, &block);
        let mut info = program.info.borrow_mut();
        info.user_data[0] = 0x1000;
        drop(info);
        let mut runtime_info = tcs_runtime();
        let read = |address: u64| -> u32 {
            match address {
                0x1000 => 0x2000,
                0x2000 => 0x20,
                0x2004 => 0x10,
                _ => 0,
            }
        };
        tessellation_preprocess(&program, &mut runtime_info, &read);

        let info = program.info.borrow();
        assert_eq!(info.persistent.tess_consts_ptr_base, ScalarReg::S0);
        assert_eq!(info.persistent.tess_consts_dword_offset, 0);
        drop(info);
        let SwInfo::Tcs(tcs) = &runtime_info.sw else {
            panic!("the runtime info is TCS");
        };
        assert_eq!(tcs.ls_stride, 0x20);
        assert_eq!(tcs.hs_output_cp_stride, 0x10);
        assert_eq!(read_const_buffer.borrow().get_opcode(), Opcode::Void);
        assert_eq!(shared.borrow().get_opcode(), Opcode::LoadSharedU32);
        assert_eq!(patch_id.borrow().get_opcode(), Opcode::Void);
        assert_eq!(sink.borrow().arg(0).u32(), 0);
    }

    #[test]
    fn a_passthrough_hull_shader_copies_its_inputs_to_the_outputs() {
        let block = Block::new_ref();
        append(&block, Opcode::Prologue, &[]);
        append(&block, Opcode::GetScc, &[]);
        append(&block, Opcode::Barrier, &[]);
        let mut runtime_info = tcs_runtime();
        let SwInfo::Tcs(tcs) = &mut runtime_info.sw else {
            panic!("the runtime info is TCS");
        };
        tcs.ls_stride = 0x20;
        tcs.hs_output_cp_stride = 0x20;
        tcs.hs_output_base = 0;
        tcs.num_threads = 1;
        let program = make_program(HwStage::Hull, SwStage::TessellationControl, &block);
        hull_shader_transform(&program, &runtime_info);

        let ops = opcodes(&block);
        assert_eq!(ops[2], Opcode::GetAttributeU32);
        assert_eq!(ops[3], Opcode::GetTessGenericAttribute);
        assert_eq!(ops[4], Opcode::SetTcsGenericAttribute);
        assert_eq!(
            ops.len(),
            3 + 1 + 2 * 4 * 2,
            "invocation id plus two attributes of two components"
        );
    }
}
