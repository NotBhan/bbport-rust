//! Build revision strings, ported from `legacy/gpu/shadps4/common/scm_rev.h`. The C++ fills
//! these from a generated `scm_rev.cpp`; the Rust port reads the crate's own metadata and
//! leaves the VCS fields empty unless a build script fills them.

/// `Common::g_version`.
pub const G_VERSION: &str = env!("CARGO_PKG_VERSION");
/// `Common::g_is_release`: debug builds carry the debug flag.
pub const G_IS_RELEASE: bool = !cfg!(debug_assertions);

/// `Common::g_scm_rev` (empty when the build has no VCS information).
pub const G_SCM_REV: &str = "";
/// `Common::g_scm_branch` (empty when the build has no VCS information).
pub const G_SCM_BRANCH: &str = "";
/// `Common::g_scm_desc` (empty when the build has no VCS information).
pub const G_SCM_DESC: &str = "";
/// `Common::g_scm_remote_name` (empty when the build has no VCS information).
pub const G_SCM_REMOTE_NAME: &str = "";
/// `Common::g_scm_remote_url` (empty when the build has no VCS information).
pub const G_SCM_REMOTE_URL: &str = "";
/// `Common::g_scm_date` (empty when the build has no VCS information).
pub const G_SCM_DATE: &str = "";

/// `GetRemoteNameFromLink`: the `owner/repository` part of a GitHub remote URL, or an empty
/// string for anything else.
pub fn get_remote_name_from_link() -> String {
    remote_name_from(G_SCM_REMOTE_URL)
}

fn remote_name_from(link: &str) -> String {
    let link = link.trim();
    for prefix in [
        "https://github.com/",
        "http://github.com/",
        "git@github.com:",
        "ssh://git@github.com/",
        "git://github.com/",
    ] {
        if let Some(rest) = link.strip_prefix(prefix) {
            let rest = rest.strip_suffix(".git").unwrap_or(rest);
            let rest = rest.trim_end_matches('/');
            return if rest.split('/').count() == 2 {
                rest.to_string()
            } else {
                String::new()
            };
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_link_yields_owner_and_repository_only() {
        assert_eq!(
            remote_name_from("https://github.com/shadps4-emu/shadPS4.git"),
            "shadps4-emu/shadPS4"
        );
        assert_eq!(
            remote_name_from("git@github.com:shadps4-emu/shadPS4.git"),
            "shadps4-emu/shadPS4"
        );
        assert_eq!(remote_name_from("https://gitlab.com/owner/repo"), "");
        assert_eq!(remote_name_from("https://github.com/owner"), "");
        assert_eq!(remote_name_from(""), "");
    }

    #[test]
    fn the_version_is_the_crate_version() {
        assert!(!G_VERSION.is_empty());
        assert_eq!(G_IS_RELEASE, !cfg!(debug_assertions));
        assert!(G_SCM_REV.is_empty(), "no VCS information is compiled in");
        assert_eq!(get_remote_name_from_link(), "");
    }
}
