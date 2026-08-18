# Feature f007 — Write tier: structural edits (the `b004` gaps)

**Status:** Done (structural **delete** shipped, and since `yqr-f013` it is
yqr's whole delete path rather than a fallback; **key rename** shipped
2026-08-16, **comment editing** and **sequence reorder** 2026-08-18, as
`yqr-a002` slices 1, 2 and 3 — §7, §8 and §9. All four `b004` gaps are
closed; what remains under §6 is scope and addressing work this spec never
claimed, tracked there for whoever picks it up)
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f006` (write tier v1 — the value-replacement core this builds
on), `yqr-a002` (the addressing grammar for the three deferred slices),
`yqr-b004` (the noyalib 0.0.14 mutation-API gap catalog) and its §6.6 (the
comment-mutator asymmetries the comment slice must pre-check), `yqr-b010` (the
reorder trivia disagreement that blocked the last slice, fixed by yqr in
noyalib 0.0.23), `yqr-m002` §4/§6.2 (write-tier seam)

## 1. Scope

The surgical edits that have **no first-class API** in noyalib 0.0.14 and so are
excluded from `f006`. Each is cataloged in `yqr-b004` §2:

- **Structural delete** — multi-line, nested, sole-entry, and flow deletes
  (`b004` 2.4). **Shipped** (§5); the last two classes since `yqr-f016` §5.
- **Comment editing** — add / change / remove a comment attached to a node
  (`b004` 2.1; comments were read-only in 0.0.14). **Shipped** (§8).
- **Key rename** — `.a.b` key renamed in place, preserving `:` and value
  (`b004` 2.2). **Shipped** (§7).
- **Sequence reorder / move / swap** — reorder block-sequence items (`b004` 2.3).
  **Shipped** (§9); unblocked by noyalib 0.0.23, which is yqr's own fix
  (`yqr-b010`).

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

- [x] Delete of a multi-line / nested node, byte-exact elsewhere. Sole-entry
      and flow deletes are supported as of `yqr-f016` §5, so the "clear error
      where still unsupported" half of this criterion no longer has a subject.
- [x] Every `replace_span` edit is guarded and covered by a byte-exact test.
- [x] `line_comment(<path>)` / `head_comment(<path>)` set and remove
      (`yqr-a002` §2.4), byte-exact elsewhere. Remove refuses (exit 5) wherever
      set does, rather than inheriting upstream's `Ok`-and-no-op.
- [x] `key(<path>) = "new"` renames a key, preserving value + trailing comment;
      `key(<path>)` reads the document's key token, not the filter's path
      segment (`yqr-a002` §7.2).
- [x] `swap(<path>; i; j)` / `move(<path>; from; to)` reorder by index,
      re-parse-guarded, with each item's comments travelling with the item —
      met by the backend since noyalib 0.0.23 (`yqr-b010`) and pinned here by
      test rather than assumed.

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
3. **Route the two shapes that do not own whole lines**, both supported since
   `yqr-f016` §5 (2026-08-17) and each by a different implementation:

   - An item of a **flow** collection (`[a, b]` / `{a: 1}`, detected from the
     parent's own bytes — including a root-level flow collection, whose parent
     is the document itself) is **delegated** to noyalib's `remove`. Upstream
     owns the separator arithmetic; this module has none, so re-deriving it
     would produce a second copy rather than a second opinion.
   - The **sole entry** of a block is **implemented here**: the collection is
     written out explicitly (`{}` / `[]`) at the entry's own indentation and
     line terminator, since deleting the bytes would leave a dangling key that
     re-parses as `null` — a type change, not a removal. Kept in this module
     because the range to replace includes the entry's head-comment run, which
     upstream's own sole-entry path misses (`yqr-f016` §4.4, noyalib#280).

   The oracle argument decides the split in both directions: keep an
   independent implementation where it can disagree with something, skip it
   where it cannot.
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
  `remove_leading_comment`, single-line nodes only. Measured on 0.0.22,
  upstream places a leading block **above** the addressed entry — the correct
  side, and the opposite of yq. The *placement* is therefore adoptable; the
  calls are not adoptable bare. Three asymmetries need a yqr-side pre-check
  first, catalogued per direction in `yqr-a002` §5.1/§5.2 and recorded as
  adoption findings in `b004` §6.6: the two removers return `Ok(())` on
  everything their setters refuse, `set_inline_comment` guards on the value
  span so a nested entry's comment lands on its **child's** line, and the
  leading mutators absorb a blank-detached comment block that this module's own
  `delete_entry` deliberately excludes.
- **Key rename** (`b004` 2.2) — **shipped 2026-08-16**; the record moved to
  §7 and this bullet is kept only so the three-gap list stays readable.
- **Sequence reorder** (`b004` 2.3) — **shipped 2026-08-18**; the record moved
  to §9. It was blocked because `swap_items` / `move_item` exchanged value
  bytes only, so a reorder silently re-attributed every comment in the range at
  exit 0 and past upstream's own guard — §5.1's failure class exactly, with
  this module's entry-range arithmetic as the reference implementation for the
  fix. `yqr-b010` carries the measurement and the route; `yqr-a002` §6 carries
  the consequence for the grammar.

The upstream `PR-with-fix` path (§2) already ran its course for all three, and
0.0.22 is pinned (`yqr-f015`), so comment editing and key rename are now
implementation over a live API — with the pre-checks above, which is the part
"two of the three are a call, not a splice" understates. Raw `replace_span`
stays the route of last resort rather than the expected one.

§5.1's reminder earned a third clause along the way: "upstream has the call",
"upstream does what its docs say", and "upstream has yqr's semantics" are three
different questions. The measuring pass answered the second "no" three times
(`yqr-b010` §4, the stale `swap_items` refusals) and the third "no" in nine
rows of `yqr-a002` §5.1–§5.3. Both counts came from probing each call in
**every** direction — set, remove, read — rather than only in the one its
setter takes.

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

  **Reopened 2026-08-16 on a new argument, and it is not the pre-declined
  one.** noyalib closed `#221` by extending `remove` to cover **flow members
  and sole entries** — the two classes §5 refuses — shipping in 0.0.23
  (`yqr-f016` §2; not published yet, so nothing has moved). That invalidates
  this measurement's premise, not just its age: the record above says the only
  two failures were flow cases where "upstream *also* refuses ... and both fail
  only the assertion that the message names the flow collection". Upstream no
  longer refuses either class, so those two tests now measure yqr **declining
  what the backend can do**, which is a different fact from a diagnostic
  difference.

  The three reasons above are untouched by this and still stand; what changed
  is that the "and yqr loses nothing by keeping its own path" half is now "and
  yqr forgoes two delete classes". Upstream also asked directly for the
  measurement — point yqr at 0.0.23 with the fallback disabled and report what
  breaks — which is the same check that made noyalib#261 obviously correct
  rather than merely plausible. Re-run and decision: `yqr-f016` §4/§5. No
  decision is recorded here in advance of it.
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
- **A write tier in the shared corpus.** `yqr-b008` §6 names the specific gap:
  the unit tests pin the multi-line-insert fix, but `yqr-m003`'s byte-exact
  `EngineCase` tier has no multi-line-string insert, so a backend swap could
  reintroduce the corruption without failing `tests/corpus_validation.rs`. The
  cause is one level up — `EngineCase` runs through `fidelity::run`, which
  refuses a mutating filter, so **no** edit is covered by the corpus at all.
  Slices 1 and 2 could add corpus cases because they have read forms; a reorder
  has none, so §9 is unit- and CLI-tested only and adds nothing here. Closing
  this is a third case tier (`WriteCase`) plus its two consumers, not a case.

_(The grammar and the upstream API surface are now both known; the per-slice
criteria live in `yqr-a002` §9. What stays open here is the scope and
addressing work above, which `yqr-a002` §8 explicitly does not decide.)_

## 7. Key rename (shipped)

`yqr-a002` slice 1, and the first use of its addressing grammar. Landed
2026-08-16.

### 7.1 Surface

```text
key(<path>)             read the entry's key token
key(<path>) = "new"     rename it in place
```

`key` is recognised only in **function position** — an identifier immediately
followed by `(`. `.key` is still a field access, and so are `.swap`, `.move`,
`.del`, `.line_comment`, `.head_comment` and `.foot_comment`; one test covers
all seven, because `swap` and `move` are ordinary YAML field names and a
regression there would break read-only queries.

`del`'s argument is unchanged: it still takes a pipeline, so `del(.a | .b)`
parses and deletes exactly as before. What is new is that it takes a *target*,
of which a pipeline is one kind — which is what makes `del(key(...))`
expressible, and therefore refusable with a reason instead of a syntax error.

### 7.2 What it is not

Three near-misses, each refused rather than guessed:

| Form | Result |
|---|---|
| `del(key(.a))` | parse error: a key cannot outlive its entry; use `del(.a)` |
| `key(.a) += 1` | parse error: `+=` appends to a sequence |
| `key(<absent>) = "x"` | no-op, **not** a new key |

The last one is the trap worth naming. Value assignment resolves through
`resolve_assign_target`, whose absent-leaf branch *creates* a mapping key —
correct for `.a.b = 1`, wrong for a rename, where an absent path means there is
no key to rename. The rename uses the plain resolver and skips the document, as
`del` does. The comment slice will meet the same fork.

### 7.3 The read comes from the document

`key(<path>)` goes through `Document::key_span` and the existing read seam, not
through the resolved `Path`'s last segment. `PathSeg::Key` is stored *decoded*,
so the shortcut would echo the filter back: a key authored `"a"` would read as
`a`, and a key reached through a `<<` merge would read as a token that appears
nowhere in the file. Reading the document keeps `key(...)` on the same footing
as every other read — print the bytes that are there — and `key_span` returning
`None` is both the source of the bytes and the test for whether there are any
(a sequence item, an absent path, a merge or alias site).

One edge is documented rather than solved: a key holding `.` or `[` is
unaddressable (§6, `yqr-a002` §7.3), so `key(...)` on one reads `null` — the
same answer as "this node has no key". Reads are total (`yqr-a002` §4.4) and
there is no correct typed fallback, so the honest move was to say so in the
guide. It resolves when the dotted-key item does.

### 7.4 Coverage

Thirteen unit tests on the write path (byte-exactness with a head comment, an
inline comment and odd spacing all preserved; key order kept; quote style
matched; multi-line values untouched; the absent no-op; multi-document skip;
and one per refusal), four on the engine's `key_bytes` (verbatim token,
document-relative offsets in a stream, and `None` for each keyless shape),
nine on the grammar, ten black-box CLI tests including `-i` and the
refusal-leaves-the-file-untouched contract, and three shared-corpus
`EngineCase`s — one of which is a merge-produced key, the case that separates
a document read from an echo.

## 8. Comment editing (shipped)

`yqr-a002` slice 2, the second use of its addressing grammar. Landed
2026-08-18.

### 8.1 Surface

```text
line_comment(<path>)          read the `# ...` on the entry's own line
line_comment(<path>) = "t"    set or change it
del(line_comment(<path>))     remove it

head_comment(<path>)          read the block directly above the entry
head_comment(<path>) = "a\nb" set it, one line per segment
del(head_comment(<path>))     remove it
```

`= ""` writes a bare `#`; only `del(...)` removes (`yqr-a002` §4.2). Reading
strips the `#` and exactly one leading space, which is what makes set and read
exact inverses — the §4.3 round-trip property, pinned as an executable test
over bodies with inner spaces, leading spaces, a `#`, and a `:`.

`foot_comment(...)` parses and is then refused with its reason, which is the
only thing it exists for (`yqr-a002` §8).

### 8.2 What yqr checks that upstream does not

Three pre-checks, each for a measured case where upstream's guard answers a
different question from the one the filter asked. All three were silent wrong
results, not refusals.

| Case | Upstream | yqr |
|---|---|---|
| entry's value starts on the next line | comments the **child's** line; removal deletes the child's comment | refused both ways |
| head block detached by a blank line | replaces / deletes it | refused; read is `null` |
| head block *partly* detached | treats the whole run as the entry's | refused, same check |

The third generalizes the second, and one comparison covers both: yqr computes
the contiguous same-indent run itself and refuses when it differs from
`comments_at().before`. A mismatch means the edit would reach bytes the path
does not name.

### 8.3 The no-op is detected, not enumerated

Upstream's two removers return `Ok(())` on an unresolved path, on a missing
comment, and on every shape their setters refuse — a leading block on a
sequence item being the measured case. `yqr-a002` §4.4 forbids a mutation that
no-ops, so each of those has to become a refusal.

Enumerating the shapes would have been the obvious route and is the wrong one:
the list belongs to upstream and can change under yqr silently. Instead a
removal **checks afterwards** that the comment is gone, and refuses when it is
not. That covers any such shape without knowing which they are, and the
document is already unchanged in exactly that case, so there is nothing to roll
back.

### 8.4 Reads are total, and read the same ownership

`line_comment` reports `null` where the entry has no line of its own;
`head_comment` reports only the run yqr owns, so a detached block reads `null`
while an attached one reads its body. A sequence item's head comment **reads**
(§5.2 requires it) even though writing one is refused upstream — reads and
writes answer to the same ownership rule but not to the same capability.

Two bugs worth recording, both about what a sequence item's *line* is, and
both caught only because something else disagreed:

- The head-comment walk measured an item's indentation from its **value**,
  which sits past `- `, so a comment aligned with the dash never matched and
  every item read `null`. `delete_entry` had this right already; this path had
  to learn it separately.
- The §8.2 value-starts-on-the-next-line check asked only whether the entry had
  a key token on the value's line, and answered "yes, fine" for every sequence
  item — on the reasoning that an item always sits on its own line. A bare `-`
  with the value below it is the counterexample, and it is the mapping defect
  exactly: the comment lands on the child. So the guard this slice exists to
  add was bypassed for half its cases until review found it. Both now resolve
  the marker the same way, key token or `-` indicator.

A third, in the read path: `attached_head_len` counts source lines while
`comments_at().before` comes from the CST, and an alias-valued entry makes them
disagree. Slicing a tail longer than the list **panicked** — on a read
documented as total, which is worse than any error it could have returned. A
disagreement now reads `null`, since it means ownership cannot be established.

### 8.5 Coverage

Twelve write-path unit tests (set, change, remove, bare `#`, multi-line block,
CRLF, absent-path skip, non-string body, and one per refusal), three parser
tests, nine black-box CLI tests including the round-trip property and the
refusal-leaves-the-file-untouched contract, and three corpus `EngineCase`s.

## 9. Sequence reorder (shipped)

`yqr-a002` slice 3, the last of the four `b004` gaps and the only one that is
not a naming function. Landed 2026-08-18.

### 9.1 Surface

```text
swap(<path>; i; j)        exchange two items of the sequence at <path>
move(<path>; from; to)    move one item, shifting the items between by one
```

An ordering has no bytes of its own and no node to name, so there is nothing
for the `key`/`line_comment` wrapper shape to wrap (`yqr-a002` §2.2). It is a
verb with arguments instead, and it costs the grammar one token — `;`, jq's
argument separator, taken because `,` is spoken for by the stream operator on
the M1 roadmap.

`swap` and `move` are recognized in **function position at the start of a
program** only, so they are not reserved words: `.swap` and `.move` still read
fields of those names, which matters because both are ordinary YAML keys. The
existing seven-word parser test covers them, and a CLI test reads a document
whose keys are literally `swap` and `move`.

Indices count from zero and a negative index counts from the end, resolved
through the **same function** `.[-1]` resolves through
(`eval::resolve_seq_index`). That is what makes "as `.[-1]` does"
(`yqr-a002` §4.5) a fact about the code rather than a claim about it.

### 9.2 What is not in the grammar

- **A reorder is not a target.** `del(swap(...))` and `swap(...) = 5` are parse
  errors. Neither has a meaning to guess at: the verb *is* the edit.
- **Indices are integer literals**, not an `Rhs`. A path or a string there
  would have nothing to fall back on, so the refusal names the form.
- **No read form.** There is nothing to print — which is also why this slice
  adds no shared-corpus case (§6), where slices 1 and 2 each added three.

### 9.3 The trivia property is the backend's, and yqr pins it anyway

This slice is one call per verb. That is only true because of `yqr-b010`:
before noyalib 0.0.23, `swap_items` and `move_item` exchanged the two items'
**value bytes** and nothing else, so every comment stayed with the *slot* and
silently came to document whatever item landed in it — at `Ok`, at exit 0, and
past upstream's own integrity guard, which compares typed values and therefore
cannot see a comment move. yqr argued the semantics, wrote the fix, and it
shipped in 0.0.23 (`yqr-b010` §5).

So the criterion "an item's comments travel with it" is met by the engine, not
by this module. It is still a yqr test — `an_items_comments_travel_with_the_item`
in `write.rs` — because a property yqr sells and does not own is exactly the one
worth pinning against engine drift. `yqr-f007` §5.1's reminder in its third
clause: "upstream has yqr's semantics" is a question that has to be re-asked
every release, and a test is how it gets asked automatically.

### 9.4 What yqr adds around the call

Three things, all before the call, all so a refusal is yqr's own rather than an
engine message quoting an engine path:

- **The sequence length**, read from the typed view. yqr needs it up front
  regardless — a negative index resolves against it — so once it is in hand,
  forwarding the range check to upstream would produce a second message shape
  for the same mistake. `yqr-a002` §5.4 records that row as "forwards"; it is a
  yqr pre-check instead, and the reason is the negative-index resolution the
  same section requires. One message covers `i` and `-i` alike, and it names the
  valid range.
- **A path that is not a sequence** is refused naming what it is, since the
  typed walk that produced the length already knows.
- **Path spelling.** Diagnostics quote the path as a filter spells it (`.xs[0]`,
  and `.` for the root), not as the engine does — its root path is the empty
  string, which in a message reads as a missing argument rather than as the
  whole document.

An absent path stays a per-document **skip**, exactly as `del` and the rename
skip it (`yqr-a002` §4.4's one permitted no-op). `swap(.xs; 1; 1)` is *not* that
case and is a plain success: the request has a well-defined result, and the
unchanged document is it.

### 9.5 Coverage

Twenty write-path unit tests: the byte-exact swap and move; comments
travelling with the item (§9.3); negative indices; the root sequence; a
sequence written at its key's own column; multi-line items; a flow sequence
reordered rather than refused; index-with-itself; an anchored item whose alias
still resolves; a blank-detached header left alone; a document with no trailing
newline (the `- b- a` splice `yqr-b010` §5 warns about); CRLF; a block-scalar
item; each refusal, including the empty and single-item sequences; the
multi-document skip and the multi-document apply; and an unaddressable key.
Six parser tests and two lexer tests cover the grammar, including the
`,`-for-`;` mistake, which the lexer answers by naming the separator rather than
the byte. Nine black-box CLI tests cover `-i`, the refusal-leaves-the-file
contract, and `.swap` / `.move` still reading fields; two library tests cover
the public `Mutation::Reorder`, which a caller can build without going through
the grammar at all.

The implementation lives in `src/fidelity/write/reorder.rs`, a sibling of
`delete.rs` under the same rule 9 split — the trait method in `write.rs`
delegates to it. Adding it inline would have pushed `write.rs` further past the
500-line production limit it already exceeds (`yqr-m002` §8 records the split
that owes).
