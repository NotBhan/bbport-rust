//! Discord rich presence, ported from `legacy/gpu/shadps4/common/discord_rpc_handler.h`. The
//! C++ links the discord-rpc library; the port takes the same calls through a backend the
//! launcher installs, and stays disabled when no backend is present.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

/// `DiscordRPCHandler::RPCStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpcStatus {
    Idling,
    Playing,
}

/// What the presence shows: the C++'s `details`/`state` pair and the start time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activity {
    pub details: String,
    pub state: String,
    pub start_timestamp: u64,
}

/// The discord-rpc calls the host provides.
pub trait DiscordBackend: Send {
    fn init(&mut self) -> bool;
    fn set_activity(&mut self, activity: Option<&Activity>);
    fn shutdown(&mut self);
}

fn backend() -> &'static Mutex<Option<Box<dyn DiscordBackend>>> {
    static BACKEND: OnceLock<Mutex<Option<Box<dyn DiscordBackend>>>> = OnceLock::new();
    BACKEND.get_or_init(|| Mutex::new(None))
}

static INSTALLED: AtomicBool = AtomicBool::new(false);

pub fn set_discord_backend(backend_impl: Box<dyn DiscordBackend>) {
    *backend().lock().unwrap_or_else(|error| error.into_inner()) = Some(backend_impl);
    INSTALLED.store(true, Ordering::Release);
}

/// `DiscordRPCHandler::RPC`.
#[derive(Debug, Default)]
pub struct Rpc {
    start_timestamp: u64,
    rpc_enabled: bool,
    status: Option<RpcStatus>,
}

impl Rpc {
    pub const fn new() -> Self {
        Self {
            start_timestamp: 0,
            rpc_enabled: false,
            status: None,
        }
    }

    /// `RPC::init`: the presence turns on only when the backend starts.
    pub fn init(&mut self) {
        let mut backend = backend().lock().unwrap_or_else(|error| error.into_inner());
        self.rpc_enabled = match backend.as_mut() {
            Some(backend) => backend.init(),
            None => false,
        };
        self.start_timestamp = now();
        self.status = Some(RpcStatus::Idling);
    }

    /// `RPC::setStatusIdling`.
    pub fn set_status_idling(&mut self) {
        self.status = Some(RpcStatus::Idling);
        if !self.rpc_enabled {
            return;
        }
        if let Some(backend) = backend()
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_mut()
        {
            backend.set_activity(None);
        }
    }

    /// `RPC::setStatusPlaying`.
    pub fn set_status_playing(&mut self, game_name: &str, game_id: &str) {
        self.status = Some(RpcStatus::Playing);
        if !self.rpc_enabled {
            return;
        }
        let activity = Activity {
            details: game_name.to_string(),
            state: game_id.to_string(),
            start_timestamp: self.start_timestamp,
        };
        if let Some(backend) = backend()
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_mut()
        {
            backend.set_activity(Some(&activity));
        }
    }

    /// `RPC::shutdown`.
    pub fn shutdown(&mut self) {
        if self.rpc_enabled {
            if let Some(backend) = backend()
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .as_mut()
            {
                backend.shutdown();
            }
        }
        self.rpc_enabled = false;
        self.status = None;
    }

    /// `RPC::getRPCEnabled`.
    pub fn rpc_enabled(&self) -> bool {
        self.rpc_enabled
    }

    pub fn status(&self) -> Option<RpcStatus> {
        self.status
    }

    pub fn start_timestamp(&self) -> u64 {
        self.start_timestamp
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}
#[cfg(test)]
mod tests {
    use super::*;

    /// The presence is a process-wide singleton, exactly as in the C++, so both the refused
    /// and the accepted paths are driven from one test that owns the global backend.
    struct Recording {
        accept: bool,
        log: &'static Mutex<Vec<String>>,
    }

    impl DiscordBackend for Recording {
        fn init(&mut self) -> bool {
            self.log
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push("init".to_string());
            self.accept
        }

        fn set_activity(&mut self, activity: Option<&Activity>) {
            let mut log = self.log.lock().unwrap_or_else(|error| error.into_inner());
            match activity {
                Some(activity) => {
                    log.push(format!("play {} / {}", activity.details, activity.state))
                }
                None => log.push("idle".to_string()),
            }
        }

        fn shutdown(&mut self) {
            self.log
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push("shutdown".to_string());
        }
    }

    static LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

    /// Installs a backend that accepts or refuses, with an empty log.
    fn install(accept: bool) {
        LOG.lock()
            .unwrap_or_else(|error| error.into_inner())
            .clear();
        set_discord_backend(Box::new(Recording { accept, log: &LOG }));
    }

    fn log() -> Vec<String> {
        LOG.lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    #[test]
    fn the_presence_only_starts_for_a_willing_backend_and_reports_each_step() {
        assert!(
            !INSTALLED.load(Ordering::Acquire),
            "nothing is installed yet"
        );

        // A backend that refuses to start leaves the presence off: later status changes are
        // recorded but never sent.
        install(false);
        let mut rpc = Rpc::new();
        assert!(!rpc.rpc_enabled());
        rpc.init();
        assert!(!rpc.rpc_enabled());
        assert_eq!(rpc.status(), Some(RpcStatus::Idling));
        assert!(rpc.start_timestamp() > 0);
        rpc.set_status_playing("Bloodborne", "CUSA00207");
        assert_eq!(rpc.status(), Some(RpcStatus::Playing));
        rpc.shutdown();
        assert_eq!(rpc.status(), None);
        assert_eq!(log(), ["init"], "a refused presence sends nothing");

        // A willing backend receives the play activity, the idle clear and the shutdown.
        install(true);
        assert!(INSTALLED.load(Ordering::Acquire));
        let mut rpc = Rpc::new();
        rpc.init();
        assert!(rpc.rpc_enabled());
        rpc.set_status_playing("Bloodborne", "CUSA00207");
        rpc.set_status_idling();
        rpc.shutdown();
        assert!(!rpc.rpc_enabled());
        assert_eq!(
            log(),
            [
                "init".to_string(),
                "play Bloodborne / CUSA00207".to_string(),
                "idle".to_string(),
                "shutdown".to_string()
            ]
        );
    }
}
