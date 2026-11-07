//! The copy shader parser, ported from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/copy_shader.h` and `copy_shader.cpp`. A copy
//! shader exports attributes straight from buffer loads, so this walks its code looking for the
//! loads and the exports that name them, and works out how many attributes and vertices the
//! shader has.

use crate::gpu::shader_recompiler::frontend::decode::{GcnCodeSlice, GcnDecodeContext};
use crate::gpu::shader_recompiler::frontend::instruction::OperandFieldRange;
use crate::gpu::shader_recompiler::frontend::opcodes::{Opcode, OperandField};
use crate::gpu::shader_recompiler::ir::attribute::{is_position, Attribute};
use std::collections::BTreeMap;

/// `Shader::CopyShaderData`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CopyShaderData {
    /// `attr_map`: the C++ keeps a `std::map`, so the port keeps the offsets ordered.
    pub attr_map: BTreeMap<u32, (Attribute, u32)>,
    pub num_attrs: u32,
    pub output_vertices: u32,
    pub num_comps: u32,
}

/// `ParseCopyShader`: the attribute map a copy shader's code describes. The C++ asserts on
/// malformed code, which the port keeps as panics.
pub fn parse_copy_shader(code: &[u32]) -> CopyShaderData {
    let mut code_slice = GcnCodeSlice::new(code);
    let mut decoder = GcnDecodeContext::new();

    const TOKEN_MOV_VCCHI: u32 = 0xBEEB03FF;
    assert!(
        code[0] == TOKEN_MOV_VCCHI,
        "First instruction is not s_mov_b32 vcc_hi, #imm"
    );

    let mut offsets = [-1i32; 64];
    let mut sources = [-1i32; 256];

    let mut data = CopyShaderData::default();
    let mut last_attr = Attribute::Position0;
    while !code_slice.at_end() {
        let inst = decoder.decode_instruction(&mut code_slice);
        match inst.opcode {
            Opcode::S_LSHL_B32 => {
                assert!(
                    inst.src[0].field == OperandField::SignedConstIntPos.0
                        && inst.src[1].field == OperandField::SignedConstIntPos.0,
                    "s_lshl_b32 shifts a signed constant"
                );
                let shift = inst.src[1].code - OperandFieldRange::SignedConstIntPosMin as u32 + 1;
                sources[inst.dst[0].code as usize] =
                    ((inst.src[0].code - OperandFieldRange::SignedConstIntPosMin as u32 + 1)
                        << shift) as i32;
            }
            Opcode::S_MOVK_I32 => {
                sources[inst.dst[0].code as usize] = inst.control.sopk().simm as i32;
            }
            Opcode::S_MOV_B32 => {
                sources[inst.dst[0].code as usize] = inst.src[0].code as i32;
            }
            Opcode::S_ADDK_I32 => {
                sources[inst.dst[0].code as usize] += inst.control.sopk().simm as i32;
            }
            Opcode::S_BFM_B32 => {
                assert!(
                    inst.src[0].field == OperandField::SignedConstIntPos.0
                        && inst.src[1].field == OperandField::SignedConstIntPos.0,
                    "s_bfm_b32 takes two signed constants"
                );
                let src0 = inst.src[0].code - OperandFieldRange::SignedConstIntPosMin as u32 + 1;
                let src1 = inst.src[1].code - OperandFieldRange::SignedConstIntPosMin as u32 + 1;
                sources[inst.dst[0].code as usize] = (((1u32 << src0) - 1) << src1) as i32;
            }
            Opcode::EXP => {
                let exp = inst.control.exp();
                let semantic = Attribute::from_raw(exp.target()).expect("the export target");
                for i in 0..inst.src_count {
                    if exp.en() & (1 << i) == 0 {
                        continue;
                    }
                    let ofs = offsets[inst.src[i as usize].code as usize];
                    if ofs != -1 {
                        data.attr_map.insert(ofs as u32, (semantic, i));
                        if semantic > last_attr {
                            last_attr = semantic;
                        }
                    }
                }
            }
            Opcode::BUFFER_LOAD_DWORD => {
                assert!(
                    (inst.src[1].code as usize) < offsets.len(),
                    "offsets array for geometry shaders is too short"
                );
                offsets[inst.src[1].code as usize] = inst.control.mubuf().offset() as i32;
                if inst.src[3].field != OperandField::ConstZero.0 {
                    let index = inst.src[3].code as usize;
                    assert!(sources[index] != -1, "the offset register was never set");
                    offsets[inst.src[1].code as usize] += sources[index];
                }
                data.num_comps += 1;
            }
            _ => {}
        }
    }

    if !is_position(last_attr) {
        data.num_attrs = last_attr.raw() as u32 - Attribute::Param0.raw() as u32 + 1;
    }
    if data.attr_map.len() > 1 {
        let mut it = data.attr_map.iter();
        let first = *it.next().unwrap().0;
        let second = *it.next().unwrap().0;
        let comp_stride = second - first;
        data.output_vertices = comp_stride / 64;
    }

    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::frontend::opcodes::{
        InstEncoding, OpcodeEXP, OpcodeMUBUF, OpcodeSOP1, OpcodeSOP2, OpcodeSOPK,
    };

    /// An SOP1 word, which is where s_mov_b32 lives.
    fn sop1(op: u32, ssrc0: u32, sdst: u32) -> u32 {
        InstEncoding::SOP1.0 | (op << 8) | (sdst << 16) | ssrc0
    }

    /// An SOP2 word.
    fn sop2(op: u32, ssrc0: u32, ssrc1: u32, sdst: u32) -> u32 {
        InstEncoding::SOP2.0 | (op << 23) | (sdst << 16) | (ssrc1 << 8) | ssrc0
    }

    /// An SOPK word with its immediate.
    fn sopk(op: u32, sdst: u32, simm: u16) -> u32 {
        InstEncoding::SOPK.0 | (op << 23) | (sdst << 16) | simm as u32
    }

    /// The two words of a buffer load, with its offset and its four operands.
    fn buffer_load_dword(
        offset: u64,
        vaddr: u32,
        vdata: u32,
        srsrc: u32,
        soffset: u32,
    ) -> [u32; 2] {
        let word = InstEncoding::MUBUF.0 as u64
            | ((OpcodeMUBUF::BUFFER_LOAD_DWORD.0 as u64) << 18)
            | offset
            | ((vaddr as u64) << 32)
            | ((vdata as u64) << 40)
            | ((srsrc as u64) << 48)
            | ((soffset as u64) << 56);
        [word as u32, (word >> 32) as u32]
    }

    /// The two words of an export, with its target, its enable mask and its four sources.
    fn exp(target: u64, en: u64, sources: [u32; 4]) -> [u32; 2] {
        let word = InstEncoding::EXP.0 as u64
            | ((OpcodeEXP::EXP.0 as u64) << 16)
            | target << 4
            | en
            | ((sources[0] as u64) << 32)
            | ((sources[1] as u64) << 40)
            | ((sources[2] as u64) << 48)
            | ((sources[3] as u64) << 56);
        [word as u32, (word >> 32) as u32]
    }

    /// `s_mov_b32 vcc_hi, #imm`, which every copy shader starts with.
    const MOV_VCCHI: u32 = 0xBEEB03FF;

    /// The vcc_hi destination that token names.
    const VCC_HI: u32 = 107;

    #[test]
    fn the_first_instruction_has_to_be_the_vcc_hi_move() {
        let decoded = {
            let words = [MOV_VCCHI, 0];
            let mut slice = GcnCodeSlice::new(&words);
            GcnDecodeContext::new().decode_instruction(&mut slice)
        };
        assert_eq!(decoded.opcode, Opcode::S_MOV_B32);
        assert_eq!(decoded.dst[0].code, VCC_HI);
        assert_eq!(
            decoded.src[0].field,
            OperandField::LiteralConst.0,
            "the immediate follows the instruction"
        );
        assert_eq!(decoded.length, 8, "the move reads the literal word as well");
    }

    #[test]
    #[should_panic(expected = "First instruction is not s_mov_b32 vcc_hi, #imm")]
    fn a_shader_that_does_not_start_that_way_is_refused() {
        parse_copy_shader(&[0x0000_0000]);
    }

    #[test]
    fn a_load_and_an_export_become_an_attribute() {
        // The move, a load into source 3 at offset 16, then an export of source 3 as Param0.
        let mut code = vec![MOV_VCCHI, 0];
        code.extend_from_slice(&buffer_load_dword(16, 0, 3, 0, OperandField::ConstZero.0));
        code.extend_from_slice(&exp(Attribute::Param0.raw(), 0b1, [3, 0, 0, 0]));
        let data = parse_copy_shader(&code);
        assert_eq!(data.attr_map.len(), 1);
        assert_eq!(
            data.attr_map.get(&16),
            Some(&(Attribute::Param0, 0)),
            "the export lands on the load's offset"
        );
        assert_eq!(data.num_comps, 1);
        assert_eq!(data.num_attrs, 1, "Param0 alone");
        assert_eq!(data.output_vertices, 0, "one attribute has no stride");
    }

    #[test]
    fn a_register_offset_is_added_to_the_load() {
        // s_movk_i32 s0, 8; a load at offset 16 whose soffset is s0.
        let mut code = vec![MOV_VCCHI, 0];
        code.push(sopk(OpcodeSOPK::S_MOVK_I32.0, 0, 8));
        code.extend_from_slice(&buffer_load_dword(16, 0, 3, 0, 0));
        code.extend_from_slice(&exp(Attribute::Param0.raw(), 0b1, [3, 0, 0, 0]));
        let data = parse_copy_shader(&code);
        assert_eq!(
            data.attr_map.get(&24),
            Some(&(Attribute::Param0, 0)),
            "16 + 8 from the scalar register"
        );

        // The same through s_mov_b32 and s_addk_i32, which add to what is already there.
        let mut code = vec![MOV_VCCHI, 0];
        code.push(sopk(OpcodeSOPK::S_MOVK_I32.0, 0, 4));
        code.push(sopk(OpcodeSOPK::S_ADDK_I32.0, 0, 12));
        code.extend_from_slice(&buffer_load_dword(0, 0, 3, 0, 0));
        code.extend_from_slice(&exp(Attribute::Param0.raw(), 0b1, [3, 0, 0, 0]));
        let data = parse_copy_shader(&code);
        assert_eq!(
            data.attr_map.get(&16),
            Some(&(Attribute::Param0, 0)),
            "4 + 12"
        );

        // And through s_mov_b32, which copies the code of its source.
        let mut code = vec![MOV_VCCHI, 0];
        code.push(sop1(OpcodeSOP1::S_MOV_B32.0, 20, 1));
        code.extend_from_slice(&buffer_load_dword(0, 0, 3, 0, 1));
        code.extend_from_slice(&exp(Attribute::Param0.raw(), 0b1, [3, 0, 0, 0]));
        let data = parse_copy_shader(&code);
        assert_eq!(data.attr_map.get(&20), Some(&(Attribute::Param0, 0)));

        // s_movk_i32 writes a signed immediate, so a negative one subtracts.
        let mut code = vec![MOV_VCCHI, 0];
        code.push(sopk(OpcodeSOPK::S_MOVK_I32.0, 0, (-8i16) as u16));
        code.extend_from_slice(&buffer_load_dword(16, 0, 3, 0, 0));
        code.extend_from_slice(&exp(Attribute::Param0.raw(), 0b1, [3, 0, 0, 0]));
        let data = parse_copy_shader(&code);
        assert_eq!(data.attr_map.get(&8), Some(&(Attribute::Param0, 0)));
    }

    #[test]
    fn the_shift_and_bit_field_forms_build_their_own_offsets() {
        // A signed constant's code is its value plus one, so the shift and the mask are built
        // from `code - SignedConstIntPosMin + 1` on both sides.
        let signed = OperandFieldRange::SignedConstIntPosMin as u32;
        let value = |code: u32| code - signed + 1;

        let left = signed + 1;
        let right = signed + 3;
        let mut code = vec![MOV_VCCHI, 0];
        code.push(sop2(OpcodeSOP2::S_LSHL_B32.0, left, right, 0));
        code.extend_from_slice(&buffer_load_dword(0, 0, 3, 0, 0));
        code.extend_from_slice(&exp(Attribute::Param0.raw(), 0b1, [3, 0, 0, 0]));
        let data = parse_copy_shader(&code);
        let expected = value(left) << value(right);
        assert_eq!(expected, 32, "two shifted left by four");
        assert_eq!(data.attr_map.get(&expected), Some(&(Attribute::Param0, 0)));

        // s_bfm_b32 makes a mask of the first operand's width, moved up by the second.
        let width = signed + 3;
        let shift = signed + 2;
        let mut code = vec![MOV_VCCHI, 0];
        code.push(sop2(OpcodeSOP2::S_BFM_B32.0, width, shift, 0));
        code.extend_from_slice(&buffer_load_dword(0, 0, 3, 0, 0));
        code.extend_from_slice(&exp(Attribute::Param0.raw(), 0b1, [3, 0, 0, 0]));
        let data = parse_copy_shader(&code);
        let expected = ((1u32 << value(width)) - 1) << value(shift);
        assert_eq!(expected, 120, "fifteen moved up by three");
        assert_eq!(data.attr_map.get(&expected), Some(&(Attribute::Param0, 0)));
    }

    #[test]
    fn the_export_mask_decides_which_components_are_recorded() {
        // Two loads, one export enabling components 0 and 2.
        let mut code = vec![MOV_VCCHI, 0];
        code.extend_from_slice(&buffer_load_dword(0, 0, 1, 0, OperandField::ConstZero.0));
        code.extend_from_slice(&buffer_load_dword(64, 0, 2, 0, OperandField::ConstZero.0));
        code.extend_from_slice(&exp(Attribute::Param1.raw(), 0b101, [1, 0, 2, 0]));
        let data = parse_copy_shader(&code);
        assert_eq!(data.attr_map.get(&0), Some(&(Attribute::Param1, 0)));
        assert_eq!(data.attr_map.get(&64), Some(&(Attribute::Param1, 2)));
        assert_eq!(data.attr_map.len(), 2);
        assert_eq!(data.num_comps, 2);
        // The attribute with the highest number decides how many the shader uses.
        assert_eq!(data.num_attrs, 2, "Param0 through Param1");
        // Two vertices: the two offsets are 64 apart, and a vertex is 64 words wide.
        assert_eq!(data.output_vertices, 1);
    }

    #[test]
    fn a_position_export_leaves_the_attribute_count_alone() {
        let mut code = vec![MOV_VCCHI, 0];
        code.extend_from_slice(&buffer_load_dword(0, 0, 1, 0, OperandField::ConstZero.0));
        code.extend_from_slice(&exp(Attribute::Position0.raw(), 0b1, [1, 0, 0, 0]));
        let data = parse_copy_shader(&code);
        assert_eq!(data.attr_map.get(&0), Some(&(Attribute::Position0, 0)));
        assert_eq!(data.num_attrs, 0, "a position is not a parameter");
        assert_eq!(data.output_vertices, 0);
    }

    #[test]
    fn an_export_of_an_unloaded_source_is_skipped() {
        let mut code = vec![MOV_VCCHI, 0];
        code.extend_from_slice(&exp(Attribute::Param0.raw(), 0b1, [9, 0, 0, 0]));
        let data = parse_copy_shader(&code);
        assert!(data.attr_map.is_empty());
        assert_eq!(data.num_attrs, 0);
    }

    #[test]
    #[should_panic(expected = "offsets array for geometry shaders is too short")]
    fn a_load_past_the_offsets_array_is_refused() {
        let mut code = vec![MOV_VCCHI, 0];
        code.extend_from_slice(&buffer_load_dword(0, 0, 64, 0, OperandField::ConstZero.0));
        let _ = parse_copy_shader(&code);
    }

    #[test]
    #[should_panic(expected = "the offset register was never set")]
    fn a_load_using_an_unset_register_is_refused() {
        let mut code = vec![MOV_VCCHI, 0];
        code.extend_from_slice(&buffer_load_dword(0, 0, 1, 0, 3));
        let _ = parse_copy_shader(&code);
    }
}
