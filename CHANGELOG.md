# Changelog

All notable changes to `yqr` are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and the project adheres to
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **`--preserve` / `-p` flag** for byte/comment-preserving reads. It turns on
  fidelity mode with the default backend, so `yqr -p '.' file.yaml` reproduces
  the input byte-for-byte — comments, quoting, indentation, and line endings
  survive.

### Changed

- **`--engine` now selects the backend parser only** and no longer implies
  preservation. It picks *which* library performs a `--preserve` read (default
  `noyalib`); *whether* to preserve is `--preserve`'s job. Without `--preserve`,
  `--engine` has no observable effect.

### Breaking

- `--engine noyalib '.'` no longer preserves bytes on its own — use
  `--preserve` (optionally with `--engine noyalib` to name the backend
  explicitly). This decouples backend choice from fidelity mode.

## [0.2.1] - 2026-07-10

The first crates.io release. yqr consolidates on a single YAML engine, which
removes every git dependency and makes the crate publishable.

### Changed

- **One YAML engine.** yqr now uses noyalib for both the standard pipeline and
  byte-preserving reads. `--engine noyalib` (the fidelity engine) still emits
  untouched nodes as their original source bytes — comments, quoting,
  indentation, and line endings survive, and the identity filter reproduces the
  input byte-for-byte. It is always built in, so there are no backend build
  features to toggle.
- yqr now has its own value type instead of re-exporting the YAML library's, so
  the parser is a swappable internal detail. Library users of `yqr::Value` get a
  stable type that does not change when the engine does.
- Minimum supported Rust version is now 1.97 (the pinned toolchain was updated
  from 1.96).
- **Non-string mapping keys.** Integer, boolean, and composite keys (`1:`,
  `? [a, b]:`) are preserved byte-for-byte via `--engine noyalib`, but the
  standard re-serializing pipeline now renders them as strings, and a document
  that mixes keys colliding as strings (`1` and `"1"`) is rejected rather than
  kept distinct. This is rare in typical Kubernetes/CI/config documents — and
  GitHub Actions' `on:` is unaffected (it stays the string `on`).

### Removed

- The `rust-yaml` fidelity backend and the `--engine rust-yaml` option, along
  with the `backend-noyalib` / `backend-rust-yaml` build features and the
  `--no-default-features` minimal build. The fidelity engine is now always
  compiled in.

### Notes

- First release published to crates.io — install with `cargo install yqr`. (The
  0.2.0 tag below was a GitHub-only release that predates this consolidation.)
- `--engine` remains pluggable; an experimental `skald` engine is recognized for
  future comparison but is not built into the released binary.

## [0.2.0] - 2026-07-08

### Added

- Fidelity engines for byte-preserving reads, selectable at runtime with the
  new `--engine` flag. With `--engine noyalib` or `--engine rust-yaml`,
  untouched nodes are emitted as their original source bytes — comments,
  quoting, indentation, and line endings survive, and the identity filter
  reproduces the input byte-for-byte.
- Both fidelity backends now ship in the default build, so `--engine noyalib`
  and `--engine rust-yaml` are switchable in one binary without recompiling.
  Build with `--no-default-features` for a minimal binary that carries neither
  backend (the standard re-serializing pipeline still works; `--engine` then
  reports the backend as unavailable).
- A backend-agnostic fidelity round-trip harness (`tests/fidelity.rs`) that
  checks the `parse -> emit == input` property across backends, one case per
  formatting dimension (comments, blank lines, indent, quote/block/flow style,
  CRLF, BOM, multi-doc, anchors, numbers, key order).
- A shared real-world corpus (`tests/corpus/`) driving both the validation
  suite and the Criterion benchmarks from one case table (Kubernetes, GitHub
  Actions, Docker Compose, Helm, application config).
- Kubernetes usage guide in the documentation.

### Changed

- An unknown `--engine` value is now diagnosed before any input is read, so a
  typo is reported immediately instead of after consuming stdin or the file.

## [0.1.1] - 2026-06-21

### Changed

- Packaging only: dev-only files (`.agent/`, `.github/`, `specs/`, `AGENT.md`)
  are now excluded from the published crate. No functional or API changes —
  the compiled code is identical to 0.1.0; the source tarball is just slimmer
  (21 files vs 36).

This release supersedes 0.1.0, which has been yanked from crates.io.

## [0.1.0] - 2026-06-21

### Added

- Initial release (M0 foundation): a jq-style processor for YAML, operating
  natively on YAML via `rust-yaml` (no JSON round-trip).
- Filters: identity `.`, field access (`.foo`, `.a.b`, `.["k"]`), array
  indexing (`.[n]`, negative from end), iteration (`.[]`), pipe (`a | b`),
  and optional error suppression (`f?`).
- CLI with `--raw-output`, file/stdin input, and jq-style exit codes.
- `--version` reports the git commit, build timestamp, and target triple.
