//! NetCtl callbacks, ported from
//! `bbnet/vendor/shadps4/core/libraries/network/net_ctl_obj.h` and `net_ctl_obj.cpp`.

use std::sync::Mutex;

use crate::gpu::bbnet::core::libraries::network::net_ctl_codes::*;

/// `OrbisNetCtlCallback`.
pub type OrbisNetCtlCallback = unsafe extern "C" fn(event_type: i32, arg: *mut core::ffi::c_void);
/// `OrbisNetCtlCallbackForNpToolkit`.
pub type OrbisNetCtlCallbackForNpToolkit =
    unsafe extern "C" fn(event_type: i32, arg: *mut core::ffi::c_void);

/// `NetCtlCallback`.
#[derive(Debug, Clone, Copy, Default)]
pub struct NetCtlCallback {
    pub func: Option<OrbisNetCtlCallback>,
    pub arg: *mut core::ffi::c_void,
}

/// `NetCtlCallbackForNpToolkit`.
#[derive(Debug, Clone, Copy, Default)]
pub struct NetCtlCallbackForNpToolkit {
    pub func: Option<OrbisNetCtlCallbackForNpToolkit>,
    pub arg: *mut core::ffi::c_void,
}

/// `NetCtlInternal`.
#[derive(Debug, Default)]
pub struct NetCtlInternal {
    callbacks: Mutex<[NetCtlCallback; 8]>,
    nptool_callbacks: Mutex<[NetCtlCallbackForNpToolkit; 8]>,
}

impl NetCtlInternal {
    pub fn new() -> Self {
        Self::default()
    }

    /// `RegisterCallback`; returns the slot index or the callback-max error.
    pub fn register_callback(&self, func: OrbisNetCtlCallback, arg: *mut core::ffi::c_void) -> i32 {
        let mut callbacks = self
            .callbacks
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some(index) = callbacks.iter().position(|entry| entry.func.is_none()) else {
            return ORBIS_NET_CTL_ERROR_CALLBACK_MAX;
        };
        callbacks[index] = NetCtlCallback {
            func: Some(func),
            arg,
        };
        index as i32
    }

    /// `RegisterNpToolkitCallback`.
    pub fn register_nptoolkit_callback(
        &self,
        func: OrbisNetCtlCallbackForNpToolkit,
        arg: *mut core::ffi::c_void,
    ) -> i32 {
        let mut callbacks = self
            .nptool_callbacks
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some(index) = callbacks.iter().position(|entry| entry.func.is_none()) else {
            return ORBIS_NET_CTL_ERROR_CALLBACK_MAX;
        };
        callbacks[index] = NetCtlCallbackForNpToolkit {
            func: Some(func),
            arg,
        };
        index as i32
    }

    /// `CheckCallback`.
    pub fn check_callback(&self, connected: bool) {
        let event = if connected {
            ORBIS_NET_CTL_EVENT_TYPE_IPOBTAINED
        } else {
            ORBIS_NET_CTL_EVENT_TYPE_DISCONNECTED
        };
        let callbacks = self
            .callbacks
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for callback in callbacks.iter() {
            if let Some(func) = callback.func {
                // SAFETY: the callback and its argument come from the caller.
                unsafe {
                    func(event, callback.arg);
                }
            }
        }
    }

    /// `CheckNpToolkitCallback`.
    pub fn check_nptoolkit_callback(&self, connected: bool) {
        let event = if connected {
            ORBIS_NET_CTL_EVENT_TYPE_IPOBTAINED
        } else {
            ORBIS_NET_CTL_EVENT_TYPE_DISCONNECTED
        };
        let callbacks = self
            .nptool_callbacks
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for callback in callbacks.iter() {
            if let Some(func) = callback.func {
                // SAFETY: the callback and its argument come from the caller.
                unsafe {
                    func(event, callback.arg);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI32, Ordering};

    static LAST_EVENT: AtomicI32 = AtomicI32::new(-1);

    unsafe extern "C" fn callback(event: i32, _arg: *mut core::ffi::c_void) {
        LAST_EVENT.store(event, Ordering::Release);
    }

    #[test]
    fn callbacks_fill_eight_slots_then_report_the_limit() {
        let internal = NetCtlInternal::new();
        for index in 0..8 {
            assert_eq!(
                internal.register_callback(callback, std::ptr::null_mut()),
                index
            );
        }
        assert_eq!(
            internal.register_callback(callback, std::ptr::null_mut()),
            ORBIS_NET_CTL_ERROR_CALLBACK_MAX
        );
    }

    #[test]
    fn checking_delivers_the_connection_event() {
        let internal = NetCtlInternal::new();
        assert_eq!(
            internal.register_callback(callback, std::ptr::null_mut()),
            0
        );
        LAST_EVENT.store(-1, Ordering::Release);
        internal.check_callback(true);
        assert_eq!(
            LAST_EVENT.load(Ordering::Acquire),
            ORBIS_NET_CTL_EVENT_TYPE_IPOBTAINED
        );
        internal.check_callback(false);
        assert_eq!(
            LAST_EVENT.load(Ordering::Acquire),
            ORBIS_NET_CTL_EVENT_TYPE_DISCONNECTED
        );

        assert_eq!(
            internal.register_nptoolkit_callback(callback, std::ptr::null_mut()),
            0
        );
        internal.check_nptoolkit_callback(true);
        assert_eq!(
            LAST_EVENT.load(Ordering::Acquire),
            ORBIS_NET_CTL_EVENT_TYPE_IPOBTAINED
        );
    }
}
