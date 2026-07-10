# Implementation m004 — crates.io release posture (git-dep fidelity backends)

**Status:** Done — yqr is **published to crates.io** (`0.2.1`, 2026-07-10). The dependency blockers were resolved (`noyalib` re-pinned to crates.io `0.0.14`; `rust-yaml-rt` removed via the single-engine consolidation, `yqr-m005`), leaving zero git deps, and the maintainer's `cargo publish` has landed — `cargo install yqr` works.
**Owner:** yqr maintainers
**Last updated:** 2026-07-10
**Related:** `yqr-m005` (single-engine consolidation — removed the last git-dep), `yqr-f004` (engine parity — superseded), `yqr-m002` (engine seam), `yqr-b002` (the noyalib fixes), `yqr-b001`

## 1. Purpose

Record why yqr cannot currently be published to crates.io at `0.2.0`, and the
exact conditions that unblock it. crates.io **rejects git dependencies** — every
declared dependency (optional included) must resolve to a published version, and
the `git`/`branch` spec is stripped on publish. `f004` made both fidelity
backends part of the default build, and both are pinned to unreleased fork
branches, so `cargo publish` fails at the manifest-verification step:

```
error: all dependencies must have a version requirement specified when publishing.
       dependency `noyalib` does not specify a version
```

`rust-yaml-rt` fails identically. This is a release-channel constraint, not a
code defect: the crate builds, tests (156 passing), and is GitHub-released.

## 2. Blocking dependencies (both resolved)

| Dependency | Current pin (`Cargo.toml`) | crates.io state |
|---|---|---|
| `noyalib` ✅ **resolved** | `noyalib = "0.0.14"` (crates.io) | **0.0.14 released** with b002 2.2–2.7; upstream release [noyalib#160](https://github.com/sebastienrousseau/noyalib/pull/160) merged. git-dep dropped 2026-07-10 |
| `rust-yaml-rt` ✅ **resolved (removed)** | *(dependency removed)* | yqr consolidated on a single engine (`yqr-m005`): the rust-yaml fork backend and its git-dep were deleted, so there is nothing left to publish |

Historical note: `0.1.1` published cleanly because it predates `f004` —
`noyalib` was `"0.0.13"` (a real version, optional/off) and `rust-yaml-rt` did
not exist. Adding the two fork backends as defaults is precisely what a
crates.io release cannot express.

## 3. Unblock conditions (all required)

1. ✅ **Done (2026-07-10) — noyalib on crates.io with the fixes.** [noyalib#160](https://github.com/sebastienrousseau/noyalib/pull/160)
   released v0.0.14 (folding in b002 2.2–2.7); yqr is re-pinned to
   `noyalib = "0.0.14"` and the `git`/`branch` spec is dropped.
2. ✅ **Done (2026-07-10) — no git-dep backend.** Rather than find a crates.io
   home for `RoundTripDocument`, yqr **removed** the rust-yaml fork backend and
   consolidated on noyalib (`yqr-m005`), eliminating the git dependency outright.
3. ✅ **Done (2026-07-10) — published.** `cargo publish --dry-run` passed (zero
   git deps) and the maintainer ran `cargo publish`; crates.io serves `0.2.1`.

## 4. Release state

- **GitHub release `v0.2.0`** is live: <https://github.com/zoosky/yqr/releases/tag/v0.2.0>
  (tag `v0.2.0` → commit `c9e432f`). This is independent of crates.io and valid.
- **crates.io** now serves **0.2.1** — the first crates.io release, carrying the
  single-engine consolidation. `0.2.0` was GitHub-only (its git-dep backends
  could not be published); `0.2.1` reconciled the version for crates.io.

## 5. Acceptance criteria

- [x] `noyalib` re-pinned to a crates.io version carrying b002 2.2–2.7 (0.0.14+).
- [x] `rust-yaml-rt` git-dep eliminated (backend removed, `yqr-m005`).
- [x] `cargo publish --dry-run` passes with no git-dep error.
- [x] `cargo publish` run by the maintainer; crates.io shows `0.2.1` (the reconciled version).
