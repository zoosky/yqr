# Feature f048 — Serializable diagnostics and filter-error spans

**Status:** Done (2026-11-07)
**Epic:** Library surface and wasm (`f046`–`f050`)
**Owner:** yqr maintainers
**Related:** `yqr-f049` (the wasm bindings that carry these across the
JS boundary), accentcms admin editor integration (tracked there as
`f430`)

## Why

An editor renders diagnostics as squiggles and gutter markers, not as
terminal text. `validate::Diagnostic` already carries a typed `Code`
and positions, and `validate/render.rs` renders them compiler-style —
but a host that wants the structure must scrape that text today. The
same holds for the filter language: when a half-typed filter fails to
parse, `YqrError` explains it in prose, and a query input box cannot
underline the offending token without the span.

## What

1. **`serde::Serialize` on the diagnostic types** — `Diagnostic`,
   `Code`, and the span/position types it carries — behind an optional
   `serde` cargo feature, so the default graph does not grow. Derive
   `Serialize` only; these types are outputs.
2. **Both coordinate systems.** A diagnostic names the position as line
   and column today; the serialized form also carries byte offsets, so
   a CodeMirror-style editor does not re-scan the document to place a
   marker.
3. **Filter parse errors carry a span.** The lexer and parser know
   where they stopped; `YqrError`'s parse variants expose the byte
   range into the filter string, and the serialized form carries it.
   The rendered prose message is unchanged.
4. **Stability note in the docs.** The serialized shape becomes an
   interface the moment an editor ships against it: document the fields
   on the types themselves and state that removals are breaking.

## Out of scope

- `Deserialize` on diagnostics (nothing constructs them from JSON).
- Changing any rendered CLI message. The text output is pinned by the
  CLI suite and stays byte-identical.

## As landed (2026-11-07)

- `Diagnostic` gains a public `offset` field (byte offset of the same
  location `position` names); every producer sets it from the byte
  anchor it already had, and a test asserts the two coordinate systems
  agree on every finding.
- `YqrError::Lex`/`Parse` now carry a `FilterError { message, span }`.
  The convenience constructors (`parse`, `lex`) are unchanged and
  spanless; `parse_at`/`lex_at` attach a byte range, and
  `YqrError::span()` reads it back. The lexer tracks token byte spans
  (`lex_spanned`, crate-private), the parser keeps them parallel to its
  token stream, and every token-anchored error site uses them. Refusals
  about a filter's *meaning* (a mutation in a read-only pipeline, an
  unwritable left-hand side) stay spanless by design.
- `Code` serializes as its stable string (`"Y101"`), not the Rust
  variant name. Serialized shapes are documented on the types as an
  interface: fields may be added, removals are breaking.
- The `serde` feature is additive and off by default. Note: the
  *default* graph already carries serde transitively (`schema` pulls
  `serde_json`, and noyalib pulls `serde_core`); what this feature
  keeps optional is the derive stack on yqr's own types, and the
  minimal (`--no-default-features`) graph stays serde-free — the CI
  dependency guard now refuses `serde` there too.

## Acceptance criteria

- [x] With `--features serde`, `serde_json::to_string` of a
      `Vec<Diagnostic>` carries code, message, line/column, and byte
      offsets; a unit test pins the shape.
- [x] A filter parse error exposes its span through the lib API, and a
      test shows the span slices the offending token out of the filter
      string (plus an empty end-of-source span for a filter that ends
      too early).
- [x] Default-feature builds do not enable the `serde` feature; the
      minimal lib graph carries no serde and the CI guard enforces it
      (see the transitive-dependency note above).
- [x] The CLI suite passes byte-identical.
