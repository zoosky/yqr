# yqr

[![Benchmarks](https://img.shields.io/badge/benchmarks-live%20dashboard-blue?logo=rust&logoColor=white)](https://zoosky.github.io/yqr/dev/bench/)

`yqr` ("YAML query in Rust") is a **fidelity-first**, jq-style command-line tool
for **YAML**. It queries *and* edits YAML while preserving every byte it was not
asked to change — comments, quoting, indentation, key order, and line endings all
survive.

- **Byte-exact reads, by default.** `yqr '.' file.yaml` reproduces the input
  exactly — no flag, no reflow.
- **Surgical edits.** `yqr -i '.spec.replicas = 5' deploy.yaml` rewrites only the
  bytes the filter targets, or refuses — clean diffs, guaranteed. jq is JSON-only
  and cannot preserve YAML formatting at all; yq edits in place but its docs admit
  comment and whitespace issues. yqr changes nothing but the edit site, or errors.
- **Native YAML, no JSON round-trip.** Parsing and emission run through the
  [`noyalib`](https://crates.io/crates/noyalib) engine — the lossless CST behind
  both the default read path and the `--normalize` pipeline; the CLI uses
  [`clap`](https://crates.io/crates/clap).

## Install / build

Install the published crate from crates.io:

```sh
cargo install yqr
# binary at ~/.cargo/bin/yqr
```

Or build from a source checkout (requires the Rust **1.97.1** toolchain, pinned
via `rust-toolchain.toml`):

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
  -r, --raw-output    Emit string results without YAML quoting
  -N, --normalize     Re-serialize output (drop comments, canonicalize scalars)
  -i, --in-place      Edit the input file in place (mutating filters only)
  -h, --help          Print help
  -V, --version       Print version
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

## Byte-preserving reads (default) and `--normalize`

**yqr preserves formatting by default.** Untouched nodes are emitted as their
**original source bytes**, so the identity filter reproduces the input exactly —
comments, quoting, indentation, and line endings all survive. Pass
**`--normalize`** (`-N`) to re-serialize the output instead, which canonicalizes
scalars and drops comments.

Byte-preserving reads are powered by [`noyalib`](https://crates.io/crates/noyalib)'s
lossless CST — yqr's one and only YAML engine.

```bash
# Identity reproduces the file byte-for-byte -- comments, blank lines,
# quoting, block scalars, CRLF, BOM, and multi-document streams survive
yqr '.' config.yaml | diff config.yaml -   # no diff (no flag needed)

# Projections keep the original spelling
echo "zip: 007" | yqr '.zip'      # => 007   (not 7)
echo "s: 'hi'"  | yqr '.s'        # => 'hi'  (quotes kept)

# --normalize re-serializes (lossy: drops comments, canonicalizes scalars)
echo "zip: 007" | yqr --normalize '.zip'   # => 7    (re-typed)
yqr --normalize '.' config.yaml            # comments dropped, scalars canonicalized
```

Results that are computed rather than selected (and nodes an engine cannot
address faithfully — entries merged in via `<<`, alias references) fall back to
the regular typed rendering. Multi-document inputs run the filter against every
document. `-r` keeps its usual meaning and prints string *values*.

Fidelity notes:

- Projected nested **block collections** are emitted at their original
  indentation (the slice is extended to the line start), so the output is
  uniformly indented and re-parses to the selected value.
- **Empty input** produces no output in the default (byte-preserving) mode
  (byte-identity with the empty file), where `--normalize` prints `null`.
- The noyalib backend's value model has **string-only mapping keys**: non-string
  keys (`true:`, `8080:`) are matched by spelling; distinct keys that collide
  after string conversion (`1` and `"1"`) are refused with an error. Duplicate
  keys resolve last-wins and emit the last occurrence's real bytes. Keep-chomped
  (`|+`) block scalars retain their kept trailing blank lines, alias references
  project the anchor's real bytes, block-collection spans start at their first
  line's indent, and classic-Mac CR-only line endings are accepted.

## Surgical edits (`=`, `+=`, `del`, `-i`)

yqr can also **edit** YAML, not just read it — and it changes only the bytes the
filter targets, leaving every other byte (comments, indentation, quoting, key
order) untouched, or refuses. Edits always run through the fidelity engine, so a
mutating filter is byte-exact except at the edit site.

The mutation surface:

| Filter                    | Meaning                                              |
|---------------------------|------------------------------------------------------|
| `<path> = <value>`        | Replace the scalar at `path` (style-matched quoting) |
| `<path>.<newkey> = <value>` | Add a new mapping entry under an existing mapping  |
| `<path> += <value>`       | Append an item to the block sequence at `path`       |
| `del(<path>)`             | Remove the block entry at `path` (single- or multi-line) |

`<value>` is a scalar literal (`5`, `1.5`, `"web"`, `true`, `false`, `null`) or a
`.`-rooted path that copies the value found at another location.

```bash
# Replace a value; the comment and every other line are preserved verbatim
echo 'spec:
  replicas: 3   # keep me
  image: web' | yqr '.spec.replicas = 5'
# => spec:
#      replicas: 5   # keep me
#      image: web

# Append to a block sequence at the right indent
yqr '.spec.ports += 9090' deploy.yaml

# Add a new key, delete an entry (a nested/multi-line block closes up cleanly)
yqr '.metadata.env = "prod"' deploy.yaml
yqr 'del(.metadata.labels)' deploy.yaml
yqr 'del(.spec.template)' deploy.yaml

# Edit the file in place (rewritten atomically: temp file + rename)
yqr -i '.spec.replicas = 5' deploy.yaml
git diff deploy.yaml   # touches only that one line
```

Guarantees and limits:

- **Structural integrity.** An edit whose result would re-parse to a different
  structure is **refused** (exit 5) rather than emitted; under `-i` the file is
  left unchanged.
- **No-match is a no-op.** A filter that matches no node succeeds and leaves the
  document unchanged (jq/yq semantics), so `del(.x)` across a batch of files does
  not fail the ones that lack `.x`.
- **`-i` needs a file.** Using `--in-place` with stdin, or with a read-only
  filter, is an error (diagnosed before any input is read). Writes are atomic
  (temp file + `fsync` + rename) and edit *through* a symlink to the real file;
  the original mode is preserved. Owner/group, SELinux context, ACLs, extended
  attributes, and hardlinks are **not** carried across the replace — the same
  temp-file+rename tradeoff `sed -i` makes.
- **Multi-document.** The edit applies to each document whose path resolves; the
  others are emitted byte-identically.
- **Scalar RHS only.** `=`, `+=`, and new-key values are scalars (number, string,
  bool, null) or a path copying a scalar; a collection RHS is refused.
- **Structural delete.** `del` removes multi-line and nested block entries too,
  not just single-line ones; it closes up the entry's lines and leaves every
  surviving byte identical. Deleting the *only* entry of a block (which would
  empty it) or an item of a *flow* collection (`[a, b]`) is refused with a clear
  message.
- **Unsupported operations.** Computed updates (`|=`), key rename, and sequence
  reorder / comment edits each fail with a clear message.

## Query filters

| Filter    | Meaning                                          |
|-----------|--------------------------------------------------|
| `.`       | Identity                                         |
| `.foo`    | Field access (`.["foo"]` for non-bareword keys)  |
| `.a.b`    | Nested field access                              |
| `.[n]`    | Array index (`.[-1]` counts from the end)        |
| `.[]`     | Iterate sequence elements / mapping values       |
| `a \| b`  | Pipe                                             |
| `f?`      | Suppress errors from `f`                         |

Planned: object/array construction, builtins (`length`, `keys`, `select`,
`map`, …), arithmetic, multi-document/slurp mode, and more. See the spec.

## Website

The project website — install paths, recipes for running yqr against
`kubectl` output and other YAML (CI configs, Compose files, Ansible
playbooks, OpenAPI specs, alerting rules, app config), plus the full
browsable spec tree — lives at
**[zoosky.github.io/yqr](https://zoosky.github.io/yqr/)**.

It is an [Accent CMS](https://github.com/AccentCMS/accent) site built from
[`docs/`](docs/) with [`specs/`](specs/) mounted at `/specs`, deployed by the
`Website` workflow on every push to `main`. Local preview:
`cd docs && accent serve`.

## Architecture

```
filter ──▶ lexer ──▶ parser ──▶ Ast ──▶ evaluator ──▶ Value(s) ──▶ YAML
YAML   ──▶ noyalib::from_str ──▶ Value ──┘
```

| Module          | Responsibility                                    |
|-----------------|---------------------------------------------------|
| `src/lexer.rs`  | Filter string → tokens                            |
| `src/parser.rs` | Tokens → `Ast`                                    |
| `src/ast.rs`    | Filter AST node definitions                       |
| `src/eval.rs`   | `Ast` × `Value` → stream of `Value`               |
| `src/value.rs`  | yqr's `Value` model (converts to/from `noyalib`)  |
| `src/fidelity/` | Byte-preserving read engine (default reads) + write tier (`src/fidelity/write.rs`) |
| `src/error.rs`  | `YqrError` + jq-style exit-code mapping            |
| `src/cli.rs`    | `clap` argument parsing                           |
| `src/lib.rs`    | Public API (`eval_str`, `render`)                 |
| `src/main.rs`   | Binary entry + exit-code mapping                  |

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
