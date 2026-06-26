# Research r001 — jq Feature-Gap Analysis

**Status:** Draft
**Owner:** yqr maintainers
**Last updated:** 2026-06-26
**Baseline analyzed:** commit `e234b55` (v0.1.1), branch `main`

## 1. Purpose

`yqr` aims for "feature parity with the most commonly used subset of jq,
operating natively on YAML" (see `yqr.f001`). This document inventories the gap
between the **current implementation** and the **full jq language** (jq 1.7
manual as reference), so the roadmap can be prioritized by user value rather
than by what happens to be easy.

The findings are grounded in direct inspection of the source at the baseline
commit — not the roadmap's aspirations. Every "implemented" claim below maps to
an actual `Ast` variant, evaluator arm, or CLI flag.

## 2. Method

- Read the evaluator (`src/eval.rs`) to enumerate the handled `Ast` variants.
- Read the parser/lexer (`src/parser.rs`, `src/lexer.rs`) for the accepted
  grammar.
- Read `src/cli.rs` and `src/lib.rs` for CLI flags and the YAML load/dump path.
- Cross-referenced against the jq 1.7 language manual feature categories.

## 3. What is implemented today

The compiled filter language has exactly **six** `Ast` node kinds:

| jq construct            | Form(s) supported                         | Notes |
|-------------------------|-------------------------------------------|-------|
| Identity                | `.`                                       | |
| Field access            | `.foo`, `.a.b`, `.["key"]`                | string keys only |
| Array index             | `.[n]`, `.[-n]`                           | negative from end; OOB → `null` |
| Iteration               | `.[]`                                     | sequences and mapping values |
| Pipe                    | `a \| b`                                  | |
| Error suppression       | `f?`                                      | swallows runtime errors → empty stream |

Runtime / CLI surface:

- **Input:** a single YAML document from a file or stdin (`rust_yaml::Yaml::load_str`).
- **Output:** each result emitted as YAML (`dump_str`); `-r/--raw-output` prints
  top-level string results verbatim.
- **Flags:** `<FILTER>`, `[FILE]`, `-r/--raw-output`, `--version`/`-V`, `--help`.
- **Exit codes:** `0` ok, `2` usage, `3` lex/parse, `5` runtime/io.

This is a faithful but small **path-navigation subset**. It can drill into a
document and fan out collections; it cannot yet *transform*, *compute*, *filter
by predicate*, or *reshape* data.

## 4. Gap inventory

Legend — **Priority**: 🔴 high (blocks everyday jq use) · 🟡 medium · 🟢 low
(niche/advanced). **Maps to**: existing milestone in `yqr.f001` or "new".

### 4.1 Path & navigation

| Feature | Example | Status | Priority | Maps to |
|---------|---------|--------|----------|---------|
| Array/string slices | `.[2:4]`, `.[:3]`, `.[3:]` | ✗ | 🔴 | new |
| Recursive descent | `..` | `.a` everywhere | ✗ | 🟡 | M4 |
| Optional on a path step | `.a?.b`, `.["k"]?` | partial (`f?` only) | 🟡 | M1 |
| Iterate with index keys | `.[ "a", "b" ]` (multi-key) | ✗ | 🟢 | new |
| Negative/forgiving object index | `.["missing"]` → null | ✓ | — | — |

### 4.2 Operators

| Feature | Example | Status | Priority | Maps to |
|---------|---------|--------|----------|---------|
| Comma (stream concat) | `.a, .b` | ✗ | 🔴 | M1 |
| Arithmetic | `+ - * / %` | ✗ | 🔴 | M2 |
| Comparison | `== != < <= > >=` | ✗ | 🔴 | M2 |
| Boolean logic | `and`, `or`, `not` | ✗ | 🔴 | M2 |
| Alternative / default | `//` | ✗ | 🔴 | M2 |
| Assignment family | `=`, `\|=`, `+=`, `-=`, `*=`, `/=`, `%=`, `//=` | ✗ | 🟡 | M4 |
| Destructuring alt | `?//` | ✗ | 🟢 | new |

### 4.3 Value construction & literals

| Feature | Example | Status | Priority | Maps to |
|---------|---------|--------|----------|---------|
| Array construction | `[ .a, .b ]` | ✗ | 🔴 | M1 |
| Object construction | `{a: .x, b}` | ✗ | 🔴 | M1 |
| Computed/`$`-keys | `{(.k): .v}`, `{$id}` | ✗ | 🟡 | M1 |
| Scalar literals | `1`, `"s"`, `true`, `false`, `null` | ✗ | 🔴 | M1 |
| String interpolation | `"\(.name)!"` | ✗ | 🔴 | M1 |
| Format/`@`-strings | `@base64`, `@json`, `@csv`, `@tsv`, `@uri`, `@html`, `@sh`, `@text` | ✗ | 🟡 | new |

### 4.4 Control flow & binding

| Feature | Example | Status | Priority | Maps to |
|---------|---------|--------|----------|---------|
| Conditional | `if c then a else b end`, `elif` | ✗ | 🔴 | M2 |
| try/catch | `try f catch g` | partial (`f?`) | 🟡 | M2 |
| Variable binding | `… as $x \| …`, destructuring `as [$a,$b]` | ✗ | 🟡 | M4 |
| `reduce` | `reduce .[] as $x (0; .+$x)` | ✗ | 🟡 | M4 |
| `foreach` | `foreach .[] as $x (…; …; …)` | ✗ | 🟡 | M4 |
| `label`/`break` | `label $out \| … break $out` | ✗ | 🟢 | new |
| Function defs | `def f(p): …;` and recursion | ✗ | 🟡 | new |

### 4.5 Builtin functions

jq ships ~150 builtins; **zero** are implemented. Grouped by theme:

| Group | Representative builtins | Priority |
|-------|-------------------------|----------|
| Introspection | `length`, `utf8bytelength`, `type`, `keys`, `keys_unsorted`, `values`, `has`, `in`, `contains`, `inside` | 🔴 |
| Selection/transform | `select(f)`, `map(f)`, `map_values(f)`, `recurse`, `empty`, `error`, `paths`, `leaf_paths` | 🔴 |
| Array/agg | `add`, `any`, `all`, `flatten`, `range`, `min`, `max`, `min_by`, `max_by`, `sort`, `sort_by`, `group_by`, `unique`, `unique_by`, `reverse`, `first`, `last`, `nth`, `limit`, `until`, `repeat` | 🔴 |
| Object | `to_entries`, `from_entries`, `with_entries`, `getpath`, `setpath`, `delpaths`, `del`, `paths`, `walk` | 🟡 |
| String | `ascii_downcase/upcase`, `ltrimstr`, `rtrimstr`, `startswith`, `endswith`, `split`, `join`, `explode`, `implode`, `ascii`, `ltrimstr` | 🟡 |
| Numeric/math | `floor`, `ceil`, `round`, `fabs`, `sqrt`, `pow`, `log`, `exp`, `isnan`, `isinfinite`, `infinite`, `nan`, `tonumber`, `tostring` | 🟡 |
| Regex (Oniguruma) | `test`, `match`, `capture`, `scan`, `splits`, `split(re;flags)`, `sub`, `gsub` | 🟡 |
| Date/time | `now`, `strftime`, `strptime`, `mktime`, `gmtime`, `localtime`, `date`, `dateadd` | 🟢 |
| JSON interop | `tojson`, `fromjson` | 🟡 |
| SQL-style | `INDEX`, `GROUP_BY`, `UNIQUE_BY`, `IN` | 🟢 |
| I/O / meta | `input`, `inputs`, `debug`, `stderr`, `env`, `$ENV`, `builtins`, `halt`, `halt_error`, `input_line_number`, `$__loc__` | 🟢 |

### 4.6 CLI options (jq parity)

| Option | Meaning | Status | Priority | Maps to |
|--------|---------|--------|----------|---------|
| `-r` | raw output | ✓ | — | — |
| `-s` / `--slurp` | read all inputs into one array | ✗ | 🔴 | M3 |
| `-n` / `--null-input` | run with `null` input | ✗ | 🟡 | M3 |
| `-e` / `--exit-status` | exit code from last output | ✗ | 🟡 | M3 |
| `-S` / `--sort-keys` | sort object keys on output | ✗ | 🟡 | new |
| `--indent N` / `--tab` | output indentation | ✗ | 🟡 | new |
| `-j` / `--join-output` | no newline between outputs | ✗ | 🟢 | new |
| `-f FILE` / `--from-file` | read filter from a file | ✗ | 🟡 | new |
| `--arg`, `--argjson` | inject named string/JSON vars | ✗ | 🟡 | M4 |
| `--slurpfile`, `--rawfile` | inject file contents as vars | ✗ | 🟢 | M4 |
| `--args`, `--jsonargs` | positional `$ARGS.positional` | ✗ | 🟢 | M4 |
| `-R` / `--raw-input` | treat input lines as strings | ✗ | 🟢 | new |
| `--stream` | streaming `[path, leaf]` events | ✗ | 🟢 | new |

## 5. YAML-native gaps (yqr's differentiators)

These are **not** jq features — they are where a YAML-native tool should *beat*
jq. They are also unimplemented today and arguably higher-leverage than chasing
jq's long tail, because they are the reason to pick `yqr` over `jq | yq`.

| Capability | Current state | Priority |
|------------|---------------|----------|
| Multi-document streams (`---`) | only the first document is read (`load_str`) | 🔴 |
| Multi-document output (`dump_all_str`) | single-value emit only | 🔴 |
| Comment preservation | `rust-yaml` exposes `load_str_with_comments` / `CommentedValue`; not wired | 🟡 |
| Anchors/aliases (`&a`/`*a`) round-trip | depends on parser; not validated | 🟡 |
| Tag handling (`!!str`, custom tags) | not surfaced | 🟢 |
| Output style control (block vs flow, indent, quoting) | fixed default emitter | 🟡 |
| Int/Float number model vs jq's single number | `Int`/`Float` distinct — arithmetic semantics undecided (open question in `yqr.f001`) | 🔴 (design) |

## 6. Coverage summary

Counting jq's surface coarsely (operators, construction, control flow, ~150
builtins, ~20 CLI options, path forms):

- **Path navigation:** ~60% of common forms (missing slices, `..`, per-step `?`).
- **Operators:** ~10% (pipe only; no comma/arithmetic/compare/boolean/`//`).
- **Construction & literals:** 0%.
- **Control flow & binding:** ~5% (only `?` as a degenerate `try`).
- **Builtins:** 0 of ~150.
- **CLI parity:** ~1 of ~20 options.

Overall the tool is a **read-only path navigator** — perhaps **10–15%** of
everyday jq usage by feature weight. The single highest-impact missing pieces
are the comma operator, value construction (`[]`/`{}` + literals + `\(...)`),
and the `select`/`map`/`length`/`keys` builtin core — these unlock the
"query *and reshape*" workflows that motivate jq in the first place.

## 7. Recommended prioritization

The roadmap in `yqr.f001` (M1 construction → M2 builtins/arithmetic →
M3 multi-doc → M4 advanced) is sound. Two adjustments fall out of this analysis:

1. **Pull multi-document handling forward.** It is the YAML differentiator,
   it is a small change (`load_all_str` + a `--slurp`/document-stream mode), and
   today's silent "first document only" behavior is a correctness footgun for
   real-world YAML (Kubernetes manifests, Helm output, CI configs).
2. **Pair the comma operator with M1 construction.** `[]`/`{}` are far less
   useful without `,`, and `,` is trivial to add (one `Ast` variant, one
   evaluator arm that concatenates streams). Treat them as one unit.

Suggested near-term feature specs to file (each gets its own `yqr.fNNN`):

- **f002** — Comma operator + value construction (`[]`, `{}`, scalar literals,
  string interpolation). *(M1)*
- **f003** — Core builtins: `length`, `keys`, `values`, `has`, `type`,
  `select`, `map`. *(M2)*
- **f004** — Arithmetic, comparison, boolean, and `//` operators. *(M2)*
- **f005** — Multi-document input/output and `--slurp` / `-n` / `-e`. *(M3,
  pulled forward per §7.1)*

## 8. Open questions

> **Resolved.** These questions were ratified in `yqr-a001` (Fidelity-First,
> Surgical-Edit Model). Summary below; see a001 for rationale.

- **Number model:** **preserve types** — `Int op Int → Int` when exact, `Float`
  only when genuinely fractional; compare/sort by value. Fidelity forbids
  turning `replicas: 3` into `3.0`.
- **Comment preservation × transformation:** preserved via **source spans**, not
  `CommentedValue` strings. Untouched nodes are copied byte-for-byte; deleted
  nodes take their comments; synthesized nodes get none.
- **Regex engine:** "jq-like" on the Rust `regex` crate; explicit errors on
  unsupported constructs (lookaround/backrefs). Not jq-identical.
- **`def` / modules:** local `def` functions eventually in scope; module system
  (`import`/`include`) is a non-goal absent real demand.

## 9. Prioritization update (a001)

`yqr-a001` makes **fidelity the top priority for Cohort B**. This reorders the
near-term plan: the source-preserving **read path (slice-on-emit)** and the
**`yqr .` byte-for-byte round-trip** property come *before* the construction/
builtin features above, since today's `Value`-round-trip silently reformats
files. A new feature spec should capture this:

- **f002** (reprioritized) — Source-preserving read path + round-trip guarantee
  + multi-document/BOM/CRLF fidelity. *(implements a001 §4.1, §2)*
