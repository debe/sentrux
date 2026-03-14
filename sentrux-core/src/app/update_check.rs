//! Non-blocking update checker + anonymous usage telemetry.
//!
//! Runs once per day (cached in ~/.sentrux/last_update_check). Does not block
//! the main thread — spawns a background thread. Collects the same data
//! that VS Code, Next.js, Homebrew, Cargo, and npm collect.
//!
//! ## Design (race-free)
//!
//! All telemetry state is protected by `TELEMETRY_LOCK` (a `Mutex`).
//! This eliminates three classes of race conditions:
//!
//! 1. **Concurrent `record_*` calls** — two threads calling `record_scan`
//!    simultaneously could produce a stale file write (last writer wins with
//!    a partial snapshot). The lock serializes file writes.
//!
//! 2. **`check_and_notify` vs `record_*`** — the ping thread could load the
//!    file, then a scan thread writes new data, then the ping thread clears
//!    the file — losing that scan. The lock makes load+clear atomic.
//!
//! 3. **Ping failure** — if the network call fails after we've already
//!    cleared counters, data is lost. We now snapshot-and-clear under lock
//!    BEFORE the network call, and restore on failure.
//!
//! The lock is held only during file I/O (microseconds), NEVER during
//! the network call (~3 seconds), so there is zero user-visible contention.
//!
//! Respects SENTRUX_NO_UPDATE_CHECK=1 to disable entirely.
//! Respects SENTRUX_DEV=1 to tag pings as internal/dev traffic.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Cloudflare Worker endpoint.
const UPDATE_CHECK_URL: &str = "https://api.sentrux.dev/version";

/// How often to send (24 hours).
const CHECK_INTERVAL: Duration = Duration::from_secs(86400);

// ── Telemetry state ──
//
// All access to the pending counters file AND the in-process counters
// goes through this lock. The lock is NEVER held during I/O that could
// block (network calls), only during fast file reads/writes.

static TELEMETRY_LOCK: Mutex<TelemetryState> = Mutex::new(TelemetryState::new());

struct TelemetryState {
    scans: u32,
    mcp_calls: u32,
    gate_runs: u32,
    files: u32,
    grade: u32,
}

impl TelemetryState {
    const fn new() -> Self {
        Self { scans: 0, mcp_calls: 0, gate_runs: 0, files: 0, grade: 0 }
    }

    /// Persist current counters to disk so they survive process exit.
    fn persist(&self) {
        let path = match pending_path() {
            Some(p) => p,
            None => return,
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = format!(
            "{{\"scans\":{},\"mcp_calls\":{},\"gate_runs\":{},\"files\":{},\"grade\":{}}}",
            self.scans, self.mcp_calls, self.gate_runs, self.files, self.grade,
        );
        let _ = std::fs::write(&path, json);
    }

    /// Take a snapshot of all counters and reset to zero.
    /// Also loads any values persisted from previous sessions and merges them.
    /// After this call, both in-memory state and disk file are zeroed.
    fn snapshot_and_reset(&mut self) -> TelemetrySnapshot {
        // Merge disk (previous sessions) + in-memory (current session)
        let disk = load_pending_from_disk();
        let snap = TelemetrySnapshot {
            scans: self.scans + disk.scans,
            mcp_calls: self.mcp_calls + disk.mcp_calls,
            gate_runs: self.gate_runs + disk.gate_runs,
            files: std::cmp::max(self.files, disk.files),
            grade: std::cmp::max(self.grade, disk.grade),
        };

        // Zero everything
        *self = Self::new();
        self.persist(); // write zeros to disk (don't delete — prevents race)

        snap
    }

    /// Restore a snapshot back into counters (used when ping fails).
    /// Merges with any new activity that happened during the ping attempt.
    fn restore(&mut self, snap: &TelemetrySnapshot) {
        self.scans += snap.scans;
        self.mcp_calls += snap.mcp_calls;
        self.gate_runs += snap.gate_runs;
        self.files = std::cmp::max(self.files, snap.files);
        self.grade = std::cmp::max(self.grade, snap.grade);
        self.persist();
    }
}

/// Immutable copy of counters for sending in the ping.
struct TelemetrySnapshot {
    scans: u32,
    mcp_calls: u32,
    gate_runs: u32,
    files: u32,
    grade: u32,
}

// ── Disk persistence helpers ──

fn pending_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".sentrux").join("telemetry_pending.json"))
}

/// Raw disk load — does NOT touch the in-memory state.
fn load_pending_from_disk() -> TelemetrySnapshot {
    let path = match pending_path() {
        Some(p) => p,
        None => return TelemetrySnapshot { scans: 0, mcp_calls: 0, gate_runs: 0, files: 0, grade: 0 },
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return TelemetrySnapshot { scans: 0, mcp_calls: 0, gate_runs: 0, files: 0, grade: 0 },
    };
    let get = |key: &str| -> u32 {
        content
            .split(key)
            .nth(1)
            .and_then(|s| {
                s.trim_start_matches(|c: char| c == '"' || c == ':' || c == ' ')
                    .split(|c: char| c == ',' || c == '}' || c == '\n')
                    .next()
            })
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    };
    TelemetrySnapshot {
        scans: get("\"scans\""),
        mcp_calls: get("\"mcp_calls\""),
        gate_runs: get("\"gate_runs\""),
        files: get("\"files\""),
        grade: get("\"grade\""),
    }
}

// ── Public recording API ──

/// Record a scan event (called from scanner).
pub fn record_scan(file_count: u32, grade: char) {
    let g = match grade {
        'A' => 1, 'B' => 2, 'C' => 3, 'D' => 4, 'E' => 5, 'F' => 6, _ => 0,
    };
    if let Ok(mut state) = TELEMETRY_LOCK.lock() {
        state.scans += 1;
        state.files = file_count;
        state.grade = g;
        state.persist();
    }
}

/// Record an MCP tool call.
pub fn record_mcp_call() {
    if let Ok(mut state) = TELEMETRY_LOCK.lock() {
        state.mcp_calls += 1;
        state.persist();
    }
}

/// Record a gate run.
pub fn record_gate_run() {
    if let Ok(mut state) = TELEMETRY_LOCK.lock() {
        state.gate_runs += 1;
        state.persist();
    }
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

/// Returns true if SENTRUX_DEV=1 is set (internal/dev traffic).
fn is_dev() -> bool {
    std::env::var("SENTRUX_DEV").is_ok_and(|v| v == "1")
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
///
/// Three-phase design to avoid races:
///   Phase 1 (lock held): snapshot all counters and reset to zero.
///   Phase 2 (lock released): send the HTTP ping (slow, ~3s timeout).
///   Phase 3 (lock held): on failure, restore the snapshot.
fn check_and_notify(current_version: &str) {
    // ── Phase 1: Atomic snapshot under lock ──
    let snapshot = match TELEMETRY_LOCK.lock() {
        Ok(mut state) => state.snapshot_and_reset(),
        Err(_) => return, // poisoned mutex, bail
    };
    // Lock released here — record_* calls are unblocked.

    let new = if is_new_user() { "1" } else { "0" };
    let mode = detect_mode();
    let plugins = crate::analysis::lang_registry::plugin_count();
    let tier = crate::license::current_tier();
    let dev = if is_dev() { "1" } else { "0" };
    let url = format!(
        "{}?v={}&p={}&new={}&m={}&pl={}&t={}&s={}&mc={}&g={}&f={}&gr={}&dev={}",
        UPDATE_CHECK_URL,
        current_version,
        platform_id(),
        new,                 // new user
        mode,                // gui/mcp/cli/plugin
        plugins,             // loaded plugin count
        tier,                // Free/Pro/Team
        snapshot.scans,      // scans since last ping
        snapshot.mcp_calls,  // MCP calls since last ping
        snapshot.gate_runs,  // gate runs since last ping
        snapshot.files,      // last scanned file count
        snapshot.grade,      // last health grade (1=A..6=F, 0=none)
        dev,                 // 1 = internal/dev traffic
    );

    // ── Phase 2: Network call (NO lock held) ──
    let output = std::process::Command::new("curl")
        .args(["-fsSL", "--max-time", "3", &url])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => {
            // ── Phase 3a: Ping failed — restore snapshot so nothing is lost ──
            if let Ok(mut state) = TELEMETRY_LOCK.lock() {
                state.restore(&snapshot);
            }
            return;
        }
    };

    let body = String::from_utf8_lossy(&output.stdout);
    let latest = body
        .split("\"latest\"")
        .nth(1)
        .and_then(|s| s.split('"').nth(1));

    match latest {
        Some(latest_version) => {
            // ── Phase 3b: Ping succeeded — counters already zeroed in Phase 1 ──
            save_check_timestamp();
            if is_newer(current_version, latest_version) {
                eprintln!(
                    "\n  New version available: {} → {}\n  Update: brew upgrade sentrux\n",
                    current_version, latest_version
                );
            }
        }
        None => {
            // Response was malformed — restore snapshot
            if let Ok(mut state) = TELEMETRY_LOCK.lock() {
                state.restore(&snapshot);
            }
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
    fn test_record_scan_increments() {
        // record_scan must increment scans and set files/grade
        record_scan(100, 'B');
        let state = TELEMETRY_LOCK.lock().unwrap();
        assert!(state.scans >= 1);
        // Can't assert exact value since tests share state,
        // but files/grade are set (not additive)
        drop(state);
    }

    #[test]
    fn test_platform_id() {
        let p = platform_id();
        assert!(!p.is_empty());
    }

    #[test]
    fn test_detect_mode() {
        let m = detect_mode();
        assert!(!m.is_empty());
    }

    #[test]
    fn test_pending_counters_parse() {
        // Test the JSON parsing logic directly using a temp file.
        let dir = std::env::temp_dir().join("sentrux_test_telemetry");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_pending.json");

        let json = r#"{"scans":42,"mcp_calls":17,"gate_runs":9,"files":250,"grade":4}"#;
        std::fs::write(&path, json).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let get = |key: &str| -> u32 {
            content
                .split(key)
                .nth(1)
                .and_then(|s| {
                    s.trim_start_matches(|c: char| c == '"' || c == ':' || c == ' ')
                        .split(|c: char| c == ',' || c == '}' || c == '\n')
                        .next()
                })
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0)
        };
        assert_eq!(get("\"scans\""), 42);
        assert_eq!(get("\"mcp_calls\""), 17);
        assert_eq!(get("\"gate_runs\""), 9);
        assert_eq!(get("\"files\""), 250);
        assert_eq!(get("\"grade\""), 4);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_snapshot_and_restore() {
        // Simulate: accumulate → snapshot → restore on failure
        // Use the real lock to verify no deadlocks.
        {
            let mut state = TELEMETRY_LOCK.lock().unwrap();
            state.scans += 100;
            state.mcp_calls += 50;
            let snap = state.snapshot_and_reset();
            assert!(snap.scans >= 100);
            assert!(snap.mcp_calls >= 50);
            // After reset, in-memory is zero
            assert_eq!(state.scans, 0);
            assert_eq!(state.mcp_calls, 0);
            // Restore on "failure"
            state.restore(&snap);
            assert!(state.scans >= 100);
            assert!(state.mcp_calls >= 50);
        }
    }

    #[test]
    fn test_is_dev_defaults_false() {
        if std::env::var("SENTRUX_DEV").is_err() {
            assert!(!is_dev());
        }
    }

    #[test]
    fn test_no_deadlock_concurrent_record() {
        // Verify that multiple record_* calls don't deadlock.
        // Each acquires and releases the lock independently.
        record_scan(50, 'A');
        record_mcp_call();
        record_gate_run();
        record_scan(60, 'C');
        // If we get here, no deadlock occurred.
    }
}
