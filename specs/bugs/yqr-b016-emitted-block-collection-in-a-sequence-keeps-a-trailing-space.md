# Bug b016 — An emitted block collection reached through a sequence item keeps a trailing space

**Status:** Open — found 2026-08-20 by `yqr-f017`'s first output; pre-existing
and reachable through `--normalize` since long before it
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

## 5. Route

Upstream, on the `yqr-b004` §5 `PR-with-fix` precedent, and naturally batched
with `yqr-b015`: both are trailing whitespace written by noyalib where a line
should have ended, one on the write path and one on the emit path, and the
same maintainer is holding both.

Not yet filed — `b015` (noyalib#294 / #296) is open at the time of writing, and
two whitespace reports at once from the same reporter is worse than one
followed by the other.
