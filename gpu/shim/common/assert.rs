//! GPU-side assertion handling, ported from `legacy/gpu/shim/common/assert.cpp`.
//! Failures stop the port with exit code 23.

/// `assert_fail_impl`: flush stdout, print the STOP line, exit 23.
pub fn assert_fail_impl() -> ! {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    eprintln!("STOP: GPU library assertion failed (see GPU log above)");
    std::process::exit(23)
}

/// `unreachable_impl`: the C++ build throws after the stop; here it never returns.
pub fn unreachable_impl() -> ! {
    assert_fail_impl()
}

/// `assert_fail_debug_msg`: log the message at critical level, then stop.
pub fn assert_fail_debug_msg(message: &str) -> ! {
    crate::gpu::shim::common::logging::log::write(
        "Debug",
        crate::gpu::shim::common::logging::log::Level::Critical,
        file!(),
        line!(),
        "assert_fail_debug_msg",
        message,
    );
    assert_fail_impl()
}
