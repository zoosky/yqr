# Feature f038 — Delete inside a shared anchor

**Status:** Draft — filed 2026-09-19 from `yqr-f036`'s code review
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f036` (which gave assignment this behaviour), `yqr-b026`
and `yqr-f026` (the anchor-definition write), `yqr-f016` (structural
delete)

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

## 3. Acceptance criteria

- [ ] Decide: follow the anchor rule, or keep the refusal with an accurate
      message. Record why.
- [ ] If it follows the rule: `del` inside a shared block mapping and
      sequence works, the alias sites show the removal, and nothing else
      changes.
- [ ] Either way, the refusal that remains names the anchor.
- [ ] `local-ci.sh` clean.
