# Bug b027 — `validate` is quadratic in document size

**Status:** Resolved — fixed 2026-09-02 in the change that found it (the
values corpus, `yqr-m003` §7)
**Severity:** High — `validate` took over two minutes on a 530 KB values
file in a debug build and three seconds in release, sixty times the
default read of the same file; the editing loop's correctness gate was the
slowest thing in it
**Component:** `src/validate/scan.rs` (the under-indentation scan),
`src/validate/mod.rs` (the conflict-marker search and the collision note),
`src/validate/render.rs` (the line table)
**Related:** `yqr-f012` (the validate command), `yqr-m003` §7 (the corpus
that measured it), `yqr-b025` (the file it was measured on)

## 1. Summary

Three places in `validate` visited every line or every mapping entry of
the input and, for each, recomputed something that costs the whole input:

| site | per item | cost |
|---|---|---|
| `scan::under_indented_entry`, on every block-mapping entry | `position_in(source, byte)` counted line breaks from the start of the document, twice | O(entries × bytes), on every successful parse |
| `first_conflict_marker_line`, on every line | `render::line_text(source, n)` rebuilt the line table | O(lines × bytes), on every parse failure |
| `collision_document_note`, on every line and again per chunk | the same | O(lines × bytes), on a key collision |

The first one runs on every clean document, so it set the cost of the
common case. Measured with the shape generator from the values corpus
(`tenants(n)`, 530 bytes per tenant) on the shipped code, debug profile:

| tenants | bytes | `validate` |
|---|---|---|
| 100 | 53 KB | 2.9 s |
| 300 | 160 KB | 21 s |
| 500 | 266 KB | 62 s |
| 700 | 373 KB | 120 s |
| 1000 | 533 KB | 136 s (149 s with `--strict`) |

Release profile at 1000 tenants: 3.1 s, against 0.05 s for `yqr .` on
the same file, which parses it through the same engine. The second site
showed as 26 s in debug for a 575 KB document the parser refuses (the
generator at 1100 tenants, past the alias budget) — the whole time spent
locating a merge-conflict marker that is not there.

## 2. Fix

Every check stays local to what it inspects:

- The same-line test between a key and its value is "no line break in the
  bytes between them"; the column is a walk back to the previous line
  break. Both are bounded by the line, not the document.
- `render::lines(source)` yields every line from one pass over the line
  table; the marker search and the collision note iterate that instead of
  asking for line *n* in a loop.

After the fix, debug profile: 0.2 s at 1000 tenants, 0.6 s for the
refused 1100-tenant document, 0.04 s for the refused production file.
The corpus's command-line tier, which validates the 1000-tenant shape
and the production file, went from 227 s to 2 s.

## 3. Why nothing caught it

Every validate test ran on documents of a few lines, where quadratic and
linear are the same number. The corpus's scale cases are the first
validate inputs above a kilobyte; `corpus/scale_tenants/validate_strict`
in `benches/corpus_bench.rs` now tracks the cost at 100, 400 and 1000
tenants beside the read and write paths on the same document, so a
regression shows as a ratio, not a feeling.

## 4. Acceptance

- [x] `validate` at 1000 tenants under a second in debug; the ratio to
      `yqr .` on the same file within a small constant in release.
- [x] No remaining per-line `line_text` or per-entry document scan in
      `src/validate/`.
- [x] `cli/scale/validate-1000` in the corpus, and the bench group above.
- [x] Every existing validate test unchanged and green.
