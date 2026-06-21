---
name: cargo-quality
description: Run all Rust quality checks (fmt, clippy, test, bench). Use before committing or when validating code changes.
---

# Cargo Quality Checks

Run the full quality gate required by this project. The CI pipeline tests
three feature profiles — always run all three locally to catch
edition-specific failures before pushing.

## Full CI Mirror (use before every PR)

```bash
bash .github/scripts/local-ci.sh
```

This runs fmt + clippy + tests for all three profiles exactly as CI does:
- `default` (all features)
- `--no-default-features --features edition-core`
- `--features edition-pro`

## Individual Commands

```bash
# 1. Format code
cargo fmt

# 2. Clippy -- default features
cargo clippy -- -D warnings

# 3. Clippy -- edition-core (no optional features)
cargo clippy --no-default-features --features edition-core -- -D warnings

# 4. Clippy -- edition-pro (all features)
cargo clippy --features edition-pro -- -D warnings

# 5. Run the full test suite
cargo test

# 6. Compile benchmarks (catches benchmark build errors)
cargo bench --no-run

# 7. Build documentation
cargo doc --no-deps
```

## Why three clippy profiles?

`cargo clippy -- -D warnings` only runs with the default (all-features)
profile. Imports inside `#[cfg(feature = "...")]` blocks that are unused
when that feature is disabled only surface when compiling without that
feature. The `edition-core` profile is the strictest (fewest features) and
catches the most unused-import lint violations.

## Expected Results

- All commands must pass with exit code 0
- Clippy must produce zero warnings across all three profiles
- All tests must pass

## When to Use

- Before creating a commit
- Before opening a pull request
- After making any code changes
- When CI fails and you need to reproduce locally
