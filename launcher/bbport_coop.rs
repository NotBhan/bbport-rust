//! Seamless co-op (experimental): BBSC by CLS_AMG, set up and run from the launcher, ported
//! from `bbport_coop.py`.
//!
//! BBSC (<https://github.com/Classmg22/BBSC>, GPL-2.0-or-later) keeps a co-op session alive
//! through deaths, lamps, areas and bosses for up to five hunters. Nothing of BBSC is shipped
//! with the port: its official release packages are downloaded on demand, checked against the
//! release's published SHA-256, and unpacked under `%LOCALAPPDATA%\bbport-coop`
//! (`~/bbport-coop` elsewhere).

use crate::update::Fetcher;
use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use thiserror::Error;

pub const REPO: &str = "Classmg22/BBSC";
pub const RELEASES: &str = "https://github.com/Classmg22/BBSC/releases";
pub const DISCORD: &str = "https://discord.gg/Df7BCmfWdt";

/// `(release tag, asset, folder inside the zip)`: the BBport client package of the newest
/// release and the server package (only published with 0.0.1).
pub const PACKAGES: [(&str, &str, &str); 2] = [
    ("client", "0.0.1_hotfix", "BBSC-0.0.1-BBport.zip"),
    ("server", "v0.0.1", "BBSC-0.0.1-Server.zip"),
];

pub const SHADNET_PORT: u16 = 31313;
pub const MATCHING_PORT: u16 = 31314;
pub const WEBAPI_PORT: u16 = 31315;
pub const GAME_PORT: u16 = 20443;
pub const RELAY_PORT: u16 = 20444;
pub const INVITE_PREFIX: &str = "BBCOOP-";

/// Inbound rules: the server ports, and the game itself (the host's game takes the guests'
/// connections). Tailscale's range and this PC's own networks only - nothing opens to the
/// internet.
pub const FIREWALL_RANGES: [&str; 2] = ["100.64.0.0/10", "LocalSubnet"];
pub const FIREWALL_PORTS: [(&str, &str, u16); 7] = [
    ("shadnet", "TCP", SHADNET_PORT),
    ("matching", "UDP", MATCHING_PORT),
    ("webapi", "TCP", WEBAPI_PORT),
    ("game server", "TCP", GAME_PORT),
    ("relay", "TCP", RELAY_PORT),
    ("sessions", "TCP", 18671),
    ("STUN", "UDP", 3478),
];
pub const RULE_PREFIX: &str = "Bloodborne co-op";

#[derive(Debug, Error)]
pub enum CoopError {
    #[error("{0}")]
    Message(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Zip(#[from] crate::zipread::ZipError),
}

fn message(text: impl Into<String>) -> CoopError {
    CoopError::Message(text.into())
}

pub fn coop_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("bbport-coop")
}

fn package_entry(kind: &str) -> Result<(&'static str, &'static str, &'static str), CoopError> {
    PACKAGES
        .iter()
        .find(|(name, _, _)| *name == kind)
        .copied()
        .ok_or_else(|| message(format!("unknown co-op package {kind}")))
}

pub fn package_dir(kind: &str) -> Result<PathBuf, CoopError> {
    let (_, _, folder) = package_entry(kind)?;
    Ok(coop_dir().join(folder))
}

pub fn installed(kind: &str) -> Result<bool, CoopError> {
    let folder = package_dir(kind)?;
    let files: &[&str] = if kind == "client" {
        &[
            "bbsc/bbsc-launch.exe",
            "bbsc/bbsc.dll",
            "account/shadnet-sample.exe",
        ]
    } else {
        &[
            "server/shadnet.exe",
            "server/BBSC-GameServer.exe",
            "server/shadnet.cfg",
        ]
    };
    Ok(files.iter().all(|file| folder.join(file).is_file()))
}

/// `(download URL, published sha256)` of one release asset.
pub fn release_asset(
    fetcher: &dyn Fetcher,
    tag: &str,
    name: &str,
) -> Result<(String, String), CoopError> {
    let text = fetcher
        .get(&format!(
            "https://api.github.com/repos/{REPO}/releases/tags/{tag}"
        ))
        .map_err(message)?;
    let release: Value = serde_json::from_str(&text).map_err(|error| message(error.to_string()))?;
    for asset in release
        .get("assets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if asset.get("name").and_then(Value::as_str) != Some(name) {
            continue;
        }
        let digest = asset
            .get("digest")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(expected) = digest.strip_prefix("sha256:") else {
            return Err(message(format!(
                "{name}: the release publishes no SHA-256 to check it against."
            )));
        };
        let url = asset
            .get("browser_download_url")
            .and_then(Value::as_str)
            .ok_or_else(|| message(format!("{name} has no download URL")))?
            .to_string();
        return Ok((url, expected.to_lowercase()));
    }
    Err(message(format!(
        "{name} is not in BBSC release {tag} any more ({RELEASES})."
    )))
}

fn safe_target(root: &Path, name: &str) -> Result<PathBuf, CoopError> {
    use std::path::Component;
    let relative = Path::new(name);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(message(format!("Unsafe path in the package: {name}")));
    }
    let target = root.join(relative);
    if !target.starts_with(root) {
        return Err(message(format!("Unsafe path in the package: {name}")));
    }
    Ok(target)
}

/// Extract every member, refusing paths that escape `destination`.
pub fn safe_extract(
    archive: &crate::zipread::ZipArchive,
    destination: &Path,
) -> Result<(), CoopError> {
    let root = destination
        .canonicalize()
        .unwrap_or_else(|_| destination.to_path_buf());
    for entry in archive.entries() {
        let target = safe_target(&root, &entry.name)?;
        if entry.is_dir {
            fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = fs::File::create(&target)?;
        let mut writer = BufWriter::new(file);
        archive.extract_to(entry, &mut writer)?;
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, CoopError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// Download and unpack one BBSC package; the server package keeps an existing database,
/// registration key and logs.
pub fn download(
    kind: &str,
    fetcher: &dyn Fetcher,
    progress: Option<&dyn Fn(&str)>,
) -> Result<PathBuf, CoopError> {
    let (tag, name, folder) = package_entry(kind)?;
    let (url, expected) = release_asset(fetcher, tag, name)?;
    fs::create_dir_all(coop_dir())?;
    let zip_path = coop_dir().join(format!(".bbsc-{name}"));
    if let Some(progress) = progress {
        progress(&format!("Downloading {name}…"));
    }
    fetcher.download_to(&url, &zip_path).map_err(message)?;
    if sha256_file(&zip_path)? != expected {
        let _ = fs::remove_file(&zip_path);
        return Err(message(format!(
            "{name} does not match the checksum BBSC published; not installed."
        )));
    }
    let staging = coop_dir().join(format!(".bbsc-unpacked-{}", std::process::id()));
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)?;
    let archive = crate::zipread::ZipArchive::open(&zip_path)?;
    safe_extract(&archive, &staging)?;
    let new = staging.join(folder);
    let old = package_dir(kind)?;
    if kind == "server" && old.is_dir() {
        // The host's own data survives an update of the programs.
        for keep in ["server/shadnet.cfg", "server/db", "server/logs"] {
            let source = old.join(keep);
            if !source.exists() {
                continue;
            }
            let target = new.join(keep);
            if target.is_dir() {
                let _ = fs::remove_dir_all(&target);
            } else if target.exists() {
                let _ = fs::remove_file(&target);
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(&source, &target)?;
        }
    }
    if old.exists() {
        fs::remove_dir_all(&old)?;
    }
    if let Some(parent) = old.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(&new, &old)?;
    let _ = fs::remove_file(&zip_path);
    let _ = fs::remove_dir_all(&staging);
    Ok(old)
}

// --- Network -------------------------------------------------------------------------------

/// This machine's Tailscale address (Windows); `None` elsewhere.
pub fn tailscale_ip() -> Option<String> {
    #[cfg(windows)]
    {
        let tailscale = std::env::var("ProgramFiles")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(r"C:\Program Files"))
            .join("Tailscale")
            .join("tailscale.exe");
        if !tailscale.is_file() {
            return None;
        }
        let output = Command::new(tailscale).args(["ip", "-4"]).output().ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        return text
            .split_whitespace()
            .find(|part| is_tailscale(part))
            .map(str::to_string);
    }
    #[cfg(not(windows))]
    {
        None
    }
}

pub fn is_tailscale(address: &str) -> bool {
    let parts: Vec<&str> = address.split('.').collect();
    if parts.len() != 4 || parts[0] != "100" {
        return false;
    }
    parts[1]
        .parse::<u64>()
        .map(|second| (64..=127).contains(&second))
        .unwrap_or(false)
}

/// This PC's address on its home network (no packet is sent).
pub fn local_ip() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    socket
        .local_addr()
        .ok()
        .map(|address| address.ip().to_string())
}

/// `(address, network name)`: Tailscale when it is connected, else the home network.
pub fn host_address() -> (Option<String>, Option<&'static str>) {
    if let Some(ip) = tailscale_ip() {
        return (Some(ip), Some("Tailscale"));
    }
    if let Some(ip) = local_ip() {
        return (
            Some(ip),
            Some("your home network only (Tailscale is not connected)"),
        );
    }
    (None, None)
}

// --- Hosting -------------------------------------------------------------------------------

/// The bundled co-op server folder (`bin/coop-server` or `out/coop-server`).
pub fn bundled_server(port_dir: &Path) -> Option<PathBuf> {
    [
        port_dir.join("bin").join("coop-server"),
        port_dir.join("out").join("coop-server"),
    ]
    .into_iter()
    .find(|folder| folder.join("shadnet.exe").is_file())
}

/// The folder `shadnet.exe` runs from. With `sync`, copy the bundled programs and BBSC's
/// configuration there.
pub fn shadnet_dir(port_dir: &Path, sync: bool) -> Result<PathBuf, CoopError> {
    let Some(bundled) = bundled_server(port_dir) else {
        return Ok(package_dir("server")?.join("server"));
    };
    let target = coop_dir().join("shadnet-bbport");
    if sync {
        copy_if_newer(&bundled, &target)?;
        let bbsc = package_dir("server")?.join("server");
        for name in ["shadnet.cfg", "worlds.cfg", "scoreboards.cfg"] {
            let destination = target.join(name);
            let source = bbsc.join(name);
            if !destination.exists() && source.is_file() {
                fs::create_dir_all(&target)?;
                fs::copy(&source, &destination)?;
            }
        }
    }
    Ok(target)
}

fn copy_if_newer(source: &Path, target: &Path) -> Result<(), CoopError> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let destination = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_if_newer(&entry.path(), &destination)?;
            continue;
        }
        let source_path = entry.path();
        let needs_copy = match fs::metadata(&destination) {
            Ok(metadata) => {
                metadata.len() != entry.metadata()?.len()
                    || metadata.modified().ok() < entry.metadata()?.modified().ok()
            }
            Err(_) => true,
        };
        if needs_copy {
            fs::copy(&source_path, &destination)?;
        }
    }
    Ok(())
}

/// The host's own key (created on first use): only players who have it can make accounts.
pub fn registration_key(port_dir: &Path) -> Result<String, CoopError> {
    let mut cfg = shadnet_dir(port_dir, false)?.join("shadnet.cfg");
    if !cfg.is_file() {
        cfg = shadnet_dir(port_dir, true)?.join("shadnet.cfg");
    }
    let text = fs::read_to_string(&cfg)?;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("RegistrationSecretKey=") {
            let value = value.trim();
            if !value.is_empty() {
                return Ok(value.to_string());
            }
            let key = random_hex(20)?;
            fs::write(
                &cfg,
                text.replace(line, &format!("RegistrationSecretKey={key}")),
            )?;
            return Ok(key);
        }
    }
    let key = random_hex(20)?;
    fs::write(
        &cfg,
        format!(
            "{}\nRegistrationSecretKey={key}\n",
            text.trim_end_matches('\n')
        ),
    )?;
    Ok(key)
}

fn random_hex(bytes: usize) -> Result<String, CoopError> {
    let mut buffer = vec![0u8; bytes];
    use std::io::Read;
    let mut source = fs::File::open("/dev/urandom")?;
    source.read_exact(&mut buffer)?;
    Ok(buffer.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// The two server programs while this launcher hosts.
pub struct Host {
    processes: Vec<Child>,
    pub address: Option<String>,
}

impl Default for Host {
    fn default() -> Self {
        Self::new()
    }
}

impl Host {
    pub fn new() -> Self {
        Self {
            processes: Vec::new(),
            address: None,
        }
    }

    pub fn running(&mut self) -> bool {
        self.processes
            .iter_mut()
            .any(|process| process.try_wait().ok().flatten().is_none())
    }

    pub fn start(&mut self, address: &str, port_dir: &Path) -> Result<(), CoopError> {
        self.stop();
        let server = package_dir("server")?.join("server");
        let shadnet = shadnet_dir(port_dir, true)?;
        let logs = server.join("logs");
        fs::create_dir_all(&logs)?;
        let stamp = timestamp();
        let shadnet_log = fs::File::create(logs.join(format!("shadnet-{stamp}.log")))?;
        self.processes.push(
            windows_program(&shadnet.join("shadnet.exe"))
                .current_dir(&shadnet)
                .env("SHADNET_BLOODBORNE_SEAMLESS_COOP", "1")
                .stdout(Stdio::from(shadnet_log))
                .stderr(Stdio::null())
                .stdin(Stdio::null())
                .spawn()
                .map_err(|error| {
                    message(format!(
                        "A co-op server could not start: {error}; see {}",
                        logs.display()
                    ))
                })?,
        );
        std::thread::sleep(std::time::Duration::from_secs(2));
        let game_log = fs::File::create(logs.join(format!("gameserver-{stamp}.log")))?;
        self.processes.push(
            windows_program(&server.join("BBSC-GameServer.exe"))
                .args([
                    "--shadnet",
                    address,
                    "--host",
                    "0.0.0.0",
                    "--advertise",
                    address,
                    "--summon-interval",
                    "5",
                    "--no-playlog",
                    "--log-dir",
                ])
                .arg(&logs)
                .current_dir(&server)
                .stdout(Stdio::from(game_log))
                .stderr(Stdio::null())
                .stdin(Stdio::null())
                .spawn()
                .map_err(|error| message(format!("A co-op server could not start: {error}")))?,
        );
        std::thread::sleep(std::time::Duration::from_secs(1));
        if !self.running() {
            self.stop();
            return Err(message(format!(
                "A co-op server stopped at start; see {}.",
                logs.display()
            )));
        }
        self.address = Some(address.to_string());
        Ok(())
    }

    pub fn stop(&mut self) {
        for process in self.processes.iter_mut() {
            if process.try_wait().ok().flatten().is_none() {
                end_tree(process);
            }
        }
        self.processes.clear();
        self.address = None;
    }
}

fn timestamp() -> String {
    // `%Y%m%d-%H%M%S` without a date library: derive from the Unix clock.
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let (days, rest) = (seconds / 86_400, seconds % 86_400);
    let (hours, minutes, secs) = (rest / 3600, (rest % 3600) / 60, rest % 60);
    let (year, month, day) = civil_from_days(days as i64);
    format!("{year:04}{month:02}{day:02}-{hours:02}{minutes:02}{secs:02}")
}

/// Howard Hinnant's civil-from-days algorithm.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// BBSC-GameServer.exe runs a second copy of itself as a child: end the whole tree, or the
/// child keeps the ports after Stop hosting.
pub fn end_tree(process: &mut Child) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &process.id().to_string(), "/T", "/F"])
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = process.kill();
    }
    let _ = process.wait();
}

// --- Invites and accounts ------------------------------------------------------------------

pub fn make_invite(address: &str, key: &str) -> String {
    let payload = json!({"a": address, "k": key}).to_string();
    format!(
        "{INVITE_PREFIX}{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload)
    )
}

pub fn read_invite(code: &str) -> Result<(String, String), CoopError> {
    // Chat apps wrap long codes over lines and add spaces: none belong to the code.
    let code: String = code
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let Some(body) = code.strip_prefix(INVITE_PREFIX) else {
        return Err(message(format!(
            "An invite code starts with {INVITE_PREFIX}"
        )));
    };
    let mut padded = body.to_string();
    while padded.len() % 4 != 0 {
        padded.push('=');
    }
    let decoded = base64::engine::general_purpose::URL_SAFE
        .decode(padded)
        .map_err(|_| message("That invite code is damaged; ask the host to copy it again."))?;
    let data: Value = serde_json::from_slice(&decoded)
        .map_err(|_| message("That invite code is damaged; ask the host to copy it again."))?;
    let address = data
        .get("a")
        .and_then(Value::as_str)
        .ok_or_else(|| message("That invite code is damaged; ask the host to copy it again."))?;
    let key = data.get("k").and_then(Value::as_str).unwrap_or_default();
    Ok((address.to_string(), key.to_string()))
}

// --- Accounts ------------------------------------------------------------------------------

/// Run a BBSC `.exe`; on Linux the official Windows packages run through Wine.
fn windows_program(path: &Path) -> Command {
    #[cfg(windows)]
    {
        Command::new(path)
    }
    #[cfg(not(windows))]
    {
        let mut command = Command::new("wine");
        command.arg(path);
        command
    }
}

fn account_tool(address: &str, arguments: &[&str]) -> Result<String, CoopError> {
    let tool = package_dir("client")?
        .join("account")
        .join("shadnet-sample.exe");
    if !tool.is_file() {
        return Err(message("Set up co-op first (the BBSC package is missing)."));
    }
    let output = windows_program(&tool)
        .arg(address)
        .arg(SHADNET_PORT.to_string())
        .args(arguments)
        .current_dir(tool.parent().unwrap_or(Path::new(".")))
        .output()
        .map_err(|error| message(format!("Could not create the co-op account: {error}")))?;
    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

/// Create or verify the account on a host's shadnet server.
pub fn create_account(
    address: &str,
    key: &str,
    name: &str,
    password: &str,
) -> Result<&'static str, CoopError> {
    if !(3..=16).contains(&name.len())
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(message("A co-op name is 3-16 letters, numbers, - or _."));
    }
    if password.len() < 4 {
        return Err(message(
            "Choose a password of at least 4 characters (one you use nowhere else).",
        ));
    }
    let login = account_tool(address, &["login", name, password])?;
    if login.contains("[login] OK") {
        return Ok("signed in");
    }
    let email = format!("{name}@bbport-coop.invalid");
    let mut arguments = vec!["register", name, password, email.as_str()];
    if !key.is_empty() {
        arguments.push(key);
    }
    let registered = account_tool(address, &arguments)?;
    if !registered.contains("Account created successfully") {
        return Err(message(
            "The host's server did not create the account (the name may be taken, or the invite              is from another host).",
        ));
    }
    if !account_tool(address, &["login", name, password])?.contains("[login] OK") {
        return Err(message(
            "The account was created but does not sign in; try another name.",
        ));
    }
    Ok("created")
}

// --- The game ------------------------------------------------------------------------------

/// The normal game environment, pointed at the host's servers with BBSC loaded.
pub fn co_op_environment(
    env: &mut std::collections::BTreeMap<String, String>,
    address: &str,
    name: &str,
    password: &str,
    probe: &Path,
    save_copy: Option<&Path>,
) -> Result<(), CoopError> {
    let launcher = package_dir("client")?.join("bbsc").join("bbsc-launch.exe");
    let overrides = coop_dir().join("host_overrides.json");
    if let Some(parent) = overrides.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(&json!({
        "https://ss4.scej-network.jp:20443": format!("http://{address}:{GAME_PORT}"),
        "http://thehuntersdream.com:18671/summon_messenger": format!("http://{address}:{WEBAPI_PORT}"),
        "https://thehuntersdream.com:18671/summon_messenger": format!("http://{address}:{WEBAPI_PORT}"),
    }))
    .unwrap_or_default()
        + "\n";
    fs::write(&overrides, &text)?;
    let port_user = probe
        .canonicalize()
        .unwrap_or_else(|_| probe.to_path_buf())
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new("."))
        .join("user");
    fs::create_dir_all(&port_user)?;
    fs::write(port_user.join("host_overrides.json"), &text)?;

    for key in [
        "BB_NET_BACKEND",
        "BB_THD_TOKEN",
        "BB_THD_ONLINE_ID",
        "BB_THD_SERVER",
        "BB_THD_AUTH",
        "BB_THD_RULESET",
    ] {
        env.remove(key);
    }
    let save_dir = save_copy.and_then(active_save_dir).or_else(|| {
        env.get("BB_USER_DIR")
            .map(PathBuf::from)
            .and_then(|dir| active_save_dir(&dir))
    });
    if let Some(save_dir) = save_dir {
        env.insert(
            "BBSC_SAVE_DIR".into(),
            save_dir.to_string_lossy().into_owned(),
        );
    }
    env.insert("BBSC_RELAY".into(), format!("{address}:{RELAY_PORT}"));
    env.insert("BB_PROBE".into(), launcher.to_string_lossy().into_owned());
    env.insert("BBSC_EMULATOR".into(), probe.to_string_lossy().into_owned());
    env.insert("BB_ONLINE".into(), "1".into());
    env.insert(
        "BB_SHADNET_SERVER".into(),
        format!("{address}:{SHADNET_PORT}"),
    );
    env.insert(
        "BB_SHADNET_WEBAPI".into(),
        format!("http://{address}:{WEBAPI_PORT}"),
    );
    env.insert(
        "BB_HOST_OVERRIDES".into(),
        overrides.to_string_lossy().into_owned(),
    );
    env.insert("BB_SHADNET_NPID".into(), name.to_string());
    env.insert("BB_SHADNET_PASSWORD".into(), password.to_string());
    env.insert("BB_USER_NAME".into(), name.to_string());
    env.insert("BB_COOP".into(), "1".into());
    Ok(())
}

/// The game's save folder (`SPRJ0005`, `userdata0000...`) written most recently, or `None`.
pub fn active_save_dir(user_dir: &Path) -> Option<PathBuf> {
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let accounts = fs::read_dir(user_dir.join("savedata")).ok()?;
    for account in accounts.flatten() {
        let title = account.path().join("CUSA00207").join("SPRJ0005");
        let Ok(files) = fs::read_dir(&title) else {
            continue;
        };
        let mut latest: Option<std::time::SystemTime> = None;
        for file in files.flatten() {
            if !file.file_name().to_string_lossy().starts_with("userdata00") {
                continue;
            }
            if let Ok(metadata) = file.metadata() {
                if metadata.is_file() {
                    if let Ok(modified) = metadata.modified() {
                        latest = Some(latest.map_or(modified, |current| current.max(modified)));
                    }
                }
            }
        }
        if let Some(latest) = latest {
            if newest
                .as_ref()
                .map(|(time, _)| latest > *time)
                .unwrap_or(true)
            {
                newest = Some((latest, title));
            }
        }
    }
    newest.map(|(_, path)| path)
}

/// A copy of the saves before every co-op session (BBSC 0.0.1 is a first test).
pub fn backup_saves(user_dir: &Path) -> Result<Option<PathBuf>, CoopError> {
    let saves = user_dir.join("savedata");
    if !saves.is_dir() {
        return Ok(None);
    }
    let target = user_dir
        .join("coop-backups")
        .join(timestamp())
        .join("savedata");
    copy_if_newer(&saves, &target)?;
    Ok(Some(target.parent().unwrap_or(&target).to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_names_and_passwords_are_validated() {
        let error = create_account("100.64.0.1", "", "x", "password").unwrap_err();
        assert!(error.to_string().contains("3-16"));
        let error = create_account("100.64.0.1", "", "hunter", "x").unwrap_err();
        assert!(error.to_string().contains("at least 4"));
    }

    #[test]
    fn invites_round_trip() {
        let code = make_invite("100.64.0.1", "secret");
        assert!(code.starts_with(INVITE_PREFIX));
        let (address, key) = read_invite(&code).unwrap();
        assert_eq!(address, "100.64.0.1");
        assert_eq!(key, "secret");
        // Whitespace from chat apps is ignored.
        let wrapped = format!("{INVITE_PREFIX} {}\n", &code[INVITE_PREFIX.len()..]);
        assert_eq!(read_invite(&wrapped).unwrap().0, "100.64.0.1");
        assert!(read_invite("nope").is_err());
    }

    #[test]
    fn zip_paths_cannot_escape_the_destination() {
        let root = Path::new("/tmp/bbport-coop-root");
        assert_eq!(
            safe_target(root, "server/shadnet.exe").unwrap(),
            root.join("server/shadnet.exe")
        );
        for unsafe_name in [
            "",
            "..",
            "../outside",
            "server/../../outside",
            "/etc/passwd",
        ] {
            assert!(
                safe_target(root, unsafe_name).is_err(),
                "{unsafe_name} must be rejected"
            );
        }
    }

    #[test]
    fn tailscale_addresses_are_recognized() {
        assert!(is_tailscale("100.64.0.1"));
        assert!(is_tailscale("100.127.255.254"));
        assert!(!is_tailscale("100.128.0.1"));
        assert!(!is_tailscale("10.0.0.1"));
        assert!(!is_tailscale("100.64.0"));
    }

    #[test]
    fn active_save_dir_picks_the_newest_folder() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("bbport-rs-coop-{stamp}"));
        let old = dir.join("savedata/1/CUSA00207/SPRJ0005");
        let new = dir.join("savedata/2/CUSA00207/SPRJ0005");
        fs::create_dir_all(&old).unwrap();
        fs::create_dir_all(&new).unwrap();
        fs::write(old.join("userdata0000"), b"old").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(new.join("userdata0000"), b"new").unwrap();
        assert_eq!(active_save_dir(&dir).unwrap(), new);
        let backup = backup_saves(&dir).unwrap().unwrap();
        assert!(backup.join("savedata").is_dir());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn co_op_environment_points_at_the_host() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("bbport-rs-coop-env-{stamp}"));
        let probe = dir.join("bin").join("bb-probe");
        fs::create_dir_all(probe.parent().unwrap()).unwrap();
        fs::write(&probe, b"x").unwrap();
        let mut env = std::collections::BTreeMap::new();
        co_op_environment(&mut env, "100.64.0.1", "hunter", "pw", &probe, None).unwrap();
        assert_eq!(env.get("BB_COOP").map(String::as_str), Some("1"));
        assert_eq!(
            env.get("BB_SHADNET_SERVER").map(String::as_str),
            Some("100.64.0.1:31313")
        );
        assert!(dir.join("user").join("host_overrides.json").is_file());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn timestamp_is_sortable() {
        let stamp = timestamp();
        assert_eq!(stamp.len(), 15);
        assert!(stamp
            .chars()
            .all(|character| character.is_ascii_digit() || character == '-'));
    }
}
