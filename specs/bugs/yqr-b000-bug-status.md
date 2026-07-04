# Bug Status Tracker

Single source of truth for the state of every `yqr-bNNN` bug. Update this file
in the same change that opens, advances, or resolves a bug (mirrors the feature
status tracker convention).

**Status legend:** Open · In Progress · Fixed · Resolved · Won't Fix · Duplicate

## Open

| Bug | Title | Severity | Status | Related |
|-----|-------|----------|--------|---------|
| [b001](yqr-b001-roundtrip-discards-whitespace-and-formatting.md) | Round-trip through `rust-yaml` discards whitespace, comments, and formatting | High | Open (default pipeline; engine path closed on both backends — `yqr-f002` noyalib + `yqr-f003` rust-yaml fork `RoundTripDocument`, the rust-yaml#73 substrate) | `yqr-a001`, `yqr-r001`, `yqr.f001`, `yqr-f002`, `yqr-f003` |
| [b002](yqr-b002-noyalib-cst-span-and-key-model-deficiencies.md) | noyalib CST deficiencies: span boundaries, duplicate-key policy, string-only key model | Medium | Open (upstream; yqr-side mitigations shipped; deficiency 2.1 fix filed as noyalib#143) | `yqr-f002`, `yqr-r002`, `yqr-m002` |
| [b003](yqr-b003-rustyaml-roundtrip-trailing-doc-end-marker.md) | rust-yaml fork `RoundTripDocument::parse_all` errors on a trailing `...` after a block collection | Medium | Open (upstream fork; yqr-side documented + pinned in `yqr-f003`) | `yqr-f003`, `yqr-b001`, `yqr-m002` |

## Resolved

_None yet._

## Summary

- Total bugs: 3
- Open: 3 (1 High, 2 Medium)
- Resolved: 0
