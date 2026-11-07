//! String literals carrying their size, ported from `legacy/gpu/shadps4/common/string_literal.h`.
//!
//! The C++ `StringLiteral<N>` copies the N bytes of a string literal (including the terminator);
//! the Rust port keeps the same array so sizes stay part of the type.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StringLiteral<const N: usize> {
    pub value: [u8; N],
}

impl<const N: usize> StringLiteral<N> {
    /// `StringLiteral::len`: the array size, terminator included.
    pub const LEN: usize = N;

    /// The constructor's `std::copy_n(str, N, value)`.
    pub const fn new(value: [u8; N]) -> Self {
        Self { value }
    }

    /// The bytes without the trailing terminator, as text.
    pub fn as_str(&self) -> &str {
        let end = self.value.iter().position(|byte| *byte == 0).unwrap_or(N);
        std::str::from_utf8(&self.value[..end]).unwrap_or("")
    }
}

impl<const N: usize> From<&[u8; N]> for StringLiteral<N> {
    fn from(value: &[u8; N]) -> Self {
        Self::new(*value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_size_includes_the_terminator_and_the_text_does_not() {
        let literal = StringLiteral::from(b"TrophyKeySet\0");
        assert_eq!(StringLiteral::<13>::LEN, 13);
        assert_eq!(literal.as_str(), "TrophyKeySet");
        assert_eq!(literal.value.len(), 13);
        // A literal without a terminator reads to its end.
        assert_eq!(StringLiteral::from(b"abc").as_str(), "abc");
    }
}
