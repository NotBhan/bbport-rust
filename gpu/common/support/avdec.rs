//! FFmpeg error strings, ported from `legacy/gpu/shadps4/common/support/avdec.h`. The C++
//! replaces `av_err2str` (a macro that misbehaves in C++) with `av_err2string`; the Rust port
//! exposes the same call directly.

/// `AV_ERROR_MAX_STRING_SIZE`: the buffer FFmpeg formats into.
pub const AV_ERROR_MAX_STRING_SIZE: usize = 64;

/// `av_strerror`'s rule: a negative `AVERROR` is the negated system error.
pub fn av_err_string(errnum: i32) -> String {
    let system_error = if errnum < 0 {
        errnum.unsigned_abs()
    } else {
        errnum as u32
    };
    if system_error == 0 {
        return "Success".to_string();
    }
    let mut buffer = [0i8; AV_ERROR_MAX_STRING_SIZE];
    let result =
        unsafe { libc::strerror_r(system_error as i32, buffer.as_mut_ptr(), buffer.len()) };
    if result != 0 {
        return format!("Unknown error {errnum}");
    }
    let bytes: Vec<u8> = buffer
        .iter()
        .take_while(|byte| **byte != 0)
        .map(|byte| *byte as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// The C++'s `av_err2str(err)`: the message for an `AVERROR`.
pub fn av_err2str(errnum: i32) -> String {
    av_err_string(errnum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_message_follows_the_negated_system_error() {
        assert_eq!(av_err_string(0), "Success");
        let invalid = av_err_string(-libc::EINVAL);
        assert!(
            invalid.to_lowercase().contains("invalid"),
            "EINVAL reads as an invalid argument: {invalid}"
        );
        assert_eq!(av_err2str(-libc::EINVAL), invalid);
        assert_eq!(
            av_err_string(libc::EINVAL),
            invalid,
            "a non-negative error is taken as-is"
        );
        let unknown = av_err_string(-123_456);
        assert!(!unknown.is_empty(), "an unknown error still yields text");
    }
}
