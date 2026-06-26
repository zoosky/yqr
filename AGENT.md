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
9. **500-Line Rule**: Any Rust source file exceeding ~500 lines of production code must be split into a directory module (`mod.rs` + sub-modules) with `pub use` re-exports to preserve existing import paths. This keeps files agent-navigable, reduces merge conflicts in parallel worktrees, and enables clean `#[cfg(feature)]` gating.
10. Never commit console.logs
11. **NEVER** add references to `the agent` or `Generated with the agent Code` or similar to the code base, commit messages, pull requests, or issue reports (including issue/bug specs under `specs/`). This includes `Co-Authored-By: the agent` trailers, `Generated with the agent Code` footers, and any "this was AI-assisted" attribution. The only place such mentions may legitimately appear is the `memory/` directory (auto-memory) and AGENT.md itself, which are addressed to or about the assistant.
12. **Literate Programming Principle**: All code must be self-documenting using Rust Doc comments (`///` and `//!`). Every module, struct, enum, trait, and public function must have doc comments that:
    - Explain the purpose and responsibility (the "why")
    - Provide usage examples where applicable
    - Document error conditions and edge cases
    - **No feature IDs in doc comments** (see rule 19)
13. **Feature Spec to Code Traceability**: When implementing a feature spec from `specs/features/`, add a `// Feature fNNN` code comment (not `///` or `//!`) near the item. The code should read like documentation of the feature, but feature IDs must never appear in doc comments (see rule 19).
14. Never ever start implementing a feature without a specs/feature spec unless you ask the user if you really should to this.
15. **Content Documentation**: When yqr gains a new user-facing feature, **both** of the following are required:
    - **a) Usage guide**: Update the relevant documentation pages in `docs/content/` so users know the feature exists and how to use it:
      - Other relevant pages as appropriate
      - Keep documentation consistent with existing style and structure

16. **Issue tracking via `specs/`**:

- yqr spec files filename prefix and are referenced as `yqr.fNNN` in prose,
- All bugs live in `specs/bugs/yqr-bNNN-...md`
- Architecture / cross-cutting docs go in `specs/architecture/yqr-aNNN-...md`; implementation/ops specs in `specs/implementation/yqr-mNNN-...md`; research in `specs/research/yqr-rNNN-...md`; marketing in `specs/marketing/yqr-kNNN-...md`. Pick the next free identifier by listing the directory. Each spec carries a `**Status:**` field (Draft / In Progress / Done / Resolved / Superseded / Historical). The code-traceability comment in rule 13 keeps the bare `// Feature fNNN` form (it never crosses trees). **Do not** use TodoWrite, or scratch markdown files for task tracking.

17. **Feature Status Updates Before PR**: Prior to creating a pull request, you **must** update:
    - **a) The feature spec** (`specs/features/yqr-fNNN-*.md`): Set `**Status:**` to `Done` and check off acceptance criteria for any feature completed by the PR.
    - **b) The status tracker** (`specs/features/yqr-f0000-feature-status.md`): Update the feature's status in its epic table, the epic's progress line, and the summary totals at the bottom of the file.
    - This ensures the spec files and status tracker always reflect the true state of the codebase at the time code is merged.
18. **Implementation Specs (`specs/implementation/`)**: This folder contains system specifications, fact sheets, and non-functional requirements (e.g., port allocation, thread safety, extension points, license key management). These documents are the **source of truth** for cross-cutting concerns. When making changes that affect these specs, update the relevant document to stay in sync with the codebase. When adding a new cross-cutting concern or system-wide convention, create a new `yqr-mNNN-*.md` file here.
19. **No Internal Spec References in User-Facing Output** (Feature f136): Feature IDs (`Feature fNNN`), spec paths (`specs/features/...`), and internal tracker references must **never** appear in:
    - **Rust doc comments** (`///` or `//!`) -- these render in `cargo doc` output. Use plain `// Feature fNNN` code comments instead for traceability.
    - **Site documentation** (`docs/content/`) -- wrap in HTML comments (`<!-- Feature fNNN -->`) so they are invisible in rendered HTML but preserved for grep.
    - **CLI output** -- help text, error messages, and printed output must not contain feature IDs.
    - The `specs/` directory, `AGENT.md`, and `#[cfg(test)]` blocks are exempt (they are developer-only).
20. **Admit and stop when a URL is unreachable**: When a user provides a URL (research link, upstream repo, issue, doc page, etc.), **always actually fetch it** via `WebFetch`, `gh api`, or another appropriate tool before citing it. If the fetch fails (network error, 404, auth required, blocked by tool restrictions, redirect loop, etc.), **stop and tell the user explicitly** that the URL could not be accessed and ask how to proceed. Never fabricate content, version numbers, changelog entries, API shapes, or repository metadata from training data or inference. This applies to research docs, code comments, PR descriptions, and spec updates alike -- unverified claims about external sources are worse than a visible blocker.

21. **Admit when a file is not accessible**: If a file is not accessible — over the web, on disk, or because its format cannot be read with the available tooling — admit it and ask the user for help. Never silently work around it (e.g. by installing tools unprompted or reconstructing the contents from inference). This extends rule 20 from URLs to files of every kind.

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

1. **File issues for remaining work** - Create a `specs/features/yqr-fNNN-...md` or `specs/bugs/yqr-bNNN-...md` for anything that needs follow-up (see rule 16)
2. **Run quality gates** (if code changed) - `cargo fmt && cargo clippy -- -D warnings && cargo test`
3. **Update spec status** - Mark finished specs as Done/Resolved; update in-progress items in `yqr-f0000-feature-status.md` / `yqr-b000-bug-status.md`
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

yqr is a high-performance, large YAML file query & transformation tool written in Rust. 

### Running the CLI

```bash
cargo run -- '<filter>' [file.yaml]   # omit the file to read YAML from stdin
```

## Agent Toolkit (`.agent/`)

Project-specific agent configuration, reusable skills, and hooks live in `.agent/`:

- **`.agent/skills/`** — invokable skills for common workflows:
  - `cargo-quality` — run the full quality gate (fmt, clippy, test, bench)
  - `cargo-doc` — look up crate docs in Markdown from `target/doc-md/` (see "Crate Documentation Lookup" below)
  - `benchmark` — run and analyze the criterion benchmarks
  - `dep-upgrade` — upgrade dependencies one at a time with impact analysis
  - `security-audit` — audit dependencies and review for vulnerabilities
  - `pr-prepare` — quality checks, commit, and PR creation
- **`.agent/commands/codereview.md`** — multi-agent pull-request code review.
- **`.agent/hooks/notify-sound.sh`** — `Stop`-hook chime (macOS) for when the agent needs input.
- **`.agent/settings.json`** — shared permissions, env vars, and hooks. Per-machine overrides belong in `.agent/settings.local.json`, which is git-ignored and must never be committed.

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
#    surface in yqr's benchmark.yml on main without this step.
cargo bench --no-run

# 5. Run benchmarks to catch performance regressions (perf-sensitive PRs only)
cargo bench
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
- **Fidelity harness** (`tests/fidelity.rs`): A backend-agnostic round-trip
  harness that checks the a001 byte-for-byte property (`parse -> emit == input`)
  across YAML backend libraries, one case per b001 formatting dimension
  (comments, blank lines, indent, quote/block/flow style, CRLF, BOM, multi-doc,
  anchors, numbers, key order). It pins bug b001 (the shipped `rust-yaml` path is
  lossy) and research r002 (the optional `noyalib` CST round-trips byte-for-byte).
  Add a backend by implementing the `Backend` trait and registering it in
  `backends()`.
  - Default run (rust-yaml only, minimal deps):
    `cargo test --test fidelity -- --nocapture`
  - Include the experimental `noyalib` CST backend (gated, off by default):
    `cargo test --test fidelity --features backend-noyalib -- --nocapture --test-threads=1`

### Benchmarking Requirements

- Benchmarks live in `benches/` directory using `criterion`

## Project Structure

Run `tree -I 'target|.git'` for the live layout. The key files:

```
yqr/
├── src/
│   ├── main.rs        # Binary entry; maps results to jq-style exit codes
│   ├── cli.rs         # clap (derive) args + --version strings
│   ├── lib.rs         # Public API: eval_str, render, re-exports
│   ├── error.rs       # YqrError enum + exit-code mapping
│   ├── lexer.rs       # Filter source -> Token stream
│   ├── ast.rs         # Filter AST node definitions
│   ├── parser.rs      # Recursive-descent Tokens -> Ast
│   └── eval.rs        # Evaluator: Ast x Value -> stream of Values
├── benches/eval.rs    # Criterion benchmarks (parse, end-to-end eval_str)
├── tests/
│   ├── cli.rs         # Black-box tests of the compiled binary
│   └── integration.rs # Library end-to-end tests via the public API
├── build.rs           # Stamps git hash / build time / target into --version
├── specs/features/    # Feature specs (yqr.fNNN-*.md)
├── .agent/            # Agent toolkit: skills, command, hook, settings
├── .github/
│   ├── workflows/     # ci.yml, benchmark.yml
│   └── scripts/       # local-ci.sh (local CI mirror)
├── Cargo.toml
├── rust-toolchain.toml  # Pins the 1.96 toolchain
├── AGENT.md
└── README.md
```

## Module Organization

### Core Principles

1. **Separation of concerns**: Each module has a single responsibility
2. **Public API in lib.rs**: Export only what's needed for library users
3. **Error handling**: one crate-wide error enum (`YqrError`) with a `Result` alias; propagate with `?`
4. **Keep it synchronous**: yqr is a small synchronous CLI — do not introduce an async runtime (`tokio`/`async`) unless a feature genuinely requires it

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

1. Add or update benchmarks in `benches/`
2. Establish baseline: run the full bench suite (step 5 of "Before Every Change") with `-- --save-baseline before` appended
3. Make changes
4. Compare: same suite with `-- --baseline before` appended
5. Only merge if no regressions (or regressions are justified)

## Coding Conventions

### Error Handling

yqr uses a single hand-rolled error enum (`src/error.rs`) with a `Result` alias
and an `exit_code()` mapping — keep dependencies minimal rather than pulling in
`thiserror`.

```rust
pub type Result<T> = std::result::Result<T, YqrError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YqrError {
    Lex(String),
    Parse(String),
    Eval(String),
    Io(String),
}

impl YqrError {
    /// jq-style process exit code for this error category.
    pub fn exit_code(&self) -> i32 {
        match self {
            YqrError::Lex(_) | YqrError::Parse(_) => 3,
            YqrError::Eval(_) | YqrError::Io(_) => 5,
        }
    }
}
```

### Testing

```rust
// Inline unit tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_field_access() {
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
  clap/index.md               # Crate root docs
  clap/builder.md             # clap::builder module
  rust_yaml/index.md          # rust_yaml crate root
  criterion/index.md          # criterion crate root
```

To find docs for a crate, read `target/doc-md/<crate_name>/index.md`.
For a specific module, read `target/doc-md/<crate_name>/<module>.md`.
Hyphens in crate names become underscores in directory names (e.g., `rust-yaml` -> `rust_yaml`).

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

| Crate | Purpose | Doc Path |
|-------|---------|----------|
| clap | CLI argument parser (derive feature) | `target/doc-md/clap/` |
| rust-yaml | YAML parsing and emission (`Value` model) | `target/doc-md/rust_yaml/` |
| criterion | Benchmark harness (dev-dependency) | `target/doc-md/criterion/` |

## Dependencies Policy

- Prefer well-maintained, minimal-dependency crates
- Security-audit dependencies with `cargo audit`
- Pin major versions in `Cargo.toml`
- Document why each dependency is needed

## CI/CD Expectations

The following should pass in CI:

```yaml
bash .github/scripts/local-ci.sh   # fmt, clippy, build, test, bench compile, doc
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

| Event | `security` + `changes` | `check`  |
|-------|------------------------|----------------------|
| Pull request | `[self-hosted, linux, x64, rust]` | `[self-hosted, linux, x64, rust]` |
| Push to main | `ubuntu-latest` | `[self-hosted, linux, x64, rust]` |

All jobs on PRs run on the self-hosted LAN runner to avoid GitHub-hosted costs.

See `/specs/implementation/yqr-m001-ci-release-process.md` for full details.

### Continuous Benchmarking (`.github/workflows/benchmark.yml`)

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
  bash .github/scripts/local-ci.sh
  ```

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
cargo run -- docs agent-md

```



