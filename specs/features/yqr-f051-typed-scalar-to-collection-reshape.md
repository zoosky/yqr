# Feature f051 — A typed scalar-to-collection re-shape for the edit API

**Status:** Done (2026-11-09: noyalib 0.0.59 ships `grow_value` — PR #505,
incorporated upstream with authorship preserved — so the verification
pin is gone and the crates.io release is the dependency)
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

## Design: the re-shape lives in the engine, the policy in yqr

Two implementations were designed and compared before this one was
chosen.

**Rejected: a yqr-side placeholder splice** (the mirror of
`write::collapse`). yqr would splice a minimal placeholder collection
over the scalar's span on a cloned document, then hand the real value
to `set_value`'s collection arm (noyalib #328), which renders and
verifies. It works, and `collapse.rs` is a proven template — but it
duplicates layout knowledge the engine already owns (indent column,
comment hoisting, line-break respelling) on the client side of the
seam, and it couples yqr to undocumented behavior: that
`replace_collection_value` re-indents to the placeholder's column and
how the placeholder's span absorbs trailing trivia. The engine refuses
the crossing as "a layout decision"; tricking it through the byte
surface answers the layout question in the wrong module.

**Chosen: a typed route in noyalib.** The engine's collection arm
already holds every mechanism the re-shape needs — serializer
rendering at the document's `indent_unit`, re-indentation to the
site's column, flow rendering at flow sites, CRLF respelling, and the
load-back oracle run against the candidate *before* the splice. The
only missing pieces are the type check and the one placement rule
#328 did not define: a block collection replacing a scalar that sits
on its mapping key's line moves onto its own lines, one indent step
past the key, with a trailing comment kept on the key's line (the
engine's #333 hoist, in the same direction). A scalar already on its
own line, and a sequence item's scalar, take the existing in-place
re-indent unchanged.

The extension is a new opt-in method, `Document::grow_value(path,
value)` — named for the engine's own error text, which has always
said "grow a scalar into a collection". `set_value`'s refusal is
unchanged (existing callers may depend on it); its remedy text names
`grow_value` instead of the fragment API. Refusals: a scalar value
(that is `set_value`'s job), an alias token (a collection grown in a
reference's place is not an in-place edit), a property-led scalar (the
`&anchor` / `!tag` would be dropped), and the shared-anchor policy
(#338), with the oracle as the backstop behind all of them.

**The policy stays in yqr.** The filter language's `=` keeps refusing
the crossing (acceptance criterion 3) while `edit::Op::Set` performs
it, and both converge on the same write tier — so the write tier
grows a caller-chosen policy: `write::apply` keeps its exact behavior,
a crate-internal `apply_with` carries a re-shape flag, and only
`edit::set` passes allow. Inside the tier the flag routes
`set_value_unless_unchanged`'s refusal into a new `FidelityWriter`
method (`reshape_scalar`), implemented by one `grow_value` call plus
the tier's usual integrity checks. Index sites keep their refusal:
`Op::Append` is the typed route onto a sequence, and the `del`-shifts
argument against an in-place item re-shape still holds.

**Verification path.** The engine change is developed on the noyalib
fork (`zoosky/noyalib`, checked out at
`accentcms-proj/dependencies/noyalib`) and consumed by yqr through a
`[patch.crates-io]` entry pinned to the fork commit, so the crates.io
dependency and `cargo package` stay intact. This feature's branch does
not merge to `main` while the patch is in place. The design proved out
end to end (yqr's suite, the ported accent contract test, the corpus,
and accent's full workspace suite against the pins), and the engine
change is upstream as noyalib PR #505; yqr adopts the crates.io
release that carries it and drops the patch. accent deletes its
fallback only after that yqr release ships.

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

- [x] A typed `set` of a collection value over a scalar succeeds, in
      block style at the node's indent, and the re-parse guard holds
      (`src/edit.rs` f051 tests; the engine's oracle is stronger: the
      candidate must load back as the document with exactly the one
      path replaced).
- [x] The ported accent contract test passes in yqr's suite
      (`tests/edit_contract.rs::a_scalar_grows_into_a_collection`).
- [x] The filter language is unchanged (`=` still refuses the
      crossing; the grammar cannot spell a collection literal anyway).
      Pinned by the existing guard tests, which run under the default
      `WritePolicy`.

Completed 2026-11-09: noyalib 0.0.59 carries `grow_value` (#505,
incorporated upstream into its v0.0.59 release with integration fixes
for block-scalar comments and CR line endings), the `[patch.crates-io]`
pin is out of `Cargo.toml`, and the whole suite is byte-identical
against the release. The same release carries the #499 insert-anchor
fix (yqr's upstream #506), pinned by
`new_key_is_inserted_into_a_dash_line_item` in the write tier's tests —
a new key lands in a sequence-item mapping whose only key sits on the
`- ` line, the shape that refused before. Sequence-item sites keep
their re-shape refusal by decision (the typed route onto a sequence is
`Op::Append`); the mid-path crossing stays refused because writing past
a scalar would discard it on the way to somewhere else. accent's
fallback deletion (its f434) waits on the next yqr release.
