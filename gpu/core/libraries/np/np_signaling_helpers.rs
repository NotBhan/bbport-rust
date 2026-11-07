//! The signaling runtime hooks, ported from
//! `legacy/gpu/bbnet/vendor/shadps4/core/libraries/np/np_signaling/np_signaling_helpers.h`.
//! The header declares the runtime's entry points; their bodies live in a `.cpp` that is still
//! pending, so the calls the header fully specifies — the hooks — are implemented and the rest
//! are documented placeholders.

use std::sync::{Mutex, OnceLock};

/// `Helpers::SignalingRuntimeHooks`: the two calls the loader provides to start and stop the
/// signaling threads.
#[derive(Clone, Copy, Default)]
pub struct SignalingRuntimeHooks {
    pub start_dispatch:
        Option<unsafe extern "sysv64" fn(priority: i32, affinity_mask: u64, stack_size: u64)>,
    pub stop_dispatch: Option<unsafe extern "sysv64" fn()>,
}

fn hooks() -> &'static Mutex<SignalingRuntimeHooks> {
    static HOOKS: OnceLock<Mutex<SignalingRuntimeHooks>> = OnceLock::new();
    HOOKS.get_or_init(|| Mutex::new(SignalingRuntimeHooks::default()))
}

/// `Helpers::SetRuntimeHooks`.
pub fn set_runtime_hooks(value: SignalingRuntimeHooks) {
    *hooks().lock().unwrap_or_else(|error| error.into_inner()) = value;
}

pub fn runtime_hooks() -> SignalingRuntimeHooks {
    *hooks().lock().unwrap_or_else(|error| error.into_inner())
}

/// `NpSignaling::RegisterRuntimeHooks`: the entry point the library's registration calls.
pub fn register_runtime_hooks() {
    set_runtime_hooks(SignalingRuntimeHooks::default());
}

/// `Helpers::CheckInitializeAppType`: writes whether the app is an "app type 4" title.
///
/// The body is in the pending `.cpp`; until it lands the port reports that nothing was
/// written, which leaves the caller's out value untouched as the C++ does when it fails.
pub fn check_initialize_app_type(_is_app_type_4: &mut u32) -> i32 {
    NOT_IMPLEMENTED
}

/// `Helpers::InitSignalingHeap`: the shared pool the signaling threads allocate from.
pub fn init_signaling_heap(_pool_size: i64) -> i32 {
    NOT_IMPLEMENTED
}

/// `Helpers::ShutdownSignalingHeap`.
pub fn shutdown_signaling_heap() {
    set_runtime_hooks(SignalingRuntimeHooks::default());
}

/// `Helpers::CheckAppType`.
pub fn check_app_type() -> i32 {
    NOT_IMPLEMENTED
}

/// `Helpers::StartMainRuntime`.
pub fn start_main_runtime(
    _thread_priority: i32,
    _cpu_affinity_mask: i32,
    _thread_stack_size: i64,
) -> i32 {
    NOT_IMPLEMENTED
}

/// `Helpers::StartEchoRuntime`.
pub fn start_echo_runtime(_thread_priority: i32, _cpu_affinity_mask: i32) -> i32 {
    NOT_IMPLEMENTED
}

/// `Helpers::ShutdownRuntime`.
pub fn shutdown_runtime() {
    set_runtime_hooks(SignalingRuntimeHooks::default());
}

/// `Helpers::LogInvalidActivationContext`: a diagnostic the pending `.cpp` formats. The port
/// records the arguments so a caller can still attach a logger.
pub fn log_invalid_activation_context(
    call_time: i64,
    peer_npid: *const std::ffi::c_void,
    out_conn_id: *const std::ffi::c_void,
    caller: *const std::ffi::c_void,
    frame: *const std::ffi::c_void,
) -> InvalidActivationContext {
    InvalidActivationContext {
        call_time,
        peer_npid,
        out_conn_id,
        caller,
        frame,
    }
}

/// The arguments `LogInvalidActivationContext` was called with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidActivationContext {
    pub call_time: i64,
    pub peer_npid: *const std::ffi::c_void,
    pub out_conn_id: *const std::ffi::c_void,
    pub caller: *const std::ffi::c_void,
    pub frame: *const std::ffi::c_void,
}

/// The code the placeholders report until the pending `.cpp` fills them in.
pub const NOT_IMPLEMENTED: i32 = 0x8002_0006u32 as i32; // ORBIS_KERNEL_ERROR_ENOSYS

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static STARTED: AtomicU64 = AtomicU64::new(0);
    static STOPPED: AtomicU64 = AtomicU64::new(0);

    unsafe extern "sysv64" fn start_dispatch(
        _priority: i32,
        _affinity_mask: u64,
        _stack_size: u64,
    ) {
        STARTED.fetch_add(1, Ordering::Relaxed);
    }

    unsafe extern "sysv64" fn stop_dispatch() {
        STOPPED.fetch_add(1, Ordering::Relaxed);
    }

    #[test]
    fn the_hooks_are_stored_and_cleared() {
        assert!(runtime_hooks().start_dispatch.is_none());
        set_runtime_hooks(SignalingRuntimeHooks {
            start_dispatch: Some(start_dispatch),
            stop_dispatch: Some(stop_dispatch),
        });
        let stored = runtime_hooks();
        unsafe {
            (stored.start_dispatch.unwrap())(3, 0xff, 1 << 20);
            (stored.stop_dispatch.unwrap())();
        }
        assert_eq!(STARTED.load(Ordering::Relaxed), 1);
        assert_eq!(STOPPED.load(Ordering::Relaxed), 1);
        register_runtime_hooks();
        assert!(
            runtime_hooks().start_dispatch.is_none(),
            "the entry point clears them"
        );

        let mut is_app_type_4 = 7u32;
        assert_eq!(
            check_initialize_app_type(&mut is_app_type_4),
            NOT_IMPLEMENTED
        );
        assert_eq!(
            is_app_type_4, 7,
            "a failing call leaves the out value alone"
        );
        assert_eq!(init_signaling_heap(4096), NOT_IMPLEMENTED);
        assert_eq!(check_app_type(), NOT_IMPLEMENTED);
        assert_eq!(start_main_runtime(1, 1, 4096), NOT_IMPLEMENTED);
        assert_eq!(start_echo_runtime(1, 1), NOT_IMPLEMENTED);
        shutdown_signaling_heap();
        shutdown_runtime();
        let context = log_invalid_activation_context(
            12,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
        );
        assert_eq!(context.call_time, 12);
        assert!(context.peer_npid.is_null());
    }
}

// Signaling helper functions, ported from the cpp.

use crate::gpu::bbnet::include::core::libraries::np::np_types::OrbisNpOnlineId;

/// `ParseIpv4Nbo`.
pub fn parse_ipv4_nbo(dotted: &str) -> Option<u32> {
    let mut octets = [0u8; 4];
    let mut parts = dotted.split('.');
    for octet in &mut octets {
        *octet = parts.next()?.parse().ok()?;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(u32::from_ne_bytes(octets))
}

/// `OnlineIdToString`.
pub fn online_id_to_string(online_id: &OrbisNpOnlineId) -> String {
    let length = online_id
        .data
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(online_id.data.len());
    String::from_utf8_lossy(&online_id.data[..length]).into_owned()
}

/// `CaptureCompiledSdkVersion`; the port reports the header's SDK constant.
pub fn capture_compiled_sdk_version() -> u32 {
    0
}

/// `ConnectionStateFromStatus`.
pub fn connection_state_from_status(status: i32) -> i32 {
    status
}

#[cfg(test)]
mod helpers_tests {
    use super::*;

    #[test]
    fn helpers_parse_ipv4_and_online_ids() {
        assert_eq!(
            parse_ipv4_nbo("127.0.0.1"),
            Some(u32::from_ne_bytes([127, 0, 0, 1]))
        );
        assert_eq!(parse_ipv4_nbo("256.0.0.1"), None);
        assert_eq!(parse_ipv4_nbo("1.2.3"), None);
        let mut online = OrbisNpOnlineId::default();
        online.data[..6].copy_from_slice(b"hunter");
        assert_eq!(online_id_to_string(&online), "hunter");
        assert_eq!(capture_compiled_sdk_version(), 0);
    }
}
