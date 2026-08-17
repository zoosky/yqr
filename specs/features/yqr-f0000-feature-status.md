# Feature Status Tracker

Single source of truth for the state of every `yqr-fNNN` feature spec. Update
this file in the same change that advances a feature (CLAUDE.md rule 17).

**Status legend:** Draft · In Progress · Done · Superseded · Historical

## Epic: jq-style YAML processor (f001)

| Feature | Title | Status |
|---------|-------|--------|
| [f001](yqr-f001-yaml-jq-m0.md) | yqr: a Swiss Army knife for YAML (M0 foundation) | In Progress (M0 done; M1+ open) |
| [f017](yqr-f017-to-entries.md) | `to_entries`: enumerate a mapping without losing the keys | Draft (scoped, not started) |

Progress: M0 foundation landed (lexer/parser/eval/CLI, tests, CI); M1-M4 open.
f017 is the one builtin **pulled forward** out of that queue, on field evidence
rather than on the gap table: `yqr-r003` records an agent session that hit the
"iterate a mapping and keep the keys" wall on a real file and left for a Python
script. `key(...)` (`f007` §7) closed half of it after the fact; `to_entries`
closes the other half, and turns out not to be gated on M1 object construction
the way the queue assumed (`f017` §3).

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
| [f007](yqr-f007-write-tier-structural-edits.md) | Write tier: structural edits (the `b004` gaps) | In Progress (structural delete shipped; comment/rename grammar settled in `yqr-a002`, unimplemented; reorder blocked on `yqr-b010`) |
| [f008](yqr-f008-write-tier-computed-updates.md) | Write tier: computed updates (`\|=`) | Draft (stub — gated on `f001` M2) |
| [f013](yqr-f013-noyalib-0-0-18-adoption.md) | Adopt noyalib 0.0.18: pin bump and the released CST mutation API | Done |
| [f014](yqr-f014-noyalib-0-0-21-adoption.md) | Adopt noyalib 0.0.21: the silent-corruption fixes and the typed insertion tier | Done |
| [f015](yqr-f015-noyalib-0-0-22-adoption.md) | Adopt noyalib 0.0.22: delete the CRLF workaround the upstream fix subsumes | Done |
| [f016](yqr-f016-noyalib-0-0-23-adoption.md) | Adopt noyalib 0.0.23: the extended `remove`, and the two deletes yqr still refuses | Draft (blocked — 0.0.23 unpublished) |

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
Two of the three are now implementation over a live API. The third is **not**:
measuring 0.0.22 rather than reading its docs showed the backend *is* the
blocker for reorder — `swap_items`/`move_item` exchange value bytes only, so a
reorder silently re-attributes every comment in the range at exit 0, filed as
**`yqr-b010`** (open, unfiled upstream). The same pass corrected three further
upstream asymmetries the comment slice must pre-check rather than delegate
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
temporary 0.0.21 pin and pass on 0.0.22 (`f015` §4). f016 **draft and blocked**:
noyalib closed the `b004` umbrella (`#221`) on 2026-08-16 by extending `remove`
to cover flow members and sole entries — the two classes `f007` §5 refuses — but
0.0.23 has no tag, no release and nothing on the crates.io index, so the pin
cannot move. It also does **not** fix `b010`. The interesting part is not the
bump: it invalidates the premise of `f007` §6's "delegate delete: no", whose
measurement recorded its only two failures as flow cases where upstream also
refused and only the diagnostic differed. Re-run and decision are `f016` §4/§5,
and no decision is recorded ahead of the measurement. f008 (`|=` computed
updates) is gated on `f001` M2 (arithmetic/builtins). Priority order: f006
(done) → f007 delete (done) → f013 (done) → f014 (done) → f015 (done) → f016
(blocked on the release) → f007 remainder → M2 → f008.

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

- Total features: 17
- Draft: 3 (f008 — computed updates, gated on `f001` M2; f016 — noyalib 0.0.23
  adoption, blocked on the release, and owing the `f007` §6 re-measurement;
  f017 — `to_entries`, scoped from the `yqr-r003` usage report)
- In Progress: 2 (f001 M0; f007 — structural delete shipped; the comment /
  rename / reorder grammar settled in `yqr-a002` and staged as three slices,
  none implemented, and slice 3 blocked on `yqr-b010`)
- Done: 9 (f002, f006, f009, f010, f011, f012, f013, f014, f015)
- Superseded: 3 (f003, f004 — single-engine consolidation, `yqr-m005`; f005 —
  fidelity-by-default flip, `yqr-f009`)
- Released in `v0.3.0`: f002 (fidelity engine) and f005 (`--preserve`, later
  superseded by f009 which makes fidelity the default)
- Released in `v0.4.0`: f006 (write tier — assignment, `+=`, new-key, `del`,
  `-i`), f007 (structural-delete slice), and f009 (fidelity by default;
  `--normalize` replaces `--preserve`)
