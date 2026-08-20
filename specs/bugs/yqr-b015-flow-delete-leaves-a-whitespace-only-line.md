# Bug b015 — Deleting a member of a wrapped flow collection leaves a whitespace-only line

**Status:** Open — found 2026-08-20 while verifying `yqr-b011` against the
noyalib 0.0.25 release; not yet filed upstream (§4)
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

## 4. Route

Upstream, on the `yqr-b004` §5 `PR-with-fix` precedent, once the maintainer has
had the four fixes of 0.0.25 land. The fix is the same shape as the one yqr's
`owned_line_span` makes on the block path: a removed member owns its line's
indentation when nothing else on that line survives, so the range should run
from the line start rather than from the member's first byte. The
single-line case must stay as it is, which is what makes the condition "the
member is alone on its line" rather than "the collection is wrapped".

**Not yet filed.** Four yqr-authored fixes landed upstream on 2026-08-19; this
is deliberately held until that release has settled rather than opened on top
of it.

## 5. Reproduction

Pinned in `tests/cli.rs` (`a_wrapped_flow_collection_edits_only_at_the_site`
covers the `set` half, which is correct). The delete half is **not** pinned
yet — pinning a whitespace-only line as expected output invites a future reader
to preserve it. It goes in as a regression test with the fix, per `yqr-m003`'s
rule that a pin states what the bug does, and this one's shape makes the pin
worse than the prose.
