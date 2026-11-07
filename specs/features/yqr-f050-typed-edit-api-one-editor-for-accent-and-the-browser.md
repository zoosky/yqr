# Feature f050 — A typed edit API: one editor for accentcms and the browser

**Status:** Done (2026-11-07; the scalar-to-collection re-shape is
split out as `yqr-f051`, see the decision log)
**Epic:** Library surface and wasm (`f046`–`f050`)
**Owner:** yqr maintainers
**Related:** `yqr-f006`–`f008` (the write tier this exposes),
`yqr-f049` (the wasm boundary the ops will cross), accentcms `f336`
(the parallel editor this converges with) and accentcms `f430` (the
accent-side adoption)

## The duplication, and which way the transfer runs

accentcms carries a lossless frontmatter editor (`f336`,
`src/content/frontmatter/edit.rs`): typed operations — set, remove,
rename — applied to a `noyalib::cst::Document`, rewriting only the
bytes a change touches. yqr's fidelity write tier is the same machine
over the same engine: `ast::Mutation` (`Assign`, which also creates a
new key and renames through `Target::Key`; `Append`; `Update`;
`Delete`; head-comment targets) applied through
`fidelity::write::apply`, guarded and re-parse-checked, with the
noyalib bug ledger (`b025`–`b036`, upstream `#327`–`#477`) behind it.

Two implementations of surgical YAML editing on one engine will drift:
each fixes its own edge cases against its own tests. The goal is one
implementation, and the question "transfer accent's `f336` into yqr?"
has a measured answer: **no code needs to move — the machinery is
already here.** What yqr lacks is not the editor but the *interface*
accent (and a browser) can call: today `Mutation` is built from parsed
filter text, so a host composing an edit programmatically would have to
render yqr filter syntax — quoting rules included — and re-lex it.
That is the gap this feature closes. accent then deletes its editor's
internals and keeps a thin adapter; the browser editor (`f049`) calls
the same code compiled to wasm. That is the isomorphism.

## What

1. **Paths as data.** A public path type built from typed segments
   (key, index), with no trip through the lexer, convertible into the
   write tier's `Target`. A key containing `.`, `"`, or filter
   metacharacters is just data here — the composition problems of
   textual paths do not exist.
2. **Ops as data.** A public edit-operation type mirroring the write
   tier's mutations: set (create-missing-key included), remove, rename,
   append, plus the head-comment write the tier already has. Each op
   maps onto exactly one guarded mutation; the refusal behavior is the
   write tier's, unchanged.
3. **A change-set entry point.** `apply_all(ops, input) -> Result<String>`
   with the contract accent's editor callers depend on: an empty change
   set returns the input bytes unchanged, and ops apply in order with
   the first refusal reported by op index.
4. **Serializable ops** (behind the `serde` feature, `f048`): the same
   op values cross the wasm boundary as JSON, so the browser editor and
   a server compose edits in one vocabulary.
5. **Port accent's contract tests.** accent's `f336` editor tests
   (`src/content/frontmatter/edit/tests.rs`) encode the guarantees its
   admin save, MCP tools, and CLI depend on — comments, blank lines,
   quoting style, and key order survive; a no-op change set is a no-op
   on disk. Those cases join yqr's corpus as the acceptance floor, so
   the convergence cannot silently weaken what accent ships today.

## Decisions (recorded 2026-11-07, before implementation)

- **Parent creation is the typed API's default.** An editor host that
  sets `menu.visible` means *make it so*; a flag would be one every
  host passes always, which is a default spelled as a flag. The filter
  language is unchanged (`.a.b = 1` on a missing `.a` stays a no-op).
  Implementation: the typed layer resolves the longest existing prefix
  of the path against the document's value view, then applies **one**
  guarded `Assign` that creates the first missing key with the nested
  remainder as its value. A missing level addressed by *index* is
  refused — only mapping levels are created, as in accent.
- **Document creation.** A `set` against an empty document creates it
  (accent's editor opens an empty block as an empty mapping and the
  first set fills it); `remove` and `rename` against an empty document
  are no-ops. The write tier itself still refuses an empty stream —
  the typed layer owns this case.
- **Scalar-to-collection replacement: deferred.** The typed `set`
  refuses it with the write tier's own message, and accent keeps its
  fragment fallback until yqr grows a typed re-shape — that is a write
  tier feature (fragment rendering in the replaced node's style, plus
  the re-parse guard), not a binding-layer patch, and it is filed as
  `yqr-f051`. The contract tests that cover the fallback stay in
  accent until then; the ported floor covers everything else.
- **What stays out.** Frontmatter extraction (the `---` block split)
  is markdown-domain and stays in accent; yqr sees only YAML. Field
  coercion, schema-aware validation, and the mapping `diff` that
  derives change sets stay in accent's layer.
- **Engine lockstep.** Once accent consumes yqr, both crates must pin
  the same noyalib version, or one document parses two ways in one
  product. Where that rule lives (a yqr implementation spec, and the
  comment above accent's `yqr` dependency) is decided when accent's
  adoption lands; it must exist before it does. Unchanged by this
  implementation.
- **Serde is two-way for ops.** Unlike `f048`'s output-only
  diagnostics, ops are *inputs*: behind the `serde` feature they
  derive `Serialize` and `Deserialize`, and `Value` gains both (JSON
  data model; a non-string mapping key refuses to serialize). The wasm
  boundary gains `applyAll(ops, yaml)`.

## As landed (2026-11-07)

`yqr::edit`: `Path` (typed segments: `Key`/`Index`), `Op`
(`Set`/`Remove`/`Rename`/`Append`/`SetHeadComment`), and
`apply_all(ops, input)` with the contracts above. Each op lowers to one
guarded write-tier mutation; `Set` additionally resolves missing
mapping levels (one `Assign` of the first missing key with the nested
remainder) and creates the document when the input is empty. `Remove`
and `Rename` against an empty document are no-ops, matching accent.
Behind the `serde` feature, ops and `Value` (de)serialize in the JSON
data model, and the wasm boundary gains `applyAll(ops, yaml)`.

The ported contract floor lives in `tests/edit_contract.rs` (18
cases). Accent-domain cases stayed behind: the `---` block split, the
mapping `diff`, the CST path-grammar spelling (typed segments have no
grammar to misread — one case pins that hostile keys are just data),
and the scalar-to-collection fragment fallback, deferred as
`yqr-f051`.

## Acceptance criteria

- [x] A host composes set/remove/rename/append ops from typed parts —
      no filter text — and applies them through one entry point.
- [x] Empty change set in, input bytes out, byte-identical; a test
      pins it.
- [x] Every op is exercised through the wasm bindings (`f049`) as JSON.
- [x] accent's `f336` contract tests run in yqr's suite and pass — the
      YAML-editing floor in full; the scalar-to-collection case moves
      with its feature to `yqr-f051`, and accent keeps its fallback
      until then (see the decision log).
- [x] The parent-creation and scalar-to-collection decisions are
      recorded in this spec before implementation starts.
