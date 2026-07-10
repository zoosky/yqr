# Feature f004 — Engine parity: both fidelity backends default-on and runtime-switchable, from the zoosky forks

**Status:** Done
**Owner:** yqr maintainers
**Last updated:** 2026-07-08
**Implements:** `yqr-a001` §4 (fidelity engine seam), consumes `yqr-b002` 2.2-2.7
**Related:** `yqr-f002` (the seam + noyalib backend), `yqr-f003` (backend A over
the rust-yaml fork), `yqr-b002` (the noyalib fixes this consumes), `yqr-m002`
§7.2 (the noyalib adapter)

## 1. Summary

Bring the two fidelity engines to **parity** and make `--engine` a genuine
runtime switch:

- **Both backends ship in the default build.** `default = ["backend-noyalib",
  "backend-rust-yaml"]` in `Cargo.toml`, so a single `cargo build` produces one
  binary that answers both `--engine noyalib` and `--engine rust-yaml` with no
  rebuild. `--no-default-features` still yields a minimal binary that carries
  neither backend (the classic re-serializing pipeline is unchanged, and
  `--engine` reports the backend as unavailable).
- **Engine sourcing.** `rust-yaml-rt` tracks `zoosky/rust-yaml`
  `feat/roundtrip-document` (f003). `noyalib` originally tracked
  `zoosky/noyalib` `feat/fidelity-span-fixes` — an integration branch off the
  0.0.13 release carrying the submitted upstream fixes for deficiencies 2.2-2.7
  (`yqr-b002`) — so yqr could ship the fixed fidelity engine ahead of an
  upstream release. Once those fixes shipped in **noyalib 0.0.14**, yqr re-pinned
  to the crates.io release (`noyalib = "0.0.14"`, 2026-07-10; see `yqr-m004`).
- **The noyalib adapter consumes the fixes.** The guards that previously
  degraded to `Synthetic` now resolve to `Found` where the fork made the span
  faithful, and the redundant cross-check is removed (see §3).

No CLI surface changes: `--engine <name>` and its diagnostics are exactly as
`f002`/`f003` shipped them; the only observable change is that both names now
work in the stock binary and the noyalib path emits more nodes verbatim.

## 2. Why default-on

`f002`/`f003` gated each backend off by default to keep the stock build
dependency-minimal. That made `--engine` a *build-time* choice: a binary built
without a feature answered the flag with "not available in this build". Parity
and runtime switching require both backends compiled into one artifact, so the
default feature set now enables both. Users who want the minimal build opt out
with `--no-default-features`; the classic pipeline never depended on either
backend and is untouched either way.

## 3. Adapter deltas consumed from the noyalib fork (b002 2.2-2.7)

The noyalib adapter (`src/fidelity/noyalib.rs`) re-verifies every candidate
slice by re-parsing it against the typed value, so adopting the fixed engine can
only upgrade `Synthetic → Found`, never emit a wrong node. The observable
changes:

- **2.3 keep-chomped block scalars** — `|+` / `>+` spans now include the kept
  trailing blank lines, so the slice re-parses to the full value and is emitted
  verbatim (`Found`) instead of degrading.
- **2.6 alias resolve-through** — an alias reference resolves to the anchor
  value's span, so `*anc` emits the anchor's original bytes (`Found`).
- **2.4 block-collection line-start** — a nested block collection's span starts
  at its first line's indent; the adapter's line-start extension becomes a
  fallback rather than the routine path.
- **2.5 key-collision diagnostic** — the fork's loader raises
  `Error::KeyCollision` on the `parse_stream` path, so distinct string-colliding
  keys are refused at parse time. The adapter's rust-yaml entry-count
  cross-check in `open()` (and its `entry_counts_diverge` helper) is removed as
  redundant.
- **2.2 implicit-null → no span** and **2.7 lone-CR line break** — `span_at`
  returns no span for byte-less implicit nulls (still `Synthetic`), and a
  classic-Mac CR-only stream now scans and round-trips.

## 4. Acceptance criteria

- [x] `default` feature set enables both `backend-noyalib` and
      `backend-rust-yaml`; `cargo build` (no flags) produces a binary where both
      `--engine noyalib` and `--engine rust-yaml` work at runtime.
- [x] `--no-default-features` builds cleanly; `--engine <name>` reports the
      backend unavailable; the classic pipeline is unchanged.
- [x] `noyalib` is pinned to a source carrying b002 2.2-2.7 (at f004:
      `zoosky/noyalib` `feat/fidelity-span-fixes`; re-pinned to the crates.io
      `noyalib = "0.0.14"` release on 2026-07-10, see `yqr-m004`);
      `rust-yaml-rt` remains pinned to `zoosky/rust-yaml`
      `feat/roundtrip-document`.
- [x] The three adapter tests that flip with the fixed engine assert `Found`
      (keep-chomped, alias) or the line-start-inclusive bytes (duplicate
      collection), and the lone-CR case round-trips.
- [x] `cargo fmt`, `cargo clippy -- -D warnings` across the default,
      `--no-default-features`, `backend-noyalib`-only, and
      `backend-rust-yaml`-only profiles, `cargo test` (default and
      `--no-default-features`), the `backend-noyalib` fidelity harness, and
      `cargo bench --no-run` all pass.
