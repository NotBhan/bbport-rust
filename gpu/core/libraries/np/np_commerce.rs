//! NP commerce dialog, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_commerce/np_commerce.{h,cpp}`.

pub const MAX_LABEL_LEN: u64 = 128;
pub const ORBIS_NP_COMMERCE_DIALOG_NUM_TARGETS_MAX: u32 = 10;
pub const ORBIS_COMMON_DIALOG_RESULT_OK: i32 = 0;
pub const ORBIS_COMMON_DIALOG_RESULT_USER_CANCELED: i32 = 1;
pub const ORBIS_NP_COMMERCE_DIALOG_RESULT_PURCHASED: i32 = 2;
pub const ORBIS_NP_COMMERCE_DIALOG_MODE_CATEGORY: i32 = 0;
pub const ORBIS_NP_COMMERCE_DIALOG_MODE_PRODUCT: i32 = 1;
pub const ORBIS_NP_COMMERCE_DIALOG_MODE_PRODUCT_CODE: i32 = 2;
pub const ORBIS_NP_COMMERCE_DIALOG_MODE_CHECKOUT: i32 = 3;
pub const ORBIS_NP_COMMERCE_DIALOG_MODE_DOWNLOADLIST: i32 = 4;
pub const ORBIS_NP_COMMERCE_DIALOG_MODE_PLUS: i32 = 5;

/// `CommonDialog::BaseParam`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BaseParam {
    pub size: u32,
    pub mode: u32,
    pub user_data: *mut core::ffi::c_void,
}

/// `OrbisNpCommerceDialogParam`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OrbisNpCommerceDialogParam {
    pub base_param: BaseParam,
    pub size: i32,
    pub user_id: i32,
    pub mode: i32,
    pub service_label: u32,
    pub targets: *const *const u8,
    pub num_targets: u32,
    pub features: u64,
    pub user_data: *mut core::ffi::c_void,
    pub reserved: [u8; 32],
    pub reserved2: [u8; 32],
}

/// `OrbisNpCommerceDialogResult`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpCommerceDialogResult {
    pub result: i32,
    pub authorized: bool,
    pub user_data: *mut core::ffi::c_void,
    pub reserved: [u8; 32],
}

/// `OrbisNpCommerceDialogMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum CommerceMode {
    Category = 0,
    Product = 1,
    ProductCode = 2,
    Checkout = 3,
    DownloadList = 4,
    Plus = 5,
}

/// `CommerceDialogState`.
#[derive(Debug, Default)]
pub struct CommerceDialogState {
    pub targets: Vec<String>,
    pub username: String,
    pub mode: i32,
    pub user_id: i32,
    pub service_label: u32,
    pub features: u64,
    pub user_data: usize,
    pub finished: bool,
    pub result: i32,
}

impl CommerceDialogState {
    /// `Finish`.
    pub fn finish(&mut self, result: i32) {
        self.finished = true;
        self.result = result;
    }

    /// `sceNpCommerceDialogGetResult`.
    pub fn get_result(&self) -> OrbisNpCommerceDialogResult {
        OrbisNpCommerceDialogResult {
            result: self.result,
            authorized: self.result == ORBIS_NP_COMMERCE_DIALOG_RESULT_PURCHASED,
            user_data: self.user_data as *mut core::ffi::c_void,
            reserved: [0; 32],
        }
    }

    pub fn open(&mut self, param: &OrbisNpCommerceDialogParam) -> i32 {
        if param.num_targets > ORBIS_NP_COMMERCE_DIALOG_NUM_TARGETS_MAX {
            return -1;
        }
        self.mode = param.mode;
        self.user_id = param.user_id;
        self.service_label = param.service_label;
        self.features = param.features;
        self.user_data = param.user_data as usize;
        self.targets.clear();
        for index in 0..param.num_targets as usize {
            if param.targets.is_null() {
                break;
            }
            // SAFETY: the caller provides `num_targets` C strings.
            let target = unsafe { *param.targets.add(index) };
            if !target.is_null() {
                self.targets.push(
                    unsafe { std::ffi::CStr::from_ptr(target as *const i8) }
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        self.finished = false;
        0
    }
}

/// `PsStoreIconPos` / `PsStoreIconLayout`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum PsStoreIconPos {
    Center = 0,
    Left = 1,
    Right = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum PsStoreIconLayout {
    Default = 0,
    FollowSafeArea = 1,
    Fixed90 = 2,
}

/// `PsStoreIconLayer`.
#[derive(Debug, Default)]
pub struct PsStoreIconLayer {
    pub shown: bool,
    pub pos: i32,
    pub layout: i32,
}

impl PsStoreIconLayer {
    pub fn show(&mut self, pos: i32) {
        self.shown = true;
        self.pos = pos;
    }

    pub fn hide(&mut self) {
        self.shown = false;
    }

    pub fn set_layout(&mut self, layout: i32) {
        self.layout = layout;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commerce_dialogs_open_finish_and_report_results() {
        let mut state = CommerceDialogState::default();
        let target = std::ffi::CString::new("sku").unwrap();
        let targets = [target.as_ptr()];
        let param = OrbisNpCommerceDialogParam {
            mode: ORBIS_NP_COMMERCE_DIALOG_MODE_PRODUCT,
            targets: targets.as_ptr() as *const *const u8,
            num_targets: 1,
            ..OrbisNpCommerceDialogParam::default()
        };
        assert_eq!(state.open(&param), 0);
        assert_eq!(state.targets, vec!["sku"]);
        state.finish(ORBIS_NP_COMMERCE_DIALOG_RESULT_PURCHASED);
        assert!(state.get_result().authorized);
        let mut icon = PsStoreIconLayer::default();
        icon.show(PsStoreIconPos::Left as i32);
        assert!(icon.shown);
        assert_eq!(core::mem::size_of::<OrbisNpCommerceDialogParam>(), 0x80);
    }
}
