//! The Hunter's Dream backend surface, ported from `bbnet/thd/thd.h`.

use std::sync::{Mutex, OnceLock};

use crate::gpu::bbnet::include::core::libraries::libs::SymbolsResolver;

/// The backend the launcher installs when `BB_NET_BACKEND=thd`.
pub trait Backend: Send {
    /// `Initialize`.
    fn initialize(&mut self);
    /// `Register`.
    fn register(&mut self, resolver: &mut dyn SymbolsResolver);
    /// `SetImage`.
    fn set_image(&mut self, image: &mut [u8]);
    /// `AddGameHeaders`.
    fn add_game_headers(&self, headers: &mut Vec<(String, String)>);
}

fn backend() -> &'static Mutex<Option<Box<dyn Backend>>> {
    static BACKEND: OnceLock<Mutex<Option<Box<dyn Backend>>>> = OnceLock::new();
    BACKEND.get_or_init(|| Mutex::new(None))
}

/// Installs the backend implementation.
pub fn set_backend(implementation: Box<dyn Backend>) {
    *backend().lock().unwrap_or_else(|error| error.into_inner()) = Some(implementation);
}

/// `Enabled`: `BB_NET_BACKEND=thd`.
pub fn enabled() -> bool {
    std::env::var("BB_NET_BACKEND").is_ok_and(|value| value == "thd")
}

fn with_backend(mut operation: impl FnMut(&mut dyn Backend)) -> bool {
    let mut backend = backend().lock().unwrap_or_else(|error| error.into_inner());
    match backend.as_mut() {
        Some(backend) => {
            operation(backend.as_mut());
            true
        }
        None => false,
    }
}

/// `Initialize`.
pub fn initialize() {
    with_backend(|backend| backend.initialize());
}

/// `Register`.
pub fn register(resolver: &mut dyn SymbolsResolver) {
    with_backend(|backend| backend.register(resolver));
}

/// `SetImage`.
pub fn set_image(image: &mut [u8]) {
    with_backend(|backend| backend.set_image(image));
}

/// `AddGameHeaders`.
pub fn add_game_headers(headers: &mut Vec<(String, String)>) {
    let backend = backend().lock().unwrap_or_else(|error| error.into_inner());
    if let Some(backend) = backend.as_ref() {
        backend.add_game_headers(headers);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::bbnet::include::core::libraries::libs::SymbolType;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    struct MockBackend {
        initialized: Arc<AtomicU32>,
        resolved: Arc<AtomicU32>,
    }

    impl Backend for MockBackend {
        fn initialize(&mut self) {
            self.initialized.fetch_add(1, Ordering::AcqRel);
        }

        fn register(&mut self, resolver: &mut dyn SymbolsResolver) {
            resolver.add_symbol("nid", "lib", "mod", SymbolType::Function, 1);
            self.resolved.fetch_add(1, Ordering::AcqRel);
        }

        fn set_image(&mut self, image: &mut [u8]) {
            image.fill(0xAA);
        }

        fn add_game_headers(&self, headers: &mut Vec<(String, String)>) {
            headers.push(("X-BBHost-Ruleset".to_string(), "vanilla".to_string()));
        }
    }

    #[derive(Default)]
    struct Resolver {
        count: u32,
    }

    impl SymbolsResolver for Resolver {
        fn add_symbol(
            &mut self,
            _nid: &str,
            _library: &str,
            _module: &str,
            _symbol_type: SymbolType,
            _address: u64,
        ) {
            self.count += 1;
        }
    }

    #[test]
    fn the_backend_receives_the_calls() {
        let initialized = Arc::new(AtomicU32::new(0));
        let resolved = Arc::new(AtomicU32::new(0));
        set_backend(Box::new(MockBackend {
            initialized: initialized.clone(),
            resolved: resolved.clone(),
        }));

        initialize();
        assert_eq!(initialized.load(Ordering::Acquire), 1);

        let mut resolver = Resolver::default();
        register(&mut resolver);
        assert_eq!(resolver.count, 1);
        assert_eq!(resolved.load(Ordering::Acquire), 1);

        let mut image = [0u8; 4];
        set_image(&mut image);
        assert_eq!(image, [0xAA; 4]);

        let mut headers = Vec::new();
        add_game_headers(&mut headers);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, "X-BBHost-Ruleset");

        // `Enabled` is environment-driven and read-only here.
        let _ = enabled();
    }
}
