# Bug b031 — An entry left empty cannot be deleted or commented, though it can be read, written and renamed

**Status:** Resolved — 2026-09-09. The delete face is fixed: the range
comes from the key token when the value owns no bytes, and the one shape
with neither span, an empty sequence item, is delegated. The comment face
**stays refused**, because it cannot be fixed here (§4); what changed is
that the refusal says which case it is and names a value to write first,
and the upstream half is filed
**Severity:** Low — an ordinary edit at a resolvable path is refused, and
the message describes an internal step rather than anything the user can
act on. Nothing is corrupted, and the file is left untouched
**Component:** write tier — `src/fidelity/write/delete.rs` (delete) and
`src/fidelity/write.rs` (`check_comment_site`)
**Related:** `yqr-b021` (the write face of the same shape, fixed upstream
in noyalib 0.0.28), `yqr-b022` (a `:` at end of input, its sibling),
`yqr-f032` §5 (where this was recorded before it was filed), `yqr-f007`
§5 (the standing argument for keeping delete in yqr's own code)

## 1. Summary

An entry written with nothing after the colon is an implicit null. yqr
reads it, writes into it and renames its key, but cannot delete it:

```console
$ printf 'k:\nafter: 1\n' | yqr 'del(.k)'
yqr: runtime error: cannot delete k: cannot locate its bytes
$ echo $?
5
```

The same entry written out explicitly deletes without complaint:

```console
$ printf 'k: null\nafter: 1\n' | yqr 'del(.k)'
after: 1
```

So the two spellings of the same value behave differently, which is the
shape `yqr-b021` and `yqr-b022` both had. The message is the other half of
the defect: "cannot locate its bytes" describes a step inside yqr, not
anything the user did or can change.

## 2. Extent, measured

Every shape of the empty entry is refused, and the class is wider than
delete.

| operation | `k:` (left empty) | `k: null` |
|---|---|---|
| read `.k` | `null` | `null` |
| assign `.k = 1` | writes `k: 1` (`yqr-b021`) | writes |
| rename `key(.k) = "z"` | renames | renames |
| **`del(.k)`** | ~~refused~~ **deletes** (§4) | deletes |
| **`line_comment(.k) = "why"`** | refused, and it stays refused — the reason is upstream's (§4.2) | sets |
| `validate` | valid | valid |

Refused in every layout: with a trailing comment (`k:   # todo`), with
trailing spaces, nested (`a.b`), as the last entry of a file, as the sole
entry of a mapping, and as an empty sequence item (`-` with nothing after
it).

**One correction to the table above.** It was written as though one change
would fix both faces. It does not: the delete face was yqr's own, the
comment face is upstream's, and they part company in §4.2.

## 3. Cause

Both faces read the same thing and find nothing there.

- `delete_entry` derives the range it removes from `span_at(path)`, the
  **value's** span. An absent value has none, so the `ok_or_else` on that
  lookup produces the message above (`delete.rs:162`).
- `check_comment_site` refuses when `span_at(path)` is `None`, which is
  correct for the cases it was written for — a merge-produced key, an
  alias site — and wrong for this one (`write.rs:757`).

The entry itself is perfectly locatable: `key_span` resolves for every
mapping shape above. It is only the value that has no bytes, and neither
operation needs the value's bytes to do its job. Deleting an entry needs
the entry's range, and the key is where that starts.

## 4. The engine already does the delete

Measured on the pinned noyalib 0.0.41, `Document::remove` handles every
shape yqr refuses:

| input | `remove` result |
|---|---|
| `k:\nafter: 1\n` | `after: 1\n` |
| `k:   # todo\nafter: 1\n` | `after: 1\n` — the comment goes with the entry |
| `a:\n  b:\n  c: 1\n` (`a.b`) | `a:\n  c: 1\n` |
| `xs:\n  - 1\n  -\n` (`xs[1]`) | `xs:\n  - 1\n` |

`span_at` returns `None` for all four while `key_span` resolves for the
three mapping ones, which is the same asymmetry yqr's own code trips on.

## 5. The route taken

**Both, split by whether yqr has an implementation that can disagree.**
That is `yqr-f016` §5's own rule applied one level down.

### 5.1 Mapping shapes keep yqr's implementation

The range comes from `key_span` when `span_at` is `None`, through a new
infallible sibling of `owned_line_span`. It needs no backward marker scan,
because the key token *is* the entry's first content byte rather than
something below it, and no multi-line extent, because a value occupying no
bytes occupies no lines. Everything else in `delete_entry` is untouched:
`target_seq_len`, `empty_collection` and the typed yardstick all come from
the typed model, not the span.

Delegating these instead was measured and rejected on a fact the filing did
not have. For every single-line mapping entry upstream takes an **unguarded
fast path**, with no re-parse and no typed oracle. Delegating would have
moved five of six shapes out from under yqr's structural-integrity
contract. A test pins that: `del(.k)` on `k:` / `k: 1` still fails the
yardstick.

### 5.2 The empty sequence item is delegated

It has no anchor at all: `key_span` and `span_at` are both `None`, and the
only route left is a column-counting scan from the parent, which is the
indentation heuristic `yqr-b006` removed. That would be a second copy, not
a second opinion. Upstream locates the dash from the zero-width leaf it
keeps and is on its guarded path here, since the fast path needs a key span
an index never has.

**The sole empty item is refused rather than forwarded.** Upstream refuses
it too, but as a parse error over a document that parsed fine, which is the
`yqr-b024` shape. yqr's own answer for a sole item is to write the
collection out explicitly, and that needs a range it cannot derive here
either. So the refusal is yqr's, and it names the remedy that works:
delete the whole entry.

The `yqr-b014` risk the plan flagged did not materialise. The worry was
that delegating would route `on:` / `-` through upstream's sole-entry
range for the first time, where nothing downstream could catch a wrong
indent. Upstream refuses that shape, so it never reaches the splice.

### 4.2 The comment face is upstream's, and stays refused

Relaxing `check_comment_site` would not deliver the capability. Measured on
the pinned 0.0.41:

| upstream call | behaviour when the value span is `None` |
|---|---|
| `comments_at` | an **empty bundle**: the `# todo` is not reported at all |
| `set_inline_comment`, `set_leading_comment` | refuse |
| `remove_inline_comment`, `remove_leading_comment` | **`Ok`**, having removed nothing |

So the site check stays, and the only thing yqr owns is which case it
names. It now splits: no value span *and* no key span keeps the old
wording, which is right for a merge-produced key or an alias site; no value
span *but* a key span says nothing is written after the `:` and names a
value to write first.

The remedy is `.k = ""`, not `.k = null`. Assigning null over an implicit
null is an equal-value write, which the `yqr-b018` guard skips, so it
produces no bytes and no comment site. Measured, and the test runs the
remedy rather than asserting the sentence.

**The read side does not move.** `comment_body` has the same guard, so
`line_comment(.k)` on `k:   # todo` reads `null`. If the read reported the
comment while the write refused it, reading a comment and writing it back
would stop being a no-op. A test pins the read with the coupling named, so
whoever fixes one is forced to see the other.

The upstream half is worth filing: `comments_at` and the four mutators
should anchor on the key line for an implicit null, the way `resolve_span`
was taught to keep the zero-width leaf for `yqr-b021`. Not filed yet.

## 6. Why it was not fixed on the spot

Found while writing a refusal message in `yqr-f032`: the
scalar-to-collection refusal wanted to name "remove the entry and write it
again", and that remedy fails for this shape. The feature named no remedy
there instead, which is `yqr-f025`'s rule, and the missing capability was
recorded rather than chased so the feature stayed one change. With the
delete face fixed, that hedge is gone and the ordinary remedy covers it.

## 7. Tests

One per layout in `delete.rs`: baseline, trailing comment, trailing spaces,
nested, last in file and with no final newline, sole entry at two indents,
head comment absorbed, blank-detached block kept, next sibling's comment
kept, CRLF, both delegated sequence positions, the sole empty item's
refusal with its remedy executed, the merge-provided key's new wording, and
a duplicate key that must still fail the typed yardstick. The comment face
has its two refusals in `write.rs` with the remedy run, the read-side pin
in `noyalib.rs`, two CLI tests, and a corpus write case on the document
that ends with a blank value and no final newline.
