//! Background update checking and notification.
//!
//! Checks for new CLI versions via the GitHub Releases API every 24 hours and
//! notifies the user via stderr. Skipped when the binary lives in /opt/homebrew/bin
//! (Homebrew manages updates there), when RICOCHET_NO_UPDATE_CHECK is set, or when
//! skip_update_check is set in the config (auto-set after repeated GitHub API failures).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config::Config;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const GITHUB_API_URL: &str = "https://api.github.com/repos/ricochet-rs/cli/releases/latest";
const RELEASE_NOTES_BASE: &str = "https://github.com/ricochet-rs/cli/releases/tag";
const CHECK_INTERVAL_SECS: u64 = 60 * 60 * 24; // 24 hours
const MAX_CONSECUTIVE_FAILURES: u32 = 3;

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCache {
    pub last_checked: chrono::DateTime<chrono::Utc>,
    pub latest_version: String,
    #[serde(default)]
    pub consecutive_failures: u32,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// Returns true if update checks should be suppressed.
fn should_suppress_checks(config: &Config) -> bool {
    if config.skip_update_check == Some(true) {
        return true;
    }
    if std::env::var("RICOCHET_NO_UPDATE_CHECK")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
    {
        return true;
    }
    if std::env::var("CI").is_ok() {
        return true;
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
        && parent.starts_with("/opt/homebrew")
    {
        return true;
    }
    false
}

pub fn cache_path() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir().context("Failed to get cache directory")?;
    Ok(cache_dir.join("ricochet").join("update-check.json"))
}

pub fn load_cache() -> Option<UpdateCache> {
    let path = cache_path().ok()?;
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_cache(cache: &UpdateCache) -> Result<()> {
    let path = cache_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create cache directory")?;
    }
    let content =
        serde_json::to_string_pretty(cache).context("Failed to serialize update cache")?;
    std::fs::write(&path, content).context("Failed to write update cache")?;
    Ok(())
}

pub fn release_notes_url(version: &str) -> String {
    format!("{}/v{}", RELEASE_NOTES_BASE, version)
}

/// Fetch the latest release version string (without leading 'v') from GitHub.
pub async fn fetch_latest_version() -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("ricochet-cli/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let release: GitHubRelease = client
        .get(GITHUB_API_URL)
        .send()
        .await
        .context("Failed to contact GitHub API")?
        .error_for_status()
        .context("GitHub API returned error")?
        .json()
        .await
        .context("Failed to parse GitHub API response")?;

    Ok(release.tag_name.trim_start_matches('v').to_string())
}

/// Returns true if `candidate` is a newer version than `current`.
pub fn is_newer(current: &str, candidate: &str) -> bool {
    fn parse(v: &str) -> Option<(u64, u64, u64)> {
        let v = v.split('-').next()?;
        let parts: Vec<u64> = v.split('.').filter_map(|p| p.parse().ok()).collect();
        if parts.len() >= 3 {
            Some((parts[0], parts[1], parts[2]))
        } else {
            None
        }
    }
    match (parse(current), parse(candidate)) {
        (Some(c), Some(n)) => n > c,
        _ => false,
    }
}

/// Background task: fetch latest version and save to cache.
/// On success, resets the failure counter. On failure, increments it.
/// After MAX_CONSECUTIVE_FAILURES, auto-disables update checks in the config
/// and notifies the user via stderr.
pub async fn check_for_update() -> Option<String> {
    let previous_failures = load_cache()
        .map(|c| c.consecutive_failures)
        .unwrap_or(0);

    match fetch_latest_version().await {
        Ok(latest) => {
            let cache = UpdateCache {
                last_checked: chrono::Utc::now(),
                latest_version: latest.clone(),
                consecutive_failures: 0,
            };
            let _ = save_cache(&cache);
            if is_newer(CURRENT_VERSION, &latest) {
                Some(latest)
            } else {
                None
            }
        }
        Err(_) => {
            let failures = previous_failures + 1;
            let cache = UpdateCache {
                last_checked: chrono::Utc::now(),
                latest_version: load_cache()
                    .map(|c| c.latest_version)
                    .unwrap_or_else(|| CURRENT_VERSION.to_string()),
                consecutive_failures: failures,
            };
            let _ = save_cache(&cache);

            if failures >= MAX_CONSECUTIVE_FAILURES {
                disable_update_checks();
            }

            None
        }
    }
}

/// Disable update checks by setting skip_update_check in the config file,
/// and inform the user via stderr.
fn disable_update_checks() {
    use colored::Colorize;

    if let Ok(mut config) = Config::load() {
        config.skip_update_check = Some(true);
        let _ = config.save();
    }

    eprintln!(
        "\n{} Automatic update checks have been disabled after {} consecutive failures reaching GitHub.\n  To re-enable, set {} in your config file or run:\n  {}",
        "notice:".yellow().bold(),
        MAX_CONSECUTIVE_FAILURES,
        "skip_update_check = false".bright_cyan(),
        "ricochet self-update".bright_cyan(),
    );
}

/// Print a one-line stderr notice if a newer version is recorded in the cache.
/// Reads the on-disk cache synchronously — no network call.
pub fn maybe_notify_update(config: &Config) {
    if should_suppress_checks(config) {
        return;
    }
    use colored::Colorize;
    let Some(cache) = load_cache() else { return };
    if is_newer(CURRENT_VERSION, &cache.latest_version) {
        eprintln!(
            "\n{} A new version of ricochet is available: {} -> {}\n  Update with: {}\n  Release notes: {}",
            "notice:".yellow().bold(),
            CURRENT_VERSION.dimmed(),
            cache.latest_version.green().bold(),
            "ricochet self-update".bright_cyan(),
            release_notes_url(&cache.latest_version).dimmed(),
        );
    }
}

/// If the last update check was more than 24h ago (or never), spawn a background
/// tokio task to fetch the latest version and refresh the cache.
/// Returns the JoinHandle so the caller can await it with a timeout.
pub fn trigger_background_check(config: &Config) -> Option<tokio::task::JoinHandle<()>> {
    if should_suppress_checks(config) {
        return None;
    }

    let should_check = match load_cache() {
        None => true,
        Some(cache) => {
            let age = chrono::Utc::now()
                .signed_duration_since(cache.last_checked)
                .num_seconds()
                .unsigned_abs();
            age >= CHECK_INTERVAL_SECS
        }
    };

    if should_check {
        Some(tokio::spawn(async {
            let _ = check_for_update().await;
        }))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_basic() {
        assert!(is_newer("0.3.0", "0.4.0"));
        assert!(is_newer("0.3.0", "0.3.1"));
        assert!(is_newer("0.3.0", "1.0.0"));
        assert!(!is_newer("0.3.0", "0.3.0"));
        assert!(!is_newer("0.4.0", "0.3.0"));
    }

    #[test]
    fn test_is_newer_double_digit() {
        assert!(is_newer("0.9.0", "0.10.0"));
        assert!(!is_newer("0.10.0", "0.9.0"));
    }

    #[test]
    fn test_is_newer_with_prerelease() {
        assert!(is_newer("0.3.0-abc1234", "0.4.0"));
        assert!(!is_newer("0.4.0", "0.3.0-abc1234"));
    }

    #[test]
    fn test_is_newer_unparseable_returns_false() {
        assert!(!is_newer("0.3.0", "garbage"));
        assert!(!is_newer("garbage", "0.4.0"));
        assert!(!is_newer("garbage", "also-garbage"));
    }

    #[test]
    fn test_release_notes_url() {
        assert_eq!(
            release_notes_url("0.4.0"),
            "https://github.com/ricochet-rs/cli/releases/tag/v0.4.0"
        );
    }

    #[test]
    fn test_cache_roundtrip_with_failures() {
        let cache = UpdateCache {
            last_checked: chrono::Utc::now(),
            latest_version: "0.4.0".to_string(),
            consecutive_failures: 2,
        };
        let json = serde_json::to_string(&cache).unwrap();
        let loaded: UpdateCache = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.consecutive_failures, 2);
        assert_eq!(loaded.latest_version, "0.4.0");
    }

    #[test]
    fn test_cache_deserializes_without_failures_field() {
        // Backward compat: old cache files won't have consecutive_failures
        let json = r#"{"last_checked":"2026-02-20T00:00:00Z","latest_version":"0.3.0"}"#;
        let loaded: UpdateCache = serde_json::from_str(json).unwrap();
        assert_eq!(loaded.consecutive_failures, 0);
    }
}
