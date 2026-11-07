//! HLE symbol registration, ported from `bbnet/include/core/libraries/libs.h`.

/// `Core::Loader::SymbolType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolType {
    Function,
    Object,
}

/// `Core::Loader::SymbolsResolver`.
pub trait SymbolsResolver {
    fn add_symbol(
        &mut self,
        nid: &str,
        library: &str,
        module: &str,
        symbol_type: SymbolType,
        address: u64,
    );
}

/// The error a guest entry returns when a Rust panic crosses the boundary.
pub const ORBIS_KERNEL_ERROR_EIO: i32 = 0x8002_0005u32 as i32;

/// `LIB_FUNCTION`.
pub fn register_function(
    resolver: &mut dyn SymbolsResolver,
    nid: &str,
    library: &str,
    _library_version: &str,
    module: &str,
    address: u64,
) {
    resolver.add_symbol(nid, library, module, SymbolType::Function, address);
}

/// `LIB_OBJ`.
pub fn register_object(
    resolver: &mut dyn SymbolsResolver,
    nid: &str,
    library: &str,
    _library_version: &str,
    module: &str,
    address: u64,
) {
    resolver.add_symbol(nid, library, module, SymbolType::Object, address);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Resolver {
        entries: Vec<(String, String, String, SymbolType, u64)>,
    }

    impl SymbolsResolver for Resolver {
        fn add_symbol(
            &mut self,
            nid: &str,
            library: &str,
            module: &str,
            symbol_type: SymbolType,
            address: u64,
        ) {
            self.entries.push((
                nid.to_string(),
                library.to_string(),
                module.to_string(),
                symbol_type,
                address,
            ));
        }
    }

    #[test]
    fn functions_and_objects_register_with_their_type() {
        let mut resolver = Resolver::default();
        register_function(&mut resolver, "nid1", "lib", "1", "mod", 0x1000);
        register_object(&mut resolver, "nid2", "lib", "1", "mod", 0x2000);
        assert_eq!(resolver.entries.len(), 2);
        assert_eq!(resolver.entries[0].3, SymbolType::Function);
        assert_eq!(resolver.entries[0].4, 0x1000);
        assert_eq!(resolver.entries[1].3, SymbolType::Object);
        assert_eq!(ORBIS_KERNEL_ERROR_EIO, 0x80020005u32 as i32);
    }
}
