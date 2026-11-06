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

# Features f046-f048: the lib must lint, test and compile without the CLI
# and JSON Schema dependency graphs (and in the schema-only editor profile),
# and clap, jsonschema, serde_json and serde must stay out of the minimal graph
# entirely -- a later change that re-couples the lib to a gated dependency
# fails here, not at a consumer. `cargo tree` writes to a file as its own
# command: under `set -e` a tool failure stops the script instead of handing
# grep empty input, and grep never SIGPIPEs the producer. grep's exit code is
# inspected explicitly so a grep *error* (exit 2) fails the gate rather than
# reading as "no match".
echo "=== lib without default features ==="
cargo clippy --lib --no-default-features --locked -- -D warnings
cargo test --lib --no-default-features --locked
cargo check --lib --no-default-features --features schema --locked
lib_deps=$(mktemp)
trap 'rm -f "$lib_deps"' EXIT
cargo tree --no-default-features --locked -e normal --prefix none > "$lib_deps"
rc=0
grep -E '^(clap|jsonschema|serde_json|serde) ' "$lib_deps" || rc=$?
if [ "$rc" -eq 0 ]; then
  echo "error: a gated dependency is in the no-default-features graph" >&2
  exit 1
elif [ "$rc" -ne 1 ]; then
  echo "error: the dependency guard's grep failed (exit $rc)" >&2
  exit 1
fi

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
