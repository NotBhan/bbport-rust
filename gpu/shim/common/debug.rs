//! Debug/profiler helpers without Tracy, ported from `legacy/gpu/shim/common/debug.h`.

pub const EMULATOR_MARKER_COLOR: i32 = 0x264653;
pub const RENDERER_MARKER_COLOR: i32 = 0x2a9d8f;
pub const HLE_MARKER_COLOR: i32 = 0xe9c46a;
pub const GPU_MARKER_COLOR: i32 = 0xf4a261;
pub const RESERVED1_MARKER_COLOR: i32 = 0xe76f51;

/// `BREAKPOINT`: the C++ build traps here; the port aborts with the same intent.
pub fn breakpoint() -> ! {
    std::process::abort()
}

/// Tracy is not bundled into the port.
pub const fn is_profiler_connected() -> bool {
    false
}

pub const TRACY_GPU_ENABLED: bool = false;

/// `TRACE_HINT`/`TRACE_WARN`/`TRACE_ERROR`/`TRACE_CRIT` compile to no-ops.
pub fn trace_hint(_text: &str) {}
pub fn trace_warn(_text: &str) {}
pub fn trace_error(_text: &str) {}
pub fn trace_crit(_text: &str) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiler_is_absent_and_colors_match() {
        assert!(!is_profiler_connected());
        assert_eq!(GPU_MARKER_COLOR, 0xf4a261);
        trace_hint("no-op");
    }
}
