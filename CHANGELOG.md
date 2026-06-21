# Changelog

All notable changes to `yqr` are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and the project adheres to
[Semantic Versioning](https://semver.org/).

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
