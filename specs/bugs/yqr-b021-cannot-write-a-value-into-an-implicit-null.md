# Bug b021 — A value cannot be written into an implicit null, and the refusal says the path does not exist

**Status:** Resolved **2026-08-24** — noyalib#310 / noyalib#311 shipped in
**noyalib 0.0.28**, and yqr pins it. Filed 2026-08-22 while fixing `yqr-b020`,
found by a test written to hold `b020`'s check off a case that is not a merge;
patched upstream the next day, released the day after that. See §6
**Severity:** Low — a refusal, not damage; but `a:` → `a: 1` is an ordinary
edit, and the reason given is the same false one `b020` was about
**Component:** `src/fidelity/write.rs`, `NoyalibWriter::set_value` — the
message comes from noyalib's `Document::write_span`
**Related:** `yqr-b020` (the same false reason, different cause; fixed there
for merged keys only), `yqr-b019`, `yqr-f006`, noyalib#310 / noyalib#311 (the
fix), noyalib#312 (a second defect the same measurement turned up)

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

## 4. Fix route, and what was actually done

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

### 4.1 Resolved upstream, 2026-08-23

Reported as **noyalib#310** and fixed by **noyalib#311**. The two questions
§4 wanted settled first turned out to be settled by the same mechanism, and
the fix is smaller than this spec assumed, because it needed no new machinery
at all.

noyalib#165 already marks an implicit null with a **zero-width** `SpanTree`
leaf, and that leaf still carries the position of the `:` / `-` it followed —
which is exactly where a value goes. `resolve_span` was discarding it one
level too early, so every write path read *"no bytes"* as *"no node"*. The
resolver now keeps the position; `span_at` discards it, so #165's read
contract is untouched, and `write_span` turns it into an insertion point one
byte past the indicator.

That byte placement answers the trailing-comment question by construction: the
value lands **before** `# todo`, not after it, where it would have been
commented out. The sequence-item question needed no separate answer either —
the `-` is an indicator like the `:`, so one rule covers both.

The measurement also turned up something this spec did not suspect and yqr
cannot reach: `insert_entry_value` is an upsert everywhere except an implicit
null, where it **appended a duplicate key** at `Ok`. Silent, and invisible to
the load-back oracle, because the loader resolves duplicates last-wins so the
loaded value looks correct. The duplication existed only in the bytes.

Verified against yqr with a local `[patch.crates-io]`: the §5 reproduction
becomes `a: 1`, the sequence-item and trailing-comment shapes work, and the
rest of the suite stays green — with the one exception in §4.2.

### 4.2 One yqr test flips on adoption

`a_mappings_own_entry_is_untouched_by_the_merged_key_check` (`tests/cli.rs`)
asserts that `.a = 1` over `a:` exits **5**. That assertion exists to keep
`b020`'s merged-key message *off* a case that is not a merge, and it pinned
the b021 refusal as a side effect — deliberately, and with a comment pointing
here.

When yqr adopts the fix that assertion is wrong, and the test is the thing
that will say so. Replace the exit-5 half with the write succeeding; keep the
part that checks the merged-key reason does not appear, which is what the test
is actually for.

## 5. Reproduction

```console
$ printf 'a:\nb: 2\n' | yqr '.a = 1'        # exit 5, "path not found: a"
$ printf 'a:\nb: 2\n' | yqr '.a'            # null -- the path resolves
$ printf 'a:\nb: 2\n' | yqr '.a = null'     # exit 0, unchanged (b018 guard)
$ printf 'a:\n  -\n  - 2\n' | yqr '.a[0] = 1'   # exit 5, same wording
```

## 6. Adoption, 2026-08-24

yqr moved its pin from `noyalib = "0.0.27"` to `"0.0.28"`. Every §5
reproduction now writes:

```console
$ printf 'a:\nb: 2\n' | yqr '.a = 1'
a: 1
b: 2
$ printf 'a:\n  -\n  - 2\n' | yqr '.a[0] = 1'
a:
  - 1
  - 2
$ printf 'a:   # todo\nb: 2\n' | yqr '.a = 1'
a: 1   # todo
b: 2
```

The trailing-comment shape is the one §4 could not settle in advance: the
value lands **before** `# todo`, and the gutter the author wrote is still
three spaces wide.

Pinned by four tests, one per shape §4 raised plus the one §4.2 predicted
would flip:

- `a_value_is_written_into_an_implicit_null` (`tests/cli.rs`) — the mapping
  entry.
- `a_value_is_written_into_an_empty_sequence_item` — the `-` with nothing
  after it, whose column a wrong insertion point would move.
- `a_value_written_into_an_implicit_null_lands_before_a_trailing_comment` —
  the shape that decides where the byte goes. Getting it wrong comments the
  value out and still exits 0, so it is the case worth a test of its own.
- `write/assign/fills-in-an-implicit-null` (`tests/corpus/mod.rs`) — the same
  write on the corpus's blank-tail fragment, which asserts every byte the case
  does not name comes back unchanged and that the result validates cleanly.

`a_mappings_own_entry_is_untouched_by_the_merged_key_check` flipped exactly as
§4.2 said it would. The half that pinned this refusal now asserts the write
succeeds; the half that keeps `b020`'s merged-key reason off a case that is not
a merge — what the test is actually for — is unchanged.

The silent duplicate-key defect §4.1 found in `insert_entry_value` is fixed in
the same release. yqr cannot reach that path, so nothing here pins it.
