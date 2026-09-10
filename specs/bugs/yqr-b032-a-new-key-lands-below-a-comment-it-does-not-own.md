# Bug b032 — Adding a key beneath a nested block steals the next key's comment

**Status:** Open — filed 2026-09-10. yqr ships the defect on the pinned
noyalib 0.0.41. **Fixed upstream** the same day in
noyalib#427; yqr closes this when the release carrying it is adopted
**Severity:** Medium — a write that names one path silently detaches a
comment from a different key. Nothing refuses, nothing warns, and the
file still parses. It is not a byte-fidelity break: every original byte
survives in its original order, which is exactly why no guard sees it
**Component:** write tier — `src/fidelity/write/backend.rs:265`
(`insert_entry_value`), and upstream `Document::mapping_insert_anchor`
**Related:** `yqr-b012` (whose upstream fix caused this, §4),
`yqr-b029` and `yqr-b030` (the same class: a defect no value-based guard
can see), `yqr-b033` (the read that could have caught it, filed from
here), `yqr-a001` (the fidelity contract this sits just outside of)

## 1. Summary

Adding a key to a mapping whose last entry is a **nested block
collection** writes the new key below the comment that follows, instead
of above it.

```console
$ cat config.yaml
a:
  b:
    n: 1
# why z matters
z: 9

$ yqr '.a.c = "2"' config.yaml
a:
  b:
    n: 1
# why z matters
  c: "2"
z: 9
```

The filter named `.a.c`. The comment belonged to `z`, and now belongs to
`c`. yqr's own reader agrees that it changed hands:

```console
$ yqr 'head_comment(.z)' config.yaml
why z matters
$ yqr '.a.c = "2"' config.yaml | yqr 'head_comment(.z)'
null
```

The column-0 comment now sits inside a two-space block, which is the
visible half. The invisible half is the reattachment.

## 2. Extent, measured

On the pinned 0.0.41, against `a:` / `  b:` / `    n: 1` / `# trailing`.

| what is added | result |
|---|---|
| `.a.c = "2"` | new key **below** the comment |
| `.c = "2"` (root) | new key **below** the comment |
| deeper nest under `b` | below |
| nested block **sequence** under `b` | above, correctly |
| two comment lines | below both |
| blank line then comment | below both |
| comment indented **inside** `b`'s block | below, and that is correct |
| last entry is a **scalar** (`b: 1`) | above, correctly |
| flow mapping (`a: {}`) | not this path, unaffected |

So the trigger is narrow: the anchor entry must be a nested block
**mapping**, and a comment must follow it. That covers the ordinary
Kubernetes and Helm shape of a manifest that ends in a commented block.

**Not affected:** the value written is correct, the document re-parses,
`validate --strict` passes, and adding a key then deleting it again
returns the original bytes exactly. This is a placement defect with a
semantic consequence, not a corruption.

## 3. Why nothing catches it

The byte diff is a **pure insertion**:

```diff
4a5
>   c: "2"
```

No existing byte is rewritten, moved or removed. Every yqr guard on this
path asks a question that the damaged document answers correctly:

| guard | what it asks | verdict here |
|---|---|---|
| the typed load-back oracle | does the document load to the value we intended? | yes |
| the re-parse guard | does the result parse? | yes |
| `Integrity` / `bare_line_feeds` | did the line and node counts move as expected? | yes, one line added |
| `validate --strict` | is the document well formed? | yes |
| `yqr-a001` byte fidelity | were unnamed bytes rewritten? | no |

The property that broke is *which node a comment sits above*, and no
value-based check can see it, because a comment is not a value. This is
the third member of the class `yqr-b029` and `yqr-b030` opened.

**One reader would have seen it, and yqr does not use it.** Upstream's
`comments_at("a.c").before` reports `" trailing"` on the damaged
document. yqr's own `head_comment(.a.c)` reports `null` on the same
bytes, because the two anchor on different things. The disagreement is
filed separately as `yqr-b033`, and it costs this bug something concrete:
yqr can only demonstrate the reattachment when the robbed key holds a
scalar, which is the less common shape in the files this happens to.

## 4. Cause, upstream, and where it came from

An insert splices after the mapping's last entry that owns source bytes.
`mapping_insert_anchor` takes that entry's end from the loader's **span
tree**, which runs on to the next token and so sweeps up the blank and
comment lines beneath the entry. The green tree trims a block collection
to its content instead. The two agree for a scalar entry, which has
nothing beneath it to sweep, and part company for a nested block
collection.

**The anchor moved to the span tree to fix `yqr-b012`** — yqr's own
filing, noyalib#288, fixed in noyalib#289 and released in 0.0.25. Before
that it composed each candidate key back into a path string, which no key
containing a `.` survived, so no key could be added beside a Kubernetes
label. The span tree was the right answer to that and carried this with
it. Six releases later it surfaced as noyalib#418, reported by someone
else.

Worth recording plainly: a yqr fix caused a regression that yqr then
shipped for three weeks without noticing, because the only guard that
could have caught it is one yqr does not run.

## 5. The upstream fix

`mapping_insert_anchor` now walks the anchor's lines and stops at the
last one the entry actually owns, rather than taking `end_of_line(end)`.
Indentation decides which trailing comments are the entry's:

- a comment indented **strictly deeper** than the anchor's key sits
  inside that entry's block, so the new sibling goes below it;
- a comment at the key's own column or shallower is not inside it, and
  the sibling goes above.

The equal-column case looked like a genuine tie, and the reporter settled
it: their follow-up names that shape as part of the defect and expects
the new key above the comment. Going above is also what the anchor did
before 0.0.25.

`insert_entry`, `insert_entry_value` and `set_path` all reach the anchor,
so all three are fixed, at either nesting depth and under CRLF. Only the
first two matter to yqr: a path with a missing intermediate level is a
no-op here, not a creation, so yqr never takes upstream's multi-level
route. Twenty-one tests in `cst_insert_anchor_trailing_comment.rs`;
eleven of the first eighteen fail without the patch and the seven that
pass are the controls.

**Filed as noyalib#427**, 2026-09-10, and the root-mapping case rides on
the equal-column rule, since both columns are 0 there.

**Verified by sweep, not only by tests.** 220 shapes through
`insert_entry_value` with and without the patch, checking that the output
differs from the input only by the inserted line and that it re-parses
with the key present. Both hold on every shape that succeeds, either way.

Three shapes change from success to refusal, all a nested block mapping
followed by a comment indented with a literal **tab**. The new key now
lands on the far side of that line, and the result does not re-parse, so
the guard rolls back. The cause is not the anchor: noyalib accepts a
tab-indented comment after a *plain* scalar and rejects it after a
*quoted* one. Filed as **noyalib#428**, with the testbed table — four of
five reference implementations reject both documents, js-yaml accepts
both, and noyalib is the only one that splits. Refusing a document four
parsers call invalid is the better of the two answers, so nothing here
needs a workaround.

## 6. What yqr does now

**Nothing is patched locally.** There is no yqr-side workaround worth
having: yqr does not compute the anchor, and reimplementing it would be a
column-counting scan from the parent, which is the indentation heuristic
`yqr-b006` removed. The same argument `yqr-b031` §5.2 makes for the empty
sequence item applies here.

Instead the behaviour is **pinned as it currently is**, per `yqr-m003`:
a corpus write case and unit tests assert today's wrong placement, with
the expected placement written beside it in a comment. Adopting the fixed
release flips those assertions, which is the signal that the adoption
actually delivered something.

## 7. Tests

- A corpus `WriteCase` on a document whose last entry is a nested block
  followed by a commented sibling, pinning the current placement.
- Unit tests in `src/fidelity/write/backend.rs` for the reported shape,
  the root-level shape, the scalar-anchor control and the
  comment-inside-the-block control.
- A CLI test running the §1 reproduction end to end, including the
  `head_comment(.z)` reading that shows the comment changed hands.
