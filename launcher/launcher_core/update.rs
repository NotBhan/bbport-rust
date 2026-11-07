//! GitHub release checks and the update cache, ported from the Linux launcher's update code.
//!
//! Networking goes through a small [`Fetcher`] trait so the logic is testable; the default
//! implementation shells out to `curl`, which every Linux distribution ships.

use crate::fsutil::atomic_write;
use crate::paths::Paths;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Command;
use thiserror::Error;

pub const VERSION: &str = "1.10.12";
pub const REPO_URL: &str = "https://github.com/NotBhan/bloodborne-ps2pc";
pub const RELEASES_API: &str =
    "https://api.github.com/repos/NotBhan/bloodborne-ps2pc/releases/latest";
pub const RELEASES_PAGE: &str = "https://github.com/NotBhan/bloodborne-ps2pc/releases/latest";

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("network request failed: {0}")]
    Request(String),
    #[error("release reply is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("cannot write the update cache: {0}")]
    Cache(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub tag: String,
    pub asset_url: Option<String>,
    pub page: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCache {
    pub checked: f64,
    pub version: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub page: String,
}

pub trait Fetcher {
    fn get(&self, url: &str) -> Result<String, String>;

    /// POST a JSON body and return the reply body, including HTTP error bodies (the account
    /// service answers errors as JSON, like the Python `urllib` code reads `HTTPError.read()`).
    fn post_json(&self, url: &str, body: &str) -> Result<String, String> {
        let _ = (url, body);
        Err("this fetcher does not support POST".to_string())
    }

    /// Download a URL to a file, used for release assets.
    fn download_to(&self, url: &str, path: &std::path::Path) -> Result<(), String> {
        let _ = (url, path);
        Err("this fetcher does not support downloads".to_string())
    }
}

/// Uses the system `curl`, the same binary the shell scripts rely on.
pub struct CurlFetcher;

impl Fetcher for CurlFetcher {
    fn get(&self, url: &str) -> Result<String, String> {
        let output = Command::new("curl")
            .args([
                "-fsSL",
                "--max-time",
                "20",
                "-H",
                "Accept: application/vnd.github+json",
                "-H",
                "User-Agent: bbport-launcher",
                url,
            ])
            .output()
            .map_err(|error| format!("cannot run curl: {error}"))?;
        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if message.is_empty() {
                format!("curl exited with {}", output.status)
            } else {
                message
            });
        }
        String::from_utf8(output.stdout).map_err(|error| format!("reply is not UTF-8: {error}"))
    }

    fn post_json(&self, url: &str, body: &str) -> Result<String, String> {
        let output = Command::new("curl")
            .args([
                "-sS",
                "--max-time",
                "15",
                "-X",
                "POST",
                "-H",
                "Content-Type: application/json",
                "-H",
                "User-Agent: bbport-launcher",
                "-d",
                body,
                url,
            ])
            .output()
            .map_err(|error| format!("cannot run curl: {error}"))?;
        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if message.is_empty() {
                format!("curl exited with {}", output.status)
            } else {
                message
            });
        }
        String::from_utf8(output.stdout).map_err(|error| format!("reply is not UTF-8: {error}"))
    }

    fn download_to(&self, url: &str, path: &std::path::Path) -> Result<(), String> {
        let output = Command::new("curl")
            .args([
                "-fSL",
                "--retry",
                "3",
                "--max-time",
                "900",
                "-H",
                "User-Agent: bbport-launcher",
                "-o",
            ])
            .arg(path)
            .arg(url)
            .output()
            .map_err(|error| format!("cannot run curl: {error}"))?;
        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if message.is_empty() {
                format!("curl exited with {}", output.status)
            } else {
                message
            });
        }
        Ok(())
    }
}

/// `re.findall(r"\d+", text)` as numbers, like the launcher's `version_tuple`.
pub fn version_tuple(text: &str) -> Vec<u64> {
    let mut numbers = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        if character.is_ascii_digit() {
            current.push(character);
        } else if !current.is_empty() {
            if let Ok(number) = current.parse() {
                numbers.push(number);
            }
            current.clear();
        }
    }
    if !current.is_empty() {
        if let Ok(number) = current.parse() {
            numbers.push(number);
        }
    }
    numbers
}

pub fn release_is_newer(tag: &str, current: &str) -> bool {
    version_tuple(tag) > version_tuple(current)
}

/// `(tag, tar.gz URL, release page)` of the newest GitHub release, cached for the UI.
pub fn latest_release(fetcher: &dyn Fetcher, paths: &Paths) -> Result<Release, UpdateError> {
    let body = fetcher.get(RELEASES_API).map_err(UpdateError::Request)?;
    let release: Value = serde_json::from_str(&body)?;
    let tag = release
        .get("tag_name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let asset_url = release
        .get("assets")
        .and_then(Value::as_array)
        .and_then(|assets| {
            assets.iter().find_map(|asset| {
                let name = asset.get("name").and_then(Value::as_str)?;
                name.to_lowercase()
                    .ends_with(".tar.gz")
                    .then(|| asset.get("browser_download_url").and_then(Value::as_str))
                    .flatten()
                    .map(str::to_string)
            })
        });
    let page = release
        .get("html_url")
        .and_then(Value::as_str)
        .filter(|page| !page.is_empty())
        .unwrap_or(RELEASES_PAGE)
        .to_string();

    let cache = UpdateCache {
        checked: now_seconds(),
        version: tag.clone(),
        url: asset_url.clone(),
        page: page.clone(),
    };
    if let Ok(text) = serde_json::to_string(&cache) {
        let _ = atomic_write(&paths.update_cache, text.as_bytes());
    }
    Ok(Release {
        tag,
        asset_url,
        page,
    })
}

/// The last GitHub answer, when it is newer than this launcher.
pub fn remembered_update(paths: &Paths) -> Option<(String, Option<String>, String)> {
    let text = std::fs::read_to_string(&paths.update_cache).ok()?;
    let cache: UpdateCache = serde_json::from_str(&text).ok()?;
    if !release_is_newer(&cache.version, VERSION) {
        return None;
    }
    let page = if cache.page.is_empty() {
        RELEASES_PAGE.to_string()
    } else {
        cache.page
    };
    Some((cache.version, cache.url, page))
}

fn now_seconds() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsutil::atomic_write;

    struct FakeFetcher(String);

    impl Fetcher for FakeFetcher {
        fn get(&self, _url: &str) -> Result<String, String> {
            Ok(self.0.clone())
        }
    }

    fn temp_paths(tag: &str) -> (Paths, std::path::PathBuf) {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("bbport-rs-update-{tag}-{stamp}"));
        std::fs::create_dir_all(root.join("launcher")).unwrap();
        (
            Paths::with_config_dir(root.join("launcher"), root.join("config")),
            root,
        )
    }

    #[test]
    fn version_tuples_compare_like_python() {
        assert_eq!(version_tuple("v1.10.12"), vec![1, 10, 12]);
        assert!(release_is_newer("v1.10.12", "1.10.11"));
        assert!(!release_is_newer("v1.10.11", "1.10.11"));
        assert!(release_is_newer("1.10", "1.9.9"));
        assert!(!release_is_newer("1.10", "1.10.0"));
    }

    #[test]
    fn latest_release_selects_the_tarball_asset_and_caches_it() {
        let (paths, root) = temp_paths("release");
        let fetcher = FakeFetcher(
            r#"{"tag_name":"v1.10.13","html_url":"https://example/page",
                "assets":[{"name":"bbport-windows.zip","browser_download_url":"https://example/zip"},
                          {"name":"bbport-linux-x86_64.tar.gz","browser_download_url":"https://example/tar"}]}"#
                .into(),
        );
        let release = latest_release(&fetcher, &paths).unwrap();
        assert_eq!(release.tag, "v1.10.13");
        assert_eq!(release.asset_url.as_deref(), Some("https://example/tar"));
        let remembered = remembered_update(&paths).unwrap();
        assert_eq!(remembered.0, "v1.10.13");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn remembered_update_ignores_the_current_version() {
        let (paths, root) = temp_paths("remembered");
        let cache = UpdateCache {
            checked: 0.0,
            version: "v1.10.12".into(),
            url: None,
            page: RELEASES_PAGE.into(),
        };
        atomic_write(
            &paths.update_cache,
            serde_json::to_string(&cache).unwrap().as_bytes(),
        )
        .unwrap();
        assert!(remembered_update(&paths).is_none());
        let _ = std::fs::remove_dir_all(root);
    }
}
