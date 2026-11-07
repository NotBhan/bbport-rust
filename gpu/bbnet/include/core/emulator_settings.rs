//! Private networking settings, ported from `bbnet/include/core/emulator_settings.h`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

/// `BbNetSettingsImpl`.
#[derive(Debug, Default)]
pub struct BbNetSettingsImpl {
    shadnet_session_disabled: AtomicBool,
}

impl BbNetSettingsImpl {
    /// `GetInstance`.
    pub fn instance() -> &'static BbNetSettingsImpl {
        static INSTANCE: OnceLock<BbNetSettingsImpl> = OnceLock::new();
        INSTANCE.get_or_init(BbNetSettingsImpl::default)
    }

    /// `Flag`.
    pub fn flag(name: &str, fallback: bool) -> bool {
        match std::env::var(name) {
            Ok(value) => value.starts_with('1'),
            Err(_) => fallback,
        }
    }

    /// `Text`.
    pub fn text(name: &str) -> String {
        std::env::var(name).unwrap_or_default()
    }

    /// `IsConnectedToNetwork`.
    pub fn is_connected_to_network(&self) -> bool {
        Self::flag("BB_ONLINE", false)
    }

    /// `IsShadNetEnabled`.
    pub fn is_shadnet_enabled(&self) -> bool {
        self.is_connected_to_network() && !self.shadnet_session_disabled.load(Ordering::Acquire)
    }

    /// `SetShadNetSessionDisabled`.
    pub fn set_shadnet_session_disabled(&self, disabled: bool) {
        self.shadnet_session_disabled
            .store(disabled, Ordering::Release);
    }

    /// `GetShadNetServer`.
    pub fn shadnet_server(&self) -> String {
        Self::text("BB_SHADNET_SERVER")
    }

    /// `GetShadNetWebApiServer`.
    pub fn shadnet_webapi_server(&self) -> String {
        Self::text("BB_SHADNET_WEBAPI")
    }

    /// `IsUPnPEnabled`.
    pub fn is_upnp_enabled(&self) -> bool {
        Self::flag("BB_UPNP", false)
    }

    /// `IsForcedHttpsDisabled`.
    pub fn is_forced_https_disabled(&self) -> bool {
        Self::flag("BB_DISABLE_HTTPS", false)
    }

    /// `GetP2PPort`.
    pub fn p2p_port(&self) -> i32 {
        match std::env::var("BB_P2P_PORT") {
            Ok(value) if !value.is_empty() => value.parse().unwrap_or(3658),
            _ => 3658,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_and_text_fall_back_when_unset() {
        assert!(BbNetSettingsImpl::flag(
            "BBPORT_TEST_DEFINITELY_UNSET_FLAG",
            true
        ));
        assert!(!BbNetSettingsImpl::flag(
            "BBPORT_TEST_DEFINITELY_UNSET_FLAG",
            false
        ));
        assert_eq!(
            BbNetSettingsImpl::text("BBPORT_TEST_DEFINITELY_UNSET_TEXT"),
            ""
        );
    }

    #[test]
    fn the_session_disable_switch_round_trips() {
        let settings = BbNetSettingsImpl::instance();
        settings.set_shadnet_session_disabled(true);
        assert!(!settings.is_shadnet_enabled());
        settings.set_shadnet_session_disabled(false);
        let _ = settings.is_shadnet_enabled();
    }
}
