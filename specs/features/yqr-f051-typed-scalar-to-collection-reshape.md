# Feature f051 — A typed scalar-to-collection re-shape for the edit API

**Status:** Draft
**Epic:** Library surface and wasm (`f046`–`f050`, follow-up)
**Owner:** yqr maintainers
**Related:** `yqr-f050` (the typed edit API whose `set` defers this),
accentcms `f336` (the fragment fallback this replaces), noyalib `#328`
(typed collections, the engine behavior underneath)

## Why

The engine writes a collection over a collection and a scalar over a
scalar; it has no typed route from one shape to the other, and both
yqr's write tier and noyalib's `set_value` refuse the crossing.
accent's editor needs it (`--set tags=["a","b"]` on a page whose
`tags:` is a scalar), so it renders a YAML fragment by hand, splices it
through the CST's text `set`, and re-parses — the last hand-rendered
YAML in accent. `yqr-f050` deliberately deferred this: its typed `set`
refuses the crossing, and accent keeps the fallback.

## What

1. The write tier grows a guarded re-shape: replacing a scalar node
   with a collection value, rendered in block style at the node's
   indent (or flow when the document's surrounding style argues for
   it), with the usual re-parse check certifying the result reads back
   as the value written.
2. `edit::Op::Set` uses it instead of refusing, and the refusal message
   that today names the crossing goes away.
3. accent deletes `render_fragment` and the `is_scalar_to_collection`
   fallback; its contract test
   (`a_scalar_grows_into_a_collection_through_the_fragment_fallback`)
   ports into `tests/edit_contract.rs`.

## Note: the set op's extra parse

`f050`'s typed `set` parses the current document state once per op (the
missing-prefix resolution walk) on top of the write tier's own parse —
the price of deciding creation *before* mutating. Sharing that parse
needs a write-tier entry point that takes an already-parsed stream.
This feature adds a write-tier entry point anyway (the re-shape), so
when it is implemented, weigh folding the shared-parse optimization in;
it only matters if change-set sizes make the double parse measurable.
The note in the code lives on `edit::set`.

## Acceptance criteria

- [ ] A typed `set` of a collection value over a scalar succeeds, in
      block style at the node's indent, and the re-parse guard holds.
- [ ] The ported accent contract test passes in yqr's suite.
- [ ] The filter language is unchanged (`=` still refuses the
      crossing; the grammar cannot spell a collection literal anyway).
