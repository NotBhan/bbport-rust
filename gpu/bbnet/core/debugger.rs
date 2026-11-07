//! The process id helper the HTTP capture code uses, ported from
//! `legacy/gpu/bbnet/include/core/debugger.h`.

/// `Core::Debugger::GetCurrentPid`.
pub fn get_current_pid() -> i32 {
    std::process::id() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pid_is_the_running_process() {
        assert_eq!(get_current_pid(), std::process::id() as i32);
        assert!(get_current_pid() > 0);
    }
}
