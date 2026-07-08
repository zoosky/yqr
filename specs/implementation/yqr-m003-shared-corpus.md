# Implementation m003 — Shared real-world corpus for validation and benchmarks

**Status:** Done
**Owner:** yqr maintainers
**Last updated:** 2026-07-08
**Related:** `yqr.f001` (the filter language under test), `yqr-f002`/`yqr-f003`/`yqr-f004` (the fidelity engines), `yqr-m002` (the engine seam)

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
  mod.rs     # types (Case, Expect, EngineCase, Engine) + case tables
  docs.rs    # the real-world YAML documents + the `inventory(n)` generator
tests/corpus_validation.rs   # asserts every case (classic + engine)
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
  **exact bytes** emitted, for each backend the case applies to (`BOTH` for the
  byte-identity guarantees every backend must honor). Backends absent from the
  current build are skipped, so the corpus is correct under
  `--no-default-features` and each single-backend profile.

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

`corpus_ids_are_unique` guards against silently colliding benchmark labels.

## 5. Benchmark shape

`benches/corpus_bench.rs` groups: `corpus/parse_all` (parse every filter),
`corpus/classic_all` (run every classic case), `corpus/engine_all` (every engine
case on every compiled-in backend), and two scale groups built from
`inventory(n)` at n = 100 / 1000 — `corpus/scale_iterate` (classic
iterate-and-project) and `corpus/scale_engine_identity` (byte-for-byte identity)
— annotated with element throughput.

## 6. Extending

Add a `Case` to `classic_cases()` (or `EngineCase` to `engine_cases()`) with a
unique `category/name` id. The validation suite asserts it and the benchmark
times it automatically. Add new documents to `docs.rs`; keep them verbatim so
the fidelity engines exercise genuine formatting.

## 7. Acceptance criteria

- [x] One corpus module consumed by both the validation test and the benchmark.
- [x] At least one case per implemented filter operation and pipeline behavior,
      plus the full error taxonomy and fidelity byte-identity dimensions.
- [x] Classic expectations compared semantically; raw and engine expectations
      byte-exact.
- [x] Correct under the default, `--no-default-features`, and each
      single-backend profile (absent backends skipped).
- [x] `cargo test`, `cargo clippy -- -D warnings` (all four profiles),
      `cargo bench --no-run`, and a live `cargo bench --bench corpus_bench` all
      pass.
