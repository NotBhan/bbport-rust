//! String helpers, ported from `legacy/gpu/shadps4/common/string_util.h`. The UTF-16 pair is
//! Windows-only in the C++ because `wchar_t` is 16 bits there; the Rust port keeps UTF-16 as
//! an explicit unit type on every platform.

/// `Common::ToLower`: the C++ maps `tolower` over the bytes, so only ASCII folds.
pub fn to_lower(text: &str) -> String {
    text.to_ascii_lowercase()
}

/// `Common::ToLowerInPlace`.
pub fn to_lower_in_place(text: &mut str) {
    // A `&mut str` cannot be reassigned, so the bytes are folded where they lie. Folding
    // ASCII only touches bytes below 0x80, so the text stays valid UTF-8.
    let bytes = unsafe { text.as_bytes_mut() };
    for byte in bytes {
        byte.make_ascii_lowercase();
    }
}

/// `Common::SplitString`: empty segments are kept, as the C++ does.
pub fn split_string(text: &str, delimiter: char) -> Vec<String> {
    text.split(delimiter).map(|part| part.to_string()).collect()
}

/// `Common::U8stringToString`: the bytes are already UTF-8 in Rust; invalid text is dropped.
pub fn u8string_to_string(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap_or("")
}

/// `Common::UTF16ToUTF8`.
pub fn utf16_to_utf8(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
}

/// `Common::UTF8ToUTF16W`.
pub fn utf8_to_utf16(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercasing_folds_ascii_only() {
        assert_eq!(to_lower("UPSCALER=Fsr3"), "upscaler=fsr3");
        let mut text = String::from("BB_UPSCALER");
        to_lower_in_place(&mut text);
        assert_eq!(text, "bb_upscaler");
        // `tolower` is byte-wise in the C++, so a non-ASCII byte is left where it is.
        assert_eq!(to_lower("Üpscaler"), "Üpscaler");
    }

    #[test]
    fn splitting_keeps_empty_segments() {
        assert_eq!(split_string("a,b,,c", ','), ["a", "b", "", "c"]);
        assert_eq!(split_string("", ','), [""]);
        assert_eq!(split_string("a;b", ';'), ["a", "b"]);
    }

    #[test]
    fn the_utf16_pair_round_trips() {
        let text = "Trophy 🏆";
        let units = utf8_to_utf16(text);
        assert_eq!(
            units.len(),
            text.chars().count() + 1,
            "the astral character takes two units"
        );
        assert_eq!(utf16_to_utf8(&units), text);
        assert_eq!(
            utf16_to_utf8(&[0xD83C]),
            "\u{FFFD}",
            "a lone surrogate is replaced"
        );
        assert_eq!(u8string_to_string(b"bbport.ini"), "bbport.ini");
        assert_eq!(u8string_to_string(&[0xff, 0xfe]), "");
    }
}
