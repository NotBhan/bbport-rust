//! Process-wide singletons, ported from `legacy/gpu/shadps4/common/singleton.h`.
//!
//! The C++ keeps one lazily built `T` per type for the process's lifetime; the Rust port
//! leaks the instance on purpose, so its address stays valid exactly as a static's does.

use std::any::TypeId;
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

fn instances() -> &'static Mutex<BTreeMap<TypeId, usize>> {
    static INSTANCES: OnceLock<Mutex<BTreeMap<TypeId, usize>>> = OnceLock::new();
    INSTANCES.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// `Singleton<T>::Instance`.
pub fn instance<T: Default + Send + Sync + 'static>() -> &'static T {
    let mut instances = instances()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Some(&pointer) = instances.get(&TypeId::of::<T>()) {
        // The pointer comes from a leaked box of this exact type and is never freed.
        return unsafe { &*(pointer as *const T) };
    }
    let leaked: &'static T = Box::leak(Box::new(T::default()));
    instances.insert(TypeId::of::<T>(), leaked as *const T as usize);
    leaked
}

/// Whether a type's instance exists already (diagnostics).
pub fn is_instantiated<T: 'static>() -> bool {
    instances()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .contains_key(&TypeId::of::<T>())
}

fn _assert_object_safe(_value: &dyn Send) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct First {
        value: u32,
    }

    #[derive(Default)]
    struct Second {
        value: u32,
    }

    #[test]
    fn one_instance_per_type_is_reused() {
        let first = instance::<First>();
        let again = instance::<First>();
        assert!(std::ptr::eq(first, again), "the same instance comes back");
        assert_eq!(first.value, 0);
        assert!(!is_instantiated::<Second>());
        assert_eq!(instance::<Second>().value, 0);
        assert!(is_instantiated::<Second>());
        assert!(is_instantiated::<First>());
    }
}
