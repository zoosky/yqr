#!/usr/bin/env bash
# Local CI mirror for borches. Runs exactly the gates .github/workflows/ci.yml
# runs, serially, so a green run here means a green run there. Scaled-down
# sibling of accentcms/.github/scripts/local-ci-fast.sh — this workspace has a
# single profile (no editions), so there is nothing to parallelize yet.
#
# Usage: bash .github/scripts/local-ci.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
START_TS=$(date +%s)

cd "$REPO_DIR/accent"

echo "=== fmt ==="
cargo fmt --all -- --check

echo "=== clippy (-D warnings, all targets) ==="
cargo clippy --workspace --all-targets -- -D warnings

echo "=== test ==="
cargo test --workspace

# Compile the T6 criterion benches without running them: a bench that
# references a moved field or renamed function fails here, not silently on a
# perf run. cargo test does not build benches; --all-targets clippy does, but
# this keeps the guarantee explicit and independent of the lint pass.
echo "=== bench (compile only) ==="
cargo bench --workspace --no-run

echo "=== doc ==="
# --workspace: without it, default-members limits rustdoc to accent-server.
cargo doc --workspace --no-deps

# cargo audit runs as its own CI job; locally it is opt-in because the
# advisory DB fetch needs the network (the test suite itself must pass
# air-gapped, a005 §6).
if command -v cargo-audit >/dev/null 2>&1; then
  echo "=== audit ==="
  cargo audit
else
  echo "=== audit skipped (cargo-audit not installed) ==="
fi

bash "$SCRIPT_DIR/check_agent_isolation.sh"

echo
echo "All CI gates passed in $(( $(date +%s) - START_TS ))s."
