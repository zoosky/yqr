# Implementation m001 — CI pipeline and release process

**Status:** In Progress (documents the shipped pipeline; §5 records the gaps)
**Owner:** yqr maintainers
**Last updated:** 2026-07-29
**Related:** `yqr-m004` (crates.io release posture), `yqr-m003` (the shared
corpus the test job runs), `yqr-f010` (the website workflow)

## 1. Purpose

`AGENT.md` points here for the details of continuous integration and
releases. This document is the **source of truth** for both: what runs, on
what trigger, on which runner, and what a human does by hand. It was written
after the v0.5.0 release found `AGENT.md`'s CI/CD section describing a
pipeline yqr never had (see §5).

Keep this file in sync when a workflow changes — the workflows themselves
are short enough to read, but the *manual* half of the release only exists
here.

## 2. Workflows

Three workflows live in `.github/workflows/`. There are no others.

### 2.1 `ci.yml` — build, test, lint

- **Triggers:** pushes to **any** branch and pull requests, both filtered to
  Rust-relevant paths (`**/*.rs`, `**/Cargo.toml`, `Cargo.lock`,
  `rust-toolchain.toml`, `ci.yml` itself). Filtering is GitHub's native
  `on.<event>.paths`; no filter action is involved. Markdown- and
  spec-only changes therefore skip CI entirely, which is why docs PRs show
  no `build · test · lint` check at all rather than a green one.
- **Runner:** `ubuntu-latest`. There is no self-hosted runner.
- **Jobs:** exactly one, `test` (displayed as `build · test · lint`), on the
  pinned 1.97.1 toolchain with a cargo registry/target cache:

  | Step | Command |
  |------|---------|
  | Format | `cargo fmt --all -- --check` |
  | Clippy | `cargo clippy --all-targets --all-features -- -D warnings` |
  | Build | `cargo build --all-targets --locked` |
  | Test | `cargo test --all-targets --locked` |
  | Test (all features) | `cargo test --all-targets --all-features --locked` |

  The two test passes are historical: they differed when the fidelity
  backends were feature-gated. yqr has had **no `[features]` section** since
  the single-engine consolidation (`yqr-m005`, `yqr-f011`), so the passes
  are currently equivalent. The second is kept as a guard for the day a
  feature returns.

### 2.2 `benchmark.yml` — continuous benchmarking

- **Triggers:** pushes to `main` only, Rust-path filtered. Never on PRs.
- **Runner:** `ubuntu-latest`.
- **What it runs:** `cargo bench --bench eval --locked -- --output-format
  bencher`. Only the `eval` target: the `corpus_bench` target
  (`yqr-m003`) is **not** tracked over time — it compiles in CI via
  `cargo bench --no-run` locally, but its timings are not stored.
- **Storage and alerting:** `benchmark-action/github-action-benchmark@v1`
  stores results on the `gh-pages` branch (published at `/dev/bench`) and
  alerts with a commit comment at `alert-threshold: 130%` — that is, a
  regression worse than 30% against the stored baseline.
- `gh-pages` is shared with the website deploy (`yqr-f010`), which is why
  `pages.yml` excludes `dev/` from its `rsync --delete`.

### 2.3 `pages.yml` — website

Builds the Accent CMS site from `docs/` (with `specs/` mounted at `/specs`)
and deploys it to `gh-pages`. Documented in `yqr-f010`; the accent binary is
pinned by `ACCENT_VERSION` and the build runs with `--strict-links`.

## 3. Release process

Releases are **manual**. No workflow reacts to tags — pushing `vX.Y.Z`
builds nothing and attaches nothing. Every step below is run by a human (or
an agent acting for one).

```bash
# 1. Update CHANGELOG.md: turn [Unreleased] into [X.Y.Z] - YYYY-MM-DD,
#    with a short lead paragraph in the style of the previous entries.
# 2. Bump `version` in Cargo.toml, and `softwareVersion` in the
#    SoftwareApplication JSON-LD (docs/themes/default/templates/home.html.jinja).
# 3. cargo check          # refreshes Cargo.lock's own yqr entry
# 4. bash .github/scripts/local-ci.sh   # full gate before tagging

git add CHANGELOG.md Cargo.toml Cargo.lock
git commit -m "chore: release vX.Y.Z"
# Ground rule 1 applies: this lands on main through a pull request.

# After the PR merges, from an up-to-date main:
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin vX.Y.Z
gh release create vX.Y.Z --title "vX.Y.Z" --notes-file <changelog-section>

# 5. Publish to crates.io (see yqr-m004):
cargo publish
```

Notes that only live here:

- **Version choice.** yqr is pre-1.0, so a breaking change — of the CLI
  *or* of the library API — bumps the **minor**. v0.5.0 was a minor because
  `--engine` was removed; a purely additive release is a patch.
- **Release notes** are the CHANGELOG section body, verbatim. Releases
  carry **no binary assets** (see §5.1), matching every release since
  v0.1.0.
- **`cargo publish` is irreversible** — a version can be yanked but never
  unpublished, and the version number can never be reused. It is a separate,
  explicitly-authorized step: an agent must not infer authorization for it
  from "cut the release".
- **After publishing**, confirm the version is live on crates.io and that
  docs.rs built it (a failed docs.rs build is invisible from the repo).

## 4. Local mirror

`bash .github/scripts/local-ci.sh` runs a superset of `ci.yml`: fmt, clippy,
build, test, test (all features), `cargo bench --no-run`, `cargo doc
--no-deps`, a package-contents check, and `cargo audit` when it is installed.
Running it before pushing is the cheapest way to avoid a red PR — `cargo bench
--no-run` in particular catches bench targets broken by a refactor, which
`cargo test` never compiles. The package-contents gate fails when `cargo
package --list` names a dev-only path; it can only live here, because the
change that trips it is a docs-only one that `ci.yml` skips by design
(`yqr-m004` §6).

## 5. Known gaps

### 5.1 No tag-triggered release workflow

Nothing builds or attaches binaries, so users install via `cargo install
yqr` or build from source; there is no `curl | sh` path and no
per-platform archive. Adding one would mean a `release.yml` triggered on
`v*` tags, cross-compiling the documented targets, attaching archives plus
checksums to the GitHub release, and (optionally) publishing to crates.io
from CI with a token. That is a feature, not a doc fix — it needs its own
`specs/features/` spec before implementation.

### 5.2 No automated PR review

`AGENT.md` previously described an automated PR-review workflow (an
`ANTHROPIC_API_KEY` secret, an agent posting review comments). No such
workflow has ever existed in this repository. Reviews are run on demand
from a developer's own tooling instead. The description has been removed
rather than reinstated; if an automated reviewer is wanted later, it starts
from a spec.

### 5.3 Benchmark coverage

Only the `eval` bench target is tracked over time; `corpus_bench` timings
are not stored, so a regression visible only in the corpus suite would not
alert.
