//! Guest process and module queries, ported from
//! `legacy/gpu/shadps4/core/libraries/kernel/process.h`. The `.cpp` is not part of the tree,
//! so the entry points report a documented placeholder and the structures they fill are exact.

use crate::gpu::common::types::VAddr;
use crate::gpu::shim::core::libraries::libs::SymbolsResolver;

/// `ORBIS_DBG_MAX_NAME_LENGTH`.
pub const ORBIS_DBG_MAX_NAME_LENGTH: usize = 256;

/// `Libraries::Kernel::OrbisModuleInfoForUnwind`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OrbisModuleInfoForUnwind {
    pub st_size: u64,
    pub name: [u8; ORBIS_DBG_MAX_NAME_LENGTH],
    pub eh_frame_hdr_addr: VAddr,
    pub eh_frame_addr: VAddr,
    pub eh_frame_size: u64,
    pub seg0_addr: VAddr,
    pub seg0_size: u64,
}

impl Default for OrbisModuleInfoForUnwind {
    fn default() -> Self {
        Self {
            st_size: 0,
            name: [0; ORBIS_DBG_MAX_NAME_LENGTH],
            eh_frame_hdr_addr: 0,
            eh_frame_addr: 0,
            eh_frame_size: 0,
            seg0_addr: 0,
            seg0_size: 0,
        }
    }
}

impl OrbisModuleInfoForUnwind {
    /// The module name as text, without the terminator.
    pub fn name_str(&self) -> &str {
        let end = self
            .name
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(self.name.len());
        std::str::from_utf8(&self.name[..end]).unwrap_or("")
    }

    /// The size the caller reports, which `st_size` is checked against.
    pub const fn reported_size() -> u64 {
        std::mem::size_of::<Self>() as u64
    }
}

/// The code the entry points report until the missing `.cpp` fills them in.
pub const NOT_IMPLEMENTED: i32 = 0x8002_0006u32 as i32; // ORBIS_KERNEL_ERROR_ENOSYS

/// `sceKernelIsNeoMode`: the emulator reports Neo hardware, as the C++ does.
pub fn sce_kernel_is_neo_mode() -> i32 {
    1
}

/// `sceKernelGetCompiledSdkVersion`.
pub fn sce_kernel_get_compiled_sdk_version(_version: &mut i32) -> i32 {
    NOT_IMPLEMENTED
}

/// `sceKernelGetModuleInfoForUnwind`.
pub fn sce_kernel_get_module_info_for_unwind(
    _address: VAddr,
    _flags: i32,
    _info: &mut OrbisModuleInfoForUnwind,
) -> i32 {
    NOT_IMPLEMENTED
}

/// `Libraries::Kernel::RegisterProcess`: the NID table lives in the missing `.cpp`, so the
/// resolver is handed to the loader's own lookup, as the other library ports do.
pub fn register_process<F>(resolver: &mut SymbolsResolver, mut lookup: F)
where
    F: FnMut(&str) -> Option<(&'static str, u64)>,
{
    for name in EXPORT_NAMES {
        if let Some((nid, address)) = lookup(name) {
            resolver.add_symbol(
                nid,
                "libSceKernel",
                "process",
                crate::gpu::shim::core::libraries::libs::SymbolType::Function,
                address,
            );
        }
    }
}

/// The exports `RegisterProcess` registers.
pub const EXPORT_NAMES: [&str; 3] = [
    "sceKernelIsNeoMode",
    "sceKernelGetCompiledSdkVersion",
    "sceKernelGetModuleInfoForUnwind",
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn the_unwind_info_is_the_size_the_sdk_expects() {
        assert_eq!(ORBIS_DBG_MAX_NAME_LENGTH, 256);
        // 8 + 256 + 8 + 8 + 8 + 8 + 8.
        assert_eq!(size_of::<OrbisModuleInfoForUnwind>(), 304);
        assert_eq!(OrbisModuleInfoForUnwind::reported_size(), 304);
        let info = OrbisModuleInfoForUnwind {
            name: [b'A'; ORBIS_DBG_MAX_NAME_LENGTH],
            seg0_size: 0x1000,
            ..Default::default()
        };
        assert_eq!(info.name_str().len(), ORBIS_DBG_MAX_NAME_LENGTH);
        assert_eq!(info.seg0_size, 0x1000);
    }

    #[test]
    fn the_entry_points_report_the_neo_mode_and_the_placeholder() {
        assert_eq!(sce_kernel_is_neo_mode(), 1);
        let mut version = 0;
        assert_eq!(
            sce_kernel_get_compiled_sdk_version(&mut version),
            NOT_IMPLEMENTED
        );
        assert_eq!(version, 0, "a failed call leaves the out value alone");
        let mut info = OrbisModuleInfoForUnwind::default();
        assert_eq!(
            sce_kernel_get_module_info_for_unwind(0x8000_0000, 0, &mut info),
            NOT_IMPLEMENTED
        );
    }

    #[test]
    fn registering_binds_the_exports_the_loader_knows() {
        let mut resolver = SymbolsResolver::default();
        register_process(&mut resolver, |name| {
            (name == "sceKernelIsNeoMode").then_some(("hHhF9Dt5X8k", 0x8002_4000u64))
        });
        assert_eq!(resolver.count(), 1);
        let symbol = resolver.resolve("hHhF9Dt5X8k").unwrap();
        assert_eq!(symbol.module, "process");
        assert_eq!(symbol.library, "libSceKernel");

        let mut empty = SymbolsResolver::default();
        register_process(&mut empty, |_| None);
        assert_eq!(empty.count(), 0);
    }
}
