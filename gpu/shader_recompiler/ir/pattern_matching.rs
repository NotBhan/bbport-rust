//! Instruction and value patterns, ported from
//! `legacy/gpu/shadps4/shader_recompiler/ir/pattern_matching.h`. The C++ composes matcher
//! objects with templates; the port uses one enum with the same variants and helper
//! constructors, so a pattern is built with nested `inst(...)` calls.

use super::attribute::Attribute;
use super::opcodes::Opcode;
use super::value::Value;

/// The C++ `MatchObject` hierarchy, as one type.
pub enum Pattern<'a> {
    /// `MatchIgnore`: any value matches.
    Ignore,
    /// `MatchValue`: binds the value.
    Value(&'a mut Value),
    /// `MatchImm`: binds an immediate.
    Imm(&'a mut Value),
    /// `MatchAttribute`: the value is that attribute.
    Attribute(Attribute),
    /// `MatchU32`: the value is that immediate `u32`.
    U32(u32),
    /// `MatchInstObject`: the value is an instruction of that opcode with matching arguments.
    Inst {
        opcode: Opcode,
        args: Vec<Pattern<'a>>,
    },
}

impl Pattern<'_> {
    /// The C++ `Match`.
    pub fn matches(&mut self, value: &Value) -> bool {
        match self {
            Pattern::Ignore => true,
            Pattern::Value(slot) => {
                **slot = value.clone();
                true
            }
            Pattern::Imm(slot) => {
                if !value.is_immediate() {
                    return false;
                }
                **slot = value.clone();
                true
            }
            Pattern::Attribute(attribute) => {
                value.ty() == super::r#type::Type::Attribute && value.attribute() == *attribute
            }
            Pattern::U32(imm) => {
                value.is_immediate()
                    && value.ty() == super::r#type::Type::U32
                    && value.u32() == *imm
            }
            Pattern::Inst { opcode, args } => {
                let Some(inst) = value.try_inst() else {
                    return false;
                };
                if inst.borrow().get_opcode() != *opcode {
                    return false;
                }
                assert_eq!(
                    args.len(),
                    inst.borrow().num_args(),
                    "the pattern and the opcode have different arities"
                );
                for (index, arg_pattern) in args.iter_mut().enumerate() {
                    let arg = inst.borrow().arg(index);
                    if !arg_pattern.matches(&arg) {
                        return false;
                    }
                }
                true
            }
        }
    }
}

/// `MatchIgnore()`.
pub fn ignore<'a>() -> Pattern<'a> {
    Pattern::Ignore
}

/// `MatchValue(value)`.
pub fn value<'a>(slot: &'a mut Value) -> Pattern<'a> {
    Pattern::Value(slot)
}

/// `MatchImm(value)`.
pub fn imm<'a>(slot: &'a mut Value) -> Pattern<'a> {
    Pattern::Imm(slot)
}

/// `MatchAttribute(attribute)`.
pub fn attribute<'a>(attribute: Attribute) -> Pattern<'a> {
    Pattern::Attribute(attribute)
}

/// `MatchU32(imm)`.
pub fn u32_val<'a>(imm: u32) -> Pattern<'a> {
    Pattern::U32(imm)
}

/// `MakeInstPattern<opcode>(args...)`.
pub fn inst<'a>(opcode: Opcode, args: Vec<Pattern<'a>>) -> Pattern<'a> {
    Pattern::Inst { opcode, args }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::shader_recompiler::ir::microinstruction::{set_arg, Inst};
    use crate::gpu::shader_recompiler::ir::r#type::Type;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn inst_of(op: Opcode, args: &[Value]) -> Value {
        let inst = Rc::new(RefCell::new(Inst::new(op, 0)));
        for (index, arg) in args.iter().enumerate() {
            set_arg(&inst, index, arg.clone());
        }
        Value::from_inst(inst)
    }

    #[test]
    fn an_instruction_pattern_checks_the_opcode_and_every_argument() {
        let value = inst_of(Opcode::IAdd32, &[Value::from_u32(1), Value::from_u32(2)]);
        assert!(inst(Opcode::IAdd32, vec![u32_val(1), u32_val(2)]).matches(&value));
        assert!(!inst(Opcode::ISub32, vec![u32_val(1), u32_val(2)]).matches(&value));
        assert!(!inst(Opcode::IAdd32, vec![u32_val(1), u32_val(3)]).matches(&value));
        assert!(!inst(Opcode::IAdd32, vec![ignore(), u32_val(3)]).matches(&value));
    }

    #[test]
    fn the_binding_matches_capture_the_value_and_immediates() {
        let node = inst_of(Opcode::IAdd32, &[Value::from_u32(7), Value::from_u32(9)]);
        let mut a = Value::empty();
        let mut b = Value::empty();
        assert!(inst(Opcode::IAdd32, vec![value(&mut a), imm(&mut b)]).matches(&node));
        assert_eq!(a.u32(), 7);
        assert_eq!(b.u32(), 9);

        // An immediate pattern refuses an instruction.
        let mut slot = Value::empty();
        assert!(!imm(&mut slot).matches(&node));
        // A value pattern accepts anything.
        assert!(value(&mut slot).matches(&node));
        assert_eq!(slot, node);
    }

    #[test]
    fn attribute_and_u32_patterns_check_the_type() {
        let attribute_value = Value::from_attribute(Attribute::TessFactorsBufferBase);
        assert!(attribute(Attribute::TessFactorsBufferBase).matches(&attribute_value));
        assert!(!attribute(Attribute::InvocationId).matches(&attribute_value));
        assert!(!u32_val(0).matches(&attribute_value));

        let number = Value::from_u32(5);
        assert!(u32_val(5).matches(&number));
        assert!(!u32_val(6).matches(&number));
        let typed = Value::from_f32(5.0);
        assert!(!u32_val(5).matches(&typed), "the type has to be U32");
        assert_eq!(number.ty(), Type::U32);
    }

    #[test]
    fn a_nested_pattern_walks_the_instruction_tree() {
        // GetAttributeU32(attribute, 0) inside a BitFieldUExtract.
        let get = inst_of(
            Opcode::GetAttributeU32,
            &[
                Value::from_attribute(Attribute::PackedHullInvocationInfo),
                Value::from_u32(0),
            ],
        );
        let extract = inst_of(
            Opcode::BitFieldUExtract,
            &[get, Value::from_u32(8), Value::from_u32(5)],
        );
        let mut pattern = inst(
            Opcode::BitFieldUExtract,
            vec![
                inst(
                    Opcode::GetAttributeU32,
                    vec![attribute(Attribute::PackedHullInvocationInfo), ignore()],
                ),
                u32_val(8),
                u32_val(5),
            ],
        );
        assert!(pattern.matches(&extract));
    }
}
