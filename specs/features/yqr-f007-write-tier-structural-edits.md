# Feature f007 — Write tier: structural edits (the `b004` gaps)

**Status:** In Progress (structural **delete** shipped, and since `yqr-f013`
it is yqr's whole delete path rather than a fallback; comment editing, key
rename, and sequence reorder remain deferred)
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f006` (write tier v1 — the value-replacement core this builds
on), `yqr-b004` (the noyalib 0.0.14 mutation-API gap catalog), `yqr-m002`
§4/§6.2 (write-tier seam)

## 1. Scope

The surgical edits that have **no first-class API** in noyalib 0.0.14 and so are
excluded from `f006`. Each is cataloged in `yqr-b004` §2:

- **Structural delete** — multi-line, nested, sole-entry, and flow deletes that
  `Document::remove` rejects (`b004` 2.4). **Shipped** (§5).
- **Comment editing** — add / change / remove a comment attached to a node
  (`b004` 2.1; comments were read-only in 0.0.14). **Deferred** (§6).
- **Key rename** — `.a.b` key renamed in place, preserving `:` and value
  (`b004` 2.2). **Deferred** (§6).
- **Sequence reorder / move / swap** — reorder block-sequence items (`b004` 2.3).
  **Deferred** (§6).

## 2. Dependencies & approach

- **Preferred:** upstream noyalib mutators — reported as umbrella issue
  noyalib#221 and contributed as PRs-with-fix on the #118/#123 precedent
  (`b004` §5). All five are **released in noyalib 0.0.18** (2026-07-31) as
  `set_inline_comment`/`set_leading_comment`, `rename_key`,
  `swap_items`/`move_item`, a `remove` that handles multi-line and nested
  values, and the `Emit` insertion tier. yqr is **pinned to 0.0.18** since
  `yqr-f013`; what remains deferred here is therefore the **grammar** in §6,
  not the backend. One exception, settled by `yqr-f013` §3.2: upstream
  `remove` is *not* adopted, because it scopes a delete to the entry's
  key/value lines and so diverges on trivia in three ways that `b006`
  classifies as silent wrongness (§5.1).
- **Own the arithmetic:** where no upstream API fits — either because one does
  not exist, or because the one that exists has different semantics — yqr
  performs the edit via raw `Document::replace_span`, owning the
  indent/quote/line arithmetic itself, behind an **integrity guard yqr
  enforces** (§3). `replace_span` guarantees only that the result is *valid
  YAML*, not that it preserves structure (`b004` 2.5), so the guard is yqr's,
  not the backend's. Delete is the settled instance of this and is no longer
  framed as temporary (§5.1); anything else on this route is.

## 3. Structural-integrity contract

Identical to `f006` §7: an accepted edit changes only the targeted node's bytes;
an edit that would restructure the document is refused (exit 5); `-i` leaves the
file untouched on refusal. Because `replace_span` does not enforce this, every
edit routed through it must **prove** the property before committing: apply it to
a private copy, re-parse it, and commit only if the re-parsed document equals the
original value with exactly the target change applied — otherwise refuse, leaving
the document untouched.

## 4. Acceptance criteria

- [x] Delete of a multi-line / nested node, byte-exact elsewhere; a clear error
      where still unsupported (sole-entry, flow).
- [x] Every `replace_span` edit is guarded and covered by a byte-exact test.
- [ ] Comment set/insert/remove at a resolved path, byte-exact elsewhere.
- [ ] `.old |= key-rename` (final syntax TBD) renames a key, preserving value +
      trailing comment.
- [ ] Sequence reorder/move/swap by index, re-parse-guarded.

## 5. Structural delete (shipped)

### 5.1 Surface

No new grammar: `del(<path>)` already parses (`f006`). f006 routed every delete
through noyalib's `Document::remove`, which in 0.0.14 handled only single-line
block entries and refused everything else; f007 added a fallback for the
refusals. Since `yqr-f013` there is no fallback and no upstream call:

```
del(path) -> delete_entry()   # every delete, single-line or not
```

`delete_entry` lives in `src/fidelity/write/delete.rs`, extending
`NoyalibWriter` (the value-write trait stays in `write.rs`; the byte-arithmetic
concern is a sibling sub-module).

**Why not upstream `remove`.** 0.0.18's `remove` accepts every shape this
module maps, so the obvious move on the pin bump was to call it and delete this
module. Measured, it is wrong: upstream treats an entry as its key/value lines,
where yqr treats an entry as owning its trivia. Three cases diverge, all of them
silent successes rather than refusals — the `b006` failure class:

| Case | `delete_entry` | `Document::remove` as of 0.0.18 |
|------|----------------|---------------------------|
| Head comment above the entry | removed with the entry | survives, silently re-attributed to the next sibling |
| Keep-chomped (`\|+`) scalar's kept trailing blanks | removed with the entry | left behind as stray blank lines |
| A following comment belonging to the *next* sibling | left in place | swallowed |

The first two under-delete, the third over-deletes and loses a comment
outright. §5.4's tests pin all three; each one fails against upstream `remove`,
which is how the divergence was measured rather than assumed.

**Since fixed upstream, and yqr still does not delegate.** All three were filed
as noyalib#225 and fixed by yqr's noyalib#226, released in **noyalib 0.0.19**;
yqr pins 0.0.21 (`yqr-f014`). The table above therefore describes 0.0.18, not
the current pin. Delegation was reconsidered and declined on fresh grounds —
0.0.21 fixed a `remove` that destroyed a whole flow collection while returning
`Ok`, so this is where upstream churn concentrates, and yqr's path carries no
open defect. See §6 for the standing item.

Independently worth keeping: yqr's flow pre-check reports `removing an item
from a flow collection is not supported`, where upstream surfaces `remove:
could not locate '-' indicator preceding sequence item`.

### 5.2 Algorithm

For the target entry (final path segment `last`, resolved value byte span
`value_start..value_end` from `Document::span_at`):

1. **Locate the entry's first line.** Walk back from `value_start` over
   insignificant whitespace/newlines to the entry marker (`:` for a mapping key,
   `-` for a sequence index), stepping over a trailing line comment on the key
   line (`key:  # note`). The marker's line start is the entry's first line.
2. **Derive the owned range from the value's own span, not an indentation
   heuristic.** The end is the end of the line holding `value_end`'s last content
   byte — except when `value_end` already sits at a line boundary, which happens
   only for a keep-chomped (`|+` / `>+`) block scalar whose trailing blank lines
   are content (`span_at` keeps them); extending then would swallow the next
   sibling's line. This is exact where an indentation walk errs:
   - a **keep-chomped scalar**'s kept trailing blanks are owned (removed with the
     entry), leaving no stray blank line;
   - a **following comment** that documents the next sibling lies outside
     `value_end` and survives, while a comment **interleaved inside** the value
     lies within `value_end` and goes with it;
   - a **block sequence written at its key's own column** (`on:\n- a\n- b`, the
     GitHub Actions / Ansible / Kubernetes list style) — which `span_at`
     under-reports to just its first `-` — has its true end recovered from the
     last item's span, so it deletes cleanly instead of being refused.
   A contiguous run of same-indent comment lines **directly above** the entry is
   its head comment and is folded into the range, so a delete never silently
   re-attributes the comment to the following sibling.
3. **Refuse the unsupported shapes** with a clear message: the **sole entry** of
   a block (removing it would empty the block, which re-parses as `null`), and an
   item of a **flow** collection (`[a, b]` / `{a: 1}`, detected from the parent's
   own bytes — including a root-level flow collection, whose parent is the
   document itself).
4. **Splice, guard, and commit byte-preservingly.** Re-parse the spliced source
   and require it to lower to the original document value with the target removed
   (mapping key order preserved, sequence indices shifted); a dangling alias
   (deleting a referenced anchor), an over-broad span, or a flow mis-edit all
   diverge here and are refused with the document untouched. The commit itself
   goes through `Document::replace_span`, which splices the source buffer in
   place: every surviving byte is the original byte verbatim, so no parse→emit
   round-trip can normalize an untouched node.

### 5.3 Why this is safe

The deletion range is derived from the target value's authoritative source span,
so it cannot reach into a sibling's content, and its head-comment extension is
bounded to contiguous same-indent comment lines that document the entry. Surviving
bytes are preserved *by construction* — the commit is an in-place buffer splice
(`replace_span`), not a re-emit. The remaining proof obligation — that the removed
range is precisely the target's — is discharged by the re-parse-equals-expected
guard (§3). The worst failure mode is therefore a *refusal* of a deletable entry
(a benign over-refusal), never a silent corruption.

### 5.4 Coverage

Unit (`write/delete.rs`), library (`tests/integration.rs`), and CLI
(`tests/cli.rs`) tests cover: nested block-mapping delete; multi-line
sequence-item delete; comment/blank-line/sibling preservation; a comment on the
key line; a following sibling's comment left intact; a head comment removed with
its entry (and a blank-detached comment left in place); a keep-chomped scalar's
trailing blanks removed with the entry; a block sequence at its key's own column
(top-level and nested); last-entry and multi-document deletes; and the refusals
(sole entry, sole top-level entry, flow item — nested and root-level,
alias-breaking delete). `-i` writes the closed-up document back atomically; a
refused delete leaves the file unchanged.

Four of those unit tests double as the §5.1 divergence net: the head-comment
pair, the keep-chomped scalar, and the following-sibling's comment all fail if
`del` is routed through upstream `remove`. They run through the public `apply`
entry point, not `delete_entry` directly, so a future re-adoption of `remove`
cannot slip past them.

## 6. Deferred gaps (roadmap)

The remaining three gaps each need **new user-facing grammar** the epic has not
settled; they stay deferred and continue to error with a clear "not yet
supported" message. The byte arithmetic, however, is no longer yqr's problem —
noyalib 0.0.18 ships a guarded API for each, so once the grammar is settled
these become a call, not a splice (adoption: `yqr-f013` §3.4):

- **Comment editing** (`b004` 2.1) — needs a comment-addressing syntax.
  Upstream: `set_inline_comment` / `remove_inline_comment`,
  `set_leading_comment` / `remove_leading_comment`, single-line nodes only.
- **Key rename** (`b004` 2.2) — needs a rename syntax. Upstream: `rename_key`
  with style-matched quoting and sibling-duplicate refusal, plus `key_span`
  for the key-token range `span_at` never exposed.
- **Sequence reorder** (`b004` 2.3) — needs a reorder syntax. Upstream:
  `swap_items` and `move_item`, the latter a guarded run of adjacent swaps, so
  no offset re-basing in yqr.

The upstream `PR-with-fix` path (§2) already ran its course for all three, and
0.0.22 is pinned (`yqr-f015`), so each of these is now a grammar decision over
a live API. Raw `replace_span` is the route of last resort rather than the
expected one — though §5.1 is the standing reminder that "upstream has the
call" and "upstream has yqr's semantics" are different questions.

Two further items are open here, neither gated on grammar:

- **Re-evaluate delegating delete to upstream `remove`.** The three trivia
  divergences that made §5.1 keep yqr's path were fixed by yqr's noyalib#226,
  released in 0.0.19, so `yqr-f013` §3.2's option (b) is unblocked.
  `yqr-f014` §3.3 declined it for a different reason: 0.0.21 fixed a
  `remove` that destroyed a flow collection while returning `Ok`, so delete is
  where upstream churn concentrates, and yqr's path has no open defect. Cheap
  to revisit; revisit deliberately, not as a side effect of another change.
- **Collection right-hand sides for `+=` / new-key assignment.** Since
  `yqr-b008` these route through the typed `Emit` tier, which can spell a
  nested collection — so the "collections are not yet supported" refusal is now
  a scope limit, not a backend one. Lifting it is a user-facing surface change
  and belongs here or in `yqr-f008`, with its own tests and docs.
- **Creating a key that holds `.` or `[`.** `insert_entry_value` can splice one
  (it needs a path only to *replace* an existing key, so adding
  `app.kubernetes.io/name` is supported upstream), but yqr still refuses it in
  `insert_key`. The refusal is no longer about expressing the key — it is that
  yqr's path grammar could not then address it, so the edit would write a key
  the tool cannot read back. Lifting it means settling that addressing question
  first (an escape or quoting form in the path syntax), which is grammar work,
  not a splice change. This is the common Kubernetes label/annotation case and
  is worth doing.
- **A multi-line-insert case in the shared corpus.** `yqr-b008` §6: the unit
  tests pin the fix, but `yqr-m003`'s byte-exact `EngineCase` tier has no
  multi-line-string insert, so a backend swap could reintroduce the corruption
  without failing `tests/corpus_validation.rs`.

_(These criteria firm up once the grammar and the upstream API surface are
known.)_
