//! shadPS4's one logged-in user, ported from `legacy/gpu/shim/core/user_manager.h`.

#[derive(Debug, Clone, Default)]
pub struct User {
    pub user_id: i32,
    pub user_name: String,
    pub user_color: u32,
    pub player_index: i32,
    pub logged_in: bool,
    pub shadnet_npid: String,
    pub shadnet_password: String,
    pub shadnet_token: String,
    pub shadnet_email: String,
    pub shadnet_enabled: bool,
    pub np_country: String,
    pub np_language: String,
    pub np_age: u8,
    pub np_date_of_birth: String,
}

pub type LoggedInUsers = [Option<User>; 4];

pub const RUNTIME_USER_ID: i32 = 1;

#[derive(Debug, Clone)]
pub struct UserManager {
    user: User,
}

impl Default for UserManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UserManager {
    pub fn new() -> Self {
        let text = |name: &str, fallback: &str| {
            std::env::var(name)
                .ok()
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| fallback.to_string())
        };
        let npid = text("BB_SHADNET_NPID", "");
        Self {
            user: User {
                user_id: RUNTIME_USER_ID,
                user_name: text("BB_USER_NAME", "Hunter"),
                user_color: 1,
                player_index: 1,
                logged_in: true,
                shadnet_enabled: !npid.is_empty(),
                shadnet_npid: npid,
                shadnet_password: text("BB_SHADNET_PASSWORD", ""),
                shadnet_token: text("BB_SHADNET_TOKEN", ""),
                np_country: text("BB_NP_COUNTRY", "us"),
                np_language: text("BB_NP_LANGUAGE", "en"),
                np_age: 30,
                np_date_of_birth: "1994-01-01".into(),
                ..User::default()
            },
        }
    }

    pub fn get_user_by_id(&self, user_id: i32) -> Option<&User> {
        (user_id == RUNTIME_USER_ID).then_some(&self.user)
    }

    pub fn get_logged_in_users(&self) -> LoggedInUsers {
        [Some(self.user.clone()), None, None, None]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_runtime_user_is_id_one_and_logged_in() {
        let manager = UserManager::new();
        let user = manager.get_user_by_id(1).unwrap();
        assert!(!user.user_name.is_empty());
        assert!(user.logged_in);
        assert!(manager.get_user_by_id(2).is_none());
        assert!(manager.get_logged_in_users()[0].is_some());
    }
}
