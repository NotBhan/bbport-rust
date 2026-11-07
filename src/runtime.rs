//! Shared runtime contracts, mirroring `legacy/src/runtime.h` and the helpers in
//! `legacy/src/runtime.c` (guest error codes, the NID symbol table and the export lookup).

use crate::src::runtime_imports::IMPORT_NAMES;

/// Base of the guest-visible Orbis error codes (`0x8002_0000 | code`).
pub const ERR_BASE: u32 = 0x8002_0000;

pub const fn orbis(code: u32) -> i32 {
    (ERR_BASE | code) as i32
}

/// A host invariant the C runtime stops on. Kept as an explicit, greppable stop so the reason
/// is visible in logs, like the C `STOP:` lines.
pub fn stop(message: &str) -> ! {
    eprintln!("STOP: {message}");
    std::process::exit(21)
}

/// One entry of a module's export table: the symbol name and the guest-callable address.
#[derive(Debug, Clone, Copy)]
pub struct RuntimeExport {
    pub name: &'static str,
    pub function: usize,
}

/// Symbol name for a scoped NID, or `None` when the eboot/libc never imports it.
pub fn runtime_symbol(nid: &str) -> Option<&'static str> {
    IMPORT_NAMES
        .iter()
        .find(|(candidate, _)| *candidate == nid)
        .map(|(_, symbol)| *symbol)
}

/// Same symbol name means the same contract whether imported from libkernel or libScePosix, so
/// newer modules resolve by name through [`runtime_symbol`].
pub fn runtime_lookup(table: &[RuntimeExport], nid: &str) -> usize {
    let Some(symbol) = runtime_symbol(nid) else {
        return 0;
    };
    table
        .iter()
        .find(|entry| entry.name == symbol)
        .map(|entry| entry.function)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_nids_resolve_to_their_symbols() {
        assert_eq!(runtime_symbol("8G2LB+A3rzg#q#q"), Some("atexit"));
        assert_eq!(runtime_symbol("nope"), None);
    }

    #[test]
    fn lookup_matches_by_symbol_name() {
        fn sample() -> i32 {
            7
        }
        let table = [RuntimeExport {
            name: "atexit",
            function: sample as usize,
        }];
        assert_eq!(runtime_lookup(&table, "8G2LB+A3rzg#q#q"), sample as usize);
        assert_eq!(runtime_lookup(&table, "unrelated#q#q"), 0);
    }

    #[test]
    fn orbis_errors_set_the_base_bit() {
        assert_eq!(orbis(22), (0x8002_0000u32 | 22) as i32);
    }
}
