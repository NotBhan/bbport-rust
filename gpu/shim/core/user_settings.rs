//! shadPS4's user-settings singleton, ported from `legacy/gpu/shim/core/user_settings.h`.

use super::user_manager::UserManager;
use std::sync::OnceLock;

pub struct UserSettings {
    manager: UserManager,
}

impl UserSettings {
    fn new() -> Self {
        Self {
            manager: UserManager::new(),
        }
    }

    /// `UserSettingsImpl::GetInstance` (the process-wide singleton).
    pub fn instance() -> &'static Self {
        static INSTANCE: OnceLock<UserSettings> = OnceLock::new();
        INSTANCE.get_or_init(Self::new)
    }

    pub fn user_manager(&self) -> &UserManager {
        &self.manager
    }
}
