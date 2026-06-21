//! Build script for yqr.
//!
//! Captures git commit hash, build timestamp, and edition profile 
//! for version info.

use std::process::Command;

fn main() {
    // Git commit hash (short)
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string());

    println!("cargo:rustc-env=GIT_HASH={git_hash}");

    // Build timestamp (UTC) -- pure Rust for cross-platform compatibility (Feature f072b)
    let build_time = {
        use std::time::SystemTime;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let secs_per_day = 86400u64;
        let days = now / secs_per_day;
        let day_secs = now % secs_per_day;
        let hours = day_secs / 3600;
        let minutes = (day_secs % 3600) / 60;
        let seconds = day_secs % 60;
        // Safe: days since epoch fits in i64 for any realistic timestamp
        #[allow(clippy::cast_possible_wrap)]
        let (y, m, d) = civil_from_days(days as i64);
        format!("{y:04}-{m:02}-{d:02} {hours:02}:{minutes:02}:{seconds:02} UTC")
    };

    println!("cargo:rustc-env=BUILD_TIME={build_time}");

    // Target triple (set by cargo)
    let target = std::env::var("TARGET").unwrap_or_default();
    println!("cargo:rustc-env=BUILD_TARGET={target}");

    // Edition detection (Feature f078).
    // Cargo features are not available via cfg!() in build scripts, so we
    // check the CARGO_FEATURE_* environment variables that Cargo sets.
    let edition = if std::env::var("CARGO_FEATURE_EDITION_PRO").is_ok() {
        "pro"
    } else if std::env::var("CARGO_FEATURE_EDITION_STANDARD").is_ok() {
        "standard"
    } else {
        "core"
    };

    // Expose the mermaid-rs-renderer version as MMDR_VERSION for
    // version_guard::check_mmdr_version() (Feature f160a).
     println!("cargo:rerun-if-changed=Cargo.lock");

    // Rebuild if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/");
}

/// Read mermaid-rs-renderer's version from the vendored dev tree, if present.
///


/// Read a named package's version from Cargo.lock.
///
/// Cargo.lock always records the resolved version of every dependency,
/// regardless of whether the dep is declared as a path, git, or crates.io
/// source. This is the CI-compatible path -- CI checks out only the
/// yqr repo, but Cargo.lock is committed.
fn read_lockfile_version(package_name: &str) -> Option<String> {
    let contents = std::fs::read_to_string("Cargo.lock").ok()?;
    let target_name_line = format!("name = \"{package_name}\"");
    let mut in_target = false;
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed == "[[package]]" {
            in_target = false;
            continue;
        }
        if in_target {
            if let Some(rest) = trimmed.strip_prefix("version = ") {
                return Some(rest.trim_matches('"').to_string());
            }
        } else if trimmed == target_name_line {
            in_target = true;
        }
    }
    None
}

/// Converts days since Unix epoch to (year, month, day).
///
/// Uses the `civil_from_days` algorithm by Howard Hinnant.
/// Reference: <https://howardhinnant.github.io/date_algorithms.html>
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = i64::from(yoe) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
