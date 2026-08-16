# Bug b010 — `swap_items` / `move_item` move values, not entries: every comment stays behind

**Status:** Open (measured 2026-08-15 on the noyalib 0.0.22 pin; not yet filed
upstream)
**Severity:** Medium — a silent wrong result at exit 0, not a refusal
**Component:** noyalib `cst::Document::swap_items` / `cst::Document::move_item`
(upstream), reached from yqr's planned reorder verb
**Blocks:** the `swap` / `move` slice of `yqr-a002` §9
**Related:** `yqr-b004` §2.3 (the gap whose fix shipped this API) and §6.5 (the
adoption finding that points here), `yqr-b006` (the same failure class in
yqr's own delete), `yqr-a002` §6 (the architecture consequence),
`yqr-f007` §5.1 (the standing reminder this is the third instance of),
`yqr-f015` (the 0.0.22 pin)

## 1. Summary

noyalib's sequence-reorder mutators exchange the items' **value bytes** and
nothing else. Every comment stays attached to the position rather than to the
item it documents, so a reorder silently re-attributes the file's
documentation. Both calls return `Ok`, the exit code is 0, and `yqr -i` would
write the damage to the user's file.

This is not a reopened gap — `yqr-b004` §2.3's API exists and ships. It is a
defect *inside* a shipped mutator, of the same class as `b004` §6.1 and §6.4.

## 2. Measurement

Driven directly against the pinned crate, 2026-08-15, re-confirmed 2026-08-16.
Two separate inputs, one per call:

```text
in                   swap_items("", 0, 1)      in                move_item("", 0, 2)
- one  # first       - two  # first            - a  # ca         - b  # ca
- two  # second      - one  # second           - b  # cb         - c  # cb
                                               - c  # cc         - a  # cc
```

Head comments behave identically:

```text
in                   swap_items("", 0, 1)
# about one          # about one
- one                - two
# about two          # about two
- two                - one
```

`# about one` stays above index 0 while the value it described moves away.

Note that `move_item` needs the three-item input: `move_item(_, 0, 2)` on the
two-item sequence refuses with `index out of bounds for the sequence at ``
(length 2): from 0, to 2`. The out-of-range refusal is correct and is the only
guard in the neighbourhood that fires.

## 3. Why the guard does not catch it

Both mutators run upstream's integrity guard, and both pass it **by
construction**: the guard re-parses and compares the *typed* value, and a
comment is not in the typed value. A guard that compares typed values can
never observe a comment moving. This is the same reason `b004` §6.1 and §6.4
needed an independent implementation to become visible.

## 4. Two stale documented refusals — three, in fact

While measuring this, three of `swap_items`' documented errors turned out not
to fire on 0.0.22:

| Case | Doc comment | 0.0.22 |
|---|---|---|
| flow sequence | error | succeeds: `[one, two, three]` -> `[three, two, one]` |
| multi-line items | error | succeeds: `- a: 1\n  b: 2\n- c: 3\n  d: 4` swaps whole entries |
| differently-indented items | error | succeeds: `- a\n-   b\n` -> `- b\n-   a\n` |

Nothing yqr plans depends on any of them, and they are not part of this
bug's fix. They are recorded because they are the second half of the same
lesson: `yqr-f007` §5.1's reminder now needs three clauses — "upstream has the
call", "upstream does what its docs say" and "upstream has yqr's semantics"
are three different questions.

## 5. Route: upstream, and yqr already owns the reference implementation

On the `yqr-b004` §5 `PR-with-fix` precedent. `delete_entry`
(`src/fidelity/write/delete.rs`) computes exactly the range an entry owns —
value span, continuation lines, and the contiguous same-indent head-comment
run above it, with a blank-detached comment deliberately excluded. A
trivia-aware reorder is two of those ranges exchanged. That arithmetic was
written, argued and tested for delete in `yqr-b006`; it transfers.

**Not yet filed.** The filing hangs here.

## 6. Impact on yqr

`yqr-a002` settles the reorder grammar (`swap(<path>; i; j)`,
`move(<path>; from; to)`) and then stages that slice **last** and blocked, on
this bug. Until it is fixed, `swap`/`move` either stay unimplemented or ship
with a yqr-side refusal when either item carries a comment — honest and small,
but it declines the common case, since a commented list item is the normal
shape of the files yqr targets (`spec.containers`, GitHub Actions `steps`,
Ansible tasks).

This is the third time an independent yqr implementation has been the thing
that made an upstream trivia divergence measurable (`b004` §6.1, §6.4, here).

## 7. Acceptance

- [ ] Filed upstream with the §2 measurement and the §5 reference.
- [ ] A released noyalib moves an entry's inline and head comments with the
      item, for both `swap_items` and `move_item`.
- [ ] `yqr-a002` §9 slice 3's criteria pass against that release.
- [ ] `yqr-b000` and this file move to Resolved in the same change.
