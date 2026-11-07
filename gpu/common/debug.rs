//! Debug and profiling markers, ported from `legacy/gpu/shadps4/common/debug.h`.
//!
//! Tracy is not part of the port; the markers keep the same names and colors so instrumentation
//! call sites translate directly, and the trace helpers compile to nothing.

/// `MarkersPalette`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum MarkersPalette {
    EmulatorMarkerColor = 0x26_4653,
    RendererMarkerColor = 0x2a_9d8f,
    HleMarkerColor = 0xe9_c46a,
    GpuMarkerColor = 0xf4_a261,
    Reserved1 = 0xe7_6f51,
}

/// `IsProfilerConnected`; always false without Tracy.
pub fn is_profiler_connected() -> bool {
    false
}

/// `BREAKPOINT`.
pub fn breakpoint() {
    // SAFETY: raising SIGTRAP has no preconditions.
    unsafe {
        libc::raise(libc::SIGTRAP);
    }
}

/// `TRACY_GPU_ENABLED`.
pub const TRACY_GPU_ENABLED: bool = false;

/// `TRACE_HINT`.
pub fn trace_hint(_text: &str) {}

/// `TRACE_WARN`.
pub fn trace_warn(_message: &str) {}

/// `TRACE_ERROR`.
pub fn trace_error(_message: &str) {}

/// `TRACE_CRIT`.
pub fn trace_crit(_message: &str) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_marker_colors_match_the_header() {
        assert_eq!(MarkersPalette::EmulatorMarkerColor as i32, 0x264653);
        assert_eq!(MarkersPalette::Reserved1 as i32, 0xe76f51);
    }

    #[test]
    fn the_profiler_is_never_connected_without_tracy() {
        assert!(!is_profiler_connected());
    }

    #[test]
    fn the_trace_helpers_accept_messages() {
        trace_hint("frame");
        trace_warn("warn");
        trace_error("error");
        trace_crit("critical");
    }
}
