//! NP profile dialog, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_profile_dialog/np_profile_dialog.{h,cpp}`
//! and its UI state.

pub const ORBIS_NP_PROFILE_DIALOG_MODE_INVALID: u32 = 0;
pub const ORBIS_NP_PROFILE_DIALOG_MODE_NORMAL: u32 = 1;
pub const ORBIS_NP_PROFILE_DIALOG_MODE_FRIEND_REQUEST: u32 = 2;
pub const ORBIS_NP_PROFILE_DIALOG_MODE_ADD_TO_BLOCK_LIST: u32 = 3;
pub const ORBIS_NP_PROFILE_DIALOG_MODE_GRIEF_REPORT: u32 = 4;

pub const ORBIS_NP_PROFILE_DIALOG_MENU_GRIEF_REPORT_ITEM_INVALID: i32 = 0x00000000;
pub const ORBIS_NP_PROFILE_DIALOG_MENU_GRIEF_REPORT_ITEM_ONLINE_ID: i32 = 0x00000001;
pub const ORBIS_NP_PROFILE_DIALOG_MENU_GRIEF_REPORT_ITEM_NAME: i32 = 0x00000002;
pub const ORBIS_NP_PROFILE_DIALOG_MENU_GRIEF_REPORT_ITEM_PICTURE: i32 = 0x00000004;
pub const ORBIS_NP_PROFILE_DIALOG_MENU_GRIEF_REPORT_ITEM_ABOUT_ME: i32 = 0x00000008;

/// `OrbisNpProfileDialogMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OrbisNpProfileDialogMode {
    Invalid = 0,
    Normal = 1,
    FriendRequest = 2,
    AddToBlockList = 3,
    GriefReport = 4,
}

/// `CommonDialog::BaseParam`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BaseParam {
    pub size: u32,
    pub mode: u32,
    pub user_data: *mut core::ffi::c_void,
}

/// `OrbisNpProfileGriefReportParam`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpProfileGriefReportParam {
    pub report_item: i32,
    pub reserved: [u8; 28],
}

/// `OrbisNpProfileDialogParam`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNpProfileDialogParam {
    pub base_param: BaseParam,
    pub size: u64,
    pub mode: u32,
    pub user_id: i32,
    pub target_online_id: [u8; 16],
    pub user_data: *mut core::ffi::c_void,
    pub reserved: [u8; 32],
}

/// `OrbisNpProfileDialogResult`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpProfileDialogResult {
    pub result: i32,
    pub user_action: i32,
    pub user_data: *mut core::ffi::c_void,
    pub reserved: [u8; 32],
}

/// `NpProfileDialogState`.
#[derive(Debug, Default)]
pub struct NpProfileDialogState {
    pub online_id: String,
    pub account_id: u64,
    pub has_account_id: bool,
    pub mode: u32,
    pub user_id: i32,
    pub finished: bool,
    pub result: OrbisNpProfileDialogResult,
}

impl NpProfileDialogState {
    pub fn open(&mut self, param: &OrbisNpProfileDialogParam) -> i32 {
        self.mode = param.mode;
        self.user_id = param.user_id;
        let length = param
            .target_online_id
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(param.target_online_id.len());
        self.online_id = String::from_utf8_lossy(&param.target_online_id[..length]).into_owned();
        self.finished = false;
        self.result = OrbisNpProfileDialogResult {
            user_data: param.user_data,
            ..OrbisNpProfileDialogResult::default()
        };
        0
    }

    pub fn finish(&mut self, user_action: i32) {
        self.finished = true;
        self.result.result = 0;
        self.result.user_action = user_action;
    }

    pub fn get_result(&self) -> OrbisNpProfileDialogResult {
        self.result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_dialogs_copy_the_target_user_and_result() {
        let mut state = NpProfileDialogState::default();
        let mut param = OrbisNpProfileDialogParam {
            mode: OrbisNpProfileDialogMode::Normal as u32,
            user_id: 3,
            ..OrbisNpProfileDialogParam::default()
        };
        param.target_online_id[..6].copy_from_slice(b"hunter");
        assert_eq!(state.open(&param), 0);
        assert_eq!(state.online_id, "hunter");
        assert_eq!(state.user_id, 3);
        state.finish(1);
        assert!(state.finished);
        assert_eq!(state.get_result().user_action, 1);
        assert_eq!(ORBIS_NP_PROFILE_DIALOG_MENU_GRIEF_REPORT_ITEM_ABOUT_ME, 8);
    }
}
