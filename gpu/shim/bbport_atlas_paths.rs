//! Hunter's Atlas path environment variables, ported from
//! `legacy/gpu/shim/bbport_atlas_paths.h`.

use std::path::PathBuf;

/// `BbAtlasPath`: the environment variable when set and non-empty, else `fallback`.
pub fn atlas_path(name: &str, fallback: impl Into<PathBuf>) -> PathBuf {
    match std::env::var_os(name) {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => fallback.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_value_wins_and_fallback_is_used_otherwise() {
        let name = "BBPORT_RS_ATLAS_TEST_PATH";
        std::env::set_var(name, "/tmp/atlas");
        assert_eq!(atlas_path(name, "/fallback"), PathBuf::from("/tmp/atlas"));
        std::env::remove_var(name);
        assert_eq!(atlas_path(name, "/fallback"), PathBuf::from("/fallback"));
    }
}
