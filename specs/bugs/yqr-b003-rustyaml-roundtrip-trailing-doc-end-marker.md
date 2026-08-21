# Bug b003 — rust-yaml fork `RoundTripDocument::parse_all` errors on a trailing `...` after a block collection


> **Historical: resolved.** yqr no longer behaves as described below. The
> **Status** line records what fixed it and when; the rest is kept as the
> reproduction and the reasoning, written in the present tense of the time it
> was filed.

**Status:** Resolved — moot (2026-07-10). The rust-yaml fork `RoundTripDocument` backend was removed when yqr consolidated on noyalib (`yqr-m005`), so this fork bug no longer affects yqr. (The upstream fork issue itself is unchanged.)
**Severity:** Medium — breaks the a001 byte-for-byte identity of `--engine rust-yaml` for any stream ending in a `...` document-end marker after a top-level block collection; the default pipeline and other engine inputs are unaffected
**Owner:** yqr maintainers
**Last updated:** 2026-07-04
**Affects:** the `--engine rust-yaml` fidelity read path (`yqr-f003`); irrelevant to the default pipeline and to `--engine noyalib`
**Component:** the rust-yaml fork's `roundtrip::document_boundaries` / `RoundTripDocument::parse_all` ([zoosky/rust-yaml](https://github.com/zoosky/rust-yaml) branch `feat/roundtrip-document`; the rust-yaml#73 substrate)
**Related:** `yqr-f003` (backend A), `yqr-b001` §8 (#73), `yqr-m002` §6.1 (document-boundary policy)

## 1. Summary

Adversarially reviewing backend A surfaced a document-segmentation bug in the
fork's `RoundTripDocument::parse_all`: a stream that **ends with a `...`
document-end marker after a top-level block collection** is rejected with a
leaked internal error instead of parsing. Because it fails in `parse_all`, the
whole stream is rejected before evaluation, so **every** filter (identity and
projections) aborts — the a001 north-star (`yqr --engine rust-yaml '.' f` ==
`cat f`) is impossible for such a file.

```text
$ printf 'a: 1\n...\n' | yqr --engine rust-yaml '.'
yqr: io error: failed to parse YAML input: Error at line 1, column 1: \
internal document accounting mismatch: 2 byte ranges, 1 span trees, 1 values
$ printf 'a: 1\n...\n' | yqr '.'      # default pipeline: fine
a: 1
```

The input is textbook-valid YAML (one block mapping terminated by an explicit
`...` marker), which the default pipeline handles.

## 2. Root cause

`parse_all` cross-checks three per-document counts and refuses on any mismatch:

```rust
if boundaries.len() != trees.len() || trees.len() != values.len() {
    return Err(Error::parse(/* "internal document accounting mismatch: N byte
                                ranges, M span trees, K values" */));
}
```

For `a: 1\n...\n` the composer yields **1** span tree and **1** value, but
`document_boundaries` returns **2** byte ranges: `[0, input_len]`. The trailing
`...` after block content leaves a phantom boundary at EOF (an empty
`[input_len, input_len]` range) with no matching tree or value, tripping the
guard.

Confirmed specificity (from the review):

- **Fails** (exit 5): `a: 1\n...\n` (block map), `- a\n- b\n...\n` (block seq),
  `a: {b: 1}\n...\n` (block map with nested value).
- **Works** (exit 0): `42\n...\n`, `"hi"\n...\n` (scalars); `{}\n...\n`,
  `{a: 1}\n...\n` (flow collections); `a: 1\n...\n---\nb: 2\n` (content follows
  the marker); `a: 1\n` (no marker).

The block-vs-flow/scalar asymmetry points at how the block-collection close
tokens interact with the boundary counter around the `...` `DocumentEnd`.

## 3. Upstream ask (fork)

In `roundtrip::document_boundaries` (or the reconciliation in `parse_all`): do
not emit a boundary for an empty trailing region after a `...` marker — an
explicit `---` starts a new (possibly null) document, but a trailing `...` only
closes the current one. The boundary count must agree with the composer's
document count for a stream ending in `...` after a block collection. Add
`a: 1\n...\n` and `- a\n- b\n...\n` to `parse_all`'s round-trip cases.

Per `yqr-m002` §6.1, the last document extends to EOF including trailing trivia,
so `a: 1\n...\n` is a single document whose slice is the whole input.

## 4. yqr-side handling (shipped in f003)

- **Documented limitation.** `yqr-f003` and the README `--engine` notes record
  that backend A currently rejects a trailing `...` after a block collection.
- **Pinned as a test.** `tests/fidelity_engine_rustyaml.rs`
  (`document_end_marker_after_block_is_a_known_limitation`) asserts the current
  error, so a future fork bump that fixes this flips the test and prompts
  turning it into a byte-for-byte identity assertion.
- No adapter work-around is attempted: `parse_all` fails wholesale, so there is
  no faithful span to recover; consistent with how `--engine noyalib` errors on
  inputs its parser rejects (`yqr-f002` §4a).

## 5. Acceptance criteria

Resolved when the fork's `parse_all` accepts a trailing `...` after a block
collection (byte-for-byte), the yqr git dependency is bumped to include the fix,
and the pinning test is converted to assert `yqr --engine rust-yaml '.' f ==
cat f` for `a: 1\n...\n` and `- a\n- b\n...\n`.
