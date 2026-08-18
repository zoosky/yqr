# Implementation m003 — Shared real-world corpus for validation and benchmarks

**Status:** Done
**Owner:** yqr maintainers
**Last updated:** 2026-08-18
**Related:** `yqr.f001` (the filter language under test), `yqr-f002`/`yqr-f003`/`yqr-f004` (the fidelity engines), `yqr-m002` (the engine seam), `yqr-m005` (the single-engine consolidation), `yqr-f006`/`yqr-f007` (the write tier the third case tier covers), `yqr-b008` §6 (the gap that asked for it)

## 1. Purpose

A single, real-world **corpus** that is the source of truth for both
functional validation and performance measurement. One case, authored once, is
asserted by the test suite and timed by the benchmark suite — so coverage and
benchmarks never drift apart, and a new case is measured the moment it is
validated.

The corpus is built from genuine documents (Kubernetes, GitHub Actions, Docker
Compose, Helm, application config, multi-document streams) rather than toy
input, so it exercises the pipeline on the shapes users actually run.

## 2. Layout

```
tests/corpus/
  mod.rs     # types (Case/Expect, EngineCase, WriteCase/WriteExpect) + case tables
  docs.rs    # the real-world YAML documents + the `inventory(n)` generator
tests/corpus_validation.rs   # asserts every case (classic + engine + write)
benches/corpus_bench.rs      # times the same cases + scale variants
```

`tests/corpus/` is a subdirectory, so Cargo does **not** compile it as its own
test crate. It is pulled into both consumers with `#[path = "…/corpus/mod.rs"]
mod corpus;`; Rust resolves the nested `mod docs;` relative to `mod.rs`'s real
location, so the same files compile unchanged in either crate. The module
depends only on `std` and its own data types, keeping it consumer-agnostic.

## 3. Case model

- **Classic cases** (`Case`) run through the re-serializing pipeline
  (`eval_str` + optional `render`). Expectations (`Expect`):
  - `Values(yaml)` — the output stream equals the documents parsed from `yaml`,
    compared **semantically** (value equality), so the emitter's formatting is
    irrelevant.
  - `Empty` — the output stream is empty (e.g. an error swallowed by `?`).
  - `Raw(s)` — `render(out, raw = true)` equals the exact string `s`.
  - `Err(code)` — the pipeline fails with jq-style exit code `code`
    (`3` lex/parse, `5` eval/IO).
- **Engine cases** (`EngineCase`) run through `fidelity::run` and assert the
  **exact bytes** emitted. The tier predates `yqr-m005`, which made noyalib the
  only engine and removed the per-case backend field; there is one engine, and
  every engine case applies to it.
- **Write cases** (`WriteCase`) compile a **mutating** filter with
  `parser::parse_program` and apply it with `fidelity::write::apply`. Neither
  read tier can reach a mutation — `fidelity::run` goes through
  `parser::parse`, which rejects one by design — so without this tier no edit
  is covered by the corpus at all, and a backend bump could reintroduce a
  splice corruption without failing a single case. Expectations
  (`WriteExpect`):
  - `Rewrites([(from, to), ...])` — the output is the input with each `from`
    span rewritten to `to`, once, **and no other byte changed**. The expected
    document is *built* from the input, so the assertion covers the whole file
    while the case states only the edit; each `from` must match exactly one
    span, so an anchor can never drift onto the wrong bytes.
  - `Unchanged` — the output is byte-identical to the input, either because
    the target resolves nowhere (a no-op, so a batch edit skips files lacking
    the path) or because the edit writes back what was there.
  - `Err(code)` — the mutation is refused with jq-style exit code `code`.
    Refusal is half the write contract: an edit that would restructure the
    document must fail rather than emit something plausible.

  Every successful write case additionally has its **output** validated
  (`validate::check_str`, strict): an edit may not produce a document yqr
  itself would reject.

## 4. Coverage

Every implemented filter operation and pipeline behavior has at least one case:

| Area | Cases |
|---|---|
| Identity | `identity/k8s` (+ engine byte-identity) |
| Field access | top-level, nested, deep-through-index, bracketed special-character key, quoted-scalar value |
| Indexing | positive, negative, out-of-range → null |
| Iteration | sequences, mapping values |
| Pipes | iterate-then-field, multi-stage, explicit stages |
| Optional `?` | suppresses type error, passes value through |
| Null propagation | missing field, propagation through a field |
| Raw output | top-level string, iterated strings, non-string fallback |
| Multi-document | classic first-document semantics |
| Error taxonomy | field-on-scalar, iterate-scalar, index-mapping (exit 5); non-dot start, trailing bracket, lexer (exit 3) |
| Fidelity engines | byte-identity (k8s, compose, rich formatting, multi-doc), source-preserving projections, raw output |
| Write: assignment | scalar in place, neighbouring quote style, idempotent write, multi-line string |
| Write: insertion | new key under a nested mapping, multi-line value, type-forcing quote; refused on a mapping whose keys hold a `.` (`yqr-b012`) |
| Write: append | sequence item at the site's indent, multi-line item, multi-line item with inner indentation |
| Write: delete | nested multi-line mapping, sole entry, flow sequence member, flow mapping member, absent path (no-op); refused when it would strand an alias |
| Write: key rename | key token only; refused on a sibling collision |
| Write: comments | inline added, inline changed, inline removed, head above the entry; refused on a multi-line entry and on an entry whose value starts below |
| Write: reorder | `swap` of multi-line items, `move` with a negative index, flow members; refused out of range |
| Write: cross-cutting | CRLF terminator on an inserted line (`yqr-b009`), a multi-document stream edited in every document that resolves |

`corpus_ids_are_unique` guards against silently colliding benchmark labels.
Every corpus document — read and write tiers alike — must also validate
cleanly in strict mode (`corpus_documents_validate_cleanly`).

## 5. Benchmark shape

`benches/corpus_bench.rs` groups: `corpus/parse_all` (compile every filter —
write filters through `parse_program`, since `parse` rejects a mutation and
timing them any other way times the error path), `corpus/classic_all` (run
every classic case), `corpus/engine_all` (every engine case),
`corpus/write_all` (compile and apply every write case, which is the span
arithmetic plus the re-parse integrity guard), and three scale groups built
from `inventory(n)` at n = 100 / 1000 — `corpus/scale_iterate` (classic
iterate-and-project), `corpus/scale_engine_identity` (byte-for-byte identity)
and `corpus/scale_write` (one targeted edit, whose cost is dominated by the
O(document) re-parse guard rather than the splice) — annotated with element
throughput.

Only the `eval` bench target is tracked over time on `main` (`yqr-m001`), so
these groups are a local regression check, not a dashboard series.

## 6. Extending

Add a `Case` to `classic_cases()`, an `EngineCase` to `engine_cases()`, or a
`WriteCase` to `write_cases()`, with a unique `category/name` id. The
validation suite asserts it and the benchmark times it automatically. Add new
documents to `docs.rs`; keep them verbatim so the fidelity engines exercise
genuine formatting.

Two conventions the write tier adds:

- **Pin what the engine does, not what it should do.** Two upstream behaviours
  the tier found on its first run — `yqr-b012` and `yqr-b013` — are recorded
  as the refusal and the spelling they actually produce, with a comment naming
  the bug. A case that asserts the desired behaviour instead would have to be
  `#[ignore]`d, and an ignored case measures nothing; a pinned one fails the
  day the behaviour changes, which is exactly when someone should look at it.
- **Keep the anchors small and unique.** A `Rewrites` anchor is a literal
  substring of the document, and the checker requires it to match exactly one
  span, so prefer the line with its terminator (`"  replicas: 3\n"`) over a
  bare fragment.

## 7. Acceptance criteria

- [x] One corpus module consumed by both the validation test and the benchmark.
- [x] At least one case per implemented filter operation and pipeline behavior,
      plus the full error taxonomy and fidelity byte-identity dimensions.
- [x] Classic expectations compared semantically; raw and engine expectations
      byte-exact.
- [x] At least one case per implemented **write** operation, plus a refusal per
      integrity guard, each stating the bytes it changes and thereby the bytes
      it does not.
- [x] Every corpus document validates cleanly in strict mode, and so does the
      output of every successful write case.
- [x] `cargo test`, `cargo clippy -- -D warnings`, `cargo bench --no-run`, and
      a live `cargo bench --bench corpus_bench` all pass.
