# Feature Status Tracker

Single source of truth for the state of every `yqr-fNNN` feature spec. Update
this file in the same change that advances a feature (CLAUDE.md rule 17).

**Status legend:** Draft · In Progress · Done · Superseded · Historical

## Epic: jq-style YAML processor (f001)

| Feature | Title | Status |
|---------|-------|--------|
| [f001](yqr-f001-yaml-jq-m0.md) | yqr: a Swiss Army knife for YAML (M0 foundation) | In Progress (M0 done; M1+ open) |

Progress: M0 foundation landed (lexer/parser/eval/CLI, tests, CI); M1-M4 open.

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
| [f007](yqr-f007-write-tier-structural-edits.md) | Write tier: structural edits (the `b004` gaps) | In Progress (structural delete shipped; comment/rename/reorder deferred) |
| [f008](yqr-f008-write-tier-computed-updates.md) | Write tier: computed updates (`\|=`) | Draft (stub — gated on `f001` M2) |

Progress: f006 shipped on noyalib 0.0.14's first-class, re-parse-guarded mutators
(`set_value`/`insert_entry`/`push_back`/`remove`) — `=`, `+=`, new-key assign,
`del`, scalar-literal / path RHS, and atomic `-i`, all through the fidelity write
seam (`src/fidelity/write.rs`), zero upstream work. f007 landed its first slice:
structural **delete** of multi-line / nested block entries via the interim
`replace_span` fallback (`src/fidelity/write/delete.rs`) behind a re-parse
integrity guard yqr enforces itself (`b004` 2.4/2.5), sole-entry and flow deletes
refused. The remaining f007 gaps (comment editing, key rename, sequence reorder)
each need new grammar and stay deferred. f008 (`|=` computed updates) is gated on
`f001` M2 (arithmetic/builtins). Priority order: f006 (done) → f007 (delete done,
rest deferred) → M2 → f008.

## Epic: Editing-loop tooling (f012)

| Feature | Title | Status |
|---------|-------|--------|
| [f012](yqr-f012-validate-command.md) | `yqr validate`: actionable YAML correctness checking (rustc-style diagnostics, exit 0/1/5, `--strict`) | Draft |

Progress: spec drafted. Closes the editing loop (edit, then verify): a
dedicated subcommand that parses every document, re-asserts the a001
byte-tiling invariant, and reports rustc-style diagnostics
(`error[Ynnn]`, `--> file:line:col`, source window, help) that humans and
agents can act on. `--strict` adds duplicate-key and stringified-key
collision findings. Rendering is hand-rolled over noyalib's core
`Location`/`CroppedRegion` API — no new dependencies.

## Epic: Project website (f010)

| Feature | Title | Status |
|---------|-------|--------|
| [f010](yqr-f010-accent-website.md) | Accent CMS website over docs/ and specs/, deployed to GitHub Pages | Done |

Progress: the site builds from `docs/` (the home page is a real CMS page:
original hand-authored markup in `content/index.md` with the design as a
dedicated `home` template, framed by the theme header and footer) with
`specs/` mounted at `/specs`, using the vendored Accent default theme
restyled with the home page's design tokens. `pages.yml`
fetches the pinned accent binary (>= v0.23.0, native sub-path support) from
the upstream GitHub release, builds with the `/yqr` prefix derived from the
base URL, and deploys to `gh-pages` while preserving the benchmark
dashboard.

## Summary

- Total features: 12
- Draft: 2 (f008 — computed updates, gated on `f001` M2; f012 — validate
  command)
- In Progress: 2 (f001 M0; f007 — structural delete shipped, rest deferred)
- Done: 5 (f002, f006, f009, f010, f011)
- Superseded: 3 (f003, f004 — single-engine consolidation, `yqr-m005`; f005 —
  fidelity-by-default flip, `yqr-f009`)
- Released in `v0.3.0`: f002 (fidelity engine) and f005 (`--preserve`, later
  superseded by f009 which makes fidelity the default)
- Released in `v0.4.0`: f006 (write tier — assignment, `+=`, new-key, `del`,
  `-i`), f007 (structural-delete slice), and f009 (fidelity by default;
  `--normalize` replaces `--preserve`)
