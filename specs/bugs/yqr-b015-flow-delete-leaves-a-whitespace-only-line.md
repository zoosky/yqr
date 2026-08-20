# Bug b015 — Deleting a member of a wrapped flow collection leaves a whitespace-only line


> **Historical: resolved.** yqr no longer behaves as described below. The
> **Status** line records what fixed it and when; the rest is kept as the
> reproduction and the reasoning, written in the present tense of the time it
> was filed.

**Status:** Resolved — found 2026-08-20 while verifying `yqr-b011` against the
noyalib 0.0.25 release, filed the same day as noyalib#294 with a fix in
noyalib#296, merged as `ab4c235` and **released in noyalib 0.0.26** the same
evening. Verified against the published crate by `yqr-f020` §3, controls
included, with the outputs loaded back under PyYAML and Psych. yqr pins 0.0.26,
and the regression test §5 deferred until a fix existed is now in `tests/cli.rs`
**Severity:** Low — the result is valid YAML and loads back correctly; what is
wrong is a blank, trailing-whitespace line left at the edit site
**Component:** noyalib's `Document::remove` (upstream), reached from yqr's
`del` — the flow class is delegated by `yqr-f016` §5
**Related:** `yqr-b011` (the parse refusal that hid this), `yqr-f019` §3.5
(where it was found), `yqr-b006` (the same defect class on the block path,
which yqr fixed by keeping its own implementation), `yqr-a001`

## 1. Summary

Deleting a member of a flow collection that is spread over several lines
removes the member's bytes and its separator, but leaves the line's
indentation behind:

```console
$ printf 'ports: [\n  80,\n  443,\n]\n' | yqr 'del(.ports[0])' | sed -n l
ports: [$
  $
  443,$
]$
```

The `  ` on line 2 is all that is left of `  80,`. The same happens when the
removed member is the last one, and on the mapping form:

```text
del(.ports[1])   ports: [ / ␣␣80, / ␣␣ / ]
del(.cfg.a)      cfg: { / ␣␣ / ␣␣b: 2, / }
```

A single-line flow collection is unaffected — `ports: [80, 443]` becomes
`ports: [443]`, which is correct and has been since `yqr-f016`.

## 2. It is upstream's

yqr delegates the whole flow class to `Document::remove` (`yqr-f016` §5, and
`src/fidelity/write/delete.rs` says so at the call site). Calling it directly
on noyalib 0.0.25 produces the same bytes:

```text
remove("ports[0]") on "ports: [\n  80,\n  443,\n]\n"
  -> "ports: [\n  \n  443,\n]\n"
```

So this is not yqr's splice, and the fix belongs upstream — the same shape of
finding as `yqr-b006`, which was the block path's version of exactly this
question (what trivia belongs to a removed entry) and which yqr answered in its
own code because it owns that path.

## 3. Why it is worth tracking rather than shrugging at

- It loads back correctly under every implementation, so nothing is broken and
  nothing is silently wrong. That is the whole reason the severity is Low.
- But it introduces **trailing whitespace on a line that had none**, which
  `git diff --check`, `editorconfig-checker`, `yamllint` (`trailing-spaces`)
  and most pre-commit setups flag. A tool whose promise is "the diff is one
  line" producing a diff that fails the repo's own lint is the promise's
  failure mode, not a cosmetic detail.
- It is at the edit site, which `yqr-a001` singles out as the one place an
  edit's spelling has to be right.

## 4. Route — taken 2026-08-20

Upstream, on the `yqr-b004` §5 `PR-with-fix` precedent: filed as
**noyalib#294** and fixed in **noyalib#296**. The estimate held — the fix is
the same shape yqr's `owned_line_span` makes on the block path, and the
condition is "the member is alone on its line" rather than "the collection is
wrapped", so the single-line case is untouched.

### 4.1 The argument

The one that carried it is upstream's own inconsistency, the shape that carried
`b010` and `b014`: noyalib's **block** path already answers the identical
question the other way for the same bytes. `owned_entry_range` takes a removed
entry's whole line, indentation included, so `ports:` / `- 80` / `- 443` minus
`ports[0]` leaves no residue. Same operation, same question, opposite answers —
and only because one path is written in terms of lines and the other in terms
of separators. "PyYAML and yamllint dislike the output" is true and is the
weaker half.

### 4.2 What the filing declined to decide

Two shapes the patch deliberately leaves alone, both stated up front rather
than left for the maintainer to find:

- **The last member keeps the comma on the line above.** `[80,]` is valid — the
  entries after a `,` are optional in the flow productions — and both PyYAML
  and Psych read it back as `[80]`. Reaching up a line to delete a separator
  the removal does not own is reformatting, not removing.
- **A comment on the member's line keeps the line.** What a comment orphaned by
  a removal *means* is a semantics question of the `b010` kind, and a
  whitespace rule is the wrong instrument to settle it. Raised as a separate
  question rather than decided in passing.

### 4.3 Verification offered with the patch

A 24-shape before/after battery (sequences and mappings, first/middle/last
member, CRLF, tab indent, root-level, nested, no final newline, pre-existing
blank lines, and each control): **14 outputs fixed, 10 byte-identical**, the 10
being exactly the ones that should be. Every fixed output checked against
PyYAML and Psych. The `fuzz_editors` invariants run over a generated corpus of
336 accepted removals — re-parses, exactly one leaf fewer, no line gains
trailing whitespace, CRLF stays CRLF — all holding, and on unpatched `main`
only the whitespace invariant failing, which is what shows the change moves
nothing else. libFuzzer itself was **not** run (`cargo-fuzz` is not installed
locally and was not installed for this), and the PR says so.

## 5. Reproduction — pinned 2026-08-20, once the fix existed

At filing time the delete half was deliberately **not** pinned: pinning a
whitespace-only line as expected output invites a future reader to preserve it,
and this bug's shape made the pin worse than the prose. With noyalib 0.0.26
released, the pin states the right thing, so `tests/cli.rs` gains both halves
(`yqr-f020` §4):

- `deleting_from_a_wrapped_flow_collection_takes_the_whole_line` — first
  member, last member, and the flow-mapping form; each asserts the exact bytes
  *and* that no line carries trailing whitespace.
- `a_flow_delete_leaves_a_line_it_does_not_own_standing` — the four controls:
  single-line, opening indicator, closing indicator, comment.

The controls are the more valuable half. The first test would pass on a fix
that stripped whitespace indiscriminately; only the controls distinguish the
rule that was implemented from the one that would have been easier to write.
