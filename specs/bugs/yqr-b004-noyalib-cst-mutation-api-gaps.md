# Bug b004 — noyalib CST mutation-API gaps: comment editing, key rename, sequence reorder, nested/multi-line delete

**Status:** Open — upstream gaps in **noyalib 0.0.14**'s edit API. yqr's fidelity write/edit tier is not yet built (`yqr-m002` §4/§6.2, `yqr-f002` §5), so these do not block current code, but they constrain the automatic-editing roadmap. Every gap has the same fallback: raw `Document::replace_span(start, end, repl)` byte splicing.
**Severity:** Medium — roadmap-gating for yqr's core goal (surgical editing of YAML: values, keys, structures, comments). No current code path depends on these (the fidelity engines are read-only today, `yqr-m002` §9), and each has a raw-`replace_span` workaround — but that workaround forfeits the indent/quote synthesis and the "reject if the result re-parses differently" guard that the first-class mutators provide.
**Owner:** yqr maintainers
**Last updated:** 2026-07-11
**Affects:** the planned fidelity write/edit tier (`yqr-m002` §4/§6.2, `yqr-f002` §5). Irrelevant to the read path and the default pipeline.
**Component:** noyalib 0.0.14 — `cst::Document` (`document.rs`), `cst::Entry` (`entry.rs`), `cst::annotated` (`annotated.rs`)
**Related:** `yqr-b002` (noyalib CST span/key-model deficiencies — resolved in 0.0.14), `yqr-r002` (noyalib fidelity evaluation), `yqr-m002` §4/§6.2 (engine seam / write-tier design), and the noyalib-vs-rust-yaml backend comparison. Upstream precedent: noyalib#118/#123 (BOM fix, PR-with-fix — issues are disabled upstream).

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

## 2. Gaps (noyalib 0.0.14)

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

### 2.3 No sequence reorder / move / swap

`Document` can append/insert items (`push_back`, `insert_after`) but cannot
reorder them — there is no reorder / move / swap method (confirmed absent across
`cst/`). Reordering means several `replace_span` calls with manually shifted
offsets, where each edit invalidates the offsets computed for the others.

**Impact on yqr:** reordering list items has no safe primitive; hand-rolled
multi-splice is error-prone.

**Upstream ask:** `swap_items(seq_path, i, j)` and/or
`move_item(seq_path, from, to)`.

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

Medium. The fidelity engines are read-only today, so nothing regresses now; these
become gating when the write/edit tier is built (`yqr-m002` §4/§6.2). Each gap
has a raw-`replace_span` workaround, so none is a hard blocker — but each
workaround re-implements, inside yqr, the indent/quote/guard logic noyalib
already owns for the supported operations. Making that cost visible (and driving
it upstream) is the point of this spec.

## 5. Upstream reporting

noyalib has GitHub issues disabled, so each item is a **PR-with-fix candidate**
following the accepted #118/#123 precedent (as with b002). Prioritise **2.1
(comment editing)** and **2.2 (key rename)** — the two most central to yqr's
automatic-editing goal.
