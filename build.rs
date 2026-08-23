//! Build script for yqr.
//!
//! Captures the git commit hash, build timestamp, and target triple and
//! exposes them as compile-time environment variables (`GIT_HASH`,
//! `BUILD_TIME`, `BUILD_TARGET`) consumed by the `--version` output in
//! `src/cli.rs`.

use std::process::Command;

fn main() {
    let git_hash = git_hash();
    println!("cargo:rustc-env=GIT_HASH={git_hash}");

    // Build timestamp (UTC), computed in pure Rust to avoid a chrono dependency.
    let build_time = {
        use std::time::SystemTime;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let secs_per_day = 86_400u64;
        let days = now / secs_per_day;
        let day_secs = now % secs_per_day;
        let hours = day_secs / 3600;
        let minutes = (day_secs % 3600) / 60;
        let seconds = day_secs % 60;
        // Safe: days since epoch fits in i64 for any realistic timestamp.
        #[allow(clippy::cast_possible_wrap)]
        let (y, m, d) = civil_from_days(days as i64);
        format!("{y:04}-{m:02}-{d:02} {hours:02}:{minutes:02}:{seconds:02} UTC")
    };
    println!("cargo:rustc-env=BUILD_TIME={build_time}");

    // Target triple (set by Cargo for the build script).
    let target = std::env::var("TARGET").unwrap_or_default();
    println!("cargo:rustc-env=BUILD_TARGET={target}");

    // Re-run when the commit or lockfile changes so the stamped info stays
    // fresh without rebuilding on every unrelated source edit.
    println!("cargo:rerun-if-changed=Cargo.lock");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/");
    println!("cargo:rerun-if-changed=.cargo_vcs_info.json");
}

/// The short commit this build came from, by whichever route is available.
///
/// Two, because a crate is built in two different places. In a checkout `git`
/// answers. From a published tarball there is no `.git`, but Cargo wrote the
/// commit into `.cargo_vcs_info.json` when it packaged the crate, so the
/// answer is still on disk — which is why an installed `yqr` can name its
/// commit at all rather than shrugging.
///
/// `"unknown"` only when neither exists: a source tree that is not a checkout
/// and was not packaged by Cargo.
// Bug b023.
fn git_hash() -> String {
    head_via_git()
        .or_else(head_via_vcs_info)
        .unwrap_or_else(|| "unknown".to_string())
}

/// Ask `git`, in a checkout.
///
/// The status check is the whole point. Outside a repository `git rev-parse`
/// exits 128 with its complaint on *stderr* and an empty stdout, so `output()`
/// is `Ok` and the string is `""` — which is how `--version` came to print an
/// empty pair of parentheses instead of falling back (`yqr-b023`).
fn head_via_git() -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let hash = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!hash.is_empty()).then_some(hash)
}

/// Read the commit out of the manifest Cargo packages into a published crate.
///
/// The file is Cargo's own, three keys deep and machine-written, so it is
/// scanned rather than parsed — a JSON dependency for one field would be paid
/// for by every downstream build.
fn head_via_vcs_info() -> Option<String> {
    let text = std::fs::read_to_string(".cargo_vcs_info.json").ok()?;
    let rest = text.split_once("\"sha1\"")?.1;
    let rest = rest.split_once('"')?.1;
    let sha = rest.split_once('"')?.0;
    // Match the width `git rev-parse --short` prints, so the two routes are
    // indistinguishable in `--version` output.
    let short: String = sha.chars().take(7).collect();
    (short.len() == 7 && short.chars().all(|c| c.is_ascii_hexdigit())).then_some(short)
}

/// Converts days since the Unix epoch to a `(year, month, day)` triple.
///
/// Uses Howard Hinnant's `civil_from_days` algorithm.
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
