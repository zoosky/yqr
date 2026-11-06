# Feature f048 — Serializable diagnostics and filter-error spans

**Status:** Draft
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

## Acceptance criteria

- [ ] With `--features serde`, `serde_json::to_string` of a
      `Vec<Diagnostic>` carries code, message, line/column, and byte
      offsets; a unit test pins the shape.
- [ ] A filter parse error exposes its span through the lib API, and a
      test shows the span slices the offending token out of the filter
      string.
- [ ] Default-feature builds do not depend on serde.
- [ ] The CLI suite passes byte-identical.
