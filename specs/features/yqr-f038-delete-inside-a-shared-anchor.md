# Feature f038 — Delete inside a shared anchor

**Status:** Done — shipped 2026-10-08; filed 2026-09-19 from
`yqr-f036`'s code review
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f036` (which gave assignment this behaviour), `yqr-b026`
and `yqr-f026` (the anchor-definition write), `yqr-f016` (structural
delete), `yqr-b036` (the span defect this feature's measurement found)

## 1. The question

yqr writes inside an anchored value at its definition, and every `*name`
site sees the change. That is what an anchor means, and `yqr-b026` made it
the rule for assignment: `.a.k.n = 5` on `a: &x` / `  k:` / `    n: 1` /
`b: *x` succeeds, and `b.k.n` reads `5`. Since `yqr-f036`, so does
`.a.k = 5` over a block.

`del` does not follow it:

```console
$ printf 'a: &x\n  k:\n    n: 1\n  z: 2\nb: *x\n' | yqr 'del(.a.k)'
yqr: runtime error: cannot delete a.k: the edit would change the document structure and was refused
```

The splice itself is right: it removes `k` from the definition. The value
check refuses it because it expects `b` unchanged, and `b` loses `k` too.
That is the same reflection assignment already accepts.

## 2. Proposal

Accept the reflection in `delete_entry`'s value check, the way
`changes_are_the_assignment` does for assignment: the result must equal
the original with the entry removed, except at alias sites of the anchor
that covers the entry, which must show exactly the same removal.

Two things to settle first:

- **Whether a user expects it.** An assignment names a new value, and
  seeing it at the alias sites is the point of the anchor. A deletion
  that removes a key from every site sharing the value is the same rule,
  but it is the more surprising edit. Record the argument, and consider
  whether the refusal should stay with an accurate message instead.
- **The message today.** Whatever is decided, "would change the document
  structure" should name the anchor and the alias sites, as the
  removed-anchor refusal in `f036` does.

## 3. Decision (2026-10-08)

**Follow the anchor rule.** The deciding argument is §1's own: the
splice was already right, and the refusal guarded a reflection that is
what an anchor means. yqr cannot hold that `.a.k = 5` changes `b`
through `*x` by design while `del(.a.k)` changing `b` is corruption —
one semantics per mechanism. The "more surprising edit" worry from §2
is answered the same way assignment answered it: the surprise is the
anchor's, not the verb's, and a user who shares a value has asked for
shared edits. The refusal that remains for removing the `&name` itself
while an alias still uses it (`yqr-f036`) is untouched and still names
the anchor and the alias.

### 3.1 What the check accepts

Not the assignment rule verbatim. `changes_are_the_assignment` sees a
reflection as a subtree swapping the parent's before-value for its
after-value, which holds at a plain `*x` alias site — but a `<<` merge
site is a **larger mapping losing one key** (the tenants shape:
`ops: {<<: *o1, own: ...}`), which that rule cannot express; measured
directly when the corpus case on the production shape refused. `del`
gets its own `changes_are_the_deletion`: a divergent collection is
accepted only when it is the expected one with exactly the deleted
segment removed, surviving entries matching in order, recursively. Any
other divergence still refuses.

### 3.2 What the measurement found

The sequence case on an **anchored key with indented items**
(`a: &x` / `  - 1`) is refused with "its source layout is not
supported": upstream's `span_at` reports the items' spans shifted by
the anchor property's width. Filed as `yqr-b036` and pinned as it
behaves; the same delete works when the items sit at the key's own
column, and reads of the shape are correct (the wrong-node guard
degrades to a typed render).

## 4. Acceptance criteria

- [x] Decide: follow the anchor rule, or keep the refusal with an accurate
      message. Record why. (§3: follow the rule.)
- [x] If it follows the rule: `del` inside a shared block mapping and
      sequence works, the alias sites show the removal, and nothing else
      changes. (Unit tests cover the alias, merge-site, nested and
      sequence cases plus a coincidentally-equal sibling; the corpus
      write case runs it on the production tenants shape.)
- [x] Either way, the refusal that remains names the anchor. (The
      `f036` removed-anchor refusal is unchanged; the structure refusal
      no longer fires for the reflection, and what it still catches has
      no anchor to name.)
- [x] `local-ci.sh` clean.
