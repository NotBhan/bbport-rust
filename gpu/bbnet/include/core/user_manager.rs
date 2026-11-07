//! The single runtime user, ported from `bbnet/include/core/user_manager.h`.

/// `User`.
#[derive(Debug, Clone, PartialEq, Eq)]
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

impl Default for User {
    fn default() -> Self {
        Self {
            user_id: -1,
            user_name: String::new(),
            user_color: 1,
            player_index: 0,
            logged_in: false,
            shadnet_npid: String::new(),
            shadnet_password: String::new(),
            shadnet_token: String::new(),
            shadnet_email: String::new(),
            shadnet_enabled: false,
            np_country: "us".to_string(),
            np_language: "en".to_string(),
            np_age: 30,
            np_date_of_birth: "1994-01-01".to_string(),
        }
    }
}

/// `LoggedInUsers`.
pub type LoggedInUsers<'a> = [Option<&'a User>; 4];

/// `UserManager`.
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
    /// `RuntimeUserId`.
    pub const RUNTIME_USER_ID: i32 = 1;

    /// `UserManager()`.
    pub fn new() -> Self {
        let text = |name: &str, fallback: &str| {
            let value = std::env::var(name).unwrap_or_default();
            if value.is_empty() {
                fallback.to_string()
            } else {
                value
            }
        };
        let mut user = User {
            user_id: Self::RUNTIME_USER_ID,
            user_name: text("BB_USER_NAME", "Hunter"),
            player_index: 1,
            logged_in: true,
            shadnet_npid: text("BB_SHADNET_NPID", ""),
            shadnet_password: text("BB_SHADNET_PASSWORD", ""),
            shadnet_token: text("BB_SHADNET_TOKEN", ""),
            shadnet_enabled: false,
            np_country: text("BB_NP_COUNTRY", "us"),
            np_language: text("BB_NP_LANGUAGE", "en"),
            ..User::default()
        };
        user.shadnet_enabled = !user.shadnet_npid.is_empty();
        Self { user }
    }

    /// `GetUserByID`.
    pub fn get_user_by_id(&self, user_id: i32) -> Option<&User> {
        (user_id == Self::RUNTIME_USER_ID).then_some(&self.user)
    }

    /// `GetLoggedInUsers`.
    pub fn get_logged_in_users(&self) -> LoggedInUsers<'_> {
        [Some(&self.user), None, None, None]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_runtime_user_is_logged_in() {
        let manager = UserManager::new();
        assert_eq!(
            manager.get_user_by_id(UserManager::RUNTIME_USER_ID),
            Some(&manager.user)
        );
        assert!(manager.get_user_by_id(2).is_none());
        let users = manager.get_logged_in_users();
        assert!(users[0].is_some());
        assert!(users[1..].iter().all(Option::is_none));
        assert_eq!(users[0].unwrap().user_id, 1);
        assert_eq!(users[0].unwrap().player_index, 1);
    }
}
