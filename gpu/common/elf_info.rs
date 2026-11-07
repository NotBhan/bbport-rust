//! Executable metadata, ported from `legacy/gpu/shadps4/common/elf_info.h`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// `Common::PSFAttributes`: the packed attribute word with the header's bit positions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PsfAttributes {
    pub raw: u32,
}

macro_rules! psf_bits {
    ($( $name:ident : $bit:expr ),* $(,)?) => {
        impl PsfAttributes {
            $(
                #[doc = concat!("`", stringify!($name), "`.")]
                pub fn $name(&self) -> bool {
                    self.bit($bit)
                }
            )*
        }
    };
}

psf_bits! {
    support_initial_user_logout: 0,
    enter_button_cross: 1,
    ps_move_warning: 2,
    support_stereoscopic_3d: 3,
    ps_button_suspend: 4,
    enter_button_system: 5,
    override_share_menu: 6,
    special_res_ps_button_suspend: 8,
    enable_hdcp: 9,
    disable_hdcp_non_game: 10,
    support_ps_vr: 14,
    six_cpu_mode: 15,
    seven_cpu_mode: 16,
    support_neo_mode: 23,
    require_ps_vr: 26,
    support_hdr: 29,
    display_location: 31,
}

impl PsfAttributes {
    /// Reads a bit by its header offset.
    pub fn bit(&self, bit: u32) -> bool {
        (self.raw >> bit) & 1 != 0
    }

    /// Writes a bit by its header offset.
    pub fn set_bit(&mut self, bit: u32, value: bool) {
        if value {
            self.raw |= 1 << bit;
        } else {
            self.raw &= !(1 << bit);
        }
    }
}

/// `Common::ElfInfo`.
#[derive(Debug, Clone, Default)]
pub struct ElfInfo {
    initialized: bool,
    pub(crate) game_serial: String,
    pub(crate) title: String,
    pub(crate) app_ver: String,
    pub(crate) firmware_ver: u32,
    pub(crate) raw_firmware_ver: u32,
    pub(crate) sdk_ver: u32,
    pub(crate) psf_attributes: PsfAttributes,
    pub(crate) splash_data: Vec<u8>,
    pub(crate) game_folder: PathBuf,
    pub(crate) np_comm_ids: Vec<String>,
    pub(crate) trophy_index_map: BTreeMap<i32, String>,
}

impl ElfInfo {
    pub const FW_100: u32 = 0x1000000;
    pub const FW_150: u32 = 0x1500000;
    pub const FW_160: u32 = 0x1600000;
    pub const FW_170: u32 = 0x1700000;
    pub const FW_200: u32 = 0x2000000;
    pub const FW_250: u32 = 0x2500000;
    pub const FW_300: u32 = 0x3000000;
    pub const FW_350: u32 = 0x3500000;
    pub const FW_400: u32 = 0x4000000;
    pub const FW_450: u32 = 0x4500000;
    pub const FW_500: u32 = 0x5000000;
    pub const FW_550: u32 = 0x5500000;
    pub const FW_600: u32 = 0x6000000;
    pub const FW_700: u32 = 0x7000000;
    pub const FW_750: u32 = 0x7500000;
    pub const FW_800: u32 = 0x8000000;
    pub const FW_900: u32 = 0x9000000;
    pub const FW_1000: u32 = 0x10000000;
    pub const FW_1150: u32 = 0x11500000;

    /// `ElfInfo::Instance()`.
    pub fn instance() -> &'static Mutex<ElfInfo> {
        static INSTANCE: OnceLock<Mutex<ElfInfo>> = OnceLock::new();
        INSTANCE.get_or_init(|| Mutex::new(ElfInfo::default()))
    }

    /// Marks the metadata as loaded; the loader sets the fields before calling this.
    pub fn mark_initialized(&mut self) {
        self.initialized = true;
    }

    /// Whether the metadata was loaded.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// `GameSerial`.
    pub fn game_serial(&self) -> &str {
        assert!(self.initialized);
        &self.game_serial
    }

    /// `Title`.
    pub fn title(&self) -> &str {
        assert!(self.initialized);
        &self.title
    }

    /// `AppVer`.
    pub fn app_ver(&self) -> &str {
        assert!(self.initialized);
        &self.app_ver
    }

    /// `FirmwareVer`.
    pub fn firmware_ver(&self) -> u32 {
        assert!(self.initialized);
        self.firmware_ver
    }

    /// `RawFirmwareVer`.
    pub fn raw_firmware_ver(&self) -> u32 {
        assert!(self.initialized);
        self.raw_firmware_ver
    }

    /// `CompiledSdkVer`.
    pub fn compiled_sdk_ver(&self) -> u32 {
        assert!(self.initialized);
        self.sdk_ver
    }

    /// `GetPSFAttributes`.
    pub fn psf_attributes(&self) -> PsfAttributes {
        assert!(self.initialized);
        self.psf_attributes
    }

    /// `GetSplashData`.
    pub fn splash_data(&self) -> &[u8] {
        &self.splash_data
    }

    /// `GetGameFolder`.
    pub fn game_folder(&self) -> &PathBuf {
        &self.game_folder
    }

    /// `GetTrophyIndexMap`.
    pub fn trophy_index_map(&self) -> &BTreeMap<i32, String> {
        &self.trophy_index_map
    }

    /// `GetNpCommIds`.
    pub fn np_comm_ids(&self) -> &[String] {
        &self.np_comm_ids
    }

    /// `SetNpCommIds`.
    pub fn set_np_comm_ids(&mut self, ids: Vec<String>) {
        self.np_comm_ids = ids;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_psf_attributes_pack_into_four_bytes() {
        assert_eq!(core::mem::size_of::<PsfAttributes>(), 4);
        let mut attributes = PsfAttributes::default();
        attributes.set_bit(0, true);
        attributes.set_bit(31, true);
        assert!(attributes.support_initial_user_logout());
        assert!(attributes.display_location());
        assert!(!attributes.enter_button_cross());
        assert_eq!(attributes.raw, 0x8000_0001);
        attributes.set_bit(0, false);
        assert!(!attributes.support_initial_user_logout());
    }

    #[test]
    fn the_firmware_constants_match_the_header() {
        assert_eq!(ElfInfo::FW_100, 0x1000000);
        assert_eq!(ElfInfo::FW_1150, 0x11500000);
    }

    #[test]
    fn the_getters_expose_the_loaded_metadata() {
        let mut info = ElfInfo {
            game_serial: "CUSA00900".to_string(),
            title: "Bloodborne".to_string(),
            app_ver: "01.09".to_string(),
            firmware_ver: ElfInfo::FW_900,
            np_comm_ids: vec!["npcomm".to_string()],
            ..Default::default()
        };
        assert!(!info.is_initialized());
        info.mark_initialized();
        assert!(info.is_initialized());
        assert_eq!(info.game_serial(), "CUSA00900");
        assert_eq!(info.title(), "Bloodborne");
        assert_eq!(info.app_ver(), "01.09");
        assert_eq!(info.firmware_ver(), ElfInfo::FW_900);
        assert_eq!(info.np_comm_ids(), ["npcomm"]);
    }
}
