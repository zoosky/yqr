# Bug b022 — A file with no trailing newline after a blank value is misread, and `validate` calls it invalid

**Status:** Resolved **2026-08-24** — reported as noyalib#312, fixed upstream
by noyalib#313, released in **noyalib 0.0.28**, and yqr pins it. Found
2026-08-23 while patching `yqr-b021` upstream. See §7
**Severity:** Medium — `validate` reports an error on valid YAML, which is the
one output that has to be trustworthy; the read faces are silent wrong values
**Component:** noyalib's parser, reached from every yqr entry point
**Related:** `yqr-b021` (the write-side bug whose measurement turned this up),
noyalib#312 (the report) and noyalib#313 (the fix), `yqr-f012` (`validate`),
`yqr-a001`

## 1. Summary

A `:` at **end of input** is not read as a mapping value indicator. A `:`
followed by a space or a line break is. So the same document with and without
a trailing newline parses two different ways, and yqr shows it three times
over:

```console
$ printf 'a:' | yqr '.a'
yqr: runtime error: cannot index string with field "a"

$ printf 'a: 1\nb:' | yqr '.b'
yqr: io error: failed to parse YAML input: YAML parse error: simple key was required but not found

$ printf 'a: 1\nb:' > f.yaml && yqr validate f.yaml
error[Y001]: simple key was required but not found
 --> f.yaml
```

PyYAML and Psych both read `a:` as `{a: nil}` and `a: 1\nb:` as
`{a: 1, b: nil}`. The controls agree everywhere: `a: `, `a:\n`, `a: 1\nb:\n`
and `a: #c` all load as mappings in all three, and `a:#c` is correctly a plain
scalar in all three (a `#` needs preceding whitespace to open a comment).

## 2. `validate` is the face that matters

The read faces are bad, but `validate` is worse than either, because it is the
command whose whole job is to answer "is this file correct". It answers **no**
about a file two reference implementations accept, with a `Y001` and a caret.
A user acting on that would go looking for a syntax error that is not there.

It is also the most reachable face. It needs one ordinary mapping whose last
value is blank and no trailing newline — `printf` without `\n`, a heredoc, an
editor configured not to add one, a generated fragment.

## 3. Not a fidelity defect

Worth stating, because the adjacent guarantee is yqr's headline one. The
identity read is byte-exact:

```console
$ printf 'a:' | yqr '.'
a:
```

The fidelity engine slices original bytes, so a misparse of this shape does not
corrupt output — it produces a wrong *value*, and on the `a: 1\nb:` shape a
refusal. `yqr-a001` §1 holds throughout.

## 4. Fix route

Upstream's. yqr has no lever here: the parser decides what a document is, and
every yqr tier — classic, fidelity, `validate` — reads that decision.

Reported as **noyalib#312** with the nine-shape table and both reference
implementations. The argument is the library's own inconsistency: `"a:\n"` and
`"a:"` are the same document one byte apart, the newline is not content, and
noyalib reads the first as a mapping and the second as a string.

**Do not work around it in yqr.** Appending a newline before parsing would
change the bytes the fidelity engine slices, which trades a wrong value for a
broken guarantee — the wrong side of `yqr-a001` §1. Wait for the fix.

## 5. On adoption

Add the four shapes to `tests/fidelity.rs` or the corpus as a read case, and
the `validate` face as a CLI test. None can be pinned now: they would assert
the defect.

## 6. Reproduction

```console
$ printf 'a:' | yqr '.a'                    # error; expected null
$ printf 'a: 1\nb:' | yqr '.b'              # parse failure; expected null
$ printf 'a: 1\nb:' > f.yaml; yqr validate f.yaml   # Y001 on valid YAML
$ printf 'a:\n' | yqr '.a'                  # null -- correct, one byte apart
```

## 7. Adoption, 2026-08-24

The fix is upstream's, as §4 said it had to be: noyalib#313 treats a `:` at end
of input as a value indicator, so the scanner stops depending on a byte that is
not content. yqr moved its pin from `noyalib = "0.0.27"` to `"0.0.28"` and
changed nothing else — there was no workaround to remove, because §4's
instruction not to write one was followed.

Every §6 reproduction now reads as a mapping:

```console
$ printf 'a:' | yqr '.a'                            # null
$ printf 'a: 1\nb:' | yqr '.b'                      # null
$ printf 'a: 1\nb:' > f.yaml; yqr validate f.yaml   # silent, exit 0
$ printf 'a:\n' | yqr '.a'                          # null -- the control
```

§5 asked for the four shapes as a read case and the `validate` face as a CLI
test. Both, plus the byte guarantee §3 is about:

- `a_colon_at_end_of_input_is_a_value_indicator` (`tests/cli.rs`) — the four
  shapes, and the sibling key the parse failure used to take with it.
- `validate_accepts_a_blank_value_at_end_of_input` — the face that mattered
  most, in `--strict` where a false positive would be loudest.
- `a_document_with_no_trailing_newline_is_echoed_byte_for_byte` — §3's
  property, now that a fix has touched the parser that feeds it.
- `field/implicit-null-at-end-of-input` and `field/sibling-of-a-blank-tail`
  (`tests/corpus/mod.rs`) — the read case, on a corpus document
  (`BLANK_TAIL_FRAGMENT`) whose last value is blank and which ends without a
  newline. Adding it to the corpus pins the `validate` face a second time for
  free: `corpus_documents_validate_cleanly` runs every corpus document through
  the validator in strict mode.
- `engine/identity/blank-tail-fragment` — the identity read stays byte-exact,
  and no newline is invented on the way out.
