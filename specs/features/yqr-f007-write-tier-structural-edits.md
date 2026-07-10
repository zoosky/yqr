# yqr.f007 — Write tier: structural edits (the `b004` gaps)

**Status:** Draft (stub — gated on upstream noyalib)
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f006` (write tier v1 — the value-replacement core this builds
on), `yqr-b004` (the noyalib 0.0.14 mutation-API gap catalog), `yqr-m002`
§4/§6.2 (write-tier seam)

> **Stub.** Detail is deliberately deferred: the gap inventory already lives in
> `yqr-b004`, and this feature is gated on upstream noyalib work whose shape may
> change. This spec is a roadmap marker; it gets fleshed out when f006 has
> shipped and the first upstream API lands.

## 1. Scope

The surgical edits that have **no first-class API** in noyalib 0.0.14 and so are
excluded from `f006`. Each is cataloged in `yqr-b004` §2:

- **Comment editing** — add / change / remove a comment attached to a node
  (`b004` 2.1; comments are read-only in 0.0.14).
- **Key rename** — `.a.b` key renamed in place, preserving `:` and value
  (`b004` 2.2).
- **Sequence reorder / move / swap** — reorder block-sequence items (`b004` 2.3).
- **Structural delete** — multi-line, nested, sole-entry, and flow deletes that
  `Document::remove` rejects (`b004` 2.4).

## 2. Dependencies & approach

- **Preferred:** upstream noyalib mutators (`set_comment`, `rename_key`,
  `swap_items`/`move_item`, `remove_subtree`) — each a PR-with-fix to noyalib
  (issues disabled upstream; #118/#123 precedent, `b004` §5). yqr then calls the
  guarded API directly.
- **Interim:** where an upstream API is not yet available, yqr performs the edit
  via raw `Document::replace_span`, owning the indent/quote arithmetic itself,
  behind the same re-parse-safety guard f006 uses (see `f006` §7). Each such
  fallback is called out in code and tests as a temporary path.

## 3. Structural-integrity contract

Identical to `f006` §7: an accepted edit changes only the targeted node's bytes;
an edit that would restructure the document is refused (exit 5); `-i` leaves the
file untouched on refusal.

## 4. Acceptance criteria (outline)

- [ ] Comment set/insert/remove at a resolved path, byte-exact elsewhere.
- [ ] `.old |= key-rename` (final syntax TBD) renames a key, preserving value +
      trailing comment.
- [ ] Sequence reorder/move/swap by index, re-parse-guarded.
- [ ] Delete of a multi-line / nested / sole-entry / flow node, or a clear error
      where still unsupported.
- [ ] Every interim `replace_span` fallback is guarded and covered by a
      byte-exact test.

_(Criteria firm up once the upstream API surface is known.)_
