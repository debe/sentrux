//! Non-blocking update checker — checks GitHub releases for newer versions.
//!
//! Runs once per day (cached in ~/.sentrux/last_update_check). Does not block
//! the main thread — spawns a background thread that prints a message if a
//! newer version is available.
//!
//! Respects SENTRUX_NO_UPDATE_CHECK=1 to disable entirely.
//! When a proxy endpoint is available, changing UPDATE_CHECK_URL enables
//! usage counting without any binary change.

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// URL to check for latest version. Cloudflare Worker endpoint that
/// serves version info while counting daily active users.
const UPDATE_CHECK_URL: &str =
    "https://api.sentrux.dev/version";

/// How often to check (24 hours).
const CHECK_INTERVAL: Duration = Duration::from_secs(86400);

/// Path to the cache file that stores last check timestamp.
fn cache_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".sentrux").join("last_update_check"))
}

/// Check if enough time has passed since last check.
fn should_check() -> bool {
    if std::env::var("SENTRUX_NO_UPDATE_CHECK").is_ok() {
        return false;
    }
    let path = match cache_path() {
        Some(p) => p,
        None => return true, // can't cache, check anyway
    };
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let last_check: f64 = content.trim().parse().unwrap_or(0.0);
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
            (now - last_check) > CHECK_INTERVAL.as_secs_f64()
        }
        Err(_) => true, // no cache file = never checked
    }
}

/// Save current timestamp as last check time.
fn save_check_timestamp() {
    if let Some(path) = cache_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        let _ = std::fs::write(&path, format!("{:.0}", now));
    }
}

/// Parse version string like "v0.2.0" or "0.2.0" into (major, minor, patch).
fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let v = v.strip_prefix('v').unwrap_or(v);
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
}

/// Check if `latest` is newer than `current`.
fn is_newer(current: &str, latest: &str) -> bool {
    match (parse_version(current), parse_version(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => false,
    }
}

/// Spawn a background thread that checks for updates.
/// Non-blocking — returns immediately. Prints to stderr if update available.
pub fn check_for_updates_async(current_version: &str) {
    if !should_check() {
        return;
    }

    let version = current_version.to_string();
    std::thread::Builder::new()
        .name("update-check".into())
        .spawn(move || {
            check_and_notify(&version);
        })
        .ok(); // silently fail if thread spawn fails
}

/// Detect current platform for the analytics ping.
fn platform_id() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    { "darwin-arm64" }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    { "darwin-x86_64" }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    { "linux-x86_64" }
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
    )))]
    { "other" }
}

/// The actual check — runs in background thread.
fn check_and_notify(current_version: &str) {
    // Send version + platform so the endpoint can track usage
    let url = format!("{}?v={}&p={}", UPDATE_CHECK_URL, current_version, platform_id());

    // 3 second timeout to avoid blocking on slow networks
    let output = std::process::Command::new("curl")
        .args([
            "-fsSL",
            "--max-time", "3",
            &url,
        ])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return, // silently fail on network errors
    };

    let body = String::from_utf8_lossy(&output.stdout);

    // Parse "latest" field from JSON response: {"latest": "0.3.2", ...}
    let latest = body
        .split("\"latest\"")
        .nth(1)
        .and_then(|s| s.split('"').nth(1));

    if let Some(latest_version) = latest {
        save_check_timestamp();
        if is_newer(current_version, latest_version) {
            eprintln!(
                "\n  New version available: {} → {}\n  Update: brew upgrade sentrux\n",
                current_version, latest_version
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("0.2.0"), Some((0, 2, 0)));
        assert_eq!(parse_version("v0.2.0"), Some((0, 2, 0)));
        assert_eq!(parse_version("1.10.3"), Some((1, 10, 3)));
        assert_eq!(parse_version("bad"), None);
    }

    #[test]
    fn test_is_newer() {
        assert!(is_newer("0.1.3", "0.2.0"));
        assert!(is_newer("0.2.0", "0.2.1"));
        assert!(is_newer("0.2.0", "1.0.0"));
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("0.2.0", "0.1.9"));
    }
}
