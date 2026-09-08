# Feature f033 — Split `src/fidelity/write.rs` into its directory module

**Status:** Draft — filed 2026-09-08 from the code review of `yqr-f032`
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

Not decided here — the point of filing is that the shape deserves thought
rather than a drive-by move of whichever function was touched last. The
seams the file already has:

| candidate | what it holds |
|---|---|
| the seam | `FidelityWriter`, `Replacement`, the trait docs |
| the driver | `apply`, `apply_to_doc`, `append_item`, the no-op guards |
| the type-change guards | `refuse_scalar_to_collection`, `refuse_block_collection_to_scalar`, `Integrity`, `check_integrity`, `after_properties` |
| the backend | `NoyalibWriter` and its `FidelityWriter` impl |
| comments | `set_comment` / `current_comment` / `remove_comment` and `check_comment_site` |

Tests move with the code they cover, which is most of the 1085 test lines
and the reason the file reads as larger than it is.

Whatever the split, `pub use` re-exports keep `crate::fidelity::write::…`
paths working, so no caller and no test import changes.

## 4. Acceptance criteria

- [ ] No file under `src/fidelity/write/` exceeds ~500 lines of
      production code, `mod.rs` included.
- [ ] Import paths are unchanged: no caller outside the module is edited.
- [ ] The suite is green with no test edited except for its module path.
- [ ] `yqr-m002` §8 is updated to describe the layout that exists.
- [ ] One commit that moves code and one that changes it, never the same
      commit, so the diff is reviewable.
