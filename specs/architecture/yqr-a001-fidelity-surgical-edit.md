# Architecture a001 — Fidelity-First, Surgical-Edit Model

**Status:** Accepted
**Owner:** yqr maintainers
**Last updated:** 2026-06-26
**Supersedes:** open questions in `yqr-r001` §8 (number model, comments)
**Affects:** `yqr.f001` goals; all future mutation/emit features

## 1. Decision

**yqr prioritizes Cohort B — config wranglers (Kubernetes, Helm, Ansible, CI
YAML) — whose primary need is surgical edits that do not vandalize the file.**

The product makes one **hard, non-negotiable guarantee**:

> **Comments, key ordering, and invisible characters (blank lines, indentation
> style, quoting style, trailing whitespace, BOM, and line endings) present in
> the input are preserved in the output. yqr never rewrites bytes it did not
> change.**

This is a product promise *and* an architectural constraint. It outranks jq
compatibility wherever the two conflict.

## 2. North-star acceptance test

The guarantee reduces to one falsifiable property that gates every release:

```
yqr '.' input.yaml   ==   cat input.yaml        # byte-for-byte identical
```

…and more generally, for any filter that mutates exactly one scalar:

```
diff <(cat input.yaml) <(yqr '.a.b = 5' input.yaml)
# → differs ONLY in the bytes of the value at .a.b; everything else identical
```

If `yqr .` does not reproduce its input byte-for-byte, the build is broken.
(Note: this is a *new* requirement the current v0.1.1 pipeline does **not**
meet — see §5.)

## 3. Why the current architecture cannot satisfy this

Today's pipeline is `text → rust_yaml::Value → dump_str → text`. The `Value`
model (`src/eval.rs`, `src/lib.rs`) is a **lossy semantic tree**:

- `Value` carries **no source positions** (verified: `src/value.rs` enum is
  `Null|Bool|Int|Float|String|Sequence|Mapping`, no span fields).
- `CommentedValue` re-attaches comments only as **semantic strings**
  (`leading`/`trailing`/`inner: Vec<String>`), not as source spans — so blank
  lines, exact indentation, quote style (`'x'` vs `"x"` vs bare), flow-vs-block
  style, and line endings are all dropped on the round trip.
- Re-emission via `dump_str` **normalizes** formatting by design.

No amount of polishing the emitter fixes this: a semantic round-trip is
structurally incapable of byte fidelity for untouched regions.

## 4. Chosen architecture: source-preserving, span-based editing

The canonical representation is **the original source text plus a node → byte
span index** — not a detached value tree. This mirrors the round-trip model of
`ruamel.yaml` and `toml_edit`.

```
                         ┌─────────────────────────────────────────┐
  input bytes ──────────▶│ source text (kept verbatim, immutable)   │
        │                └─────────────────────────────────────────┘
        ▼                         ▲ slice                 ▲ splice
  scanner (rust_yaml Tokens   ┌───┴───────┐         ┌─────┴───────────┐
  with start/end Position) ──▶│ span index │         │ edit list        │
                              │ path→[a,b) │         │ (path, newbytes) │
                              └───┬────────┘         └─────┬───────────┘
                                  │ build Value-with-paths │ from filter eval
                                  ▼                        ▼
                           read/query path           write/mutate path
```

Two paths, deliberately asymmetric:

### 4.1 Read / query path (`.a.b`, `.[]`, `select`, …)
- Parse to a value model **annotated with each node's byte span** (built from
  scanner `Token.start_position.index … end_position.index`; `Position.index`
  is a **byte** offset — verified via `Position::advance` using `len_utf8`).
- Evaluate the filter to select nodes.
- **Emit a selected node by slicing its original source span**, not by
  re-serializing it — so a projected subtree keeps its own comments, ordering,
  and formatting. Re-serialize **only** synthesized nodes (literals, `{...}`,
  `[...]`) that have no original bytes.

### 4.2 Write / mutate path (`=`, `|=`, `del`, in-place `-i`)
- The filter produces a set of `(path, new_value)` edits.
- Map each path to its byte span via the index; **splice** the new
  serialization into just that range. Every other byte is copied verbatim,
  making untouched regions byte-identical *by construction* rather than by
  best-effort.
- Synthesized values are serialized to match **local context** (inherit the
  sibling's indent and quote style where determinable).

### 4.3 Implementation seam
A new module owns the source/span layer (proposed `src/source.rs` or a
`fidelity/` module), built on `rust_yaml`'s **scanner token stream** (which
exposes spans) rather than on `Value` (which does not). `Value` remains the
evaluation currency; it is wrapped/paired with spans, not replaced.

## 5. Consequences

- **Breaking the v0.1.1 contract is intended.** Current `yqr .` reformats; under
  a001 it must round-trip. This is a deliberate, pre-1.0 correction, captured
  here so the change is a decision and not a regression.
- The **read path can ship fidelity first** (slice-on-emit) before the full
  mutate path exists — high user value, lower complexity. Recommended first
  increment.
- **Multi-document and BOM/CRLF handling are in scope from the start**, because
  they are invisible-character fidelity, not a later nicety (reinforces the
  `yqr-r001` §7 recommendation to pull multi-doc forward).
- A **fidelity test corpus** of real artifacts (a K8s manifest with comments and
  anchors, a Helm `values.yaml`, a GitHub Actions workflow, a CRLF file, a
  BOM file) becomes a required test asset; the §2 property runs over all of it.

## 6. Resolution of `yqr-r001` open questions

| r001 open question | Resolution under a001 |
|--------------------|------------------------|
| **Number model** (`Int` vs `f64`) | **Preserve types.** `Int op Int → Int` when exact; `Float` only when genuinely fractional. Fidelity forbids silently turning `replicas: 3` into `3.0`; large `i64` IDs must not lose precision. Compare/sort by mathematical value. |
| **Comments × transformation** | Preserved via **source spans**, not `CommentedValue` strings. Untouched node → bytes copied verbatim. Deleted node → its attached comments/blank-lines go with its span. Moved/renamed → comments follow the span when identity is clear; nearest surviving anchor otherwise. Synthesized node → no comments; never fabricated. |
| **`--raw-output` scope** | Unchanged: top-level string results only (matches jq). Orthogonal to fidelity. |
| **Regex engine** | Unchanged by a001: "jq-like" on the Rust `regex` crate, explicit errors on unsupported constructs. Not a fidelity concern. |

## 7. Known risks

- **Span precision in scanner fast paths.** `Position::advance_by` increments
  `index` by a raw count rather than per-`char`; if any hot path uses it for
  multi-byte text, spans could be off. Must be validated against the §5 corpus
  (especially non-ASCII keys/values) before relying on splices.
- **Anchors/aliases & merge keys (`<<`).** Editing an aliased node, or a node
  reachable through an alias, needs a defined policy (edit anchor? expand?).
  Out of scope for the first increment; flag explicitly when encountered.
- **Synthesized-value formatting** (indent/quote inheritance) is heuristic;
  document where yqr's choice may differ from a hand edit.

## 8. Non-goals (unchanged or newly explicit)

- jq-*identical* numeric/regex semantics (we are jq-*like*; see "Differences
  from jq" — to be added to user docs).
- Module system / `import` / `include` (per the senior-design recommendation:
  local `def` eventually; modules never, absent real demand).
- Reflowing or "prettifying" YAML — yqr is an editor, not a formatter. The
  absence of normalization is the feature.
