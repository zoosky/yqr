# Feature Status Tracker

Single source of truth for the state of every `yqr-fNNN` feature spec. Update
this file in the same change that advances a feature (CLAUDE.md rule 17).

**Status legend:** Draft · In Progress · Done · Superseded · Historical

## Epic: jq-style YAML processor (f001)

| Feature | Title | Status |
|---------|-------|--------|
| [f001](yqr-f001-yaml-jq-m0.md) | yqr: a Swiss Army knife for YAML (M0 foundation) | Superseded ([a003](../architecture/yqr-a003-what-yqr-is.md)) |
| [f017](yqr-f017-to-entries.md) | `to_entries`: enumerate a mapping without losing the keys | Done |

Progress: M0 foundation landed (lexer/parser/eval/CLI, tests, CI). M1-M4 are
no longer a plan -- see the re-scope note below.
**Re-scoped and closed (2026-08-21).** [a003](../architecture/yqr-a003-what-yqr-is.md)
found that `a001`'s reprioritization was recorded in `r001` §9 and in `f001`'s
own §2/§3 but never in §7, so the milestone list went on sequencing M1–M4 as
near-term work that nothing was doing. Measured the same day, `f001` §7 was 4
of 31 while `a001`'s priorities were complete. **Ratified**: `f001` is
Superseded, M1–M4 are a menu rather than a plan, and `r001` is the catalogue
they are drawn from. The bar for adopting one is `r003`'s — field evidence
plus a check that the feature is not gated on machinery yqr declined to
build.

f017 is the one builtin **pulled forward** out of that queue, on field evidence
rather than on the gap table: `yqr-r003` records an agent session that hit the
"iterate a mapping and keep the keys" wall on a real file and left for a Python
script. `key(...)` (`f007` §7) closed half of it after the fact; `to_entries`
closes the other half, and turned out not to be gated on M1 object construction
the way the queue assumed (`f017` §3) — the gate was construction *syntax*,
which a builtin does not need since it builds a `Value` in Rust.

f017 **done** (2026-08-20). The AST gains `Ast::Builtin`, a third recognition
rule beside the `f007` selectors and reorder verbs and the only one that costs
no `(`: a builtin is an identifier where a path was expected, and no yqr path
can start with one. Its single non-additive grammar change is that the chain
after a path now also follows a builtin, so `to_entries[]` and
`to_entries[].key` parse. Every write form is refused at parse through
`Ast::builtin()`, because the pairs exist in no file. §10's open question — two
ways to enumerate keys — is **settled** in §11.1 and taught in the guide:
`key(...)` is what your file says (the token, quotes included), `to_entries` is
what it means (the decoded string), and `-r` collapses the difference because
`-r` asks for the value rather than the spelling. What the feature's own first
output found is filed as **`b016`**: the emitter writes a trailing space after
`key:` when a block collection is reached through a sequence item, which is
most `to_entries` pairs. Carried visibly rather than worked around — a blanket
line-strip in `render` was measured to change `"a␣␣\nb"` into `"a\nb"`.

## Epic: Fidelity-first architecture (a001)

| Feature | Title | Status |
|---------|-------|--------|
| [f002](yqr-f002-fidelity-read-floor.md) | Fidelity read floor (`FidelityEngine` seam + noyalib backend) | Done |
| [f003](yqr-f003-fidelity-backend-a-rustyaml.md) | Fidelity backend A (rust-yaml fork `RoundTripDocument` adapter) | Superseded (`yqr-m005`) |
| [f004](yqr-f004-engine-parity-runtime-switch.md) | Engine parity: both backends default-on and runtime-switchable, from the zoosky forks | Superseded (`yqr-m005`) |
| [f005](yqr-f005-preserve-flag-decouple.md) | Decouple byte/comment preservation from backend selection (`--preserve`) | Superseded (`yqr-f009`) |
| [f009](yqr-f009-fidelity-default-normalize.md) | Byte fidelity by default; classic pipeline behind `--normalize` | Done |
| [f011](yqr-f011-remove-engine-flag.md) | Remove `--engine`: noyalib is the only engine | Done |

Progress: the `FidelityEngine` seam + the noyalib CST backend shipped (f002).
The rust-yaml fork backend (f003) and the two-engine parity/runtime-switch story
(f004) were **superseded** when yqr consolidated on noyalib as its sole YAML
engine (`yqr-m005`) — removing the rust-yaml dependencies and unblocking the
crates.io publish. noyalib round-trips the b001 corpus byte-for-byte. The
byte-preserving read first shipped as an opt-in `--preserve` flag, with
`--engine` reduced to backend selection (f005, `v0.3.0`); it then became the
**default** read — the classic pipeline moved behind `--normalize` and
`--preserve` was removed (f009), closing the lossy-default bug `b001`.
Finally the `--engine` flag and the `BackendId` runtime seam were removed
outright (f011): noyalib is yqr's one and only engine, and the skald
placeholder is retired.

## Epic: Fidelity write tier (f006–f008)

Surgical, provably-lossless edits — change only the bytes the filter targets,
leave every other byte untouched, or refuse. yqr's differentiating niche (jq is
JSON-only; yq's edits admit whitespace issues). Split into three features by
dependency/release timing.

| Feature | Title | Status |
|---------|-------|--------|
| [f006](yqr-f006-fidelity-write-tier.md) | Write tier v1: value assignment and in-place edits (`--in-place`) | Done |
| [f007](yqr-f007-write-tier-structural-edits.md) | Write tier: structural edits (the `b004` gaps) | Done (all four `b004` gaps: structural delete, **key rename**, **comment editing** and **sequence reorder**) |
| [f008](yqr-f008-write-tier-computed-updates.md) | Write tier: computed updates (`\|=`) | Done |
| [f013](yqr-f013-noyalib-0-0-18-adoption.md) | Adopt noyalib 0.0.18: pin bump and the released CST mutation API | Done |
| [f014](yqr-f014-noyalib-0-0-21-adoption.md) | Adopt noyalib 0.0.21: the silent-corruption fixes and the typed insertion tier | Done |
| [f015](yqr-f015-noyalib-0-0-22-adoption.md) | Adopt noyalib 0.0.22: delete the CRLF workaround the upstream fix subsumes | Done |
| [f016](yqr-f016-noyalib-0-0-23-adoption.md) | Adopt noyalib 0.0.23: the extended `remove`, and the two deletes yqr used to refuse | Done |
| [f018](yqr-f018-noyalib-0-0-24-adoption.md) | Adopt noyalib 0.0.24: the sole-entry head comment, and what is left of the delegation question | Done |
| [f019](yqr-f019-noyalib-0-0-25-adoption.md) | Adopt noyalib 0.0.25: four bugs closed, and the delegation question answered | Done |
| [f020](yqr-f020-noyalib-0-0-26-adoption.md) | Adopt noyalib 0.0.26: the wrapped-flow delete, and the one bug it does not carry | Done |
| [f023](yqr-f023-noyalib-0-0-27-adoption.md) | Adopt noyalib 0.0.27: the last open engine bug | Done |
| [f025](yqr-f025-override-a-merged-in-key.md) | Override a merged-in key by creating an explicit entry | Draft |
| [f026](yqr-f026-noyalib-0-0-31-adoption.md) | Adopt the noyalib release that carries #373: close b025 on the default path | Done (0.0.31, 2026-09-03: b025 and b026 closed; the definition write for noyalib#338 landed as guarded span surgery; the classic pipeline reads multi-document streams through `load_all_with_config`) |

Progress: f006 shipped on noyalib 0.0.14's first-class, re-parse-guarded mutators
(`set_value`/`insert_entry`/`push_back`/`remove`) — `=`, `+=`, new-key assign,
`del`, scalar-literal / path RHS, and atomic `-i`, all through the fidelity write
seam (`src/fidelity/write.rs`), zero upstream work. f007 landed its first slice:
structural **delete** of multi-line / nested block entries via `replace_span`
(`src/fidelity/write/delete.rs`) behind a re-parse integrity guard yqr enforces
itself (`b004` 2.4/2.5), sole-entry and flow deletes refused. The remaining f007 gaps
(comment editing, key rename, sequence reorder) each needed new grammar, and
**that grammar is now settled in `yqr-a002`** (2026-08-15): one naming function
wrapping a path — `line_comment(p)`, `head_comment(p)`, `key(p)`, assignable
with `=` and removable with `del(...)` — plus a reorder verb `swap(p; i; j)` /
`move(p; from; to)`, staged as three slices with per-slice acceptance criteria.
**All three have now shipped.** Slice 1 landed 2026-08-16 as `key(<path>)` /
`key(<path>) = "new"` (`f007` §7) — the whole new grammar path under the
operation with the fewest cases; its read goes through `key_span` rather than
the filter's own resolved path segment, which is what keeps `key(...)` printing
the document's bytes instead of echoing the query. Slices 2 and 3 landed
2026-08-18: `line_comment` / `head_comment` (`f007` §8) and `swap` / `move`
(`f007` §9). Reorder was the one whose *backend* was the blocker — measuring
0.0.22 rather than reading its docs showed `swap_items`/`move_item` exchanging
value bytes only, so a reorder silently re-attributed every comment in the
range at exit 0 — filed as **`yqr-b010`**, argued as a semantics disagreement
rather than a defect, fixed by yqr's own commit, and released in 0.0.23; the
slice is one call per verb as a result, with the trivia property pinned by a yqr
test rather than assumed. The same measuring pass corrected three further
upstream asymmetries the comment slice pre-checks rather than delegates
(`b004` §6.6, `yqr-a002` §4.1/§5): the comment *removers* refuse nothing,
`set_inline_comment`'s guard fires on the value span so a nested entry's
comment lands on its child's line, and the leading mutators absorb a
blank-detached comment block. f013 **done**: the
pin is 0.0.18, the lockfile moved that one crate and nothing else, `cargo audit`
is clean, and both fidelity harnesses passed untouched. Its one code change was
to stop calling upstream `remove` — on the bump it started accepting the shapes
it used to refuse and promptly failed four delete tests, including one
divergence `b004` §6.1 had missed (it *swallows* a following sibling's comment).
So delete stays yqr's own path by decision, renamed `delete_entry` and no longer
framed as a fallback (`f007` §5.1). Also settled: `key_span` does **not** replace
`validate`'s green-tree walk. f014 **done**: the pin is now 0.0.21, taking
0.0.19 (which carries yqr's own noyalib#226, the `remove`-trivia fix that closes
`b004` §6.4) and 0.0.21 (three silent-corruption fixes in the mutators, two of
which reached yqr through `set_value` — `.k = "a:"` errored and `.k = "\n"`
silently wrote the wrong value). Its code change fixes `b008`: `+=` and new-key
assignment hand-built a text fragment, so a multi-line string was spliced at the
rendering's indentation rather than the insertion site's — producing unparseable
output or a wrong value, both at exit 0. Both now use the typed tier
(`insert_entry_value` / `push_back_value`), which is the `f013` §3.4 hand-off
coming due early. f015 **done**: the pin is now 0.0.22, a single-purpose release
carrying yqr's own noyalib#261 — merged unmodified — so the feature is a bump
plus a **deletion**, retiring the `b009` CRLF restore from `emit` now that the
engine derives an inserted line's terminator from the document the same way it
already derived the indentation. The control is what makes that safe rather than
plausible: with the workaround removed, the same three tests fail against a
temporary 0.0.21 pin and pass on 0.0.22 (`f015` §4). f016 **done**: 0.0.23 published 2026-08-17 and the pin has moved, with
the suite green untouched. Two things came out of it. **`b010` is fixed** —
yqr's own reorder commit ships in this release, so an item's comments travel
with it and `a002` slice 3 was unblocked, leaving slices 2 and 3 as
implementation with nothing waiting on upstream; both then shipped the next
day. And the `f007` §6 delegation
question was re-measured per test: **seven failures, all "yqr refuses and
upstream now succeeds"**, none of them a trivia divergence — but the
sole-entry half of that strands the removed entry's head comment above an empty
`{}` (filed as noyalib#280), which is the `b006`/`b010` failure class a third
time. The flow half is clean. That asymmetry decided §5: **each half went to
whichever implementation was already correct** — flow delegated to upstream,
sole-entry implemented in `delete_entry`, where the head-comment run travels
with the entry. Both classes now work, so `f007` §5 has no refusals left. With slice 3 in,
**`f007` is Done**: all four `b004` gaps are closed and the epic's remaining
items are scope and addressing work `f007` §6 tracks — collection right-hand
sides and keys holding `.` or `[`. The third of those, a write tier for the
shared corpus, was **closed 2026-08-18** (`m003` §3–§6): 31 write cases plus
seven refusals now cover every shipped edit, and the tier found two upstream
defects on its first run, `b012` and `b013`. f008
(`|=` computed updates) is gated on a value-producing right-hand side —
arithmetic, or a builtin that returns one (`yqr-a003` retired the M2
framing; `yqr-a001` §6 already settles the semantics). Priority
order: f006 (done) → f007 delete (done) → f013 (done) → f014 (done) → f015
(done) → f016 (done) → f007 remainder (done) → f018 (done) → f019 (done)
→ f020 (done) → M2 → f008.

f018 **done**: 0.0.24 published 2026-08-18 carrying one functional change, the
fix for yqr's noyalib#280 — a sole entry's head comment now goes with the
entry. The pin moved, the suite is green untouched, and the lockfile *loses* a
crate (noyalib was the last holder of `hashbrown 0.15.5`). The point of the
feature is the re-measurement it forces: `f016` §5 had kept sole-entry delete
in yqr's own code **because** of that stranded comment, so with the reason gone
the delegation was re-run rather than assumed. 242 of 244 lib tests pass under
it; the two that fail are one shape — the sole item of a block sequence at its
key's own column, where upstream writes `on:` / `[]`, which noyalib accepts and
both PyYAML and Psych reject. The class stays in `delete_entry` on that
finding, and the finding is filed as **`b014`**, whose live half is a false
negative in yqr's own `validate` (it walks noyalib's tree, so it inherits the
leniency).

f019 **done**: 0.0.25 published 2026-08-20 carrying **all four** of yqr's open
engine bugs, filed upstream the previous day and released the next morning as
noyalib#287, "four fixes from @zoosky". Three are yqr's commits cherry-picked
with authorship intact (`b011` the wrapped flow parse, `b012` the insert
anchor, `b014` the sole-entry indent); the fourth is `b013`, the one filed
deliberately *without* a patch because the dominance heuristic has a public API
attached — the maintainer took the second of the two options the issue offered
and scored the quote vote at the edit site. Each was verified against the
published crate on its own reproduction rather than from the release notes, the
two `m003` write-tier cases that pinned `b012`/`b013` as-they-behaved were
flipped, and the three bugs that had no yqr-side test gained one. The feature's
real content is §4: the `f018` §5 delegation revisit, which this release was
the trigger for. It comes back **zero divergence** — 382 tests, both `f018`
§4.1 failures gone — and the sole-entry delete stays in `delete_entry` anyway,
for the first time on the standing `f007` §6 argument alone. The argument that
carried it is that all four divergences to date were found *by* having a second
implementation to disagree, so deleting one ends that exactly when the
disagreements stop, which is when it looks safest and is worth the least. The
release supplied its own evidence: verifying `b011` walked the write verbs over
the shape it unblocked and found `b015`, an upstream defect reaching yqr's
output through the flow class, which *is* delegated.

f023 **done**: 0.0.27 published 2026-08-21 carrying yqr's noyalib#298, which
closes `b016` -- and with it every bug yqr has filed against this engine is
fixed in a published release: b011-b014 in 0.0.25, b015 in 0.0.26, b016 in
0.0.27. Six bugs, three releases, four days, five of the six fixed by yqr's
own commits upstream. The release also carries two loader changes that are not
yqr's (alias resolution on the replay branch, and only a plain `<<` being a
merge key), verified against yqr's merge-key and alias behaviour rather than
assumed harmless because they came from elsewhere. The feature's own lesson is
about the pin: `f017` recorded `b016` *as it behaved* on the `m003` rule, and
this bump came back with exactly one failing test -- the pin saying the bump
had changed something. Without it the fix would have landed silently and the
guide would still apologise for a wart that no longer exists.

f020 **done**: 0.0.26 published 2026-08-20 carrying one functional change,
yqr's noyalib#296 — a flow member alone on its line takes the line with it,
closing `b015`. Verified against the published crate on its own reproduction
with all four controls, and the outputs loaded back under PyYAML and Psych.
Its more interesting half is what the release does **not** carry: `b016` is
filed and fixed upstream (noyalib#297 / #298, green) but unmerged, so the pin
in `tests/cli.rs` and the guide's trailing-space note both stay. §5 states that
explicitly rather than letting it read as an oversight — a bug pinned as it
behaves is what tells the *next* bump whether the bump changed it, which is
exactly the job `m003` asks of a pin. The feature also carries the regression
test `b015` §5 deliberately deferred until a fix existed, controls included:
the positive test would pass on a fix that stripped whitespace
indiscriminately, and only the controls distinguish the rule that was
implemented from the easier one.

## Epic: Editing-loop tooling (f012)

| Feature | Title | Status |
|---------|-------|--------|
| [f012](yqr-f012-validate-command.md) | `yqr validate`: actionable YAML correctness checking (rustc-style diagnostics, exit 0/1/5, `--strict`) | Done |

Progress: shipped, then hardened by a full-branch code review (15
confirmed findings, all fixed). yqr's first subcommand parses every
document, re-asserts the a001 byte-tiling invariant, and reports
rustc-style diagnostics (`error[Ynnn]`, `--> file:line:col`, source
window, help) that humans and agents can act on; exit codes 0/1/5,
highest wins across inputs; stdin explicit (`-`), empty file lists a loud
usage error. `--strict` walks the lossless green tree and reports every
duplicate key — merge keys included — with both occurrences' positions;
collisions report as `Y102` in the default checks, non-UTF-8 input as
`Y003`. Conflict-marker files are recognized whole-file and anchored at
the first marker. Rendering is hand-rolled — no new dependencies. The
sized schema follow-up (`--schema`) stays open in the spec's §5.1.

## Epic: Project website (f010)

| Feature | Title | Status |
|---------|-------|--------|
| [f010](yqr-f010-accent-website.md) | Accent CMS website over docs/ and specs/, deployed to GitHub Pages | Done |
| [f021](yqr-f021-split-public-and-internal-sites.md) | Split the public site from the spec site | Done |
| [f022](yqr-f022-traceability-out-of-published-pages.md) | Move docs traceability out of the published page body | Done |
| [f024](yqr-f024-accent-0-25-0-adoption.md) | Adopt accent 0.25.0: the five `llms.txt` findings, fixed | Done |

Progress: the site builds from `docs/` (the home page is a real CMS page:
original hand-authored markup in `content/index.md` with the design as a
dedicated `home` template, framed by the theme header and footer) with
`specs/` mounted at `/specs`, using the vendored Accent default theme
restyled with the home page's design tokens. `pages.yml`
fetches the pinned accent binary (v0.24.0; floor is >= v0.24.0 for the
search-index fixes, sub-path support since v0.23.0) from
the upstream GitHub release, builds with the `/yqr` prefix derived from the
base URL, and deploys to `gh-pages` while preserving the benchmark
dashboard.

## Summary

- Total features: 26
- Draft: 1 (f025)
- In Progress: 0
- Done: 21 (f002, f006, f007, f008, f009, f010, f011, f012, f013, f014, f015,
  f016, f017, f018, f019, f020, f021, f022, f023, f024, f026)
- Superseded: 4 (f003, f004 — single-engine consolidation, `yqr-m005`; f005 —
  fidelity-by-default flip, `yqr-f009`; f001 — re-scoped by `yqr-a003`, M0
  landed and M1–M4 retired as a plan)
- Released in `v0.3.0`: f002 (fidelity engine) and f005 (`--preserve`, later
  superseded by f009 which makes fidelity the default)
- Released in `v0.4.0`: f006 (write tier — assignment, `+=`, new-key, `del`,
  `-i`), f007 (structural-delete slice), and f009 (fidelity by default;
  `--normalize` replaces `--preserve`)
