# Feature f050 — A typed edit API: one editor for accentcms and the browser

**Status:** Draft
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

## Semantics to reconcile (open questions)

- **Parent creation.** accent's `set` creates missing intermediate
  mappings (noyalib's parent-creating `set_path`, upstream `#327`);
  yqr's `Assign` creates only a missing final key. Decide: a flag on
  the set op, or parent creation as the typed API's default with the
  filter language unchanged.
- **Scalar-to-collection replacement.** noyalib's `set_value` refuses
  replacing a scalar with a collection; accent's editor renders a YAML
  fragment for exactly this case. The typed API must cover it, ideally
  by moving that fragment path into yqr so accent's last hand-rendered
  YAML goes away.
- **What stays out.** Frontmatter extraction (the `---` block split)
  is markdown-domain and stays in accent; yqr sees only YAML. Field
  coercion and schema-aware validation stay in accent's API layer.
- **Engine lockstep.** Once accent consumes yqr, both crates must pin
  the same noyalib version, or one document parses two ways in one
  product. Where that rule lives (a yqr implementation spec, and the
  comment above accent's `yqr` dependency) is decided when accent's
  adoption lands; it must exist before it does.

## Acceptance criteria

- [ ] A host composes set/remove/rename/append ops from typed parts —
      no filter text — and applies them through one entry point.
- [ ] Empty change set in, input bytes out, byte-identical; a test
      pins it.
- [ ] Every op is exercised through the wasm bindings (`f049`) as JSON.
- [ ] accent's `f336` contract tests run in yqr's suite and pass.
- [ ] The parent-creation and scalar-to-collection decisions are
      recorded in this spec before implementation starts.
