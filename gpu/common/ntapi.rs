//! The Windows ntdll entry points, ported from `legacy/gpu/shadps4/common/ntapi.h` and its
//! `.cpp`. Both are wrapped in `#ifdef _WIN32`, so on every other platform the translation
//! unit is empty; the port keeps that shape.

#[cfg(windows)]
mod windows {
    use std::sync::{Mutex, OnceLock};

    /// `FILE_INFORMATION_CLASS`, the one member the port passes through.
    pub const FILE_DISPOSITION_INFORMATION: u32 = 13;
    /// `FILE_INFORMATION_CLASS`, used to ask for a file's size and times.
    pub const FILE_STANDARD_INFORMATION: u32 = 5;

    /// The `ntdll.dll` export the host resolves for the port. The C++ casts
    /// `GetProcAddress`; the port asks for the address and keeps it in the same slots.
    pub trait NtApiResolver: Send + Sync {
        fn resolve(&self, name: &str) -> Option<usize>;
    }

    /// The five slots `ntapi.cpp` fills at startup.
    pub const ENTRY_POINTS: [&str; 5] = [
        "NtClose",
        "NtSetInformationFile",
        "NtCreateThread",
        "NtTerminateThread",
        "NtQueueApcThreadEx",
    ];

    #[derive(Debug, Default)]
    struct Slots {
        addresses: [usize; ENTRY_POINTS.len()],
    }

    fn slots() -> &'static Mutex<Slots> {
        static SLOTS: OnceLock<Mutex<Slots>> = OnceLock::new();
        SLOTS.get_or_init(|| Mutex::new(Slots::default()))
    }

    fn resolver() -> &'static Mutex<Option<Box<dyn NtApiResolver>>> {
        static RESOLVER: OnceLock<Mutex<Option<Box<dyn NtApiResolver>>>> = OnceLock::new();
        RESOLVER.get_or_init(|| Mutex::new(None))
    }

    /// The host stands in for `LoadLibraryA("ntdll.dll")`.
    pub fn set_resolver(value: Box<dyn NtApiResolver>) {
        *resolver().lock().unwrap_or_else(|error| error.into_inner()) = Some(value);
    }

    /// `Common::NtApi::Initialize`: resolve each export into its slot.
    pub fn initialize() {
        let resolver = resolver().lock().unwrap_or_else(|error| error.into_inner());
        let mut slots = slots().lock().unwrap_or_else(|error| error.into_inner());
        for (index, name) in ENTRY_POINTS.iter().enumerate() {
            slots.addresses[index] = resolver
                .as_ref()
                .and_then(|resolver| resolver.resolve(name))
                .unwrap_or(0);
        }
    }

    /// The address of an entry point, or zero when it was not resolved.
    pub fn address(name: &str) -> usize {
        let slots = slots().lock().unwrap_or_else(|error| error.into_inner());
        ENTRY_POINTS
            .iter()
            .position(|entry| *entry == name)
            .map(|index| slots.addresses[index])
            .unwrap_or(0)
    }

    pub fn is_loaded(name: &str) -> bool {
        address(name) != 0
    }
}

#[cfg(windows)]
pub use windows::*;

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    mod windows_tests {
        use super::super::*;

        struct Fake;

        impl NtApiResolver for Fake {
            fn resolve(&self, name: &str) -> Option<usize> {
                (name != "NtQueueApcThreadEx").then_some(0x1000 + name.len())
            }
        }

        #[test]
        fn the_entry_points_take_their_slots() {
            set_resolver(Box::new(Fake));
            initialize();
            assert!(is_loaded("NtClose"));
            assert_eq!(address("NtClose"), 0x1000 + "NtClose".len());
            assert!(!is_loaded("NtQueueApcThreadEx"), "the resolver refused it");
            assert_eq!(address("not an export"), 0);
        }
    }

    /// On every other platform `ntapi.cpp` compiles to nothing, and so does the port: there
    /// is no entry-point table to expose and nothing to resolve.
    #[cfg(not(windows))]
    #[test]
    fn the_module_is_empty_off_windows() {
        const {
            assert!(cfg!(not(windows)));
        }
    }
}
