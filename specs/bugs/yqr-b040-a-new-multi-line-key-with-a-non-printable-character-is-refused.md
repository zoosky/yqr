# Bug b040 — A new multi-line key with a non-printable character is refused

**Status:** Resolved — 2026-11-21 by noyalib 0.0.63 (yqr v0.14.1)
**Severity:** Low — only a write that adds a new key, with a multi-line
string that holds one of a few rare characters; the file stays unchanged
and yqr exits 5, so no bad bytes reach the disk
**Component:** `src/fidelity/write/shadow.rs` (the insert path), noyalib's
CST emitter
**Related:** `yqr-f052` (the noyalib 0.0.62 adoption that exposed it),
noyalib#533 (the report), noyalib#534 (the fix), accentcms `b212` (the
property test that found it)

## 1. What was observed

On yqr 0.14.0, an assignment that adds a new key fails when the value is a
multi-line string that holds one of these characters:

- a C1 control (U+0080 to U+009F), but not NEL (U+0085)
- the non-characters U+FFFE and U+FFFF
- a byte order mark (U+FEFF)

```text
$ printf 'title: T\n' | yqr '.k = "a￾\nb"'
yqr: runtime error: cannot assign at "k": YAML parse error: insert_entry_value:
inserting `k` into `` could not be spliced (YAML parse error at line 5,
column 1: scalar contains a non-printable Unicode character); the document
was left unchanged
```

The exit code is 5. The same value written over an existing key works,
because noyalib's `set_value` takes a different emitter path.

## 2. Cause

noyalib 0.0.62 refuses these characters outside a quoted scalar. Its CST
emitter still wrote such a string as a literal block scalar (`|`), because
its `can_use_block_literal` check refused only C0 controls and DEL. The
insert then failed its own load-back check, and noyalib left the document
unchanged. yqr forwarded the error.

The bug came with the 0.0.62 adoption (`yqr-f052`). The measurement in
`yqr-f052` §2 tested reads, not inserts with these characters, so it did
not find the bug. The Accent CMS frontmatter property test found it on the
value `"\u{fffe}\n"`.

## 3. Fix

The fix is upstream. noyalib#534 makes `can_use_block_literal` refuse every
character that the parser refuses in a block scalar, so the string takes
the double-quoted spelling, as `to_string` writes it. The maintainer added
the commit to the v0.0.63 release (noyalib#535). yqr v0.14.1 moves to
noyalib 0.0.63. The lockfile changes only noyalib.

The unit test
`a_new_multi_line_key_with_a_non_printable_character_reads_back` in
`src/fidelity/write/shadow.rs` adds each character class as a new key and
reads the value back. It fails on 0.0.62 and passes on 0.0.63.

## 4. Acceptance criteria

- [x] The four inserts in §1 succeed, write a double-quoted value, and read
      back unchanged.
- [x] A test fails on noyalib 0.0.62 and passes on 0.0.63.
- [x] The lockfile changes only noyalib.
