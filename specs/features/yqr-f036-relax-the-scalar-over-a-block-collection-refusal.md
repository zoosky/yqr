# Feature f036 — Reconsider the scalar-over-a-block-collection refusal

**Status:** Done — option 3 shipped 2026-09-18; filed 2026-09-17 from
`yqr-f035` §2.4
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-b029` (the defect the refusal was built for, resolved by
it), `yqr-f032` (which added it), `yqr-f025` (a refusal names a remedy
that runs), noyalib#424, `yqr-b026` (the anchor-definition write),
`yqr-b035` and `yqr-f038` (follow-ups from its code review)

## 1. Why this is open

`.k = 5` where `k` holds a block mapping is refused at exit 5, and the
refusal names the remedy: `del(.k)` then assign, which puts the value at
the end of the mapping. The refusal exists because of `yqr-b029`: the
engine wrote the scalar at the key's own column, producing a document
noyalib read back and PyYAML and Psych rejected.

noyalib#424, released in 0.0.44, fixes that. Measured with yqr's
pre-emptive guard bypassed:

| release | `.k = 5` over `k:` / `  a: 1` |
|---|---|
| 0.0.43 | `k:` / `5` — column 0, invalid |
| 0.0.44 | `k:` / `  5` — valid, means `k: 5` |

So the refusal now guards against a layout that is correct rather than one
that is broken.

## 2. The question

Valid is not the same as right. `k:` / `  5` is a layout no author writes,
produced by an edit that named a value, and a reviewer reading the diff
sees a key whose value moved to the next line for no reason they asked
for. The alternatives:

1. **Keep the refusal.** The remedy works and puts the value where an
   author would write it. Costs a capability that the engine now supports.
2. **Drop it and take upstream's layout.** One fewer refusal; produces the
   odd two-line spelling.
3. **Write the scalar on the key's own line.** What the user means, and
   what `del`-then-assign produces except for the position in the mapping.
   Needs yqr to replace the entry's whole span rather than its value's,
   which is a write path yqr does not have.

Option 3 is the one worth pricing, because it is the only one that makes
the edit do what it says. The `yqr-b006` argument applies to how the span
is derived: from the engine's key and value spans, not from a
column-counting scan.

## 3. Decision: option 3

A scalar written over a block collection at a mapping key goes on the key's
own line. `.k = 5` over `k:` / `  a: 1` writes `k: 5`.

Three reasons, each measured:

- **It is what the edit says.** The filter names a value, and the author of
  the file would have typed `k: 5`. Option 2's `k:` / `  5` is valid but
  reads in a diff as a value that moved for no reason.
- **Sequence items already work this way.** `.l[0] = 5` over a block item
  writes `- 5` on the dash's line, because an item's value starts there.
  The refusal only ever covered mapping keys, so option 1 kept an
  inconsistency, and option 3 removes it.
- **It costs yqr no rendering.** The implementation (§4) leaves every byte
  of the new value to the engine, which is the `yqr-b006` rule applied:
  spans from the engine, not a column-counting scan, and no second emitter.

Option 2's output is worse than its layout suggests. On 0.0.45, with the
guard bypassed, upstream also leaves the first child's head comment and a
comment below the last child in place around the new scalar:
`k:` / `  # about a` / `  5` / `  # trailing`.

## 4. How it works

`src/fidelity/write/collapse.rs`, called from `NoyalibWriter::set_value`
where the refusal used to be. On a copy of the document:

1. `replace_span` puts a `null` placeholder over everything from just after
   the key's `:` to the end of the value's span. A comment on the key's own
   line is kept, after the placeholder.
2. `set_value` writes the real value over the placeholder. That is a scalar
   over a scalar, so the engine chooses the spelling: plain, quoted
   (`"yes"`), a block scalar for multi-line text, and the document's line
   break.

The copy replaces the document only when it loads as the original with the
assignment applied, and when every byte before the `:` and after the value
is unchanged. `check_integrity` still runs after it.

What goes with the old value follows `del`: the children, and the head
comments above them. A comment below the last child is not the value's for
`del` and stays here too, as does a blank line a `|+` scalar kept.

Two cases change the second step or stop before the first:

- **The block sits inside a value `*name` sites share** (`a: &x` /
  `  k:` / `    n: 1` / `b: *x`, then `.a.k = 5`). The engine refuses
  `set_value` there. The write goes to the definition instead, through the
  same splice `write::anchor` uses for a scalar inside an anchor, and `b.k`
  reads `5`. That is the `yqr-b026` rule, and `.a.k.n = 5` already worked
  this way. Like that path, it takes a value that fits on one line; a
  multi-line one is refused with "does not fit on a single line here".
- **The block defines an anchor an alias outside it still uses**
  (`k:` / `  a: &x 1` / `j: *x`). Replacing it would leave `*x` pointing
  at nothing, or, when an earlier `&x` exists, bind `*x` to that one and
  change its value. Both are refused, naming the anchor and the alias's
  line. The check reads the engine's `anchors()` and `aliases()` lists and
  applies YAML's binding rule: an alias refers to the closest `&name`
  before it. `del` shares the check, since it had the same gap. An alias
  inside the block goes with it and is not a reason to refuse.

Both were found in code review. Before, the first refused with a message
pointing at the engine's `materialise_aliases_of`, which a yqr user cannot
call. The second said "unknown anchor: x", which the user's file does not
have. The remedy the new refusal names is an edit to the file, because
yqr cannot yet remove or change an alias-valued entry (`yqr-b035`).

Three shapes do not take this path:

| shape | result | why |
|---|---|---|
| `k: &an` / `  a: 1`, or `k: !t` / ... | refused, naming `&an` or `!t` | the scalar would drop the property, and an anchor may be what other entries refer to |
| `k: *x`, where `x` is a block | the engine's refusal: "edit the anchor definition or replace the alias explicitly" | an alias is not a block in the source. The old guard fired first here with the wrong reason ("at its key's own column"), because `span_at` on an alias returns the anchor's bytes |
| `k: {a: 1}` | the engine's `set_value`, unchanged | a flow value already shares its key's line |

One byte is the engine's choice, not yqr's: when the new value becomes a
block scalar, a comment on the key's line is written `k: |- # c`, with one
space before the `#` however many there were.

## 5. Measured

Every result below re-parses and passes `yqr validate --strict`.

| input | filter | 0.0.45 before | now |
|---|---|---|---|
| `k:` / `  a: 1` | `.k = 5` | refused | `k: 5` |
| `k:` / `- 1` / `- 2` (sequence at the key's column) | `.k = 5` | refused | `k: 5` |
| `top:` / `  k:` / `    a: 1` | `.top.k = 5` | refused | `  k: 5` |
| `- k:` / `    a: 1` | `.[0].k = 5` | refused | `- k: 5` |
| `k:  # tuned` / `  a: 1` | `.k = 5` | refused | `k: 5  # tuned` |
| `k:` / `  # about a` / `  a: 1` / `  # trailing` | `.k = 5` | refused | `k: 5` / `  # trailing` |
| `k:` / `  a: 1` | `.k = "a\nb"` | refused | `k: \|-` / `  a` / `  b` |
| the same, CRLF | `.k = "a\nb"` | refused | the same, every line `\r\n` |
| `"k":` / `  a: 1` | `.k = 5` | refused | `"k": 5` |
| `? k` / `:` / `  a: 1` | `.k = 5` | refused | `? k` / `: 5` |
| `k:` / `  a: 1` | `.k \|= "x"` | refused | `k: x` |
| `k:` / `  a: 1` | `.k = .missing` | refused | `k: null` |

## 6. Tests

- `src/fidelity/write/collapse.rs`: ten unit tests covering nested, root,
  inside-a-sequence-item and key-column-sequence keys; a key-line comment;
  comments inside the block; the engine's spelling of plain, quoted,
  multi-line and null values; CRLF; path right-hand sides; the property
  refusal; and the alias falling through to the engine.
- `src/fidelity/write/collapse.rs`, from code review: the write inside a
  shared anchor, with the alias reading the new value; the multi-line
  refusal there; the removed-anchor refusal with no earlier definition and
  with one; and an anchor used only inside the block.
- `src/fidelity/write/delete.rs`: the removed-anchor refusal for `del`,
  and its line number counted from the start of a multi-document input.
- `src/fidelity/write/anchor.rs`: the binding rule, including a later
  definition that shadows the removed one.
- `src/fidelity/write/guards.rs`: the two refusal tests are removed. The
  flow, sequence-item and property tests stay, since those shapes never
  went through the refusal.
- `tests/corpus/mod.rs`: `write/collection-rhs/refuses-a-scalar-over-a-block-collection`
  becomes `write/collection-rhs/a-scalar-replaces-a-block-collection`, and
  it pins the bytes: `labels:` and its two lines become `labels: 3`.

## 7. Acceptance criteria

- [x] One of the three options in §2 chosen, with the reason recorded (§3).
- [x] The corpus case and the `guards.rs` tests updated, and the byte
      result pinned (§6).
- [x] The entry-span write path covered for a nested key, a root key, a key
      with a trailing comment, and a key whose value block carries comments
      of its own (§5, §6).
- [x] The Kubernetes guide and the README no longer list the refusal.
- [x] Code review: a block inside a shared anchor is written at the
      definition, and removing an anchor still in use is refused by name,
      for `del` too (§4). `del` inside a shared anchor is filed as
      `yqr-f038`, and the missing alias-entry edits as `yqr-b035`.
- [x] `local-ci.sh` clean.
