# Implementation m003 — Shared real-world corpus for validation and benchmarks

**Status:** Done
**Owner:** yqr maintainers
**Last updated:** 2026-09-02
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
  values.rs  # the values corpus: tests/data/values.yaml, the `tenants(n)`
             # generator, and its cases for the three tiers above (s7)
  cli.rs     # the command-line tier: CliCase/Out, every option and variant (s7)
tests/data/values.yaml       # a production tenants values file, verbatim (282 KB)
tests/corpus_validation.rs   # asserts every case (classic + engine + write)
tests/corpus_cli.rs          # runs every command-line case through the binary
benches/corpus_bench.rs      # times the same cases + scale variants + the binary
```

`tests/corpus/` is a subdirectory, so Cargo does **not** compile it as its own
test crate. It is pulled into its consumers with `#[path = "…/corpus/mod.rs"]
mod corpus;`; Rust resolves the nested `mod docs;` relative to `mod.rs`'s real
location, so the same files compile unchanged in either crate. The data
tables depend only on `std` and their own types, keeping them
consumer-agnostic; the command-line tier's check functions may call the
`yqr` library, which every consumer links. `tests/data/` is excluded from
the crates.io package (`yqr-m004` s6): the fixture is 282 KB and no
published crate runs its tests.

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
- **Command-line cases** (`CliCase`, `tests/corpus/cli.rs`) run the compiled
  binary. A case names its document (`Doc::None`, a fixed `Doc::Static`, or
  the generated shape at a size, `Doc::Tenants(n)`), how it reaches the
  binary (`Feed::Stdin` pipes it; `Feed::File` writes it to a scratch file
  that replaces every `@doc` token in the arguments — `@invalid`, `@dup` and
  `@missing` name the fixtures a `validate` case needs), the arguments, and
  the contract: exit status, stdout, stderr, and, for `--in-place` and
  refused edits, what the file holds afterwards. Each stream is an `Out`:
  `Exact`, `Input` (the document byte for byte), `Rewrites` (the document
  with only the named edits, as in the write tier), `Empty`, `Contains`,
  `Lines(n)`, or `Satisfies(fn)` for a check that needs more than a
  substring, such as re-parsing normalized output and counting tenants.
  This is the only tier that covers argument parsing, the stdin / `-` /
  file variants, `-i` on a real file, `validate`, help and version, and the
  exit-code contract end to end. Classic `Expect` gains `Count(n)` for
  iterations over a document too large to list.

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
| Values corpus (s7) | the production file through the classic pipeline (scalars, alias and merge resolution, `to_entries` over 355 tenants, the error taxonomy); the shape through the engine (identity, own bytes beside merged values, typed fallback, comment and key selectors, indent-preserving projections) and the write tier (every mutating form, the merged-key refusal and its remedy, the reorder and comment refusals) |
| Command line (s7) | `-h`/`--help`, `validate --help`, `-V`/`--version`; every usage error (no arguments, unknown and removed flags, `help` as a filter, a flag before `validate`, `validate` without files or with `-` twice); reads from a file, stdin and `-`, `--` before the filter, a flag after the positionals; `-r`/`--raw-output`, `-N`/`--normalize`, `-rN` and either order; every selector and the `foot_comment` refusal; `--normalize` refusing a selector; `-i`/`--in-place` for every mutating form, refused with stdin, `-`, a read-only filter, and an edit the engine refuses, each leaving the file unchanged; `validate` with `--strict` before or after the files, stdin, a file and stdin, an invalid or missing file among valid ones, a duplicate key by default and under `--strict`; the shape at 1000 tenants on every path and the alias-budget refusal at 1100; the production file on every path |

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

Three groups come from the values corpus (s7): `corpus/values`
(`classic_all`, `classic_identity` over the production file, byte
throughput), `corpus/scale_tenants` (`engine_identity`,
`classic_identity`, `merged_read`, `write` and `validate_strict` on the
shape at 100 / 400 / 1000 tenants, byte throughput — the read, write and
validate cost of one document side by side, which is how `yqr-b027`
stays fixed), and `corpus/cli` (the compiled binary end to end over the
shape at 400 tenants and the production file: process start, argument
parsing, the read, the output; noisier, and the only measurement of what
a user waits for).

Only the `eval` bench target is tracked over time on `main` (`yqr-m001`), so
these groups are a local regression check, not a dashboard series.

## 6. Extending

Add a `Case` to `classic_cases()`, an `EngineCase` to `engine_cases()`, or a
`WriteCase` to `write_cases()`, with a unique `category/name` id. The
validation suite asserts it and the benchmark times it automatically. Add new
documents to `docs.rs`; keep them verbatim so the fidelity engines exercise
genuine formatting. Cases on the values corpus go in the matching table in
`values.rs`, which the three functions above append.

A command-line case is a `CliCase` in `cli_cases()` with an id of the form
`cli/<group>/<name>`; the runner has one test per group and refuses an id
whose group it does not know, so a case can never be added and silently
skipped. Prefer `Doc::Tenants(n)` over a fixed document — the shape has
every feature of the production file at any size — and state a write's
contract as `Rewrites`, never as the whole expected file.

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

## 7. The values corpus and the command-line tier (added 2026-09-02)

`tests/data/values.yaml` is a production Helm-style values file for a
multi-tenant deployment, verbatim: 282 KB, 7 889 lines, 355 tenants under
`argo.tenants`, 23 anchors, 923 aliases, 281 `<<` merges, 259 flow
collections, 3 616 double-quoted scalars, 508 blank lines and 35
comments. It is the file that found `yqr-b025`, and the shape most users of
a values file actually have: a few anchored default blocks, then hundreds
of entries that merge them and override a few keys each. `values.rs`
embeds it with `include_str!` and pins what is in it (`VALUES_TENANTS`,
first and last tenant).

**`tenants(n)`** builds that shape from a template at any size: one
anchored operations block per eight tenants under `argo.global.opsDefaults`,
merged into each tenant's `ops` with `<<: *oK`; flow values, double-quoted
strings, an inline comment on every `editorDomain`, a head comment on the
first tenant of each block, a section comment above each block that no
path addresses, blank lines between tenants. The alias-to-anchor ratio
stays under 8 at every `n`, so the default byte-preserving path reads the
generated document today while the production file waits on `yqr-f026`;
the parser's absolute alias budget (1 024 expansions) is the ceiling, and
the corpus pins the refusal at 1 100 tenants on every path. Deterministic
layout: tenant `i` is `t{i}`, its hosts `host-{i}.example.invalid`, its
block `o{i / 8}`, its block's language rotating `de`, `fr`, `it`, `en`.
`TENANTS_40` and `TENANTS_1000` are the two sizes the tables use.

**Scaling.** A case is written once against the shape and holds at any
size; the same generator feeds the tests at 40 and 1000 tenants and the
benchmark at 100, 400 and 1000. Two limits on the shipped pin are part of
the record rather than worked around: the production file is refused on
the default path (`cli/values/*-on-this-pin`, `values_file_validates_once_the_engine_reads_it`,
flipping with `yqr-f026`), and the alias budget caps the merge shape at
1 023 tenants. The first thing the scale cases found was `yqr-b027`:
`validate` was quadratic in document size, invisible on every earlier
test input.

**Pins worth knowing.** A merged-in value has no bytes of its own at its
path, so the engine renders it from the typed view (`fr`, unquoted, where
the definition says `"fr"`); a mapping that contains a merge renders the
same way; an alias to a mapping prints the anchor's bytes with their
indentation; a head comment above a block-valued entry reads as absent and
its write is refused; an assignment at the anchor definition is the
`b020` remedy and works on noyalib 0.0.28 (`yqr-f026` §3 for 0.0.29).

## 8. Acceptance criteria

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
- [x] Every command-line option and variant has a case in the command-line
      tier, on the generated shape or the production file (s7).
- [x] The values corpus scales: one generator feeds the tests at 40 and 1000
      tenants and the benchmark at 100, 400 and 1000; the two limits on the
      shipped pin are pinned, not worked around.
