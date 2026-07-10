# Bug Status Tracker

Single source of truth for the state of every `yqr-bNNN` bug. Update this file
in the same change that opens, advances, or resolves a bug (mirrors the feature
status tracker convention).

**Status legend:** Open · In Progress · Fixed · Resolved · Won't Fix · Duplicate

## Open

| Bug | Title | Severity | Status | Related |
|-----|-------|----------|--------|---------|
| [b001](yqr-b001-roundtrip-discards-whitespace-and-formatting.md) | Round-trip through the default pipeline discards whitespace, comments, and formatting | High | Open (default pipeline is a lossy semantic round trip — now through noyalib, `yqr-m005`; the byte-faithful path is the fidelity engine, opt-in via `--preserve`, `yqr-f005`) | `yqr-a001`, `yqr-r001`, `yqr.f001`, `yqr-f002`, `yqr-f005`, `yqr-m005` |
| [b004](yqr-b004-noyalib-cst-mutation-api-gaps.md) | noyalib CST mutation-API gaps: comment editing, key rename, sequence reorder, nested/multi-line delete | Medium | Open (upstream noyalib 0.0.14; roadmap-gating for the write/edit tier, not yet blocking — read-only engines today) | `yqr-b002`, `yqr-r002`, `yqr-m002`, `yqr-m005` |

## Resolved

| Bug | Title | Severity | Status | Related |
|-----|-------|----------|--------|---------|
| [b002](yqr-b002-noyalib-cst-span-and-key-model-deficiencies.md) | noyalib CST deficiencies: span boundaries, duplicate-key policy, string-only key model | Medium | Resolved — all 7 fixes released upstream in noyalib 0.0.14 (2.1 in 0.0.13; 2.2-2.7 in the v0.0.14 release #160); yqr pins crates.io `noyalib = "0.0.14"`, git-dep dropped — `yqr-m004`, `yqr-f004` | `yqr-f002`, `yqr-f004`, `yqr-r002`, `yqr-m002` |
| [b003](yqr-b003-rustyaml-roundtrip-trailing-doc-end-marker.md) | rust-yaml fork `RoundTripDocument::parse_all` errors on a trailing `...` after a block collection | Medium | Resolved (moot) — the rust-yaml fork backend was removed when yqr consolidated on noyalib (`yqr-m005`); no longer affects yqr | `yqr-m005`, `yqr-b001`, `yqr-m002` |

## Summary

- Total bugs: 4
- Open: 2 (1 High — b001; 1 Medium — b004)
- Resolved: 2 (b002 — 2.2-2.7 fixes released in noyalib 0.0.14, git-dep dropped;
  b003 — moot after the single-engine consolidation removed the rust-yaml
  backend, `yqr-m005`)
