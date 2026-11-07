//! Log class names, ported from `legacy/gpu/shadps4/common/logging/classes.h`.
//!
//! `Common::Log::Class`.
#![allow(dead_code)]

pub const COMMON: &str = "Common";
pub const COMMON_FILESYSTEM: &str = "Common.Filesystem";
pub const COMMON_MEMORY: &str = "Common.Memory";
pub const CONFIG: &str = "Config";
pub const CORE: &str = "Core";
pub const CORE_DEVICES: &str = "Core.Devices";
pub const CORE_LINKER: &str = "Core.Linker";
pub const DEBUG: &str = "Debug";
pub const FRONTEND: &str = "Frontend";
pub const IPC: &str = "IPC";
pub const IM_GUI: &str = "ImGui";
pub const INPUT: &str = "Input";
pub const KERNEL: &str = "Kernel";
pub const KERNEL_EVENT: &str = "Kernel.Event";
pub const KERNEL_FS: &str = "Kernel.Fs";
pub const KERNEL_PTHREAD: &str = "Kernel.Pthread";
pub const KERNEL_SCE: &str = "Kernel.Sce";
pub const KERNEL_VMM: &str = "Kernel.Vmm";
pub const KEY_MANAGER: &str = "KeyManager";
pub const LIB: &str = "Lib";
pub const LIB_AJM: &str = "Lib.Ajm";
pub const LIB_APP_CONTENT: &str = "Lib.AppContent";
pub const LIB_AUDIO3D: &str = "Lib.Audio3d";
pub const LIB_AUDIO_IN: &str = "Lib.AudioIn";
pub const LIB_AUDIO_OUT: &str = "Lib.AudioOut";
pub const LIB_AV_PLAYER: &str = "Lib.AvPlayer";
pub const LIB_CAMERA: &str = "Lib.Camera";
pub const LIB_COMMON_DLG: &str = "Lib.CommonDlg";
pub const LIB_COMPANION_HTTPD: &str = "Lib.CompanionHttpd";
pub const LIB_COMPANION_UTIL: &str = "Lib.CompanionUtil";
pub const LIB_CONTENT_EXPORT: &str = "Lib.ContentExport";
pub const LIB_DISC_MAP: &str = "Lib.DiscMap";
pub const LIB_ERROR_DIALOG: &str = "Lib.ErrorDialog";
pub const LIB_FIBER: &str = "Lib.Fiber";
pub const LIB_FONT: &str = "Lib.Font";
pub const LIB_FONT_FT: &str = "Lib.FontFt";
pub const LIB_GAME_LIVE_STREAMING: &str = "Lib.GameLiveStreaming";
pub const LIB_GNM_DRIVER: &str = "Lib.GnmDriver";
pub const LIB_HMD: &str = "Lib.Hmd";
pub const LIB_HMD_SETUP_DIALOG: &str = "Lib.HmdSetupDialog";
pub const LIB_HTTP: &str = "Lib.Http";
pub const LIB_HTTP2: &str = "Lib.Http2";
pub const LIB_IME: &str = "Lib.Ime";
pub const LIB_IME_DIALOG: &str = "Lib.ImeDialog";
pub const LIB_INVITATION_DIALOG: &str = "Lib.InvitationDialog";
pub const LIB_JPEG: &str = "Lib.Jpeg";
pub const LIB_KERNEL: &str = "Lib.Kernel";
pub const LIB_LIBC_INTERNAL: &str = "Lib.LibcInternal";
pub const LIB_MOUSE: &str = "Lib.Mouse";
pub const LIB_MOVE: &str = "Lib.Move";
pub const LIB_MSG_DLG: &str = "Lib.MsgDlg";
pub const LIB_NET: &str = "Lib.Net";
pub const LIB_NET_CTL: &str = "Lib.NetCtl";
pub const LIB_NGS2: &str = "Lib.Ngs2";
pub const LIB_NP_AUTH: &str = "Lib.NpAuth";
pub const LIB_NP_COMMERCE: &str = "Lib.NpCommerce";
pub const LIB_NP_COMMON: &str = "Lib.NpCommon";
pub const LIB_NP_MANAGER: &str = "Lib.NpManager";
pub const LIB_NP_MATCHING2: &str = "Lib.NpMatching2";
pub const LIB_NP_SIGNALING: &str = "Lib.NpSignaling";
pub const LIB_NP_PARTNER: &str = "Lib.NpPartner";
pub const LIB_NP_PARTY: &str = "Lib.NpParty";
pub const LIB_NP_PROFILE_DIALOG: &str = "Lib.NpProfileDialog";
pub const LIB_NP_SCORE: &str = "Lib.NpScore";
pub const LIB_NP_SNS_FACEBOOK_DIALOG: &str = "Lib.NpSnsFacebookDialog";
pub const LIB_NP_TROPHY: &str = "Lib.NpTrophy";
pub const LIB_NP_TUS: &str = "Lib.NpTus";
pub const LIB_NP_UTILITY: &str = "Lib.NpUtility";
pub const LIB_NP_WEB_API: &str = "Lib.NpWebApi";
pub const LIB_NP_WEB_API2: &str = "Lib.NpWebApi2";
pub const LIB_PAD: &str = "Lib.Pad";
pub const LIB_PLAY_GO: &str = "Lib.PlayGo";
pub const LIB_PLAY_GO_DIALOG: &str = "Lib.PlayGoDialog";
pub const LIB_PNG: &str = "Lib.Png";
pub const LIB_RANDOM: &str = "Lib.Random";
pub const LIB_RAZOR_CPU: &str = "Lib.RazorCpu";
pub const LIB_REMOTEPLAY: &str = "Lib.Remoteplay";
pub const LIB_RTC: &str = "Lib.Rtc";
pub const LIB_RUDP: &str = "Lib.Rudp";
pub const LIB_SAVE_DATA: &str = "Lib.SaveData";
pub const LIB_SAVE_DATA_DIALOG: &str = "Lib.SaveDataDialog";
pub const LIB_SCREENSHOT: &str = "Lib.Screenshot";
pub const LIB_SHARE_PLAY: &str = "Lib.SharePlay";
pub const LIB_SIGNIN_DIALOG: &str = "Lib.SigninDialog";
pub const LIB_SSL: &str = "Lib.Ssl";
pub const LIB_SSL2: &str = "Lib.Ssl2";
pub const LIB_SYS_MODULE: &str = "Lib.SysModule";
pub const LIB_SYSTEM_GESTURE: &str = "Lib.SystemGesture";
pub const LIB_SYSTEM_SERVICE: &str = "Lib.SystemService";
pub const LIB_USBD: &str = "Lib.Usbd";
pub const LIB_USER_SERVICE: &str = "Lib.UserService";
pub const LIB_VDEC2: &str = "Lib.Vdec2";
pub const LIB_VDECSW: &str = "Lib.Vdecsw";
pub const LIB_VIDEO_OUT: &str = "Lib.VideoOut";
pub const LIB_VIDEODEC: &str = "Lib.Videodec";
pub const LIB_VIDEO_RECORDING: &str = "Lib.VideoRecording";
pub const LIB_VOICE: &str = "Lib.Voice";
pub const LIB_VR_TRACKER: &str = "Lib.VrTracker";
pub const LIB_WEB_BROWSER_DIALOG: &str = "Lib.WebBrowserDialog";
pub const LIB_ZLIB: &str = "Lib.Zlib";
pub const LOADER: &str = "Loader";
pub const LOG: &str = "Log";
pub const NP_HANDLER: &str = "NpHandler";
pub const RENDER: &str = "Render";
pub const RENDER_RECOMPILER: &str = "Render.Recompiler";
pub const RENDER_VULKAN: &str = "Render.Vulkan";
pub const SHAD_NET: &str = "ShadNet";
pub const TTY: &str = "Tty";

/// Every log class, in declaration order (`ALL_LOGGERS`).
pub const ALL_LOGGERS: &[&str] = &[
    COMMON,
    COMMON_FILESYSTEM,
    COMMON_MEMORY,
    CONFIG,
    CORE,
    CORE_DEVICES,
    CORE_LINKER,
    DEBUG,
    FRONTEND,
    IPC,
    IM_GUI,
    INPUT,
    KERNEL,
    KERNEL_EVENT,
    KERNEL_FS,
    KERNEL_PTHREAD,
    KERNEL_SCE,
    KERNEL_VMM,
    KEY_MANAGER,
    LIB,
    LIB_AJM,
    LIB_APP_CONTENT,
    LIB_AUDIO3D,
    LIB_AUDIO_IN,
    LIB_AUDIO_OUT,
    LIB_AV_PLAYER,
    LIB_CAMERA,
    LIB_COMMON_DLG,
    LIB_COMPANION_HTTPD,
    LIB_COMPANION_UTIL,
    LIB_CONTENT_EXPORT,
    LIB_DISC_MAP,
    LIB_ERROR_DIALOG,
    LIB_FIBER,
    LIB_FONT,
    LIB_FONT_FT,
    LIB_GAME_LIVE_STREAMING,
    LIB_GNM_DRIVER,
    LIB_HMD,
    LIB_HMD_SETUP_DIALOG,
    LIB_HTTP,
    LIB_HTTP2,
    LIB_IME,
    LIB_IME_DIALOG,
    LIB_INVITATION_DIALOG,
    LIB_JPEG,
    LIB_KERNEL,
    LIB_LIBC_INTERNAL,
    LIB_MOUSE,
    LIB_MOVE,
    LIB_MSG_DLG,
    LIB_NET,
    LIB_NET_CTL,
    LIB_NGS2,
    LIB_NP_AUTH,
    LIB_NP_COMMERCE,
    LIB_NP_COMMON,
    LIB_NP_MANAGER,
    LIB_NP_MATCHING2,
    LIB_NP_SIGNALING,
    LIB_NP_PARTNER,
    LIB_NP_PARTY,
    LIB_NP_PROFILE_DIALOG,
    LIB_NP_SCORE,
    LIB_NP_SNS_FACEBOOK_DIALOG,
    LIB_NP_TROPHY,
    LIB_NP_TUS,
    LIB_NP_UTILITY,
    LIB_NP_WEB_API,
    LIB_NP_WEB_API2,
    LIB_PAD,
    LIB_PLAY_GO,
    LIB_PLAY_GO_DIALOG,
    LIB_PNG,
    LIB_RANDOM,
    LIB_RAZOR_CPU,
    LIB_REMOTEPLAY,
    LIB_RTC,
    LIB_RUDP,
    LIB_SAVE_DATA,
    LIB_SAVE_DATA_DIALOG,
    LIB_SCREENSHOT,
    LIB_SHARE_PLAY,
    LIB_SIGNIN_DIALOG,
    LIB_SSL,
    LIB_SSL2,
    LIB_SYS_MODULE,
    LIB_SYSTEM_GESTURE,
    LIB_SYSTEM_SERVICE,
    LIB_USBD,
    LIB_USER_SERVICE,
    LIB_VDEC2,
    LIB_VDECSW,
    LIB_VIDEO_OUT,
    LIB_VIDEODEC,
    LIB_VIDEO_RECORDING,
    LIB_VOICE,
    LIB_VR_TRACKER,
    LIB_WEB_BROWSER_DIALOG,
    LIB_ZLIB,
    LOADER,
    LOG,
    NP_HANDLER,
    RENDER,
    RENDER_RECOMPILER,
    RENDER_VULKAN,
    SHAD_NET,
    TTY,
];
