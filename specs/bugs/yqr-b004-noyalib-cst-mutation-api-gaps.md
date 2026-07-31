# Bug b004 — noyalib CST mutation-API gaps: comment editing, key rename, sequence reorder, nested/multi-line delete

**Status:** Fixed upstream, unreleased — every gap below is now
implemented on noyalib's `feat/v0.0.18` branch, but **0.0.18 is not
published**: crates.io still tops out at 0.0.17 (2026-07-25) and yqr
pins `noyalib = "0.0.17"`, whose edit API is unchanged since 0.0.14
(the v0.0.14...v0.0.17 diff touches no `cst/` source file — 0.0.15 was
loader-parity + coverage hardening, 0.0.16 a build fix / MSRV 1.86 /
dependency refresh, 0.0.17 a no-change lockstep republish). This bug
closes when the pin picks up a released 0.0.18; until then the gaps are
still real for yqr's shipped code, and every one of them has the same
fallback: raw `Document::replace_span(start, end, repl)` byte splicing.
**Reported upstream (2026-07-29):** umbrella issue
[noyalib#221](https://github.com/sebastienrousseau/noyalib/issues/221)
covers all five gaps and is still open. The earlier note here that
issues are disabled upstream is stale — they are enabled.
**Contributed upstream:** the §2.2 key rename as
[noyalib#222](https://github.com/sebastienrousseau/noyalib/pull/222)
(**merged** into `feat/v0.0.18`) and the §2.5 auto-formatting tier as
[noyalib#223](https://github.com/sebastienrousseau/noyalib/pull/223)
(**open**). The remaining three gaps — §2.1, §2.3, §2.4 — were
implemented upstream by the maintainer on the same branch on 2026-07-31.
**Severity:** Medium — roadmap-gating for yqr's core goal (surgical editing of YAML: values, keys, structures, comments). No current code path depends on these (the fidelity engines are read-only today, `yqr-m002` §9), and each has a raw-`replace_span` workaround — but that workaround forfeits the indent/quote synthesis and the "reject if the result re-parses differently" guard that the first-class mutators provide.
**Owner:** yqr maintainers
**Last updated:** 2026-07-31
**Affects:** the planned fidelity write/edit tier (`yqr-m002` §4/§6.2, `yqr-f002` §5). Irrelevant to the read path and the default pipeline.
**Component:** noyalib 0.0.14 (unchanged through 0.0.17, yqr's pin; fixed on the unreleased `feat/v0.0.18`) — `cst::Document` (`document.rs`), `cst::Entry` (`entry.rs`), `cst::annotated` (`annotated.rs`), `cst::emit` (`emit.rs`, new in 0.0.18)
**Related:** `yqr-b002` (noyalib CST span/key-model deficiencies — resolved in 0.0.14), `yqr-r002` (noyalib fidelity evaluation), `yqr-m002` §4/§6.2 (engine seam / write-tier design), and the noyalib-vs-rust-yaml backend comparison. Upstream precedent: noyalib#118/#123 (BOM fix, PR-with-fix) and the b002 fix series. Upstream reports for this bug: noyalib#221 (umbrella issue), noyalib#222 (rename_key PR, merged), noyalib#223 (Emit tier PR, open).

## 1. Summary

noyalib 0.0.14 already provides first-class, re-parse-guarded mutators for the
**common** surgical edits, and preserves unedited bytes via `Arc` structural
sharing in its green tree:

- `Document::set(path, fragment)` / `set_value(path, &Value)` — replace a scalar
  value (`set_value` matches the neighbouring quote style) (`document.rs:478,546`).
- `Document::insert_entry(mapping_path, key, fragment)` — add a `key: value`,
  synthesising and re-indenting (`document.rs:811`).
- `Document::push_back(path, fragment)` / `insert_after(item_path, fragment)` —
  add a block-sequence item (`document.rs:637,927`).
- `Document::remove(path)` — delete a single-line block entry (`document.rs:601`).
- `Document::rename_anchor(old, new)` — atomic anchor rename (`anchor.rs:322`).

This spec records the editing operations that yqr's automatic-editing goal needs
which have **no first-class API** in 0.0.14. Each forces the caller down to raw
`Document::replace_span` byte arithmetic.

**Out of scope:** byte-for-byte round-trip fidelity is *not* in question — that
property is solid and heavily tested (`yqr-r002`; noyalib's 351-case suite + 10
fuzz targets). This spec is strictly about the **mutation surface**, not
preservation correctness.

## 2. Gaps (cataloged against noyalib 0.0.14, with upstream status per gap)

### 2.1 Comment editing is unsupported (comments are read-only)

Comments are exposed only for **reading**, via `CommentBundle` and
`Document::comments_at(path)` (`annotated.rs:63,109`). There is no
`set_comment` / `insert_comment` / `remove_comment`. Editing, adding, or removing
a comment means locating its bytes and calling `replace_span` manually, with no
help attaching a comment to a node, distinguishing leading / trailing / inline
position, or fixing up the `#` prefix and surrounding whitespace.

**Impact on yqr:** comment-preserving edits are an explicit part of the goal;
today yqr would own all comment byte-arithmetic itself.

**Upstream ask:** a comment mutation API on `Document` (e.g.
`set_comment(path, position, text)` / `remove_comment(path, position)`) built on
the existing `comments_at` addressing.

**Upstream status — implemented on `feat/v0.0.18`** (2026-07-31, by the
maintainer): `Document::set_inline_comment(path, text)` /
`remove_inline_comment(path)` for the trailing `#` on a single-line node,
and `set_leading_comment(path, text)` / `remove_leading_comment(path)` for
the block above a single-line mapping key. Both carry the re-parse +
value-unchanged guard with rollback (a comment carries no data, so the
typed value must be identical). Per the v0.0.18 changelog, still deferred:
multi-line / nested entries, and leading blocks on sequence items.

### 2.2 No key rename

The write-span resolver and `span_at` target the *value*; key spans are computed
only for span-end math and then discarded. There is no `rename_key` / `set_key`
(only `rename_anchor`, `anchor.rs:322`). Renaming a mapping key requires locating
the key token's bytes and `replace_span`-ing them by hand.

**Impact on yqr:** renaming keys ("replace key: values") is in scope; without an
API, yqr re-derives key byte ranges and owns the quoting/escaping the mutators
would otherwise handle.

**Upstream ask:** `Document::rename_key(path, new_key)` that resolves the key
token, applies quoting as needed, and preserves the `:` and the value.

**Upstream status — merged.**
[noyalib#222](https://github.com/sebastienrousseau/noyalib/pull/222)
implements exactly this ask (style-matched quoting via the `set_value`
helpers, sibling-duplicate refusal, eager re-parse + typed-value oracle
guard with snapshot rollback; explicit `? key` supported, flow mappings a
follow-up), reusing the key spans the `remove` resolver already computes.
Merged into `feat/v0.0.18`; 36 tests. The maintainer additionally added
`Document::key_span(path)` on that branch — the read-only companion to
`span_at`, exposing the key token's byte range — which was the secondary
ask in #221 and is what `yqr validate` walks the green tree by hand for
today (`src/validate/scan.rs`). This gap closes for yqr when 0.0.18 ships
and the pin picks it up.

### 2.3 No sequence reorder / move / swap

`Document` can append/insert items (`push_back`, `insert_after`) but cannot
reorder them — there is no reorder / move / swap method (confirmed absent across
`cst/`). Reordering means several `replace_span` calls with manually shifted
offsets, where each edit invalidates the offsets computed for the others.

**Impact on yqr:** reordering list items has no safe primitive; hand-rolled
multi-splice is error-prone.

**Upstream ask:** `swap_items(seq_path, i, j)` and/or
`move_item(seq_path, from, to)`.

**Upstream status — implemented on `feat/v0.0.18`** (2026-07-31, by the
maintainer), under both names asked for. `swap_items(path, i, j)`
rewrites only the two items' value bytes, leaving the `- ` indicators and
every other item byte-identical; `move_item(path, from, to)` is a run of
adjacent swaps, so it inherits the per-step guard and the whole move is
atomic. Both are held to the re-parse + typed-value oracle: a swap the
byte exchange cannot preserve (items at different indentation depths, for
instance) is refused rather than applied.

### 2.4 Delete is restricted: no multi-line, nested, sole-entry, or flow delete

`Document::remove(path)` (`document.rs:601`) is documented to handle only
single-line block entries. It rejects multi-line values, nested collections,
removing the **only** entry of a block mapping/sequence (the result would parse
as an empty collection), and flow collections. Deleting a nested block or a
multi-line value falls back to raw `replace_span`, and the caller must compute
the correct line/indent span so the result still re-parses.

**Impact on yqr:** structural deletes are part of the goal; only the simplest
case is first-class.

**yqr status (interim fallback shipped, `yqr-f007` §5):** yqr keeps `remove` as
the first choice for single-line entries and, on refusal, falls back to a
`replace_span`-based structural delete for **multi-line / nested block** entries
(`src/fidelity/write/delete.rs`). It computes the entry's owned source lines and
commits only if the re-parsed document equals the original value minus the target
— the integrity guard yqr must own, since `replace_span` guarantees only valid
YAML, not structure preservation (see 2.5). Sole-entry and flow deletes stay
refused with a clear message, pending the upstream ask below.

**Upstream ask:** extend `remove` (or add `remove_subtree`) to cover
multi-line/nested block values and flow entries, keeping the existing "reject if
the result parses differently" guard. Landing this lets yqr drop the interim
fallback and inherit noyalib's own indent/boundary computation.

**Upstream status — implemented on `feat/v0.0.18`** (2026-07-31, by the
maintainer): `remove` now deletes a key whose value is a nested mapping,
block sequence, or block scalar — the whole entry, key/`-` through its
last owned line — guarded by an eager re-parse and a typed-value oracle
(the document minus exactly that path) with rollback, which is the same
design as yqr's interim fallback. The single-line case keeps its original
fast path. **Still refused upstream:** removing the sole entry of a block,
and flow-collection entries — the two cases yqr also refuses today, so
`src/fidelity/write/delete.rs` can shrink to its refusals rather than
disappear.

### 2.5 (Note) Fragment mutators splice verbatim — no auto-quoting

`set` / `insert_entry` / `push_back` take a raw `fragment: &str` and splice it
as-is. They synthesise *indentation* but do not quote/escape a value that needs
it — the auto-formatting `Emit` trait is an explicit deferred follow-up
(`document.rs:459`). Only `set_value(&Value)` does style-matched quoting. A
fragment containing `:`, a leading `-`, or a newline can silently restructure the
document, because the re-parse guard rejects *invalid* YAML, not valid-but-
misinterpreted YAML.

**Impact on yqr:** yqr must quote/escape values itself before calling the
fragment-taking APIs, or route all scalar writes through `set_value`.

**Upstream ask:** land the `Emit` / auto-formatting trait so the fragment
mutators quote/escape as needed (already tracked as a noyalib follow-up).

**Upstream status — contributed as
[noyalib#223](https://github.com/sebastienrousseau/noyalib/pull/223)**
(open, against `feat/v0.0.18`), the one gap of the five the v0.0.18 work
did not cover. It adds `cst::Emit` / `cst::EmitCtx` and the typed
insertion mutators `insert_entry_value`, `push_back_value` and
`insert_after_value`. `Emit` pairs `emit(ctx) -> String` with
`expected_value() -> Value`, and the mutators use the second as an oracle
for the first: after the splice the document must re-parse **and** load
back as the pre-edit value with exactly that one insertion applied, or it
rolls back. That is precisely the guard §2.5 says is missing — the
existing one rejects invalid YAML, not valid-but-misinterpreted YAML.

Behaviour: strings that would change type or structure are quoted
(`8080`, `true`, `- x`, `a: b`, `#lead`); keys are quoted only when they
must be; multi-line strings become block scalars; nested collections use
the file's detected indent. Refused (document left byte-identical): `<<`
and non-printable keys, tagged values, growing an existing scalar entry
into a collection, replacing a key holding `.` or `[` (unaddressable by
the path syntax — *adding* one such as `app.kubernetes.io/name` works),
insertions inside an aliased anchor, and empty collections with no indent
anchor. `Entry::insert_value` / `or_insert_value` route through the tier,
closing the same hole on that API (their **key** was spliced verbatim
too), and `Entry` gains `push_back_value` / `insert_after_value`.

The PR also fixes a latent bug in the existing mutators that its tests
surfaced: `insert_entry` / `push_back` / `insert_after` splice at the end
of the anchor entry's line, which for a document not ending in `\n` is the
end of the source — so the new entry landed on the tail of the old one
(`a: 1  b: 2`) and was rejected as a parse error.

Left alone deliberately, and worth knowing before yqr adopts: nested
collections inherit the serializer's conservative quoting (`cpu: "100m"`
where the file would write `cpu: 100m` — correct, not minimal, and
`set_value` shares it), and a splice into a CRLF document inserts `\n`,
as the existing mutators do.

## 3. What is NOT affected (scope guard)

- **Round-trip fidelity is solid.** `parse_document(s).to_string() == s` holds
  byte-for-byte for accepted input and is heavily tested (`yqr-r002`). These gaps
  are about mutation *coverage*, not preservation.
- **The common edits are first-class:** value replace (`set` / `set_value`), add
  key (`insert_entry`), add sequence item (`push_back` / `insert_after`), and
  single-line delete (`remove`) all exist and are re-parse-guarded.
- **b002 is not re-litigated:** its span/key-model deficiencies (2.1–2.7) are
  resolved in 0.0.14.

## 4. Priority for yqr

Medium, and now waiting rather than working. The fidelity engines are read-only
today, so nothing regresses now; these become gating when the write/edit tier is
built (`yqr-m002` §4/§6.2). Each gap has a raw-`replace_span` workaround, so none
is a hard blocker — but each workaround re-implements, inside yqr, the
indent/quote/guard logic noyalib already owns for the supported operations.
Making that cost visible (and driving it upstream) was the point of this spec,
and it worked: all five gaps are now addressed upstream. The remaining work is
the 0.0.18 release and yqr's adoption of it (§6).

## 5. Upstream reporting

Issues **are** enabled on noyalib (the earlier note here was wrong), so the
five gaps went up as one umbrella issue,
[noyalib#221](https://github.com/sebastienrousseau/noyalib/issues/221)
(2026-07-29), with fixes contributed as PRs following the accepted
#118/#123 precedent (as with b002).

| Gap | Upstream | State |
|-----|----------|-------|
| 2.1 comment editing | maintainer, `feat/v0.0.18` | implemented (inline + leading, single-line nodes) |
| 2.2 key rename | [noyalib#222](https://github.com/sebastienrousseau/noyalib/pull/222) (ours) | merged; `key_span` added alongside |
| 2.3 sequence reorder | maintainer, `feat/v0.0.18` | implemented (`swap_items`, `move_item`) |
| 2.4 extended delete | maintainer, `feat/v0.0.18` | implemented (multi-line / nested); sole-entry + flow still refused |
| 2.5 `Emit` auto-formatting | [noyalib#223](https://github.com/sebastienrousseau/noyalib/pull/223) (ours) | open |

Umbrella issue #221 remains open pending #223 and the release.

## 6. Adoption in yqr (blocked on the 0.0.18 release)

Nothing here is actionable until noyalib 0.0.18 is on crates.io — the pin
is `= "0.0.17"` and the branch work is unpublished. When it lands, bump the
pin and then:

- **Shrink `src/fidelity/write/delete.rs`** (`yqr-f007` §5): the
  multi-line / nested structural-delete fallback is what upstream `remove`
  now does natively, with the same oracle guard. What stays is the
  sole-entry and flow refusals, which upstream also refuses.
- **Reconsider `src/validate/scan.rs`**: `Document::key_span(path)` gives
  the key token's byte range without a hand-rolled green-tree walk. It is
  path-addressed while the duplicate-key scan *enumerates* every mapping,
  so this may not remove the walk — check before assuming it does.
- **Wire up the new mutators** where the write tier currently owns byte
  arithmetic: `rename_key` for key edits, `swap_items` / `move_item` for
  sequence reorder, the comment setters for comment edits, and the
  `*_value` insertion tier (if #223 lands) for anything yqr would
  otherwise have to quote itself.

Track this as its own feature spec when the release appears; this section
is the handoff note, not the plan.
