# yqr - Agent Code Guidelines

## Ground Rules

**CRITICAL: These rules must ALWAYS be followed.**

1. **NEVER push directly to `main`** - All changes must go through a Pull Request
2. **Always create a feature branch first** - Use `git checkout -b feature/your-change` or `fix/your-fix`
3. **Run quality checks before committing** - `cargo fmt && cargo clippy -- -D warnings && cargo test`
4. **Create a PR for review** - Use `gh pr create` to submit changes
5. **Wait for CI and review** - PRs must pass CI and be reviewed before merging
6. No emojis in codebase
7. Refrain from purple hues in frontend
8. Always test code before deployment
9. **500-Line Rule**: Any Rust source file exceeding ~500 lines of production code must be split into a directory module (`mod.rs` + sub-modules) with `pub use` re-exports to preserve existing import paths. This keeps files agent-navigable, reduces merge conflicts in parallel worktrees, and enables clean `#[cfg(feature)]` gating. See `../accentcms/specs/implementation/m011-agent-friendly-project-structure.md` for rationale.
10. Never commit console.logs
11. **NEVER** add references to `the agent` or `Generated with the agent Code` or similar to the code base, commit messages, pull requests, or issue reports (including issue/bug specs under `specs/`). This includes `Co-Authored-By: the agent` trailers, `Generated with the agent Code` footers, and any "this was AI-assisted" attribution. The only place such mentions may legitimately appear is the `memory/` directory (auto-memory) and AGENT.md itself, which are addressed to or about the assistant.
12. **Literate Programming Principle**: All code must be self-documenting using Rust Doc comments (`///` and `//!`). Every module, struct, enum, trait, and public function must have doc comments that:
    - Explain the purpose and responsibility (the "why")
    - Provide usage examples where applicable
    - Document error conditions and edge cases
    - **No feature IDs in doc comments** (see rule 19)
13. **Feature Spec to Code Traceability**: When implementing a feature spec from `specs/features/`, add a `// Feature fNNN` code comment (not `///` or `//!`) near the item. The code should read like documentation of the feature, but feature IDs must never appear in doc comments (see rule 19).
14. Never ever start implementing a feature without a specs/feature spec unless you ask the user if you really should to this.
15. **Content Documentation**: When Accent CMS gains a new user-facing feature, **both** of the following are required:
    - **a) Usage guide**: Update the relevant documentation pages in `../accentcms/site-docs/content/` so users know the feature exists and how to use it:
      - `../accentcms/site-docs/content/04.docs/01.getting-started/default.md` - For setup/usage features
      - `../accentcms/site-docs/content/04.docs/02.markdown-guide/default.md` - For markdown/rendering features
      - `../accentcms/site-docs/content/04.docs/04.templating-guide/default.md` - For template-related features
      - Other relevant pages as appropriate
      - Keep documentation consistent with existing style and structure
    - **b) Demonstrating template**: A Jinja template in the default theme (`../accentcms/site-docs/themes/default/templates/`) must demonstrate the feature in action. This can be an existing template updated to use the new feature, or a new template if the feature requires one (e.g., `tags.html.jinja` for taxonomy routes). The template serves as both a real working example and a reference for theme developers.
16. **Issue tracking via `specs/`**: yqr spec files carry a `bo-` filename prefix and are referenced as `bo.fNNN` in prose, keeping them distinct from the sibling `accentcms/specs/` tree (which stays bare); see `specs/implementation/bo-m005-sigil-specs-namespace.md`. All implementation features and follow-ups live in `specs/features/bo-fNNN-...md` (registered in `specs/features/bo-f0000-feature-status.md` with the appropriate epic table, progress line, and summary totals). All bugs live in `specs/bugs/bo-bNNN-...md` (registered in `specs/bugs/bo-b000-bug-status.md`). Architecture / cross-cutting docs go in `specs/architecture/bo-aNNN-...md`; implementation/ops specs in `specs/implementation/bo-mNNN-...md`; research in `specs/research/bo-rNNN-...md`; marketing in `specs/marketing/bo-kNNN-...md`. Pick the next free identifier by listing the directory. Each spec carries a `**Status:**` field (Draft / In Progress / Done / Resolved / Superseded / Historical). The code-traceability comment in rule 13 keeps the bare `// Feature fNNN` form (it never crosses trees). **Do not** use TodoWrite, or scratch markdown files for task tracking.
17. **Feature Status Updates Before PR**: Prior to creating a pull request, you **must** update:
    - **a) The feature spec** (`specs/features/bo-fNNN-*.md`): Set `**Status:**` to `Done` and check off acceptance criteria for any feature completed by the PR.
    - **b) The status tracker** (`specs/features/bo-f0000-feature-status.md`): Update the feature's status in its epic table, the epic's progress line, and the summary totals at the bottom of the file.
    - This ensures the spec files and status tracker always reflect the true state of the codebase at the time code is merged.
18. **Implementation Specs (`specs/implementation/`)**: This folder contains system specifications, fact sheets, and non-functional requirements (e.g., port allocation, thread safety, extension points, license key management). These documents are the **source of truth** for cross-cutting concerns. When making changes that affect these specs, update the relevant document to stay in sync with the codebase. When adding a new cross-cutting concern or system-wide convention, create a new `bo-mNNN-*.md` file here.
19. **No Internal Spec References in User-Facing Output** (Feature f136): Feature IDs (`Feature fNNN`), spec paths (`specs/features/...`), and internal tracker references must **never** appear in:
    - **Rust doc comments** (`///` or `//!`) -- these render in `cargo doc` and `accent docs` output. Use plain `// Feature fNNN` code comments instead for traceability.
    - **Site documentation** (`../accentcms/site-docs/content/`, `../accentcms/site-dev/content/`) -- wrap in HTML comments (`<!-- Feature fNNN -->`) so they are invisible in rendered HTML but preserved for grep.
    - **CLI output** -- help text, error messages, and printed output must not contain feature IDs.
    - The `specs/` directory, `AGENT.md`, and `#[cfg(test)]` blocks are exempt (they are developer-only).
20. **Admit and stop when a URL is unreachable**: When a user provides a URL (research link, upstream repo, issue, doc page, etc.), **always actually fetch it** via `WebFetch`, `gh api`, or another appropriate tool before citing it. If the fetch fails (network error, 404, auth required, blocked by tool restrictions, redirect loop, etc.), **stop and tell the user explicitly** that the URL could not be accessed and ask how to proceed. Never fabricate content, version numbers, changelog entries, API shapes, or repository metadata from training data or inference. This applies to research docs, code comments, PR descriptions, and spec updates alike -- unverified claims about external sources are worse than a visible blocker.
21. **Vendored forked dependencies (side-by-side with `accentcms/`)**: Several upstream crates are vendored as editable git clones **parallel to the `accentcms/` checkout, not inside it**. Most live under `../dependencies/<crate>/`; the Mermaid renderer lives at `../accent-mmdr/` after the move to the private `zoosky/accent-mmdr` fork. Each is a fork we maintain so Accent can land hardening and contributions the upstream has not yet merged. Operating rules:
    - **Treat the fork as the canonical source.** Accent's `Cargo.toml` points at a pinned revision of the fork (via `path = "../dependencies/<crate>"` during development, or `git = "..."` with a rev-pin for release builds), never at crates.io for a forked crate.
    - **Never `git commit` inside a dep from the Accent session's working directory.** `cd` into the dep first so the commit lands in the correct repository. The two repos have separate `origin` remotes; crossing them is a common and silent mistake.
    - **Each dep has its own PR workflow** against `zoosky/<crate>`, not against the Accent repo. Upstream PRs (to `GitoxideLabs/gitoxide`, `1jehuang/mermaid-rs-renderer`, etc.) are proposed from the fork but are never release-gating -- we ship the fork either way.
    - **Rebase the fork against its upstream on a monthly cadence** to pull in improvements; cherry-pick trusted outside PRs from the upstream tracker into the fork when they are relevant (e.g. panic fixes).
    - **Sync PRs must use "Create a merge commit", never squash or rebase.** Squash- and rebase-merge destroy the upstream parent pointer and orphan the upstream commits in your fork's history (`git log main..upstream/main` keeps reporting the orphaned commits forever even though the patches are in the tree).
   
22. **Admit when a file is not accessible**: If a file is not accessible — over the web, on disk, or because its format cannot be read with the available tooling — admit it and ask the user for help. Never silently work around it (e.g. by installing tools unprompted or reconstructing the contents from inference). This extends rule 20 from URLs to files of every kind.

### Workflow for Every Change

```bash
# 1. Create a feature branch (NEVER work directly on main)
git checkout -b feature/my-change

# 2. Make your changes and run quality checks
cargo fmt && cargo clippy -- -D warnings && cargo test

# 3. Commit changes
git add . && git commit -m "Description of change"

# 4. Push to feature branch
git push -u origin feature/my-change

# 5. Create PR (NEVER push to main directly)
gh pr create --title "My change" --body "Description"
```

### Session Completion (Landing the Plane)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create a `specs/features/bo-fNNN-...md` or `specs/bugs/bo-bNNN-...md` for anything that needs follow-up (see rule 16)
2. **Run quality gates** (if code changed) - `cargo fmt && cargo clippy -- -D warnings && cargo test`
3. **Update spec status** - Mark finished specs as Done/Resolved; update in-progress items in `bo-f0000-feature-status.md` / `bo-b000-bug-status.md`
4. **PUSH TO REMOTE** - This is MANDATORY:

   ```bash
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```

5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**

- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

## Project Overview

accent is a high-performance markdown CMS written in Rust. It serves markdown files as HTML with on-the-fly rendering, caching, and Jinja2-style templating (via MiniJinja). Inspired by Grav CMS theming concepts.

For accent's product feature set (tracked in the sibling `accentcms/` tree, not here), run `accent --help` (agent-friendly, lists every command and feature) and consult `../accentcms/specs/features/f0000-feature-status.md` as the source of truth. To map a feature **name** to its `fNNN` spec ID (or back), see [`../accentcms/specs/features/f0000a-feature-id-mapping.md`](../accentcms/specs/features/f0000a-feature-id-mapping.md). To map the **CLI option tree** to editions and feature specs, see [`../accentcms/specs/features/f0000b-cli-feature-mapping.md`](../accentcms/specs/features/f0000b-cli-feature-mapping.md) (generated by `../accentcms/tools/cli-tree/`).

### Running the Server

Todo

## Code Quality Requirements

### Before Every Change

All code changes **must** pass the following checks:

```bash
# 1. Format code
cargo fmt

# 2. Run clippy with strict settings (must pass with no warnings)
cargo clippy -- -D warnings

# 3. Run the full test suite
cargo test

# 4. Compile-check the bench targets. `cargo test` does NOT build benches;
#    a bench that references a moved field or renamed function will only
#    surface in accentcms's benchmark.yml on main without this step (see accentcms f177).
cargo bench --no-run

# 5. Run benchmarks to catch performance regressions (perf-sensitive PRs only)
cargo bench --bench admin --bench cache --bench content --bench diagrams --bench e2e --bench markdown --bench media --bench template
```

### Clippy Configuration

The project enforces strict clippy lints. See `Cargo.toml` for the full configuration. Key requirements:

- No warnings allowed (`-D warnings`)
- Pedantic lints enabled where practical
- Security-related lints enforced

### Testing Requirements

- **Unit tests**: Every module must have inline unit tests (`#[cfg(test)]`)
- **Integration tests**: Located in `tests/` directory
- **Coverage target**: Aim for >80% code coverage
- **Property-based tests**: Use `proptest` for complex logic where applicable

### Benchmarking Requirements

- Benchmarks live in `../accentcms/benches/` directory using `criterion`

## Project Structure

Use `tree` tool.

Important annotated directories:

Todo

## Module Organization

### Core Principles

1. **Separation of concerns**: Each module has a single responsibility
2. **Public API in lib.rs**: Export only what's needed for library users
3. **Error handling**: Use `thiserror` for custom error types, propagate with `?`
4. **Async-first**: Use `async`/`await` throughout for I/O operations

### Module Dependencies

```
main.rs -> cli.rs (Clap dispatch)



## Development Workflow

### Adding a New Feature

1. Create or update the feature spec in `specs/features/`
2. Write failing tests first (TDD approach encouraged)
3. Implement the feature
4. Run the quality checks listed in "Before Every Change" above (steps 1-4; add step 5 for perf-sensitive features)
5. Update documentation if public API changes

### Fixing a Bug

1. Write a test that reproduces the bug
2. Fix the bug
3. Verify the test passes
4. Run full quality checks

### Performance Work

1. Add or update benchmarks in `../accentcms/benches/`
2. Establish baseline: run the full bench suite (step 5 of "Before Every Change") with `-- --save-baseline before` appended
3. Make changes
4. Compare: same suite with `-- --baseline before` appended
5. Only merge if no regressions (or regressions are justified)

## Coding Conventions

### Error Handling

```rust
// Use thiserror for error types
#[derive(Debug, thiserror::Error)]
pub enum ContentError {
    #[error("page not found: {0}")]
    NotFound(String),

    #[error("invalid frontmatter: {0}")]
    InvalidFrontmatter(#[from] serde_yaml::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

// Use Result type alias
pub type Result<T> = std::result::Result<T, ContentError>;
```

### Async Code

```rust
// Prefer async functions
pub async fn load_page(path: &Path) -> Result<Page> {
    let content = tokio::fs::read_to_string(path).await?;
    // ...
}
```

### Async Handler Guard Safety

`state.config()` returns a `std::sync::RwLockReadGuard` which is `!Send`. Holding it across an `.await` makes the handler future `!Send`, which axum rejects with an opaque `Handler<_, _> not satisfied` error. **Always** read config into owned values inside a block scope:

```rust
let (max_bytes, content_dir) = {
    let cfg = state.config();
    (cfg.admin.media.max_upload_mb * 1_048_576, cfg.content.directory.clone())
};
// Guard dead here. Safe to .await below.
let index = state.get_all_pages().await;
```

Do not rely on `drop(cfg)` -- a block scope is unambiguous. Same rule applies to `state.plugins.read()` and any other `std::sync` guard. See `../accentcms/specs/implementation/m030-async-handler-guard-safety.md` for diagnosis techniques.

### Testing

```rust
// Inline unit tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        // ...
    }

    #[tokio::test]
    async fn test_async_operation() {
        // ...
    }
}
```

## Crate Documentation Lookup

Use this when you need to look up API signatures, types, or usage examples
for any Rust crate used in this project. Prefer local docs in `target/doc-md/`
over training data or web lookups -- they match the exact versions in `Cargo.lock`.

### Looking Up Documentation

Docs are organized as one directory per crate with Markdown files per module:

```
target/doc-md/
  index.md                    # main index of all crates
  axum/index.md               # Crate root docs
  axum/routing.md             # axum::routing module
  tokio/sync/index.md         # tokio::sync module
  serde_json/index.md         # serde_json crate root
```

To find docs for a crate, read `target/doc-md/<crate_name>/index.md`.
For a specific module, read `target/doc-md/<crate_name>/<module>.md`.
Hyphens in crate names become underscores in directory names (e.g., `tower-http` -> `tower_http`).

### Regenerating Docs

Docs should be regenerated when `Cargo.lock` is newer than `target/doc-md/index.md`,
which means dependencies were updated.

```bash
# Full regeneration (all dependencies, including private items)
cargo +nightly doc-md --include-private

# Targeted regeneration (specific crates, faster)
cargo +nightly doc-md --include-private -p <crate1> -p <crate2>

# First-time setup (if cargo-doc-md is not installed)
rustup install nightly
cargo +nightly install cargo-doc-md
```

### Key Crates in This Project

Todo

## Dependencies Policy

- Prefer well-maintained, minimal-dependency crates
- Security-audit dependencies with `cargo audit`
- Pin major versions in `Cargo.toml`
- Document why each dependency is needed

## CI/CD Expectations

The following should pass in CI:

```yaml
 ../accentcms/.github/scripts/local-ci-fast.sh 
- cargo doc --no-deps   # Documentation builds
```

## GitHub Actions Workflows

This project uses automated CI/CD pipelines to maintain code quality, especially important for multi-agent development where multiple AGENT instances may be working concurrently.

### CI Pipeline (`.github/workflows/ci.yml`)

**Triggers**: All pull requests and pushes to `main`

**Job sequence** — security runs first and gates all other jobs:

```
security ──┐
           ├── check (if Rust files changed)
changes  ──┘
           └── editions (if Rust files changed)
```

**Runner policy**:

| Event | `security` + `changes` | `check` + `editions` |
|-------|------------------------|----------------------|
| Pull request | `[self-hosted, linux, x64, rust]` | `[self-hosted, linux, x64, rust]` |
| Push to main | `ubuntu-latest` | `[self-hosted, linux, x64, rust]` |

All jobs on PRs run on the self-hosted LAN runner to avoid GitHub-hosted costs.

See `../accentcms/specs/implementation/m014-ci-release-process.md` for full details.

### Continuous Benchmarking (`../accentcms/.github/workflows/benchmark.yml`)

**Triggers**: Push to `main` only (Rust files changed). Does **not** run on PRs.

**What it does**:

- Runs the 8 regression-gated Criterion benchmark suites (admin, cache, content, diagrams, e2e, markdown, media, template) with `--output-format bencher` on the self-hosted runner
- Stores results in `gh-pages` branch as baseline via `benchmark-action/github-action-benchmark@v1`
- Alerts on >30% regressions against the stored baseline

### Release Process

Releases are triggered by pushing a semver tag. The full checklist:

```bash
# 1. Update CHANGELOG.md — add [X.Y.Z] - YYYY-MM-DD section
# 2. Bump version in Cargo.toml
# 3. cargo check  (updates Cargo.lock)
git add CHANGELOG.md Cargo.toml Cargo.lock
git commit -m "chore: release vX.Y.Z"
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push && git push origin vX.Y.Z
gh release create vX.Y.Z --title "vX.Y.Z" --notes "..."
```



### AGENT PR Review

**Triggers**: When PRs are opened, updated, or reopened

**What it does**:

- Automatically reviews pull requests using agent Sonnet 4.5
- Reads this agent.md file to understand project guidelines
- Analyzes the PR diff for:
  - Summary of changes
  - Code quality assessment
  - Potential bugs, performance issues, or security concerns
  - Suggestions for improvement
  - Recommendation (approve/request changes/reject)
- Posts detailed review as a PR comment
- Handles large PRs (>100KB) gracefully with a warning

**Setup Required**:
The agent PR review workflow requires an Anthropic API key configured as a GitHub secret:

1. Go to repository **Settings → Secrets and variables → Actions**
2. Add a new secret:
   - Name: `ANTHROPIC_API_KEY`
   - Value: Your Anthropic API key from <https://console.anthropic.com/>

**For Multi-Agent Development**:

- Each PR gets automatically reviewed by the agent, ensuring consistency across agents
- CI must pass before merging - all agents' code must meet the same quality standards
- The automated review catches issues early, reducing back-and-forth
- PR reviews provide learning feedback for future the agent instances

### Working with the Pipelines

**Before creating a PR**:

- Run the full local CI mirror to catch all edition-specific failures before pushing:

  ```bash
  bash .github/scripts/local-ci.sh        # serial, ~15 min on a 28-core box
  bash ../accentcms/.github/scripts/local-ci-fast.sh   # parallel (accentcms f178), ~5 min cargo phase
  ```

  Both run fmt, clippy, and tests for all three feature profiles (`default`, `edition-core`, `edition-pro`) exactly as CI does. The fast variant runs the three editions concurrently using per-profile `CARGO_TARGET_DIR=target{,-core,-pro}` and uses `cargo nextest run` for the test phase; it needs `cargo-nextest` installed and ~25 GB extra disk for the auxiliary target dirs.
- **CRITICAL**: `cargo clippy -- -D warnings` alone is not sufficient. It only runs with the default (all-features) profile. Unused imports inside `#[cfg(feature = "...")]` blocks only show up when that feature is disabled. Always run all three profiles.
- **Docs/specs-only PRs**: CI and benchmarks auto-skip when only `.md` or spec files change (path filtering via `dorny/paths-filter`). No need to run `cargo` checks for markdown-only changes

**When CI fails**:

- Click on the failed job in GitHub Actions to see detailed logs
- Fix the issues locally and push again
- CI will automatically re-run on new commits

**Reviewing the agent's feedback**:

- The automated the agent review is advisory - use your judgment
- It's based on the guidelines in this file, so keeping the agent.md updated improves reviews
- the agent may miss context that you have - that's okay

**Updating workflows**:

- Workflow files are in `.github/workflows/`
- Test workflow changes in a feature branch first
- Changes to workflows also trigger CI validation

## CLI Quick Reference (cargo run)

# Generate agent documentation

cargo run -- docs agent-readme
cargo run -- docs the agent-md

```



