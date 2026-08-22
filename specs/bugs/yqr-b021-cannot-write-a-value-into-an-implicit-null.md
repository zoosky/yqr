# Bug b021 — A value cannot be written into an implicit null, and the refusal says the path does not exist

**Status:** Open — filed 2026-08-22 while fixing `yqr-b020`, found by a test
written to hold `b020`'s check off a case that is not a merge
**Severity:** Low — a refusal, not damage; but `a:` → `a: 1` is an ordinary
edit, and the reason given is the same false one `b020` was about
**Component:** `src/fidelity/write.rs`, `NoyalibWriter::set_value` — the
message comes from noyalib's `Document::write_span`
**Related:** `yqr-b020` (the same false reason, different cause; fixed there
for merged keys only), `yqr-b019`, `yqr-f006`

## 1. Summary

A block-mapping key with no value is an implicit null. It reads as `null`, and
it cannot be written:

```console
$ printf 'a:\nb: 2\n' | yqr '.a'
null
$ printf 'a:\nb: 2\n' | yqr '.a = 1'
yqr: runtime error: cannot assign at "a": YAML parse error: path not found: a
```

Same for an empty sequence item:

```console
$ printf 'a:\n  -\n  - 2\n' | yqr '.a[0] = 1'
yqr: runtime error: cannot assign at "a[0]": YAML parse error: path not found: a[0]
```

Writing `null` there *is* accepted, because it changes nothing and the
`yqr-b018` guard skips it before the writer sees it. So the operation succeeds
exactly when it does nothing.

## 2. Two faults, and only one of them is the message

**The reason is false**, the same way `yqr-b020`'s was: `.a` resolves, and the
line above prints its value. `b020` fixed that sentence for keys a `<<` merge
produced, deliberately and only there — this is a different cause reaching the
same wrong words.

**And the refusal itself is probably wrong.** Unlike a merged-in key or an
alias site, there is a real entry here, in the source, with a key token of its
own. `a:` → `a: 1` restructures nothing, touches one line, and is the kind of
edit yqr exists for. Filling in a value someone left blank is ordinary work on
a config file.

That second half is what makes this a bug rather than a wording defect, and
why it is filed apart from `b020` rather than folded into it.

## 3. Cause

The node has no bytes of its own, so it has no span to splice over. noyalib's
`resolve_span` reports a zero-width leaf as `None`:

```rust
// noyalib 0.0.27, src/cst/document.rs
SpanTree::Leaf(s, e) if s == e => None,
```

`write_span` turns that `None` into `path not found`, the same fallback
`b020` §3 measured for a merge-produced key. Two distinct shapes, one
undifferentiated answer.

yqr can tell them apart from the public API: an implicit null has a `key_span`
and no `span_at`, where a merged-in key has neither. `borrowed_site` already
relies on exactly that distinction to keep `b020`'s message off this case, and
`a_mappings_own_entry_is_untouched_by_the_merged_key_check` pins it.

## 4. Fix route

The write needs an insertion point rather than a span to replace: the position
just after the `:` (or after the `-`), where a space and the rendered value go.
That is closer to `insert_entry_value` than to `set_value`, and it is upstream
work either way — the same argument as `b020` §4, and the same reason to
measure before filing.

Two things to settle first, neither obvious:

- **A sequence item and a mapping entry differ.** The mapping case appends to
  an existing line. An empty `-` item does too, but the indicator's column is
  the sequence's, so the check that the result re-parses as the same document
  with one value filled in matters more.
- **A trailing comment.** `a:   # todo` has an implicit null *and* a line
  comment. The value goes before the comment, not after it, and getting that
  wrong silently comments out the value.

Until then the refusal stands, and the message should at least stop denying a
path yqr can read. That is a smaller change than the write and could ship
first.

## 5. Reproduction

```console
$ printf 'a:\nb: 2\n' | yqr '.a = 1'        # exit 5, "path not found: a"
$ printf 'a:\nb: 2\n' | yqr '.a'            # null -- the path resolves
$ printf 'a:\nb: 2\n' | yqr '.a = null'     # exit 0, unchanged (b018 guard)
$ printf 'a:\n  -\n  - 2\n' | yqr '.a[0] = 1'   # exit 5, same wording
```
