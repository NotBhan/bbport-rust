//! Types shared by the NP libraries, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_types.h`.

/// `OrbisNpAccountId`.
pub type OrbisNpAccountId = u64;

/// `OrbisNpTitleId`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpTitleId {
    pub id: [u8; 13],
    pub padding: [u8; 3],
}

/// `OrbisNpTitleSecret`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbisNpTitleSecret {
    pub data: [u8; 128],
}

impl Default for OrbisNpTitleSecret {
    fn default() -> Self {
        Self { data: [0; 128] }
    }
}

pub const ORBIS_NP_ONLINEID_MAX_LENGTH: i32 = 16;

/// `OrbisNpOnlineId`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpOnlineId {
    pub data: [u8; ORBIS_NP_ONLINEID_MAX_LENGTH as usize],
    pub term: i8,
    pub dummy: [i8; 3],
}

/// `OrbisNpId`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpId {
    pub handle: OrbisNpOnlineId,
    pub opt: [u8; 8],
    pub reserved: [u8; 8],
}

/// `SetNpOnlineId`.
pub fn set_np_online_id(destination: &mut OrbisNpOnlineId, source: &str) {
    *destination = OrbisNpOnlineId::default();
    let length = source.len().min(ORBIS_NP_ONLINEID_MAX_LENGTH as usize);
    destination.data[..length].copy_from_slice(&source.as_bytes()[..length]);
    destination.term = 0;
}

/// `SetNpId`.
pub fn set_np_id(destination: &mut OrbisNpId, online_id: &str) {
    *destination = OrbisNpId::default();
    set_np_online_id(&mut destination.handle, online_id);
}

/// `OrbisNpClientId`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbisNpClientId {
    pub id: [u8; 129],
    pub padding: [u8; 7],
}

impl Default for OrbisNpClientId {
    fn default() -> Self {
        Self {
            id: [0; 129],
            padding: [0; 7],
        }
    }
}

/// `OrbisNpAuthorizationCode`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbisNpAuthorizationCode {
    pub code: [u8; 129],
    pub padding: [u8; 7],
}

impl Default for OrbisNpAuthorizationCode {
    fn default() -> Self {
        Self {
            code: [0; 129],
            padding: [0; 7],
        }
    }
}

/// `OrbisNpClientSecret`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbisNpClientSecret {
    pub secret: [u8; 257],
    pub padding: [u8; 7],
}

impl Default for OrbisNpClientSecret {
    fn default() -> Self {
        Self {
            secret: [0; 257],
            padding: [0; 7],
        }
    }
}

/// `OrbisNpIdToken`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbisNpIdToken {
    pub token: [u8; 4097],
    pub padding: [u8; 7],
}

impl Default for OrbisNpIdToken {
    fn default() -> Self {
        Self {
            token: [0; 4097],
            padding: [0; 7],
        }
    }
}

/// `OrbisNpServiceLabel`.
pub type OrbisNpServiceLabel = u32;
pub const ORBIS_NP_INVALID_SERVICE_LABEL: i32 = 0xFFFF_FFFFu32 as i32;

/// `OrbisNpPlatformType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum OrbisNpPlatformType {
    None = 0,
    Ps3 = 1,
    Vita = 2,
    Ps4 = 3,
}

/// `OrbisNpPeerAddressA`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbisNpPeerAddressA {
    pub account_id: OrbisNpAccountId,
    pub platform: OrbisNpPlatformType,
    pub padding: [u8; 4],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting_an_online_id_terminates_and_truncates() {
        let mut id = OrbisNpOnlineId::default();
        set_np_online_id(&mut id, "hunter");
        assert_eq!(&id.data[..6], b"hunter");
        assert_eq!(id.data[6], 0);
        assert_eq!(id.term, 0);

        set_np_online_id(&mut id, "0123456789abcdefghij");
        assert_eq!(&id.data, b"0123456789abcdef");
        assert_eq!(id.term, 0);
    }

    #[test]
    fn the_np_struct_sizes_match_the_layout() {
        assert_eq!(core::mem::size_of::<OrbisNpTitleId>(), 16);
        assert_eq!(core::mem::size_of::<OrbisNpOnlineId>(), 20);
        assert_eq!(core::mem::size_of::<OrbisNpId>(), 36);
        assert_eq!(core::mem::size_of::<OrbisNpPeerAddressA>(), 16);

        let mut np_id = OrbisNpId::default();
        set_np_id(&mut np_id, "hunter");
        assert_eq!(&np_id.handle.data[..6], b"hunter");
    }
}
