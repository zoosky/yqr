# Feature f016 — Adopt noyalib 0.0.23: the extended `remove`, and the two deletes yqr still refuses

**Status:** In Progress — 0.0.23 published 2026-08-17 and **the pin has
moved**; §4's measurement is run and recorded; §5's decision is the one thing
still open
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f015` (the 0.0.22 adoption this succeeds), `yqr-b004` (the
mutation-API gap catalog whose umbrella issue this release closes), `yqr-f007`
§5 (the two delete refusals this release makes optional) and §6 (the
delegation decision whose measurement this release makes stale), `yqr-b010`
(the one defect that survives this release), `yqr-a002` (the addressing
grammar, measured on the 0.0.22 pin), `yqr-m004` (crates.io publishing)

## 1. Scope

Bump `noyalib = "0.0.22"` to `0.0.23` and decide what to do with the two
delete classes yqr currently refuses. Unlike `yqr-f015`, this is not a pin
bump plus a deletion: the release adds a *capability* yqr does not have, so
the feature is a decision first and a bump second.

**In scope:** the pin bump; re-running the `yqr-f007` §6 delegation
measurement, whose recorded result is now stale (§4); deciding whether yqr's
`delete_entry` grows flow-member and sole-entry support, delegates those two
classes, or keeps refusing (§5); recording the `#221` close-out (§6).

**Out of scope:** the `yqr-a002` comment/rename slices — nothing in 0.0.23
touches the comment mutators, `rename_key`, or the addressing grammar, so
`yqr-a002` §5's catalog stands as measured on 0.0.22. Sequence reorder, which
this release does **not** fix (`yqr-b010`, §3). Keys holding `.` or `[`
(`yqr-f007` §6), untouched.

## 2. What 0.0.23 contains

One functional change: `Document::remove` now covers **flow members** and
**sole entries**, the two classes it previously refused. From upstream's
changelog and [the close-out comment on
#221](https://github.com/sebastienrousseau/noyalib/issues/221#issuecomment-5305973890):

- **Flow members.** `a: {x: 1, y: 2}` -> `remove("a.x")` -> `a: {y: 2}`;
  `a: [1, 2, 3]` -> `remove("a[1]")` -> `a: [1, 3]`. The member's span is
  spliced with **exactly one separator** — the comma after it, or for the last
  member the comma before it — so neither `{, y: 2}` nor `{x: 1, }` can
  result. A separator on another line is deliberately not matched: a
  multi-line flow collection refuses rather than splicing bytes it cannot
  account for.
- **Sole entries.** The last entry of a collection empties it explicitly:
  `a:`/`  x: 1` -> `a:`/`  {}`, a lone item leaves `[]`, a single-key document
  becomes `{}`. Deleting the bytes would leave a dangling `a:`, which
  re-parses as **null** — a type change, not a removal.
- A trailing-newline defect found while building it: overwriting a
  collection's span took the document's final newline with it. Upstream places
  it in the same family as the CRLF bug yqr filed as
  [#261](https://github.com/sebastienrousseau/noyalib/pull/261) — valid YAML,
  invisible to a value comparison, and a whole-file diff for a lossless CST.
- `remove_subtree` was **not** added, deliberately: `remove` was extended
  instead, so a second entry point would be a synonym rather than a
  capability. Upstream offers to add one if yqr's fallback needs a distinct
  entry point for a reason the extended `remove` does not cover.

### 2.1 Released 2026-08-17

The blocker this section recorded is gone. Verified rather than assumed:

| Signal | State |
|---|---|
| crates.io | `0.0.23` on the sparse index |
| git tag / GitHub release | `v0.0.23`, published 2026-08-17T00:49:54Z |
| yqr's pin | moved `0.0.22` -> `0.0.23`; `Cargo.lock` shows that one crate and nothing else |
| yqr's suite on the new pin, untouched | **green** — 189 lib, 56 cli, and every other suite |

The bump itself is therefore a non-event, which is the expected shape for a
release yqr contributed to. What the release changes is §4 and §5.

## 3. What this release fixes for yqr

**`yqr-b010` is fixed.** The reorder trivia change landed as yqr's own
contribution (`d397330`, upstream #271) and ships here. Verified against the
published crate, not the branch:

| Case | 0.0.23 |
|---|---|
| inline comments, `swap_items` | travel with the item |
| inline comments, `move_item(_, 0, 2)` | travel |
| head comments, swap | travel |
| `- a\n- b`, no final newline | `- b\n- a`, not `- b- a` |
| flow sequence | value-span swap kept, as designed |
| multi-line items | now supported |
| blank-detached header | stays put |

So the `yqr-a002` §9 slice 3 block is lifted: `swap`/`move` can ship against
this release with the trivia criterion met by the backend rather than worked
around. That is now the only slice of `yqr-a002` still unimplemented.

## 4. The measurement, run 2026-08-17

`yqr-f007` §6 settled "delegate `delete` to upstream `remove`: **no**" on the
0.0.22 pin and closed with "reopen only on a *new* argument". 0.0.23 supplies
one, and the measurement is re-run here per test rather than in aggregate,
because the aggregate is what hid the shape last time.

**Method.** `delete` routed to `Document::remove` on this branch, whole suite
run, patch reverted. The tree as committed contains only the pin bump.

### 4.1 Seven failures, one shape

| Failing test | Family |
|---|---|
| `refuses_a_flow_collection_item` | flow member |
| `refuses_a_root_flow_collection_item_with_a_clear_message` | flow member |
| `refuses_the_sole_entry_of_a_block` | sole entry |
| `refuses_the_sole_top_level_entry` | sole entry |
| `write::tests::sole_entry_delete_is_refused` | sole entry |
| `cli::refused_edit_leaves_the_file_unchanged_under_in_place` | sole entry |
| `integration::sole_entry_delete_is_still_refused` | sole entry |

Every other test passes, in every suite. **Not one failure is a trivia or
fidelity divergence** — each is "yqr refuses, and upstream now succeeds".

This is the fact that makes the argument new. The 0.0.22 record said the only
two failures were flow cases where "upstream *also* refuses ... and both fail
only the assertion that the message names the flow collection". That premise
is gone.

### 4.2 The `yqr-b006` trivia cases still agree

Re-checked because `resolve_span` and `entry_line_span` both changed in this
release, and they are the code yqr's own noyalib#226 fixed. All six agree with
`delete_entry`: a head comment above the entry goes with it, multiple
contiguous head comments go, a **blank-detached** comment is correctly left,
the next sibling's comment is not eaten, a keep-chomped scalar's kept blanks go,
and same-column block sequences close up correctly.

### 4.3 The flow-member output is what `yqr-a001` would want

```text
a: {x: 1, y: 2}   remove("a.x")   ->   a: {y: 2}
a: [1, 2, 3]      remove("a[1]")  ->   a: [1, 3]
a: {x: 1, y: 2}   remove("a.y")   ->   a: {x: 1}
```

Exactly one separator goes with the member, from the correct side. Nothing to
object to.

### 4.4 The sole-entry output strands the entry's head comment

This is the finding, and it is not in the release notes.

Removing the last entry of a collection writes the collection out explicitly,
which is right — deleting the bytes would leave a dangling `a:` that re-parses
as null. But the span it replaces **begins below the entry's head comment**, so
the comment survives and now documents an empty collection:

```text
in                    del(.a.x) via upstream remove
a:                    a:
  # documents x         # documents x
  x: 1                  {}
b: 2                  b: 2
```

Measured in every shape: a single comment, a run of contiguous comments, and a
document-level comment above a single-key document all survive the same way.
An *inline* comment on the entry is correctly removed, since it sits inside the
collection span.

`yqr-b006` is the bug that says an entry owns the comment run directly above
it, and `delete_entry` implements exactly that. Upstream's sole-entry path does
not, so a delegated `del(.a.x)` would leave a comment describing something that
is no longer there — at exit 0, and invisible to the typed oracle, because a
comment is not in the typed value.

That is the `yqr-b006` / `yqr-b010` failure class for the third time, found the
same way each time: by having a second implementation that disagrees.

### 4.5 The trailing-newline fix holds

`only: 1\n` -> `remove("only")` -> `{}\n`, and `a:`/`  x: 1` -> `a:`/`  {}`,
both terminated. The defect upstream found while building this release does not
reach yqr.
## 5. The decision this feature owes — open

`yqr-f007` §5 refuses two delete classes. §4 measured what delegating each
would actually buy and cost, so the options can now be weighed on evidence
rather than on principle. **Not decided here**; recorded for a decision.

| | Flow member | Sole entry |
|---|---|---|
| Upstream output | correct (§4.3) | correct *value*, strands the head comment (§4.4) |
| yqr today | refuses, with its own clearer message | refuses |
| Cost of delegating | none found | a `yqr-b006`-class regression |

That asymmetry is the new information, and it cuts across all three options as
they were originally framed:

1. **Delegate the two classes.** §4 says these are not one decision. The flow
   half is clean; the sole-entry half is not, unless yqr pre-passes the head
   comment itself — at which point it is doing the arithmetic anyway.
2. **Delegate delete entirely.** Now worse than it looked. It buys the flow
   class and takes the §4.4 regression, on the mutator yqr has already had to
   fix twice.
3. **Implement both classes in `delete_entry`.** The one option §4 makes more
   attractive rather than less: yqr already owns the head-comment arithmetic
   that §4.4 shows upstream lacks here, so the sole-entry case is a spelling
   question (`{}` / `[]`) on top of a range it already computes. The flow case
   is the separator arithmetic, which is genuinely new.

A fourth shape §4 suggests and the original framing did not: **delegate the
flow class only, implement the sole-entry class.** Each half goes to whichever
implementation is already correct for it.

Also worth settling alongside it, because it is a user-visible policy and not
just an implementation choice: **should `del` of a sole entry write `{}` at
all?** Upstream's `a:`/`  {}` is semantically right and introduces flow syntax
into a block document; `del(.only)` on a single-key document rewrites the whole
file to `{}`. Refusing, as yqr does today, is defensible for a tool whose
selling point is not surprising you. This is a scope question, not a backend
one, and it survives whichever option above is taken.

**Route.** §4.4 is worth reporting upstream regardless of what yqr does — it is
a real trivia defect in a shipped mutator, of the class upstream has taken a
fix for three times. Filing it does not depend on this decision.
## 6. Upstream close-out

`#221` is closed. Its five sub-asks all shipped, and four of the merged fixes
came from this collaboration:

| # | Ask | Shipped |
|---|---|---|
| 1 | Comment mutation | v0.0.21 |
| 2 | `rename_key` + key spans | v0.0.18 |
| 3 | `swap_items` / `move_item` | v0.0.18 (defective — `yqr-b010`) |
| 4 | Extended `remove` | v0.0.23 |
| 5 | Fragment containment | v0.0.21 |

`yqr-b004` tracked this umbrella from yqr's side and stays Resolved; its §6
adoption findings are unchanged by the close-out, except that §6.4's
"delegation is cheap to revisit" now has a live reason to (§4).

Upstream's standing offer — port yqr's `replace_span` fallback approach and
tests upstream — is recorded in `yqr-f007` §5.1 and is not taken up here.

## 7. Acceptance criteria

- [x] 0.0.23 published to crates.io; the pin moves and `Cargo.lock` shows that
      one crate moving and nothing else.
- [x] §4's measurement run and recorded per test, not in aggregate.
- [ ] §5's decision taken, recorded here and reflected in `yqr-f007` §5/§6.
- [x] `cargo audit` clean; both fidelity harnesses pass; the byte-exact
      `EngineCase` tier passes untouched.
- [ ] If a delete class is delegated or implemented, it gains a byte-exact test
      per shape (flow member first/middle/last, sole mapping entry, sole
      sequence item, single-key document) and the trailing newline is asserted.
- [x] `yqr-b010` fixed upstream and verified against the published crate (§3).
- [ ] §4.4 filed upstream.
