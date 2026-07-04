# Bug Status Tracker

Single source of truth for the state of every `yqr-bNNN` bug. Update this file
in the same change that opens, advances, or resolves a bug (mirrors the feature
status tracker convention).

**Status legend:** Open · In Progress · Fixed · Resolved · Won't Fix · Duplicate

## Open

| Bug | Title | Severity | Status | Related |
|-----|-------|----------|--------|---------|
| [b001](yqr-b001-roundtrip-discards-whitespace-and-formatting.md) | Round-trip through `rust-yaml` discards whitespace, comments, and formatting | High | Open (default pipeline; engine path mitigated by `yqr-f002`, upstream fix in-flight as rust-yaml#73) | `yqr-a001`, `yqr-r001`, `yqr.f001` |
| [b002](yqr-b002-noyalib-cst-span-and-key-model-deficiencies.md) | noyalib CST deficiencies: span boundaries, duplicate-key policy, string-only key model | Medium | Open (upstream; yqr-side mitigations shipped) | `yqr-f002`, `yqr-r002`, `yqr-m002` |

## Resolved

_None yet._

## Summary

- Total bugs: 2
- Open: 2 (1 High, 1 Medium)
- Resolved: 0
