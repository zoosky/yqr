# Feature f007 — Write tier: structural edits (the `b004` gaps)

**Status:** In Progress (structural **delete** shipped on the interim
`replace_span` fallback; comment editing, key rename, and sequence reorder
remain deferred)
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
  (`b004` 2.1; comments are read-only in 0.0.14). **Deferred** (§6).
- **Key rename** — `.a.b` key renamed in place, preserving `:` and value
  (`b004` 2.2). **Deferred** (§6).
- **Sequence reorder / move / swap** — reorder block-sequence items (`b004` 2.3).
  **Deferred** (§6).

## 2. Dependencies & approach

- **Preferred:** upstream noyalib mutators — reported as umbrella issue
  noyalib#221 and contributed as PRs-with-fix on the #118/#123 precedent
  (`b004` §5). All five now exist on noyalib's unreleased `feat/v0.0.18`
  as `set_inline_comment`/`set_leading_comment`, `rename_key`,
  `swap_items`/`move_item`, a `remove` that handles multi-line and nested
  values, and the `Emit` insertion tier. yqr calls the guarded API
  directly once 0.0.18 ships (`b004` §6).
- **Interim:** where an upstream API is not yet available, yqr performs the edit
  via raw `Document::replace_span`, owning the indent/quote/line arithmetic
  itself, behind an **integrity guard yqr enforces** (§3). `replace_span`
  guarantees only that the result is *valid YAML*, not that it preserves
  structure (`b004` 2.5), so the guard is yqr's, not the backend's. Each such
  fallback is called out in code and tests as a temporary path.

## 3. Structural-integrity contract

Identical to `f006` §7: an accepted edit changes only the targeted node's bytes;
an edit that would restructure the document is refused (exit 5); `-i` leaves the
file untouched on refusal. Because `replace_span` does not enforce this, each
interim fallback must **prove** the property before committing: apply the edit to
a private copy, re-parse it, and commit only if the re-parsed document equals the
original value with exactly the target change applied — otherwise refuse, leaving
the document untouched.

## 4. Acceptance criteria

- [x] Delete of a multi-line / nested node, byte-exact elsewhere; a clear error
      where still unsupported (sole-entry, flow).
- [x] Every interim `replace_span` fallback is guarded and covered by a
      byte-exact test.
- [ ] Comment set/insert/remove at a resolved path, byte-exact elsewhere.
- [ ] `.old |= key-rename` (final syntax TBD) renames a key, preserving value +
      trailing comment.
- [ ] Sequence reorder/move/swap by index, re-parse-guarded.

## 5. Structural delete (shipped)

### 5.1 Surface

No new grammar: `del(<path>)` already parses (`f006`). f006 routed every delete
through noyalib's `Document::remove`, which handles only single-line block
entries and refuses everything else. f007 keeps `remove` as the first choice and
adds a **fallback** for the entries it refuses:

```
del(path) -> Document::remove(path)              # single-line block entry
          └─ on refusal -> delete_structural()   # multi-line / nested (interim)
```

`delete_structural` lives in `src/fidelity/write/delete.rs`, extending
`NoyalibWriter` (the value-write trait stays in `write.rs`; the byte-arithmetic
concern is a sibling sub-module).

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

## 6. Deferred gaps (roadmap)

The remaining three gaps each need **new user-facing grammar** the epic has not
settled, plus harder byte arithmetic; they stay deferred and continue to error
with a clear "not yet supported" message:

- **Comment editing** (`b004` 2.1) — needs a comment-addressing syntax and
  `#`-prefix / whitespace fixup over `comments_at` + `replace_span`.
- **Key rename** (`b004` 2.2) — needs a rename syntax and the key-token span
  (`span_at` resolves the *value* span, not the key), then `replace_span`
  preserving the `:`, value, and trailing comment.
- **Sequence reorder** (`b004` 2.3) — needs a reorder syntax and a multi-splice
  that re-bases offsets after each move.

Each is a `PR-with-fix` candidate upstream first (the preferred path, §2); the
interim `replace_span` approach applies if the upstream API does not land.

_(These criteria firm up once the grammar and the upstream API surface are
known.)_
