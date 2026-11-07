//! Fixed-size null-terminated strings, ported from `legacy/gpu/shadps4/common/cstring.h`.

/// The character types `CString` is instantiated with. `wchar_t` is 32-bit on the Linux
/// target, so `CWString` uses `u32`.
pub trait CStringChar: Copy + Default + PartialEq {
    const NUL: Self;
}

impl CStringChar for u8 {
    const NUL: Self = 0;
}

impl CStringChar for u16 {
    const NUL: Self = 0;
}

impl CStringChar for u32 {
    const NUL: Self = 0;
}

/// `Common::CString`: a fixed array with a NUL-terminated view.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CString<C: CStringChar, const N: usize> {
    data: [C; N],
}

impl<C: CStringChar, const N: usize> Default for CString<C, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: CStringChar, const N: usize> CString<C, N> {
    /// `CString()`.
    pub fn new() -> Self {
        Self { data: [C::NUL; N] }
    }

    /// The templated `CString(const CString<M>&)`, storing up to `N - 1` characters.
    pub fn from_chars(chars: &[C]) -> Self {
        let mut string = Self::new();
        let count = chars.len().min(N.saturating_sub(1));
        string.data[..count].copy_from_slice(&chars[..count]);
        string
    }

    /// `FromString` for a character slice.
    pub fn from_slice(&mut self, chars: &[C]) {
        self.zero();
        let count = chars.len().min(N.saturating_sub(1));
        self.data[..count].copy_from_slice(&chars[..count]);
    }

    /// `Zero`.
    pub fn zero(&mut self) {
        self.data.fill(C::NUL);
    }

    /// The NUL-terminated character slice.
    pub fn as_chars(&self) -> &[C] {
        let length = self
            .data
            .iter()
            .position(|character| *character == C::NUL)
            .unwrap_or(N);
        &self.data[..length]
    }

    /// `to_string` for the narrow string.
    pub fn to_string_lossy(&self) -> String
    where
        C: Into<u32>,
    {
        self.as_chars()
            .iter()
            .map(|character| char::from_u32((*character).into()).unwrap_or('\u{fffd}'))
            .collect()
    }

    /// `capacity`.
    pub const fn capacity(&self) -> usize {
        N
    }

    /// `size`.
    pub fn size(&self) -> usize {
        self.as_chars().len()
    }

    /// `operator[]`.
    pub fn get(&self, index: usize) -> Option<&C> {
        self.data.get(index)
    }

    /// The mutable `operator[]`.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut C> {
        self.data.get_mut(index)
    }

    /// The whole backing array, including the terminator.
    pub fn as_array(&self) -> &[C; N] {
        &self.data
    }

    /// The whole backing array, mutably.
    pub fn as_array_mut(&mut self) -> &mut [C; N] {
        &mut self.data
    }

    /// `begin()`/`end()` over the backing array.
    pub fn iter(&self) -> impl Iterator<Item = &C> {
        self.data.iter()
    }
}

impl<const N: usize> From<&str> for CString<u8, N> {
    fn from(value: &str) -> Self {
        Self::from_chars(value.as_bytes())
    }
}

impl<const N: usize> CString<u8, N> {
    /// A UTF-8 view of the bytes, when they are valid text.
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(self.as_chars()).ok()
    }
}

/// `Common::CWString`.
pub type CWString<const N: usize> = CString<u32, N>;
/// `Common::CU16String`.
pub type CU16String<const N: usize> = CString<u16, N>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layout_is_the_same_as_a_plain_array() {
        assert_eq!(
            core::mem::size_of::<CString<u8, 13>>(),
            core::mem::size_of::<[u8; 13]>()
        );
    }

    #[test]
    fn a_narrow_string_stops_at_the_terminator() {
        let string = CString::<u8, 8>::from("hello");
        assert_eq!(string.as_str(), Some("hello"));
        assert_eq!(string.size(), 5);
        assert_eq!(string.capacity(), 8);
        assert_eq!(string.get(0), Some(&b'h'));
        assert_eq!(string.get(7), Some(&0));
        let mut zeroed = string;
        zeroed.zero();
        assert_eq!(zeroed.size(), 0);
    }

    #[test]
    fn construction_truncates_to_leave_room_for_the_terminator() {
        let string = CString::<u8, 4>::from("abcdef");
        assert_eq!(string.as_chars(), b"abc");
        assert_eq!(string.get(3), Some(&0));
    }

    #[test]
    fn the_wide_aliases_store_their_own_width() {
        let wide = CWString::<4>::from_chars(&['a' as u32, 'b' as u32]);
        assert_eq!(wide.as_chars(), &[b'a' as u32, b'b' as u32]);
        assert_eq!(wide.size(), 2);
        let utf16 = CU16String::<4>::from_chars(&[0x41, 0x42]);
        assert_eq!(utf16.size(), 2);
    }
}
