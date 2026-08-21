# Bug b016 — An emitted block collection reached through a sequence item keeps a trailing space


> **Historical: resolved.** yqr no longer behaves as described below. The
> **Status** line records what fixed it and when; the rest is kept as the
> reproduction and the reasoning, written in the present tense of the time it
> was filed.

**Status:** Resolved — found 2026-08-20 by `yqr-f017`'s first output,
pre-existing and reachable through `--normalize` since long before it, filed
2026-08-21 as noyalib#297 with a fix in noyalib#298, and **released in noyalib
0.0.27** the same day. Verified against the published crate by `yqr-f023` §2.1,
both faces, with the controls byte-identical to 0.0.26. The pin `yqr-f017`
left in `tests/cli.rs` is what caught the change; it is now flipped and joined
by the block-scalar face and the string-owns-whitespace control
**Severity:** Low — the output is valid YAML that loads back correctly; what is
wrong is a trailing space on a line that should not have one
**Component:** noyalib's `to_string_value` (upstream), reached from yqr's
`render` — the typed emitter used for **computed** and absent values
**Related:** `yqr-f017` (the feature whose output made it routine),
`yqr-b015` (the same defect class on the write path, and the same upstream
function family), `yqr-a001` (why yqr cares about a space nobody reads)

## 1. Summary

When a block collection is the value of a mapping entry **reached through a
sequence item**, the emitter writes the `key:` with a trailing space before the
newline:

```console
$ printf 'x:\n  - key: a\n    value:\n      d: 1\n' | yqr --normalize '.' | sed -n l
x:$
  - key: a$
    value: $
      d: 1$
```

The space belongs before an inline scalar (`key: 1`). Here the value is on the
following line, so nothing follows the colon and the space is left over.

The same mapping *not* reached through a sequence is correct:

```console
$ printf 'x:\n  k:\n    d: 1\n' | yqr --normalize '.' | sed -n l
x:$
  k:$
    d: 1$
```

## 2. It is upstream, and it is not about mappings

`render` (`src/lib.rs`) calls `noyalib::to_string_value` and does nothing to
the result but trim the final newline. Calling the emitter directly reproduces
it, and the battery narrows the condition to the sequence item rather than to
the value's kind:

| input | emitted | trailing space |
|---|---|---|
| `x:` / `␣␣- key: a` / `␣␣␣␣value:` / `␣␣␣␣␣␣d: 1` | `value: ` | yes |
| `x:` / `␣␣- key: a` / `␣␣␣␣value:` / `␣␣␣␣␣␣- 1` | `value: ` | yes |
| `- a:` / `␣␣␣␣b: 1` | `- a: ` | yes |
| `x:` / `␣␣- - 1` | `- ` | yes |
| `x:` / `␣␣k:` / `␣␣␣␣d: 1` | `k:` | no |

So it is any block collection written on the line below its indicator once a
sequence is in the path, mapping value and nested sequence alike.

## 3. Why it is worth tracking

- Nothing misreads. Both PyYAML and Psych load the output to the same value,
  and yqr reads it back unchanged.
- But it is trailing whitespace, which `git diff --check`, `yamllint`'s
  `trailing-spaces` and most pre-commit hooks reject. `yqr --normalize` is
  offered as the canonicalizing pipeline; output a repo's own lint refuses is a
  poor canonical form.
- `yqr-f017` makes it routine rather than obscure: **every** `to_entries` pair
  whose value is a mapping or a sequence has one, which is most of them, and it
  lands on the line a reader is looking at.

## 4. Why yqr does not work around it

The obvious local fix — strip trailing whitespace from each rendered line — is
**wrong**, and measurably so. A block scalar's content may legitimately end a
line with spaces, and stripping changes the value:

```text
Value::String("a  \nb\n")  emits  "|\n  a  \n  b\n"     loads back as "a  \nb\n"
after a blanket strip      "|\n  a\n  b\n"              loads back as "a\nb\n"
```

Silently altering a string is a strictly worse defect than a cosmetic space, so
the workaround is refused and the bug is carried visibly instead. A narrower
workaround — strip only lines that end in `:` — would be arguing with the
emitter about YAML from outside it, which is the shape of thing that belongs
upstream.

`yqr-f017` pins the current output in `tests/cli.rs`
(`to_entries_output_carries_the_emitters_trailing_space`) rather than hiding
it, on the `yqr-m003` rule that a pin states what the bug does.

## 5. Route — taken 2026-08-20

Upstream, on the `yqr-b004` §5 `PR-with-fix` precedent: filed as
**noyalib#297** and fixed in **noyalib#298**, independent of `b015`
(noyalib#294 / #296) — both touch trailing whitespace but in different files,
and either can land first.

## 6. What the filing found that this spec had not

### 6.1 A second, larger source

The indicator is not the only place noyalib emits trailing whitespace. A block
scalar's content loop writes the block's indent before **every** line including
empty ones, leaving the indent standing on a line that holds nothing:

```text
"k: |\n  a\n\n  b\n"   emits   "k: |\n  a\n␣␣\n  b\n"
```

Across noyalib's own fixture corpus this accounts for **36 documents to the
indicator's 9** — so a report naming only the indicator would have described
the smaller half and implied the rest was clean.

Found by holding the fix to a property (*no emitted line carries trailing
whitespace*) rather than to the reproduction. The reproduction was satisfied
after the first fix; the property was not, and the gap was the second source.
That is the transferable part: a fix measured against the case that prompted it
stops exactly where that case stops.

### 6.2 The same cause, three times over

Both faces are duplicated logic that drifted from a sibling already applying
the rule. `write_mapping` has always written `key:` without a space before a
block value; `write_sequence` carries its own copy of the key-writing — twice —
and neither consulted `needs_block_layout`. The block-scalar content loop
exists in **three** identical copies (`|` auto, `|` explicit, `>`), so the rule
was missing from all three and fixing any one would have left the others
writing it.

The patch therefore extracts a rule per site rather than patching the callers,
which is what makes the fix a fix rather than a round of whack-a-mole:
`indicator_takes_a_space` and `write_block_scalar_body`, one each, used by
every caller.

### 6.3 What the differential run measured

384 documents, ~12.5k emitted lines: dangling indicators 9 docs → 0, leftover
trailing whitespace 45 docs → 0. Two rows matter as much: emissions that fail
to re-parse (5 docs) and that re-parse to a different value (4 docs) are the
**same sets** before and after — pre-existing, unrelated, and untouched. They
are noted to the maintainer as unexamined rather than left implied.

§4's refusal to work around it held up under measurement and is pinned as two
control tests upstream, including an all-spaces content line — which is
distinguishable from an emitted indent only by what the string says, and is
precisely why the rule keys on `line.is_empty()` rather than on how the
emitted line looks.
