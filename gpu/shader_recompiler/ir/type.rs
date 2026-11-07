//! Value types, ported from `legacy/gpu/shadps4/shader_recompiler/ir/type.h` and its `.cpp`.
//! The C++ enum is a bit mask with flag operators; the port keeps the same bit values behind a
//! small newtype so the flag combinations stay meaningful.

use std::fmt;

/// `Shader::IR::Type`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Type(u32);

macro_rules! define_types {
    ($void:ident = $void_value:expr; $($name:ident = $value:expr),* $(,)?) => {
        impl Type {
            #[allow(non_upper_case_globals)]
            pub const $void: Type = Type($void_value);

            $(
                #[allow(non_upper_case_globals)]
                pub const $name: Type = Type($value);
            )*

            /// Every named type, in bit order — the order `NameOf` walks. `Void` is not in the
            /// list, matching the C++'s names array: it is the empty type, and carries no bit.
            pub const ALL: &'static [Type] = &[$(Type::$name),*];
        }
    };
}

define_types! {
    Void = 0;
    Opaque = 1 << 0,
    ScalarReg = 1 << 1,
    VectorReg = 1 << 2,
    VirtualReg = 1 << 3,
    Attribute = 1 << 4,
    Patch = 1 << 5,
    U1 = 1 << 6,
    U8 = 1 << 7,
    U16 = 1 << 8,
    U32 = 1 << 9,
    U64 = 1 << 10,
    F16 = 1 << 11,
    F32 = 1 << 12,
    F64 = 1 << 13,
    U32x2 = 1 << 14,
    U32x3 = 1 << 15,
    U32x4 = 1 << 16,
    F16x2 = 1 << 17,
    F16x3 = 1 << 18,
    F16x4 = 1 << 19,
    F32x2 = 1 << 20,
    F32x3 = 1 << 21,
    F32x4 = 1 << 22,
    F64x2 = 1 << 23,
    F64x3 = 1 << 24,
    F64x4 = 1 << 25,
    StringLiteral = 1 << 26,
}

impl Type {
    /// The raw bits, as `static_cast<size_t>(type)` gives them.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// The C++'s implicit conversion from its integer value.
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// Whether the type is empty, which is only `Void`.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether every bit of `other` is set here.
    pub const fn contains(self, other: Type) -> bool {
        self.0 & other.0 == other.0
    }

    /// The C++'s `operator|`.
    pub const fn union(self, other: Type) -> Type {
        Type(self.0 | other.0)
    }

    /// The C++'s `operator&`.
    pub const fn intersection(self, other: Type) -> Type {
        Type(self.0 & other.0)
    }

    /// The C++'s `operator~`, masked to the named bits.
    pub const fn inverted(self) -> Type {
        Type(!self.0 & ((1 << 27) - 1))
    }
}

impl fmt::Debug for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&name_of(*self))
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&name_of(*self))
    }
}

/// `Shader::IR::NameOf`: `Void` for the empty type, else the set bits joined by `|`.
pub fn name_of(ty: Type) -> String {
    const NAMES: [&str; 27] = [
        "Opaque",
        "ScalarReg",
        "VectorReg",
        "VirtualReg",
        "Attribute",
        "Patch",
        "U1",
        "U8",
        "U16",
        "U32",
        "U64",
        "F16",
        "F32",
        "F64",
        "U32x2",
        "U32x3",
        "U32x4",
        "F16x2",
        "F16x3",
        "F16x4",
        "F32x2",
        "F32x3",
        "F32x4",
        "F64x2",
        "F64x3",
        "F64x4",
        "StringLiteral",
    ];
    let bits = ty.bits();
    if bits == 0 {
        return "Void".to_string();
    }
    let mut result = String::new();
    for (index, name) in NAMES.iter().enumerate() {
        if bits & (1 << index) != 0 {
            if !result.is_empty() {
                result.push('|');
            }
            result.push_str(name);
        }
    }
    result
}

/// `Shader::IR::AreTypesCompatible`: an opaque matches anything.
pub fn are_types_compatible(lhs: Type, rhs: Type) -> bool {
    lhs == rhs || lhs == Type::Opaque || rhs == Type::Opaque
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bit_values_are_the_headers() {
        assert_eq!(Type::Void.bits(), 0);
        assert_eq!(Type::Opaque.bits(), 1);
        assert_eq!(Type::U1.bits(), 1 << 6);
        assert_eq!(Type::U32.bits(), 1 << 9);
        assert_eq!(Type::F32.bits(), 1 << 12);
        assert_eq!(Type::StringLiteral.bits(), 1 << 26);
        assert!(Type::Void.is_empty());
        assert!(!Type::U1.is_empty());
        assert_eq!(Type::from_bits(Type::F64x4.bits()), Type::F64x4);
        assert_eq!(Type::ALL.len(), 27, "the names array holds no Void");
        assert_eq!(Type::ALL[0], Type::Opaque);
        assert_eq!(Type::ALL[26], Type::StringLiteral);
    }

    #[test]
    fn names_come_out_per_bit_and_in_bit_order() {
        assert_eq!(name_of(Type::Void), "Void");
        assert_eq!(name_of(Type::U32), "U32");
        assert_eq!(name_of(Type::Attribute), "Attribute");
        assert_eq!(
            name_of(Type::F32.union(Type::U32x2)),
            "F32|U32x2",
            "the C++ joins set bits with a pipe"
        );
        assert_eq!(format!("{}", Type::F16x4), "F16x4");
        assert_eq!(format!("{:?}", Type::Opaque), "Opaque");
        // A combination not in the named list still names the bits it has.
        assert_eq!(name_of(Type::from_bits(0x3)), "Opaque|ScalarReg");
    }

    #[test]
    fn compatibility_is_equality_or_opacity() {
        assert!(are_types_compatible(Type::U32, Type::U32));
        assert!(!are_types_compatible(Type::U32, Type::F32));
        assert!(are_types_compatible(Type::Opaque, Type::F32));
        assert!(are_types_compatible(Type::U32, Type::Opaque));
        assert!(are_types_compatible(Type::Opaque, Type::Opaque));
        assert!(Type::from_bits(0b101).contains(Type::Opaque));
        assert!(!Type::U32.contains(Type::Opaque));
        assert_eq!(Type::U32.intersection(Type::U32), Type::U32);
        assert!(Type::U32.inverted().contains(Type::F32));
        assert!(!Type::U32.inverted().contains(Type::U32));
        assert_eq!(
            Type::from_bits(0b101).union(Type::from_bits(0b010)).bits(),
            0b111
        );
    }
}
