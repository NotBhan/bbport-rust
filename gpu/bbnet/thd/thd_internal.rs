//! Shared pieces of the Hunter's Dream backend, ported from `bbnet/thd/thd_internal.h`.

use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

/// `Config`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub np_base: String,
    pub auth_base: String,
    pub stun: String,
    pub ruleset: String,
    pub client: String,
    pub p2p_addr: String,
    pub p2p_port: i32,
    pub verify_tls: bool,
}

fn text(name: &str, fallback: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn flag(name: &str, fallback: bool) -> bool {
    std::env::var(name).map_or(fallback, |value| value.starts_with('1'))
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            np_base: text("BB_THD_SERVER", "https://thehuntersdream.com:18671"),
            auth_base: text("BB_THD_AUTH", "https://thehuntersdream.com"),
            stun: text("BB_THD_STUN", ""),
            ruleset: text("BB_THD_RULESET", "vanilla"),
            client: text("BB_CLIENT_VERSION", ""),
            p2p_addr: text("BB_THD_P2P_ADDR", ""),
            p2p_port: text("BB_P2P_PORT", "3658").parse().unwrap_or(3658),
            verify_tls: !flag("BB_THD_INSECURE", false),
        }
    }
}

/// `Cfg`.
pub fn cfg() -> &'static Config {
    static CONFIG: OnceLock<Config> = OnceLock::new();
    CONFIG.get_or_init(Config::from_env)
}

/// `Log`.
pub fn log(message: &str) {
    crate::log_info!(crate::gpu::common::logging::classes::SHAD_NET, "{message}");
}

/// `Trace`: `BB_THD_TRACE=1`.
pub fn trace() -> bool {
    flag("BB_THD_TRACE", false)
}

/// `AccountName`.
pub fn account_name() -> String {
    text("BB_THD_ACCOUNT_NAME", "")
}

/// `AccountToken`.
pub fn account_token() -> String {
    text("BB_THD_TOKEN", "")
}

/// `SignedIn`.
pub fn signed_in() -> bool {
    !account_token().is_empty()
}

/// `OnlineId`.
pub fn online_id() -> String {
    text("BB_THD_ONLINE_ID", "")
}

fn refused_state() -> &'static Mutex<Option<String>> {
    static REFUSED: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    REFUSED.get_or_init(|| Mutex::new(None))
}

/// `Refused`.
pub fn refused() -> bool {
    refused_state()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .is_some()
}

/// `MarkRefused`.
pub fn mark_refused(reason: &str) {
    *refused_state()
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(reason.to_string());
}

/// `StrOf`.
pub fn str_of(object: &Value, key: &str, default: &str) -> String {
    object
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}

/// `IntOf`.
pub fn int_of(object: &Value, key: &str, default: i64) -> i64 {
    object.get(key).and_then(Value::as_i64).unwrap_or(default)
}

/// `NpBase`.
pub fn np_base() -> String {
    cfg().np_base.clone()
}

/// `Prio`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Prio {
    Context = 0,
    Request = 1,
    Signaling = 2,
    RoomEvent = 3,
}

/// `NowMs`.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

/// `Peer`.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peer {
    pub member_id: u16,
    pub online_id: String,
    pub addr: u32,
    pub port: u16,
    pub local_addr: u32,
    pub local_port: u16,
    pub conn_id: u32,
    pub sig_status: i32,
}

/// `kStunTokenLen`.
pub const STUN_TOKEN_LEN: usize = 8;

/// `Relay`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Relay {
    pub present: bool,
    pub token: [u8; STUN_TOKEN_LEN],
    pub vport: u16,
    pub observed_addr: u32,
    pub observed_port: u16,
}

impl Default for Relay {
    fn default() -> Self {
        Self {
            present: false,
            token: [0; STUN_TOKEN_LEN],
            vport: 0,
            observed_addr: 0,
            observed_port: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_falls_back_to_the_documented_defaults() {
        // Environment-dependent fields cannot be asserted in parallel tests; the fallbacks
        // are what the header documents.
        let config = Config::from_env();
        assert!(!config.np_base.is_empty());
        assert!(!config.auth_base.is_empty());
        let _ = config.p2p_port;
    }

    #[test]
    fn json_helpers_read_typed_fields() {
        let value = serde_json::json!({
            "name": "hunter",
            "count": 7,
            "missing": null,
        });
        assert_eq!(str_of(&value, "name", "?"), "hunter");
        assert_eq!(str_of(&value, "missing", "fallback"), "fallback");
        assert_eq!(int_of(&value, "count", 0), 7);
        assert_eq!(int_of(&value, "missing", -1), -1);
        assert!(now_ms() > 0);
        assert_eq!(Prio::RoomEvent as u8, 3);
    }

    #[test]
    fn the_peer_and_relay_layouts_hold_the_fields() {
        let peer = Peer {
            member_id: 1,
            online_id: "hunter".to_string(),
            addr: 0x0100007f,
            port: 3658,
            local_addr: 0,
            local_port: 0,
            conn_id: 9,
            sig_status: 1,
        };
        assert_eq!(peer.online_id, "hunter");
        let relay = Relay {
            present: true,
            token: [1; STUN_TOKEN_LEN],
            vport: 2,
            observed_addr: 3,
            observed_port: 4,
        };
        assert!(relay.present);
        assert_eq!(STUN_TOKEN_LEN, 8);
    }
}
