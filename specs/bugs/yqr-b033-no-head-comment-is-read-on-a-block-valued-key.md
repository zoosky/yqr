# Bug b033 — A head comment above a key whose value is a block collection reads as null

**Status:** Resolved **2026-09-18** by `yqr-f037` (noyalib 0.0.45,
carrying yqr's noyalib#442). Filed 2026-09-10, found while measuring
`yqr-b032`. Re-measured unchanged on 0.0.44 (`yqr-f035`), as §3 predicted.
0.0.45 measures a leading comment from the entry's key line, so the §1
reproduction reads the comment for all three value shapes (§7)
**Severity:** Low — a read reports nothing where a comment plainly sits.
It is a total read returning the safe answer, so nothing is corrupted and
no write is misled. The cost is that the comment is invisible to
`head_comment`, and to anything built on it
**Component:** read tier — `src/fidelity/noyalib.rs` `comment_body`, and
upstream `Document::comments_at`
**Related:** `yqr-b032` (found here; the same reader is what could have
caught it), `yqr-f007` §4.4 (a read must be total), noyalib#426 (fixed the
sibling shape, not this one), noyalib#442 (the fix), `yqr-f037`

## 1. Summary

A comment above a key is read only when that key's value is a scalar.

```console
$ printf '# doc for k\nk: 1\n' | yqr -r 'head_comment(.k)'
doc for k

$ printf '# doc for k\nk:\n  n: 1\n' | yqr -r 'head_comment(.k)'
null

$ printf '# doc for k\nk:\n  - 1\n' | yqr -r 'head_comment(.k)'
null
```

The three documents put the comment in the same place. Only the value's
shape differs, and the comment is not the value's.

## 2. Cause

Two readers anchor on different things and yqr refuses to guess when they
disagree.

- yqr's `attached_head_len` anchors on `key_span`, so it measures from the
  `k:` line and counts one comment above it.
- Upstream's `comments_at` anchors on `span_at`, the **value's** span. For
  a block collection that span starts on the *next* line, so the line
  above it is `k:` — content, not a comment — and the run ends before it
  starts. `before` comes back empty.

Measured on the pinned 0.0.41, and again on 0.0.44 with the same result:

| document | `span_at("k")` | `key_span("k")` | `comments_at("k").before` |
|---|---|---|---|
| `# doc for k` / `k: 1` | `(15, 16)` | `(12, 13)` | `[" doc for k"]` |
| `# doc for k` / `k:` / `  n: 1` | `(15, 21)` | `(12, 13)` | `[]` |

yqr then takes the `owned > before.len()` branch in `comment_body` and
returns `None`. That branch is correct and deliberate: it exists because
taking a tail longer than the list panicked, and `yqr-f007` §4.4 says a
read must be total. It is doing its job over a disagreement it should not
have to see.

## 3. Why it is upstream's to fix

The same diagnosis noyalib#426 makes for an entry with no value at all:
the comment API asks for the *value's* span where it should ask for the
*entry's*. #426 introduces `comment_anchor_span` and routes the six
`annotated.rs` call sites through it, which fixes the implicit-null shape.
It does **not** fix this one: a block collection has a value span, so the
helper returns it unchanged and the anchor still sits a line too low.

So this is the second member of the family, and the fix is the same idea
carried one step further: a mapping entry's comment anchor is its key
line, whatever its value looks like. Whether that belongs in
`comment_anchor_span` or beside it is the maintainer's call, and worth
raising once #426 has an answer rather than opening a second thread on the
same design question.

#426 merged 2026-09-16 and shipped in 0.0.44. The §2 table re-measured on
that release is identical: `head_comment(.k)` reads the comment above
`k: 1` and `null` above the same comment on `k:` / `  n: 1`. So the
answer is in, the helper exists, and raising the block-collection case
against it is now the next step rather than a second thread.

## 4. What it cost

`yqr-b032` is a write that detaches a head comment from an unrelated key.
yqr's reader can see that when the robbed key holds a scalar, and cannot
when it holds a block collection, which is the more common shape in a
Kubernetes or Helm file. So the corpus case for `b032` had to be built
around a scalar-valued key to have anything but a byte assertion.

More generally, a tool that reads a manifest's comments to carry them
somewhere sees nothing above `spec:`, `metadata:` or `resources:`, which
is where the interesting comments in such a file usually are.

## 5. Not fixed locally

yqr could anchor its own scan and skip upstream's `before` entirely. It
should not. The two readers disagreeing is the defect; making yqr's the
authority would hide it and hand yqr a second implementation of comment
attachment to keep correct, which is what `yqr-f016` §5 exists to prevent.
The refusal branch is the right behaviour until the anchors agree.

## 6. Tests

`a_head_comment_above_a_block_valued_key_is_the_keys` in
`src/fidelity/noyalib.rs` reads the comment above a mapping, a sequence
and a multi-entry mapping, and checks that a comment on the block's first
child is reported for the child only.
`a_head_comment_read_is_total_even_when_the_two_counts_disagree` beside it
now asserts the alias-valued entry reads its comment, and covers the one
disagreement left. In `tests/cli.rs`,
`head_comment_edits_a_block_valued_entry` and
`deleting_a_parents_head_comment_leaves_its_first_childs_alone` pin the
write side. Three corpus cases flipped (`yqr-f037` §2.5).

## 7. Resolution

noyalib#442 adds `leading_comment_anchor`: the key token when the path
names one, and `comment_anchor_span` otherwise. `comments_at` and both
leading-comment mutators measure from it. This is §3's "carried one step
further", placed beside `comment_anchor_span` rather than inside it, so the
inline comment still anchors on the value's line. The maintainer carried
the commit verbatim into release PR #443.

On 0.0.45 the §2 table's last column reads `[" doc for k"]` for both
documents, yqr's `attached_head_len` and upstream's `before` agree, and
`comment_body` returns the comment without a code change on yqr's side.
§5 held: yqr kept one implementation of comment attachment, and the
disagreement it refused to hide is gone at the source.

Two effects beyond §1, both measured in `yqr-f037` §2.2:

- `head_comment(.k) = ...` and `del(head_comment(.k))` on a block-valued
  key go through. Upstream's leading-comment mutator refused multi-line
  entries until this change.
- On 0.0.44 upstream reported a comment on the block's first child as the
  parent's too, and its remover deleted it when asked about the parent.
  yqr's count guard stopped that, with a misleading message. The guard now
  never sees it.

§4's cost is paid back: `yqr-b032`'s corpus case could now use a
block-valued key. It stays on a scalar, which still demonstrates the
damage, and its comment in `tests/corpus/docs.rs` records why.
