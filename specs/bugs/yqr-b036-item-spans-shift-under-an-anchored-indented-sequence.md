# Bug b036 — Item spans shift under an anchored, indented block sequence

**Status:** Open — filed 2026-10-08, found while measuring `yqr-f038`
**Severity:** Low — no path yqr ships emits wrong bytes or corrupts a
file; the cost is a delete that is refused on one layout and item reads
that fall back to typed rendering
**Component:** upstream `Document::span_at`; reaches yqr through
`src/fidelity/noyalib.rs` (read resolve) and
`src/fidelity/write/delete/mod.rs` (the item's owned range)
**Related:** `yqr-f038` (whose sequence case found it), `yqr-b028` (the
previous span-offset defect, fixed in 0.0.36), `yqr-b035` (the other
alias/anchor span gap)

## 1. Summary

When a mapping key carries an anchor **and** its value is a block
sequence whose items are indented past the key, `span_at` reports wrong
spans for the items. Measured on noyalib 0.0.56 over
`a: &x\n  - 1\n  - 2\nb: *x\n`:

| path | reported | correct |
|---|---|---|
| `a` | `(3, 17)` | `(3, 17)` — includes the anchor property, as elsewhere |
| `a[0]` | `(5, 9)` | `(10, 11)` |
| `a[1]` | `(11, 11)` | `(16, 17)` |

`a[0]`'s reported span starts on the line break after `&x` and covers
the item's indentation; `a[1]`'s is empty. Both are shifted by the
anchor property's width. Two controls isolate the trigger: without the
anchor (`a:`), the spans are exact; with the anchor but the items at
the key's own column (`a: &x\n- 1\n- 2\n`), they are exact too. Only
the combination shifts.

## 2. What it costs through yqr

Nothing silent, which is why the severity is low:

- **Reads are correct.** `.a[0]` prints `1`: the fidelity engine's
  wrong-node guard sees that the reported bytes do not denote the
  selected value, degrades to `Resolved::Synthetic`, and the caller
  renders from the typed value. The guard is doing exactly the job
  `yqr-m002` gave it.
- **The delete is refused, not wrong.** `del(.a[0])` on this layout
  cannot find the item's `-` marker from the shifted span and refuses
  with "its source layout is not supported" — safe, but a gap: the
  same delete works when the items sit at the key's own column, and
  `yqr-f038` made it work inside a shared anchor on that layout.

Pinned as it behaves in `src/fidelity/write/delete/mod.rs` (the
`yqr-f038` test block), so the noyalib bump that fixes the spans will
flip a test rather than pass silently.

## 3. Likely shape of a fix

The spans are consistent with the item offsets being computed relative
to a base that excludes the anchor property while the document's are
not, the same class as `yqr-b028`'s document-relative stream positions.
Upstream owns the fix; yqr has no workaround to add, since both of its
consumers already degrade safely.
