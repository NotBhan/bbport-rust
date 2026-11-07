//! Resource discovery, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/passes/resource_discover_pass.cpp`.
//!
//! Every buffer, image and data-ring instruction gets a `ResourceDiscovery` entry listing the
//! sharp dwords it consumes. The pass also recognizes the access patterns games use to build a
//! V# from an inline constant buffer or patch the stride, and the T#/S# patterns that force a
//! cubemap to a 2D array or adjust sampler clamping.

use crate::gpu::shader_recompiler::ir::microinstruction::InstRef;
use crate::gpu::shader_recompiler::ir::opcodes::Opcode;
use crate::gpu::shader_recompiler::ir::passes::resource_pass::{
    is_buffer_instruction, is_data_ring_instruction, is_image_instruction, ResourceDiscovery,
    ResourceDiscoveryList, SharpReference,
};
use crate::gpu::shader_recompiler::ir::program::Program;
use crate::gpu::shader_recompiler::ir::value::Value;
use crate::gpu::shader_recompiler::profile::Profile;
use crate::gpu::shader_recompiler::resource::SharpFetchPostOp;

/// `IsSharpSource`.
fn is_sharp_source(inst: &InstRef) -> bool {
    matches!(
        inst.borrow().get_opcode(),
        Opcode::GetUserData | Opcode::ReadConst | Opcode::ReadConstBuffer
    )
}

/// `CheckStridePatchPattern`.
fn check_stride_patch_pattern(value: Value) -> (Value, u32, bool) {
    let Some(inst) = value.try_inst() else {
        return (value, 0, false);
    };
    if inst.borrow().get_opcode() != Opcode::BitwiseOr32 || !inst.borrow().arg(1).is_immediate() {
        return (value, 0, false);
    }
    let dword = inst.borrow().arg(0);
    let mask = inst.borrow().arg(1).u32();
    (dword, mask, true)
}

/// `CheckInlineCbufPattern`.
fn check_inline_cbuf_pattern(value: Value) -> (Value, bool) {
    let Some(inst) = value.try_inst() else {
        return (value, false);
    };
    if inst.borrow().get_opcode() != Opcode::IAdd32
        || inst.borrow().arg(0).is_immediate() == inst.borrow().arg(1).is_immediate()
    {
        return (value, false);
    }
    let offset = if inst.borrow().arg(0).is_immediate() {
        inst.borrow().arg(0).u32()
    } else {
        inst.borrow().arg(1).u32()
    };
    let prod = if inst.borrow().arg(0).is_immediate() {
        inst.borrow().arg(1).inst()
    } else {
        inst.borrow().arg(0).inst()
    };
    if prod.borrow().get_opcode() != Opcode::GetPcLo {
        return (value, false);
    }
    let base = prod.borrow().arg(0).u32();
    let dword = Value::from_u32(base + offset);
    (dword, true)
}

/// `CheckCubeTo2DArrayPattern`.
fn check_cube_to_2d_array_pattern(dword3: Value, dword4: Value) -> (Value, Value, bool) {
    let Some(dw3_inst) = dword3.try_inst() else {
        return (dword3, dword4, false);
    };
    if dw3_inst.borrow().get_opcode() != Opcode::BitwiseOr32
        || dw3_inst.borrow().arg(1).is_immediate()
        || !dw3_inst.borrow().arg(0).is_immediate()
        || dw3_inst.borrow().arg(0).u32() != 0xd000_0000
    {
        return (dword3, dword4, false);
    }
    let dw3_and = dw3_inst.borrow().arg(1).inst();
    if dw3_and.borrow().get_opcode() != Opcode::BitwiseAnd32
        || !dw3_and.borrow().arg(1).is_immediate()
        || dw3_and.borrow().arg(1).u32() != 0x0fff_ffff
    {
        return (dword3, dword4, false);
    }

    let Some(dw4_inst) = dword4.try_inst() else {
        return (dword3, dword4, false);
    };
    if dw4_inst.borrow().get_opcode() != Opcode::BitwiseOr32
        || dw4_inst.borrow().arg(0).is_immediate()
        || dw4_inst.borrow().arg(1).is_immediate()
    {
        return (dword3, dword4, false);
    }
    let dw4_rhs = dw4_inst.borrow().arg(1).inst();
    if dw4_rhs.borrow().get_opcode() != Opcode::BitwiseAnd32
        || !dw4_rhs.borrow().arg(1).is_immediate()
        || dw4_rhs.borrow().arg(1).u32() != 0xffff_e000
    {
        return (dword3, dword4, false);
    }
    let first = dw3_and.borrow().arg(0);
    let second = dw4_rhs.borrow().arg(0);
    (first, second, true)
}

/// `CheckDisableAnisoLod0Pattern`.
fn check_disable_aniso_lod0_pattern(value: Value) -> (Value, Value, bool) {
    let Some(inst) = value.try_inst() else {
        return (value, Value::empty(), false);
    };
    if inst.borrow().get_opcode() != Opcode::SelectU32 {
        return (value, Value::empty(), false);
    }
    let prod0 = inst.borrow().arg(0).inst();
    if prod0.borrow().get_opcode() != Opcode::IEqual32
        || !(prod0.borrow().arg(1).is_immediate() && prod0.borrow().arg(1).u32() == 0)
    {
        return (value, Value::empty(), false);
    }
    let prod0_arg0 = prod0.borrow().arg(0).inst();
    assert_ne!(prod0_arg0.borrow().get_opcode(), Opcode::Phi);
    if prod0_arg0.borrow().get_opcode() != Opcode::BitFieldUExtract
        || !(prod0_arg0.borrow().arg(1).is_immediate() && prod0_arg0.borrow().arg(1).u32() == 12)
        || !(prod0_arg0.borrow().arg(2).is_immediate() && prod0_arg0.borrow().arg(2).u32() == 8)
    {
        return (value, Value::empty(), false);
    }
    let prod1 = inst.borrow().arg(1).inst();
    if prod1.borrow().get_opcode() != Opcode::BitwiseAnd32
        || prod1.borrow().arg(1).u32() != 0xffff_f1ff
    {
        return (value, Value::empty(), false);
    }
    let prod2 = inst.borrow().arg(2).inst();
    assert_ne!(prod2.borrow().get_opcode(), Opcode::Phi);
    let ssharp = inst.borrow().arg(2);
    let tsharp = prod0_arg0.borrow().arg(0);
    (ssharp, tsharp, true)
}

/// `CheckForceClampToWrapPattern`.
fn check_force_clamp_to_wrap_pattern(value: Value) -> (Value, bool) {
    let Some(inst) = value.try_inst() else {
        return (value, false);
    };
    if inst.borrow().get_opcode() != Opcode::BitwiseAnd32
        || !inst.borrow().arg(0).is_immediate()
        || inst.borrow().arg(0).u32() != 0xffff_fe00
    {
        return (value, false);
    }
    let dword = inst.borrow().arg(1);
    (dword, true)
}

/// `CheckForceClampToLastTexelPattern`.
fn check_force_clamp_to_last_texel_pattern(value: Value) -> (Value, bool) {
    let Some(inst) = value.try_inst() else {
        return (value, false);
    };
    if inst.borrow().get_opcode() != Opcode::BitwiseOr32
        || inst.borrow().arg(1).is_immediate()
        || !inst.borrow().arg(0).is_immediate()
        || inst.borrow().arg(0).u32() != 0x12
    {
        return (value, false);
    }
    let prod = inst.borrow().arg(1).inst();
    if prod.borrow().get_opcode() != Opcode::BitwiseAnd32
        || !prod.borrow().arg(0).is_immediate()
        || prod.borrow().arg(0).u32() != 0xffff_fe00
    {
        return (value, false);
    }
    let dword = prod.borrow().arg(1);
    (dword, true)
}

/// `CheckClearAnisoRatioAndThresholdPattern`.
fn check_clear_aniso_ratio_and_threshold_pattern(value: Value) -> (Value, bool) {
    let Some(inst) = value.try_inst() else {
        return (value, false);
    };
    if inst.borrow().get_opcode() != Opcode::BitwiseAnd32
        || !inst.borrow().arg(1).is_immediate()
        || inst.borrow().arg(1).u32() != 0xfff8_f1ff
    {
        return (value, false);
    }
    let dword = inst.borrow().arg(0);
    (dword, true)
}

/// `MarkReadConstBufferSharpSources`.
fn mark_read_const_buffer_sharp_sources(sharp: &SharpReference) {
    for index in 0..sharp.num_dwords {
        let Some(source) = sharp.dwords[index as usize].try_inst() else {
            continue;
        };
        assert!(is_sharp_source(&source));
        if source.borrow().get_opcode() == Opcode::ReadConstBuffer {
            let mut flags = source.borrow().buffer_info();
            flags.set_sharp_source(1);
            source.borrow_mut().set_flags(flags.raw());
        }
    }
}

/// `DiscoverBufferSharp`.
fn discover_buffer_sharp(inst: &InstRef, resources: &mut ResourceDiscoveryList) {
    let handle = inst.borrow().arg(0).inst();
    resources.push(ResourceDiscovery::new(inst.clone()));
    let resource = resources.last_mut().expect("the discovery was just added");
    let vsharp = &mut resource.sharps[0];

    let num_args = handle.borrow().num_args();
    vsharp.num_dwords = num_args as u32;
    for index in 0..num_args {
        vsharp.dwords[index] = handle.borrow().arg(index);
        if let Some(source) = vsharp.dwords[index].try_inst() {
            if source.borrow().get_opcode() == Opcode::ReadFirstLane {
                vsharp.dwords[index] = source.borrow().arg(0);
            }
        }
    }

    let (dword0, inline_found) = check_inline_cbuf_pattern(vsharp.dwords[0].clone());
    if inline_found {
        vsharp.post_op = SharpFetchPostOp::OffsetByProgramBase;
        vsharp.dwords[0] = dword0;
        vsharp.dwords[1] = Value::from_u32(0);
    } else {
        let dword1 = handle.borrow().arg(1);
        let (patched, mask, found) = check_stride_patch_pattern(dword1);
        if found {
            vsharp.post_op = SharpFetchPostOp::BitwiseOrDw1WithImm;
            vsharp.post_op_data.dw1_mask = mask;
            vsharp.dwords[1] = patched;
        }
    }

    mark_read_const_buffer_sharp_sources(vsharp);
}

/// `DiscoverImageSharp`.
fn discover_image_sharp(inst: &InstRef, resources: &mut ResourceDiscoveryList) {
    let image_handle = inst.borrow().arg(0).inst();
    assert_eq!(image_handle.borrow().get_opcode(), Opcode::ImageHandle);
    resources.push(ResourceDiscovery::new(inst.clone()));
    let resource = resources.last_mut().expect("the discovery was just added");

    {
        let tsharp = &mut resource.sharps[0];
        let tsharp_low = image_handle.borrow().arg(0).inst();
        let low_args = tsharp_low.borrow().num_args();
        for index in 0..low_args {
            tsharp.dwords[tsharp.num_dwords as usize] = tsharp_low.borrow().arg(index);
            tsharp.num_dwords += 1;
        }
        if let Some(tsharp_high) = image_handle.borrow().arg(1).try_inst() {
            let high_args = tsharp_high.borrow().num_args();
            for index in 0..high_args {
                tsharp.dwords[tsharp.num_dwords as usize] = tsharp_high.borrow().arg(index);
                tsharp.num_dwords += 1;
            }
        }

        let (dword3, dword4, found) =
            check_cube_to_2d_array_pattern(tsharp.dwords[3].clone(), tsharp.dwords[4].clone());
        if found {
            tsharp.post_op = SharpFetchPostOp::ConvertCubeTo2DArray;
            tsharp.dwords[3] = dword3;
            tsharp.dwords[4] = dword4;
        }
        mark_read_const_buffer_sharp_sources(tsharp);
    }

    if inst.borrow().get_opcode() != Opcode::ImageSampleRaw {
        return;
    }

    let sampler = inst.borrow().arg(1).inst();
    let ssharp = &mut resource.sharps[1];
    let sampler_args = sampler.borrow().num_args();
    for index in 0..sampler_args {
        ssharp.dwords[ssharp.num_dwords as usize] = sampler.borrow().arg(index);
        ssharp.num_dwords += 1;
    }

    let (dword0, lod_prod, found) = check_disable_aniso_lod0_pattern(ssharp.dwords[0].clone());
    if found {
        ssharp.post_op = SharpFetchPostOp::DisableAnisoIfSingleLod;
        ssharp.post_op_data.lod_prod = lod_prod;
        ssharp.dwords[0] = dword0;
    } else {
        let (dword0, found) = check_force_clamp_to_wrap_pattern(ssharp.dwords[0].clone());
        if found {
            ssharp.post_op = SharpFetchPostOp::ForceRepeatXyzClamp;
            ssharp.dwords[0] = dword0;
        } else {
            let (dword0, found) = check_force_clamp_to_last_texel_pattern(ssharp.dwords[0].clone());
            if found {
                ssharp.post_op = SharpFetchPostOp::ForceLastTexelXyClamp;
                ssharp.dwords[0] = dword0;
            } else {
                let (dword0, found) =
                    check_clear_aniso_ratio_and_threshold_pattern(ssharp.dwords[0].clone());
                if found {
                    ssharp.post_op = SharpFetchPostOp::ClearAnisoRatioAndThreshold;
                    ssharp.dwords[0] = dword0;
                }
            }
        }
    }
    mark_read_const_buffer_sharp_sources(ssharp);
}

/// `ResourceDiscoverPass`.
pub fn resource_discover_pass(program: &Program, _profile: &Profile) -> ResourceDiscoveryList {
    let mut sharp_usages = ResourceDiscoveryList::new();
    for block in &program.blocks {
        let instructions = block.borrow().instructions.clone();
        for inst in &instructions {
            if is_buffer_instruction(inst) {
                discover_buffer_sharp(inst, &mut sharp_usages);
            } else if is_image_instruction(inst) {
                discover_image_sharp(inst, &mut sharp_usages);
            } else if is_data_ring_instruction(inst) {
                sharp_usages.push(ResourceDiscovery::new(inst.clone()));
            }
        }
    }
    sharp_usages
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::info::Info;
    use crate::gpu::shader_recompiler::ir::basic_block::{append_new_inst, Block, BlockRef};
    use crate::gpu::shader_recompiler::ir::reg::ScalarReg;
    use crate::gpu::shader_recompiler::params::ShaderParams;
    use crate::gpu::shader_recompiler::runtime_info::{HwStage, SwStage};
    use std::cell::RefCell;
    use std::rc::Rc;

    fn program(block: &BlockRef) -> Program {
        let info = Rc::new(RefCell::new(Info::new(
            HwStage::Vertex,
            SwStage::Vertex,
            ShaderParams::new([0; 16], &[], 0),
        )));
        let mut program = Program::new(info);
        program.blocks = vec![block.clone()];
        program
    }

    fn append(block: &BlockRef, op: Opcode, args: &[Value]) -> InstRef {
        let index = append_new_inst(block, op, args);
        block.borrow().instructions[index].clone()
    }

    fn user_data(block: &BlockRef, reg: ScalarReg) -> InstRef {
        append(block, Opcode::GetUserData, &[Value::from_scalar_reg(reg)])
    }

    fn handle(block: &BlockRef, dwords: &[InstRef]) -> InstRef {
        append(
            block,
            Opcode::CompositeConstructU32x4,
            &dwords
                .iter()
                .map(|dword| Value::from_inst(dword.clone()))
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn a_buffer_load_discovers_its_sharp_dwords() {
        let block = Block::new_ref();
        let dwords: Vec<InstRef> = (0..4)
            .map(|index| {
                user_data(
                    &block,
                    [ScalarReg::S0, ScalarReg::S1, ScalarReg::S2, ScalarReg::S3][index],
                )
            })
            .collect();
        let handle = handle(&block, &dwords);
        let load = append(
            &block,
            Opcode::LoadBufferU32,
            &[Value::from_inst(handle), Value::from_u32(0)],
        );

        let discoveries = resource_discover_pass(&program(&block), &Profile::default());
        assert_eq!(discoveries.len(), 1);
        assert_eq!(
            discoveries[0].user.borrow().get_opcode(),
            Opcode::LoadBufferU32
        );
        assert_eq!(discoveries[0].sharps[0].num_dwords, 4);
        assert!(Rc::ptr_eq(
            &discoveries[0].sharps[0].dwords[0].inst(),
            &dwords[0]
        ));
        let _ = load;
    }

    #[test]
    fn an_inline_cbuf_vsharp_gets_the_program_base_post_op() {
        let block = Block::new_ref();
        let pc = append(&block, Opcode::GetPcLo, &[Value::from_u32(0x1000)]);
        let dword0 = append(
            &block,
            Opcode::IAdd32,
            &[Value::from_u32(0x20), Value::from_inst(pc)],
        );
        let mut dwords = vec![dword0];
        for index in 1..4 {
            dwords.push(user_data(
                &block,
                [ScalarReg::S0, ScalarReg::S1, ScalarReg::S2, ScalarReg::S3][index],
            ));
        }
        let handle = handle(&block, &dwords);
        append(
            &block,
            Opcode::LoadBufferU32,
            &[Value::from_inst(handle), Value::from_u32(0)],
        );

        let discoveries = resource_discover_pass(&program(&block), &Profile::default());
        let vsharp = &discoveries[0].sharps[0];
        assert_eq!(vsharp.post_op, SharpFetchPostOp::OffsetByProgramBase);
        assert_eq!(vsharp.dwords[0].u32(), 0x1020);
        assert_eq!(vsharp.dwords[1].u32(), 0);
    }

    #[test]
    fn a_data_ring_instruction_gets_an_entry_without_sharps() {
        let block = Block::new_ref();
        append(
            &block,
            Opcode::DataAppend,
            &[Value::from_u32(0), Value::from_u32(1)],
        );

        let discoveries = resource_discover_pass(&program(&block), &Profile::default());
        assert_eq!(discoveries.len(), 1);
        assert_eq!(discoveries[0].sharps[0].num_dwords, 0);
    }
}
