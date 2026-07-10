# yqr.f006 — Write tier v1: value assignment and in-place edits (`--in-place`)

**Status:** Draft
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f007` (write tier: structural edits — the `b004` gaps),
`yqr-f008` (write tier: computed updates `|=`), `yqr-f002` (fidelity read floor /
engine seam), `yqr-f005` (`--preserve`), `yqr-m002` §4/§6.2 (write-tier seam
design), `yqr-b004` (noyalib 0.0.14 mutation-API gaps), `yqr.f001` (M1 literals)

> **Epic anchor.** This is the first of three features in the **Fidelity write
> tier** epic. f006 (this spec) ships the value-replacement core on noyalib
> 0.0.14's first-class mutators; `f007` covers the structural edits that lack a
> first-class API today (the `b004` gaps); `f008` adds `|=` computed updates
> once `f001` M2 lands. The epic thesis lives in §1.

## 1. Thesis — where yqr wins (epic-level)

yqr reads YAML byte-for-byte today (`f002`/`f005`). The differentiator is the
next step: **surgical edits that change only the bytes the filter targets and
leave every other byte — comments, indentation, quoting, key order — untouched,
or refuse.** No mainstream tool occupies this niche:

- **jq** is JSON-only; it cannot preserve YAML comments/formatting at all.
- **yq** edits in place but its own docs admit comment/whitespace "issues" — it
  does not *guarantee* byte-identical, structurally-verified edits.

This is the payoff of the a001 fidelity-first architecture: a **provably
lossless** editor for Kubernetes manifests, Helm values, CI configs, and GitOps
automation — "clean diffs, guaranteed."

## 2. What noyalib 0.0.14 already gives us

Per `yqr-b004` §1, noyalib 0.0.14 provides **first-class, re-parse-guarded**
mutators, and preserves unedited bytes by construction (green-tree `Arc`
sharing). This tier builds on them rather than hand-rolling byte arithmetic:

| Operation | noyalib 0.0.14 API |
|---|---|
| Replace a scalar value (style-matched quoting) | `Document::set_value(path, &Value)` |
| Add a `key: value` (synthesises + re-indents) | `Document::insert_entry(map_path, key, frag)` |
| Append / insert a block-sequence item | `Document::push_back` / `insert_after` |
| Delete a single-line block entry | `Document::remove(path)` |

Each rejects an edit whose result would **re-parse differently** — that guard is
the structural-integrity property this feature promises.

## 3. Scope (v1)

A minimal, jq-aligned mutation surface routed entirely through the first-class,
guarded mutators above:

- **Assignment** `p = rhs`, where `rhs` is a **scalar literal** (number, string,
  `true`/`false`/`null`) or a **path** (`.a.b`, copy the value at another path).
  Emitted via `set_value` so quoting matches the neighbouring style.
- **Delete** `del(p)` for a single-line block entry (`remove`).
- **Add**: `p += item` sugar for appending a block-sequence item (`push_back`),
  and object-merge assignment to a new key `p.newkey = rhs` (`insert_entry`).
- **`-i` / `--in-place`**: write the mutated document back to the input file.
  Without `-i`, the mutated document is printed to stdout (byte-exact except the
  edit).

All writes go through the fidelity engine (`--preserve` semantics are implied
for any mutating filter; the classic re-serializing pipeline is never used for
edits).

## 4. Out of scope (deferred to sibling features)

- **The `yqr-b004` structural-edit gaps** — comment editing (2.1), key rename
  (2.2), sequence reorder/move/swap (2.3), and multi-line / nested / sole-entry /
  flow delete (2.4). These have no first-class noyalib 0.0.14 API and are the
  subject of **`yqr-f007`**. In f006 each errors with a message naming the
  limitation.
- **`|=` update with a computed RHS** — needs the expression evaluator on the
  right (arithmetic/builtins, `f001` M2), tracked as **`yqr-f008`**. In f006 a
  `|=` filter errors with a clear "not yet supported" message.
- **Fragment auto-quoting** (`b004` 2.5) — avoided in f006 by routing all scalar
  writes through `set_value` rather than the raw `fragment` mutators.

## 5. Dependencies

- **Minimal literal RHS** — a scalar-literal subset of `f001` M1 (number,
  string, bool, null) so assignments have a value source. Full M1 construction
  (`{}`, `[]`, interpolation) is not required for v1.
- **`f002` span resolver** — the read path already resolves a filter path to a
  concrete `Path` (`eval_traced`) and to a byte span (`Resolved::Found`); the
  write tier reuses it to target `set_value` / `remove`.
- **`m002` write-tier seam** — extend the engine seam with a mutation surface
  (e.g. a `FidelityWriter` trait, or mutation methods on a boxed `Document`),
  keeping backends pluggable (`m002` §4/§6.2).
- **noyalib 0.0.14** — the mutators and the re-parse guard in §2.

## 6. Design sketch

### 6.1 Grammar (parser)
Add mutating top-level forms to the filter grammar:
- `<path> = <rhs>` (assignment), `<path> += <item>` (append), `del(<path>)`.
- `<rhs>` is a scalar literal or a `.`-rooted path.
- A filter is either a (streaming, read-only) query **or** a single mutation;
  mixing is a parse error in v1.

### 6.2 Evaluation
1. Resolve the LHS `<path>` to a concrete `Path` against the parsed value.
2. Resolve the RHS to a `Value` (literal, or the value at the RHS path).
3. Dispatch to the guarded mutator: `set_value` (`=`), `push_back` (`+=`),
   `insert_entry` (new key), `remove` (`del`).
4. If the mutator's re-parse guard rejects, surface a runtime error (exit 5);
   the input file is left untouched under `-i`.

### 6.3 Output / `-i`
- Default: print `Document::to_string()` (byte-exact but for the edit).
- `-i`: write it back to the input file **atomically** (temp file + rename);
  error if the input is stdin.
- Multi-document: the edit applies to each document whose path resolves; other
  documents are emitted byte-identical.

## 7. Structural-integrity contract

For an accepted edit at path `p`:

- **Locality:** only the bytes of `p`'s node (plus any indentation the mutator
  must synthesise) change; every other byte of the document is identical.
- **Re-parse safety:** if the edit would make the document parse to a different
  structure, yqr **refuses** (exit 5) rather than emit it.
- **Idempotent identity:** an assignment that sets a node to its existing value
  is a no-op at the byte level.

## 8. Acceptance criteria

- [ ] `yqr '.spec.replicas = 5' deploy.yaml` replaces that value; a `diff`
      against the original touches only that line.
- [ ] Scalar writes are quoted to match the neighbouring style (`set_value`).
- [ ] `del(.metadata.labels)` removes a single-line entry byte-exactly.
- [ ] `.spec.ports += 9090` appends a block-sequence item at the right indent.
- [ ] `-i` writes back in place atomically; using `-i` with stdin is an error.
- [ ] An edit that would restructure the document is refused (exit 5) and, under
      `-i`, leaves the file unchanged.
- [ ] Multi-document input: the edit applies to the targeted document; the
      others are byte-identical.
- [ ] Deferred operations (`|=`, key rename, reorder, nested/multi-line delete,
      comment edits) each error with a clear, actionable "not yet supported"
      message.

## 9. Epic sequencing

The **Fidelity write tier** epic ships in three features with distinct
dependencies and release timing:

- **f006 (this spec) — value assignment + in-place.** `=`, `+=`, new-key
  assignment, `del` (single-line), `-i`, scalar-literal / path RHS — all on
  noyalib 0.0.14's first-class guarded mutators. Buildable now; self-contained.
- **`f007` — structural edits.** The `b004` gaps (comment editing, key rename,
  reorder, nested/multi-line delete), gated on **upstream noyalib PRs**; yqr uses
  raw `replace_span` fallbacks only where it must, behind the same integrity
  guard.
- **`f008` — computed updates (`|=`).** Gated on **`f001` M2**
  (arithmetic/builtins) providing the right-hand evaluator.

Priority order: f006 → f007 → (`f001` M2) → f008.

## 10. Implementation build sequence

**Core insight.** yqr already does the hard half. The read path resolves a
filter to a concrete `Path` (`eval::eval_traced`), converts it to a noyalib
string path, and calls `span_at` (`src/fidelity/noyalib.rs:144`). noyalib
0.0.14's mutators are addressed by the **same string path** and **return
`Result<()>`** — that `Result` *is* the re-parse guard (§7). So the write path is
the read pipeline with the terminal call swapped:

```
filter -> eval_traced -> Path -> noyalib path-string -> Document::set_value(path, &Value)
                                                       -> Document::remove(path)
                                                       -> Document::push_back(path, frag)
```

Confirmed noyalib 0.0.14 surface (all string-path; `set_value` takes a
`noyalib::Value`, for which `src/value.rs` already has `From<&Value>`):

| yqr op | noyalib call | src |
|---|---|---|
| `=` (scalar) | `Document::set_value(path, &Value)` | `document.rs:546` |
| `del()` | `Document::remove(path)` | `document.rs:601` |
| `+=` (append) | `Document::push_back(path, frag)` | `document.rs:637` |
| new-key `=` | `Document::insert_entry(map_path, key, frag)` | `document.rs:811` |
| escape hatch (`f007`) | `Document::replace_span(start, end, repl)` | `document.rs:328` |

### 10.1 File-by-file changes

- **`src/lexer.rs`** — add tokens `Eq` (`=`), `PlusEq` (`+=`),
  `LParen`/`RParen` (for `del(...)`); extend `lex_int` -> `lex_number` for float
  RHS. `true`/`false`/`null` already lex as `Ident` (recognised in the parser).
- **`src/ast.rs`** — add a top-level `Program` layer above `Ast` (mutations are
  top-level only in v1): `Program::{Query(Ast), Mutate(Mutation)}`,
  `Mutation::{Assign{path,rhs}, Append{path,rhs}, Delete{path}}`,
  `Rhs::{Literal(Value), Path(Ast)}`.
- **`src/parser.rs`** — `parse` returns `Program`. New `parse_program`: parse a
  `del(...)` form, else parse a pipeline then peek for `=`/`+=` and parse the
  RHS; otherwise `Query`. `parse_pipeline`/`parse_path` are untouched.
- **`src/eval.rs`** — add `resolve_target(ast, value) -> Result<Path>`: run
  `eval_traced`, require **exactly one** result carrying `Some(path)`, else a
  clear error (a mutation targets one addressable node).
- **`src/fidelity/write.rs` (new)** — a minimal `FidelityWriter` seam
  (`m002` §6.2): `set_value` / `append` / `insert_key` / `delete` / `emit`.
  `NoyalibEngine` implements it by reusing the existing `Path -> noyalib
  path-string` builder (extracted from `noyalib.rs:~140`), returning a clear
  "unaddressable" error for non-plain keys (`PathSeg::is_plain`), then calling
  the matching `Document` mutator and mapping its `Err` to `YqrError::eval`.
- **`src/cli.rs`** — add `-i` / `--in-place` (bool). `// Feature f006`.
- **`src/main.rs`** — branch on `Program`: `Query` = today's read path;
  `Mutate` = open the engine, `resolve_target`, dispatch to the writer, output
  `emit()`. With `-i`, write back **atomically** (temp + rename); error on stdin.

### 10.2 Ordered steps (each compiles, tests green)

1. **Spike** — confirm the post-mutation emit method (`Document::source()` vs
   `to_string()`/`Display` after a `set_value`). This is the one real unknown;
   settle it first (see §11).
2. **Lexer** — `=`, `+=`, `(`, `)`, float; unit tests.
3. **AST + parser** — `Program`/`Mutation`/`Rhs`; parse `=`, `+=`, `del(...)`,
   literal/path RHS. `main` still runs only `Query`.
4. **`resolve_target`** in eval + tests (single-node requirement, error text).
5. **`FidelityWriter`** on `NoyalibEngine` + engine unit tests.
6. **Wire `main.rs`** for `Mutate` to stdout; black-box byte-locality test.
7. **`-i`** atomic write-back + stdin guard + tests.
8. **Deferred-op errors** — `|=`, key rename, reorder, nested delete each return
   a clear "not yet supported (`f007`/`f008`)" message.

Estimate: ~6 small, independently-green PRs. Needs **zero upstream noyalib
work** — everything is on the shipped 0.0.14 API.

## 11. Open questions to settle during implementation

1. **Post-mutation emit** — does `Document::source()` reflect edits (likely, as
   `replace_span` mutates in place), or is `to_string()`/`Display` required?
   Resolved by the §10.2 step-1 spike; not assumed here.
2. **Absent-key routing** — an assignment whose LHS final segment does not yet
   exist must route to `insert_entry(parent, key, ...)`, not `set_value` on a
   missing path. Small branch in the writer.
3. **Fragment quoting** (`b004` 2.5) — all scalar writes go through
   `set_value(&Value)` (style-matched quoting); `+=` / new-key fragments must be
   produced via the same `Value -> noyalib` conversion, never a raw user string.
4. **Special-char / non-string keys** — the string-path addressing cannot
   express `a.b`-style keys; the writer returns a clear "unaddressable" error,
   the same honest gap the read path already declares.
