# Bug b014 — An empty collection written at its key's own column is invalid YAML that noyalib accepts

**Status:** Open — **§3.2 fixed 2026-08-19** (route 3: `validate` reports it
as `Y103`); §3.1, the upstream writer, is still open and not filed upstream as
part of either change
**Severity:** Medium — nothing in yqr writes this shape today, so there is no
live corruption; what is live is the **validator false negative** in §3.2,
and the fact that every guard in the write loop is blind to the shape
**Component:** noyalib's parser (leniency) and `Document::remove`'s sole-entry
arm (the writer), reached from yqr's `validate` and from any future
delegation of the sole-entry delete
**Related:** `yqr-f018` §4 (the measurement that found it), `yqr-f016` §5 (the
decision it re-decides), `yqr-b006`/`yqr-b010` (the same silent-wrongness
class), `yqr-b011` (the mirror image — noyalib *refusing* valid YAML),
`yqr-f012` (the validator)

## 1. Summary

A block-mapping value must be indented more than its key. noyalib's parser
accepts one that is not, and its sole-entry `remove` writes one:

```console
$ printf 'on:\n[]\njobs: {}\n' | python3 -c 'import sys,yaml; yaml.safe_load(sys.stdin.read())'
yaml.scanner.ScannerError: while scanning a simple key

$ printf 'on:\n[]\njobs: {}\n' | ruby -ryaml -e 'YAML.safe_load($stdin.read)'
Psych::SyntaxError: could not find expected ':' while scanning a simple key

$ printf 'on:\n[]\njobs: {}\n' > samecol.yaml && yqr validate --strict samecol.yaml
$ echo $?
0
```

Two independent implementations reject the document. yqr calls it valid, in
strict mode, and reads it back happily.

## 2. How it is produced

`Document::remove` on the sole item of a block sequence written at its key's
own column — the GitHub Actions / Ansible idiom:

```text
in                  upstream 0.0.24        yqr's delete_entry
on:                 on:                    on:
- push        ->    []                       []
jobs: {}            jobs: {}               jobs: {}
```

The empty collection takes the removed item's own column, which for this
layout *is* the key's column. yqr's own path indents it one level deeper,
because `delete_entry` derives the indent from the parent key rather than from
the deleted line, and that is the only spelling that re-parses everywhere.

## 3. The two faces

### 3.1 The writer

Upstream's sole-entry arm emits the shape at exit 0. yqr does not reach it
today: `yqr-f016` §5 kept the sole-entry class in `delete_entry`, and
`yqr-f018` §5 keeps it there on the strength of exactly this finding. So this
is a live defect in noyalib and a *blocked route* for yqr, not a yqr bug.

### 3.2 The validator — **fixed 2026-08-19**

`yqr validate` walked noyalib's green tree, inherited the parser's leniency,
and reported the file clean. A user running yqr as the correctness gate in a
pipeline got a pass on a file their next tool cannot read. This was `yqr-b011`
seen from the other side: there, noyalib refuses YAML the ecosystem accepts;
here, it accepted YAML the ecosystem refuses.

Fixed by route 3 — the same green-tree walk that already finds duplicate keys
now also measures each block-mapping entry's value against its key's column,
and reports `Y103` in **default** mode (§5). The two same-column layouts that
are genuinely valid are exempt, and each exemption was checked against both
oracles rather than reasoned from the spec:

| Layout | Verdict | Why |
|---|---|---|
| `on:` / `- push` (block sequence at the key's column) | exempt | valid, and the GitHub Actions / Ansible idiom — flagging it would be worse than no check |
| `a:` / `\|` / `  x` (block scalar header at the key's column) | exempt | the scalar's own content sets its indentation |
| `? a` / `: b` (explicit key) | skipped | the value is measured against the `?`, a rule this scan does not claim to know |
| `a:` / `b: 1` (no value, sibling follows) | not a finding | `b` is a sibling entry, and the tree says so |
| `on:` / `[]`, `on:` / `foo` | **`Y103`** | rejected by PyYAML and Psych |

**Coverage of the fix, stated honestly.** noyalib's leniency here is narrower
and less consistent than the shape suggests: it rejects `on:` / `{}` and
`on:` / `[a, b]` outright (those surface as `Y001`), rejects the same
under-indented value when the mapping has no following sibling, and accepts
`on:` / `[]` when one follows. `Y103` covers what the parser lets through; a
document rejected by the parser was never the problem.

## 4. Why every guard misses it

The write loop's integrity guard re-parses the edited document **with
noyalib** and compares typed values. A shape noyalib accepts therefore passes
the guard by construction, exactly as a moved comment passed it in `yqr-b010`
(a comment is not in the typed value) and a stranded one in noyalib#280. The
guard proves *this engine can read it back*, which is a weaker statement than
it looks, and this is the clearest instance of the gap so far.

## 5. Fix routes

1. **Upstream, writer:** derive the empty collection's indent from the parent
   key, as `delete_entry` does. Narrow, and it makes the sole-entry class
   fully delegatable (`yqr-f018` §5's revisit condition).
2. **Upstream, parser:** reject a block-mapping value that is not indented
   past its key. Correct, and the more disruptive of the two — it is a
   leniency users may be relying on, so it belongs behind whatever strictness
   knob the parser grows.
3. **yqr-side, validator: taken 2026-08-19.** The indentation check now runs
   in `validate`'s own green-tree walk, which already looks for things the
   parser does not flag. It fixed §3.2 without waiting on upstream and does
   not depend on route 2 being taken. Routes 1 and 2 remain the fix for §3.1.

## 6. Regression coverage

`src/fidelity/write/delete.rs` — `sole_item_of_a_same_column_sequence_is_indented_under_its_key`
and `sole_item_of_a_nested_same_column_sequence_clears_its_key` pin yqr's
(correct) spelling, and are the two tests that fail under delegation.

`src/validate/mod.rs` — five tests pin §3.2's fix: the located finding, that it
fires in default *and* strict mode, nested and multi-document positions, the
same-column block sequence staying clean, and the other three exempt layouts.
`tests/cli.rs` pins the rendered `error[Y103]` and the exit code. A 28-shape
differential harness against PyYAML and Psych was used to develop the check and
agreed with it on every shape; it is not committed, since it needs both
interpreters.
