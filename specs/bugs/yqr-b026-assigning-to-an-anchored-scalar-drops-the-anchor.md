# Bug b026 — Assigning to an anchored scalar drops the anchor, then fails on the alias it orphaned

**Status:** Open — filed 2026-09-02, found while verifying `yqr-b025`
**Severity:** Medium — an ordinary write at a resolvable path is refused,
and the message blames a defect the write itself created
**Component:** write tier (`src/fidelity/write/`), scalar assignment, and
the byte range the engine resolves for an anchored scalar
**Related:** `yqr-b025` (found while verifying it), `yqr-b020` (a write at
an anchor definition is the sanctioned remedy), `yqr-f006`, `yqr-f026`
(noyalib 0.0.30 moves anchored-node locations; re-check there)

## 1. Summary

```console
$ printf 'a: &x 1\nb: *x\n' | yqr '.a = 2'
yqr: runtime error: cannot assign at "a": unknown anchor: x at line 2, column 4
```

The expected output is `a: &x 2\nb: *x\n`. The anchor is a property of
the node, not part of its value, and the value is what the filter assigns.
Instead the rewritten range covers `&x 1`, so the edit removes the anchor
definition, the re-parse guard finds `*x` dangling, and the write is
refused with a message about an anchor the user never touched.

Reproduces on the shipped 0.0.28 pin and on noyalib 0.0.30.

An anchored *mapping* is not affected on 0.0.28: `base.k` under
`base: &m` writes normally (`b020`'s remedy), because the property sits on
the mapping's own line and the entry's range starts below it. On 0.0.29
and later that write is refused for a different reason, `yqr-f026` §3.

## 2. What to settle first

yqr rewrites the byte range the engine resolves for the path. For an
anchored scalar that range starts at the `&x` property rather than at the
scalar's first byte. Whether the range is wrong (upstream) or yqr should
skip a leading property before rewriting (here) is the first question.
noyalib 0.0.30 changed where tagged and anchored nodes are *located*
(commit 3e85e15, "tagged/anchored node locations anchor at the
properties", marked breaking), so the span model is in motion upstream;
check what 0.0.30 resolves for `a` before choosing a side.

The tagged case (`a: !!str 1`) is untested and is the same shape; settle
it in the same change, as a working write or as a refusal that names the
tag.

## 3. Acceptance

- [ ] `printf 'a: &x 1\nb: *x\n' | yqr '.a = 2'` prints `a: &x 2\nb: *x\n`.
- [ ] The tagged case decided and pinned.
- [ ] A test in `tests/cli.rs` for each.
