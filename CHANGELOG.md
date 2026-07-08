# Changelog

All notable changes to `yqr` are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and the project adheres to
[Semantic Versioning](https://semver.org/).

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
