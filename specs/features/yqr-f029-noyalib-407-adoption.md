# Feature f029 — Adopt the noyalib release that carries #407: trust the stream location, drop the re-parse

**Status:** Done — adopted 2026-09-07 on noyalib 0.0.39. The fix shipped in
0.0.36 (upstream #410 carried the cherry-pick of PR #408, authorship
intact); §4 records the adoption and what the four crossed releases
changed beyond it
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

- [x] The release is published; the pin moves; `Cargo.lock` moves
      noyalib and only what it newly requires.
- [x] `failing_document` no longer parses; §2 applied.
- [x] The four §1 tests green on the published crate, unchanged.
- [x] `b028` §4 records the release; `b000` unchanged (already Resolved).
- [x] The pin comment and `CHANGELOG.md` say what the bump bought.
- [x] Full suite green; `local-ci.sh` clean.

## 4. Adoption record (2026-09-07, noyalib 0.0.39)

**§1 held exactly.** The bare bump failed the four tests named there
and the two CLI tests behind them, all with `position: None`, and the
standalone reproduction from `b028` §4 reports index 21, line 5 from
every entry point on 0.0.39.

**§2 applied, with one addition.** `failing_document` is gone;
`document_note` and `document_index` derive the collision's note from
`document_starts` and the error's own byte index. The addition: noyalib
0.0.38 suggests the alias's *own* name when the anchor lives in an
earlier document of the stream (its Display says "anchors do not cross
`---`"), and on the unadopted code that case rendered no position and
the fallback "declared in the same document", which is false. The
similar-anchor arm now tells the two apart the way upstream's own
Display does, by the suggestion carrying the alias's own name, and says
"`&x` appears at line 1, in an earlier document; anchors do not cross
`---`". "Appears", not "is declared": the parser finds that earlier
`&x` by a text search over the earlier bytes, so a `# see &x` comment
satisfies it (measured; the test pins it). Tests added for `---`, `...`
and the comment; the four §1 tests unchanged.

**Review round.** A code review of the adoption confirmed six findings
against the built binary, all fixed here:

- `document_starts` mirrored only the `---` rule, while the CST parser
  splits by tokens: a `---` opens a document only after content
  (directives, comments and blank lines ahead of it are its prologue),
  and a `...` closes at the end of its line. So `%YAML 1.2\n---\n1: a\n"1":
  b\n` was noted as "document 2" on a one-document file, and a collision
  after a `...` was placed in the wrong document, where the deleted
  re-parse guard had stayed silent. The split now mirrors both rules,
  and a test holds it to the byte lengths of the documents the parser
  returns across seventeen streams (prologues, `...` before content, a
  `...` closing nothing, a trailing comment after `...`, CRLF, a BOM).
- The cross-document hint keyed on that split, so a `...` boundary lost
  it; it keys on the same-name suggestion alone now, as above.
- A merge-conflict file is anchored at its first marker again. Since
  noyalib 0.0.36 every scanner error carries a position, so the parser's
  own caret landed on the `>>>>>>>` line while the help named line 2;
  the marker wins whenever there is one, and the tests pin `2:1`
  instead of "some position".
- The two accept-to-reject changes (`<tab>- a` on the first line,
  `!!!int`) are pinned, so a later release that takes either back fails a
  test; the pin comment counted one. They live in the CLI suite, not the
  corpus: `yqr-m003` makes every corpus document one the validator must
  accept, and a first attempt to add them there failed exactly that
  guard.
- Every located finding in a stream now carries the "in document N"
  note, not only a collision; it is a pure function of the split.
- The stream test is renamed to what it asserts
  (`located_errors_in_a_stream_count_from_the_stream`).

What the review found and this does not do: the parser's earlier-anchor
search is textual where the typed loaders keep real definitions; that
is upstream's to align, and the hint's wording no longer depends on it.

**The crossed releases.** Measured against a `main` build on 0.0.34,
same inputs through both binaries:

| upstream | release | what | yqr effect |
|---|---|---|---|
| #407 / #410 | 0.0.36 | stream error locations count from the stream | this spec |
| — | 0.0.36 | CST scanner errors carry their position | more `Y001` findings render a line; none in the suite changed |
| — | 0.0.36 | a tab before a top-level flow node is one rule at every line start | `<tab>- a` on the first line of a file is now refused ("tab characters are not allowed as indentation", read exit 5, `validate` `Y001` at 1:1); 0.0.34 read it. Upstream's own parse-behaviour change, passed through |
| — | 0.0.36 | a keep-chomped block scalar of blank lines followed by `---` | accepted; 0.0.34 refused it with an unlocated `Y001` |
| — | 0.0.36 | a `...` that closes nothing is a prologue, not a document | no change in the default read, `validate`, or `--normalize` on `...\na: 1\n` |
| — | 0.0.38 | `!!!int` refused with "did you mean `!!int`?" | `validate` reports `Y001` at the tag; 0.0.34 accepted the file |
| — | 0.0.38 | cross-document alias diagnosed | the hint above |
| — | 0.0.38 | CST formatter keeps explicit keys parseable | `noyafmt`'s formatter, not `to_string`: `--normalize` output for `? a` / `: b`, `? [a, b]`, and a lone property is byte-identical to 0.0.34 |
| — | 0.0.39 | self-referential anchor named as such | the message passes through with its position (`alias \`*x\` points at \`&x\`, still being defined at line 1, column 4; ...`); 0.0.34 called it an unknown anchor |
| — | 0.0.39 | `!!int` on a YAML 1.1 spelling names it | the read error says "YAML 1.2 has no binary literal, `0b` was YAML 1.1; write 42" |
| — | 0.0.35 | budgets as pure predicates, Kani-proved | none; the ratio heuristic stays disabled and the values corpus is green |
| — | 0.0.37 | lockstep re-release, no core change | none |

The write tier: nine writes on one document (append, item assign, item
delete, whole delete, flow delete, swap, scalar assign, an anchored
delete refusal, a line comment) byte-identical to 0.0.34.

**Benchmarks.** `cargo bench --bench eval`, 0.0.34 on the `main`
worktree against 0.0.39 on this branch, back to back:

| benchmark | 0.0.34 | 0.0.39 | change |
|---|---|---|---|
| `parse/nested_path` (filter parse, no YAML) | 501.0 ns | 501.3 ns | none |
| `eval_str/field_access` | 3.680 µs | 3.642 µs | within noise |
| `eval_str/iterate_100` | 142.6 µs | 141.6 µs | within noise |

Nothing to justify; the five releases touched diagnostics, the
scanner's edge cases and the budgets' proofs, not the hot path.
