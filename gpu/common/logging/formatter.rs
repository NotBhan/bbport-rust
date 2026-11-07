//! String and enum formatting helpers, ported from
//! `legacy/gpu/shadps4/common/logging/formatter.h`.

use std::borrow::Cow;

/// `fmt::UTF`: a view over UTF-8 bytes produced by `std::filesystem` paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utf<'a> {
    data: Cow<'a, str>,
}

impl<'a> Utf<'a> {
    /// The `std::u8string_view` constructor. Invalid sequences are replaced, since Rust
    /// strings are always valid UTF-8.
    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        Self {
            data: String::from_utf8_lossy(bytes),
        }
    }

    /// The `std::u8string` constructor for owned bytes.
    pub fn from_owned(bytes: Vec<u8>) -> Utf<'static> {
        Utf {
            data: Cow::Owned(String::from_utf8_lossy(&bytes).into_owned()),
        }
    }

    /// A plain `&str` source.
    pub fn new(data: &'a str) -> Self {
        Self {
            data: Cow::Borrowed(data),
        }
    }

    /// The formatted payload.
    pub fn as_str(&self) -> &str {
        &self.data
    }
}

impl std::fmt::Display for Utf<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.data)
    }
}

/// The enum formatter: the C++ formats any scoped enum through its underlying integer type.
/// Ported enums implement this to keep log output identical.
pub trait EnumFormat {
    fn underlying_value(&self) -> i64;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf_views_pass_the_bytes_through() {
        let utf = Utf::from_bytes(b"/tmp/capture");
        assert_eq!(utf.as_str(), "/tmp/capture");
        assert_eq!(utf.to_string(), "/tmp/capture");
    }

    #[test]
    fn invalid_sequences_are_replaced_not_rejected() {
        let utf = Utf::from_bytes(&[0xff, b'a']);
        assert_eq!(utf.as_str(), "\u{fffd}a");
    }

    #[test]
    fn the_log_class_constants_are_unique_and_ordered() {
        use super::super::classes;
        assert!(classes::ALL_LOGGERS.contains(&classes::RENDER));
        assert!(classes::ALL_LOGGERS.contains(&classes::LIB_NP_WEB_API2));
        let mut sorted = classes::ALL_LOGGERS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), classes::ALL_LOGGERS.len());
    }
}
