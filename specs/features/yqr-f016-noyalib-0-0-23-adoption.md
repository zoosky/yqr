# Feature f016 — Adopt noyalib 0.0.23: the extended `remove`, and the two deletes yqr still refuses

**Status:** Draft (blocked on the release — 0.0.23 is tagged in upstream's
changelog and `Cargo.toml` but is not published; see §2.1)
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

### 2.1 The release does not exist yet

Verified 2026-08-16, and this is what blocks the feature:

| Signal | State |
|---|---|
| PR [#268](https://github.com/sebastienrousseau/noyalib/pull/268) | merged 2026-08-16T05:45:07Z |
| Issue [#221](https://github.com/sebastienrousseau/noyalib/issues/221) | closed 2026-08-16T05:45:09Z |
| `crates/noyalib/Cargo.toml` on `main` | `version = "0.0.23"` |
| `CHANGELOG.md` on `main` | `## [v0.0.23] - 2026-08-16` |
| git tag `v0.0.23` | **absent** (latest is `v0.0.22`) |
| GitHub release | **absent** (latest is `v0.0.22`, 2026-08-14) |
| crates.io sparse index | **absent** (latest published is `0.0.22`) |

So "ships in v0.0.23" is forward-looking. The pin cannot move until the crate
is published. Everything in §4 can be measured before then against a temporary
git dependency — the route `yqr-b002` used before 0.0.14 — but the git dep must
not be committed as the pin (`yqr-m004`: a published yqr cannot depend on a git
source).

## 3. What this release does **not** fix

`yqr-b010` — `swap_items` / `move_item` exchange value bytes only, so a
reorder silently re-attributes every comment in the range at exit 0. Verified
2026-08-16 against `v0.0.22...main`: the only source file the diff touches is
`crates/noyalib/src/cst/document.rs`, and its hunks are `remove` plus a new
`Removal` enum and `flow_member_range` helper. Neither `swap_items` nor
`move_item` appears in the diff, and both resolve through `span_at` +
`replace_span`, which are unchanged for them.

So the `yqr-a002` §9 slice 3 block survives this release. With `#221` now
closed, `yqr-b010` needs its own upstream issue rather than a comment on a
closed umbrella.

## 4. The measurement `yqr-f007` §6 needs re-run

`yqr-f007` §6 settled "delegate `delete` to upstream `remove`: **no**" on the
0.0.22 pin, and closed with "reopen only on a *new* argument — not on upstream
improving further, which is already accounted for above."

This is a new argument, and not the pre-declined one. The recorded measurement
says its only two failures were `refuses_a_flow_collection_item` and
`refuses_a_root_flow_collection_item_with_a_clear_message`, and that neither is
a behaviour difference because "upstream *also* refuses, returning
`YqrError::Eval`, and both fail only the assertion that the message names the
flow collection."

**That premise is now false.** Upstream no longer refuses either class. So the
two tests no longer measure a diagnostic difference; they measure yqr refusing
what the backend can do.

Re-run on 0.0.23, recording per test rather than in aggregate:

- [ ] `delete` routed to `Document::remove`, whole suite, against the published
      0.0.23.
- [ ] The two flow tests classified: does upstream now *succeed* where yqr
      refuses, and is upstream's output the one `yqr-a001` would require?
- [ ] The `yqr-b006` trivia cases re-checked, since `resolve_span` and
      `entry_line_span` both changed in this release. §6.1's four divergences
      were fixed by yqr's own noyalib#226; this is the first release since to
      touch that code.
- [ ] The trailing-newline fix confirmed against yqr's byte-exact suite — it is
      an `a001` property, and yqr's fidelity harness is the natural oracle.

## 5. The decision this feature owes

`yqr-f007` §5 refuses two delete classes — an item of a flow collection, and
the sole entry of a block — with `delete_entry` naming each. Both are now
supported upstream. Three options, and this feature picks one on §4's evidence:

1. **Delegate the two classes only.** Keep `delete_entry` for what it already
   does; route flow members and sole entries to upstream `remove`. Smallest
   change, and it keeps the differential oracle for the common path — but it
   makes the delete surface two implementations with one seam.
2. **Delegate delete entirely**, reversing `yqr-f007` §6. The trade §6 recorded
   still applies (an independent implementation is what made noyalib#225/#226
   measurable), but its "and yqr loses nothing" half is now "and yqr gains two
   classes".
3. **Implement both classes in `delete_entry`.** Keeps one implementation and
   the oracle, at the cost of writing separator arithmetic yqr does not have —
   the exact "own the arithmetic" route `yqr-f007` §2 calls the last resort.

Option 3's cost is not hypothetical: upstream's own note that a multi-line flow
collection still refuses, and that the sole-entry case is a *type* question
(`a:` re-parsing as null) rather than a byte question, is the shape of the
problem yqr would be taking on.

**Not decided here.** §4 runs first; a decision recorded before the measurement
would be the thing `yqr-f007` §6 was careful not to do.

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

- [ ] 0.0.23 published to crates.io; the pin moves and `Cargo.lock` shows that
      one crate moving and nothing else.
- [ ] §4's measurement run and recorded per test, not in aggregate.
- [ ] §5's decision taken, recorded here and reflected in `yqr-f007` §5/§6.
- [ ] `cargo audit` clean; both fidelity harnesses pass; the byte-exact
      `EngineCase` tier passes untouched.
- [ ] If a delete class is delegated or implemented, it gains a byte-exact test
      per shape (flow member first/middle/last, sole mapping entry, sole
      sequence item, single-key document) and the trailing newline is asserted.
- [ ] `yqr-b010` filed upstream as its own issue, since `#221` is closed.
