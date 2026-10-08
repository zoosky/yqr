# Feature f025 — Override a merged-in key by creating an explicit entry

**Status:** Done — shipped 2026-10-08; filed 2026-08-22 from `yqr-b020`'s
review
**Epic:** Write tier (`f006`)
**Owner:** yqr maintainers
**Related:** `yqr-b020` (whose refusal message this is the missing half of),
`yqr-b019`, `yqr-b021` (which was the other write yqr declined at a
resolvable path, until noyalib 0.0.28 made it write; this one is the last),
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

## 4. Decisions (2026-10-08)

**A bare `=` is the spelling.** The path is the choice §3 asked for:
`.c.k = 9` asks for exactly `c.k` to become 9, and the explicit shadow
entry is the only edit that does that and nothing else — writing at the
definition would change siblings the filter never named. The two edits
have two spellings already: `.c.k = 9` overrides for `c` alone,
`.defaults.k = 9` changes every inheritor. No flag, no second operator.
The same applies to `|=`, which reaches the write through the same
resolver. One consequence is deliberate: assigning the value the key
already inherits is **not** a no-op — the shadow decouples `c.k` from
the anchor, which is real work (`yqr-b019`'s rule, now with the write
it was waiting for).

**The entry goes where every new key goes.** The engine's typed
insertion places it, after the mapping's own entries. The hand-written
"override directly under the `<<`" convention loses to having one
placement rule for all new keys.

**The merge-only mapping is yqr's splice.** Upstream's insertion still
refuses it on 0.0.56 (no own entry to anchor against), and its
fragment `set` cannot express the edit either (three variants measured:
wrong column, indentation error, oracle refusal). yqr splices one
placeholder line — `k: null`, or `{}`/`[]` for a collection value, so
the follow-up write replaces like with like — after the mapping's last
line, committed only when the re-parse is the original with exactly
that shadow added. The real value then goes through the ordinary
assignment, so the engine spells it: the same division of labor as
`yqr-f036`'s collapse. The engine's merge-only refusal wording is
pinned; if upstream rewords it, a test fails rather than the route
silently dying.

**What stays refused.** A parent reached through an alias (`c: *m`,
or nested inside one): an explicit entry cannot exist there without
rewriting the alias into a block, which is a restructuring the user
must spell out. The refusal names the definition route, which works.
The alias-*valued* entry itself (`.c = 1` over `c: *m`) keeps
`yqr-b019`'s refusal untouched.

## 5. Acceptance criteria

- [x] Writing a merged-in key creates an explicit entry that shadows the
      inherited value, and the loaded-back document reflects the new value.
- [x] Every other byte is unchanged, including the `<<` line and the anchor.
- [x] It works on a merge-only mapping, with no own entry to anchor against.
- [x] The alias-*valued* case (`b: *x`, `.b = 1`) stays refused — replacing a
      reference with a literal is a different question, and `yqr-b019` settled
      it.
- [x] `yqr-b020`'s refusal message names this route once it exists. (The
      merge arm no longer refuses at all; the refusal that remains, on an
      alias-reached parent, names the definition route, and the guide
      documents both edits.)
- [x] Corpus cases on `FIDELITY_RICH`, which already carries a `<<` — the
      two `yqr-b019`/`yqr-b020` pins flipped to the override behaviour.
