//! Native error text, ported from `legacy/gpu/shadps4/common/error.h`.

/// `NativeErrorToString`: the host's text for an error number.
pub fn native_error_to_string(error: i32) -> String {
    let mut buffer = [0i8; 256];
    let result = unsafe { libc::strerror_r(error, buffer.as_mut_ptr(), buffer.len()) };
    if result != 0 {
        // The C++'s XSI branch reports the same thing when `strerror_r` refuses.
        return "(strerror_r failed to format error)".to_string();
    }
    let bytes: Vec<u8> = buffer
        .iter()
        .take_while(|byte| **byte != 0)
        .map(|byte| *byte as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// `GetLastErrorMsg`: the message for the last host error.
pub fn get_last_error_msg() -> String {
    match std::io::Error::last_os_error().raw_os_error() {
        Some(error) => native_error_to_string(error),
        None => "Success".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_comes_from_the_host_error_table() {
        assert!(native_error_to_string(libc::EINVAL)
            .to_lowercase()
            .contains("invalid"));
        assert!(native_error_to_string(libc::EACCES)
            .to_lowercase()
            .contains("permission"));
        let unknown = native_error_to_string(4_000_000);
        assert!(!unknown.is_empty(), "an unknown number still yields text");
        let last = get_last_error_msg();
        assert!(!last.is_empty());
    }

    #[test]
    fn a_failed_syscall_reports_its_own_error() {
        let missing = std::fs::File::open("/definitely/not/here-9f8e7d");
        assert!(missing.is_err());
        let message = get_last_error_msg();
        assert!(
            message.to_lowercase().contains("no such file")
                || message.to_lowercase().contains("not"),
            "the message describes the failure: {message}"
        );
    }
}
