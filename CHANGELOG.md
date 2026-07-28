# Changelog

All notable changes to `yqr` are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and the project adheres to
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **`yqr validate [--strict] [FILES]...` -- YAML correctness checking with
  compiler-style diagnostics.** yqr's first subcommand closes the editing
  loop: after a surgical, hand-made, or agent-made edit, one command answers
  whether a file is still correct YAML. A pass certifies that every document
  parses *and* that the parsed documents reproduce the input byte-for-byte
  (the fidelity invariant). Failures are rustc-style diagnostics on stderr
  with stable codes (`Y001` syntax, `Y002` stream integrity, `Y101`
  duplicate key under `--strict`, `Y102` stringified-key collision), a
  `file:line:column` location where the parser reports one, the offending
  source line with a caret, and a suggested fix -- unresolved merge-conflict
  markers get a dedicated hint. Exit codes: 0 all inputs valid, 1 validation
  findings, 5 an input could not be read (highest wins; every input is
  checked in one run). The library gains the `validate` module
  (`check_str` / `render`).

### Removed

- **The `--engine` flag is gone.** yqr has settled on noyalib as its one and
  only YAML engine, so there is no backend to select: byte-preserving reads
  and surgical edits always run on noyalib's lossless CST. `--engine noyalib`
  now fails argument parsing instead of being accepted as a no-op; drop the
  flag from any invocation that used it (the behavior is unchanged without
  it). The experimental `skald` backend is retired with the seam: its name is
  no longer recognized anywhere.
- The library API lost `fidelity::BackendId`; `fidelity::open`,
  `fidelity::run`, `fidelity::run_ast`, and `fidelity::write::apply` no
  longer take a backend argument.

### Changed

- The pinned Rust toolchain was updated from 1.97 to 1.97.1 (point release;
  no MSRV change -- `rust-version` stays 1.97).

## [0.4.0] - 2026-07-11

The fidelity write tier arrives: surgical, byte-preserving edits that change only
the bytes a filter targets and leave every other byte -- comments, indentation,
quoting, key order -- untouched, or refuse. In the same release, byte-preserving
reads become the default and the classic re-serializing pipeline moves behind
`--normalize`.

### Added

- **Write tier: surgical value edits.** yqr can now mutate a document through the
  fidelity engine, changing only the targeted bytes: assignment `.a.b = <rhs>`
  (scalar literal or a `.`-rooted path), append `.xs += <item>`, new-key assign
  `.a.new = <rhs>`, and `del(.a.b)`. Each edit passes through the engine's
  re-parse guard -- an edit that would restructure the document is refused
  (exit 5) rather than emitted, and scalar writes are quoted to match the
  neighbouring style. A filter is either a read-only query or a single mutation;
  mixing them is a parse error.
- **`-i` / `--in-place` flag** writes the mutated document back to the input file
  atomically (temp file + rename, `fsync` before rename, symlinks followed,
  owner-only temp permissions). Using `-i` with stdin or with a read-only filter
  is an error, diagnosed before any input is read. Without `-i`, the mutated
  document is printed to stdout (byte-exact except the edit).
- **Structural delete** of multi-line and nested block entries (e.g.
  `del(.spec.template)`), which the single-line delete path rejects. Flow-style
  and sole-entry deletes remain refused with a clear message.
- **`--normalize` / `-N` flag** for the classic re-serializing pipeline: it
  drops comments and canonicalizes scalars (e.g. `007` becomes `7`) -- the
  previous default behaviour.

### Changed

- **Byte-preserving reads are now the default.** `yqr '.' file.yaml` reproduces
  the input byte-for-byte -- comments, quoting, indentation, scalar spellings,
  and line endings survive -- with no flag. Untouched nodes are emitted as their
  original source bytes; computed, absent, and unaddressable nodes fall back to
  typed rendering per node.
- **`--engine` now selects the backend for the default (byte-preserving) read.**
  Under `--normalize` the classic pipeline runs and the engine choice has no
  observable effect beyond the up-front name validation.

### Breaking

- **`--preserve` / `-p` removed.** Byte preservation is now the default, so the
  flag is gone. Replace `yqr -p '.' f` with `yqr '.' f`; use
  `yqr --normalize '.' f` for the old re-serializing behaviour.

### Security

- Bumped the transitive `crossbeam-epoch` pin `0.9.18 -> 0.9.20` to clear
  RUSTSEC-2026-0204. It reaches the build only through the `criterion`
  dev-dependency (benchmarks), so released binaries were never affected; the
  change is lockfile-only.

## [0.3.0] - 2026-07-10

Byte/comment preservation becomes its own flag, decoupled from backend
selection.

### Added

- **`--preserve` / `-p` flag** for byte/comment-preserving reads. It turns on
  fidelity mode with the default backend, so `yqr -p '.' file.yaml` reproduces
  the input byte-for-byte — comments, quoting, indentation, and line endings
  survive.
- A runnable demo showcase under `docs/content/demo/` (`yqr-demo.sh` plus sample
  `deploy.yaml` / `config.yaml` inputs) walking through navigation, iteration,
  pipes, raw output, and preserve mode.

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
