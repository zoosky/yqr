---
name: dep-upgrade
description: Upgrade Rust dependencies one at a time using cargo outdated. Analyzes impact, checks for API changes, and produces a recommendation before upgrading.
---

# Dependency Upgrade Skill

Safely upgrade Rust dependencies one crate at a time with full impact analysis.

## Workflow

### Step 1: Scan for outdated dependencies

```bash
cargo outdated -R
```

This shows direct dependencies only (`-R` excludes transitive). The output columns:

- **Project**: current version in Cargo.toml
- **Compat**: latest compatible version (same major for 1.x+, same minor for 0.x)
- **Latest**: absolute latest version
- **Kind**: Normal (runtime) or Development (dev-dependency)

### Step 2: Categorize upgrades

Split the outdated list into three categories:

1. **Patch/minor compatible** (`Compat` column has a version): Safe to upgrade via
   `cargo update <crate>` without changing Cargo.toml. These rarely break anything.

2. **Major version bump** (`Compat` is `---`): Requires editing Cargo.toml to change
   the version requirement. These may have breaking API changes.

3. **Dev-dependencies** (`Kind` = Development): Lower risk since they don't affect
   the published crate. Still verify tests pass.

### Step 3: Upgrade ONE crate at a time

For each crate, follow this exact sequence:

#### 3a. Compatible (patch/minor) upgrades

```bash
# Update Cargo.lock without changing Cargo.toml
cargo update <crate>

# Verify it compiles and tests pass
cargo fmt && cargo clippy -- -D warnings && cargo test
```

#### 3b. Major version upgrades (breaking changes)

```bash
# 1. Read the changelog/release notes for the new version
#    Look for: breaking changes, removed APIs, renamed types, new required features

# 2. Check what depends on the crate in our code
grep -r "<crate_name>" src/ --include="*.rs" -l

# 3. Generate local docs for the NEW version to understand API changes
#    (after updating Cargo.toml)

# 4. Edit Cargo.toml to the new version
#    e.g., change `rust-yaml = "1.1"` to `rust-yaml = "1.2"`

# 5. Update the lock file
cargo update <crate>

# 6. Try to compile - fix any breaking changes
cargo check

# 7. If compilation fails, analyze the errors and fix imports/API usage

# 8. Run the full quality gate
cargo fmt && cargo clippy -- -D warnings && cargo test

# 9. If the upgrade is too disruptive, ROLLBACK:
git checkout Cargo.toml Cargo.lock
```

### Step 4: Analyze impact

For each upgrade, assess:

- **API breakage**: Did any public types, traits, or function signatures change?
- **Feature flags**: Did default features change? Do we need to add/remove features?
- **Transitive deps**: Did the upgrade pull in new transitive dependencies?
  Check with `cargo tree -i <crate>` before and after.
- **Binary size**: For release builds, significant dependency changes may affect size.
- **MSRV**: Does the new version require a newer Rust toolchain?

### Step 5: Report

After processing all upgrades, produce a summary table:

```
| Crate      | From   | To     | Type    | Status    | Notes                        |
|------------|--------|--------|---------|-----------|------------------------------|
| clap       | 4.6.1  | 4.6.3  | patch   | Upgraded  | No API changes               |
| rust-yaml  | 1.1.0  | 1.2.0  | minor   | Upgraded  | Additive, no breakage        |
| criterion  | 0.5    | 0.8    | major   | Upgraded  | dev-dep; bench API unchanged |
```

## Rules

- **ONE crate at a time**: Never batch upgrades. If something breaks, you need to
  know which crate caused it.
- **Commit after each successful upgrade**: Each upgrade gets its own commit so it
  can be reverted independently.
- **Prefer compatible upgrades first**: Do all patch/minor bumps before attempting
  major version upgrades.
- **Rollback on failure**: If a major upgrade causes too many breaking changes,
  revert and document why it was deferred.
- **Normal deps before dev-deps**: Upgrade runtime dependencies first, then
  dev-dependencies.
- **Check the changelog**: Always read release notes for major bumps. Never
  blindly upgrade.

## When to Use

- Periodic dependency maintenance (weekly/monthly)
- Before a release to pick up security patches
- When `cargo audit` reports a vulnerability in an outdated dependency
- When a dependency upgrade is needed for a new feature
