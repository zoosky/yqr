# Feature f013 — Adopt noyalib 0.0.18: pin bump and the released CST mutation API

**Status:** Done
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-b004` (the mutation-API gap catalog this closes; §6 is the
measured scoping this spec implements), `yqr-b006` (the structural-delete
trivia fixes §3.2 must not regress), `yqr-f007` (structural edits — its
shipped delete slice and its three deferred gaps), `yqr-f006` (write tier v1,
which already routes through noyalib's mutators), `yqr-f012` (`validate`,
whose green-tree scan §3.3 leaves alone), `yqr-m004` (crates.io publishing)

## 1. Scope

Bump `noyalib = "0.0.17"` to `0.0.18` and reconcile yqr's code with the CST
mutation API that release brings. Everything `yqr-b004` catalogued as missing
now exists upstream:

| API | Closes `b004` gap |
|-----|-------------------|
| `set_inline_comment` / `remove_inline_comment` / `set_leading_comment` / `remove_leading_comment` | 2.1 comment editing |
| `rename_key`, `key_span` | 2.2 key rename |
| `swap_items`, `move_item` | 2.3 sequence reorder |
| extended `remove` (multi-line / nested block values) | 2.4 structural delete |
| `Emit` / `EmitCtx`, `insert_entry_value` / `push_back_value` / `insert_after_value` | 2.5 fragment auto-formatting |

**In scope:** the pin bump and its lockfile/audit fallout (§3.1); reconciling
the structural-delete fallback with upstream's `remove` (§3.2); recording that
`key_span` does not change `validate` (§3.3).

**Out of scope:** shipping the user-facing comment-edit, key-rename, and
sequence-reorder operations. Those are `yqr-f007` §6 and each still needs
grammar that spec calls unsettled; the API landing does not settle it. §3.4
records what the bump makes *available* to them, so f007 can be planned against
a real surface instead of a wish list.

## 2. Why now

`yqr-b004` has sat Open purely because the fixes were unpublished. noyalib
0.0.18 reached crates.io on 2026-07-31, and the published crate was verified to
contain the work (not merely a tag ahead of the branch): `src/cst/emit.rs` is
present, `cst/mod.rs` re-exports `Emit` / `EmitCtx`, and every mutator above is
in the shipped source. The bump is the last step before `b004` resolves.

Two of yqr's contributions are in that release — noyalib#222 (`rename_key`) and
noyalib#223 (the `Emit` tier) — so the bump is also how yqr consumes its own
upstream work.

## 3. Work items

### 3.1 Pin bump

`Cargo.toml:40` and the comment above it (which names "the crates.io 0.0.17
release"). Then `cargo check` to refresh `Cargo.lock`, and the full local CI
mirror — noyalib is yqr's parser *and* emitter, so a patch-level change to it
touches every read, every write, and the corpus byte-fidelity assertions.

- **MSRV:** noyalib 0.0.18 declares `rust-version = "1.86.0"`; yqr pins the
  1.97.1 toolchain, so no impact.
- **Dependency surface:** re-run `cargo audit` after the lockfile refresh
  (`yqr-b005` is the precedent for a transitive advisory arriving this way).
- **Behavioural risk:** 0.0.18 is a substantial `cst/` change, unlike the
  0.0.15–0.0.17 releases that touched no `cst/` file. `tests/fidelity.rs` and
  `tests/corpus_validation.rs` are the guards; run both before assuming the
  bump is mechanical.

**Done.** `Cargo.lock` moved noyalib 0.0.17 → 0.0.18 and nothing else — no new
or changed transitive dependency, and `cargo audit` is clean over 94 crates.
The read path needed no change: `tests/fidelity.rs` and
`tests/corpus_validation.rs` both pass untouched, so the byte-fidelity property
is unaffected by the `cst/` rewrite. The write path did need one change, in
§3.2 — and it surfaced as four failing tests on the bump, not as a judgement
call.

### 3.2 Reconcile the structural-delete fallback

`src/fidelity/write/delete.rs` exists because 0.0.14's `remove` refused
multi-line and nested entries (`yqr-f007` §5). 0.0.18's `remove` accepts them,
so the obvious move is to delete the fallback — **and that is wrong.**

Measured against the released crate (`yqr-b004` §6.1), upstream `remove`
handles the same *shapes* and refuses the same two cases (sole entry, flow
item), but does not fold the trivia yqr treats as part of an entry:

- a contiguous same-indent **head comment** above the entry survives upstream
  and is silently re-attributed to the following sibling;
- a **keep-chomped** (`|+` / `>+`) scalar's kept trailing blank lines are left
  behind as stray blanks.

Both are silent successes, not refusals — the exact failure class `yqr-b006`
was filed for. Adopting upstream `remove` wholesale would regress that bug.

**Decide between two options**, with the `yqr-f007` §5.4 tests as the net
either way:

- **(a) Keep yqr's path.** Leave `delete.rs` as the delete implementation and
  drop only the "interim" framing — it is no longer a stopgap for a missing
  API but a deliberate superset of it. Cheapest, and keeps the trivia rules
  where their tests already are.
- **(b) Call upstream, re-apply trivia.** Let `remove` own the entry span and
  keep `absorb_head_comments` plus the keep-chomped end-of-span rule as a
  pre-pass that widens the range. Less byte arithmetic in yqr, but the
  pre-pass still needs the span it is widening, which is most of what
  `owned_line_span` computes today — so the saving may be small.

Recommendation: **(a)** for this feature, with (b) reconsidered if §3.4's
follow-up ask lands upstream and removes the divergence.

Independently, keep yqr's flow-collection pre-check regardless of the choice:
it reports `removing an item from a flow collection is not supported`, where
upstream surfaces `remove: could not locate '-' indicator preceding sequence
item`. The clearer diagnostic is worth the extra check.

**Decided: (a), and more decisively than the scoping expected.**

The bump made this concrete rather than theoretical. `delete` tried `remove`
first and fell back only on refusal, so the moment 0.0.18's `remove` stopped
refusing, it took over every case — and **four** delete tests failed on the
bump alone, before any code was touched:

- `removes_a_head_comment_with_its_entry` and
  `removes_multiple_contiguous_head_comment_lines` — the head comment survived;
- `keeps_trailing_blank_lines_of_a_keep_chomped_scalar` — a stray blank left
  behind;
- `does_not_eat_a_following_siblings_comment` — **a divergence `yqr-b004` §6.1
  had not catalogued.** Upstream *swallowed* a comment that belongs to the next
  sibling. The first three under-delete; this one over-deletes and loses a
  comment outright. §6.1 listed "a following sibling's comment" among the
  handled shapes because the probe measured refuse-or-not, not output bytes;
  `yqr-b004` §6.1 now carries the correction.

So option (b) is not merely a small saving — it is a bad trade at three
divergences, one of them data-losing. Implemented as (a) in full: `delete` no
longer calls `remove` at all, `delete_structural` is renamed `delete_entry`
(nothing about it is a fallback any more), its `remove_err` parameter and the
"delete fallback" phrasing in the layout-refusal message are gone, and the
module doc states the semantic difference — an entry owns its trivia — as the
reason rather than a missing API. All 152 lib tests and the full local CI
mirror pass. `yqr-f007` §5.1 carries the divergence table and the rationale.

### 3.3 `validate` is unaffected — record it and move on

`yqr-b004` §6 flagged `Document::key_span` as a possible replacement for
`src/validate/scan.rs`'s hand-rolled green-tree walk, to be checked rather
than assumed. Checked: **no.** `key_span(path)` is path-addressed and answers
"where is the key at this path"; `scan.rs` enumerates every block and flow
mapping's keys depth-first to find duplicates, which by definition cannot be
path-addressed (the duplicates share a path). The walk stays. This item is
closed by documenting it, not by code.

### 3.4 New capability made available (hand-off to `yqr-f007`)

The bump puts four operations within reach that yqr has no equivalent for
today. None ships here; this is the surface `yqr-f007` §6 can now plan against:

- **Comment editing** — inline and leading setters/removers, single-line nodes
  only upstream; multi-line / nested entries and leading blocks on sequence
  items remain unsupported there.
- **Key rename** — `rename_key` with style-matched quoting, sibling-duplicate
  refusal, and the re-parse + typed-value oracle guard; flow mappings are an
  upstream follow-up.
- **Sequence reorder** — `swap_items`, and `move_item` as a guarded run of
  adjacent swaps; a swap the byte exchange cannot preserve (differing
  indentation depths) is refused rather than applied.
- **Typed insertion** — `insert_entry_value` / `push_back_value` /
  `insert_after_value`, which quote and escape via `Emit` instead of splicing a
  caller-built fragment verbatim. This is the one item with a **latent
  correctness argument** for adopting it independent of new grammar: yqr's
  `value_fragment` (`src/fidelity/write.rs`) hand-builds fragments today, and
  routing scalar writes through the typed tier moves that burden upstream,
  behind an oracle that rejects valid-but-misinterpreted YAML rather than only
  invalid YAML (`yqr-b004` 2.5). Evaluate it as f007's first slice.

Two upstream behaviours to know before building on any of these
(`yqr-b004` 2.5): nested collections inherit the serializer's conservative
quoting (`cpu: "100m"` where the file would write `cpu: 100m`), and a splice
into a CRLF document inserts `\n`.

## 4. Acceptance criteria

- [x] `Cargo.toml` pins `noyalib = "0.0.18"`, the adjacent comment names 0.0.18,
      and `Cargo.lock` is refreshed.
- [x] `bash .github/scripts/local-ci.sh` passes, including `cargo audit`.
- [x] `tests/fidelity.rs` and `tests/corpus_validation.rs` pass unchanged — the
      bump alters no byte-fidelity behaviour.
- [x] The §3.2 decision is made, implemented, and recorded in `yqr-f007` §5;
      every `yqr-f007` §5.4 delete test still passes, head-comment and
      keep-chomped cases included.
- [x] `yqr-f007` §2's "unreleased `feat/v0.0.18`" framing is updated to the
      released API.
- [x] `yqr-b004` moves to Resolved in `yqr-b000-bug-status.md`, and `yqr-b004`
      §6's adoption notes are marked done.
- [x] `CHANGELOG.md` records the dependency bump.

## 5. Non-goals

- No new filter grammar. `del`, `=`, `+=` and the existing surface are
  unchanged by this feature.
- No behaviour change to the read path or `--normalize`.
- Not a release. `yqr-m001` governs when a version ships; `cargo publish` stays
  separately authorized (`yqr-m004`).

## 6. Follow-up: upstream ask — noyalib#225, fixed by noyalib#226

§3.2's divergences — three kinds, not the two this spec scoped — were filed
2026-08-02 as
[noyalib#225](https://github.com/sebastienrousseau/noyalib/issues/225), a
follow-up to #221 §4 on the #222 / #223 precedent. `remove` should fold an
entry's contiguous same-indent head comment and a keep-chomped scalar's kept
trailing blank lines into the deletion, and should *not* swallow a following
comment that lies outside the entry's value span. The issue carries a runnable
repro per case plus the two controls 0.0.18 already gets right, argues the fix
is a span-boundary refinement rather than new machinery, and offers a PR.

That PR is
[noyalib#226](https://github.com/sebastienrousseau/noyalib/pull/226), opened
2026-08-02 and **merged into `main` on 2026-08-05**. It implements all three:
`remove` now derives its range from the same value-span boundary `span_at`
reports, which is what the refinement amounts to. The cross-check that matters
here is that yqr's suite passes with `del` routed back through upstream
`remove` against the patched crate — the four tests §3.2 records as failing on
the bump included.

The merge is not yet a release: crates.io still tops out at noyalib 0.0.18 and
there is no 0.0.19 tag, so yqr has nothing to pin. When it **ships in a release
yqr can pin**, §3.2 option (b) becomes clearly correct and `delete.rs` can
genuinely shrink to a trivia pre-pass; that re-evaluation is the natural
successor to this feature, not part of it. Full case list in `yqr-b004` §6.4.
