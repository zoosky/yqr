# Feature f036 — Reconsider the scalar-over-a-block-collection refusal

**Status:** Draft — filed 2026-09-17 from `yqr-f035` §2.4
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-b029` (the defect the refusal was built for, resolved by
it), `yqr-f032` (which added it), `yqr-f025` (a refusal names a remedy
that runs), noyalib#424

## 1. Why this is open

`.k = 5` where `k` holds a block mapping is refused at exit 5, and the
refusal names the remedy: `del(.k)` then assign, which puts the value at
the end of the mapping. The refusal exists because of `yqr-b029`: the
engine wrote the scalar at the key's own column, producing a document
noyalib read back and PyYAML and Psych rejected.

noyalib#424, released in 0.0.44, fixes that. Measured with yqr's
pre-emptive guard bypassed:

| release | `.k = 5` over `k:` / `  a: 1` |
|---|---|
| 0.0.43 | `k:` / `5` — column 0, invalid |
| 0.0.44 | `k:` / `  5` — valid, means `k: 5` |

So the refusal now guards against a layout that is correct rather than one
that is broken.

## 2. The question

Valid is not the same as right. `k:` / `  5` is a layout no author writes,
produced by an edit that named a value, and a reviewer reading the diff
sees a key whose value moved to the next line for no reason they asked
for. The alternatives:

1. **Keep the refusal.** The remedy works and puts the value where an
   author would write it. Costs a capability that the engine now supports.
2. **Drop it and take upstream's layout.** One fewer refusal; produces the
   odd two-line spelling.
3. **Write the scalar on the key's own line.** What the user means, and
   what `del`-then-assign produces except for the position in the mapping.
   Needs yqr to replace the entry's whole span rather than its value's,
   which is a write path yqr does not have.

Option 3 is the one worth pricing, because it is the only one that makes
the edit do what it says. The `yqr-b006` argument applies to how the span
is derived: from the engine's key and value spans, not from a
column-counting scan.

## 3. Not decided here

`yqr-f035` is an adoption, and this is a behaviour change with its own
remedy text, its own tests and a possible new write path. The guard stays
as it is until this is picked up, which is why `b029` stays Resolved: the
defect it names is fixed twice over, once by the guard and now upstream.

## 4. Acceptance criteria

- [ ] One of the three options in §2 chosen, with the reason recorded.
- [ ] If the refusal goes: the corpus case
      `write/collection-rhs/refuses-a-scalar-over-a-block-collection` and
      the `guards.rs` tests updated, and the byte result pinned.
- [ ] If option 3: the entry-span write path covered for a nested key, a
      root key, a key with a trailing comment, and a key whose value block
      carries comments of its own.
- [ ] `local-ci.sh` clean.
