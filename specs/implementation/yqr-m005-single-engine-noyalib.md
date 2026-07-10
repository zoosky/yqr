# Implementation m005 — Single YAML engine: consolidation on noyalib

**Status:** In Progress — implemented and green (clippy `-D warnings`, 118 tests, benches compile); pending review/merge. yqr owns its `Value` (`src/value.rs`), so the engine is a swappable detail (§3).
**Owner:** yqr maintainers
**Last updated:** 2026-07-10
**Related:** `yqr-r002` (noyalib evaluation), `yqr-m002` (engine seam), `yqr-m004` (release posture — unblocked by this), `yqr-f002` (fidelity read floor), `yqr-f004` (engine parity — superseded), `yqr-f003` (backend A — retired), `yqr-b001`/`yqr-b003` (rust-yaml bugs — see §4), `yqr-b002`/`yqr-b004` (noyalib deficiencies/edit gaps), `yqr-a001` (surgical-edit north star)

## 1. Decision

yqr consolidates on **noyalib as its sole YAML engine**. The two rust-yaml
dependencies are removed entirely:

- `rust-yaml` (crates.io 1.1.0) — previously the classic pipeline's
  `Value` + load + dump.
- `rust-yaml-rt` (the `zoosky/rust-yaml` fork's `RoundTripDocument`, fidelity
  backend A, an unreleased git dependency).

noyalib provides the **parse/emit** for the classic pipeline and the **lossless
CST** behind the fidelity engine. yqr's **`Value` evaluation model is now its
own** (`src/value.rs`), independent of any YAML library — noyalib is converted
to `Value` at the parse/emit boundary, so the engine is a swappable
implementation detail (§3).

## 2. Rationale

A head-to-head assessment of the two backends (recorded against `yqr-r002`)
found noyalib the stronger fit for yqr's goal — surgical editing of large YAML:

- **Performance.** On a wide mapping, noyalib parse/round-trip is ~linear
  (~40 MB/s at 50 KB); rust-yaml's `RoundTripDocument` degrades
  super-linearly (~0.7 MB/s at 50 KB — roughly O(n²)), ~60× slower to parse and
  ~115× slower per edit at 50 KB. For "large YAML files" that is close to
  disqualifying.
- **Editing surface.** noyalib has first-class, re-parse-guarded mutators for
  value/add-key/remove-key/add-item (with indent/quote help); rust-yaml's
  `RoundTripDocument` exposes only value replacement. (Remaining noyalib edit
  gaps are tracked in `yqr-b004`.)
- **Architecture.** noyalib is a Rowan-style lossless CST with structural
  sharing — the right substrate for a structural editor.
- **Availability.** noyalib `0.0.14` is on crates.io; `RoundTripDocument` is
  merged upstream but **unreleased**, and was the sole remaining blocker for
  yqr's own crates.io publish (`yqr-m004`).

The countervailing risk — noyalib is pre-1.0 with high release churn — is a
maturity/process risk (contained by pinning), not a capability ceiling.

## 3. What changed (code)

- **Core model — yqr-owned `Value`.** yqr now defines its own value type
  (`src/value.rs`, `pub use value::Value`): a native enum
  (`Null`/`Bool`/`Int`/`Float`/`String`/`Sequence`/`Mapping`, with a
  `Value`-keyed insertion-ordered `IndexMap`) that does **not** re-export the
  parser's type. noyalib is converted **into** `Value` at parse
  (`From<noyalib::Value>`) and **back** at emit (`From<&Value> for
  noyalib::Value`). Because the shape matches the model the evaluator always
  used, `src/eval.rs` and the tests written against `Value` are **unchanged** by
  the engine switch — only the parse/emit boundary and test loaders touch
  noyalib.
- **Classic pipeline** (`src/lib.rs`): load via `noyalib::from_str::<Value>`,
  emit via `noyalib::to_string_value`.
- **Fidelity seam** (`src/fidelity/`): a single backend, `BackendId::NoyalibCst`.
  The adapter's cross-library lowering is gone — the evaluation model *is*
  noyalib's `Value`. `src/fidelity/rustyaml.rs` deleted.
- **Manifest**: `rust-yaml` and `rust-yaml-rt` removed; noyalib is a
  non-optional dependency; the `backend-noyalib` / `backend-rust-yaml` cargo
  features are removed (there is only one engine).
- **Tests/benches**: the multi-backend comparison harness (`tests/fidelity.rs`)
  is now single-backend; the shared corpus `Engine` enum has a single variant;
  `tests/fidelity_engine_rustyaml.rs` deleted.

## 4. Consequences

- **crates.io publish is unblocked (`yqr-m004`).** `Cargo.lock` has zero git
  dependencies and `cargo publish --dry-run` passes. The only remaining step is
  the maintainer's manual `cargo publish` (token-gated).
- **Engine parity (`yqr-f004`) is superseded.** There is no `--engine`
  choice between two backends; noyalib is the one fidelity engine. `--engine
  noyalib` still selects the byte-preserving path; other names are rejected.
- **Backend A (`yqr-f003`) is retired.** The rust-yaml fork's
  `RoundTripDocument` adapter and its tests are removed.
- **`yqr-b001` substrate change.** The classic (default) pipeline is still a
  *semantic* round trip that re-serializes and normalizes formatting — now via
  noyalib rather than rust-yaml. The byte-faithful path remains the fidelity
  engine. b001's characterization (the default pipeline is lossy) stands; only
  the substrate changed.
- **`yqr-b003` is moot.** It described a bug in the rust-yaml fork's
  `parse_all`; with that backend removed, it no longer affects yqr.
- **String-only mapping keys at the boundary.** yqr's own `Value` *can* hold
  non-string mapping keys, but noyalib's typed mapping cannot: on parse, a
  non-string key from noyalib arrives string-keyed (or raises `KeyCollision`,
  `yqr-b002` 2.5), and on emit a non-string `Value` key is written via its
  scalar spelling. A deliberate trade-off of a string-keyed engine.
- **Test coverage.** The rust-yaml lossy/faithful characterization pins were
  removed with their backends. Because the `Value` decoupling keeps the
  evaluator's API stable, the behavioral tests (`eval`, `integration`) are the
  originals — `tests/integration.rs` is byte-identical to pre-switch. A new
  `tests/golden_pipeline.rs` pins the classic pipeline's **exact rendered
  bytes** for type-sensitive inputs, so a future engine swap can't silently
  change behavior behind a parse-both-sides comparison. Suite: **118 tests
  green**.

## 5. Follow-ups

- The fidelity **write/edit tier** (`yqr-m002` §4/§6.2) now targets noyalib
  only; `yqr-b004` (noyalib CST mutation-API gaps) is its driver.

## 6. Engine plurality (resolved)

The `--engine` seam is **kept pluggable** rather than collapsed: `BackendId`
lists `NoyalibCst` (built-in, the default) and `Skald` (recognized by name).
"Don't put all eggs in one basket" — **skald** (elioetibr's from-scratch YAML
1.2.2 library, the rust-yaml successor) is the second candidate engine, wired as
a MINIMAL fidelity backend (identity byte-for-byte + typed eval; sub-path
projections degrade to `Synthetic`) on the **`feat/skald-engine`** branch.

skald is deliberately **kept off `main`**: it is an unreleased git dependency,
and crates.io rejects git deps even when optional (verified — `cargo publish`
fails with *"dependency `skald` does not specify a version"*). Wiring it into
`main` would re-block the very publish this consolidation unblocked (§4). So
`main` stays git-dep-free and publishable; on `main`, `--engine skald` resolves
by name but reports the backend is built only on the branch. When skald is
published to crates.io it folds back into `main` as
`skald = { version, optional = true }` (publishable and testable) and the branch
retires.
