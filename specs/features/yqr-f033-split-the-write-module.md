# Feature f033 — Split `src/fidelity/write.rs` into its directory module

**Status:** Done — split 2026-09-09. Filed 2026-09-08 from the code review of `yqr-f032`
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** CLAUDE.md ground rule 9 (the 500-line rule), `yqr-m002` §8
(the module layout this would complete), `yqr-f032` (the round that made
the drift worth filing)

## 1. The problem

`src/fidelity/write.rs` is **1259 lines of production code** (2344 with
its tests). Ground rule 9 asks for a directory module past roughly 500:

> Any Rust source file exceeding ~500 lines of production code must be
> split into a directory module (`mod.rs` + sub-modules) with `pub use`
> re-exports to preserve existing import paths.

The directory already exists — `src/fidelity/write/` holds `anchor.rs`,
`delete.rs` and `reorder.rs` — so this is not a new structure to invent.
It is the part of the split that was never finished, and every round since
has added to the file rather than to the directory: `yqr-f032` put roughly
30 more production lines in it.

`yqr-m002` §8 planned the layout and named the trigger ("if it nears ~500
lines, split"), which is the sentence this feature exists to honour.

## 2. Why it is worth doing

The rule's stated reasons, checked against this file:

- **Agent-navigable.** Finding where a refusal lives means reading past
  the seam trait, the driver, three guards, the writer implementation and
  a thousand lines of tests.
- **Merge conflicts in parallel worktrees.** Every write-tier change in
  the last month touched this one file, and several rounds touched it
  twice.
- **`#[cfg(feature)]` gating.** Not live today (yqr has no features since
  `yqr-m005`) but the rule keeps the option open.

Nothing is broken. This is debt with a rule attached, which is why it is a
Draft feature rather than a bug.

## 3. Shape of the split

The point of filing was that the shape deserves thought rather than a
drive-by move of whichever function was touched last. **Decided by what a
piece decides, not by which type owns it**, and the seams the file already
had made that easy to read off:

| file | what it holds | production lines |
|---|---|---|
| `mod.rs` | `apply`, `apply_to_doc`, the small shared helpers, and the shared test helpers | 329 |
| `seam.rs` | `FidelityWriter`, `CommentKind`, `Borrowed` | 226 |
| `guards.rs` | the no-op guards, the two type-change refusals, `Integrity` and the post-write check | 334 |
| `backend.rs` | `NoyalibWriter`, its inherent methods, its `FidelityWriter` impl | 419 |

The three existing sub-modules are untouched: `anchor.rs` 263, `delete.rs`
482, `reorder.rs` 159. Nothing is over the limit.

**One judgment call.** `refuse_block_collection_to_scalar`, `integrity` and
`check_integrity` are `NoyalibWriter` methods only because they need the
document. They go in `guards.rs` by subject, in their own `impl` block,
because keeping the two type-change guards in separate files is what made
them hard to follow in the first place.

**The comment mutators did not get their own file**, which the earlier note
in `yqr-m002` §8 had owed. They are four short methods on the trait impl:
pulling each family into its own file would have left `backend.rs` no
smaller and scattered one type across five files. The rejected seam is
recorded there.

Tests follow their subject, which is most of the 1160 test lines and the
reason the file read as larger than it was. The helpers they share are
defined once, in a `#[cfg(test)] mod testutil` in `mod.rs`, so a change to
how a mutation is built cannot land differently in two files.

`apply` is the only item anything outside the module reaches, and it stays
defined in `mod.rs`, so no re-export was needed and no caller changed.

## 4. What the move needed

Three visibility keywords, and nothing else. `anchor.rs`, `delete.rs` and
`reorder.rs` reached `doc_ref`, `doc_mut` and the free `type_name` through
ancestor-module privacy, which a sibling file does not get, so those three
became `pub(super)`. Everything else is the same code in a different file.

The one trap worth recording: extracting a run of tests by line range
carries the *next* test's `#[test]` attribute with it and leaves the next
test without one. Rust accepts both — a doubled attribute registers the
test twice, and a test with none silently stops running — so neither shows
as a failure. Comparing the full list of test names against `main`, rather
than the count, is what caught it: 143 names against 142, one of them
duplicated.

## 5. Acceptance criteria

- [x] No file under `src/fidelity/write/` exceeds ~500 lines of
      production code, `mod.rs` included. The largest is `backend.rs` at
      419; the largest overall is the pre-existing `delete.rs` at 482.
- [x] Import paths are unchanged: no caller outside the module is edited.
- [x] The suite is green with the set of test names identical to `main`,
      and no assertion edited.
- [x] `yqr-m002` §8 describes the layout that exists, and records the seam
      it had owed and why that one was not taken.
- [x] The code move and the documentation land in separate commits.
