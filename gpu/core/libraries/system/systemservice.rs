//! System-service types, ported from
//! `shadps4/core/libraries/system/systemservice.h`.

use crate::gpu::core::libraries::system::userservice::OrbisUserServiceUserId;

/// `OrbisSystemServiceParamId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum OrbisSystemServiceParamId {
    Lang = 1,
    DateFormat = 2,
    TimeFormat = 3,
    TimeZone = 4,
    Summertime = 5,
    SystemName = 6,
    GameParentalLevel = 7,
    EnterButtonAssign = 1000,
}

/// `OrbisSystemParamDateFormat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum OrbisSystemParamDateFormat {
    FmtYYYYMMDD = 0,
    FmtDDMMYYYY = 1,
    FmtMMDDYYYY = 2,
}

/// `OrbisSystemParamTimeFormat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum OrbisSystemParamTimeFormat {
    Fmt12Hour = 0,
    Fmt24Hour = 1,
}

/// `OrbisSystemParamGameParentalLevel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum OrbisSystemParamGameParentalLevel {
    Off = 0,
    Level01 = 1,
    Level02 = 2,
    Level03 = 3,
    Level04 = 4,
    Level05 = 5,
    Level06 = 6,
    Level07 = 7,
    Level08 = 8,
    Level09 = 9,
    Level10 = 10,
    Level11 = 11,
}

/// `OrbisSystemParamEnterButtonAssign`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum OrbisSystemParamEnterButtonAssign {
    Circle = 0,
    Cross = 1,
}

/// `OrbisSystemServiceEventType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum OrbisSystemServiceEventType {
    Invalid = -1,
    OnResume = 0x10000000,
    GameLiveStreamingStatusUpdate = 0x10000001,
    SessionInvitation = 0x10000002,
    EntitlementUpdate = 0x10000003,
    GameCustomData = 0x10000004,
    DisplaySafeAreaUpdate = 0x10000005,
    UrlOpen = 0x10000006,
    LaunchApp = 0x10000007,
    AppLaunchLink = 0x10000008,
    AddcontentInstall = 0x10000009,
    ResetVrPosition = 0x1000000a,
    JoinEvent = 0x1000000b,
    PlaygoLocusUpdate = 0x1000000c,
    PlayTogetherHost = 0x1000000d,
    ServiceEntitlementUpdate = 0x1000000e,
    EyeToEyeDistanceUpdate = 0x1000000f,
    JoinMatchEvent = 0x10000010,
    PlayTogetherHostA = 0x10000011,
    WebBrowserClosed = 0x10000012,
    ControllerSettingsClosed = 0x10000013,
    JoinTeamOnTeamMatchEvent = 0x10000014,
    OpenShareMenu = 0x30000000,
}

/// `OrbisSystemServiceStatus`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisSystemServiceStatus {
    pub event_num: i32,
    pub is_system_ui_overlaid: bool,
    pub is_in_background_execution: bool,
    pub is_cpu_mode7_cpu_normal: bool,
    pub is_game_live_streaming_on_air: bool,
    pub is_out_of_vr_play_area: bool,
    pub reserved: [u8; 0],
}

/// `OrbisSystemServiceDisplaySafeAreaInfo`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbisSystemServiceDisplaySafeAreaInfo {
    pub ratio: f32,
    pub reserved: [u8; 128],
}

impl Default for OrbisSystemServiceDisplaySafeAreaInfo {
    fn default() -> Self {
        Self {
            ratio: 0.0,
            reserved: [0; 128],
        }
    }
}

/// The `url_open` event parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UrlOpenEvent {
    pub source: [u8; 1024],
    pub url: [u8; 4096],
}

/// The `launch_app` event parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LaunchAppEvent {
    pub size: u32,
    pub arg: [u8; 8188],
}

/// The `app_launch_link` event parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AppLaunchLinkEvent {
    pub size: u32,
    pub arg: [u8; 2020],
}

/// The `join_event` parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct JoinEvent {
    pub user_id: i32,
    pub event_id: [u8; 37],
    pub boot_argument: [u8; 7169],
}

/// The entitlement-update parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct EntitlementUpdateEvent {
    pub user_id: OrbisUserServiceUserId,
    pub np_service_label: u32,
    pub reserved: [u8; 8184],
}

/// `OrbisSystemServiceEvent`'s payload union.
#[repr(C)]
#[derive(Clone, Copy)]
pub union SystemServiceEventPayload {
    pub param: [u8; 8192],
    pub url_open: UrlOpenEvent,
    pub launch_app: LaunchAppEvent,
    pub app_launch_link: AppLaunchLinkEvent,
    pub join_event: JoinEvent,
    pub service_entitlement_update: EntitlementUpdateEvent,
    pub unified_entitlement_update: EntitlementUpdateEvent,
    pub reserved: [u8; 8192],
}

/// `OrbisSystemServiceEvent`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OrbisSystemServiceEvent {
    pub event_type: OrbisSystemServiceEventType,
    pub payload: SystemServiceEventPayload,
}

/// The system-service entry points; the HLE loader implements them.
pub trait SystemServiceApi {
    fn is_splash_visible(&self) -> bool;
    fn push_event(&mut self, event: &OrbisSystemServiceEvent);
    fn get_param(&mut self, id: OrbisSystemServiceParamId, value: &mut u32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_event_and_status_layouts_match_the_header() {
        assert_eq!(core::mem::size_of::<OrbisSystemServiceStatus>(), 12);
        assert_eq!(
            core::mem::size_of::<OrbisSystemServiceDisplaySafeAreaInfo>(),
            132
        );
        assert_eq!(core::mem::size_of::<SystemServiceEventPayload>(), 8192);
        assert_eq!(core::mem::size_of::<OrbisSystemServiceEvent>(), 8196);
        assert_eq!(core::mem::offset_of!(OrbisSystemServiceEvent, payload), 4);
    }

    #[test]
    fn the_event_values_match_the_header() {
        assert_eq!(OrbisSystemServiceEventType::Invalid as i32, -1);
        assert_eq!(
            OrbisSystemServiceEventType::SessionInvitation as i32,
            0x10000002
        );
        assert_eq!(
            OrbisSystemServiceEventType::OpenShareMenu as i32,
            0x30000000
        );
        assert_eq!(OrbisSystemServiceParamId::EnterButtonAssign as i32, 1000);
        assert_eq!(OrbisSystemParamGameParentalLevel::Level11 as i32, 11);
        assert_eq!(OrbisSystemParamEnterButtonAssign::Cross as i32, 1);
    }

    #[test]
    fn the_payload_union_can_hold_a_launch_event() {
        let payload = SystemServiceEventPayload {
            launch_app: LaunchAppEvent {
                size: 3,
                arg: [7; 8188],
            },
        };
        unsafe {
            assert_eq!(payload.launch_app.size, 3);
            assert_eq!(payload.launch_app.arg[0], 7);
        }
    }
}
