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

# Compile the criterion benches without running them so a bench that references
# a renamed function fails here, not silently on a later perf run.
echo "=== bench (compile only) ==="
cargo bench --no-run

echo "=== doc ==="
cargo doc --no-deps

# cargo audit is opt-in locally because the advisory DB fetch needs the network.
if command -v cargo-audit >/dev/null 2>&1; then
  echo "=== audit ==="
  cargo audit
else
  echo "=== audit skipped (cargo-audit not installed) ==="
fi

echo
echo "All CI gates passed in $(( $(date +%s) - START_TS ))s."
