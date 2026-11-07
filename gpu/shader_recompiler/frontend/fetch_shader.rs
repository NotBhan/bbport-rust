//! The fetch shader parser, ported from
//! `legacy/gpu/shadps4/shader_recompiler/frontend/fetch_shader.h` and `fetch_shader.cpp`.
//! A vertex fetch shader is a short prologue the game generates from its input semantics; this
//! walks it back into an attribute list.
//!
//! The C++ reads the shader from guest memory through a user data pointer; the port takes the
//! already-read words and reports the address through `get_fetch_shader_code` for the caller to
//! read them from.

use crate::gpu::amdgpu::resource::Buffer;
use crate::gpu::shader_recompiler::frontend::decode::{GcnCodeSlice, GcnDecodeContext};
use crate::gpu::shader_recompiler::frontend::opcodes::{InstClass, Opcode};
use crate::gpu::shader_recompiler::info::Info;
use crate::gpu::shim::bbport_toggles::{self, FETCH_SHADER_CACHE};
use std::cell::RefCell;
use std::collections::HashMap;

/// `Gcn::VertexAttribute`.
#[derive(Debug, Clone, Copy, Default)]
pub struct VertexAttribute {
    pub semantic: u8,
    pub dest_vgpr: u8,
    pub num_elements: u8,
    pub sgpr_base: u8,
    pub dword_offset: u8,
    pub instance_data: u8,
    pub inst_offset: u8,
    pub data_format: u8,
    pub num_format: u8,
}

/// `VertexAttribute::InstanceIdType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InstanceIdType {
    None = 0,
    OverStepRate0 = 1,
    OverStepRate1 = 2,
    Plain = 3,
}

impl VertexAttribute {
    /// `GetStepRate`: the instance data field is the type.
    pub fn get_step_rate(&self) -> InstanceIdType {
        match self.instance_data {
            0 => InstanceIdType::None,
            1 => InstanceIdType::OverStepRate0,
            2 => InstanceIdType::OverStepRate1,
            _ => InstanceIdType::Plain,
        }
    }

    /// `ReadRawSharp`: the descriptor the game's memory holds, or the pre-build's stored copy.
    pub fn read_raw_sharp(&self, info: &Info, read: &dyn Fn(u64) -> u32) -> Buffer {
        if let Some(sharps) = OFFLINE_ATTRIB_SHARPS.with(|slot| *slot.borrow()) {
            if let Some(sharp) = sharps.get(self.semantic as usize).copied().flatten() {
                return sharp;
            }
        }
        info.read_ud_reg(self.sgpr_base as u32, self.dword_offset as u32, read)
    }

    /// `GetSharp`: the descriptor with the instruction offset added, and the format overridden
    /// when the load was typed.
    pub fn get_sharp(&self, info: &Info, read: &dyn Fn(u64) -> u32) -> Buffer {
        let mut buffer = self.read_raw_sharp(info, read);
        buffer.set_base_address(buffer.base_address() + self.inst_offset as u64);
        if self.data_format != 0 {
            buffer.set_data_format(self.data_format as u32);
            buffer.set_num_format(self.num_format as u32);
        }
        buffer
    }
}

/// The comparison the header spells out, which leaves the offset and formats out.
impl PartialEq for VertexAttribute {
    fn eq(&self, other: &Self) -> bool {
        self.semantic == other.semantic
            && self.dest_vgpr == other.dest_vgpr
            && self.num_elements == other.num_elements
            && self.sgpr_base == other.sgpr_base
            && self.dword_offset == other.dword_offset
            && self.instance_data == other.instance_data
    }
}

/// `Gcn::FetchShaderData`.
#[derive(Debug, Clone)]
pub struct FetchShaderData {
    pub size: u32,
    pub attributes: Vec<VertexAttribute>,
    /// `vertex_offset_sgpr`: the SGPR of the vertex offset from VADDR.
    pub vertex_offset_sgpr: i8,
    /// `instance_offset_sgpr`: the SGPR of the instance offset from VADDR.
    pub instance_offset_sgpr: i8,
}

impl Default for FetchShaderData {
    fn default() -> Self {
        FetchShaderData {
            size: 0,
            attributes: Vec::new(),
            vertex_offset_sgpr: -1,
            instance_offset_sgpr: -1,
        }
    }
}

/// The header's comparison, which leaves `size` out.
impl PartialEq for FetchShaderData {
    fn eq(&self, other: &Self) -> bool {
        self.attributes == other.attributes
            && self.vertex_offset_sgpr == other.vertex_offset_sgpr
            && self.instance_offset_sgpr == other.instance_offset_sgpr
    }
}

thread_local! {
    /// `offline_fetch_shader`, the parse the pre-build thread stored.
    static OFFLINE_FETCH_SHADER: RefCell<Option<FetchShaderData>> = const { RefCell::new(None) };
    /// `offline_attrib_sharps`, the vertex buffer descriptors by semantic.
    static OFFLINE_ATTRIB_SHARPS: RefCell<Option<[Option<Buffer>; 32]>> =
        const { RefCell::new(None) };
    /// The C++'s `thread_local` parse cache, keyed by code address.
    static FETCH_SHADER_CACHE_MAP: RefCell<HashMap<u64, (Vec<u32>, FetchShaderData)>> =
        RefCell::new(HashMap::new());
}

/// Sets `offline_fetch_shader`.
pub fn set_offline_fetch_shader(data: Option<FetchShaderData>) {
    OFFLINE_FETCH_SHADER.with(|slot| *slot.borrow_mut() = data);
}

/// Sets `offline_attrib_sharps`.
pub fn set_offline_attrib_sharps(sharps: Option<[Option<Buffer>; 32]>) {
    OFFLINE_ATTRIB_SHARPS.with(|slot| *slot.borrow_mut() = sharps);
}

/// `IsTypedBufferLoad`.
fn is_typed_buffer_load(
    inst: &crate::gpu::shader_recompiler::frontend::instruction::GcnInst,
) -> bool {
    matches!(
        inst.opcode,
        Opcode::TBUFFER_LOAD_FORMAT_X
            | Opcode::TBUFFER_LOAD_FORMAT_XY
            | Opcode::TBUFFER_LOAD_FORMAT_XYZ
            | Opcode::TBUFFER_LOAD_FORMAT_XYZW
    )
}

/// `GetFetchShaderCode`: the guest address the fetch shader sits at.
pub fn get_fetch_shader_code(info: &Info, sgpr_base: u32) -> u64 {
    let user_data = info.user_data();
    let low = user_data.get(sgpr_base as usize).copied().unwrap_or(0) as u64;
    let high = user_data.get(sgpr_base as usize + 1).copied().unwrap_or(0) as u64;
    low | (high << 32)
}

/// The C++'s cache bound.
const CACHE_SIZE_LIMIT: usize = 4096;

/// `ParseFetchShader`.
pub fn parse_fetch_shader(info: &Info, code: &[u32]) -> Option<FetchShaderData> {
    if !info.persistent.has_fetch_shader {
        return None;
    }
    if let Some(offline) = OFFLINE_FETCH_SHADER.with(|slot| slot.borrow().clone()) {
        return Some(offline);
    }

    let address = get_fetch_shader_code(info, info.persistent.fetch_shader_sgpr_base);
    if !bbport_toggles::disabled(FETCH_SHADER_CACHE) {
        let cached = FETCH_SHADER_CACHE_MAP.with(|cache| {
            cache
                .borrow()
                .get(&address)
                .filter(|(words, _)| words == code)
                .map(|(_, data)| data.clone())
        });
        if let Some(data) = cached {
            return Some(data);
        }
    }

    let mut code_slice = GcnCodeSlice::new(code);
    let mut decoder = GcnDecodeContext::new();

    let mut loads = [VsharpLoad::default(); 104];

    let mut data = FetchShaderData::default();
    let mut semantic_index = 0u8;
    while !code_slice.at_end() {
        let inst = decoder.decode_instruction(&mut code_slice);
        data.size += inst.length;

        if inst.opcode == Opcode::S_SETPC_B64 {
            break;
        }

        if inst.inst_class == InstClass::ScalarMemRd {
            loads[inst.dst[0].code as usize] = VsharpLoad {
                dword_offset: inst.control.smrd().offset(),
                base_sgpr: inst.src[0].code * 2,
            };
            continue;
        }

        if inst.opcode == Opcode::V_ADD_I32 {
            let vgpr = inst.dst[0].code;
            let sgpr = inst.src[0].code as u8 as i8;
            match vgpr {
                // V0 is always the vertex offset.
                0 => data.vertex_offset_sgpr = sgpr,
                // V3 is always the instance offset.
                3 => data.instance_offset_sgpr = sgpr,
                _ => panic!("Unreachable"),
            }
        }

        if inst.inst_class == InstClass::VectorMemBufFmt {
            // SRSRC is in units of 4 SGPRs while SBASE is in pairs of SGPRs.
            let base_sgpr = inst.src[2].code * 4;
            let load = loads[base_sgpr as usize];
            let mut attrib = VertexAttribute {
                semantic: semantic_index,
                dest_vgpr: inst.src[1].code as u8,
                num_elements: inst.control.mubuf().count() as u8,
                sgpr_base: load.base_sgpr as u8,
                dword_offset: load.dword_offset as u8,
                inst_offset: inst.control.mtbuf().offset() as u8,
                instance_data: inst.src[0].code as u8,
                data_format: 0,
                num_format: 0,
            };
            semantic_index = semantic_index.wrapping_add(1);
            if is_typed_buffer_load(&inst) {
                attrib.data_format = inst.control.mtbuf().dfmt() as u8;
                attrib.num_format = inst.control.mtbuf().nfmt() as u8;
            }
            data.attributes.push(attrib);
        }
    }

    if !bbport_toggles::disabled(FETCH_SHADER_CACHE) {
        let consumed = code[..code_slice.position().min(code.len())].to_vec();
        FETCH_SHADER_CACHE_MAP.with(|cache| {
            let mut cache = cache.borrow_mut();
            if cache.len() >= CACHE_SIZE_LIMIT {
                cache.clear();
            }
            cache.insert(address, (consumed, data.clone()));
        });
    }
    Some(data)
}

/// `VsharpLoad`: where a scalar load put its descriptor.
#[derive(Debug, Clone, Copy, Default)]
struct VsharpLoad {
    dword_offset: u32,
    base_sgpr: u32,
}

/// Clears the thread's parse cache, which the C++ does by hand when the code moves.
pub fn clear_fetch_shader_cache() {
    FETCH_SHADER_CACHE_MAP.with(|cache| cache.borrow_mut().clear());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::frontend::opcodes::{
        InstEncoding, OpcodeMUBUF, OpcodeSOP1, OpcodeSOPK,
    };

    /// A scalar load of a descriptor into an SGPR.
    fn s_load_dwordx4(dst: u32, sbase: u32, offset: u32) -> u32 {
        let op = crate::gpu::shader_recompiler::frontend::opcodes::OpcodeSMRD::S_LOAD_DWORDX4.0;
        InstEncoding::SMRD.0 | (sbase << 9) | (dst << 15) | (op << 22) | (offset & 0xFF)
    }

    /// A buffer load of `count` components.
    fn buffer_load(vaddr: u32, vdata: u32, srsrc: u32, op: u32, offset: u64) -> [u32; 2] {
        let word = InstEncoding::MUBUF.0 as u64
            | ((op as u64) << 18)
            | offset
            | ((vaddr as u64) << 32)
            | ((vdata as u64) << 40)
            | ((srsrc as u64) << 48);
        [word as u32, (word >> 32) as u32]
    }

    /// A typed buffer load, which is what an attribute with an explicit format uses.
    fn mtbuf_load(vaddr: u32, vdata: u32, srsrc: u32, op: u32, offset: u64) -> [u32; 2] {
        let word = InstEncoding::MTBUF.0 as u64
            | (offset & 0xFFF)
            | ((op as u64) << 16)
            | (10u64 << 19)
            | (4u64 << 23)
            | ((vaddr as u64) << 32)
            | ((vdata as u64) << 40)
            | ((srsrc as u64) << 48);
        [word as u32, (word >> 32) as u32]
    }

    fn setpc() -> u32 {
        InstEncoding::SOP1.0 | (OpcodeSOP1::S_SETPC_B64.0 << 8)
    }

    fn info_for(code: &[u32]) -> Info {
        let mut info = Info::default();
        info.persistent.has_fetch_shader = true;
        info.persistent.fetch_shader_sgpr_base = 0;
        // The user data word 0 points at the code.
        info.user_data[0] = code.as_ptr() as u32;
        info.user_data[1] = ((code.as_ptr() as u64) >> 32) as u32;
        info
    }

    #[test]
    fn a_fetch_shader_becomes_its_attribute_list() {
        let mut code = Vec::new();
        // The descriptor is loaded into s8:11 from s2:3 at offset 0x10.
        code.push(s_load_dwordx4(8, 2, 0x10));
        // A formatted load of four components from s8:11 into v4 with an instruction offset.
        code.extend_from_slice(&buffer_load(
            0,
            4,
            2,
            OpcodeMUBUF::BUFFER_LOAD_FORMAT_XYZW.0,
            0x20,
        ));
        // A second load of two components from the same descriptor into v6.
        code.extend_from_slice(&mtbuf_load(
            0,
            6,
            2,
            crate::gpu::shader_recompiler::frontend::opcodes::OpcodeMTBUF::TBUFFER_LOAD_FORMAT_XY.0,
            0x30,
        ));
        code.push(setpc());

        let info = info_for(&code);
        clear_fetch_shader_cache();
        let data = parse_fetch_shader(&info, &code).expect("a fetch shader");
        assert_eq!(data.attributes.len(), 2);
        assert_eq!(data.size, 4 + 8 + 8 + 4, "the words the shader spans");
        assert_eq!(data.vertex_offset_sgpr, -1);
        assert_eq!(data.instance_offset_sgpr, -1);

        let first = &data.attributes[0];
        assert_eq!(first.semantic, 0);
        assert_eq!(first.dest_vgpr, 4);
        assert_eq!(first.num_elements, 4);
        assert_eq!(first.sgpr_base, 4, "the load's source times two");
        assert_eq!(first.dword_offset, 0x10);
        assert_eq!(first.inst_offset, 0x20);
        assert_eq!(first.instance_data, 0);
        assert_eq!(data.attributes[1].semantic, 1);
        assert_eq!(data.attributes[1].dest_vgpr, 6);
        // The typed load overrides the formats from the instruction.
        assert_eq!(data.attributes[1].num_elements, 2);
        assert_eq!(first.data_format, 0, "a plain buffer load has none");
        assert_eq!(data.attributes[1].data_format, 10, "from the typed load");
        assert_eq!(data.attributes[1].num_format, 4);
    }

    #[test]
    fn the_vertex_and_instance_offsets_come_from_the_adds() {
        let mut code = Vec::new();
        // v_add_i32 v0, s0, v0 and v_add_i32 v3, s1, v3.
        let op = crate::gpu::shader_recompiler::frontend::opcodes::OpcodeVOP3::V_ADD_I32.0;
        let word = InstEncoding::VOP3.0 as u64 | ((op as u64) << 17);
        let first = word | (256u64 << 32);
        let second = word | 3 | (257u64 << 32);
        code.extend_from_slice(&[first as u32, (first >> 32) as u32]);
        code.extend_from_slice(&[second as u32, (second >> 32) as u32]);
        code.push(setpc());

        let info = info_for(&code);
        clear_fetch_shader_cache();
        let data = parse_fetch_shader(&info, &code).expect("a fetch shader");
        assert_eq!(data.vertex_offset_sgpr, 0);
        assert_eq!(data.instance_offset_sgpr, 1);
        assert!(data.attributes.is_empty());
    }

    #[test]
    fn the_parse_is_cached_until_the_code_changes() {
        let mut code = Vec::new();
        code.push(s_load_dwordx4(8, 2, 0));
        code.extend_from_slice(&buffer_load(
            0,
            4,
            2,
            OpcodeMUBUF::BUFFER_LOAD_FORMAT_XYZW.0,
            0,
        ));
        code.push(setpc());
        let info = info_for(&code);
        clear_fetch_shader_cache();
        let first = parse_fetch_shader(&info, &code).unwrap();
        assert_eq!(first.attributes.len(), 1);
        let second = parse_fetch_shader(&info, &code).unwrap();
        assert_eq!(first, second);
        // A changed word misses the cache and re-parses.
        let mut changed = code.clone();
        changed[0] = s_load_dwordx4(8, 2, 0x40);
        let third = parse_fetch_shader(&info, &changed).unwrap();
        assert_eq!(third.attributes[0].dword_offset, 0x40);
        assert_ne!(first, third);
        clear_fetch_shader_cache();
    }

    #[test]
    fn a_shader_without_the_flag_has_no_parse() {
        let code = [setpc()];
        let mut info = info_for(&code);
        info.persistent.has_fetch_shader = false;
        assert!(parse_fetch_shader(&info, &code).is_none());
    }

    #[test]
    fn the_offline_parse_stands_in_for_the_code() {
        let stored = FetchShaderData {
            vertex_offset_sgpr: 5,
            ..FetchShaderData::default()
        };
        set_offline_fetch_shader(Some(stored.clone()));
        let info = info_for(&[setpc()]);
        let data = parse_fetch_shader(&info, &[setpc()]).unwrap();
        assert_eq!(data.vertex_offset_sgpr, 5);
        set_offline_fetch_shader(None);
    }

    #[test]
    fn the_attribute_sharp_adds_the_instruction_offset_and_formats() {
        let attribute = VertexAttribute {
            semantic: 2,
            sgpr_base: 4,
            dword_offset: 1,
            inst_offset: 0x40,
            data_format: 10,
            num_format: 2,
            ..VertexAttribute::default()
        };
        let mut info = Info::default();
        info.user_data[4] = 0x1000;
        info.flattened_ud_buf = vec![0; 32];
        // The descriptor at 0x1000 plus one dword.
        info.flattened_ud_buf[1] = 0x2000;
        let buffer = attribute.read_raw_sharp(&info, &|address| match address {
            0x1004 => 0x2000,
            _ => 0,
        });
        assert_eq!(
            buffer.base_address(),
            0x2000,
            "the raw descriptor, no offset yet"
        );
        let sharp = attribute.get_sharp(&info, &|address| match address {
            0x1004 => 0x2000,
            _ => 0,
        });
        assert_eq!(sharp.base_address(), 0x2040);
        assert_eq!(sharp.data_format(), 10);
        assert_eq!(sharp.num_format(), 2);

        // The offline descriptors win, by semantic.
        let mut sharps: [Option<Buffer>; 32] = [None; 32];
        sharps[2] = Some(Buffer {
            word0: 0x3000,
            ..Buffer::default()
        });
        set_offline_attrib_sharps(Some(sharps));
        let buffer = attribute.read_raw_sharp(&info, &|_| 0);
        assert_eq!(buffer.base_address(), 0x3000);
        set_offline_attrib_sharps(None);
    }

    #[test]
    fn the_step_rate_comes_from_the_instance_data() {
        let mut attribute = VertexAttribute::default();
        assert_eq!(attribute.get_step_rate(), InstanceIdType::None);
        attribute.instance_data = 1;
        assert_eq!(attribute.get_step_rate(), InstanceIdType::OverStepRate0);
        attribute.instance_data = 2;
        assert_eq!(attribute.get_step_rate(), InstanceIdType::OverStepRate1);
        attribute.instance_data = 3;
        assert_eq!(attribute.get_step_rate(), InstanceIdType::Plain);
        // The comparison ignores the formats and the offset.
        let a = VertexAttribute {
            semantic: 1,
            inst_offset: 4,
            ..VertexAttribute::default()
        };
        let b = VertexAttribute {
            semantic: 1,
            inst_offset: 8,
            data_format: 3,
            ..VertexAttribute::default()
        };
        assert_eq!(a, b);
        let c = VertexAttribute {
            semantic: 2,
            ..VertexAttribute::default()
        };
        assert_ne!(a, c);
        let _ = OpcodeSOPK::S_MOVK_I32;
    }
}
