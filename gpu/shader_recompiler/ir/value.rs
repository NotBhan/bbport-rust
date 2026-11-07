//! IR values, ported from `legacy/gpu/shadps4/shader_recompiler/ir/value.h` and `value.cpp`.
//!
//! The C++ `Value` is a tagged union holding an `Inst*`, a register, an attribute, a patch, an
//! immediate or a string. The port keeps the tag and the payload, with the instruction as an
//! `Rc<RefCell<Inst>>` handle so a value can still answer `ty()` on its own — the C++ reaches
//! through the pointer for exactly that.

use super::attribute::Attribute;
use super::microinstruction::Inst;
use super::patch::Patch;
use super::r#type::Type;
use super::reg::{ScalarReg, VectorReg, VirtualReg};
use crate::gpu::common::hash::hash_combine_u64;
use std::cell::RefCell;
use std::rc::Rc;

/// A shared handle to an instruction, the port's stand-in for `IR::Inst*`.
pub type InstRef = Rc<RefCell<Inst>>;

/// The C++ union's members, split by kind. The immediate arms share one bit-carrying form, as
/// the union does: the tag is what says whether those bits are a `U32` or an `F32`.
#[derive(Clone, Debug)]
enum Payload {
    None,
    Inst(InstRef),
    ScalarReg(ScalarReg),
    VectorReg(VectorReg),
    VirtualReg(VirtualReg),
    Attribute(Attribute),
    Patch(Patch),
    Bits(u64),
    Text(&'static str),
}

/// `Shader::IR::Value`.
#[derive(Clone, Debug)]
pub struct Value {
    ty: Type,
    payload: Payload,
}

impl Default for Value {
    fn default() -> Self {
        Self::empty()
    }
}

impl Value {
    /// `Value()`: the empty value, whose type is `Void`.
    pub const fn empty() -> Self {
        Self {
            ty: Type::Void,
            payload: Payload::None,
        }
    }

    /// `Value(IR::Inst*)`.
    pub fn from_inst(inst: InstRef) -> Self {
        Self {
            ty: Type::Opaque,
            payload: Payload::Inst(inst),
        }
    }

    /// `Value(IR::ScalarReg)`.
    pub const fn from_scalar_reg(reg: ScalarReg) -> Self {
        Self {
            ty: Type::ScalarReg,
            payload: Payload::ScalarReg(reg),
        }
    }

    /// `Value(IR::VectorReg)`.
    pub const fn from_vector_reg(reg: VectorReg) -> Self {
        Self {
            ty: Type::VectorReg,
            payload: Payload::VectorReg(reg),
        }
    }

    /// `Value(IR::VirtualReg)`.
    pub const fn from_virtual_reg(reg: VirtualReg) -> Self {
        Self {
            ty: Type::VirtualReg,
            payload: Payload::VirtualReg(reg),
        }
    }

    /// `Value(IR::Attribute)`.
    pub const fn from_attribute(attribute: Attribute) -> Self {
        Self {
            ty: Type::Attribute,
            payload: Payload::Attribute(attribute),
        }
    }

    /// `Value(IR::Patch)`.
    pub const fn from_patch(patch: Patch) -> Self {
        Self {
            ty: Type::Patch,
            payload: Payload::Patch(patch),
        }
    }

    /// `Value(bool)`.
    pub const fn from_u1(value: bool) -> Self {
        Self {
            ty: Type::U1,
            payload: Payload::Bits(value as u64),
        }
    }

    /// `Value(u8)`.
    pub const fn from_u8(value: u8) -> Self {
        Self {
            ty: Type::U8,
            payload: Payload::Bits(value as u64),
        }
    }

    /// `Value(u16)`.
    pub const fn from_u16(value: u16) -> Self {
        Self {
            ty: Type::U16,
            payload: Payload::Bits(value as u64),
        }
    }

    /// `Value(u32)`.
    pub const fn from_u32(value: u32) -> Self {
        Self {
            ty: Type::U32,
            payload: Payload::Bits(value as u64),
        }
    }

    /// `Value(f32)`: the union shares its storage with `u32`, so the bits travel.
    pub fn from_f32(value: f32) -> Self {
        Self {
            ty: Type::F32,
            payload: Payload::Bits(value.to_bits() as u64),
        }
    }

    /// `Value(u64)`.
    pub const fn from_u64(value: u64) -> Self {
        Self {
            ty: Type::U64,
            payload: Payload::Bits(value),
        }
    }

    /// `Value(f64)`, sharing storage with `u64`.
    pub fn from_f64(value: f64) -> Self {
        Self {
            ty: Type::F64,
            payload: Payload::Bits(value.to_bits()),
        }
    }

    /// `Value(const char*)`.
    pub const fn from_string_literal(value: &'static str) -> Self {
        Self {
            ty: Type::StringLiteral,
            payload: Payload::Text(value),
        }
    }

    /// `Value::IsEmpty`.
    pub fn is_empty(&self) -> bool {
        self.ty == Type::Void
    }

    /// `Value::IsImmediate`.
    pub fn is_immediate(&self) -> bool {
        self.ty != Type::Opaque
    }

    /// `Value::Inst`, asserting the value holds one.
    pub fn inst(&self) -> InstRef {
        match &self.payload {
            Payload::Inst(inst) => inst.clone(),
            _ => panic!("the value is not an instruction"),
        }
    }

    /// `Value::TryInst`.
    pub fn try_inst(&self) -> Option<InstRef> {
        match &self.payload {
            Payload::Inst(inst) => Some(inst.clone()),
            _ => None,
        }
    }

    /// `Value::ScalarReg`.
    pub fn scalar_reg(&self) -> ScalarReg {
        match self.payload {
            Payload::ScalarReg(reg) => reg,
            _ => panic!("the value is not a scalar register"),
        }
    }

    /// `Value::VectorReg`.
    pub fn vector_reg(&self) -> VectorReg {
        match self.payload {
            Payload::VectorReg(reg) => reg,
            _ => panic!("the value is not a vector register"),
        }
    }

    /// `Value::VirtualReg`.
    pub fn virtual_reg(&self) -> VirtualReg {
        match self.payload {
            Payload::VirtualReg(reg) => reg,
            _ => panic!("the value is not a virtual register"),
        }
    }

    /// `Value::Attribute`.
    pub fn attribute(&self) -> Attribute {
        match self.payload {
            Payload::Attribute(attribute) => attribute,
            _ => panic!("the value is not an attribute"),
        }
    }

    /// `Value::Patch`.
    pub fn patch(&self) -> Patch {
        match self.payload {
            Payload::Patch(patch) => patch,
            _ => panic!("the value is not a patch"),
        }
    }

    /// `Value::U1`.
    pub fn u1(&self) -> bool {
        match self.payload {
            Payload::Bits(bits) if self.ty == Type::U1 => bits != 0,
            _ => panic!("the value is not a u1"),
        }
    }

    /// `Value::U8`.
    pub fn u8(&self) -> u8 {
        match self.payload {
            Payload::Bits(bits) if self.ty == Type::U8 => bits as u8,
            _ => panic!("the value is not a u8"),
        }
    }

    /// `Value::U16`. The C++ shares this member with `F16`.
    pub fn u16(&self) -> u16 {
        match self.payload {
            Payload::Bits(bits) if self.ty == Type::U16 || self.ty == Type::F16 => bits as u16,
            _ => panic!("the value is not a u16"),
        }
    }

    /// `Value::U32`, shared with `F32` in the C++.
    pub fn u32(&self) -> u32 {
        match self.payload {
            Payload::Bits(bits) if self.ty == Type::U32 || self.ty == Type::F32 => bits as u32,
            _ => panic!("the value is not a u32"),
        }
    }

    /// `Value::F32`.
    pub fn f32(&self) -> f32 {
        match self.payload {
            Payload::Bits(bits) if self.ty == Type::F32 => f32::from_bits(bits as u32),
            _ => panic!("the value is not an f32"),
        }
    }

    /// `Value::U64`, shared with `F64`.
    pub fn u64(&self) -> u64 {
        match self.payload {
            Payload::Bits(bits) if self.ty == Type::U64 || self.ty == Type::F64 => bits,
            _ => panic!("the value is not a u64"),
        }
    }

    /// `Value::F64`.
    pub fn f64(&self) -> f64 {
        match self.payload {
            Payload::Bits(bits) if self.ty == Type::F64 => f64::from_bits(bits),
            _ => panic!("the value is not an f64"),
        }
    }

    /// `Value::StringLiteral`.
    pub fn string_literal(&self) -> &'static str {
        match self.payload {
            Payload::Text(text) => text,
            _ => panic!("the value is not a string literal"),
        }
    }

    /// The stored tag, without following an instruction.
    pub const fn stored_type(&self) -> Type {
        self.ty
    }

    /// `Value::IsPhi`.
    pub fn is_phi(&self) -> bool {
        if let Payload::Inst(inst) = &self.payload {
            return inst.borrow().get_opcode() == super::opcodes::Opcode::Phi;
        }
        false
    }
}

// `Value::Type` and `Value::Hash` live outside the constructor block because they reach into the
// instruction a value may hold.
impl Value {
    /// `Value::Type`: an opaque value asks the instruction it points at.
    pub fn ty(&self) -> Type {
        if let Payload::Inst(inst) = &self.payload {
            return inst.borrow().ty();
        }
        self.ty
    }

    /// `Value::Hash`: the C++'s hash, with the instruction's address as its identity.
    pub fn hash_value(&self) -> u64 {
        let h = hash_combine_u64(self.ty.bits() as u64, 0);
        match &self.payload {
            Payload::None => h,
            Payload::Inst(inst) => Rc::as_ptr(inst) as u64,
            Payload::ScalarReg(reg) => hash_combine_u64(*reg as u64, h),
            Payload::VectorReg(reg) => hash_combine_u64(*reg as u64, h),
            Payload::Attribute(attribute) => hash_combine_u64(attribute.raw(), h),
            Payload::Bits(bits) => match self.ty {
                Type::U1
                | Type::U8
                | Type::U16
                | Type::F16
                | Type::U32
                | Type::F32
                | Type::U64
                | Type::F64 => hash_combine_u64(*bits, h),
                _ => unreachable!("Invalid type {}", self.ty),
            },
            // The C++ has no case for virtual registers, patches or strings, and aborts.
            Payload::VirtualReg(_) | Payload::Patch(_) | Payload::Text(_) => {
                unreachable!("Invalid type {}", self.ty)
            }
        }
    }
}

impl PartialEq for Value {
    /// `Value::operator==`: the tag first, then the member the union shares.
    fn eq(&self, other: &Self) -> bool {
        if self.ty != other.ty {
            return false;
        }
        match (&self.payload, &other.payload) {
            (Payload::None, _) => true,
            (Payload::Inst(lhs), Payload::Inst(rhs)) => Rc::ptr_eq(lhs, rhs),
            (Payload::ScalarReg(lhs), Payload::ScalarReg(rhs)) => lhs == rhs,
            (Payload::VectorReg(lhs), Payload::VectorReg(rhs)) => lhs == rhs,
            (Payload::Attribute(lhs), Payload::Attribute(rhs)) => lhs == rhs,
            (Payload::Bits(lhs), Payload::Bits(rhs))
                if self.ty == Type::U16 || self.ty == Type::F16 =>
            {
                (*lhs as u16) == (*rhs as u16)
            }
            (Payload::Bits(lhs), Payload::Bits(rhs))
                if self.ty == Type::U32 || self.ty == Type::F32 =>
            {
                (*lhs as u32) == (*rhs as u32)
            }
            (Payload::Bits(lhs), Payload::Bits(rhs)) => lhs == rhs,
            (Payload::Text(lhs), Payload::Text(rhs)) => lhs == rhs,
            _ => unreachable!("Invalid type {}", self.ty),
        }
    }
}

impl Eq for Value {}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hash_value().hash(state);
    }
}

/// `Shader::IR::TypedValue<type_>`: a value restricted to a set of types. The C++ passes the
/// mask as a non-type template parameter; the port does the same with a const parameter.
#[derive(Clone, Debug)]
pub struct TypedValue<const MASK: u32>(Value);

impl<const MASK: u32> TypedValue<MASK> {
    /// The mask this value was declared with.
    pub const MASK: Type = Type::from_bits(MASK);

    /// The C++'s default constructor: an empty value whose tag is `Void`, which the check in
    /// `new` would refuse. `Block::branch_cond` starts out this way.
    pub const fn empty() -> Self {
        Self(Value::empty())
    }

    /// `TypedValue(const Value&)`: asserts the value's type intersects the mask.
    pub fn new(value: Value) -> Self {
        let ty = value.ty();
        assert!(
            !ty.intersection(Self::MASK).is_empty(),
            "Incompatible types {} and {}",
            Self::MASK,
            ty
        );
        Self(value)
    }

    /// `TypedValue(IR::Inst*)`.
    pub fn from_inst(inst: InstRef) -> Self {
        Self::new(Value::from_inst(inst))
    }

    /// `TypedValue::Type()`: the wrapper carries the value's type.
    pub fn ty(&self) -> Type {
        self.0.ty()
    }

    /// `TypedValue::IsImmediate()`.
    pub fn is_immediate(&self) -> bool {
        self.0.is_immediate()
    }

    pub fn value(&self) -> &Value {
        &self.0
    }

    pub fn into_value(self) -> Value {
        self.0
    }
}

/// The C++'s implicit `TypedValue` to `Value` conversion: the same value, untyped.
impl<const MASK: u32> From<TypedValue<MASK>> for Value {
    fn from(value: TypedValue<MASK>) -> Self {
        value.into_value()
    }
}

impl<const MASK: u32> From<&TypedValue<MASK>> for Value {
    fn from(value: &TypedValue<MASK>) -> Self {
        value.value().clone()
    }
}

impl<const MASK: u32> From<Value> for TypedValue<MASK> {
    fn from(value: Value) -> Self {
        Self::new(value)
    }
}

impl<const MASK: u32> Default for TypedValue<MASK> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<const MASK: u32> PartialEq for TypedValue<MASK> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<const MASK: u32> Eq for TypedValue<MASK> {}

/// The C++'s `using` aliases, over the same masks.
pub type U1 = TypedValue<{ Type::U1.bits() }>;
pub type U8 = TypedValue<{ Type::U8.bits() }>;
pub type U16 = TypedValue<{ Type::U16.bits() }>;
pub type U32 = TypedValue<{ Type::U32.bits() }>;
pub type U64 = TypedValue<{ Type::U64.bits() }>;
pub type F16 = TypedValue<{ Type::F16.bits() }>;
pub type F32 = TypedValue<{ Type::F32.bits() }>;
pub type F64 = TypedValue<{ Type::F64.bits() }>;
pub type U32F32 = TypedValue<{ Type::U32.union(Type::F32).bits() }>;
pub type U64F64 = TypedValue<{ Type::U64.union(Type::F64).bits() }>;
pub type U32U64 = TypedValue<{ Type::U32.union(Type::U64).bits() }>;
pub type U16U32U64 = TypedValue<{ Type::U16.union(Type::U32).union(Type::U64).bits() }>;
pub type U8U16U32U64 = TypedValue<
    {
        Type::U8
            .union(Type::U16)
            .union(Type::U32)
            .union(Type::U64)
            .bits()
    },
>;
pub type F32F64 = TypedValue<{ Type::F32.union(Type::F64).bits() }>;
pub type F16F32F64 = TypedValue<{ Type::F16.union(Type::F32).union(Type::F64).bits() }>;
pub type UAny = TypedValue<
    {
        Type::U8
            .union(Type::U16)
            .union(Type::U32)
            .union(Type::U64)
            .bits()
    },
>;

#[cfg(test)]
mod tests {
    use super::super::opcodes::Opcode;
    use super::*;

    #[test]
    fn the_constructors_set_the_tag_and_the_payload() {
        let empty = Value::empty();
        assert!(empty.is_empty() && empty.is_immediate());
        assert_eq!(empty.ty(), Type::Void);
        assert!(empty.try_inst().is_none());

        let u32_value = Value::from_u32(0xDEAD_BEEF);
        assert_eq!(u32_value.stored_type(), Type::U32);
        assert_eq!(u32_value.ty(), Type::U32);
        assert_eq!(u32_value.u32(), 0xDEAD_BEEF);
        assert!(u32_value.is_immediate());
        assert!(!u32_value.is_empty());

        // F32 and U32 share storage, and the tag is what separates them.
        let f32_value = Value::from_f32(1.5);
        assert_eq!(f32_value.ty(), Type::F32);
        assert_eq!(f32_value.f32(), 1.5);
        assert_eq!(f32_value.u32(), 1.5f32.to_bits());
        assert_ne!(f32_value, u32_value);

        assert!(Value::from_u1(true).u1());
        assert!(!Value::from_u1(false).u1());
        assert_eq!(Value::from_u8(7).u8(), 7);
        assert_eq!(Value::from_u16(9).u16(), 9);
        assert_eq!(Value::from_u64(1 << 40).u64(), 1 << 40);
        assert_eq!(Value::from_f64(-2.5).f64(), -2.5);
        assert_eq!(Value::from_string_literal("hi").string_literal(), "hi");
        assert_eq!(
            Value::from_scalar_reg(ScalarReg::S4).scalar_reg(),
            ScalarReg::S4
        );
        assert_eq!(
            Value::from_vector_reg(VectorReg::V9).vector_reg(),
            VectorReg::V9
        );
        let virtual_reg = VirtualReg::new(3, Type::F32);
        assert_eq!(
            Value::from_virtual_reg(virtual_reg).virtual_reg(),
            virtual_reg
        );
        assert_eq!(
            Value::from_attribute(Attribute::Param2).attribute(),
            Attribute::Param2
        );
        assert_eq!(
            Value::from_patch(Patch::COMPONENT0).patch(),
            Patch::COMPONENT0
        );
    }

    #[test]
    fn an_opaque_value_asks_its_instruction_for_the_type() {
        let inst = Rc::new(RefCell::new(Inst::new(Opcode::ReadConst, 0)));
        let value = Value::from_inst(inst.clone());
        assert_eq!(value.stored_type(), Type::Opaque);
        assert_eq!(value.ty(), Type::U32, "ReadConst returns a U32");
        assert!(!value.is_immediate());
        assert!(Rc::ptr_eq(&value.inst(), &inst));
        assert!(!value.is_phi());

        // A phi node keeps its type in the instruction's flags.
        let phi = Rc::new(RefCell::new(Inst::new(Opcode::Phi, 0)));
        phi.borrow_mut().set_type_flags(Type::F32);
        let phi_value = Value::from_inst(phi);
        assert_eq!(phi_value.ty(), Type::F32);
        assert!(phi_value.is_phi());
    }

    #[test]
    fn equality_and_hashing_follow_the_cpp_switch() {
        assert_eq!(Value::empty(), Value::empty());
        assert_ne!(Value::from_u32(1), Value::from_u64(1), "the tag differs");
        assert_eq!(Value::from_u32(5), Value::from_u32(5));
        assert_ne!(Value::from_u32(5), Value::from_u32(6));
        assert_eq!(Value::from_u16(0x1234), Value::from_u16(0x1234));
        assert_eq!(
            Value::from_string_literal("a"),
            Value::from_string_literal("a")
        );
        assert_ne!(
            Value::from_string_literal("a"),
            Value::from_string_literal("b")
        );

        // Two instructions are equal only when they are the same instruction.
        let first = Rc::new(RefCell::new(Inst::new(Opcode::ReadConst, 0)));
        let second = Rc::new(RefCell::new(Inst::new(Opcode::ReadConst, 0)));
        assert_eq!(
            Value::from_inst(first.clone()),
            Value::from_inst(first.clone())
        );
        assert_ne!(Value::from_inst(first.clone()), Value::from_inst(second));
        assert_eq!(
            Value::from_u32(3).hash_value(),
            Value::from_u32(3).hash_value()
        );
        assert_ne!(
            Value::from_u32(3).hash_value(),
            Value::from_u32(4).hash_value()
        );
        let _ = Value::from_inst(first).hash_value();
    }

    #[test]
    fn typed_values_check_the_mask() {
        let value = Value::from_u32(7);
        assert_eq!(U32::new(value.clone()).value().u32(), 7);
        assert_eq!(U32F32::new(value.clone()).into_value().u32(), 7);
        assert_eq!(
            U8U16U32U64::MASK.bits(),
            (1 << 7) | (1 << 8) | (1 << 9) | (1 << 10)
        );
        assert!(U64::MASK.bits() == 1 << 10);
        assert!(U32F32::MASK.contains(Type::F32));
        // A value of a type the mask does not cover is refused.
        let result = std::panic::catch_unwind(|| U64::new(Value::from_u32(1)));
        assert!(result.is_err(), "incompatible types are rejected");
        let inst = Rc::new(RefCell::new(Inst::new(Opcode::ReadConst, 0)));
        let typed = U32::from_inst(inst);
        assert_eq!(typed.value().ty(), Type::U32);
    }
}
