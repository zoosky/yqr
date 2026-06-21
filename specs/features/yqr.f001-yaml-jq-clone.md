# Feature 001 — `yqr`: a jq clone for YAML

Status: **In progress** (foundation landed)
Owner: yqr maintainers
Last updated: 2026-06-21

## 1. Summary

`yqr` ("YAML query in Rust") is a command-line tool that applies
[jq](https://jqlang.github.io/jq/)-style filter expressions to YAML documents.
It reads YAML from a file or stdin, evaluates a filter against the parsed
document, and emits the resulting value(s) back as YAML (or raw text).

The goal is feature parity with the most commonly used subset of jq, operating
natively on YAML so that comments-free round-tripping, key ordering, and YAML
scalar types are preserved as faithfully as the underlying parser allows.

## 2. Goals

- Familiar jq surface syntax so existing jq muscle memory transfers.
- Native YAML in / YAML out (no lossy detour through JSON).
- Streaming, multi-value results just like jq (a filter can yield 0..N values).
- A clean, layered architecture (lexer → parser → AST → evaluator) that is
  cheap to extend filter-by-filter.
- Strong test infrastructure from day one: unit tests per layer, end-to-end
  library tests, and CLI smoke tests, all runnable with `cargo test`.

## 3. Non-goals (for the initial milestones)

- Full jq language coverage (reduce/foreach, path expressions, `@base64` and
  the rest of the builtin zoo, modules/imports, SQL-style builtins).
- Comment-preserving round trips (the parser supports it; wiring it through is
  a later milestone).
- Performance tuning / zero-copy evaluation.

## 4. Dependencies & toolchain

- Language: Rust, edition 2024, targeting the **1.96** toolchain
  (`rust-version` pinned in `Cargo.toml`; `rust-toolchain.toml` requests 1.96).
- YAML engine: [`rust-yaml`](https://crates.io/crates/rust-yaml) `1.1.0`.
  - Entry point `rust_yaml::Yaml` with `load_str`/`load_all_str` and
    `dump_str`/`dump_all_str`.
  - Document model `rust_yaml::Value`:
    `Null | Bool(bool) | Int(i64) | Float(f64) | String(String) |
     Sequence(Vec<Value>) | Mapping(IndexMap<Value, Value>)`.
- CLI: [`clap`](https://crates.io/crates/clap) `4.6` with the `derive` feature.

## 5. CLI surface

```
yqr [OPTIONS] <FILTER> [FILE]

Arguments:
  <FILTER>  The jq-style filter to apply (e.g. '.foo.bar', '.items[]')
  [FILE]    Input YAML file; reads stdin when omitted or '-'

Options:
  -r, --raw-output   Emit string results without YAML quoting
  -s, --slurp        Read all input documents into a single sequence
  -e, --exit-status  Set exit code from the last output (jq semantics)
  -h, --help         Print help
  -V, --version      Print version
```

Exit codes follow jq where practical: `0` success, `2` usage error,
`3` compile (parse) error, `5` runtime error. With `-e`, `1` means the last
output was `null`/`false` and `4` means no output was produced.

## 6. Architecture

```
            ┌────────┐   tokens   ┌────────┐   AST    ┌──────────┐
  filter ──▶│ lexer  ├───────────▶│ parser ├─────────▶│   Ast    │
            └────────┘            └────────┘          └────┬─────┘
                                                           │
  YAML ──▶ rust_yaml::Yaml::load_str ──▶ Value ──▶  ┌──────▼─────┐ ──▶ Value(s)
                                                    │  evaluator │
                                                    └────────────┘ ──▶ dump_str ──▶ YAML
```

Source layout:

| File             | Responsibility                                            |
|------------------|-----------------------------------------------------------|
| `src/main.rs`    | Thin binary entry; maps results/errors to exit codes.     |
| `src/cli.rs`     | `clap` argument definitions.                               |
| `src/lib.rs`     | Public API (`run`, `eval_str`) + re-exports.              |
| `src/error.rs`   | `YqrError` / `Result` and exit-code mapping.              |
| `src/lexer.rs`   | Filter source → `Token` stream.                           |
| `src/ast.rs`     | `Ast` filter node definitions.                            |
| `src/parser.rs`  | Recursive-descent `Token`s → `Ast`.                       |
| `src/eval.rs`    | `Ast` × `Value` → stream of `Value` (the engine).        |

Evaluation contract: every filter maps one input `Value` to an ordered
`Vec<Value>` (the output stream). `|` (pipe) feeds each left output into the
right filter and concatenates; iteration (`.[]`) explodes a collection into the
stream; `?` swallows errors from its operand, yielding an empty stream instead.

## 7. Milestones

### M0 — Foundation (this change) ✅
- Project scaffold, dependencies, toolchain pin.
- Lexer + parser + evaluator for the core path/pipe subset:
  - Identity `.`
  - Field access `.foo`, `.foo.bar`, `.["key"]`
  - Array index `.[0]`, `.[-1]` (negative from end), out-of-range → `null`
  - Iteration `.[]` over sequences and mapping values
  - Pipe `a | b`
  - Optional `f?` error suppression
- CLI with `--raw-output`, file/stdin input, jq-style exit codes.
- Test infrastructure: per-module unit tests, `tests/integration.rs`
  (library end-to-end), `tests/cli.rs` (binary smoke tests), CI workflow.

### M1 — Construction & literals
- Object `{a: .x, b: .y}` and array `[ .a, .b ]` construction.
- Scalar literals (numbers, strings, `true`/`false`/`null`) and string
  interpolation `"\(.name)"`.
- Comma operator `.a, .b` (stream concatenation).

### M2 — Builtins & arithmetic
- `length`, `keys`, `values`, `has`, `type`, `select(f)`, `map(f)`,
  `to_entries`/`from_entries`, `add`, `min`/`max`, `sort`/`sort_by`.
- Arithmetic (`+ - * / %`) and comparisons; boolean `and`/`or`/`not`.
- Alternative operator `//`.

### M3 — Multi-document & emission control
- `--slurp`, multi-document input/output via `load_all_str`/`dump_all_str`.
- Comment-preserving mode (`load_str_with_comments`).
- JSON output mode for interop.

### M4 — Advanced jq
- Variable bindings `... as $x | ...`, `reduce`, `foreach`.
- Path expressions, `paths`, `getpath`/`setpath`, assignment (`=`, `|=`).
- Recursive descent `..`.

## 8. Testing strategy

- **Unit tests** colocated in each module (`#[cfg(test)]`): lexer token
  streams, parser AST shapes, evaluator behavior on edge cases.
- **Integration tests** (`tests/integration.rs`): drive the public
  `yqr::eval_str` API across realistic YAML + filter pairs, asserting on the
  emitted YAML.
- **CLI tests** (`tests/cli.rs`): invoke the compiled binary via
  `CARGO_BIN_EXE_yqr`, piping YAML on stdin and checking stdout / exit codes.
  No extra dev-dependencies required.
- **CI** (`.github/workflows/ci.yml`): `cargo fmt --check`, `cargo clippy
  -D warnings`, `cargo test` on the pinned toolchain.

## 9. Open questions

- How closely should YAML scalar typing track jq's JSON number model
  (jq has a single number type; YAML/`rust-yaml` distinguish `Int`/`Float`)?
- Should `--raw-output` apply to nested strings or only top-level scalar
  results? (Current: top-level string results only, matching jq.)
- Comment preservation interaction with filters that synthesize new nodes.
