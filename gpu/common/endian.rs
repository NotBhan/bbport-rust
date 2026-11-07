//! Endian wrappers, ported from `legacy/gpu/shadps4/common/endian.h`.
//!
//! The C++ aliases pick `NativeEndian` or `SwappedEndian` at compile time; the port keeps the
//! same split with `cfg(target_endian)` aliases and a generic `SwappedEndian<T>`.

#![allow(non_camel_case_types)]

use std::fmt;

/// The set of types the C++ accepts for `SwappedEndian` (`std::integral`).
pub trait SwapBytes: Copy {
    fn swap_bytes(self) -> Self;
}

macro_rules! impl_swap_bytes {
    ($($ty:ty),* $(,)?) => {
        $(
            impl SwapBytes for $ty {
                fn swap_bytes(self) -> Self {
                    <$ty>::swap_bytes(self)
                }
            }
        )*
    };
}

impl_swap_bytes!(u16, u32, u64, i16, i32, i64);

/// `Common::SwappedEndian`: a value stored byte-swapped from the native order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SwappedEndian<T: SwapBytes>(T);

impl<T: SwapBytes> SwappedEndian<T> {
    /// `Raw`.
    pub fn raw(self) -> T {
        self.0
    }

    /// `Swap`.
    pub fn swap(self) -> T {
        self.0.swap_bytes()
    }

    /// `FromRaw`.
    pub fn from_raw(value: T) -> Self {
        Self(value)
    }

    /// `FromSwap`.
    pub fn from_swap(value: T) -> Self {
        Self(value.swap_bytes())
    }

    /// The implicit conversion to `T`.
    pub fn get(self) -> T {
        self.swap()
    }

    /// `operator=`.
    pub fn set(&mut self, value: T) {
        *self = Self::from_swap(value);
    }
}

impl<T: SwapBytes + Default> SwappedEndian<T> {
    /// `SwappedEndian()`.
    pub fn new() -> Self {
        Self(T::default())
    }

    /// The C++ default constructor zeroes the raw word.
    pub fn zeroed() -> Self {
        Self(T::default())
    }
}

impl<T: SwapBytes> From<T> for SwappedEndian<T> {
    fn from(value: T) -> Self {
        Self::from_swap(value)
    }
}

impl<T: SwapBytes + fmt::Display> fmt::Display for SwappedEndian<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.swap().fmt(formatter)
    }
}

/// `Common::NativeEndian<T>`.
pub type NativeEndian<T> = T;

#[cfg(target_endian = "little")]
/// `Common::LittleEndian<T>`.
pub type LittleEndian<T> = NativeEndian<T>;
#[cfg(target_endian = "big")]
/// `Common::LittleEndian<T>`.
pub type LittleEndian<T> = SwappedEndian<T>;

#[cfg(target_endian = "big")]
/// `Common::BigEndian<T>`.
pub type BigEndian<T> = NativeEndian<T>;
#[cfg(target_endian = "little")]
/// `Common::BigEndian<T>`.
pub type BigEndian<T> = SwappedEndian<T>;

/// `u16_be`.
pub type u16_be = BigEndian<u16>;
/// `u32_be`.
pub type u32_be = BigEndian<u32>;
/// `u64_be`.
pub type u64_be = BigEndian<u64>;
/// `u16_le`.
pub type u16_le = LittleEndian<u16>;
/// `u32_le`.
pub type u32_le = LittleEndian<u32>;
/// `u64_le`.
pub type u64_le = LittleEndian<u64>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_swapped_value_round_trips() {
        let value = SwappedEndian::<u32>::from(0x1122_3344);
        assert_eq!(value.get(), 0x1122_3344);
        assert_eq!(value.raw(), 0x1122_3344u32.swap_bytes());
        let raw = SwappedEndian::<u32>::from_raw(0x4433_2211);
        assert_eq!(raw.get(), 0x1122_3344);
        let swap = SwappedEndian::<u32>::from_swap(0x1122_3344);
        assert_eq!(swap.get(), 0x1122_3344);
        assert_eq!(value.get(), 0x1122_3344);
    }

    #[test]
    fn the_aliases_follow_the_host_order() {
        #[cfg(target_endian = "little")]
        {
            assert_eq!(core::mem::size_of::<u32_le>(), 4);
            assert_eq!(
                u32_le::from(0x1122_3344u32),
                0x1122_3344,
                "little-endian on a little host is the native word"
            );
            assert_eq!(u32_be::from(0x1122_3344u32).get(), 0x1122_3344);
        }
    }

    #[test]
    fn set_writes_the_swapped_form() {
        let mut value = SwappedEndian::<u16>::new();
        value.set(0x1234);
        assert_eq!(value.get(), 0x1234);
        assert_eq!(value.raw(), 0x3412);
    }
}
