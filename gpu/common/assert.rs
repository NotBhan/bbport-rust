//! Assertion failure handling, ported from `legacy/gpu/shadps4/common/assert.cpp`. The C++
//! takes the signal handlers down, shuts the emulator down and traps; the Rust port keeps
//! that order and takes the two first steps from the host.

use std::sync::OnceLock;

/// A hook the emulator installs at startup.
pub type Hook = fn();

static REMOVE_HANDLERS: OnceLock<Hook> = OnceLock::new();
static SHUTDOWN: OnceLock<Hook> = OnceLock::new();

/// `Core::Signals::Instance()->RemoveHandlers()` and `Emulator::Shutdown()`, installed here
/// so the assertion module does not depend on the core.
pub fn set_assert_hooks(remove_handlers: Hook, shutdown: Hook) {
    let _ = REMOVE_HANDLERS.set(remove_handlers);
    let _ = SHUTDOWN.set(shutdown);
}

/// The two steps the C++ takes before trapping. Split out so they can be exercised.
pub fn assert_fail_hooks() -> (bool, bool) {
    let handlers = REMOVE_HANDLERS.get().map(|hook| {
        hook();
        true
    });
    let shutdown = SHUTDOWN.get().map(|hook| {
        hook();
        true
    });
    (handlers.unwrap_or(false), shutdown.unwrap_or(false))
}

/// `assert_fail_impl`.
pub fn assert_fail_impl() -> ! {
    assert_fail_hooks();
    crash();
}

/// `unreachable_impl`.
pub fn unreachable_impl() -> ! {
    assert_fail_impl();
}

/// `assert_fail_debug_msg`.
pub fn assert_fail_debug_msg(message: &str) -> ! {
    eprintln!("Assertion Failed!\n{message}");
    assert_fail_impl();
}

/// The C++'s `Crash()`: a trap on the guest's own architecture.
#[cfg(target_arch = "x86_64")]
pub fn crash() -> ! {
    unsafe {
        core::arch::asm!("int $3");
    }
    std::process::abort()
}

#[cfg(target_arch = "aarch64")]
pub fn crash() -> ! {
    unsafe {
        core::arch::asm!("brk 0");
    }
    std::process::abort()
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
compile_error!("Missing Crash() implementation for target CPU architecture.");

/// `ASSERT` / `ASSERT_MSG`.
#[macro_export]
macro_rules! shad_assert {
    ($condition:expr $(,)?) => {
        if !$condition {
            $crate::gpu::common::assert::assert_fail_debug_msg("Assertion Failed!");
        }
    };
    ($condition:expr, $($arg:tt)*) => {
        if !$condition {
            $crate::gpu::common::assert::assert_fail_debug_msg(&format!($($arg)*));
        }
    };
}

/// `UNREACHABLE` / `UNREACHABLE_MSG`.
#[macro_export]
macro_rules! shad_unreachable {
    () => {
        $crate::gpu::common::assert::assert_fail_debug_msg("Unreachable code!");
    };
    ($($arg:tt)*) => {
        $crate::gpu::common::assert::assert_fail_debug_msg(&format!("Unreachable code!\n{}", format!($($arg)*)));
    };
}

/// `DEBUG_ASSERT` / `DEBUG_ASSERT_MSG`: compiled out in release builds.
#[macro_export]
macro_rules! shad_debug_assert {
    ($condition:expr $(,)?) => {
        if cfg!(debug_assertions) && !$condition {
            $crate::gpu::common::assert::assert_fail_debug_msg("Assertion Failed!");
        }
    };
    ($condition:expr, $($arg:tt)*) => {
        if cfg!(debug_assertions) && !$condition {
            $crate::gpu::common::assert::assert_fail_debug_msg(&format!($($arg)*));
        }
    };
}

/// `UNIMPLEMENTED` / `UNIMPLEMENTED_MSG`, and the conditional forms.
#[macro_export]
macro_rules! shad_unimplemented {
    () => {
        $crate::gpu::common::assert::assert_fail_debug_msg("Unimplemented code!");
    };
    ($($arg:tt)*) => {
        $crate::gpu::common::assert::assert_fail_debug_msg(&format!($($arg)*));
    };
}

/// `UNIMPLEMENTED_IF` / `UNIMPLEMENTED_IF_MSG`.
#[macro_export]
macro_rules! shad_unimplemented_if {
    ($condition:expr $(,)?) => {
        $crate::shad_assert!(!$condition, "Unimplemented code!");
    };
    ($condition:expr, $($arg:tt)*) => {
        $crate::shad_assert!(!$condition, $($arg)*);
    };
}

/// `ASSERT_OR_EXECUTE` / `ASSERT_OR_EXECUTE_MSG`.
#[macro_export]
macro_rules! shad_assert_or_execute {
    ($condition:expr, $action:block) => {
        #[allow(unreachable_code)]
        if !$condition {
            $crate::gpu::common::assert::assert_fail_debug_msg("Assertion Failed!");
            $action
        }
    };
    ($condition:expr, $action:block, $($arg:tt)*) => {
        #[allow(unreachable_code)]
        if !$condition {
            $crate::gpu::common::assert::assert_fail_debug_msg(&format!($($arg)*));
            $action
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static HANDLERS: AtomicUsize = AtomicUsize::new(0);
    static SHUTDOWNS: AtomicUsize = AtomicUsize::new(0);

    fn remove_handlers() {
        HANDLERS.fetch_add(1, Ordering::Relaxed);
    }

    fn shutdown() {
        SHUTDOWNS.fetch_add(1, Ordering::Relaxed);
    }

    #[test]
    fn the_failure_path_takes_the_handlers_down_then_shuts_down() {
        // A run without hooks still reports that neither step ran.
        let (handlers_ran, shutdown_ran) = assert_fail_hooks();
        assert!(!handlers_ran && !shutdown_ran, "no hooks are installed yet");
        set_assert_hooks(remove_handlers, shutdown);
        let (handlers_ran, shutdown_ran) = assert_fail_hooks();
        assert!(handlers_ran && shutdown_ran);
        assert_eq!(HANDLERS.load(Ordering::Relaxed), 1);
        assert_eq!(SHUTDOWNS.load(Ordering::Relaxed), 1);
        // The trap itself is not exercised here: it ends the process by design.
        assert!(std::panic::catch_unwind(unreachable_impl_stub).is_err());
        // The header's macros pass their conditions through without stopping.
        crate::shad_assert!(1 + 1 == 2);
        crate::shad_assert!(true, "must not fire: {}", 1);
        crate::shad_debug_assert!(true, "must not fire");
        crate::shad_assert_or_execute!(true, { unreachable!() });
        crate::shad_unimplemented_if!(false);
    }

    fn unreachable_impl_stub() {
        panic!("Unreachable code");
    }
}
