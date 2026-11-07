//! NpPartner EA-access stubs, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_partner.cpp`.

use std::sync::Mutex;

use crate::gpu::core::libraries::error_codes::ORBIS_OK;
use crate::gpu::core::libraries::np::np_partner_error::{
    ORBIS_NP_PARTNER_ERROR_INVALID_ARGUMENT, ORBIS_NP_PARTNER_ERROR_NOT_INITIALIZED,
};

#[derive(Debug, Default)]
pub struct NpPartner {
    initialized: Mutex<bool>,
}

impl NpPartner {
    pub fn new() -> Self {
        Self::default()
    }

    /// `sceNpEAAccessInitialize`.
    pub fn access_initialize(&self) -> i32 {
        *self
            .initialized
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = true;
        ORBIS_OK
    }

    /// `sceNpEAAccessTerminate`.
    pub fn access_terminate(&self) -> i32 {
        let mut initialized = self
            .initialized
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if !*initialized {
            return ORBIS_NP_PARTNER_ERROR_NOT_INITIALIZED;
        }
        *initialized = false;
        ORBIS_OK
    }

    /// `sceNpHasEAAccessSubscriptionAbortRequest`.
    pub fn has_subscription_abort_request(&self) -> i32 {
        if !*self
            .initialized
            .lock()
            .unwrap_or_else(|error| error.into_inner())
        {
            return ORBIS_NP_PARTNER_ERROR_NOT_INITIALIZED;
        }
        ORBIS_OK
    }

    /// `sceNpHasEAAccessSubscription`; the subscription check is stubbed to false.
    pub fn has_subscription(&self, result: Option<&mut bool>) -> i32 {
        if !*self
            .initialized
            .lock()
            .unwrap_or_else(|error| error.into_inner())
        {
            return ORBIS_NP_PARTNER_ERROR_NOT_INITIALIZED;
        }
        let Some(result) = result else {
            return ORBIS_NP_PARTNER_ERROR_INVALID_ARGUMENT;
        };
        *result = false;
        ORBIS_OK
    }
}

/// The exported entry points, in `RegisterLib` order.
pub const FUNCTIONS: [(&str, &str); 4] = [
    ("pMxXhNozUX8", "sceNpEAAccessTerminate"),
    ("pQfYTZHznMc", "sceNpHasEAAccessSubscriptionAbortRequest"),
    ("7CxI50-xlCk", "sceNpEAAccessInitialize"),
    ("+OnbUs1CV0M", "sceNpHasEAAccessSubscription"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partner_lifecycle_gates_the_subscription_query() {
        let partner = NpPartner::new();
        let mut result = true;
        assert_eq!(
            partner.has_subscription(Some(&mut result)),
            ORBIS_NP_PARTNER_ERROR_NOT_INITIALIZED
        );
        assert_eq!(partner.access_initialize(), ORBIS_OK);
        assert_eq!(partner.has_subscription(Some(&mut result)), ORBIS_OK);
        assert!(!result);
        assert_eq!(
            partner.has_subscription(None),
            ORBIS_NP_PARTNER_ERROR_INVALID_ARGUMENT
        );
        assert_eq!(partner.access_terminate(), ORBIS_OK);
        assert_eq!(FUNCTIONS.len(), 4);
    }
}
