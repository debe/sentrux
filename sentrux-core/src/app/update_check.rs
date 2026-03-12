//! Non-blocking update checker + anonymous usage telemetry.
//!
//! Runs once per day (cached in ~/.sentrux/last_update_check). Does not block
//! the main thread — spawns a background thread. Collects the same data
//! that VS Code, Next.js, Homebrew, Cargo, and npm collect.
//!
//! Respects SENTRUX_NO_UPDATE_CHECK=1 to disable entirely.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Cloudflare Worker endpoint.
const UPDATE_CHECK_URL: &str = "https://api.sentrux.dev/version";

/// How often to send (24 hours).
const CHECK_INTERVAL: Duration = Duration::from_secs(86400);

// ── Stats accumulator (atomic counters, lock-free) ──

/// Total scans since last ping
static SCANS: AtomicU32 = AtomicU32::new(0);
/// Total MCP tool calls since last ping
static MCP_CALLS: AtomicU32 = AtomicU32::new(0);
/// Total gate runs since last ping
static GATE_RUNS: AtomicU32 = AtomicU32::new(0);
/// Last health grade (A=1, B=2, ... F=6, 0=none)
static LAST_GRADE: AtomicU32 = AtomicU32::new(0);
/// Last file count scanned
static LAST_FILES: AtomicU32 = AtomicU32::new(0);

/// Record a scan event (called from scanner).
pub fn record_scan(file_count: u32, grade: char) {
    SCANS.fetch_add(1, Ordering::Relaxed);
    LAST_FILES.store(file_count, Ordering::Relaxed);
    let g = match grade {
        'A' => 1, 'B' => 2, 'C' => 3, 'D' => 4, 'E' => 5, 'F' => 6, _ => 0,
    };
    LAST_GRADE.store(g, Ordering::Relaxed);
}

/// Record an MCP tool call.
pub fn record_mcp_call() {
    MCP_CALLS.fetch_add(1, Ordering::Relaxed);
}

/// Record a gate run.
pub fn record_gate_run() {
    GATE_RUNS.fetch_add(1, Ordering::Relaxed);
}

// ── Cache ──

fn cache_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".sentrux").join("last_update_check"))
}

fn should_check() -> bool {
    if std::env::var("SENTRUX_NO_UPDATE_CHECK").is_ok() {
        return false;
    }
    let path = match cache_path() {
        Some(p) => p,
        None => return true,
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
        Err(_) => true,
    }
}

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

// ── Version check ──

fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let v = v.strip_prefix('v').unwrap_or(v);
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() != 3 { return None; }
    Some((parts[0].parse().ok()?, parts[1].parse().ok()?, parts[2].parse().ok()?))
}

fn is_newer(current: &str, latest: &str) -> bool {
    match (parse_version(current), parse_version(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => false,
    }
}

// ── Platform ──

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

/// Detect how sentrux was launched.
fn detect_mode() -> &'static str {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--mcp") { "mcp" }
    else if args.iter().any(|a| a == "check" || a == "gate") { "cli" }
    else if args.iter().any(|a| a == "plugin") { "plugin" }
    else { "gui" }
}

/// Check if this is the first ever ping (new user).
fn is_new_user() -> bool {
    cache_path().is_some_and(|p| !p.exists())
}

// ── Public API ──

/// Spawn a background thread that sends the daily ping.
pub fn check_for_updates_async(current_version: &str) {
    if !should_check() {
        return;
    }
    let version = current_version.to_string();
    std::thread::Builder::new()
        .name("update-check".into())
        .spawn(move || { check_and_notify(&version); })
        .ok();
}

/// The daily ping — sends version + platform + usage stats.
fn check_and_notify(current_version: &str) {
    let new = if is_new_user() { "1" } else { "0" };
    let mode = detect_mode();
    let plugins = crate::analysis::lang_registry::plugin_count();
    let tier = crate::license::current_tier();
    let scans = SCANS.load(Ordering::Relaxed);
    let mcp = MCP_CALLS.load(Ordering::Relaxed);
    let gates = GATE_RUNS.load(Ordering::Relaxed);
    let url = format!(
        "{}?v={}&p={}&new={}&m={}&pl={}&t={}&s={}&mc={}&g={}",
        UPDATE_CHECK_URL,
        current_version,
        platform_id(),
        new,           // new user
        mode,          // gui/mcp/cli/plugin
        plugins,       // loaded plugin count
        tier,          // Free/Pro/Team
        scans,         // scans since last ping
        mcp,           // MCP calls since last ping
        gates,         // gate runs since last ping
    );

    let output = std::process::Command::new("curl")
        .args(["-fsSL", "--max-time", "3", &url])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return,
    };

    let body = String::from_utf8_lossy(&output.stdout);
    let latest = body
        .split("\"latest\"")
        .nth(1)
        .and_then(|s| s.split('"').nth(1));

    if let Some(latest_version) = latest {
        save_check_timestamp();
        // Reset counters after successful ping
        SCANS.store(0, Ordering::Relaxed);
        MCP_CALLS.store(0, Ordering::Relaxed);
        GATE_RUNS.store(0, Ordering::Relaxed);
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

    #[test]
    fn test_record_scan() {
        record_scan(100, 'B');
        assert_eq!(LAST_FILES.load(Ordering::Relaxed), 100);
        assert_eq!(LAST_GRADE.load(Ordering::Relaxed), 2);
        assert!(SCANS.load(Ordering::Relaxed) >= 1);
    }

    #[test]
    fn test_platform_id() {
        let p = platform_id();
        assert!(!p.is_empty());
    }

    #[test]
    fn test_detect_mode() {
        // In test context, no --mcp or check args
        let m = detect_mode();
        assert!(!m.is_empty());
    }
}
