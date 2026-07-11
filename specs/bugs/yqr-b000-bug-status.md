# Bug Status Tracker

Single source of truth for the state of every `yqr-bNNN` bug. Update this file
in the same change that opens, advances, or resolves a bug (mirrors the feature
status tracker convention).

**Status legend:** Open · In Progress · Fixed · Resolved · Won't Fix · Duplicate

## Open

| Bug | Title | Severity | Status | Related |
|-----|-------|----------|--------|---------|
| [b004](yqr-b004-noyalib-cst-mutation-api-gaps.md) | noyalib CST mutation-API gaps: comment editing, key rename, sequence reorder, nested/multi-line delete | Medium | Open (upstream noyalib 0.0.14; the write tier's structural edits `yqr-f007`; the value-write tier `yqr-f006` avoids these gaps) | `yqr-b002`, `yqr-r002`, `yqr-m002`, `yqr-m005`, `yqr-f006`, `yqr-f007` |

## Resolved

| Bug | Title | Severity | Status | Related |
|-----|-------|----------|--------|---------|
| [b001](yqr-b001-roundtrip-discards-whitespace-and-formatting.md) | Round-trip through the default pipeline discards whitespace, comments, and formatting | High | Resolved for the default read — byte fidelity is now the **default** (`yqr-f009`); `yqr '.' f` is byte-for-byte identical to `f` via the fidelity engine (`yqr-f002`). The lossy semantic round trip is now opt-in via `--normalize` (by design) | `yqr-a001`, `yqr-r001`, `yqr.f001`, `yqr-f002`, `yqr-f009`, `yqr-m005` |
| [b006](yqr-b006-structural-delete-trivia-and-fidelity.md) | Structural delete mishandles comments, blank lines, and same-column sequences | High | Resolved — `owned_line_span` now derives its range from noyalib's value span, commits via byte-preserving `replace_span`, and folds a same-indent head comment into the delete; regression tests cover each case | `yqr-f007`, `yqr-b004`, `yqr-a001` |
| [b005](yqr-b005-crossbeam-epoch-advisory-via-criterion.md) | `cargo audit` fails on `crossbeam-epoch` advisory (RUSTSEC-2026-0204) via `criterion` dev-dep | Low | Resolved — `cargo update -p crossbeam-epoch` bumped the transitive pin `0.9.18 -> 0.9.20`; `cargo audit` exits 0, lockfile-only change | `yqr-m001` |
| [b002](yqr-b002-noyalib-cst-span-and-key-model-deficiencies.md) | noyalib CST deficiencies: span boundaries, duplicate-key policy, string-only key model | Medium | Resolved — all 7 fixes released upstream in noyalib 0.0.14 (2.1 in 0.0.13; 2.2-2.7 in the v0.0.14 release #160); yqr pins crates.io `noyalib = "0.0.14"`, git-dep dropped — `yqr-m004`, `yqr-f004` | `yqr-f002`, `yqr-f004`, `yqr-r002`, `yqr-m002` |
| [b003](yqr-b003-rustyaml-roundtrip-trailing-doc-end-marker.md) | rust-yaml fork `RoundTripDocument::parse_all` errors on a trailing `...` after a block collection | Medium | Resolved (moot) — the rust-yaml fork backend was removed when yqr consolidated on noyalib (`yqr-m005`); no longer affects yqr | `yqr-m005`, `yqr-b001`, `yqr-m002` |

## Summary

- Total bugs: 6
- Open: 1 (1 Medium — b004)
- Resolved: 5 (b001 — byte fidelity is now the default read (`yqr-f009`), closing
  the lossy-default round trip; the classic pipeline is now opt-in via
  `--normalize`;
  b006 — structural-delete trivia/fidelity defects fixed via a
  span-derived range + byte-preserving `replace_span`, `yqr-f007`;
  b005 — `crossbeam-epoch` advisory cleared by `cargo update`;
  b002 — 2.2-2.7 fixes released in noyalib 0.0.14, git-dep dropped;
  b003 — moot after the single-engine consolidation removed the rust-yaml
  backend, `yqr-m005`)
