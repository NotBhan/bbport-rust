//! User-service types, ported from `shadps4/core/libraries/system/userservice.h`.

pub const ORBIS_USER_SERVICE_MAX_LOGIN_USERS: i32 = 4;
pub const ORBIS_USER_SERVICE_MAX_REGISTER_USERS: i32 = 16;
pub const ORBIS_USER_SERVICE_MAX_USER_NAME_LENGTH: i32 = 16;

pub const ORBIS_USER_SERVICE_USER_ID_SYSTEM: i32 = 0xFF;
pub const ORBIS_USER_SERVICE_USER_ID_INVALID: i32 = -1;

/// `OrbisUserServiceUserId`.
pub type OrbisUserServiceUserId = i32;

pub const ORBIS_KERNEL_PRIO_FIFO_LOWEST: i32 = 0x2FF;
pub const ORBIS_KERNEL_PRIO_FIFO_NORMAL: i32 = 0x2BC;
pub const ORBIS_KERNEL_PRIO_FIFO_HIGHEST: i32 = 0x100;

/// `OrbisUserServiceInitializeParams`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisUserServiceInitializeParams {
    pub priority: i32,
}

/// `OrbisUserServiceLoginUserIdList`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisUserServiceLoginUserIdList {
    pub user_id: [i32; ORBIS_USER_SERVICE_MAX_LOGIN_USERS as usize],
}

/// `OrbisUserServiceRegisteredUserIdList`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisUserServiceRegisteredUserIdList {
    pub user_id: [OrbisUserServiceUserId; ORBIS_USER_SERVICE_MAX_REGISTER_USERS as usize],
}

/// `OrbisUserServiceUserColor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OrbisUserServiceUserColor {
    Blue = 0,
    Red = 1,
    Green = 2,
    Pink = 3,
}

/// `OrbisUserServiceEventType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OrbisUserServiceEventType {
    Login = 0,
    Logout = 1,
}

/// `OrbisUserServiceEvent`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbisUserServiceEvent {
    pub event: OrbisUserServiceEventType,
    pub user_id: OrbisUserServiceUserId,
}

/// The user-service entry points; the HLE loader implements them.
pub trait UserServiceApi {
    fn get_initial_user(&mut self, user_id: &mut i32) -> i32;
    fn get_event(&mut self, event: &mut OrbisUserServiceEvent) -> i32;
    fn register_user(&mut self, user_id: OrbisUserServiceUserId) -> i32;
    fn unregister_user(&mut self, user_id: OrbisUserServiceUserId) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_user_service_types_match_the_header() {
        assert_eq!(core::mem::size_of::<OrbisUserServiceInitializeParams>(), 4);
        assert_eq!(core::mem::size_of::<OrbisUserServiceLoginUserIdList>(), 16);
        assert_eq!(
            core::mem::size_of::<OrbisUserServiceRegisteredUserIdList>(),
            64
        );
        assert_eq!(core::mem::size_of::<OrbisUserServiceEvent>(), 8);
        assert_eq!(core::mem::offset_of!(OrbisUserServiceEvent, user_id), 4);
        assert_eq!(OrbisUserServiceUserColor::Pink as u32, 3);
        assert_eq!(OrbisUserServiceEventType::Logout as u32, 1);
        assert_eq!(ORBIS_USER_SERVICE_MAX_USER_NAME_LENGTH, 16);
        assert_eq!(ORBIS_USER_SERVICE_USER_ID_SYSTEM, 0xFF);
    }
}
