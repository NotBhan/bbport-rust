//! SSL2 error codes, ported from
//! `legacy/gpu/bbnet/vendor/shadps4/core/libraries/network/ssl2_error.h`.

/// `ORBIS_SSL_ERROR_INVALID_ARGUMENT`.
pub const ORBIS_SSL_ERROR_INVALID_ARGUMENT: i32 = 0x8095_F007u32 as i32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ssl_error_keeps_its_value() {
        fn code() -> i32 {
            ORBIS_SSL_ERROR_INVALID_ARGUMENT
        }
        assert_eq!(ORBIS_SSL_ERROR_INVALID_ARGUMENT, 0x8095_F007u32 as i32);
        assert!(code() < 0, "ORBIS errors are negative");
    }
}
