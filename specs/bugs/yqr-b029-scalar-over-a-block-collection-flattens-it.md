# Bug b029 — A scalar written over a block collection lands at its key's own column

**Status:** Resolved — guarded 2026-09-08 in `yqr-f032`. yqr refuses the
write before the document is touched, and a post-write integrity check
catches the class generally. The upstream half is drafted in §5 and not
yet filed
**Severity:** High — silent corruption at exit 0, on the default write
path, in a tool whose contract is that it never damages a file it edits
**Component:** write tier (`src/fidelity/write.rs`), scalar assignment
over a collection target
**Related:** `yqr-f032` (the feature whose measurement found it),
`yqr-b014` (the same class: noyalib accepts what other implementations
reject, so no guard downstream of the parse can see it), `yqr-b030`
(found in the same pass), `yqr-a001` §1, `yqr-f012` (`validate`, which
already had the check)

## 1. Summary

Reproduced on the shipped **0.8.0**:

```console
$ printf 'k:\n  a: 1\nafter: 1\n' | yqr '.k = 5'
k:
5
after: 1
$ echo $?
0
```

The value lands at column 0, where the key is. yqr's own `validate`
rejects its own output:

```console
$ printf 'k:\n  a: 1\nafter: 1\n' | yqr '.k = 5' | yqr validate -
error[Y103]: block mapping value is not indented past its key
```

PyYAML and Psych reject the document outright. noyalib reads it back,
which is exactly why nothing caught it.

## 2. Extent, measured

Ten shapes through the shipped binary. What corrupts:

| shape | before | after |
|---|---|---|
| block mapping under a key | `k:` / `  a: 1` | `k:` / `5` |
| block sequence under a key | `k:` / `  - 1` | `k:` / `5` |
| same, with an inline comment on the key | `k:  # why` | `k:  # why` / `5` |
| `\|=` rather than `=` | | identical |
| a path right-hand side, including an **absent** one (`null`) | | identical |

What does not:

- a **flow** collection (`k: {a: 1}` → `k: 5`), which has no line of its
  own to under-indent;
- a **sequence item** (`xs[0]`), which is not a mapping entry;
- a **nested** block collection, which upstream refuses — with
  "inconsistent indentation" over a document that has none, the `yqr-b024`
  shape: a parse error blamed on input that parsed fine.

So the bug is one property, not one shape: replacing a block mapping's
multi-line value with a single-line one leaves the value where the
collection's first line began, and that column is the key's.

## 3. Why every existing guard missed it

- The **re-parse guard** requires the result to parse and to load back as
  the expected value. It does, on this parser.
- **Upstream's own guard** is the same parser.
- **`yqr validate`** would catch it, but nothing ran it on a write's
  output.

`yqr-b014` recorded this class in 2026-08-18 for a *delete*, and the
conclusion there was to keep that class in yqr's own code. The same
finding applies to assignment and nobody had looked.

## 4. The fix

Two parts, both in `yqr-f032`:

- **A pre-check** (`refuse_block_collection_to_scalar`). The typed current
  value says the target is a collection, the source bytes say it is not a
  flow one, and the path's last segment says it is a mapping entry. Refused
  with a message that names the remedy: remove the entry and write it
  again, which works because a new key is the insertion path.
- **A post-write integrity check** (`check_integrity`), which counts
  `Y103` sites on either side of every `set_value` and refuses a write
  that adds one. It reuses `validate`'s own scanner
  (`validate::scan::under_indented_values`), so the two commands cannot
  drift apart in what they consider a broken document.

The pre-check exists for the message; the post-check is the net, and it
covers shapes nobody has thought of yet.

## 5. Upstream (drafted, not filed)

noyalib's `set_value` resolves a block collection's span to a range that
starts at the collection's first line, indent included, and splices the
scalar into it. Either it should refuse the shape the way it refuses a
scalar replaced by a collection, or it should collapse the entry onto the
key's line (`k: 5`), which is what every other implementation would read
back. The nested case shows it already refuses *sometimes*, with a message
that blames the input, so the shapes disagree with each other.

Reproduction for the filing:

```rust
let mut d = noyalib::cst::parse_document("k:\n  a: 1\nafter: 1\n").unwrap();
d.set_value("k", &noyalib::Value::from(5)).unwrap();
assert_eq!(d.source(), "k: 5\nafter: 1\n"); // is: "k:\n5\nafter: 1\n"
```

## 6. Tests

`a_scalar_over_a_block_collection_is_refused` walks the three corrupting
shapes; `a_scalar_over_a_flow_collection_or_a_sequence_item_still_writes`
holds the two that must keep working, which is what stops the guard from
becoming a ban on the type change;
`an_absent_right_hand_path_does_not_flatten_a_block` covers the shape
reached without naming a scalar at all. One corpus write case pins the
refusal on the deployment.
