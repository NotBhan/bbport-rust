//! HLE symbol registration, ported from `legacy/gpu/shim/core/libraries/libs.h`.
//! The C loader queries the resulting table by NID (`bbgpu_resolve`).

use std::collections::BTreeMap;
use std::panic::{catch_unwind, UnwindSafe};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolType {
    Function,
    Object,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub library: String,
    pub module: String,
    pub kind: SymbolType,
    pub address: u64,
}

/// `Core::Loader::SymbolsResolver`: NID -> symbol.
#[derive(Default)]
pub struct SymbolsResolver {
    symbols: BTreeMap<String, Symbol>,
}

impl SymbolsResolver {
    pub fn add_symbol(
        &mut self,
        nid: &str,
        library: &str,
        module: &str,
        kind: SymbolType,
        address: u64,
    ) {
        self.symbols.insert(
            nid.to_string(),
            Symbol {
                library: library.to_string(),
                module: module.to_string(),
                kind,
                address,
            },
        );
    }

    pub fn resolve(&self, nid: &str) -> Option<&Symbol> {
        self.symbols.get(nid)
    }

    pub fn count(&self) -> usize {
        self.symbols.len()
    }
}

/// `ORBIS_KERNEL_ERROR_EIO`, returned by the guest-entry guard for a failed s32 call.
pub const GUEST_ENTRY_EIO: i32 = 0x8002_0005u32 as i32;

/// The Windows `GuestEntry` guard: a panic must not unwind through guest frames.
pub fn guarded_call<R, F>(function: F, fallback: R) -> R
where
    F: FnOnce() -> R + UnwindSafe,
{
    catch_unwind(function).unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbols_are_resolved_by_nid() {
        let mut resolver = SymbolsResolver::default();
        resolver.add_symbol(
            "abc#p#J",
            "libkernel",
            "kernel",
            SymbolType::Function,
            0x1234,
        );
        assert_eq!(resolver.count(), 1);
        let symbol = resolver.resolve("abc#p#J").unwrap();
        assert_eq!(symbol.address, 0x1234);
        assert_eq!(symbol.kind, SymbolType::Function);
        assert!(resolver.resolve("missing").is_none());
    }

    #[test]
    fn the_guest_entry_guard_returns_the_fallback_on_panic() {
        let result = guarded_call(|| -> i32 { panic!("boom") }, GUEST_ENTRY_EIO);
        assert_eq!(result, GUEST_ENTRY_EIO);
        assert_eq!(guarded_call(|| 7i32, 0), 7);
    }
}
