# Bug b031 — An entry left empty cannot be deleted or commented, though it can be read, written and renamed

**Status:** Open — filed 2026-09-08 from `yqr-f032`'s code review, which
hit the delete face while looking for a remedy to name in a refusal
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
| **`del(.k)`** | **refused** | deletes |
| **`line_comment(.k) = "why"`** | **refused**, "the path does not resolve to a node" | sets |
| `validate` | valid | valid |

Refused in every layout: with a trailing comment (`k:   # todo`), with
trailing spaces, nested (`a.b`), as the last entry of a file, as the sole
entry of a mapping, and as an empty sequence item (`-` with nothing after
it).

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

## 4. The engine already does it

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

## 5. Two routes, neither chosen here

- **Derive the range from the key.** `delete_entry` already computes an
  entry's owned range from spans; starting it at `key_span` when the value
  has none is a local change to one lookup, and the existing re-parse and
  load-back guards still hold it. Keeps delete in yqr's own code, which
  `yqr-f007` §5 argues for on the grounds that an independent
  implementation is a differential oracle and has paid twice.
- **Delegate this class to upstream**, as the *flow* class already is
  (`yqr-f016` §5). The precedent is exact: each half went to whichever
  implementation was already correct, and here upstream is.

The comment face is separate and smaller: `check_comment_site` should ask
whether the **entry** resolves, not whether its value does.

Whichever route is taken, the fix wants a test per layout in §2, since the
sole-entry and same-column-sequence shapes are where delete has gone wrong
before (`yqr-b014`).

## 6. Why it was not fixed on the spot

Found while writing a refusal message in `yqr-f032`: the
scalar-to-collection refusal wanted to name "remove the entry and write it
again", and that remedy fails for this shape. The feature named no remedy
there instead, which is `yqr-f025`'s rule, and the missing capability was
recorded rather than chased so the feature stayed one change.
