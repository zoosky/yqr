# yqr

`yqr` ("YAML query in Rust") is a [jq](https://jqlang.github.io/jq/)-style
command-line processor for **YAML**. It reads a YAML document from a file or
stdin, applies a jq-like filter expression, and emits the result(s) as YAML (or
raw text).

It operates natively on YAML via the
[`rust-yaml`](https://crates.io/crates/rust-yaml) parser — no lossy round trip
through JSON — and uses [`clap`](https://crates.io/crates/clap) for its CLI.

> **Status:** foundation milestone (M0). The core path/pipe subset of jq is
> implemented. See [`specs/features/0001-yaml-jq-clone.md`](specs/features/0001-yaml-jq-clone.md)
> for the full plan and roadmap.

## Install / build

Requires the Rust **1.96** toolchain (pinned via `rust-toolchain.toml`).

```sh
cargo build --release
# binary at target/release/yqr
```

## Usage

```
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

## License

Licensed under either of Apache-2.0 or MIT at your option.
