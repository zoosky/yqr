# Bug b019 — The no-op guard runs ahead of the writer's refusals, and does not cover comments


> **Historical: resolved.** yqr no longer behaves as described below. The
> **Status** line records what fixed it and when; the rest is kept as the
> reproduction and the reasoning, written in the present tense of the time it
> was filed.

**Status:** Resolved — 2026-08-22, in review of the `yqr-b018` fix, same day
**Severity:** Medium — §3 is a silent exit 0 where the user's edit was declined
and a live alias reference was left in the file; §4 is a fidelity violation of
the same shape as `yqr-b018` itself
**Component:** `src/fidelity/write.rs`, `set_value_unless_unchanged` and the
`Target::LineComment` / `Target::HeadComment` assign arm
**Related:** `yqr-b018` (the guard this is about, and its §7.2/§7.3),
`yqr-a001` §1 (the guarantee §4 breaks), `yqr-a002` §4 (the comment surface),
`yqr-f006`, `yqr-f007`, `yqr-f008`

## 1. Summary

`yqr-b018` added one guard — skip the write when the new value equals the one
already there — and placed it before the writer. Two things follow from that
placement, and neither was intended.

**A refusal became a silent success** wherever it does not depend on the value
being written:

```console
$ printf 'a: &x 1\nb: *x\n' | yqr '.b = 2'    # exit 5, refused
yqr: runtime error: cannot assign at "b": cannot set `b`: its value is (or
resolves through) an alias reference; edit the anchor definition or replace
the alias explicitly
$ printf 'a: &x 1\nb: *x\n' | yqr '.b = 1'    # exit 0, and b is still *x
a: &x 1
b: *x
```

**And the rule stopped at values.** The same write path re-spells a comment it
did not need to change:

```console
$ printf 'a: 1 #tight\n' | yqr 'line_comment(.a) = "tight"'
a: 1 # tight
```

## 2. Scope

Two defects, one cause: a rule stated in one place and applied at the wrong
point, and the same rule missing from a sibling that needed it. They are filed
together because the fix is one reordering plus one copy of the guard, and
because separating them would hide that `yqr-b018`'s title — *an assignment
that changes nothing must not re-spell it* — already covers the second.

## 3. The guard swallows a refusal

An alias site refuses **every** value. The value the user asks for changes the
diagnostic not at all, so comparing values first and skipping on a match turns
a refusal into a success. Same for a path that resolves through a `<<` merge,
where the refusal is `path not found`:

```console
$ printf 'base: &m\n  k: 1\nc:\n  <<: *m\n' | yqr '.c.k = 9'   # exit 5
$ printf 'base: &m\n  k: 1\nc:\n  <<: *m\n' | yqr '.c.k = 1'   # exit 0, silent
```

### 3.1 Why this is not `yqr-b018` §7.2 again

§7.2 records a refusal the guard was **allowed** to skip: `.["a.b"] = 1` on a
document where `a.b` is already `1` used to fail with *"cannot address key …
characters the write path cannot express"* and now succeeds. That was called
deliberate, and it is. The two cases look alike and are not, and the line
between them is worth stating because it decides the fix:

- An unaddressable key is **yqr's own expressive limit**. The document already
  holds exactly what the user asked for, byte for byte, and nothing about it
  can change later as a result. No write was needed, so no limit was reached.
- An alias is a **property of the document**. `b` does not hold `1`; it holds a
  pointer to whatever `x` holds. Writing there would replace a reference with a
  literal — real work, and the user's own reason for asking. Skipping leaves
  the coupling in place, so a later `.a = 2` moves `b` too, and the user was
  told nothing.

The postcondition reading ("`b == 1` afterwards, so nothing to do") is what
makes the two look the same. It is the wrong reading for an alias, because the
typed value is not what differs.

## 4. The rule stops at values

`set_comment` takes a body, not a line. `#tight` and `# tight` carry the same
body, so upstream re-emits its own spacing and the line is rewritten by a write
that changed no content. That is `yqr-a001` §1's counter-example again, on a
comment instead of a scalar, and the mechanism is identical to `yqr-b018` §1:
the model handed to the writer cannot carry the spelling that is in the file.

It also breaks the read/write pair. `line_comment(.a)` prints `tight`; feeding
that straight back should be a no-op and is not.

## 5. Fix route

**Order.** Establish whether the value is *borrowed* — living elsewhere in the
document — before deciding a write can be skipped, and on a borrowed site fall
through to the writer rather than reporting anything here. Falling through is
the point: the diagnostic stays upstream's, so there is no second copy of it to
drift, and if upstream ever permits the write it simply happens.

**Detection.** noyalib decides the same question in `Document::write_span`, but
that is private, so yqr establishes it from the public span API instead of
guessing at it:

- A path ending in a **key** whose `key_span` is `None`. The key is not in the
  source, so a `<<` merge or an alias expansion produced it. Restricting the
  test to key-terminated paths is what keeps a sequence item and the root out;
  both legitimately have no key of their own.
- A value span that starts **before** its own `key_span`. YAML requires an
  anchor to precede every alias to it, so bytes ahead of the key naming them
  are the anchor's, reached by resolving an alias through.

Anything else answers *not established*, and the caller attempts the write.
That direction is the safe one: a false positive would re-spell a scalar that
needed no write, which is the bug this guard exists to prevent.

**Comments.** The same guard, on the body as the read path spells it (no `#`,
one leading space dropped), which is what makes read-then-write-back a no-op.
It reports nothing at a site `set_comment` would refuse, so §3 cannot recur
here.

## 6. What was considered and rejected

**Probing the writer on a cloned document.** `cst::Document` is `Clone`, so the
guard could attempt the write on a copy and refuse whatever that refuses. It
needs no heuristic and cannot drift from upstream, and it is wrong anyway: it
refuses too much. `printf 'a: &x 1\nb: *x\n' | yqr '.a = 1'` is a genuine
`yqr-b018` no-op on the **anchor**, and the probe fails it, because rewriting
`a`'s value drops `&x` and breaks `b`. Measured, not assumed. A false refusal
on a no-op is precisely what `yqr-b018` is about not doing.

**Matching upstream's error text** to narrow that probe. Rejected on the
project's standing objection to reading a sibling's diagnostics as an API.

**Comparing emitted bytes instead of values.** Rejected for the reason
`yqr-b018` §5 gives, unchanged.

**Widening the check to sequence items.** An alias-valued sequence item
(`- *x`) has no key to measure against, so neither rule reaches it and
`.b[0] = 1` still skips silently. Catching it means comparing against the
item's own extent, which the public API does not expose; guessing from the
nearest ancestor key would be a rule that holds only while the anchor sits
outside the sequence. Left as a known gap rather than covered approximately —
the residue is a *silent skip*, which is the status quo, not a new wrong
answer.

> **Closed 2026-08-22**, in `yqr-b020`'s review, which called it what it is: a
> live false success, where the write "succeeds" exactly when it would have
> been refused. That the residue was the status quo made it survivable, not
> acceptable.
>
> The framing above is what kept it open, and it was too narrow. Both rules
> ask one question — *do the resolved bytes start before the earliest point
> this node's own bytes could?* — and only the **floor** differs. For a
> mapping entry it is the key. For a sequence item it is the end of the item
> ahead of it, or the sequence's own start for the first. That is not the
> "nearest ancestor key" guess: it is the item's own container, so an anchor
> outside the sequence and an anchor in an earlier sibling are caught by the
> same comparison, and it cannot false-positive, because an item's bytes never
> precede its neighbour's.
>
> One shape still clears its floor honestly: an item of a sequence that is
> *itself* borrowed (`b: *x`, `.b[0]`), where the floor is measured inside the
> anchor. `value_is_borrowed` walks the ancestors for it — bytes inside a node
> that is not its owner's are not their own either — and the four shapes are
> pinned by `an_alias_valued_sequence_item_is_refused_whatever_the_value`.
>
> The check is still yqr's rather than upstream's answer. `resolve_span`
> computes `through_alias` exactly and keeps it private; if that is ever
> exposed, it replaces the floor rule outright.

## 7. Reproduction

```console
$ printf 'a: &x 1\nb: *x\n' | yqr '.b = 1'                 # exit 0   -- wrong
$ printf 'base: &m\n  k: 1\nc:\n  <<: *m\n' | yqr '.c.k = 1'  # exit 0 -- wrong
$ printf 'a: 1 #tight\n' | yqr 'line_comment(.a) = "tight"'   # a: 1 # tight -- wrong
$ printf 'a: &x 1\nb: *x\n' | yqr '.a = 1'                 # a: &x 1  -- right, keep it
$ printf 'n: 0640\n' | yqr '.n = .n'                       # n: 0640  -- right, keep it
```

## 8. Coverage

Six CLI tests, one unit test on the borrowed predicate, and two corpus cases.
Both directions are pinned, because the fix is a boundary and either side of it
is a defect:

- `a_no_op_does_not_swallow_an_alias_refusal`,
  `a_no_op_does_not_swallow_a_merge_key_refusal` — §3, both operators.
- `an_anchored_scalar_is_still_its_own_value` — the rejected probe's failure
  case, kept as a test so the cheaper design cannot be reintroduced silently.
- `a_comment_write_that_changes_nothing_changes_nothing`,
  `the_comment_guard_does_not_block_a_real_write`,
  `reading_a_comment_and_writing_it_back_is_a_no_op` — §4.
- `borrowed_is_the_value_living_somewhere_else` — the predicate itself, over
  four borrowed shapes and six own ones.
- `write/assign/a-no-op-does-not-swallow-a-borrowed-site` and
  `write/comment/idempotent-comment-write-keeps-the-line` in the shared corpus,
  plus the `yqr-b018` pair the same review corrected.

`FIDELITY_RICH` gained a second inline comment, written tight, for the last of
those. The one it already had sits behind a wide gutter *with* a space after
the `#` — which is the canonical spelling, so re-emitting it lands on itself
and the case would have passed with no guard at all. The same trap as the
`containerPort: 9090` case in `yqr-b018` §6, in the comment tier.
