//! Starting the game through the port's `run.sh`, mirroring the Python launcher's `play()`.

use crate::env::game_environment;
use crate::paths::Paths;
use crate::Settings;
use std::io;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("bbport: choose the game folder in the launcher first")]
    NoGame,
    #[error("cannot start run.sh: {0}")]
    Io(#[from] io::Error),
    #[error("run.sh exited with {0}")]
    Exit(i32),
}

/// The `bash run.sh` command with the launcher's environment and working directory.
pub fn game_command(settings: &Settings, paths: &Paths) -> Command {
    let mut command = Command::new("bash");
    command.arg(paths.port_dir.join("run.sh"));
    command.current_dir(&paths.port_dir);
    for (key, value) in game_environment(settings, paths) {
        command.env(key, value);
    }
    command
}

/// Launch the game with saved settings. On Unix this replaces the current process, like the
/// Python launcher's `os.execvpe`.
pub fn launch(settings: &Settings, paths: &Paths) -> Result<(), LaunchError> {
    if !settings.game_dir().join("eboot.bin").is_file() {
        return Err(LaunchError::NoGame);
    }
    let mut command = game_command(settings, paths);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        Err(LaunchError::Io(command.exec()))
    }
    #[cfg(not(unix))]
    {
        let status = command.status()?;
        if !status.success() {
            return Err(LaunchError::Exit(status.code().unwrap_or(-1)));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn command_uses_the_port_run_script_and_environment() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let port = std::env::temp_dir().join(format!("bbport-rs-process-{stamp}"));
        std::fs::create_dir_all(&port).unwrap();
        let paths = Paths::with_config_dir(port.clone(), port.join("config"));
        let mut settings = Settings::new(&paths);
        settings
            .set(
                "game_dir",
                Value::String(port.join("CUSA03173").to_string_lossy().into_owned()),
            )
            .unwrap();
        let command = game_command(&settings, &paths);
        assert_eq!(command.get_program(), "bash");
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec![port.join("run.sh").to_string_lossy().into_owned()]
        );
        assert_eq!(command.get_current_dir(), Some(port.as_path()));
        let envs: Vec<(String, Option<String>)> = command
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value.map(|value| value.to_string_lossy().into_owned()),
                )
            })
            .collect();
        assert!(envs.iter().any(|(key, _)| key == "BB_GAME_DIR"));
        assert!(envs.iter().any(|(key, _)| key == "BB_MODS_CONFIG"));
        let _ = std::fs::remove_dir_all(port);
    }

    #[test]
    fn launch_without_eboot_reports_no_game() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let port = std::env::temp_dir().join(format!("bbport-rs-process-nogame-{stamp}"));
        std::fs::create_dir_all(&port).unwrap();
        let paths = Paths::with_config_dir(port.clone(), port.join("config"));
        let settings = Settings::new(&paths);
        assert!(matches!(
            launch(&settings, &paths),
            Err(LaunchError::NoGame)
        ));
        let _ = std::fs::remove_dir_all(port);
    }
}
