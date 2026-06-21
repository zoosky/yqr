---
name: cargo-quality
description: Run all Rust quality checks (fmt, clippy, test, bench). Use before committing or when validating code changes.
---

# Cargo Quality Checks

Run the full quality gate required by this project.

## Full CI Mirror (use before every PR)

```bash
bash .github/scripts/local-ci.sh
```

This runs the same gates as `.github/workflows/ci.yml`: fmt, clippy, build,
test, a bench compile-check, and `cargo doc`.

## Individual Commands

```bash
# 1. Format code
cargo fmt --all -- --check

# 2. Clippy (all targets, all features, warnings denied)
cargo clippy --all-targets --all-features -- -D warnings

# 3. Run the full test suite
cargo test --all-targets --locked

# 4. Compile benchmarks (catches benchmark build errors)
cargo bench --no-run

# 5. Build documentation
cargo doc --no-deps
```

## Expected Results

- All commands must pass with exit code 0
- Clippy must produce zero warnings
- All tests must pass

## When to Use

- Before creating a commit
- Before opening a pull request
- After making any code changes
- When CI fails and you need to reproduce locally
