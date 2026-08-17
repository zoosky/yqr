# Bug b010 — `swap_items` / `move_item` move values, not entries: every comment stays behind

**Status:** Resolved (2026-08-17) — the maintainer took the semantics yqr
argued for; the fix is yqr's own commit `d397330`, landed via upstream #271 and
**released in noyalib 0.0.23**. Adopted by `yqr-f016`, which verified it
against the published crate. `noyalib#269` closed 2026-08-16
**Severity:** Medium — a silent wrong result at exit 0, not a refusal
**Component:** noyalib `cst::Document::swap_items` / `cst::Document::move_item`
(upstream), reached from yqr's planned reorder verb
**Blocks:** the `swap` / `move` slice of `yqr-a002` §9
**Related:** `yqr-b004` §2.3 (the gap whose fix shipped this API) and §6.5 (the
adoption finding that points here), `yqr-b006` (the same failure class in
yqr's own delete), `yqr-a002` §6 (the architecture consequence),
`yqr-f007` §5.1 (the standing reminder this is the third instance of),
`yqr-f015` (the 0.0.22 pin), `yqr-f016` (0.0.23, which does not fix this — §4.1)

## 1. Summary

noyalib's sequence-reorder mutators exchange the items' **value bytes** and
nothing else. Every comment stays attached to the position rather than to the
item it documents, so a reorder silently re-attributes the file's
documentation. Both calls return `Ok`, the exit code is 0, and `yqr -i` would
write the damage to the user's file.

This is not a reopened gap — `yqr-b004` §2.3's API exists and ships.

### 1.1 It is a disagreement, not a defect — corrected 2026-08-16

This spec and the first version of noyalib#269 both called it a defect "of the
same class as `b004` §6.1 and §6.4". **That framing is wrong**, and the
correction is on the record upstream.

Upstream's behaviour is deliberate, documented and tested. `swap_items`' doc
comment says it rewrites "only the two items' value bytes", and
`swap_preserves_inline_comment_position` pins exactly the §2 measurement with
its rationale in a comment: *"Only the value bytes move; the comment annotates
the slot."* §6.1/§6.4 were cases where upstream diverged from what it said it
did. This is not one; it is yqr disagreeing with what upstream says it does.

The measurement in §2 stands unchanged — only the word for it moves. And the
disagreement still blocks the slice, because what yqr needs from a reorder is
not what upstream currently provides.

**The argument yqr makes is internal consistency, not correctness in the
abstract.** `remove` already decides the same question the other way for the
same bytes: an entry owns the contiguous same-indent comment run directly above
it (`owned_entry_range` / `absorb_head_comments`), on the reasoning that leaving
it behind makes the comment "documentation for the *next* entry". Two mutators
in one crate cannot hold opposite views of who owns a comment. `# about one`
above `- one` is not plausibly annotating position 0.

For **inline** comments the slot reading is defensible — `- a  # first` can mean
"first in the list". yqr's position is that it is the rarer case and that
splitting the two would be worse than either choice, since head comments
travelling while inline ones stayed would make one swap's output incoherent.

Lesson for this repo, since it cost a mis-filed issue: measuring behaviour and
reading the implementation is not the same as reading the **tests**. A tested
behaviour with a rationale in the test body is a decision, and has to be argued
with rather than reported.

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

## 4.1 Not fixed by 0.0.23

Verified 2026-08-16 against `v0.0.22...main`, after upstream closed `#221`
with the extended `remove` (`yqr-f016` §2). The only source file the diff
touches is `crates/noyalib/src/cst/document.rs`, and its hunks are `remove`
plus a new `Removal` enum and `flow_member_range` helper. Neither
`swap_items` nor `move_item` appears in the diff, and both resolve through
`span_at` + `replace_span`, unchanged for them. This bug survives 0.0.23.

`#221` covered reorder as its sub-ask 3 and considers it shipped in 0.0.18 —
correctly, since the API exists. With the umbrella now closed, the filing in
§5 went as **its own issue** rather than a comment on a closed thread.

## 5. Route: upstream, and yqr already owns the reference implementation

On the `yqr-b004` §5 `PR-with-fix` precedent. `delete_entry`
(`src/fidelity/write/delete.rs`) computes exactly the range an entry owns —
value span, continuation lines, and the contiguous same-indent head-comment
run above it, with a blank-detached comment deliberately excluded. A
trivia-aware reorder is two of those ranges exchanged. That arithmetic was
written, argued and tested for delete in `yqr-b006`; it transfers.

**Filed and fixed, 2026-08-16.**

- [noyalib#269](https://github.com/sebastienrousseau/noyalib/issues/269) — the
  issue, plus the §1.1 correction posted as a follow-up once the pinning test
  turned up.
- [noyalib#270](https://github.com/sebastienrousseau/noyalib/pull/270) — the
  fix. It turned out that upstream *already has* the arithmetic: `#268` added
  `owned_entry_range` for the extended `remove`, so the change is
  `swap_items` asking `entry_line_span` for each item and taking the
  whole-entry path when both come back as `Removal::Line`, rather than new
  byte machinery. Flow members keep the value-span exchange. `move_item` is a
  run of adjacent swaps and inherits it.

  One detail worth carrying back if yqr ever owns this: each *position* keeps
  its own line terminator while the bodies move. A document whose last entry
  has no trailing newline otherwise splices `- a\n- b` into `- b- a`.

  Evidence offered with it: 5594 tests and 499 doctests pass, clippy and fmt
  clean, and **exactly one existing test changed** — the one pinning the old
  semantics. That number is the argument. The rest of the suite is indifferent,
  so the single disagreement is the design decision and not a blast radius.

The maintainer may prefer slot semantics and close it. §6 is written for
either outcome.

## 6. Impact on yqr

`yqr-a002` settles the reorder grammar (`swap(<path>; i; j)`,
`move(<path>; from; to)`) and then stages that slice **last** and blocked, on
this bug. Until it is fixed, `swap`/`move` either stay unimplemented or ship
with a yqr-side refusal when either item carries a comment — honest and small,
but it declines the common case, since a commented list item is the normal
shape of the files yqr targets (`spec.containers`, GitHub Actions `steps`,
Ansible tasks).

This is the third time an independent yqr implementation has been the thing
that made an upstream trivia divergence measurable (`b004` §6.1, §6.4, here) —
though this one is a disagreement rather than a divergence (§1.1).

**If upstream keeps slot semantics**, the slice ships with the yqr-side refusal
above and this bug closes as Won't Fix rather than Resolved; `yqr-a002` §9
slice 3's second criterion ("an item's inline and head comments travel with the
item") becomes unreachable through the backend and would have to be met by yqr
owning the arithmetic — the `yqr-f007` §2 route of last resort. Decide that
only if it happens.

## 7. Acceptance

- [x] Filed upstream with the §2 measurement and the §5 reference —
      noyalib#269, 2026-08-16.
- [x] Framing corrected upstream once the pinning test was found (§1.1).
- [x] Fix offered as a PR — noyalib#270.
- [x] Maintainer's decision on the semantics — taken, in yqr's favour, with
      the re-framing explicitly endorsed.
- [x] A released noyalib moves an entry's inline and head comments with the
      item, for both `swap_items` and `move_item` — 0.0.23, verified against
      the published crate (`yqr-f016` §3).
- [ ] `yqr-a002` §9 slice 3's criteria pass against that release — the slice is
      unblocked but not implemented.
- [x] `yqr-b000` and this file moved to Resolved in the same change.

## 8. Close-out

Two things worth keeping from how this went, both about the *report* rather
than the fix:

- **The §1.1 correction was the load-bearing move.** Filing it as a defect was
  wrong, and saying so before the maintainer spent time on it is what the
  close-out singled out: *"'Defect' would have been the wrong word, you caught
  it before we spent time on it, and you re-put it as the design question it
  actually was. That saved a round trip."*
- **The argument that won was internal consistency**, not correctness in the
  abstract — `remove` and reorder cannot hold opposite views of who owns a
  comment. Conceding the inline-comment half, and declining to split head from
  inline, were both endorsed as correct.

Two defects in yqr's PR were fixed by the maintainer on top, both yqr's and
both documentation-only: a public doc comment linked a private item (rejected
under `-D rustdoc::private_intra_doc_links`, a gate yqr's own `local-ci.sh`
does not have), and a new function inserted above an existing one landed
*between* that function and its doc comment, silently leaving it undocumented.
Neither is caught by clippy or by any test.
