#!/usr/bin/env bash
# Local CI mirror for yqr. Runs the same gates as .github/workflows/ci.yml,
# serially, so a green run here means a green run there.
#
# Usage: bash .github/scripts/local-ci.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
START_TS=$(date +%s)

cd "$REPO_DIR"

echo "=== fmt ==="
cargo fmt --all -- --check

echo "=== clippy (-D warnings, all targets, all features) ==="
cargo clippy --all-targets --all-features -- -D warnings

echo "=== build ==="
cargo build --all-targets --locked

echo "=== test ==="
cargo test --all-targets --locked

# Feature-gated code (the fidelity engine backend) is invisible to the
# default-feature run; without this pass its tests never execute.
echo "=== test (all features) ==="
cargo test --all-targets --all-features --locked

# Compile the criterion benches without running them so a bench that references
# a renamed function fails here, not silently on a later perf run.
echo "=== bench (compile only) ==="
cargo bench --no-run

echo "=== doc ==="
cargo doc --no-deps

# What `cargo publish` would upload. ci.yml cannot catch this: a change that
# adds files under docs/ touches no Rust-relevant path, so CI never runs for
# it, and the crate silently grows. yqr shipped its whole website in 0.6.0
# this way (yqr-m004 s6).
echo "=== package contents ==="
if stray=$(cargo package --list --allow-dirty 2>/dev/null |
    grep -E '^(docs|specs|[.]github|[.]agent)/|^(CLAUDE|AGENT)[.]md$'); then
  echo "error: dev-only files would be published to crates.io:" >&2
  echo "$stray" | sed 's/^/  /' >&2
  echo "Add them to \`exclude\` in Cargo.toml." >&2
  exit 1
fi

# cargo audit is opt-in locally because the advisory DB fetch needs the network.
if command -v cargo-audit >/dev/null 2>&1; then
  echo "=== audit ==="
  cargo audit
else
  echo "=== audit skipped (cargo-audit not installed) ==="
fi

echo
echo "All CI gates passed in $(( $(date +%s) - START_TS ))s."
