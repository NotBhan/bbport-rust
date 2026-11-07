//! Enum flag helpers, ported from `legacy/gpu/shadps4/common/enum.h`. A keyword keeps the
//! module at `common::r#enum`.
//!
//! `DECLARE_ENUM_FLAG_OPERATORS` gives a C++ enum `|`, `&`, `^`, `~`, shifts and `+=`-style
//! combining, all of which can produce values that name no enumerator. A Rust enum cannot hold
//! those, so the macro here keeps the same arithmetic over the underlying bits and hands back an
//! integer where the C++ handed back a value of the enum type. `Flags<T>` is the storage type
//! the C++ uses on top of that, and keeps its full API.

use std::marker::PhantomData;
use std::ops::{BitAnd, BitOr, BitXor};

/// The `enum_flag_operators!` macro's requirement: an enum over 32-bit values, as the C++'s
/// `std::underlying_type_t<T>` gives.
pub trait FlagBits: Copy + PartialEq {
    /// The enumerator's value, `static_cast<IntType>(f)`.
    fn index(self) -> u32;
}

/// `DECLARE_ENUM_FLAG_OPERATORS(type)`. The operators live on the trait because Rust cannot add
/// operators to a foreign or later-defined enum; the difference is that a combination that
/// names no enumerator comes back as `u32` instead of a value of the enum type.
#[macro_export]
macro_rules! enum_flag_operators {
    ($type:ty) => {
        impl $crate::gpu::common::r#enum::FlagBits for $type {
            fn index(self) -> u32 {
                self as u32
            }
        }

        impl $crate::gpu::common::r#enum::EnumFlagOperators for $type {
            fn union(self, other: Self) -> u32 {
                self as u32 | other as u32
            }

            fn intersection(self, other: Self) -> u32 {
                self as u32 & other as u32
            }

            fn toggle(self, other: Self) -> u32 {
                self as u32 ^ other as u32
            }

            fn shift_left(self, other: Self) -> u32 {
                (self as u32) << (other as u32)
            }

            fn shift_right(self, other: Self) -> u32 {
                (self as u32) >> (other as u32)
            }

            fn inverted(self) -> u32 {
                !(self as u32)
            }

            fn is_true(self) -> bool {
                self as u32 != 0
            }

            fn is_false(self) -> bool {
                self as u32 == 0
            }
        }
    };
}

/// The C++'s `operator|`, `operator&`, `operator^`, `operator~`, the shifts, and the `True` and
/// `False` helpers, over an enum whose values are bit patterns.
pub trait EnumFlagOperators: FlagBits {
    fn union(self, other: Self) -> u32;
    fn intersection(self, other: Self) -> u32;
    fn toggle(self, other: Self) -> u32;
    fn shift_left(self, other: Self) -> u32;
    fn shift_right(self, other: Self) -> u32;
    fn inverted(self) -> u32;
    fn is_true(self) -> bool;
    fn is_false(self) -> bool;
}

/// `Common::Flags<T>`: a set of an enum's bits, where each enumerator stands for a bit *index*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flags<T: FlagBits> {
    bits: u32,
    marker: PhantomData<T>,
}

impl<T: FlagBits> Flags<T> {
    /// `Flags()`: nothing set.
    pub fn new() -> Self {
        Self {
            bits: 0,
            marker: PhantomData,
        }
    }

    /// `Flags(IntType)`: the bits this way.
    pub fn from_bits(bits: u32) -> Self {
        Self {
            bits,
            marker: PhantomData,
        }
    }

    /// The C++'s `bit(f)`: an enumerator's *value* is the bit's index.
    fn bit(f: T) -> u32 {
        1u32 << f.index()
    }

    fn bits_of(flags: &[T]) -> u32 {
        flags.iter().fold(0, |bits, f| bits | Self::bit(*f))
    }

    /// `Flags::set(T f, Tx... fx)`.
    pub fn set(&mut self, f: T) {
        self.bits |= Self::bit(f);
    }

    /// `Flags::set(T f, Tx... fx)` with more than one enumerator.
    pub fn set_all(&mut self, flags: &[T]) {
        self.bits |= Self::bits_of(flags);
    }

    /// `Flags::set(Flags flags)`.
    pub fn set_flags(&mut self, flags: Flags<T>) {
        self.bits |= flags.bits;
    }

    /// `Flags::clr(T f, Tx... fx)`.
    pub fn clr(&mut self, f: T) {
        self.bits &= !Self::bit(f);
    }

    /// `Flags::clr(T f, Tx... fx)` with more than one enumerator.
    pub fn clr_all_of(&mut self, flags: &[T]) {
        self.bits &= !Self::bits_of(flags);
    }

    /// `Flags::clr(Flags flags)`.
    pub fn clr_flags(&mut self, flags: Flags<T>) {
        self.bits &= !flags.bits;
    }

    /// `Flags::any(T f, Tx... fx)`.
    pub fn any(&self, f: T) -> bool {
        self.bits & Self::bit(f) != 0
    }

    /// `Flags::any(T f, Tx... fx)` with more than one enumerator.
    pub fn any_of(&self, flags: &[T]) -> bool {
        self.bits & Self::bits_of(flags) != 0
    }

    /// `Flags::all(T f, Tx... fx)`.
    pub fn all(&self, f: T) -> bool {
        let mask = Self::bit(f);
        self.bits & mask == mask
    }

    /// `Flags::all(T f, Tx... fx)` with more than one enumerator.
    pub fn all_of(&self, flags: &[T]) -> bool {
        let mask = Self::bits_of(flags);
        self.bits & mask == mask
    }

    /// `Flags::test(T f)`.
    pub fn test(&self, f: T) -> bool {
        self.any(f)
    }

    /// `Flags::isClear()`.
    pub fn is_clear(&self) -> bool {
        self.bits == 0
    }

    /// `Flags::clrAll()`.
    pub fn clr_all(&mut self) {
        self.bits = 0;
    }

    /// `Flags::raw()`.
    pub fn raw(&self) -> u32 {
        self.bits
    }
}

impl<T: FlagBits> Default for Flags<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: FlagBits> std::hash::Hash for Flags<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bits.hash(state);
    }
}

impl<T: FlagBits> BitAnd for Flags<T> {
    type Output = Flags<T>;

    fn bitand(self, other: Flags<T>) -> Flags<T> {
        Flags::from_bits(self.bits & other.bits)
    }
}

impl<T: FlagBits> BitOr for Flags<T> {
    type Output = Flags<T>;

    fn bitor(self, other: Flags<T>) -> Flags<T> {
        Flags::from_bits(self.bits | other.bits)
    }
}

impl<T: FlagBits> BitXor for Flags<T> {
    type Output = Flags<T>;

    fn bitxor(self, other: Flags<T>) -> Flags<T> {
        Flags::from_bits(self.bits ^ other.bits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u32)]
    enum Feature {
        First = 0,
        Second = 1,
        Third = 4,
        Fourth = 31,
    }

    enum_flag_operators!(Feature);

    #[test]
    fn flags_set_and_test_bit_indices() {
        let mut flags = Flags::<Feature>::new();
        assert!(flags.is_clear());
        assert_eq!(flags.raw(), 0);
        flags.set(Feature::First);
        flags.set(Feature::Second);
        assert_eq!(flags.raw(), 0b11, "the enumerator's value is the bit index");
        assert!(flags.test(Feature::First) && flags.any(Feature::Second));
        assert!(!flags.any(Feature::Third));
        assert!(flags.all_of(&[Feature::First, Feature::Second]));
        assert!(!flags.all_of(&[Feature::First, Feature::Third]));
        assert!(flags.any_of(&[Feature::Third, Feature::Second]));
        assert!(!flags.any_of(&[Feature::Third]));

        // The third enumerator names bit 4, not bit 2.
        flags.set(Feature::Third);
        assert_eq!(flags.raw(), 0b1_0011);
        flags.clr(Feature::Second);
        assert_eq!(flags.raw(), 0b1_0001);
        assert_eq!(flags, Flags::from_bits(0b1_0001));
        flags.set_all(&[Feature::Second, Feature::Third]);
        assert_eq!(flags.raw(), 0b1_0011);
        flags.clr_all_of(&[Feature::First, Feature::Second]);
        assert_eq!(flags.raw(), 0b1_0000);
        flags.set_flags(Flags::from_bits(0b10));
        assert_eq!(flags.raw(), 0b1_0010);
        flags.clr_flags(Flags::from_bits(0b10));
        assert_eq!(flags.raw(), 0b1_0000);
        flags.clr_all();
        assert!(flags.is_clear());
    }

    #[test]
    fn flags_combine_with_the_bit_operators() {
        let first = Flags::<Feature>::from_bits(0b01);
        let second = Flags::<Feature>::from_bits(0b10);
        assert_eq!((first | second).raw(), 0b11);
        assert_eq!((first & second).raw(), 0);
        assert_eq!((first ^ second).raw(), 0b11);
        assert_eq!((first | first & second).raw(), 0b01, "the C++'s precedence");
        assert_ne!(first, second);
        assert_eq!(first, Flags::from_bits(0b01));
        // A flag at the top of the word round-trips.
        let top = Flags::<Feature>::from_bits(0);
        let mut top = top;
        top.set(Feature::Fourth);
        assert_eq!(top.raw(), 1 << 31);
        assert!(top.any(Feature::Fourth));
        assert_eq!(top, Flags::from_bits(1 << 31));
        assert_eq!(Flags::<Feature>::default().raw(), 0);
    }

    #[test]
    fn the_enum_operators_work_on_the_underlying_bits() {
        assert_eq!(Feature::First.union(Feature::Second), 1);
        assert_eq!(Feature::First.intersection(Feature::Second), 0);
        assert_eq!(Feature::First.toggle(Feature::Second), 1);
        assert_eq!(Feature::Second.shift_left(Feature::Second), 0b10);
        assert_eq!(Feature::Third.shift_right(Feature::First), 0b100);
        assert_eq!(Feature::First.inverted(), u32::MAX);
        assert!(Feature::Second.is_true());
        assert!(Feature::First.is_false());
        assert_eq!(Feature::First.index(), 0);
        assert_eq!(Feature::Third.index(), 4);
    }
}
