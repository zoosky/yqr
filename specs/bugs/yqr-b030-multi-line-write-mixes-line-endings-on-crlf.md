# Bug b030 — A multi-line write gives a CRLF document mixed line endings

**Status:** Resolved — guarded 2026-09-08 in `yqr-f032`. yqr refuses a
write that would introduce a bare line feed into a wholly CRLF document.
**Filed upstream 2026-09-09 as noyalib#421, with a fix in PR #422**;
adopting a release that carries it lifts the refusal
**Severity:** Medium — a fidelity violation at exit 0, on bytes the edit
did not name, but confined to CRLF documents and to multi-line
replacements
**Component:** write tier (`src/fidelity/write.rs`), `set_value`
**Related:** `yqr-b009` (the same defect in the *insertion* mutators,
fixed upstream by yqr's noyalib#261 and released in 0.0.22), `yqr-f032`
(the feature whose measurement found it), `yqr-f015` (which deleted the
local CRLF workaround once upstream fixed the insertion half),
`yqr-a001` §1

## 1. Summary

Reproduced on the shipped **0.8.0**:

```console
$ printf 'a: 1\r\nb: 2\r\n' | yqr '.a = "one\ntwo"' | cat -v
a: |-
  one
  two^M
b: 2^M
```

The three lines the replacement wrote end with LF; the rest of the file
ends with CRLF. Exit 0, and the bytes that changed were not only the ones
the filter named.

`yqr-b009` is this defect in the insertion mutators, filed and fixed
upstream in 2026-08-14 and adopted in `yqr-f015`, which deleted yqr's
local workaround. `set_value` was never in that patch's scope, and no
test covered a multi-line `set_value` on a CRLF document, so this half has
been live the whole time.

## 2. Extent, measured

On noyalib 0.0.41, driving each mutator directly:

| write | CRLF result |
|---|---|
| `set_value`, multi-line string | **mixed** — the replacement's own lines end with LF |
| `set_value`, collection replacement that **grows** | **mixed**, same cause |
| `set_value`, collection replacement of the **same** line count | clean, because no new line boundary is written |
| `set_value`, scalar | clean |
| `insert_entry_value`, scalar or collection | clean (`yqr-b009`'s fix) |
| `push_back_value`, scalar or collection | clean |
| flow collection replacement | clean, it is one line |

So it is one rule: an interior line boundary that the *replacement* writes
uses LF, while boundaries carried over from the original span keep the
document's.

The collection arm of `set_value` is new territory for yqr —
`yqr-f032` is what enables it — but the multi-line **string** case is
reachable today.

## 3. Why the existing guards missed it

The re-parse guard and the load-back oracle both ask whether the result
still means the same thing. Mixed line endings mean exactly the same
thing, which is the point: this is a fidelity defect, not a correctness
one, and fidelity is what yqr exists to hold rather than what its engine
checks.

## 4. The fix

`check_integrity` counts line feeds with no carriage return before them on
either side of every `set_value`, and refuses a write that adds one to a
document that had none. The rule is deliberately narrow:

- a document already mixing the two is not made worse by this rule;
- an all-LF document gains a bare line feed per added line, legitimately;
- only `set_value` is guarded, because the insertion paths were measured
  clean and carry upstream's fix.

The cost is that a collection replacement which *grows* the line count is
refused on a CRLF file even though it is a perfectly reasonable edit. That
is the same defect, not a second one, and it lifts when the engine's fix
does. The refusal names no remedy because there is none short of converting
the file's line endings, and `yqr-f025` prefers no remedy to one that
fails.

Refusing rather than repairing follows `yqr-b022`'s precedent: a
workaround that rewrites bytes to hide an engine defect trades a visible
failure for an invisible one. `yqr-f014` did carry a local restore for
`b009` while its fix was unreleased, and `yqr-f015` deleted it; if that is
wanted again here it should be a decision of its own, not a side effect.

## 5. Upstream — filed as noyalib#421, fixed in PR #422

noyalib#261 taught the insertion mutators to derive an inserted line's
terminator from the document. `set_value`'s multi-line replacement path
and the collection arm (#328) do not. The fix is the same one, applied to
the finished fragment at the splice: a `respell_breaks` helper beside
`document_break`, called from both arms. Threading a terminator through
the formatter was measured and rejected, because it misses the comment
hoist from #333, does not reach the collection arm at all, and would make
the shared `format_block_literal` an emitter that has to be told to stay
LF. In the collection arm the re-spelling lands above the pre-splice
oracle, so the bytes it parses are the bytes spliced.

Reproduction for the filing:

```rust
let mut d = noyalib::cst::parse_document("a: 1\r\nb: 2\r\n").unwrap();
d.set_value("a", &noyalib::Value::String("one\ntwo".into())).unwrap();
assert!(!d.source().contains("two\r\n") || d.source().matches('\n').count()
        == d.source().matches("\r\n").count());
```

## 6. Tests

`a_multi_line_write_into_a_crlf_document_is_refused` pins the refusal;
`the_crlf_guard_leaves_the_paths_that_are_correct_alone` holds the
insertion path and the single-line write, which is what keeps the guard
from becoming a ban on editing CRLF files; `an_all_lf_document_may_still_grow_lines`
pins the rule's other side. One corpus write case runs it on the CRLF
document the corpus already carries.
