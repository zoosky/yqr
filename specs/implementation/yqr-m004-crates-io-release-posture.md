# Implementation m004 — crates.io release posture (git-dep fidelity backends)

**Status:** Done — yqr is **published to crates.io** (`0.2.1`, 2026-07-10; latest `0.7.0`, 2026-08-23). §6 records a packaging defect found in `0.6.0` and the guard added against it. The dependency blockers were resolved (`noyalib` re-pinned to crates.io `0.0.14`; `rust-yaml-rt` removed via the single-engine consolidation, `yqr-m005`), leaving zero git deps, and the maintainer's `cargo publish` has landed — `cargo install yqr` works.
**Owner:** yqr maintainers
**Last updated:** 2026-08-20
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

## 6. Package contents (added 2026-08-20)

`exclude` in `Cargo.toml` is the only thing standing between the working tree
and the uploaded crate, and it is a **denylist** — a new top-level directory is
published by default. `0.6.0` shipped yqr's entire website that way.

**What happened.** The list read `[".agent/", ".github/", "specs/",
"AGENT.md"]`, written before `docs/` existed. The Accent site landed after
`0.5.1` (`yqr-f010`), so `0.6.0` was the first release to carry it: 84 files
and 340 KB uncompressed, **40% of the package**, the largest single file in the
crate being a 100 KB `og-image.png`. The agent guide rode along too, by a
subtler route: `exclude` names `AGENT.md`, but `CLAUDE.md` is a **symlink** to
it, and the symlink is a separate path that the denylist did not name — so the
guide was published under the other name. Uncompressed package size went 863 KB
with them, 493 KB without.

Nothing is broken for users: the crate builds, and no license or dependency
constraint is touched. It is dead weight in every `cargo install yqr`
download, and `0.6.0` cannot be corrected — crates.io versions are immutable
(yank yes, replace never), so the fix lands in the next release.

**Why nothing caught it.** `ci.yml` filters on Rust-relevant paths, so the PRs
that added `docs/` never ran CI at all, and no gate anywhere looked at what
`cargo publish` would upload. `cargo publish --dry-run` (§5) reports the file
count but was last run for `0.2.1`, when the answer was still right.

**Fix.**

1. `exclude` gains `"docs/"` and `"CLAUDE.md"`.
2. `local-ci.sh` gains a `package contents` gate that fails when `cargo package
   --list` names anything under `docs/`, `specs/`, `.github/`, `.agent/`, or
   the two agent guides. It lives there rather than in `ci.yml` deliberately:
   the failure mode is a *docs-only* change, which `ci.yml` is configured to
   skip. `yqr-m001` §3 already runs `local-ci.sh` before tagging, so the gate
   sits on the release path by construction.

The denylist stays a denylist rather than becoming an `include` allowlist: an
allowlist that omits a new source directory produces a crate that does not
build, which is a worse failure than one that is too large.

### 6.1 Acceptance criteria

- [x] `docs/` and `CLAUDE.md` excluded from the published crate.
- [x] `cargo package --list` names no dev-only path (36 files, 493 KB).
- [x] `local-ci.sh` fails when a dev-only path re-enters the package.
- [x] Confirmed against crates.io on the next release after `0.6.0`.
      **`0.7.0`, 2026-08-23**: the published `.crate` was downloaded and
      unpacked rather than inspected through `cargo package --list`, and it
      holds **38 files, 164 KB**, with no `docs/`, `specs/`, `.agent/`,
      `.github/`, `CLAUDE.md` or `AGENT.md`. Against `0.6.0`'s 493 KB that is
      a third of the size. `cargo install yqr --version 0.7.0` then ran the
      release's own features from the installed binary — which is how
      `yqr-b023` was found, since a locally built binary could not have shown
      it.
