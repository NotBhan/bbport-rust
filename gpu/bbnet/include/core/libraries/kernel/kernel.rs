//! Kernel types, ported from `bbnet/include/core/libraries/kernel/kernel.h`.

/// `CURRENT_FIRMWARE_VERSION`.
pub const CURRENT_FIRMWARE_VERSION: u32 = 0x1400_8001;

/// `Libraries::Kernel::SwVersionStruct`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SwVersionStruct {
    pub struct_size: u64,
    pub text_representation: [u8; 0x1c],
    pub hex_representation: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sw_version_struct_keeps_its_layout() {
        assert_eq!(core::mem::size_of::<SwVersionStruct>(), 0x28);
        assert_eq!(
            core::mem::offset_of!(SwVersionStruct, hex_representation),
            0x24
        );
        assert_eq!(CURRENT_FIRMWARE_VERSION, 0x14008001);
    }
}
