//! Bit field access, ported from `legacy/gpu/shadps4/common/bit_field.h`. The C++ `BitField` sits
//! in a union beside the raw word and reads and writes it in place; the Rust port keeps the same
//! arithmetic over the storage word and makes the raw access explicit.

/// The integer types a bit field can be stored in, plus the signed extraction the C++ does for
/// a signed `T`.
pub trait BitFieldStorage: Copy + PartialEq + std::fmt::Debug {
    /// How many bits the type holds.
    const WIDTH: usize;
    /// Whether the C++ would sign-extend the extracted value.
    const SIGNED: bool;

    fn from_u64(value: u64) -> Self;
    fn to_u64(self) -> u64;
    /// Sign-extends the low `bits` of `value` when the type is signed.
    fn sign_extend(value: u64, bits: usize) -> Self;
}

macro_rules! unsigned_storage {
    ($($t:ty),*) => {$(
        impl BitFieldStorage for $t {
            const WIDTH: usize = <$t>::BITS as usize;
            const SIGNED: bool = false;

            fn from_u64(value: u64) -> Self {
                value as $t
            }

            fn to_u64(self) -> u64 {
                self as u64
            }

            fn sign_extend(value: u64, _bits: usize) -> Self {
                value as $t
            }
        }
    )*};
}

macro_rules! signed_storage {
    ($($t:ty),*) => {$(
        impl BitFieldStorage for $t {
            const WIDTH: usize = <$t>::BITS as usize;
            const SIGNED: bool = true;

            fn from_u64(value: u64) -> Self {
                value as $t
            }

            fn to_u64(self) -> u64 {
                self as u64
            }

            fn sign_extend(value: u64, bits: usize) -> Self {
                let shift = Self::WIDTH - bits;
                (((value << shift) as $t) >> shift) as $t
            }
        }
    )*};
}

unsigned_storage!(u8, u16, u32, u64);
signed_storage!(i8, i16, i32, i64);

/// `BitField<Position, Bits, T>`: the bit field itself, which is the storage word.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BitField<T, const POSITION: usize, const BITS: usize>(T);

impl<T: BitFieldStorage, const POSITION: usize, const BITS: usize> BitField<T, POSITION, BITS> {
    /// `BitField::position`.
    pub const POSITION: usize = POSITION;
    /// `BitField::bits`.
    pub const BITS: usize = BITS;
    /// `BitField::mask`.
    pub const MASK: u64 = if BITS >= T::WIDTH {
        u64::MAX
    } else {
        ((1u64 << BITS) - 1) << POSITION
    };

    const _CHECK: () = {
        assert!(BITS > 0, "Invalid number of bits");
        assert!(POSITION < T::WIDTH, "Invalid position");
        assert!(BITS <= T::WIDTH, "Invalid number of bits");
        assert!(BITS + POSITION <= T::WIDTH, "Bitfield out of range");
    };

    /// A field over a raw storage word, as the union's other member would be read.
    pub const fn from_raw(storage: T) -> Self {
        let () = Self::_CHECK;
        Self(storage)
    }

    /// The whole storage word, mask and all.
    pub const fn raw(self) -> T {
        self.0
    }

    /// `BitField::FormatValue`.
    pub fn format_value(value: T) -> T {
        T::from_u64((value.to_u64() << POSITION) & Self::MASK)
    }

    /// `BitField::ExtractValue`.
    pub fn extract_value(storage: T) -> T {
        let shifted = (storage.to_u64() & Self::MASK) >> POSITION;
        if T::SIGNED {
            T::sign_extend(shifted, BITS)
        } else {
            T::from_u64(shifted)
        }
    }

    /// `BitField::Value`.
    pub fn value(self) -> T {
        Self::extract_value(self.0)
    }

    /// `BitField::Assign`: writes this field and leaves the rest of the word alone.
    pub fn assign(&mut self, value: T) {
        let cleared = self.0.to_u64() & !Self::MASK;
        self.0 = T::from_u64(cleared | Self::format_value(value).to_u64());
    }

    /// The C++'s `explicit operator bool`.
    pub fn is_set(self) -> bool {
        self.value().to_u64() != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type FirstSeven = BitField<u32, 0, 7>;
    type NextEight = BitField<u32, 7, 8>;
    type SignedFifteen = BitField<i32, 3, 15>;
    type HighBit = BitField<u64, 63, 1>;

    #[test]
    fn a_field_reads_and_writes_its_bits_only() {
        let mut word: u32 = 0;
        let mut first = FirstSeven::from_raw(word);
        first.assign(0x7F);
        word = first.raw();
        assert_eq!(word, 0x7F);
        assert_eq!(first.value(), 0x7F);

        let mut next = NextEight::from_raw(word);
        next.assign(0xAB);
        word = next.raw();
        assert_eq!(word, 0x55FF, "the low field is untouched");
        assert_eq!(first.raw(), 0x7F, "each field sees the whole word");
        assert_eq!(NextEight::from_raw(word).value(), 0xAB);

        // Format, mask and extract agree with the header's arithmetic.
        assert_eq!(FirstSeven::MASK, 0x7F);
        assert_eq!(FirstSeven::format_value(0x1FF), 0x7F);
        assert_eq!(FirstSeven::extract_value(0x80), 0);
        assert_eq!(NextEight::MASK, 0x7F80);
        const { assert!(HighBit::MASK == 1 << 63) };
        let high = HighBit::from_raw(1 << 63);
        assert!(high.is_set() && high.value() == 1);
        assert!(!HighBit::from_raw(0).is_set());
    }

    #[test]
    fn a_signed_field_sign_extends_like_the_cpp() {
        // The field spans bits 3..18 of the word.
        let mut field = SignedFifteen::from_raw(0u32 as i32);
        field.assign(0x3FFF);
        assert_eq!(field.value(), 0x3FFF, "a positive value stays positive");
        field.assign(0x4000);
        assert_eq!(field.value(), -0x4000, "the field's top bit is the sign");
        field.assign(0x7FFF);
        assert_eq!(field.value(), -1, "all fifteen bits set is -1");
        // The mask is fifteen bits wide, shifted up by the field's position.
        assert_eq!(SignedFifteen::MASK, 0x3FFF8);
        field.assign(-1);
        assert_eq!(field.raw() as u32 & SignedFifteen::MASK as u32, 0x3FFF8);
        assert_eq!(field.value(), -1);
        assert_eq!(SignedFifteen::POSITION, 3);
        assert_eq!(SignedFifteen::BITS, 15);
        const { assert!(i32::SIGNED && !u32::SIGNED) };
    }
}
