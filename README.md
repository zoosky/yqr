# yqr

[![Benchmarks](https://img.shields.io/badge/benchmarks-live%20dashboard-blue?logo=rust&logoColor=white)](https://zoosky.github.io/yqr/dev/bench/)

`yqr` ("YAML query in Rust") is a jq-style Swiss Army knife for **YAML**.
It reads a YAML document from a file or
stdin, applies a jq-like filter expression, and emits the result(s) as YAML (or
raw text).

It operates natively on YAML via the
[`rust-yaml`](https://crates.io/crates/rust-yaml) parser — no lossy round trip
through JSON — and uses [`clap`](https://crates.io/crates/clap) for its CLI.

## Install / build

Requires the Rust **1.96** toolchain (pinned via `rust-toolchain.toml`).

```sh
cargo build --release
# binary at target/release/yqr
```

## Usage

```sh
yqr [OPTIONS] <FILTER> [FILE]

Arguments:
  <FILTER>  The jq-style filter to apply (e.g. '.foo.bar', '.items[]')
  [FILE]    Input YAML file; reads stdin when omitted or '-'

Options:
  -r, --raw-output   Emit string results without YAML quoting
  -h, --help         Print help
  -V, --version      Print version
```

### Examples

```sh
# Field access
echo 'name: yqr
version: 1' | yqr .name
# => yqr

# Nested access + array indexing
echo 'authors:
  - name: ada
  - name: linus' | yqr -r '.authors[0].name'
# => ada

# Negative indexing (from the end)
echo 'tags: [cli, yaml]' | yqr -r '.tags[-1]'
# => yaml

# Iterate a collection (one result per line)
echo 'tags: [a, b, c]' | yqr -r '.tags[]'
# => a
#    b
#    c

# Pipe composition
echo 'a: {b: {c: 42}}' | yqr '.a | .b | .c'
# => 42

# Optional `?` suppresses errors
echo 'name: yqr' | yqr '.name[]?'   # prints nothing, exits 0
```

## Supported filters (M0)

| Filter         | Meaning                                             |
|----------------|-----------------------------------------------------|
| `.`            | Identity                                            |
| `.foo`         | Field access (`.["foo"]` for non-bareword keys)     |
| `.a.b`         | Nested field access                                  |
| `.[n]`         | Array index (`.[-1]` counts from the end)           |
| `.[]`          | Iterate sequence elements / mapping values          |
| `a \| b`       | Pipe                                                |
| `f?`           | Suppress errors from `f`                            |

Planned: object/array construction, builtins (`length`, `keys`, `select`,
`map`, …), arithmetic, multi-document/slurp mode, and more. See the spec.

## Byte-preserving reads (`--engine`)

By default yqr re-serializes results, which normalizes formatting (comments,
quoting, indentation are lost). With a **fidelity engine** selected, untouched
nodes are emitted as their **original source bytes** instead. Two engines are
available, each behind its own off-by-default feature:

- `--engine rust-yaml` — our rust-yaml fork's source-preserving
  `RoundTripDocument` (feature `backend-rust-yaml`). Mapping keys keep their
  full typed value, so nothing collides and special-character keys are
  addressable.
- `--engine noyalib` — noyalib's lossless CST (feature `backend-noyalib`).

```bash
# Build with an engine backend (off by default to keep the build minimal)
cargo build --release --features backend-rust-yaml

# Identity reproduces the file byte-for-byte -- comments, blank lines,
# quoting, block scalars, CRLF, BOM, and multi-document streams survive
yqr --engine rust-yaml '.' config.yaml | diff config.yaml -   # no diff

# Projections keep the original spelling
echo "zip: 007" | yqr --engine rust-yaml '.zip'      # => 007   (not 7)
echo "s: 'hi'"  | yqr --engine rust-yaml '.s'        # => 'hi'  (quotes kept)
```

Results that are computed rather than selected (and nodes an engine cannot
address faithfully — entries merged in via `<<`, alias references) fall back to
the regular typed rendering. Multi-document inputs run the filter against every
document. `-r` keeps its usual meaning and prints string *values*.

Engine-mode notes:

- Projected nested **block collections** are emitted at their original
  indentation (the slice is extended to the line start), so the output is
  uniformly indented and re-parses to the selected value.
- **Empty input** produces no output in engine mode (byte-identity with the
  empty file), where the default pipeline prints `null`.
- With `--engine rust-yaml`, mapping keys keep their **full typed value**, so
  distinct keys never collide, special-character keys (`a.b`) are addressable,
  and duplicate keys emit the last occurrence's real bytes. (An anchored
  scalar projects its value without the `&anchor` label.) Current limitation: a
  stream ending in a `...` document-end marker **after a block collection** is
  rejected (a fork parser bug being fixed upstream).
- With `--engine noyalib` (requires noyalib >= 0.0.13), the value model has
  **string-only mapping keys**: non-string keys (`true:`, `8080:`) are matched
  by spelling; distinct keys that collide after string conversion (`1` and
  `"1"`) are refused with an error. Its parser is also stricter in a few corners
  (e.g. classic-Mac CR-only line endings are rejected). Duplicate keys resolve
  last-wins and emit the last occurrence's real bytes (fixed in noyalib 0.0.13).

## Using yqr in Kubernetes

Install paths and recipes for running yqr against `kubectl` output or baking
it into a container image: [`docs/content/kubernetes.html`](docs/content/kubernetes.html).

## Architecture

```
filter ──▶ lexer ──▶ parser ──▶ Ast ──▶ evaluator ──▶ Value(s) ──▶ YAML
YAML   ──▶ rust_yaml::Yaml::load_str ──▶ Value ──┘
```

| Module          | Responsibility                          |
|-----------------|-----------------------------------------|
| `src/lexer.rs`  | Filter string → tokens                  |
| `src/parser.rs` | Tokens → `Ast`                          |
| `src/ast.rs`    | Filter AST node definitions             |
| `src/eval.rs`   | `Ast` × `Value` → stream of `Value`     |
| `src/cli.rs`    | `clap` argument parsing                 |
| `src/lib.rs`    | Public API (`eval_str`, `render`)       |
| `src/main.rs`   | Binary entry + exit-code mapping        |

## Testing

```sh
cargo test            # unit + integration + CLI tests
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

- **Unit tests** live alongside each module.
- **`tests/integration.rs`** exercises the public library API end-to-end.
- **`tests/cli.rs`** runs the compiled binary against piped input.

## Benchmarks

Criterion benchmarks live in `benches/` (`cargo bench --bench eval`). Every push
to `main` runs them in CI and publishes the results to a tracked history:

**[Live benchmark dashboard](https://zoosky.github.io/yqr/dev/bench/)** — performance over time, with alerts on >30% regressions.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
