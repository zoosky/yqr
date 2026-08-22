# Feature f025 — Override a merged-in key by creating an explicit entry

**Status:** Draft — filed 2026-08-22, from `yqr-b020`'s review
**Epic:** Write tier (`f006`)
**Owner:** yqr maintainers
**Related:** `yqr-b020` (whose refusal message this is the missing half of),
`yqr-b019`, `yqr-b021` (the other write yqr declines at a resolvable path),
`yqr-f007` §6

## 1. Scope

`.c.k = 9`, where `c` gets `k` from a `<<` merge or an alias, should be able to
write an explicit `k: 9` entry into `c`, shadowing the inherited value.

Today it is refused. `yqr-b020` argues that refusal is right *as a default* —
creating an override is a different edit from replacing a value, and the user
should choose it — but yqr currently offers no way to choose it at all.

## 2. Why this is worth a spec rather than a one-line change

It came out of a review finding on `b020`'s refusal message, which offered
*"add an explicit `k` entry here to override it"* as a remedy. Measured, the
remedy is unreachable:

- `.c.k = 9` is refused by the very check that prints the advice.
- `.c.z = 1` — inserting some *other* key — is refused too whenever the mapping
  has no entry of its own to anchor an insertion against:
  - on `c: *m`, with *"insert_entry_value: `c` is inside the value anchored by
    `&m`"*;
  - on a merge-only `c:` / `<<: *m`, with *"no entry of the mapping at `c` has
    source bytes of its own to anchor"*.
- It works only when `c` already owns a sibling entry.

So a message naming that route was naming something the tool declines, and both
refusals leak upstream API names into user-facing output. The message now names
only the anchor route, which is measured to work; this spec is where the second
route goes.

## 3. Design questions to settle first

**Is a bare `=` the right spelling?** It is the least surprising — `.c.k = 9`
means "make `c.k` be 9", and it currently does that everywhere else. Against
it: the edit is a *creation*, invisible in the filter, and it changes what a
later edit to the anchor does to this file. `yqr-b020` §2 called that a choice
the user should make deliberately. An explicit form (a flag, or a distinct
operator) makes it deliberate at the cost of a second way to say one thing.

**Where does the entry go?** After the `<<` line, before the mapping's own
entries, is the convention in hand-written YAML; appending is what the engine's
insertion mutator does. They differ, and the diff is the deliverable.

**The empty-mapping case.** A merge-only mapping has no entry of its own, and
upstream's insert refuses for exactly that reason (§2). This is upstream work
or yqr's own splice, the same fork in the road `yqr-f007` §5.1 records for
delete.

## 4. Acceptance criteria

- [ ] Writing a merged-in key creates an explicit entry that shadows the
      inherited value, and the loaded-back document reflects the new value.
- [ ] Every other byte is unchanged, including the `<<` line and the anchor.
- [ ] It works on a merge-only mapping, with no own entry to anchor against.
- [ ] The alias-*valued* case (`b: *x`, `.b = 1`) stays refused — replacing a
      reference with a literal is a different question, and `yqr-b019` settled
      it.
- [ ] `yqr-b020`'s refusal message names this route once it exists.
- [ ] Corpus cases on `FIDELITY_RICH`, which already carries a `<<`.
