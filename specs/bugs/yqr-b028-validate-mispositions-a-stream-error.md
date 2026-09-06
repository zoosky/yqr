# Bug b028 — `validate` positions a stream error relative to the document that failed

**Status:** Resolved — fixed 2026-09-06 by the noyalib 0.0.34 adoption
(`yqr-f028`), which found it
**Severity:** Medium — every located parse error in a document after the
first of a multi-document stream pointed at the wrong line, with the caret
on unrelated text, and three tests pinned one such line as correct
**Component:** `src/validate/mod.rs` (`syntax_diagnostic`)
**Related:** `yqr-f012` (the validate command), `yqr-f028` (the adoption
that found it), `yqr-b027` §1 (the collision note this replaces),
`yqr-b025` (the values file the corpus validates)

## 1. Summary

noyalib's `cst::parse_stream_with_config` splits its input at column-0
`---` markers and parses each document on its own. An error's `Location`
therefore counts from the start of the document that failed — its marker
line included — and the error does not say which document that was.
`syntax_diagnostic` passed that index to `render::position_of(source, ..)`
as if it counted from the start of the stream.

Measured on the shipped 0.0.31 build and on 0.0.34, identical:

| input | reported | actual |
|---|---|---|
| `a: 1\n---\nb: 2\n---\nc: *nope\n` | `2:3`, caret on the `---` | `5:4`, the `*nope` |
| `a: 1\n---\nb: [1,\n` | `3:3`, caret on the `[` | `3:7`, end of input — where a single document reports the same error |

The second one looked right, which is how it survived: the document-
relative index 11 lands on byte 11 of the stream, the `[`, by coincidence.
Two unit tests and one CLI test asserted it, one with the comment "the
location is absolute in the file, not relative to a document". Only the
first document of a stream was ever positioned correctly, because its
chunk starts at byte 0. The similar-anchor hint ("a similar anchor `&x` is
declared at line N") carried the same offset.

Found while adopting noyalib 0.0.34 (`yqr-f028`): 0.0.33 locates a
stringified-key collision (`Error::KeyCollisionAt`, noyalib#378), so the
`Y102` finding gained a position, and the first stream test reported line
3 for a collision on line 6.

## 2. Fix

One helper, `failing_document`, finds the document an error belongs to and
the byte offset it starts at. It splits the stream by the parser's own
marker rule (`document_starts`, mirrored from noyalib's `doc_boundary`: a
`---` that opens a line and is followed by whitespace or the end of input;
a leading marker starts the first document rather than a second one), then
re-parses the documents in order, the way the stream parser does, until
one fails. That failure has to render to the same string as the original,
location included. Otherwise the parser split the stream differently than
yqr did and no offset is trusted: the finding renders without a position
rather than with a wrong one.

Every location `syntax_diagnostic` renders goes through it — the finding's
own, the similar-anchor hint (which falls back to "declared in the same
document" when there is no trusted offset), and the collision's. A single
document has base 0 and is unaffected. The collision's document note ("in
document 3 (starting at line 4)") now comes from the same lookup; the
previous `collision_document_note`, which re-split the stream and
re-parsed every chunk matching on the key alone, is gone. It was the third
row of `b027` §1.

Cost: on the error path only, one more parse of the documents up to and
including the failing one. The success path is untouched.

## 3. Why nothing caught it

Three tests covered a located error in a stream, and all three asserted
the number the code produced. The comparison that would have caught it —
the same error on a single document reports column 7 — was never made
across the two shapes. The new tests assert the stream position against
what the single-document case reports, and pin the marker rule on its own
(leading marker, lone-CR break, `----`, a `---` mid-line).

## 4. Upstream

A stream parser that reports document-relative locations without naming
the document leaves the caller to redo the split. **Filed 2026-09-06 as
noyalib#407**, asking for either stream-relative locations from the
`parse_stream*` entry points or a document index on the located variants;
§2's re-parse is the workaround until a release carries one of them.

Measuring for the report, with the crate directly rather than through
yqr, sharpened the argument: only the CST entry points are affected.
The typed loaders already report stream positions for the same bytes.

| entry point, on `a: 1\n---\nb: 2\n---\nc: *nope\n` | index | line | column |
|---|---|---|---|
| `document::load_all` | 21 | 5 | 4 |
| `load_all_as::<Value>` | 21 | 5 | 4 |
| `cst::parse_stream` | 7 | 2 | 4 |
| `cst::parse_stream_with_config` | 7 | 2 | 4 |
| `cst::parse_stream` on the third document alone | 7 | 2 | 4 |

So the library disagrees with itself on the same input, the shape of
argument that carried `b010` and `b014`, and the fix on the CST side has
a reference implementation in the loader.

**Fix filed 2026-09-06 as noyalib PR #408**, from the fork: a
crate-private `Error::relocate(source, base)` rebuilds every located
variant (the similar-anchor suggestion included) through the same
`Location::from_index` every located error is built with, and
`parse_stream_inner` applies it to a document's failure with the
document's start as `base`. Five tests in `tests/cst_stream.rs`, one of
them asserting equality with `load_all_as` on the same bytes.

Verified against yqr with a temporary `[patch.crates-io]` on the branch,
and the result is the reason `yqr-f029` exists: the fix inverts §2's
guard. The re-parse of the document alone still reports the
slice-relative line, the stream error now reports the stream line, the
two strings differ, and every stream finding loses its position. The
adoption has to drop the re-parse and trust the location; `f029` §2 has
the diff. The draft as filed, for the record:

> **`parse_stream*` errors carry document-relative locations and no
> document index.** `cst::parse_stream_with_config` splits at `---` and
> parses each document alone, so `Error::location()` on a failure in the
> third document counts from that document's marker line. The caller
> passed the whole stream and gets no way back: it has to mirror
> `doc_boundary`'s rule and re-parse. Either offset the location by the
> document's start before returning, or add the document index to the
> located variants. Measured on 0.0.34 with
> `a: 1\n---\nb: 2\n---\nc: *nope\n`: index 7, line 2, column 3, which
> is the `---` of document 2 when read against the stream.

## 5. Acceptance

- [x] An unknown anchor in document 3 reports `5:4`, unit and CLI
- [x] A key collision in document 3 reports `6:1` with the document note
- [x] The similar-anchor hint names the anchor's line in the stream
- [x] The three `b: [1,` tests re-baselined to `3:7`, with the reason at
      the assertion
- [x] `document_starts` pinned on the marker rule
- [x] Full suite green; every other stream finding (`Y002`, `Y101`,
      `Y103`) already positioned from the stream and unchanged
- [x] Filed upstream as noyalib#407 and fixed in PR #408 (§4);
      the adoption of the release carrying it must drop the re-parse and
      trust the location, `yqr-f029`
