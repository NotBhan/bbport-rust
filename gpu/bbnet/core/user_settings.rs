//! The user settings singleton, ported from `legacy/gpu/bbnet/include/core/user_settings.h`.
//! The C++'s `UserSettings` macro is an accessor for the one process-wide instance; the port
//! keeps that call and the user manager behind it.

use crate::gpu::common::singleton::instance;
use crate::gpu::shim::core::user_manager::UserManager;

/// `UserSettingsImpl`: the single settings object.
#[derive(Debug, Default)]
pub struct UserSettingsImpl {
    manager: UserManager,
}

impl UserSettingsImpl {
    /// `UserSettingsImpl::GetInstance`.
    pub fn get_instance() -> &'static Self {
        instance::<Self>()
    }

    /// `UserSettingsImpl::GetUserManager`.
    pub fn get_user_manager(&self) -> &UserManager {
        &self.manager
    }

    pub fn get_user_manager_mut(&mut self) -> &mut UserManager {
        &mut self.manager
    }

    /// The `UserManagement` macro: the user manager of the singleton.
    pub fn user_management() -> &'static UserManager {
        Self::get_instance().get_user_manager()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_and_the_manager_behind_them_are_singletons() {
        let settings = UserSettingsImpl::get_instance();
        assert!(std::ptr::eq(settings, UserSettingsImpl::get_instance()));
        assert!(std::ptr::eq(
            settings.get_user_manager(),
            UserSettingsImpl::user_management()
        ));
        let manager = settings.get_user_manager();
        let user = manager
            .get_user_by_id(crate::gpu::shim::core::user_manager::RUNTIME_USER_ID)
            .expect("the runtime user is always present");
        assert!(!user.user_name.is_empty());
        assert!(user.logged_in);
    }
}
