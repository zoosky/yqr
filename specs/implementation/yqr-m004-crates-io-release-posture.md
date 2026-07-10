# Implementation m004 — crates.io release posture (git-dep fidelity backends)

**Status:** In Progress — v0.2.0 is tagged and GitHub-released; the crates.io publish is **held** pending the remaining `rust-yaml-rt` release (see §3). `noyalib` is now resolved: re-pinned to the crates.io `0.0.14` release (2026-07-10).
**Owner:** yqr maintainers
**Last updated:** 2026-07-10
**Related:** `yqr-f004` (both fidelity engines shipped by default), `yqr-m002` (engine seam), `yqr-b002` (the noyalib fixes the fork carries), `yqr-b001`/`rust-yaml` fork (backend A)

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

## 2. Blocking dependencies (one resolved, one remaining)

| Dependency | Current pin (`Cargo.toml`) | crates.io state |
|---|---|---|
| `noyalib` ✅ **resolved** | `noyalib = "0.0.14"` (crates.io) | **0.0.14 released** with b002 2.2–2.7; upstream release [noyalib#160](https://github.com/sebastienrousseau/noyalib/pull/160) merged. git-dep dropped 2026-07-10 |
| `rust-yaml-rt` (`package = rust-yaml`) — **remaining blocker** | `git = zoosky/rust-yaml`, branch `feat/roundtrip-document` | crates.io `rust-yaml` is **1.1.0**, which has **no `RoundTripDocument` API**; the fork's feature has **no published crate** today |

Historical note: `0.1.1` published cleanly because it predates `f004` —
`noyalib` was `"0.0.13"` (a real version, optional/off) and `rust-yaml-rt` did
not exist. Adding the two fork backends as defaults is precisely what a
crates.io release cannot express.

## 3. Unblock conditions (all required)

1. ✅ **Done (2026-07-10) — noyalib on crates.io with the fixes.** [noyalib#160](https://github.com/sebastienrousseau/noyalib/pull/160)
   released v0.0.14 (folding in b002 2.2–2.7); yqr is re-pinned to
   `noyalib = "0.0.14"` and the `git`/`branch` spec is dropped.
2. **A crates.io home for the `RoundTripDocument` backend.** Either upstream the
   API into `rust-yaml` and pin a released version, or publish the fork as a
   distinct crate and pin that. This is the harder, still-open item.
3. **Clean dry-run, then publish.** `cargo publish --dry-run` must pass, then
   `cargo publish` (requires the maintainer's crates.io token — a manual step).

## 4. Interim state

- **GitHub release `v0.2.0`** is live: <https://github.com/zoosky/yqr/releases/tag/v0.2.0>
  (tag `v0.2.0` → commit `c9e432f`). This is independent of crates.io and valid.
- **crates.io** remains at **0.1.1**.
- No version mismatch is introduced: when the deps are released and yqr is
  published, `0.2.0` on crates.io will match the GitHub release. If a different
  version is cut for crates.io first, reconcile the tag/CHANGELOG accordingly.

## 5. Acceptance criteria

- [x] `noyalib` re-pinned to a crates.io version carrying b002 2.2–2.7 (0.0.14+).
- [ ] `RoundTripDocument` backend available from a crates.io version; `rust-yaml-rt` re-pinned off `git`.
- [ ] `cargo publish --dry-run` passes with no git-dep error.
- [ ] `cargo publish` run by the maintainer; crates.io shows `0.2.0` (or the reconciled version).
