//! shadNet protocol types, ported from `bbnet/shadnet/client.h`.

pub const SHAD_HEADER_SIZE: u32 = 15;
pub const SHAD_CONNECT_TIMEOUT_MS: u32 = 10000;
pub const SHAD_CONNECT_MAX_ATTEMPTS: u32 = 4;
pub const SHAD_CONNECT_RETRY_BACKOFF_MS: u32 = 1000;
pub const SHAD_PROTOCOL_VERSION: u32 = 1;
pub const SHAD_MAX_PACKET_SIZE: u32 = 0x0080_0000;

/// `PacketType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketType {
    Request = 0,
    Reply = 1,
    Notification = 2,
    ServerInfo = 3,
}

/// `CommandType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum CommandType {
    Login = 0,
    Terminate = 1,
    Create = 2,
    Delete = 3,
    SendToken = 4,
    SendResetToken = 5,
    ResetPassword = 6,
    ResetState = 7,
    AddFriend = 8,
    RemoveFriend = 9,
    AddBlock = 10,
    RemoveBlock = 11,
    GetServerFeatures = 12,
    SetClientVersion = 13,
    GetBoardInfos = 30,
    RecordScore = 31,
    RecordScoreData = 32,
    GetScoreData = 33,
    GetScoreRange = 34,
    GetScoreFriends = 35,
    GetScoreNpid = 36,
    GetScoreAccountId = 37,
    GetScoreGameDataByAccId = 38,
    GetToken = 39,
    SetAppearOffline = 40,
    LookupOnlineId = 41,
    ContextStart = 100,
    CreateRoom = 101,
    JoinRoom = 102,
    LeaveRoom = 103,
    SearchRoom = 104,
    RequestSignalingInfos = 105,
    ContextStop = 106,
    SetUserInfo = 107,
    SetRoomDataInternal = 108,
    SetRoomDataExternal = 109,
    KickoutRoomMember = 110,
    GetWorldInfoList = 111,
    GetRoomDataExternalList = 112,
    GetUserInfoList = 113,
    GetRoomMemberDataExternalList = 114,
    SendRoomMessage = 115,
    SetRoomMemberDataInternal = 116,
    TusSetData = 201,
    TusGetData = 202,
    TusSetMultiSlotVariable = 203,
    TusGetMultiSlotVariable = 204,
    TusAddAndGetVariable = 205,
    TusGetMultiSlotDataStatus = 206,
    TusGetMultiUserDataStatus = 207,
    TusGetFriendsDataStatus = 208,
    TusDeleteMultiSlotData = 209,
    TusGetMultiUserVariable = 210,
    TusTryAndSetVariable = 211,
    TusGetFriendsVariable = 212,
    TusDeleteMultiSlotVariable = 213,
    TssGetData = 214,
    UnlockTrophy = 301,
    SyncTrophies = 302,
}

/// `NotificationType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum NotificationType {
    FriendQuery = 5,
    FriendNew = 6,
    FriendLost = 7,
    FriendStatus = 8,
    RoomEvent = 10,
    RoomMessage = 11,
    WebApiPushEvent = 17,
    ServiceUnavailable = 18,
    ClientRejected = 19,
}

/// `ErrorType`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum ErrorType {
    NoError = 0,
    #[default]
    Malformed = 1,
    Invalid = 2,
    InvalidInput = 3,
    TooSoon = 4,
    LoginError = 5,
    LoginAlreadyLoggedIn = 6,
    LoginInvalidUsername = 7,
    LoginInvalidPassword = 8,
    LoginInvalidToken = 9,
    CreationError = 10,
    CreationExistingUsername = 11,
    CreationBannedEmailProvider = 12,
    CreationExistingEmail = 13,
    RoomMissing = 14,
    RoomAlreadyJoined = 15,
    RoomFull = 16,
    RoomPasswordMismatch = 17,
    RoomPasswordMissing = 18,
    RoomGroupNoJoinLabel = 19,
    RoomGroupFull = 20,
    RoomGroupJoinLabelNotFound = 21,
    RoomGroupMaxSlotMismatch = 22,
    Unauthorized = 23,
    DbFail = 24,
    EmailFail = 25,
    NotFound = 26,
    Blocked = 27,
    AlreadyFriend = 28,
    ScoreNotBest = 29,
    ScoreInvalid = 30,
    ScoreHasData = 31,
    CondFail = 32,
    Unsupported = 33,
    TooLarge = 34,
}

/// `ShadNetState`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ShadNetState {
    #[default]
    Ok,
    FailureInput,
    FailureResolve,
    FailureConnect,
    FailureServerInfo,
    FailureAuth,
    FailureAlreadyIn,
    FailureUsername,
    FailurePassword,
    FailureToken,
    FailureProtocol,
    FailureOther,
}

/// `FriendEntry`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FriendEntry {
    pub npid: String,
    pub online: bool,
}

/// `LoginResult`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoginResult {
    pub error: ErrorType,
    pub avatar_url: String,
    pub user_id: u64,
    pub friends: Vec<FriendEntry>,
    pub requests_sent: Vec<String>,
    pub requests_received: Vec<String>,
    pub blocked: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NotifyFriendQuery {
    pub from_npid: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NotifyFriendNew {
    pub npid: String,
    pub online: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NotifyFriendLost {
    pub npid: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NotifyFriendStatus {
    pub npid: String,
    pub online: bool,
    pub timestamp: u64,
}

/// `NotifyServiceUnavailable`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NotifyServiceUnavailable {
    pub service: u32,
    pub reason: u32,
    pub in_use: u32,
    pub limit: u32,
}

impl NotifyServiceUnavailable {
    pub const SERVICE_MATCHING2: u32 = 1;
    pub const REASON_SERVER_FULL: u32 = 1;
    pub const REASON_NOT_ALLOWED: u32 = 2;
    pub const REASON_DISABLED: u32 = 3;
}

/// `NotifyClientRejected`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NotifyClientRejected {
    pub reason: u32,
    pub message: String,
    pub version: String,
}

impl NotifyClientRejected {
    pub const REASON_VERSION: u32 = 1;
}

/// `NotifyWebApiPushEvent`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NotifyWebApiPushEvent {
    pub np_service_name: String,
    pub np_service_label: u32,
    pub data_type: String,
    pub data: String,
    pub from_npid: String,
    pub to_npid: String,
    pub extd_data: Vec<(String, String)>,
    pub from_account_id: u64,
    pub to_account_id: u64,
}

/// `MatchingBinAttr`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MatchingBinAttr {
    pub attr_id: u32,
    pub update_date: u64,
    pub update_member_id: u32,
    pub data: Vec<u8>,
}

/// `NotifyRoomEvent`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NotifyRoomEvent {
    pub ctx_id: u32,
    pub room_id: u64,
    pub event: u32,
    pub event_cause: u32,
    pub error_code: i32,
    pub flags: u32,
    pub has_passwd_mask: bool,
    pub passwd_slot_mask: u64,
    pub member_npid: String,
    pub member_account_id: u64,
    pub member_platform: u32,
    pub member_id: u32,
    pub member_team_id: u32,
    pub member_is_owner: bool,
    pub member_join_date: u64,
    pub member_nat_type: u32,
    pub member_flag_attr: u32,
    pub member_group_id: u32,
    pub member_addr: String,
    pub member_port: u32,
    pub member_bin_attrs: Vec<MatchingBinAttr>,
    pub bin_attrs: Vec<MatchingBinAttr>,
}

/// `NotifyRoomMessage`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NotifyRoomMessage {
    pub ctx_id: u32,
    pub room_id: u64,
    pub src_member_id: u32,
    pub event: u32,
    pub cast_type: u32,
    pub dst_member_ids: Vec<u32>,
    pub src_npid: String,
    pub src_account_id: u64,
    pub src_platform: u32,
    pub msg: Vec<u8>,
}

/// The client surface used by the NP libraries.
pub trait ShadNetClientApi {
    fn start(&mut self, host: &str, port: u16, npid: &str, password: &str, token: &str);
    fn stop(&mut self);
    fn wait_for_connection(&mut self) -> ShadNetState;
    fn wait_for_authenticated(&mut self) -> ShadNetState;
    fn is_connected(&self) -> bool;
    fn is_authenticated(&self) -> bool;
    fn state(&self) -> ShadNetState;
    fn server_protocol_version(&self) -> u32;
    fn submit_request(&mut self, command: CommandType, payload: &[u8]) -> u64;
    fn add_friend(&mut self, npid: &str) -> u64;
    fn remove_friend(&mut self, npid: &str) -> u64;
    fn add_block(&mut self, npid: &str) -> u64;
    fn remove_block(&mut self, npid: &str) -> u64;
    fn set_appear_offline(&mut self, enable: bool) -> u64;
    fn lookup_online_id(&mut self, npid: &str) -> u64;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_protocol_constants_match_the_header() {
        assert_eq!(SHAD_HEADER_SIZE, 15);
        assert_eq!(SHAD_PROTOCOL_VERSION, 1);
        assert_eq!(SHAD_MAX_PACKET_SIZE, 8 * 1024 * 1024);
        assert_eq!(PacketType::ServerInfo as u8, 3);
        assert_eq!(CommandType::SetClientVersion as u16, 13);
        assert_eq!(CommandType::TssGetData as u16, 214);
        assert_eq!(CommandType::UnlockTrophy as u16, 301);
        assert_eq!(NotificationType::WebApiPushEvent as u16, 17);
        assert_eq!(ErrorType::TooLarge as u8, 34);
        assert_eq!(ShadNetState::default(), ShadNetState::Ok);
    }

    #[test]
    fn callback_data_defaults_are_empty() {
        let login = LoginResult::default();
        assert_eq!(login.error, ErrorType::Malformed);
        assert!(login.friends.is_empty());
        let unavailable = NotifyServiceUnavailable::default();
        assert_eq!(unavailable.service, 0);
        assert_eq!(NotifyServiceUnavailable::SERVICE_MATCHING2, 1);
        assert_eq!(NotifyClientRejected::REASON_VERSION, 1);
    }
}

/// The shadNet client runtime state (`client.cpp`).
#[derive(Debug, Default)]
pub struct ShadNetClientState {
    pub host: String,
    pub port: u16,
    pub connected: bool,
    pub authenticated: bool,
    pub protocol_version: u32,
    pub packet_counter: u64,
    pub send_queue: Vec<Vec<u8>>,
    pub server_features: Vec<u32>,
}

impl ShadNetClientState {
    pub fn start(&mut self, host: &str, port: u16) -> bool {
        if host.is_empty() || port == 0 {
            return false;
        }
        self.host = host.to_string();
        self.port = port;
        self.connected = true;
        self.packet_counter = 1;
        true
    }

    pub fn stop(&mut self) {
        self.connected = false;
        self.authenticated = false;
        self.send_queue.clear();
    }

    pub fn submit(&mut self, packet: Vec<u8>) -> Option<u64> {
        if !self.connected {
            return None;
        }
        let id = self.packet_counter;
        self.packet_counter += 1;
        self.send_queue.push(packet);
        Some(id)
    }

    pub fn on_server_info(&mut self, version: u32) {
        self.protocol_version = version;
        self.authenticated = version == SHAD_PROTOCOL_VERSION;
    }

    pub fn flush(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.send_queue)
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn client_connects_submits_and_authenticates() {
        let mut client = ShadNetClientState::default();
        assert!(!client.start("", 1));
        assert!(client.start("host", 31313));
        assert_eq!(client.submit(vec![1]), Some(1));
        assert_eq!(client.flush().len(), 1);
        client.on_server_info(SHAD_PROTOCOL_VERSION);
        assert!(client.authenticated);
        client.stop();
        assert!(client.submit(vec![2]).is_none());
    }
}
