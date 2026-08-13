# Bug b009 — an inserted line ends with `\n`, giving a CRLF document mixed line endings

**Status:** Fixed (2026-08-13) — `emit` restores the document's own convention
for the lines an edit added. Ships with `yqr-f014`.
**Severity:** Medium — silent fidelity loss on a shipped write path, at exit 0,
so `-i` wrote the mixed-ending file to disk and reported success. Lower than
`yqr-b008` only because the result still parses and holds the right values.
**Owner:** yqr maintainers
**Last updated:** 2026-08-13
**Affects:** `src/fidelity/write.rs` — the two line-*adding* operations,
`insert_key` (new-key assignment) and `append` (`+=`). Present since the write
tier shipped (`yqr-f006`, v0.4.0).
**Component:** noyalib's insertion mutators, worked around in yqr.
**Related:** `yqr-b001` (whose acceptance criterion "CRLF line endings and
trailing whitespace are preserved" this violated), `yqr-b004` §2.5 (which
recorded the upstream behaviour without filing it), `yqr-b008` (the same two
call sites, same silent-at-exit-0 class), `yqr-f014` (the release this fix
ships with), `yqr-a001` (byte-fidelity property)

## 1. Summary

noyalib's insertion mutators build a line with a hard-coded `\n` terminator —
`format!("{lead}{indent}{key}: {inline}\n")` — and their
`leading_break_for_splice` helper only ever looks for `b'\n'`. A document that
uses CRLF therefore gains one bare LF per inserted line.

`yqr-b004` §2.5 already recorded this ("a splice into a CRLF document inserts
`\n`") as a known upstream behaviour to be aware of before adopting the typed
tier. It was never filed as a yqr bug, so nothing tracked it and nothing tested
it — which is how `yqr-f014` came within one review of closing the bug tracker
to "Open: none" while shipping it.

## 2. Reproduction

Before the fix, exit **0** in both cases:

```console
$ printf 'm:\r\n  a: 1\r\n' | yqr '.m.b = 2' | od -c
m : \r \n     a :   1 \r \n     b :   2 \n
                                        ^^ bare LF

$ printf 's:\r\n  - 1\r\n' | yqr '.s += 3' | od -c
s : \r \n     - 1 \r \n     - 3 \n
                                ^^ bare LF
```

The damage compounds with `yqr-b008`'s case: a multi-line string inserted into
a CRLF document contributed a bare LF for *every* line of the block scalar.

## 3. Scope — what was and was not affected

Measured per operation on `m:\r\n  a: 1\r\ns:\r\n  - 1\r\n`:

| Operation | CRLF preserved before the fix? |
|---|---|
| read (`.`) | yes — the read path slices original bytes |
| `set_value` (existing key) | yes — replaces value bytes, adds no line |
| `insert_key` (new key) | **no** |
| `append` (`+=`) | **no** |
| `del` | yes — `delete_entry` is yqr's own span arithmetic |

Only the two line-adding operations, which is what makes the fix narrow.

## 4. Fix

`NoyalibWriter` records, per document at open time, whether that document's
source was **wholly** CRLF (`is_all_crlf`: at least one line break, and no bare
`\n` anywhere). `emit` then re-terminates any bare `\n` in such a document as
`\r\n`.

The restore is exact rather than heuristic, and that is the whole argument for
doing it in `emit` rather than tracking edit sites: a wholly-CRLF document has
no bare `\n` of its own, so every bare `\n` present in the output is one the
edit introduced. An untouched document emits byte-identically and therefore has
none, making this a no-op for the documents fidelity cares most about.

**A mixed-ending document is deliberately left alone.** There is no convention
to restore, and picking one would rewrite bytes the user did not ask about —
the same class of unasked-for change the fidelity engine exists to prevent.

## 5. Verification

Five tests in `src/fidelity/write.rs`, all byte-exact:

- `inserting_a_key_keeps_a_crlf_document_crlf`
- `appending_an_item_keeps_a_crlf_document_crlf`
- `a_multiline_insert_into_a_crlf_document_uses_crlf_throughout` — the
  `yqr-b008` overlap; also asserts the value loads back
- `an_lf_document_is_untouched_by_the_crlf_restore`
- `a_mixed_ending_document_is_left_alone`

## 6. Upstream — raised on noyalib#221 (2026-08-13), PR offered

The durable home for this is noyalib: an insertion should reproduce the
document's existing line break rather than assume `\n`, the same way it already
derives indentation from the site. Raised in
[noyalib#221](https://github.com/sebastienrousseau/noyalib/issues/221#issuecomment-5284260094),
in the same comment that corrects the status update (`yqr-f014` §4), with a PR
offered on the #222 / #223 pattern.

**Verified against `upstream/main` @ `554e883` (v0.0.21), not just yqr's
symptom.** The defect is wider than the two mutators yqr uses:

| Call | Result on `m:\r\n  a: 1\r\n` (or the sequence equivalent) |
|---|---|
| `insert_entry_value` | `…  b: 2\n` — bare LF |
| `push_back_value` | `…  - 3\n` — bare LF |
| `insert_entry` / `push_back` / `insert_after` | same, fragment forms |
| `set_comment(Before)` | `m:\r\n  # note\n  a: 1\r\n` — bare LF |
| `set_comment(Inline)` | `m:\r\n  a: 1\r  # note\n` — splices **between** the `\r` and the `\n`, leaving a lone CR |
| `set_value` | CRLF preserved (control) |
| `remove` | CRLF preserved (control) |

No data is lost in any of these — values round-trip, and the inline case stays
valid because YAML 1.2 accepts a lone `\r` as a break. The cost is a file that
comes back with two or three terminators in it.

Cause is small and local: `leading_break_for_splice`
(`crates/noyalib/src/cst/document.rs:4048`) is typed `-> &'static str` and
returns `"\n"`, and the line is built as
`format!("{indent}{key}: {fragment}\n")`.

Until a fix ships in a release yqr can pin, the workaround is yqr's, costs one
pass over the emitted string per edited document, and is covered by §5. It
should be deleted when upstream lands — yqr second-guessing its engine's line
endings is not a state to keep.
</content>
</invoke>
