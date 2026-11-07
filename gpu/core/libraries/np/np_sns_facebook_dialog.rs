//! The SNS Facebook dialog library, ported from
//! `legacy/gpu/bbnet/vendor/shadps4/core/libraries/np/np_sns_facebook_dialog.h` and its `.cpp`.

use crate::gpu::shim::core::libraries::libs::{SymbolType, SymbolsResolver};

/// `Libraries::CommonDialog::Status`. `commondialog.h` is not part of the trimmed tree, so the
/// port carries the one member the stub reports.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    None,
}

/// `sceNpSnsFacebookDialogUpdateStatus`: stubbed, as the C++ is.
pub fn sce_np_sns_facebook_dialog_update_status() -> Status {
    Status::None
}

/// The library name the C++ registers under, and its one export.
pub const LIBRARY_NAME: &str = "libSceNpSnsFacebookDialog";
pub const EXPORT_NAMES: [&str; 1] = ["sceNpSnsFacebookDialogUpdateStatus"];

/// `Libraries::Np::NpSnsFacebookDialog::RegisterLib`: `lookup` maps an export name to its NID
/// and its address in the runtime, as the loader registers them.
pub fn register_lib<F>(resolver: &mut SymbolsResolver, mut lookup: F)
where
    F: FnMut(&str) -> Option<(&'static str, u64)>,
{
    for name in EXPORT_NAMES {
        if let Some((nid, address)) = lookup(name) {
            resolver.add_symbol(
                nid,
                LIBRARY_NAME,
                LIBRARY_NAME,
                SymbolType::Function,
                address,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The NID `np_sns_facebook_dialog.cpp` registers for the stub.
    const UPDATE_STATUS_NID: &str = "fjV7C8H0Y8k";

    #[test]
    fn registering_exposes_the_dialog_entry_point() {
        let mut resolver = SymbolsResolver::default();
        register_lib(&mut resolver, |name| {
            (name == "sceNpSnsFacebookDialogUpdateStatus")
                .then_some((UPDATE_STATUS_NID, 0x8002_3000u64))
        });
        assert_eq!(resolver.count(), 1);
        let symbol = resolver.resolve(UPDATE_STATUS_NID).unwrap();
        assert_eq!(symbol.library, LIBRARY_NAME);
        assert_eq!(symbol.module, LIBRARY_NAME);
        assert_eq!(symbol.kind, SymbolType::Function);
        assert_eq!(symbol.address, 0x8002_3000);
        assert_eq!(sce_np_sns_facebook_dialog_update_status(), Status::None);
    }

    #[test]
    fn a_missing_export_registers_nothing() {
        let mut resolver = SymbolsResolver::default();
        register_lib(&mut resolver, |_| None);
        assert_eq!(resolver.count(), 0);
    }
}
