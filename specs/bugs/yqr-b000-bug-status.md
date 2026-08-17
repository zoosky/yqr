# Bug Status Tracker

Single source of truth for the state of every `yqr-bNNN` bug. Update this file
in the same change that opens, advances, or resolves a bug (mirrors the feature
status tracker convention).

**Status legend:** Open · In Progress · Fixed · Resolved · Won't Fix · Duplicate

## Open

| Bug | Title | Severity | Status | Related |
|-----|-------|----------|--------|---------|
| [b011](yqr-b011-multiline-flow-collection-fails-to-parse.md) | A multi-line flow collection is valid YAML that yqr cannot read at all | Medium | Open — noyalib refuses to parse a flow collection spread over several lines (`ports: [` / `  80,` / `]`), which PyYAML and the YAML spec both accept; the message names the right indentation rule and applies it to the closing indicator. Loud rather than silent (exit 5, no damage), but it is a **whole-file read** refusal, so `a001`'s guarantee is vacuous for these files and `validate` calls them unreadable. Found while reviewing the `yqr-f016` §5 flow-delete work — the "flow deletes only work on single-line collections" limitation is a symptom of this. **Not yet filed upstream**; the fix is in noyalib's parser, a part yqr has not contributed to before | `yqr-a001`, `yqr-b004`, `yqr-f016` |

## Resolved

| Bug | Title | Severity | Status | Related |
|-----|-------|----------|--------|---------|
| [b010](yqr-b010-reorder-moves-values-not-entries.md) | `swap_items` / `move_item` move values, not entries: every comment stays behind | Medium | Resolved — noyalib's reorder mutators exchanged **value bytes** only, so a reorder silently re-attributed every comment in the range at **exit 0**, past upstream's guard by construction (it compares typed values, and a comment is not in one). Filed as noyalib#269 and **corrected in flight**: upstream's behaviour was deliberate, documented and tested (*"the comment annotates the slot"*), so this was a semantics disagreement, not a defect of the b004 §6.1/§6.4 class. yqr's argument — `remove` already decides the same question the other way for the same bytes — was accepted; the fix is yqr's own commit `d397330`, landed via upstream #271 and **released in noyalib 0.0.23**, verified against the published crate by `yqr-f016` §3. Unblocks the `swap`/`move` slice of `yqr-a002` §9 | `yqr-b004`, `yqr-b006`, `yqr-a002`, `yqr-f007`, `yqr-f015`, `yqr-f016` |
| [b004](yqr-b004-noyalib-cst-mutation-api-gaps.md) | noyalib CST mutation-API gaps: comment editing, key rename, sequence reorder, nested/multi-line delete | Medium | Resolved — all five gaps of umbrella issue noyalib#221 released in **noyalib 0.0.18** (crates.io 2026-07-31): our noyalib#222 (key rename) and noyalib#223 (`Emit` auto-formatting tier), both merged, the other three by the maintainer. Two adoption findings stand (b004 §6): upstream `remove` diverges on delete trivia in **three** measured ways, so delete stays yqr's own code by decision rather than necessity (`yqr-f007` §5.1, would otherwise regress `yqr-b006`); and `key_span` does **not** replace `validate`'s green-tree walk. Those divergences were filed as noyalib#225 and fixed by yqr's noyalib#226 — **merged 2026-08-05, released in noyalib 0.0.19** (§6.4). yqr pins `noyalib = "0.0.22"` as of `yqr-f015`. The maintainer's 2026-08-11 status update on #221 listed three of these gaps as still open; yqr's per-item correction from the published crates (`yqr-f014` §4) was **accepted in the 2026-08-14 update**, which now records comment mutation, `rename_key` and sequence reorder as shipped and re-scopes extended `remove` and fragment quoting as partial (`yqr-f015` §5). Two later adoption findings are recorded in §6.5/§6.6 and do **not** reopen this bug: the reorder trivia semantics are disputed in **`yqr-b010`** (open, above), and upstream's leading-comment mutators absorb a blank-detached comment block — documented upstream behaviour, so no defect, but the opposite of `delete_entry`'s rule, which is why `yqr-a002` §4.1.1 declines to adopt it | `yqr-b002`, `yqr-b006`, `yqr-b008`, `yqr-b010`, `yqr-r002`, `yqr-m002`, `yqr-m005`, `yqr-f006`, `yqr-f007`, `yqr-f013`, `yqr-f014`, `yqr-f015`, `yqr-a002` |
| [b009](yqr-b009-inserted-lines-break-crlf-documents.md) | An inserted line ends with `\n`, giving a CRLF document mixed line endings | Medium | Resolved — noyalib's insertion mutators hard-coded `\n`, so new-key assignment and `+=` gave a CRLF file one bare LF per added line, at **exit 0**. Known upstream behaviour recorded in `b004` §2.5 but never filed, so nothing tracked or tested it. Fixed upstream by yqr's **noyalib#261, merged unmodified 2026-08-14 and released in noyalib 0.0.22**, raised on noyalib#221 (b009 §6); the defect was wider there — the fragment mutators and both comment setters shared it, and `set_comment(Inline)` spliced between the `\r` and the `\n`. yqr carried a local `emit` restore in the meantime (`f014`); `f015` pins 0.0.22 and **deletes** it, since the engine now derives an inserted line's terminator from the document the same way it already derived the indentation. The five byte-exact tests survive unchanged and now pin the engine — with the workaround gone, three of them fail against a temporary 0.0.21 pin. Read, `set_value` and `del` were never affected | `yqr-b001`, `yqr-b004`, `yqr-b008`, `yqr-f014`, `yqr-f015`, `yqr-a001` |
| [b008](yqr-b008-fragment-splice-corrupts-multiline-inserts.md) | Hand-built fragments corrupt `+=` and new-key inserts of a multi-line string | High | Fixed — `+=` produced unparseable output and new-key assignment a wrong value whenever the RHS string contained a newline, both at **exit 0**, so `-i` wrote the damage to the user's file and reported success. `value_fragment` rendered the value to text, and the fragment-taking mutators splice a fragment verbatim — synthesising indentation for its first line only — so a block scalar's continuation lines kept the rendering's indentation instead of the insertion site's. Both paths now use noyalib's typed tier (`insert_entry_value` / `push_back_value`), which owns the indentation and holds the splice to a load-back oracle. Three regression tests assert emitted bytes **and** loaded-back value. Ships with the 0.0.21 pin (`yqr-f014`) | `yqr-f006`, `yqr-f014`, `yqr-b004`, `yqr-f013`, `yqr-f007` |
| [b007](yqr-b007-site-links-broken-under-accent-0-23-1.md) | Website: demo-script media link and 404-page `/docs` link break under accent v0.23.1 | Low | Resolved — demo script now linked on GitHub (accent refuses `.sh` as media), 404 page links `/specs/`; CI pins accent v0.23.1 and builds with `--strict-links` so missing link targets fail the build | `yqr-f010` |
| [b001](yqr-b001-roundtrip-discards-whitespace-and-formatting.md) | Round-trip through the default pipeline discards whitespace, comments, and formatting | High | Resolved for the default read — byte fidelity is now the **default** (`yqr-f009`); `yqr '.' f` is byte-for-byte identical to `f` via the fidelity engine (`yqr-f002`). The lossy semantic round trip is now opt-in via `--normalize` (by design) | `yqr-a001`, `yqr-r001`, `yqr.f001`, `yqr-f002`, `yqr-f009`, `yqr-m005` |
| [b006](yqr-b006-structural-delete-trivia-and-fidelity.md) | Structural delete mishandles comments, blank lines, and same-column sequences | High | Resolved — `owned_line_span` now derives its range from noyalib's value span, commits via byte-preserving `replace_span`, and folds a same-indent head comment into the delete; regression tests cover each case | `yqr-f007`, `yqr-b004`, `yqr-a001` |
| [b005](yqr-b005-crossbeam-epoch-advisory-via-criterion.md) | `cargo audit` fails on `crossbeam-epoch` advisory (RUSTSEC-2026-0204) via `criterion` dev-dep | Low | Resolved — `cargo update -p crossbeam-epoch` bumped the transitive pin `0.9.18 -> 0.9.20`; `cargo audit` exits 0, lockfile-only change | `yqr-m001` |
| [b002](yqr-b002-noyalib-cst-span-and-key-model-deficiencies.md) | noyalib CST deficiencies: span boundaries, duplicate-key policy, string-only key model | Medium | Resolved — all 7 fixes released upstream in noyalib 0.0.14 (2.1 in 0.0.13; 2.2-2.7 in the v0.0.14 release #160); yqr pins crates.io `noyalib = "0.0.14"`, git-dep dropped — `yqr-m004`, `yqr-f004` | `yqr-f002`, `yqr-f004`, `yqr-r002`, `yqr-m002` |
| [b003](yqr-b003-rustyaml-roundtrip-trailing-doc-end-marker.md) | rust-yaml fork `RoundTripDocument::parse_all` errors on a trailing `...` after a block collection | Medium | Resolved (moot) — the rust-yaml fork backend was removed when yqr consolidated on noyalib (`yqr-m005`); no longer affects yqr | `yqr-m005`, `yqr-b001`, `yqr-m002` |

## Summary

- Total bugs: 11
- Open: 1 (b011 — noyalib cannot parse a multi-line flow collection, so yqr
  cannot read the file at all; loud, not silent, and not yet filed upstream)
- Resolved: 10 (b010 — noyalib's reorder exchanged value bytes only, so every
  comment stayed with the slot; filed as noyalib#269, re-framed from "defect"
  to a semantics disagreement once its pinning test turned up, argued on
  `remove`/reorder inconsistency, and fixed by yqr's own commit in 0.0.23;
  b009 — an inserted line ended with `\n` regardless of the
  document's convention, silently giving CRLF files mixed endings; worked
  around in `emit` (`yqr-f014`), then fixed at the source by yqr's
  noyalib#261, released in 0.0.22, and the workaround deleted (`yqr-f015`);
  b008 — `+=` and new-key assignment of a multi-line string
  corrupted the document at exit 0; both insert paths now route through
  noyalib's typed insertion tier, `yqr-f014`;
  b004 — all five mutation-API gaps released in noyalib
  0.0.18 and adopted by `yqr-f013`'s pin bump; delete deliberately keeps
  yqr's own path, since upstream `remove` diverges on entry trivia in
  four measured cases — those divergences are now fixed upstream by yqr's
  noyalib#226, released in 0.0.19 and pinned by `yqr-f014`;
  b007 — website links fixed for the accent v0.23.1 media
  policy and link checker, CI now builds with `--strict-links`;
  b001 — byte fidelity is now the default read (`yqr-f009`), closing
  the lossy-default round trip; the classic pipeline is now opt-in via
  `--normalize`;
  b006 — structural-delete trivia/fidelity defects fixed via a
  span-derived range + byte-preserving `replace_span`, `yqr-f007`;
  b005 — `crossbeam-epoch` advisory cleared by `cargo update`;
  b002 — 2.2-2.7 fixes released in noyalib 0.0.14, git-dep dropped;
  b003 — moot after the single-engine consolidation removed the rust-yaml
  backend, `yqr-m005`)
