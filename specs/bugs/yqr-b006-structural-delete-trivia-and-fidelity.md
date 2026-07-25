# Bug b006 — Structural delete mishandles comments, blank lines, and same-column sequences

**Status:** Resolved
**Severity:** High
**Related:** `yqr-f007` (structural delete — the shipped code these defects live
in), `yqr-b004` (the noyalib mutation-API gaps `f007` works around), `yqr-a001`
(byte-fidelity property)

> **Resolved.** The `owned_line_span` indentation heuristic was replaced by a
> range derived from noyalib's authoritative value span (`span_at`), the commit
> now goes through the byte-preserving `replace_span`, and a contiguous
> same-indent head comment is folded into the delete. All nine defects below are
> fixed with regression tests; the quality gate is green.

## Summary

The `f007` structural-delete fallback (`src/fidelity/write/delete.rs`) derived
the deleted byte range from an indentation walk and backed it only with a
semantic `Value`-equality re-parse guard. Because `crate::value::Value` carries
no comments or blank lines, the guard is blind to trivia, so several classes of
edit committed a byte-corruption at exit 0. A separate defect refused a common
valid layout, and a cluster of lower-severity issues (error messages, a masked
error, a duplicated walker, a redundant clone) rounded out the review.

## Defects

Byte-fidelity (silent corruption at exit 0):

1. **Following sibling's comment eaten.** A comment indented deeper than the key
   but belonging to the *next* sibling was swallowed by the indentation walk
   (`del(.outer)` on `outer:\n  a: 1\n  # note for next\nnext: 2\n` dropped the
   comment).
2. **Head comment silently re-attributed.** A comment on the line directly above
   the entry was left orphaned onto the following sibling
   (`del(.database)` moved `# database connection settings` onto `cache`).
3. **Keep-chomped scalar's trailing blanks survived.** The blank lines that are
   part of a `|+` block scalar's value were not owned by the entry, leaving a
   stray blank (or whitespace-only) line after the delete.

Functional:

4. **Block sequence at its key's own column wrongly refused.** The
   Kubernetes / GitHub Actions / Ansible list style (`on:\n- push\n- pull_request`)
   was refused with an opaque message because `span_at` under-reports such a
   sequence to just its first `-`.

Robustness / clarity:

5. No byte-level backstop: the re-parsed candidate was committed and re-emitted,
   trusting a parse→emit round-trip not to normalize an untouched node.
6. The `FidelityWriter::delete` trait doc still claimed multi-line/nested entries
   error, contradicting the shipped behavior.
7. A root-level flow collection got the generic "layout not supported" message
   instead of the flow-collection message.
8. The wrapped `remove` error was discarded (`Err(_)`), masking a genuine
   noyalib failure behind the fallback's generic message.
9. A duplicated `Value`-by-path walker, and a full-document clone to build the
   comparison yardstick.

## Fix

- **Derive the owned range from the value's span, not indentation.** `span_at`
  gives an authoritative `value_start..value_end`; the end is the end of the
  line holding `value_end`'s last content byte, except when `value_end` already
  sits at a line boundary (a `|+` scalar's kept trailing blanks), which fixes
  defects 1 and 3.
- **Recover a same-column sequence's end from its last item** (`path[len-1]`),
  fixing defect 4.
- **Fold a contiguous same-indent head comment into the delete**, fixing
  defect 2; a blank-detached comment is left in place.
- **Commit via `replace_span`** (in-place buffer splice) so surviving bytes are
  the original bytes verbatim, fixing defect 5.
- Trait doc corrected (6); root-level flow detection added (7); the `remove`
  error is threaded into the fallback's generic message (8); `walk_value` is
  shared from `noyalib.rs` and `remove_at_path` consumes its input (9).

## Acceptance criteria

- [x] A following sibling's comment survives a delete; a head comment is removed
      with its entry; a blank-detached comment stays in place.
- [x] A keep-chomped (`|+`) block scalar's trailing blanks are removed with the
      entry — no stray blank line.
- [x] A block sequence written at its key's own column deletes cleanly
      (top-level and nested).
- [x] The commit preserves every surviving byte verbatim (`replace_span`).
- [x] A root-level flow collection item is refused with the flow-collection
      message.
- [x] Regression tests cover each case; `cargo fmt`, `cargo clippy -- -D warnings`
      (all feature profiles), and `cargo test` are green.
