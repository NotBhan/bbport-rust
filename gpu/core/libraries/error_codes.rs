//! Generic ORBIS return codes, ported from
//! `legacy/gpu/shadps4/core/libraries/error_codes.h`.

/// `ORBIS_OK`.
pub const ORBIS_OK: i32 = 0x0000_0000;
/// `ORBIS_FAIL`.
pub const ORBIS_FAIL: i32 = 0xFFFF_FFFFu32 as i32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_generic_codes_match_the_header() {
        assert_eq!(ORBIS_OK, 0);
        assert_eq!(ORBIS_FAIL, -1);
    }
}
