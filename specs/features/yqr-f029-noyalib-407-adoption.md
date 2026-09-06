# Feature f029 — Adopt the noyalib release that carries #407: trust the stream location, drop the re-parse

**Status:** Draft — waits for the crates.io release carrying noyalib PR #408 (open 2026-09-06); the adoption diff is known, §2
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-b028` (the bug, and the upstream issue and PR), `yqr-f028`
(the adoption that found it), `yqr-f012` (validate), `yqr-f026` (the
precedent: file upstream, verify on the branch, adopt on release)

## 1. Why this is not a no-op

The fix for noyalib#407 makes `cst::parse_stream*` return locations that
count from the start of the stream, as the typed loaders already do.
yqr's workaround (`b028` §2, `failing_document` in
`src/validate/mod.rs`) finds the failing document by re-parsing the
documents in order and requires the re-parse to fail with the identical
error, location included, before it trusts the offset.

On the fixed crate that guard inverts: the re-parse of the document
alone still reports the slice-relative location (line 2), while the
stream error now reports the stream one (line 5). The strings differ,
the guard refuses the offset, and every located stream finding renders
without a position. Measured 2026-09-06 with a temporary
`[patch.crates-io]` on the fix branch: four unit tests fail
(`located_errors_in_a_stream_count_from_the_failing_document`,
`key_collision_is_a_y102_by_default_with_document_note`,
`syntax_error_is_a_located_y001`, `renders_located_diagnostic_rustc_style`,
each with `position: None`), and the binary prints `--> <stdin>` with no
line for the unknown-anchor and collision streams. Bumping the pin
without this spec would ship that.

## 2. The adoption diff

All in `src/validate/mod.rs`:

- `failing_document` stops re-parsing. The base is 0 for every error;
  the only thing left to compute is the document note, from
  `document_starts` and the error's own byte index (the last start at or
  before it). The `None` arm, which stood for "the parser split the
  stream differently", has no cause left and goes.
- `syntax_diagnostic`'s `locate` becomes `render::position_of(source,
  index)` again, and the similar-anchor hint's "declared in the same
  document" fallback is unreachable and goes with it.
- `document_starts` stays, for the note only. Its test stays.
- The four tests in §1 keep their expectations unchanged; they are the
  proof the adoption is right. The test comment on `b: [1,` that names
  the parser's document-relative index becomes history and says so.

Nothing outside `validate` reads a stream error's location.

## 3. Acceptance criteria

- [ ] The release is published; the pin moves; `Cargo.lock` moves
      noyalib and only what it newly requires.
- [ ] `failing_document` no longer parses; §2 applied.
- [ ] The four §1 tests green on the published crate, unchanged.
- [ ] `b028` §4 records the release; `b000` unchanged (already Resolved).
- [ ] The pin comment and `CHANGELOG.md` say what the bump bought.
- [ ] Full suite green; `local-ci.sh` clean.
