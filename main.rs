//! `bbport-rs`: the headless core of the Rust launcher port.
//!
//! The first milestone covers the launcher's non-UI logic (settings, bbport.ini, game
//! readiness, update checks, launch environment and start-up routing) behind a small CLI.
//! The GTK4/libadwaita front end is the next milestone.

use bbport_launcher::ini::Ini;
use bbport_launcher::paths::Paths;
use bbport_launcher::readiness::{self, Check};
use bbport_launcher::settings::{Settings, SettingsError};
use bbport_launcher::startup;
use bbport_launcher::update::{self, CurlFetcher};
use bbport_launcher::{process, sfo, VERSION};
use serde_json::{json, Number, Value};
use std::path::PathBuf;

fn main() {
    #[cfg(unix)]
    {
        // Standard Unix CLI behaviour: exit quietly when the reader closes the pipe
        // (`bbport-rs status | head`), instead of a broken-pipe panic.
        unsafe {
            libc::signal(libc::SIGPIPE, libc::SIG_DFL);
        }
    }
    std::process::exit(run(std::env::args().skip(1).collect()));
}

#[derive(Debug, Default)]
struct Cli {
    json: bool,
    port_dir: Option<PathBuf>,
    rest: Vec<String>,
}

fn parse_cli(argv: Vec<String>) -> Result<Cli, String> {
    let mut cli = Cli::default();
    let mut args = argv.into_iter();
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--json" => cli.json = true,
            "--port-dir" | "-p" => {
                let value = args.next().ok_or("--port-dir needs a folder")?;
                cli.port_dir = Some(PathBuf::from(value));
            }
            "--" => {
                cli.rest.extend(args);
                break;
            }
            other if other.starts_with("--port-dir=") => {
                cli.port_dir = Some(PathBuf::from(other.trim_start_matches("--port-dir=")));
            }
            _ => cli.rest.push(argument),
        }
    }
    Ok(cli)
}

fn run(argv: Vec<String>) -> i32 {
    let cli = match parse_cli(argv) {
        Ok(cli) => cli,
        Err(message) => {
            eprintln!("bbport-rs: {message}");
            return 2;
        }
    };
    let paths = match &cli.port_dir {
        Some(port_dir) => Paths::for_port_dir(port_dir.clone()),
        None => Paths::discover(),
    };
    let command = match cli.rest.first() {
        Some(command) => command.clone(),
        None => {
            print_help();
            return 2;
        }
    };
    let rest: Vec<String> = cli.rest.iter().skip(1).cloned().collect();
    match command.as_str() {
        "status" => cmd_status(&paths, cli.json),
        "route" => cmd_route(&paths, &rest),
        "settings" => cmd_settings(&paths, &rest),
        "ini" => cmd_ini(&paths, &rest),
        "mods" => cmd_mods(&paths),
        "patches" => cmd_patches(&paths),
        "coop" => cmd_coop(&paths, &rest),
        "vulkan" => cmd_vulkan(),
        "check-update" => cmd_check_update(&paths, cli.json),
        "launch" | "play" => cmd_launch(&paths),
        "version" | "--version" | "-V" => {
            println!("bbport-rs {VERSION}");
            0
        }
        "help" | "--help" | "-h" => {
            print_help();
            0
        }
        other => {
            eprintln!("bbport-rs: unknown command `{other}`");
            print_help();
            2
        }
    }
}

fn print_help() {
    println!(
        "bbport-rs {VERSION} — Bloodborne PC launcher core (Rust port)\n\
         \n\
         USAGE:\n  bbport-rs [--port-dir DIR] [--json] COMMAND [ARGS]\n\
         \n\
         COMMANDS:\n\
           status                 game, trophy and save readiness\n\
           route [ARGS]           start-up routing decision (use --setup/--launcher)\n\
           settings list|get K|set K V   settings.json\n\
           ini list|get K|set K V        bbport.ini\n\
           mods                   installed and unrecognized loose mods\n\
           patches                external patch XML files and their selection\n\
           vulkan                 host NVIDIA ICD setup status\n\
           check-update           newest GitHub release\n\
           launch                 start the game with saved settings (bash run.sh)\n\
           version                print the port version"
    );
}

fn cmd_status(paths: &Paths, json_output: bool) -> i32 {
    let settings = Settings::load(paths);
    let game_dir = settings.game_dir();
    let game = (!game_dir.as_os_str().is_empty()).then_some(game_dir.as_path());
    let saves = readiness::saves_dir(&settings, &paths.data_dir);
    let checks = readiness::check_readiness(game, &saves);
    let info = game.and_then(sfo::game_info);

    if json_output {
        let checks_json: Vec<Value> = checks
            .iter()
            .map(|check| {
                json!({
                    "label": check.label,
                    "level": check.level.as_str(),
                    "detail": check.detail,
                })
            })
            .collect();
        println!(
            "{}",
            json!({
                "version": VERSION,
                "port_dir": paths.port_dir,
                "data_dir": paths.data_dir,
                "config_dir": paths.config_dir,
                "game_dir": game_dir,
                "saves_dir": saves,
                "packaged": paths.packaged,
                "game": info.as_ref().map(|info| json!({"title": info.title, "version": info.version})),
                "checks": checks_json,
            })
        );
    } else {
        println!(
            "bbport-rs {VERSION} ({} layout)",
            if paths.packaged { "packaged" } else { "source" }
        );
        println!("port:    {}", paths.port_dir.display());
        println!("data:    {}", paths.data_dir.display());
        println!("config:  {}", paths.config_dir.display());
        println!("game:    {}", game_dir.display());
        println!("saves:   {}", saves.display());
        if let Some(info) = &info {
            println!("title:   {} ({})", info.title, info.version);
        }
        for check in &checks {
            println!(
                "[{:7}] {}: {}",
                check.level.as_str(),
                check.label,
                check.detail
            );
        }
    }
    if checks
        .iter()
        .any(|check: &Check| check.level == readiness::Level::Error)
    {
        1
    } else {
        0
    }
}

fn cmd_route(paths: &Paths, args: &[String]) -> i32 {
    let settings = Settings::load(paths);
    let route = startup::route(&settings, args, &paths.data_dir, false);
    println!(
        "{} {} {} {}",
        route.mode.as_str(),
        route.page,
        route.step,
        if route.problem.is_empty() {
            "-"
        } else {
            route.problem.as_str()
        }
    );
    0
}

fn cmd_settings(paths: &Paths, args: &[String]) -> i32 {
    let action = args.first().map(String::as_str).unwrap_or("list");
    match action {
        "list" => {
            let settings = Settings::load(paths);
            match serde_json::to_string_pretty(settings.values()) {
                Ok(text) => {
                    println!("{text}");
                    0
                }
                Err(error) => {
                    eprintln!("bbport-rs: cannot print settings: {error}");
                    1
                }
            }
        }
        "get" => {
            let Some(key) = args.get(1) else {
                eprintln!("bbport-rs: settings get needs a key");
                return 2;
            };
            let settings = Settings::load(paths);
            match settings.get(key) {
                Some(Value::String(text)) => {
                    println!("{text}");
                    0
                }
                Some(value) => {
                    println!("{value}");
                    0
                }
                None => {
                    eprintln!("bbport-rs: unknown setting `{key}`");
                    2
                }
            }
        }
        "set" => {
            let (Some(key), Some(raw)) = (args.get(1), args.get(2)) else {
                eprintln!("bbport-rs: settings set needs a key and a value");
                return 2;
            };
            let mut settings = Settings::load(paths);
            if let Err(error) = settings.set(key, parse_value(raw)) {
                eprintln!("bbport-rs: {error}");
                return match error {
                    SettingsError::UnknownKey(_) => 2,
                    _ => 1,
                };
            }
            if let Err(error) = settings.save(paths) {
                eprintln!("bbport-rs: {error}");
                return 1;
            }
            match settings.get(key) {
                Some(Value::String(text)) => println!("{key}={text}"),
                Some(value) => println!("{key}={value}"),
                None => {}
            }
            0
        }
        other => {
            eprintln!("bbport-rs: unknown settings action `{other}`");
            2
        }
    }
}

fn cmd_ini(paths: &Paths, args: &[String]) -> i32 {
    let path = paths.ini_path();
    let action = args.first().map(String::as_str).unwrap_or("list");
    match action {
        "list" => {
            let ini = Ini::load(&path);
            for key in ini.keys() {
                println!("{key}={}", ini.get(key).unwrap_or_default());
            }
            0
        }
        "get" => {
            let Some(key) = args.get(1) else {
                eprintln!("bbport-rs: ini get needs a key");
                return 2;
            };
            let ini = Ini::load(&path);
            println!("{}", ini.get(key).unwrap_or_default());
            0
        }
        "set" => {
            let (Some(key), Some(value)) = (args.get(1), args.get(2)) else {
                eprintln!("bbport-rs: ini set needs a key and a value");
                return 2;
            };
            let mut ini = Ini::load(&path);
            ini.set(key, value.clone());
            if let Err(error) = ini.save(&path) {
                eprintln!("bbport-rs: cannot write {}: {error}", path.display());
                return 1;
            }
            println!("{key}={value}");
            0
        }
        other => {
            eprintln!("bbport-rs: unknown ini action `{other}`");
            2
        }
    }
}

fn cmd_check_update(paths: &Paths, json_output: bool) -> i32 {
    let fetcher = CurlFetcher;
    match update::latest_release(&fetcher, paths) {
        Ok(release) => {
            if json_output {
                println!(
                    "{}",
                    json!({
                        "tag": release.tag,
                        "asset": release.asset_url,
                        "page": release.page,
                        "newer": update::release_is_newer(&release.tag, VERSION),
                    })
                );
            } else {
                println!("latest: {}", release.tag);
                match &release.asset_url {
                    Some(url) => println!("asset:  {url}"),
                    None => println!("asset:  none for this platform"),
                }
                println!("page:   {}", release.page);
            }
            0
        }
        Err(error) => {
            eprintln!("bbport-rs: update check failed: {error}");
            if let Some((tag, _url, page)) = update::remembered_update(paths) {
                eprintln!("bbport-rs: last known release {tag} ({page})");
            }
            1
        }
    }
}

fn cmd_mods(paths: &Paths) -> i32 {
    let mods_dir = paths.mods_dir();
    println!("mods:       {}", mods_dir.display());
    let installed = bbport_launcher::mods::discover(&mods_dir);
    if installed.is_empty() {
        println!("installed:  none");
    } else {
        println!("installed:  {}", installed.join(", "));
    }
    let selected = match bbport_launcher::mods::selected(&mods_dir, Some(&paths.mods_config())) {
        Ok(selected) => selected,
        Err(error) => {
            eprintln!("bbport-rs: {error}");
            return 1;
        }
    };
    println!(
        "selected:   {}",
        if selected.is_empty() {
            "-".to_string()
        } else {
            selected.join(", ")
        }
    );
    let unrecognized = bbport_launcher::mods::unrecognized(&mods_dir);
    for (name, reason) in unrecognized {
        println!("unrecognized: {name}: {reason}");
    }
    0
}

fn cmd_patches(paths: &Paths) -> i32 {
    let settings = Settings::load(paths);
    let directory = bbport_launcher::env::patches_dir(&settings, &paths.data_dir);
    let found = bbport_launcher::patches::external_patches(&directory, "01.09", None);
    let selected =
        bbport_launcher::patches::external_selection(&found, Some(&paths.patches_config()));
    println!("patches:    {}", directory.display());
    if found.is_empty() {
        println!("found:      none");
    }
    for patch in &found {
        let enabled = selected.iter().any(|chosen| chosen.key == patch.key);
        println!("[{}] {}", if enabled { "x" } else { " " }, patch.key);
    }
    0
}

fn cmd_coop(paths: &Paths, args: &[String]) -> i32 {
    let action = args.first().map(String::as_str).unwrap_or("status");
    match action {
        "status" => {
            for kind in ["client", "server"] {
                match bbport_launcher::coop::installed(kind) {
                    Ok(true) => println!("{kind}: installed"),
                    Ok(false) => println!("{kind}: not installed"),
                    Err(error) => println!("{kind}: {error}"),
                }
            }
            println!("data: {}", bbport_launcher::coop::coop_dir().display());
            match bbport_launcher::coop::host_address() {
                (Some(address), Some(network)) => println!("host: {address} ({network})"),
                (Some(address), None) => println!("host: {address}"),
                (None, _) => println!("host: none"),
            }
            0
        }
        "setup" => {
            let fetcher = CurlFetcher;
            for kind in ["client", "server"] {
                match bbport_launcher::coop::download(
                    kind,
                    &fetcher,
                    Some(&|text| {
                        eprintln!("{text}");
                    }),
                ) {
                    Ok(path) => println!("{kind}: {}", path.display()),
                    Err(error) => {
                        eprintln!("bbport-rs: {kind} setup failed: {error}");
                        return 1;
                    }
                }
            }
            0
        }
        "invite" => {
            let address = args
                .get(1)
                .cloned()
                .or_else(|| bbport_launcher::coop::host_address().0);
            let Some(address) = address else {
                eprintln!("bbport-rs: no host address; connect Tailscale or pass one");
                return 1;
            };
            match bbport_launcher::coop::registration_key(&paths.port_dir) {
                Ok(key) => {
                    println!("{}", bbport_launcher::coop::make_invite(&address, &key));
                    0
                }
                Err(error) => {
                    eprintln!("bbport-rs: cannot make invite: {error}");
                    1
                }
            }
        }
        "account" => {
            let (Some(code), Some(name), Some(password)) = (args.get(1), args.get(2), args.get(3))
            else {
                eprintln!("bbport-rs: coop account needs INVITE NAME PASSWORD");
                return 2;
            };
            let (address, key) = match bbport_launcher::coop::read_invite(code) {
                Ok(parts) => parts,
                Err(error) => {
                    eprintln!("bbport-rs: {error}");
                    return 1;
                }
            };
            match bbport_launcher::coop::create_account(&address, &key, name, password) {
                Ok(result) => {
                    println!("{result}: {name} on {address}");
                    0
                }
                Err(error) => {
                    eprintln!("bbport-rs: {error}");
                    1
                }
            }
        }
        other => {
            eprintln!("bbport-rs: unknown coop action `{other}`");
            2
        }
    }
}

fn cmd_vulkan() -> i32 {
    let mut env: std::collections::BTreeMap<String, String> = std::env::vars().collect();
    match bbport_launcher::vulkan::configure(&mut env) {
        Ok(status) => {
            println!("{status}");
            for key in ["VK_DRIVER_FILES", "LD_LIBRARY_PATH"] {
                if let Some(value) = env.get(key) {
                    println!("{key}={value}");
                }
            }
            0
        }
        Err(error) => {
            eprintln!("bbport-rs: cannot prepare the Vulkan driver: {error}");
            1
        }
    }
}

fn cmd_launch(paths: &Paths) -> i32 {
    let settings = Settings::load(paths);
    let saves = readiness::saves_dir(&settings, &paths.data_dir);
    let _ = readiness::ensure_baked_trophy_key(paths, &saves);
    match process::launch(&settings, paths) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("bbport-rs: {error}");
            1
        }
    }
}

/// CLI value parsing: booleans and integers become JSON scalars, everything else a string.
fn parse_value(raw: &str) -> Value {
    match raw {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => match raw.parse::<i64>() {
            Ok(number) => Value::Number(Number::from(number)),
            Err(_) => match raw.parse::<f64>() {
                Ok(number) if number.is_finite() && raw.contains('.') => {
                    Value::Number(Number::from_f64(number).unwrap_or_else(|| Number::from(0)))
                }
                _ => Value::String(raw.to_string()),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_flags_and_commands() {
        let cli = parse_cli(vec![
            "--json".into(),
            "--port-dir".into(),
            "/tmp/port".into(),
            "status".into(),
        ])
        .unwrap();
        assert!(cli.json);
        assert_eq!(cli.port_dir, Some(PathBuf::from("/tmp/port")));
        assert_eq!(cli.rest, vec!["status"]);

        let cli = parse_cli(vec!["--port-dir=/tmp/port".into(), "launch".into()]).unwrap();
        assert_eq!(cli.port_dir, Some(PathBuf::from("/tmp/port")));
    }

    #[test]
    fn values_parse_as_json_scalars() {
        assert_eq!(parse_value("true"), Value::Bool(true));
        assert_eq!(parse_value("12"), Value::Number(Number::from(12)));
        assert_eq!(
            parse_value("0.5"),
            Value::Number(Number::from_f64(0.5).unwrap())
        );
        assert_eq!(parse_value("Hunter"), Value::String("Hunter".into()));
    }
}
