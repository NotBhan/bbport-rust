//! NP trophy types and flag helpers, ported from
//! `bbnet/vendor/shadps4/core/libraries/np/np_trophy.h`.

use crate::gpu::bbnet::include::core::libraries::rtc::rtc::OrbisRtcTick;

pub const ORBIS_NP_TROPHY_FLAG_SETSIZE: i32 = 128;
pub const ORBIS_NP_TROPHY_FLAG_BITS_SHIFT: i32 = 5;
pub const ORBIS_NP_TROPHY_GAME_TITLE_MAX_SIZE: usize = 128;
pub const ORBIS_NP_TROPHY_GAME_DESCR_MAX_SIZE: usize = 1024;
pub const ORBIS_NP_TROPHY_GROUP_TITLE_MAX_SIZE: usize = 128;
pub const ORBIS_NP_TROPHY_GROUP_DESCR_MAX_SIZE: usize = 1024;
pub const ORBIS_NP_TROPHY_NAME_MAX_SIZE: usize = 128;
pub const ORBIS_NP_TROPHY_DESCR_MAX_SIZE: usize = 1024;
pub const ORBIS_NP_TROPHY_NUM_MAX: i32 = 128;
pub const ORBIS_NP_TROPHY_INVALID_HANDLE: i32 = -1;
pub const ORBIS_NP_TROPHY_INVALID_CONTEXT: i32 = -1;
pub const ORBIS_NP_TROPHY_INVALID_TROPHY_ID: i32 = -1;

pub type OrbisNpTrophyHandle = i32;
pub type OrbisNpTrophyContext = i32;
pub type OrbisNpTrophyId = i32;
pub type OrbisNpTrophyFlagMask = u32;

/// The number of flag masks.
pub const ORBIS_NP_TROPHY_NUM_MASKS: usize =
    (ORBIS_NP_TROPHY_FLAG_SETSIZE >> ORBIS_NP_TROPHY_FLAG_BITS_SHIFT) as usize;

/// `OrbisNpTrophyFlagArray`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpTrophyFlagArray {
    pub flag_bits: [OrbisNpTrophyFlagMask; ORBIS_NP_TROPHY_NUM_MASKS],
}

/// `ORBIS_NP_TROPHY_FLAG_ZERO`.
pub fn flag_zero(flags: &mut OrbisNpTrophyFlagArray) {
    flags.flag_bits = [0; ORBIS_NP_TROPHY_NUM_MASKS];
}

/// `ORBIS_NP_TROPHY_FLAG_SET`.
pub fn flag_set(trophy_id: i32, flags: &mut OrbisNpTrophyFlagArray) {
    if !(0..ORBIS_NP_TROPHY_FLAG_SETSIZE).contains(&trophy_id) {
        return;
    }
    let mask = (trophy_id >> ORBIS_NP_TROPHY_FLAG_BITS_SHIFT) as usize;
    let bit = (trophy_id & ((1 << ORBIS_NP_TROPHY_FLAG_BITS_SHIFT) - 1)) as u32;
    flags.flag_bits[mask] |= 1 << bit;
}

/// `ORBIS_NP_TROPHY_FLAG_SET_ALL`.
pub fn flag_set_all(flags: &mut OrbisNpTrophyFlagArray) {
    flags.flag_bits = [u32::MAX; ORBIS_NP_TROPHY_NUM_MASKS];
}

/// `ORBIS_NP_TROPHY_FLAG_CLR`.
pub fn flag_clr(trophy_id: i32, flags: &mut OrbisNpTrophyFlagArray) {
    if !(0..ORBIS_NP_TROPHY_FLAG_SETSIZE).contains(&trophy_id) {
        return;
    }
    let mask = (trophy_id >> ORBIS_NP_TROPHY_FLAG_BITS_SHIFT) as usize;
    let bit = (trophy_id & ((1 << ORBIS_NP_TROPHY_FLAG_BITS_SHIFT) - 1)) as u32;
    flags.flag_bits[mask] &= !(1 << bit);
}

/// `ORBIS_NP_TROPHY_FLAG_ISSET`.
pub fn flag_isset(trophy_id: i32, flags: &OrbisNpTrophyFlagArray) -> bool {
    if !(0..ORBIS_NP_TROPHY_FLAG_SETSIZE).contains(&trophy_id) {
        return false;
    }
    let mask = (trophy_id >> ORBIS_NP_TROPHY_FLAG_BITS_SHIFT) as usize;
    let bit = (trophy_id & ((1 << ORBIS_NP_TROPHY_FLAG_BITS_SHIFT) - 1)) as u32;
    flags.flag_bits[mask] & (1 << bit) != 0
}

/// `OrbisNpTrophyData`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpTrophyData {
    pub size: usize,
    pub trophy_id: OrbisNpTrophyId,
    pub unlocked: bool,
    pub reserved: [u8; 3],
    pub timestamp: OrbisRtcTick,
}

/// `OrbisNpTrophyGrade`.
pub type OrbisNpTrophyGrade = i32;
pub const ORBIS_NP_TROPHY_GRADE_UNKNOWN: i32 = 0;
pub const ORBIS_NP_TROPHY_GRADE_PLATINUM: i32 = 1;
pub const ORBIS_NP_TROPHY_GRADE_GOLD: i32 = 2;
pub const ORBIS_NP_TROPHY_GRADE_SILVER: i32 = 3;
pub const ORBIS_NP_TROPHY_GRADE_BRONZE: i32 = 4;

/// `OrbisNpTrophyGroupId`.
pub type OrbisNpTrophyGroupId = i32;
pub const ORBIS_NP_TROPHY_BASE_GAME_GROUP_ID: i32 = -1;
pub const ORBIS_NP_TROPHY_INVALID_GROUP_ID: i32 = -2;

/// `OrbisNpTrophyDetails`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbisNpTrophyDetails {
    pub size: usize,
    pub trophy_id: OrbisNpTrophyId,
    pub trophy_grade: OrbisNpTrophyGrade,
    pub group_id: OrbisNpTrophyGroupId,
    pub hidden: bool,
    pub reserved: [u8; 3],
    pub name: [u8; ORBIS_NP_TROPHY_NAME_MAX_SIZE],
    pub description: [u8; ORBIS_NP_TROPHY_DESCR_MAX_SIZE],
}

/// `OrbisNpTrophyGameData`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpTrophyGameData {
    pub size: usize,
    pub unlocked_trophies: u32,
    pub unlocked_platinum: u32,
    pub unlocked_gold: u32,
    pub unlocked_silver: u32,
    pub unlocked_bronze: u32,
    pub progress_percentage: u32,
}

/// `OrbisNpTrophyGameDetails`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbisNpTrophyGameDetails {
    pub size: usize,
    pub num_groups: u32,
    pub num_trophies: u32,
    pub num_platinum: u32,
    pub num_gold: u32,
    pub num_silver: u32,
    pub num_bronze: u32,
    pub title: [u8; ORBIS_NP_TROPHY_GAME_TITLE_MAX_SIZE],
    pub description: [u8; ORBIS_NP_TROPHY_GAME_DESCR_MAX_SIZE],
}

/// `OrbisNpTrophyGroupData`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrbisNpTrophyGroupData {
    pub size: usize,
    pub group_id: OrbisNpTrophyGroupId,
    pub unlocked_trophies: u32,
    pub unlocked_platinum: u32,
    pub unlocked_gold: u32,
    pub unlocked_silver: u32,
    pub unlocked_bronze: u32,
    pub progress_percentage: u32,
    pub reserved: [u8; 4],
}

/// `OrbisNpTrophyGroupDetails`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbisNpTrophyGroupDetails {
    pub size: usize,
    pub group_id: OrbisNpTrophyGroupId,
    pub num_trophies: u32,
    pub num_platinum: u32,
    pub num_gold: u32,
    pub num_silver: u32,
    pub num_bronze: u32,
    pub title: [u8; ORBIS_NP_TROPHY_GROUP_TITLE_MAX_SIZE],
    pub description: [u8; ORBIS_NP_TROPHY_GROUP_DESCR_MAX_SIZE],
}

/// The trophy entry points; the HLE loader implements them.
pub trait NpTrophyApi {
    fn create_context(
        &mut self,
        context: &mut OrbisNpTrophyContext,
        user_id: i32,
        service_label: u32,
        options: u64,
    ) -> i32;
    fn create_handle(&mut self, handle: &mut OrbisNpTrophyHandle) -> i32;
    fn destroy_context(&mut self, context: OrbisNpTrophyContext) -> i32;
    fn destroy_handle(&mut self, handle: OrbisNpTrophyHandle) -> i32;
    fn unlock_trophy(
        &mut self,
        context: OrbisNpTrophyContext,
        handle: OrbisNpTrophyHandle,
        trophy_id: OrbisNpTrophyId,
        platinum_id: &mut OrbisNpTrophyId,
    ) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flag_helpers_set_clear_and_query() {
        let mut flags = OrbisNpTrophyFlagArray::default();
        flag_zero(&mut flags);
        assert!(!flag_isset(0, &flags));
        flag_set(0, &mut flags);
        flag_set(31, &mut flags);
        flag_set(32, &mut flags);
        flag_set(127, &mut flags);
        assert!(flag_isset(0, &flags));
        assert!(flag_isset(31, &flags));
        assert!(flag_isset(32, &flags));
        assert!(flag_isset(127, &flags));
        assert!(!flag_isset(1, &flags));
        flag_clr(31, &mut flags);
        assert!(!flag_isset(31, &flags));
        flag_set_all(&mut flags);
        assert!(flag_isset(5, &flags));
        assert!(!flag_isset(-1, &flags));
        assert!(!flag_isset(128, &flags));
    }

    #[test]
    fn the_trophy_structs_match_the_header() {
        assert_eq!(core::mem::size_of::<OrbisNpTrophyFlagArray>(), 16);
        assert_eq!(core::mem::size_of::<OrbisNpTrophyData>(), 24);
        assert_eq!(core::mem::size_of::<OrbisNpTrophyDetails>(), 1176);
        assert_eq!(core::mem::size_of::<OrbisNpTrophyGameData>(), 32);
        assert_eq!(core::mem::size_of::<OrbisNpTrophyGameDetails>(), 1184);
        assert_eq!(core::mem::size_of::<OrbisNpTrophyGroupData>(), 40);
        assert_eq!(core::mem::size_of::<OrbisNpTrophyGroupDetails>(), 1184);
        assert_eq!(ORBIS_NP_TROPHY_INVALID_HANDLE, -1);
    }
}

/// Trophy context/handle allocation implemented by `np_trophy.cpp`.
#[derive(Debug, Default)]
pub struct TrophyState {
    pub next_context: i32,
    pub next_handle: i32,
    pub contexts: std::collections::HashSet<i32>,
    pub handles: std::collections::HashSet<i32>,
}

impl TrophyState {
    pub fn create_context(&mut self) -> i32 {
        self.next_context += 1;
        self.contexts.insert(self.next_context);
        self.next_context
    }

    pub fn destroy_context(&mut self, context: i32) -> i32 {
        if self.contexts.remove(&context) {
            0
        } else {
            -1
        }
    }

    pub fn create_handle(&mut self) -> i32 {
        self.next_handle += 1;
        self.handles.insert(self.next_handle);
        self.next_handle
    }

    pub fn destroy_handle(&mut self, handle: i32) -> i32 {
        if self.handles.remove(&handle) {
            0
        } else {
            -1
        }
    }

    pub fn get_unlock_state(&self, flags: &mut OrbisNpTrophyFlagArray, count: &mut u32) -> i32 {
        if !self.contexts.is_empty() {
            flag_zero(flags);
            *count = 0;
            0
        } else {
            -1
        }
    }
}

#[cfg(test)]
mod cpp_tests {
    use super::*;

    #[test]
    fn trophy_contexts_and_handles_are_tracked() {
        let mut trophy = TrophyState::default();
        let context = trophy.create_context();
        let handle = trophy.create_handle();
        assert!(trophy.contexts.contains(&context));
        assert!(trophy.handles.contains(&handle));
        assert_eq!(trophy.destroy_context(context), 0);
        assert_eq!(trophy.destroy_handle(handle), 0);
        assert_eq!(trophy.destroy_handle(handle), -1);
    }
}
