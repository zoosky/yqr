# Bug b013 — An inserted scalar is quoted because some unrelated line is quoted

**Status:** Open (found 2026-08-18 by the `yqr-m003` write tier; not filed
upstream as part of this change)
**Severity:** Low — the value is correct and round-trips; what is wrong is the
spelling at the edit site, which is the one place `yqr-a001` promises an edit
looks like the file it lands in
**Component:** noyalib's `detect_dominant_quote_style` (upstream), reached from
yqr's new-key assignment (`insert_key`) and `+=` (`append`)
**Related:** `yqr-a001` (the fidelity contract), `yqr-b008` (the other defect in
inserted-scalar spelling), `yqr-f006` §7, `yqr-m003`

## 1. Summary

An inserted string is spelled with the document's "dominant" quote style. The
dominance vote counts **only quoted scalars against each other** — plain
scalars do not vote at all — so a single quoted scalar anywhere in the file
makes every subsequent insertion quoted, however plain its neighbours are:

```console
$ printf 'labels:\n  app: web\n' | yqr '.labels.tier = "web"'
labels:
  app: web
  tier: web

$ printf 'quoted: "30"\nlabels:\n  app: web\n' | yqr '.labels.tier = "web"'
quoted: "30"
labels:
  app: web
  tier: "web"
```

The second document differs only in a line the edit does not touch, four lines
above the mapping being edited.

On a real Kubernetes Deployment the vote is decided by `value: "30"` in a
container's env block and `cpu: "1"` in its resource limits — arbitrarily far
from the labels the user is editing.

## 2. Root cause

noyalib 0.0.23, `src/cst/document.rs`:

```rust
fn detect_dominant_quote_style(root: &GreenNode) -> crate::ScalarStyle {
    let mut single = 0_usize;
    let mut double = 0_usize;
    walk_tokens(root, 0, &mut |kind, _| match kind {
        SyntaxKind::SingleQuotedScalar => single += 1,
        SyntaxKind::DoubleQuotedScalar => double += 1,
        _ => {}
    });
    if single == 0 && double == 0 { return crate::ScalarStyle::Plain; }
    if single >= double { crate::ScalarStyle::SingleQuoted } else { crate::ScalarStyle::DoubleQuoted }
}
```

`Plain` is returned only when the document contains **no** quoted scalar at
all. One quoted token out of a hundred plain ones carries the vote. The
intent is right — match the file's conventions rather than impose the
emitter's — but the scope is the whole document, and the count omits the
majority style.

## 3. Impact

- Cosmetic and non-destructive: the inserted value loads back as the string it
  was given, and `validate` is clean either way.
- Visible in every diff of an edited manifest, at the one line the user is
  looking at, and it is the kind of drive-by reformatting `a001` exists to
  prevent.
- Notably **not** what the sibling behaviours do: replacing an existing scalar
  matches its own quote style (`set_value`), and renaming a key matches the
  neighbouring key's. Only insertion reaches document-wide.

## 4. Fix route

Score the vote where the edit lands, then widen: the sibling entries of the
target mapping (or sequence) first, and only fall back to the document when the
site has no quoted or unquoted evidence of its own. Counting `PlainScalar`
alongside the two quoted kinds is the smaller half of the same fix and would
already give the right answer for both examples above.

Upstream, on the `yqr-f007` §2 route. yqr should not pre-empt this with a
post-pass — the engine owns inserted-scalar spelling as of `yqr-b008`, and
`yqr-f015` deleted the last yqr-side patch-up of engine output for good
reasons.

## 5. Regression coverage

Pinned as it behaves, in `tests/corpus/mod.rs`:
`write/insert/new-key-under-a-nested-mapping` (a plain-keyed Kubernetes labels
block gains `tier: "web"`) and `write/append/sequence-item-at-the-site-indent`
(a list of plain feature flags gains `- "billing"`, decided by `zip: "007"`).
Both flip to the plain spelling when this is fixed, and fail until the
expectations are updated.

A related shape that is **not** this bug and is pinned beside them: a multi-line
string whose own lines are indented has no unambiguous block-scalar spelling
without an explicit indentation indicator, so it is emitted as an escaped
double-quoted scalar regardless of the vote
(`write/append/multi-line-item-with-inner-indentation-is-escaped`). That is the
safe answer, and it should survive any fix to the above.
