//! The process id helper used by the HTTP capture code, ported from
//! `legacy/gpu/shim/core/debugger.h`.

/// `Core::Debugger::GetCurrentPid`.
pub fn get_current_pid() -> i32 {
    std::process::id() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pid_is_positive() {
        assert!(get_current_pid() > 0);
    }
}
