# Bug Status Tracker

Single source of truth for the state of every `yqr-bNNN` bug. Update this file
in the same change that opens, advances, or resolves a bug (mirrors the feature
status tracker convention).

**Status legend:** Open · In Progress · Fixed · Resolved · Won't Fix · Duplicate

## Open

| Bug | Title | Severity | Status | Related |
|-----|-------|----------|--------|---------|
| [b001](yqr-b001-roundtrip-discards-whitespace-and-formatting.md) | Round-trip through `rust-yaml` discards whitespace, comments, and formatting | High | Open (default pipeline; engine path closed on both backends — `yqr-f002` noyalib + `yqr-f003` rust-yaml fork `RoundTripDocument`, the rust-yaml#73 substrate) | `yqr-a001`, `yqr-r001`, `yqr.f001`, `yqr-f002`, `yqr-f003` |
| [b002](yqr-b002-noyalib-cst-span-and-key-model-deficiencies.md) | noyalib CST deficiencies: span boundaries, duplicate-key policy, string-only key model | Medium | Resolved — all 7 fixes released upstream in noyalib 0.0.14 (2.1 in 0.0.13; 2.2-2.7 in the v0.0.14 release #160); yqr pins crates.io `noyalib = "0.0.14"`, git-dep dropped — `yqr-m004`, `yqr-f004` | `yqr-f002`, `yqr-f004`, `yqr-r002`, `yqr-m002` |
| [b003](yqr-b003-rustyaml-roundtrip-trailing-doc-end-marker.md) | rust-yaml fork `RoundTripDocument::parse_all` errors on a trailing `...` after a block collection | Medium | Open (upstream fork; yqr-side documented + pinned in `yqr-f003`) | `yqr-f003`, `yqr-b001`, `yqr-m002` |

## Resolved

_None yet._

## Summary

- Total bugs: 3
- Open: 2 (1 High — b001; 1 Medium — b003)
- Resolved: 1 (b002 — the 2.2-2.7 fixes are released upstream in noyalib 0.0.14;
  yqr now consumes the crates.io `noyalib = "0.0.14"` release, git-dep dropped)
