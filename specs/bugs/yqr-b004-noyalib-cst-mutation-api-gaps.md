# Bug b004 — noyalib CST mutation-API gaps: comment editing, key rename, sequence reorder, nested/multi-line delete

**Status:** Fixed upstream and **released** — every gap below ships in
**noyalib 0.0.18** (crates.io 2026-07-31, GitHub release `v0.0.18` the
same day). Verified against the published `.crate`, not just the branch:
`src/cst/emit.rs` is present, `cst/mod.rs` re-exports `pub use
emit::{Emit, EmitCtx}`, and `rename_key`, `key_span`, `swap_items`,
`move_item`, `set_inline_comment` / `remove_inline_comment`,
`set_leading_comment` / `remove_leading_comment`,
`insert_entry_value` / `push_back_value` / `insert_after_value` are all
in the shipped source. **Still Open for yqr:** the pin is unchanged at
`noyalib = "0.0.17"` (`Cargo.toml:40`), whose edit API is unchanged
since 0.0.14 (the v0.0.14...v0.0.17 diff touches no `cst/` source file —
0.0.15 was loader-parity + coverage hardening, 0.0.16 a build fix / MSRV
1.86 / dependency refresh, 0.0.17 a no-change lockstep republish), so
the gaps remain real for shipped code and each still falls back to raw
`Document::replace_span(start, end, repl)` byte splicing. This bug
closes when the pin bump lands — tracked as `yqr-f013`, whose scope §6
now sizes against measured 0.0.18 behaviour.
**Reported upstream (2026-07-29):** umbrella issue
[noyalib#221](https://github.com/sebastienrousseau/noyalib/issues/221)
covers all five gaps and is **still open** even though the release
shipped. The earlier note here that issues are disabled upstream is
stale — they are enabled.
**Contributed upstream:** the §2.2 key rename as
[noyalib#222](https://github.com/sebastienrousseau/noyalib/pull/222)
(**merged** 2026-07-31 06:15 UTC) and the §2.5 auto-formatting tier as
[noyalib#223](https://github.com/sebastienrousseau/noyalib/pull/223)
(**merged** 2026-07-31 14:26 UTC, commit `63ea2a0`) — both into
`feat/v0.0.18`, released 3.5 h after the second merge. The remaining
three gaps — §2.1, §2.3, §2.4 — were implemented upstream by the
maintainer on the same branch on 2026-07-31.
**Severity:** Medium — roadmap-gating for yqr's core goal (surgical editing of YAML: values, keys, structures, comments). No current code path depends on these (the fidelity engines are read-only today, `yqr-m002` §9), and each has a raw-`replace_span` workaround — but that workaround forfeits the indent/quote synthesis and the "reject if the result re-parses differently" guard that the first-class mutators provide.
**Owner:** yqr maintainers
**Last updated:** 2026-08-02
**Affects:** the planned fidelity write/edit tier (`yqr-m002` §4/§6.2, `yqr-f002` §5). Irrelevant to the read path and the default pipeline.
**Component:** noyalib 0.0.14 (unchanged through 0.0.17, yqr's pin; fixed in the released 0.0.18) — `cst::Document` (`document.rs`), `cst::Entry` (`entry.rs`), `cst::annotated` (`annotated.rs`), `cst::emit` (`emit.rs`, new in 0.0.18)
**Related:** `yqr-b002` (noyalib CST span/key-model deficiencies — resolved in 0.0.14), `yqr-f013` (the 0.0.18 adoption feature this bug now hands off to), `yqr-b006` (the structural-delete trivia fixes that §6.1 shows upstream `remove` does *not* reproduce), `yqr-r002` (noyalib fidelity evaluation), `yqr-m002` §4/§6.2 (engine seam / write-tier design), and the noyalib-vs-rust-yaml backend comparison. Upstream precedent: noyalib#118/#123 (BOM fix, PR-with-fix) and the b002 fix series. Upstream reports for this bug: noyalib#221 (umbrella issue, still open), noyalib#222 (rename_key PR, merged), noyalib#223 (Emit tier PR, merged).

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

**Upstream status — released in 0.0.18** (implemented 2026-07-31 by the
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
Merged into `feat/v0.0.18` and released in 0.0.18; 36 tests. The
maintainer additionally added `Document::key_span(path) -> Option<(usize,
usize)>` on that branch — the read-only companion to `span_at`, exposing
the key token's byte range — which was the secondary ask in #221. It is
**not** a replacement for `src/validate/scan.rs`'s green-tree walk; see
§6.2. This gap closes for yqr on the pin bump (`yqr-f013`).

### 2.3 No sequence reorder / move / swap

`Document` can append/insert items (`push_back`, `insert_after`) but cannot
reorder them — there is no reorder / move / swap method (confirmed absent across
`cst/`). Reordering means several `replace_span` calls with manually shifted
offsets, where each edit invalidates the offsets computed for the others.

**Impact on yqr:** reordering list items has no safe primitive; hand-rolled
multi-splice is error-prone.

**Upstream ask:** `swap_items(seq_path, i, j)` and/or
`move_item(seq_path, from, to)`.

**Upstream status — released in 0.0.18** (implemented 2026-07-31 by the
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

**Upstream status — released in 0.0.18** (implemented 2026-07-31 by the
maintainer): `remove` now deletes a key whose value is a nested mapping,
block sequence, or block scalar — the whole entry, key/`-` through its
last owned line — guarded by an eager re-parse and a typed-value oracle
(the document minus exactly that path) with rollback, which is the same
design as yqr's interim fallback. The single-line case keeps its original
fast path. **Still refused upstream:** removing the sole entry of a block,
and flow-collection entries — the two cases yqr also refuses today.

**Correction to the earlier reading of this gap.** This spec previously
concluded that `src/fidelity/write/delete.rs` "can shrink to its
refusals". Measured against the released 0.0.18 (§6.1), that is **wrong**:
upstream `remove` deletes the *entry* but not the trivia yqr treats as
part of it, so adopting it wholesale would regress `yqr-b006`. The
fallback's trivia handling stays; only its span arithmetic for the shapes
upstream now covers is redundant. §6.1 has the case-by-case evidence.

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
[noyalib#223](https://github.com/sebastienrousseau/noyalib/pull/223),
merged and released in 0.0.18** (merged 2026-07-31 14:26 UTC into
`feat/v0.0.18`, commit `63ea2a0`; released the same day). It adds
`cst::Emit` / `cst::EmitCtx` (re-exported from `cst/mod.rs`) and the typed
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

Medium, and now actionable. The fidelity engines are read-only today, so nothing
regresses now; these become gating when the write/edit tier is built (`yqr-m002`
§4/§6.2). Each gap has a raw-`replace_span` workaround, so none is a hard blocker
— but each workaround re-implements, inside yqr, the indent/quote/guard logic
noyalib already owns for the supported operations. Making that cost visible (and
driving it upstream) was the point of this spec, and it worked: all five gaps are
addressed upstream and released in 0.0.18. The remaining work is yqr's adoption,
specced as `yqr-f013` and scoped by §6.

## 5. Upstream reporting

Issues **are** enabled on noyalib (the earlier note here was wrong), so the
five gaps went up as one umbrella issue,
[noyalib#221](https://github.com/sebastienrousseau/noyalib/issues/221)
(2026-07-29), with fixes contributed as PRs following the accepted
#118/#123 precedent (as with b002).

| Gap | Upstream | State (verified in the published 0.0.18) |
|-----|----------|------------------------------------------|
| 2.1 comment editing | maintainer | released — `set_inline_comment` / `remove_inline_comment`, `set_leading_comment` / `remove_leading_comment` (single-line nodes) |
| 2.2 key rename | [noyalib#222](https://github.com/sebastienrousseau/noyalib/pull/222) (ours) | merged 2026-07-31, released; `key_span` added alongside |
| 2.3 sequence reorder | maintainer | released — `swap_items`, `move_item` |
| 2.4 extended delete | maintainer | released — multi-line / nested; sole-entry + flow still refused; **trivia not folded** (§6.1) |
| 2.5 `Emit` auto-formatting | [noyalib#223](https://github.com/sebastienrousseau/noyalib/pull/223) (ours) | merged 2026-07-31 (`63ea2a0`), released — `Emit` / `EmitCtx`, `*_value` mutators |

Release: **noyalib 0.0.18**, crates.io + GitHub release 2026-07-31, MSRV
1.86.0 (yqr pins toolchain 1.97.1, so no MSRV impact).

Umbrella issue #221 is **still open** upstream despite every gap shipping
— it is the maintainer's to close, and yqr should not read its open state
as unfinished work.

## 6. Adoption in yqr (unblocked; specced as `yqr-f013`)

0.0.18 is on crates.io, so the pin bump from `= "0.0.17"` is now the only
gate. `yqr-f013` owns the work; this section records what was **measured**
against the released crate, because two of the three handoff notes written
before the release turned out to be wrong.

### 6.1 Upstream `remove` does not subsume yqr's delete fallback

Probed by driving noyalib 0.0.18's `Document::remove` directly over the
cases `src/fidelity/write/delete.rs` covers (`yqr-f007` §5.4). Upstream
handles the **shapes** — nested block mapping, multi-line sequence item
(`list[0]`), a block sequence at its key's own column, a comment on the
key line, a following sibling's comment, blank lines between entries — and
refuses the same two cases yqr refuses (sole entry of a mapping *and* of a
sequence; flow items).

It does **not** reproduce the trivia handling `yqr-b006` added:

| Case | yqr today | noyalib 0.0.18 `remove` |
|------|-----------|-------------------------|
| Head comment above the entry (`# doc for b` / `b: 2` / `c: 3`, delete `b`) | comment removed with its entry | comment **survives**, silently re-attributed to `c` |
| Same, indented (`top:` / `# doc` / `b: 2` / `c: 3`, delete `top.b`) | removed with the entry | survives above `c` |
| Keep-chomped scalar (`a: \|+` with kept trailing blanks, delete `a`) | kept blanks removed with the entry | **two stray blank lines** left behind |

Both divergences are silent successes, not refusals — exactly the failure
class `yqr-b006` was filed for. So the fallback's **trivia** logic
(`absorb_head_comments`, the keep-chomped end-of-span rule) must stay; what
becomes redundant is only its span arithmetic for shapes upstream now maps.
`yqr-f013` decides whether to keep yqr's path wholesale or call upstream and
re-apply the trivia rules, and either way the `yqr-f007` §5.4 tests are the
regression net.

Also worth keeping: yqr's flow pre-check produces `removing an item from a
flow collection is not supported`, where upstream surfaces `remove: could
not locate '-' indicator preceding sequence item`. yqr's message is the
better diagnostic and should not be traded away for the shorter call path.

### 6.2 `key_span` does not remove `validate`'s green-tree walk

Confirmed, so the "check before assuming" note resolves to **no**.
`Document::key_span(path) -> Option<(usize, usize)>` is path-addressed: it
answers "where is the key at this path". `src/validate/scan.rs` does the
opposite — a depth-first `scan` over every `GreenNode`, enumerating each
block and flow mapping's keys to find duplicates, which by definition it
cannot address by path (the duplicates share one). `key_span` may still be
useful for `yqr-f007`'s deferred key-rename slice; it does nothing for the
duplicate-key scan, which stays as it is.

### 6.3 Genuinely new capability to wire up

The remaining mutators have no yqr equivalent today and are the real
payload of the bump: `rename_key` for key edits, `swap_items` / `move_item`
for sequence reorder, the four comment setters for comment edits, and the
`*_value` insertion tier (`insert_entry_value` / `push_back_value` /
`insert_after_value`) for anything yqr would otherwise quote itself. Each
unblocks a deferred `yqr-f007` §6 gap, and each still needs the user-facing
grammar that spec calls out as unsettled — the API landing does not settle it.

### 6.4 Follow-up upstream ask (not yet filed)

§6.1's two divergences are worth reporting on the #221 precedent: `remove`
should fold an entry's contiguous same-indent head comment and a
keep-chomped scalar's kept trailing blank lines into the deletion, as yqr
does. Until then yqr keeps owning that arithmetic.
