//! Networking, online pool and environment configuration, ported from `bbport_net.py`.

use crate::paths::Paths;
use crate::update::{Fetcher, VERSION};
use crate::Settings;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use thiserror::Error;

pub const COMMUNITY_SERVER: &str = "https://thehuntersdream.com";
pub const PLAY_MODES: [&str; 3] = ["offline", "huntersdream", "coop"];
pub const DEFAULT_ONLINE_SERVER: &str = "srv.shadps4.net:31313";
pub const DEFAULT_ONLINE_POOL: &str = "huntersdream";

#[derive(Debug, Error)]
pub enum NetError {
    #[error("{0}")]
    Service(String),
    #[error("network request failed: {0}")]
    Request(String),
    #[error("reply is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn client_version() -> String {
    format!("bbport-jgamrot {VERSION}")
}

/// `BB_HUNTERS_DREAM=0` disables The Hunter's Dream pool, as in the Python module.
pub fn hunters_dream_pool() -> bool {
    std::env::var("BB_HUNTERS_DREAM")
        .map(|value| value != "0")
        .unwrap_or(true)
}

/// The game server as `https://host`, The Hunter's Dream by default.
pub fn community_address(value: &str) -> String {
    let value = value.trim();
    let value = if value.is_empty() {
        COMMUNITY_SERVER
    } else {
        value
    };
    let (scheme, rest) = match value.split_once("://") {
        Some((scheme, rest)) => (scheme, rest),
        None => ("", value),
    };
    let host = rest
        .split('/')
        .next()
        .unwrap_or(rest)
        .split(':')
        .next()
        .unwrap_or(rest);
    format!(
        "{}://{host}",
        if scheme.is_empty() { "https" } else { scheme }
    )
}

pub fn play_mode(settings: &Settings) -> String {
    let mode = settings.get_str("play_mode");
    if PLAY_MODES.contains(&mode.as_str()) {
        return mode;
    }
    if settings.get_bool("online") && settings.get_str("online_pool") == DEFAULT_ONLINE_POOL {
        "huntersdream".into()
    } else {
        "offline".into()
    }
}

pub fn hunters_dream_selected(settings: &Settings) -> bool {
    hunters_dream_pool() && settings.get_str("online_pool") == DEFAULT_ONLINE_POOL
}

pub fn webapi_address(server: &str) -> String {
    let rest = server.rsplit("://").next().unwrap_or(server);
    let host = rest
        .split('/')
        .next()
        .unwrap_or(rest)
        .split(':')
        .next()
        .unwrap_or(rest);
    format!("http://{host}:31315")
}

pub fn webapi_setting(settings: &Settings) -> String {
    let webapi = settings.get_str("online_webapi").trim().to_string();
    let community_host = community_address(&settings.get_str("online_community"))
        .split("://")
        .nth(1)
        .unwrap_or_default()
        .split(':')
        .next()
        .unwrap_or_default()
        .to_string();
    let webapi = if !webapi.is_empty() && webapi.contains(&community_host) {
        String::new()
    } else {
        webapi
    };
    if !webapi.is_empty() {
        return webapi;
    }
    let server = settings.get_str("online_server");
    let server = server.trim();
    let server = if server.is_empty() {
        DEFAULT_ONLINE_SERVER
    } else {
        server
    };
    webapi_address(server)
}

/// Format a connection error, noting date/time/proxy if certificate verification failed.
pub fn connection_error_text(error: &str) -> String {
    let mut text = error.to_string();
    if text.contains("CERTIFICATE_VERIFY_FAILED") {
        text.push_str(
            ". Check that the PC's date and time are right; HTTPS scanning in an \
             antivirus or a proxy can also cause this",
        );
    }
    text
}

/// Call The Hunter's Dream account service.
pub fn thd_call(fetcher: &dyn Fetcher, url: &str, body: &Value) -> Result<Value, NetError> {
    let payload = serde_json::to_string(body)?;
    let text = fetcher
        .post_json(url, &payload)
        .map_err(NetError::Request)?;
    let reply: Value = serde_json::from_str(&text)
        .map_err(|error| NetError::Service(format!("{url} has no account service ({error})")))?;
    let Some(res_kind) = reply.get("ResKind") else {
        return Err(NetError::Service(format!("{url} has no account service")));
    };
    if res_kind.as_i64() != Some(0) {
        let message = reply
            .get("Message")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or("refused");
        return Err(NetError::Service(message.to_string()));
    }
    Ok(reply)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignIn {
    pub online_id: String,
    pub token: String,
}

/// Discord sign-in flow for The Hunter's Dream account linking.
pub fn thd_sign_in(
    fetcher: &dyn Fetcher,
    game_server: &str,
    on_code: &mut dyn FnMut(&str, &str),
    cancelled: &dyn Fn() -> bool,
) -> Result<SignIn, NetError> {
    thd_sign_in_with_sleep(fetcher, game_server, on_code, cancelled, &|seconds| {
        std::thread::sleep(std::time::Duration::from_secs(seconds));
    })
}

pub fn thd_sign_in_with_sleep(
    fetcher: &dyn Fetcher,
    game_server: &str,
    on_code: &mut dyn FnMut(&str, &str),
    cancelled: &dyn Fn() -> bool,
    sleep: &dyn Fn(u64),
) -> Result<SignIn, NetError> {
    let auth = community_address(game_server);
    let host = hostname();
    let start = thd_call(
        fetcher,
        &format!("{auth}/auth/device/start"),
        &json!({"DeviceName": format!("bbport-jgamrot on {host}")}),
    )?;
    let device = start
        .get("DeviceCode")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let user_code = start
        .get("UserCode")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if device.is_empty() || user_code.is_empty() {
        return Err(NetError::Service("no code in the reply".into()));
    }
    let mut interval = start
        .get("Interval")
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .unwrap_or(5)
        .max(1);
    let deadline = now_seconds()
        + start
            .get("ExpiresIn")
            .and_then(Value::as_u64)
            .filter(|value| *value > 0)
            .unwrap_or(600);
    let page = start
        .get("VerificationUri")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{auth}/link"));
    let separator = if page.contains('?') { '&' } else { '?' };
    on_code(
        &format!("{page}{separator}code={}", encode_uri_component(&user_code)),
        &user_code,
    );

    while now_seconds() < deadline && !cancelled() {
        sleep(interval);
        let poll = thd_call(
            fetcher,
            &format!("{auth}/auth/device/poll"),
            &json!({"DeviceCode": device}),
        )?;
        match poll.get("Status").and_then(Value::as_str) {
            Some("approved") => {
                let online_id = poll
                    .get("OnlineId")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let token = poll
                    .get("Token")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if online_id.is_empty() || token.is_empty() {
                    return Err(NetError::Service("no token in the reply".into()));
                }
                return Ok(SignIn {
                    online_id: online_id.to_string(),
                    token: token.to_string(),
                });
            }
            Some("slow_down") => interval += 5,
            Some("denied") => return Err(NetError::Service("the sign-in was denied".into())),
            Some("expired") => break,
            _ => {}
        }
    }
    Err(NetError::Service(if cancelled() {
        "the sign-in was cancelled".into()
    } else {
        "the code expired; try again".into()
    }))
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "Linux".to_string())
}

fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn encode_uri_component(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// The online-aware environment from `bbport_net.py`. The Linux launcher's own
/// `game_environment` lives in [`crate::env`]; this one is kept for parity with the Windows
/// launcher, which imports the Python function.
pub fn game_environment(settings: &Settings, paths: &Paths) -> BTreeMap<String, String> {
    let mut env: BTreeMap<String, String> = std::env::vars().collect();
    env.insert("BB_PORT_VERSION".into(), VERSION.into());
    env.insert("BB_CLIENT_VERSION".into(), client_version());
    env.insert("BB_GAME_DIR".into(), settings.get_str("game_dir"));
    if !settings.get_str("user_dir").is_empty() {
        env.insert("BB_USER_DIR".into(), settings.get_str("user_dir"));
    }
    if !settings.get_str("dlc_dir").is_empty() {
        env.insert("BB_DLC_DIR".into(), settings.get_str("dlc_dir"));
    }
    let mods_dir = settings.get_str("mods_dir");
    env.insert(
        "BB_MODS_DIR".into(),
        if mods_dir.is_empty() {
            paths.mods_dir().to_string_lossy().into_owned()
        } else {
            mods_dir
        },
    );
    env.insert(
        "BB_MODS_CONFIG".into(),
        paths.mods_config().to_string_lossy().into_owned(),
    );
    env.insert(
        "BB_MODS_ENABLED".into(),
        if settings.get_bool("mods_enabled") {
            "1"
        } else {
            "0"
        }
        .into(),
    );
    let patches_dir = settings.get_str("patches_dir");
    env.insert(
        "BB_PATCHES_DIR".into(),
        if patches_dir.is_empty() {
            paths.patches_dir().to_string_lossy().into_owned()
        } else {
            patches_dir
        },
    );
    env.insert(
        "BB_PATCHES_CONFIG".into(),
        paths.patches_config().to_string_lossy().into_owned(),
    );
    env.insert("BB_LANGUAGE".into(), settings.get_str("language"));
    let player_name = settings.get_str("player_name");
    if !player_name.trim().is_empty() {
        env.insert("BB_USER_NAME".into(), player_name.trim().to_string());
    }
    env.insert(
        "BB_FULLSCREEN".into(),
        if settings.get_bool("fullscreen") {
            "1"
        } else {
            "0"
        }
        .into(),
    );
    env.insert("BB_PRESENT_MODE".into(), settings.get_str("present_mode"));
    if settings.get_bool("hdr") {
        env.insert("BB_HDR".into(), "1".into());
    }
    env.insert("BB_FPS".into(), settings.get_str("fps_mode"));
    let frame_cap = settings.get_str("frame_cap");
    let fps_limit = settings.get_i64("fps_limit");
    if !frame_cap.is_empty() {
        env.insert("BB_FPS_LIMIT".into(), frame_cap);
    } else if fps_limit > 0 {
        env.insert("BB_FPS_LIMIT".into(), fps_limit.to_string());
    }
    for (key, name) in [("draw_pipe", "BB_DRAW_PIPE"), ("readbacks", "BB_READBACKS")] {
        let value = settings.get_str(key);
        if !value.is_empty() {
            env.insert(name.into(), value);
        }
    }
    if settings.get("frames_ahead").is_some() {
        env.insert(
            "BB_FRAMES_AHEAD".into(),
            settings.get_i64("frames_ahead").to_string(),
        );
    }
    let vram = settings.get_str("vram_budget_mb");
    if !vram.is_empty() {
        env.insert("BB_GC_BUDGET_MB".into(), vram);
    }
    for (key, name) in [
        ("frame_stats", "BB_FRAME_STATS"),
        ("gpu_profile", "BB_GPU_PROFILE"),
        ("vk_validation", "BB_VK_VALIDATION"),
    ] {
        if settings.get_bool(key) {
            env.insert(name.into(), "1".into());
        }
    }
    for item in settings.get_str("extra_env").split_whitespace() {
        if let Some((key, value)) = item.split_once('=') {
            env.insert(key.to_string(), value.to_string());
        }
    }

    if settings.get_bool("online") && play_mode(settings) == "huntersdream" {
        let server = settings.get_str("online_server");
        let server = server.trim();
        let server = if server.is_empty() {
            DEFAULT_ONLINE_SERVER
        } else {
            server
        };
        env.insert("BB_ONLINE".into(), "1".into());
        env.insert("BB_SHADNET_SERVER".into(), server.to_string());
        env.insert("BB_SHADNET_WEBAPI".into(), webapi_setting(settings));
        env.insert(
            "BB_SHADNET_NPID".into(),
            settings.get_str("online_npid").trim().to_string(),
        );
        env.insert(
            "BB_SHADNET_PASSWORD".into(),
            settings.get_str("online_password"),
        );
        env.insert(
            "BB_UPNP".into(),
            if settings.get_bool("online_upnp") {
                "1"
            } else {
                "0"
            }
            .into(),
        );
        env.insert(
            "BB_COMMUNITY_SERVER".into(),
            community_address(&settings.get_str("online_community")),
        );
        if hunters_dream_selected(settings) {
            env.insert("BB_NET_BACKEND".into(), "thd".into());
            env.insert("BB_THD_ONLINE_ID".into(), settings.get_str("thd_online_id"));
            env.insert("BB_THD_TOKEN".into(), settings.get_str("thd_token"));
            for key in [
                "BB_SHADNET_SERVER",
                "BB_SHADNET_WEBAPI",
                "BB_SHADNET_NPID",
                "BB_SHADNET_PASSWORD",
            ] {
                env.remove(key);
            }
        }
    }
    env.insert("PYTHONUNBUFFERED".into(), "1".into());
    env.insert("PYTHONIOENCODING".into(), "utf-8".into());
    env
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct FakeFetcher {
        replies: RefCell<Vec<Value>>,
        urls: RefCell<Vec<String>>,
    }

    impl FakeFetcher {
        fn new(replies: Vec<Value>) -> Self {
            Self {
                replies: RefCell::new(replies),
                urls: RefCell::new(Vec::new()),
            }
        }
    }

    impl Fetcher for FakeFetcher {
        fn get(&self, _url: &str) -> Result<String, String> {
            Err("unused".into())
        }

        fn post_json(&self, url: &str, _body: &str) -> Result<String, String> {
            self.urls.borrow_mut().push(url.to_string());
            let reply = self.replies.borrow_mut().remove(0);
            Ok(reply.to_string())
        }
    }

    #[test]
    fn community_address_normalizes_scheme_and_port() {
        assert_eq!(community_address(""), COMMUNITY_SERVER);
        assert_eq!(
            community_address("thehuntersdream.com:443/x"),
            "https://thehuntersdream.com"
        );
        assert_eq!(
            community_address("http://example.com/a/b"),
            "http://example.com"
        );
    }

    #[test]
    fn webapi_address_derives_the_host() {
        assert_eq!(
            webapi_address("srv.shadps4.net:31313"),
            "http://srv.shadps4.net:31315"
        );
        assert_eq!(
            webapi_address("https://example.com"),
            "http://example.com:31315"
        );
    }

    #[test]
    fn webapi_setting_prefers_explicit_then_the_server() {
        let paths = Paths::for_port_dir(std::path::PathBuf::from("/repo"));
        let mut settings = Settings::new(&paths);
        settings.set_raw(
            "online_webapi",
            Value::String("http://example.com:31315".into()),
        );
        assert_eq!(webapi_setting(&settings), "http://example.com:31315");
        settings.set_raw("online_webapi", Value::String("".into()));
        assert_eq!(webapi_setting(&settings), "http://srv.shadps4.net:31315");
    }

    #[test]
    fn frame_cap_falls_back_to_the_linux_fps_limit() {
        let paths = Paths::for_port_dir(std::path::PathBuf::from("/repo"));
        let mut settings = Settings::new(&paths);
        settings.set_raw("frame_cap", Value::String("72".into()));
        assert_eq!(
            game_environment(&settings, &paths)
                .get("BB_FPS_LIMIT")
                .map(String::as_str),
            Some("72")
        );
        settings.set_raw("frame_cap", Value::String(String::new()));
        settings
            .set("fps_limit", serde_json::Number::from(60).into())
            .unwrap();
        assert_eq!(
            game_environment(&settings, &paths)
                .get("BB_FPS_LIMIT")
                .map(String::as_str),
            Some("60")
        );
    }

    #[test]
    fn thd_call_reports_service_errors() {
        let fetcher = FakeFetcher::new(vec![json!({"ResKind": 1, "Message": "no"})]);
        let error = thd_call(&fetcher, "https://example/auth", &json!({})).unwrap_err();
        assert!(matches!(error, NetError::Service(message) if message == "no"));
        let fetcher = FakeFetcher::new(vec![json!({"nope": 1})]);
        let error = thd_call(&fetcher, "https://example/auth", &json!({})).unwrap_err();
        assert!(error.to_string().contains("no account service"));
    }

    #[test]
    fn sign_in_polls_until_approved() {
        let fetcher = FakeFetcher::new(vec![
            json!({"ResKind": 0, "DeviceCode": "d", "UserCode": "U", "Interval": 1, "ExpiresIn": 60}),
            json!({"ResKind": 0, "Status": "approved", "OnlineId": "hunter", "Token": "t"}),
        ]);
        let mut code_url = String::new();
        let result = thd_sign_in_with_sleep(
            &fetcher,
            COMMUNITY_SERVER,
            &mut |url, _code| code_url = url.to_string(),
            &|| false,
            &|_| {},
        )
        .unwrap();
        assert_eq!(result.online_id, "hunter");
        assert!(code_url.contains("code=U"));
        assert_eq!(fetcher.urls.borrow().len(), 2);
    }
}
