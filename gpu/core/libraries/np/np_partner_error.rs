//! NP partner error codes, ported from
//! `legacy/gpu/bbnet/vendor/shadps4/core/libraries/np/np_partner_error.h`.

/// `ORBIS_NP_PARTNER_ERROR_NOT_INITIALIZED`.
pub const ORBIS_NP_PARTNER_ERROR_NOT_INITIALIZED: i32 = 0x819D_0001u32 as i32;
/// `ORBIS_NP_PARTNER_ERROR_INVALID_ARGUMENT`.
pub const ORBIS_NP_PARTNER_ERROR_INVALID_ARGUMENT: i32 = 0x819D_0002u32 as i32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_partner_errors_keep_their_values() {
        assert_eq!(
            ORBIS_NP_PARTNER_ERROR_NOT_INITIALIZED,
            0x819D_0001u32 as i32
        );
        assert_eq!(
            ORBIS_NP_PARTNER_ERROR_INVALID_ARGUMENT,
            0x819D_0002u32 as i32
        );
    }
}
