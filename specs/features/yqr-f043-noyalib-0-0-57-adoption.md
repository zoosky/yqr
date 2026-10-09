# Feature f043 — Adopt noyalib 0.0.57: both open engine bugs close

**Status:** Done — shipped 2026-10-09
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-b035` and `yqr-b036` (closed), `yqr-f036` (whose
refusal now names remedies that run), `yqr-f038` (whose anchor rule the
unblocked delete joins), noyalib#475 and noyalib#477 (yqr's own fixes,
carried by this release)

## 1. What the release carries

noyalib 0.0.57 (2026-10-09) carries both of yqr's open upstream fixes —
"two fixes from @zoosky", per its notes — plus six parse fixes from
differential fuzzing against serde_yaml_ng and libyaml, several of them
breaking parse behaviour.

- **noyalib#475 closes `b036`**: the green walk recurses into a nested
  collection from the node's own offset, not the `&anchor`/`!tag`
  prefix start, so the items of an anchored, indented block sequence
  report their own spans.
- **noyalib#477 closes `b035`**: `SpanTree::Alias` records the `*name`
  token's own span; `remove` and a scalar `set_value` act on it; a
  no-op keeps the reference; every path that resolves *through* an
  alias keeps an accurate refusal.

## 2. What moved in yqr, measured

Six pinned expectations flipped on the bump, each in the direction its
bug spec predicted:

- `del(.a[0])` on `a: &x` over indented items works, and the alias
  shows the removal (the `f038` rule) — the `b036` pin.
- A scalar at a direct alias entry or item replaces the `*name` token:
  `.j = 5`, `.c = 1` over an alias to a block, and the three
  alias-item shapes the `b019` floor rule guards. The anchor and every
  other reference keep their bytes.
- An equal value stays a **no-op that keeps the reference** — the
  engine's documented rule, keeping the author's `*x` spelling through
  a save that changes nothing. Recorded deliberately: this is the
  opposite call from `f025`'s merged-key shadow, where writing the
  inherited value decouples. The merge case creates an entry that did
  not exist; the alias case would destroy spelling that does. Both are
  the conservative reading of their own mechanism.
- A path resolving *through* an alias still refuses, now with
  upstream's own wording ("resolves through an alias reference to the
  anchor's bytes"); through-alias deletes keep the `f038`
  anchor-rule path untouched.

One yqr change rode the adoption: the block delete delegates an
alias-valued entry to upstream's `remove` — the `Borrowed::Value`
discriminator fires only when the addressed entry's value *is* the
reference — closing `b035`'s delete half (block entry, trailing
comment, sequence item; the flow member had already closed through the
delegated flow path). And `f036`'s removed-anchor refusal names
remedies that run — assign over the reference, or `del(…)` its entry —
with a test that runs one.

## 3. The parse-behaviour changes, checked

The six fuzzing fixes change how some documents parse (`0X1F` and
`0x-1` as strings, folded-scalar whitespace, escaped line breaks,
`- &a\n- x` as two items, typed-parse key spelling under a tag,
duplicate-key equivalence of `~`/`null` and radix spellings). Every
yqr suite — corpus, CLI, fidelity, values — passed with no expectation
moved beyond the six above, so none of the shapes yqr pins is
affected. The corpus carries no case for the new shapes; add one if a
field file ever exercises them.

## 4. Acceptance criteria

- [x] Pin at 0.0.57; both bug specs flipped to Resolved with their
      pinned tests inverted, not deleted.
- [x] `del` and `=` work at alias-valued entries in every layout
      `b035` §1 tabulated; through-alias paths still refuse.
- [x] The `f036` refusal names remedies a test runs.
- [x] `local-ci.sh` clean.
