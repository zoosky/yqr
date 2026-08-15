# Feature f007 — Write tier: structural edits (the `b004` gaps)

**Status:** In Progress (structural **delete** shipped, and since `yqr-f013`
it is yqr's whole delete path rather than a fallback; comment editing, key
rename, and sequence reorder remain deferred, but their grammar is settled as
of `yqr-a002` and staged in §6)
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f006` (write tier v1 — the value-replacement core this builds
on), `yqr-a002` (the addressing grammar for the three deferred slices),
`yqr-b004` (the noyalib 0.0.14 mutation-API gap catalog) and its §6.5 (the
reorder trivia defect that blocks one slice), `yqr-m002` §4/§6.2 (write-tier
seam)

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
- [ ] `line_comment(<path>)` / `head_comment(<path>)` set and remove
      (`yqr-a002` §2.4), byte-exact elsewhere.
- [ ] `key(<path>) = "new"` renames a key, preserving value + trailing comment.
- [ ] `swap(<path>; i; j)` / `move(<path>; from; to)` reorder by index,
      re-parse-guarded, with each item's comments travelling with the item.

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
yqr pins 0.0.22 (`yqr-f015`). The table above therefore describes 0.0.18, not
the current pin — **upstream now agrees with yqr on every case this module
pins**, measured, not assumed (§6). Delegation is nonetheless declined, on
grounds that do not depend on upstream being behind: an independent
implementation is the *oracle* that makes the engine's correctness checkable,
and it is what produced noyalib#225/#226 in the first place. Full reasoning in
§6.

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

The remaining three gaps each needed **new user-facing grammar** the epic had
not settled. **That grammar is now settled in `yqr-a002`** (2026-08-15), which
found the three to be one problem — yqr's path grammar addresses value nodes
and only value nodes, while a comment, a key token, and an ordering are none of
those — and answers it with a naming function wrapping a path
(`line_comment(<path>)`, `head_comment(<path>)`, `key(<path>)`, assignable with
`=` and removable with `del(...)`), plus a reorder verb (`swap(<path>; i; j)`,
`move(<path>; from; to)`) for the case with no single node to name. The three
slices, their refusal catalogs, and their acceptance criteria are staged in
`yqr-a002` §9; they stay deferred here and continue to error with a clear "not
yet supported" message until each ships.

The byte arithmetic is not yqr's problem — noyalib 0.0.18 shipped a guarded API
for each, so with the grammar settled two of the three are a call, not a splice
(adoption: `yqr-f013` §3.4):

- **Comment editing** (`b004` 2.1) — `yqr-a002` §2.4. Upstream:
  `set_inline_comment` / `remove_inline_comment`, `set_leading_comment` /
  `remove_leading_comment`, single-line nodes only. Measured on 0.0.22, upstream
  places a leading block **above** the addressed entry — the correct side, and
  the opposite of yq — so the call is adoptable as-is.
- **Key rename** (`b004` 2.2) — `key(<path>) = "new"`. Upstream: `rename_key`
  with style-matched quoting and sibling-duplicate refusal, plus `key_span`
  for the key-token range `span_at` never exposed.
- **Sequence reorder** (`b004` 2.3) — grammar settled, **slice blocked**.
  `swap_items` / `move_item` exchange value bytes only, leaving every comment
  attached to the position rather than to the item it documents, at exit 0 and
  past upstream's own guard (which compares typed values, where comments do not
  exist). That is §5.1's failure class exactly, and this module's entry-range
  arithmetic is the reference implementation for the fix. Measured and routed in
  `yqr-a002` §6 / `yqr-b004` §6.5.

The upstream `PR-with-fix` path (§2) already ran its course for all three, and
0.0.22 is pinned (`yqr-f015`), so comment editing and key rename are now
implementation over a live API. Raw `replace_span` is the route of last resort
rather than the expected one — and §5.1's reminder earned a third clause along
the way: "upstream has the call", "upstream does what its docs say", and
"upstream has yqr's semantics" are three different questions.

Four further items are recorded here, none gated on the `yqr-a002` grammar. The
first is settled; the other three are open:

- **Delegating delete to upstream `remove` — revisited 2026-08-15, and
  settled: no.** This was carried through `yqr-f013` §3.2, `yqr-f014` §3.4 and
  `yqr-b004` §6.4 as "cheap to revisit". It has now been revisited on the 0.0.22
  pin, deliberately and on its own rather than inside another change, so it is
  no longer an open question.

  **Measurement.** `delete` was routed to `Document::remove` on a throwaway
  branch and the whole suite run. **161 of 163 lib tests pass**, and every
  integration suite passes untouched (46 corpus, 17 cli, 4 fidelity, 2
  integration). The two failures are `refuses_a_flow_collection_item` and
  `refuses_a_root_flow_collection_item_with_a_clear_message` — and neither is a
  behaviour difference: upstream *also* refuses, returning `YqrError::Eval`,
  and both fail only the assertion that the message names the flow collection.
  Every `b006` case agrees: head-comment absorption, multiple contiguous head
  comments, a detached comment correctly *not* absorbed, a keep-chomped `|+`
  scalar's kept blanks, not eating the next sibling's comment, same-column
  block sequences, both sole-entry refusals, and the anchor/alias guard.

  So the reason previously on record — that delete is where upstream churn
  concentrates — no longer describes the semantics. yqr's own noyalib#226 is
  why. Three reasons that do not depend on upstream being behind keep the
  decision where it is:

  - **The independent implementation is a differential oracle, and it has paid
    twice.** yqr found upstream's trivia bugs by having a second implementation
    that disagreed — that *is* what noyalib#225/#226 were, two of yqr's four
    upstream contributions. Delegating removes the ability to notice the engine
    drifting, in precisely the property yqr sells.
  - **This is not the `yqr-f015` case wearing different clothes.** The CRLF
    workaround deleted there was pure redundancy — a post-pass over the
    engine's own output, doing a job the engine had started doing correctly.
    This module is an *alternative implementation* with its own semantics and
    failure modes. Deleting redundancy is strictly good; swapping
    implementations trades a known risk for an unknown one.
  - **The trade is asymmetric.** The gain is roughly 350 lines of production
    code deleted — written, tested, stable and quiet, the cheapest kind to own.
    The cost is making byte-fidelity-on-edit depend on the mutator with the
    most defect history in the dependency (trivia in 0.0.18, a flow collection
    destroyed at `Ok` in 0.0.21). And the module does not fully go away
    regardless: the flow pre-check has to stay for the diagnostic, which
    §5.1 records as worth keeping on its own merits.

  Reopen only on a *new* argument — not on upstream improving further, which is
  already accounted for above.
- **Collection right-hand sides for `+=` / new-key assignment.** Since
  `yqr-b008` these route through the typed `Emit` tier, which can spell a
  nested collection — so the "collections are not yet supported" refusal is now
  a scope limit, not a backend one. Lifting it is a user-facing surface change
  and belongs here or in `yqr-f008`, with its own tests and docs.
- **Keys that hold `.` or `[` — and it is wider than "creating" one.** Measured
  2026-08-15 on the 0.0.22 pin, against `app.kubernetes.io/name`:
  - yqr's **filter** grammar already addresses such a key —
    `.metadata.labels["app.kubernetes.io/name"]` parses and reads. So the
    missing piece is *not* an escape form in yqr's path syntax, as this item
    previously recorded.
  - That read is **synthetic**, not byte-exact: a `|` block scalar comes back
    re-indented to the emitter's two spaces instead of its authored four. The
    key costs read fidelity, not just write access.
  - **All three write paths refuse**, not only insert: `set_value` and `delete`
    report `cannot address key ...`, and `insert_key` reports `cannot create
    key ...`. `insert_entry_value` can still splice one, because it takes the
    key as an argument rather than in a path.
  - The blocker is **upstream and total**: noyalib's `parse_query_path`
    (`src/path.rs`) has no escape or quoting form at all — it splits on `.`,
    `[` and `*` unconditionally — and every addressing API (`span_at`,
    `key_span`, `set_value`, `remove`, `rename_key`, `swap_items`) goes through
    it. So this is an upstream grammar question on the §2 `PR-with-fix` route,
    or a yqr-owned green-tree walk, not a change to yqr's own path syntax.

  Every `yqr-a002` form inherits the same refusal (`yqr-a002` §7.3). This is the
  common Kubernetes label/annotation case and is worth doing.
- **A multi-line-insert case in the shared corpus.** `yqr-b008` §6: the unit
  tests pin the fix, but `yqr-m003`'s byte-exact `EngineCase` tier has no
  multi-line-string insert, so a backend swap could reintroduce the corruption
  without failing `tests/corpus_validation.rs`.

_(The grammar and the upstream API surface are now both known; the per-slice
criteria live in `yqr-a002` §9. What stays open here is the scope and
addressing work above, which `yqr-a002` §8 explicitly does not decide.)_
