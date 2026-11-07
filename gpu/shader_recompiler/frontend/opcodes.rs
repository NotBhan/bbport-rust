//! GCN opcode tables, ported from `legacy/gpu/shadps4/shader_recompiler/frontend/opcodes.h`.
//!
//! The header's 34 enumerations are generated into `opcodes_gen`; this module is the header's
//! tail — the `s_sendmsg` structure, the two `Common::Flags` aliases and the `magic_enum`
//! range — plus the re-exports that make the tables reachable as one module.

pub use super::opcodes_gen::*;

use crate::gpu::common::bit_field::BitField;
use crate::gpu::common::r#enum::{FlagBits, Flags};

/// `Shader::Gcn::SendMsgSimm`: the immediate `s_sendmsg` carries, with the message in the low
/// four bits and the GS operation in the next two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SendMsgSimm {
    raw: u32,
}

/// `Message msg : 4`.
type SendMsgMessageField = BitField<u32, 0, 4>;
/// `GsOp op : 2`.
type SendMsgOpField = BitField<u32, 4, 2>;

impl SendMsgSimm {
    /// The whole structure over a raw immediate.
    pub const fn from_raw(raw: u32) -> Self {
        SendMsgSimm { raw }
    }

    /// The raw immediate.
    pub const fn raw(self) -> u32 {
        self.raw
    }

    /// `msg`.
    pub fn msg(self) -> Message {
        Message(SendMsgMessageField::from_raw(self.raw).value())
    }

    pub fn set_msg(&mut self, value: Message) {
        let mut field = SendMsgMessageField::from_raw(self.raw);
        field.assign(value.0);
        self.raw = field.raw();
    }

    /// `op`.
    pub fn op(self) -> GsOp {
        GsOp(SendMsgOpField::from_raw(self.raw).value())
    }

    pub fn set_op(&mut self, value: GsOp) {
        let mut field = SendMsgOpField::from_raw(self.raw);
        field.assign(value.0);
        self.raw = field.raw();
    }
}

/// `Shader::Gcn::MimgModifierFlags`. The C++'s `Common::Flags<T>` reads an enumerator's value;
/// these tables are newtypes rather than `enum`s, so the trait is implemented by hand.
pub type MimgModifierFlags = Flags<MimgModifier>;

impl FlagBits for MimgModifier {
    fn index(self) -> u32 {
        self.0
    }
}

/// `Shader::Gcn::ImageResFlags`.
pub type ImageResFlags = Flags<ImageResComponent>;

impl FlagBits for ImageResComponent {
    fn index(self) -> u32 {
        self.0
    }
}

/// `magic_enum::customize::enum_range<Shader::Gcn::Opcode>::min`: the first opcode the
/// reflection reports.
pub const MAGIC_ENUM_MIN: i32 = Opcode::S_ADD_U32.0 as i32;

/// `magic_enum::customize::enum_range<Shader::Gcn::Opcode>::max`: the last opcode the
/// reflection reports. Unlike the IR opcode table, this one names a real final enumerator.
pub const MAGIC_ENUM_MAX: i32 = Opcode::V_MAD_MIXHI_F16.0 as i32;

/// The number of enumerators in each table, counted from the C++ header. The test below walks
/// the generated source and checks that every one of them is present.
#[cfg(test)]
const COUNTS: &[(&str, usize)] = &[
    ("OpcodeSOP2", 44),
    ("OpcodeSOPK", 22),
    ("OpcodeSOP1", 51),
    ("OpcodeSOPC", 18),
    ("OpcodeSOPP", 27),
    ("OpcodeSMRD", 14),
    ("OpcodeVOP2", 58),
    ("OpcodeVOP3", 442),
    ("OpcodeVOP3P", 23),
    ("OpcodeVOP1", 69),
    ("OpcodeVOPC", 241),
    ("OpcodeVINTRP", 4),
    ("OpcodeDS", 141),
    ("OpcodeMUBUF", 57),
    ("OpcodeMTBUF", 9),
    ("OpcodeMIMG", 93),
    ("OpcodeEXP", 2),
    ("OpMapVOP3VOPX", 3),
    ("OpcodeMap", 17),
    ("Opcode", 929),
    ("EncodingMask", 7),
    ("InstEncoding", 18),
    ("InstClass", 80),
    ("InstCategory", 10),
    ("ScalarType", 12),
    ("DivergentAction", 3),
    ("OperandField", 27),
    ("SystemValue", 41),
    ("InterpolationMode", 8),
    ("MimgModifier", 8),
    ("ImageResComponent", 4),
    ("ImageAddrComponent", 17),
    ("Message", 4),
    ("GsOp", 4),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_opcode_tables_keep_their_declared_values() {
        // The first SOP2 opcode sits at the base of the unified table.
        assert_eq!(Opcode::S_ADD_U32.0, 0);
        assert_eq!(OpcodeSOP2::S_ADD_U32.0, 0);
        assert_eq!(OpcodeSOP2::OP_RANGE_SOP2.0, 45);
        // `V_MAD_MIXHI_F16` is the last enumerator of the unified table.
        assert_eq!(Opcode::V_MAD_MIXHI_F16.0, 2143);
        assert_eq!(OpcodeVOP3P::V_MAD_MIXHI_F16.0, 34);
        assert_eq!(OpcodeVOP3P::OP_RANGE_VOP3P.0, 35);
        assert_eq!(MAGIC_ENUM_MIN, 0);
        assert_eq!(MAGIC_ENUM_MAX, 2143);
        // The maps are the running sums of the class ranges.
        assert_eq!(OpcodeMap::OP_MAP_SOP2.0, 0);
        assert_eq!(OpcodeMap::OP_MAP_SOPK.0, 45);
        assert_eq!(
            OpcodeMap::OP_MAP_VOPC.0,
            OpcodeSOP2::OP_RANGE_SOP2.0
                + OpcodeSOPK::OP_RANGE_SOPK.0
                + OpcodeSOP1::OP_RANGE_SOP1.0
                + OpcodeSOPC::OP_RANGE_SOPC.0
                + OpcodeSOPP::OP_RANGE_SOPP.0
        );
        // A unified opcode is its class opcode plus the class map base.
        assert_eq!(
            Opcode::S_MOV_B32.0,
            OpcodeSOP1::S_MOV_B32.0 + OpcodeMap::OP_MAP_SOP1.0
        );
        assert_eq!(
            Opcode::V_CMP_EQ_F32.0,
            OpcodeVOPC::V_CMP_EQ_F32.0 + OpcodeMap::OP_MAP_VOPC.0
        );
        assert_eq!(
            Opcode::BUFFER_LOAD_DWORD.0,
            OpcodeMUBUF::BUFFER_LOAD_DWORD.0 + OpcodeMap::OP_MAP_MUBUF.0
        );
        assert_eq!(Opcode::EXP.0, OpcodeEXP::EXP.0 + OpcodeMap::OP_MAP_EXP.0);
    }

    #[test]
    fn the_encoding_masks_and_fields_keep_their_bits() {
        // `EncodingMask` holds the encoding in the high bits of an instruction word.
        assert_eq!(EncodingMask::MASK_9bit.0, 0x1FFu32 << 23);
        assert_eq!(EncodingMask::MASK_7bit.0, 0x7Fu32 << 25);
        assert_eq!(EncodingMask::MASK_1bit.0, 1u32 << 31);
        assert_eq!(InstEncoding::SOP1.0, 0x17Du32 << 23);
        assert_eq!(InstEncoding::VOP3.0, 0x34u32 << 26);
        assert_eq!(InstEncoding::VOP2.0, 0, "a zero shift is still a value");
        assert_eq!(
            InstEncoding::ILLEGAL.0,
            1,
            "the implicit value follows VOP2"
        );
        assert_eq!(InstCategory::Undefined.0, 0);
        assert_eq!(InstCategory::DebugProfile.0, 9);
        assert_eq!(ScalarType::Uint16.0, 2);
        assert_eq!(ScalarType::Bool.0, 11);
        assert_eq!(DivergentAction::Execute.0, 2);
        // The gaps in `OperandField` are the reserved ranges.
        assert_eq!(OperandField::ScalarGPR.0, 0);
        assert_eq!(OperandField::M0.0, 124);
        assert_eq!(OperandField::SignedConstIntPos.0, 129);
        assert_eq!(OperandField::ConstFloatPos_0_5.0, 240);
        assert_eq!(OperandField::VectorGPR.0, 256);
        assert_eq!(OperandField::Undefined.0, 0xFFFF_FFFF);
        assert_eq!(SystemValue::SubgroupLtMask.0, 78);
        assert_eq!(InterpolationMode::LinearNoPerspectiveSample.0, 7);
        assert_eq!(ImageAddrComponent::Offsets.0, 0);
        assert_eq!(ImageAddrComponent::Clamp.0, 16);
    }

    #[test]
    fn a_name_comes_back_from_the_tables() {
        assert_eq!(opcode_name(Opcode::S_ADD_U32), "S_ADD_U32");
        assert_eq!(opcode_name(Opcode::V_MAD_MIXHI_F16), "V_MAD_MIXHI_F16");
        assert_eq!(opcode_name(Opcode::S_MOV_B32), "S_MOV_B32");
        assert_eq!(OPCODE_NAMES.len(), 929);
        // A value no enumerator carries has no name, as `magic_enum::enum_name` reports.
        assert_eq!(opcode_name(Opcode(3_000_000)), "");
        assert_eq!(inst_category_name(InstCategory::ScalarALU), "ScalarALU");
        assert_eq!(INST_CATEGORY_NAMES.len(), 10);
        // Every value in the table has a name, even where two enumerators share one.
        let mut distinct: Vec<u32> = OPCODE_NAMES.iter().map(|(_, value)| value.0).collect();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), 906, "23 pairs of names share a value");
        for (_, value) in OPCODE_NAMES {
            assert!(!opcode_name(*value).is_empty());
        }
        // The aliases are the two spellings of the compare instructions, and the table
        // reports the first one declared, which is what `magic_enum` does.
        assert_eq!(Opcode::V_CMP_TRU_F32.0, Opcode::V_CMP_T_F32.0);
        assert_eq!(opcode_name(Opcode::V_CMP_T_F32), "V_CMP_TRU_F32");
        assert_eq!(opcode_name(Opcode::V_CMP_TRU_F32), "V_CMP_TRU_F32");
    }

    #[test]
    fn every_enumerator_of_the_header_is_in_the_generated_table() {
        let source = include_str!("opcodes_gen.rs");
        for (name, expected) in COUNTS {
            let marker = format!("impl {name} {{");
            let start = source
                .find(&marker)
                .unwrap_or_else(|| panic!("{name} is missing"));
            let body = &source[start + marker.len()..];
            let end = body.find("\n}").unwrap();
            let found = body[..end]
                .lines()
                .filter(|line| line.trim_start().starts_with("pub const "))
                .count();
            assert_eq!(found, *expected, "{name} enumerator count");
        }
    }

    #[test]
    fn the_send_message_immediate_holds_two_fields() {
        let mut value = SendMsgSimm::from_raw(0);
        assert_eq!(
            value.msg(),
            Message(0),
            "zero is not one of the four messages"
        );
        assert_eq!(value.op(), GsOp::Nop);
        value.set_msg(Message::Interrupt);
        assert_eq!(value.msg(), Message::Interrupt);
        value.set_msg(Message(0));
        value.set_msg(Message::System);
        assert_eq!(value.raw(), 15);
        value.set_op(GsOp::EmitCut);
        assert_eq!(value.raw(), 15 | (3 << 4));
        assert_eq!(value.msg(), Message::System);
        assert_eq!(value.op(), GsOp::EmitCut);
        // The message is four bits, so a wider write wraps the way the bit field would.
        let wrapped = SendMsgSimm::from_raw(0xFFFF_FFFF);
        assert_eq!(wrapped.msg(), Message::System);
        assert_eq!(wrapped.op(), GsOp::EmitCut);
        assert_eq!(Message::GsDone.0, 3);
        assert_eq!(GsOp::Cut.0, 1);
    }

    #[test]
    fn the_flag_aliases_carry_one_bit_per_enumerator() {
        let mut modifiers = MimgModifierFlags::default();
        assert_eq!(modifiers.raw(), 0);
        modifiers.set(MimgModifier::LodBias);
        assert_eq!(modifiers.raw(), 1 << MimgModifier::LodBias.0);
        let mut components = ImageResFlags::default();
        components.set(ImageResComponent::MipCount);
        assert_eq!(components.raw(), 1 << ImageResComponent::MipCount.0);
        assert_eq!(ImageResComponent::MipCount.0, 3);
    }
}
