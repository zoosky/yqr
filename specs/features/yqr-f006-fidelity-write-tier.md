# yqr.f006 — Fidelity write tier: surgical, byte-preserving edits (`--in-place`)

**Status:** Draft
**Epic:** Fidelity-first architecture (a001)
**Owner:** yqr maintainers
**Related:** `yqr-f002` (fidelity read floor / engine seam), `yqr-f005`
(`--preserve`), `yqr-m002` §4/§6.2 (write-tier seam design), `yqr-b004` (noyalib
0.0.14 mutation-API gaps), `yqr.f001` (M1 literals, M4 assignment)

## 1. Thesis — where yqr wins

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

## 4. Out of scope (deferred, tracked)

- **`|=` update with a computed RHS** — needs the expression evaluator on the
  right (arithmetic/builtins, `f001` M2). Deferred to a follow-up; until then a
  `|=` filter errors with a clear "not yet supported" message.
- **The `yqr-b004` gaps** — comment editing (2.1), key rename (2.2), sequence
  reorder/move/swap (2.3), and multi-line / nested / sole-entry / flow delete
  (2.4). Each errors with a message naming the limitation; they graduate as
  upstream noyalib PRs land.
- **Fragment auto-quoting** (`b004` 2.5) — avoided by routing all scalar writes
  through `set_value` rather than the raw `fragment` mutators.

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

## 9. Sequencing (within the write tier)

- **v1 (this spec):** `=`, `+=`, new-key assignment, `del` (single-line), `-i`,
  scalar-literal / path RHS — all on 0.0.14's first-class guarded mutators.
- **v2:** the `yqr-b004` gaps (comment editing, key rename, reorder,
  nested/multi-line delete) as upstream noyalib PRs land; yqr uses raw
  `replace_span` fallbacks only where it must, behind the same integrity guard.
- **v3:** `|=` update with a computed RHS, once `f001` M2 (arithmetic/builtins)
  provides the right-hand evaluator.
