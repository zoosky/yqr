# Feature f015 — Adopt noyalib 0.0.22: delete the CRLF workaround the upstream fix subsumes

**Status:** Done
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f014` (the 0.0.21 adoption this succeeds, whose §3.3 workaround
this removes), `yqr-b009` (the bug that workaround was for, and whose §6
instruction — "delete it when upstream lands" — this carries out), `yqr-b004`
(the mutation-API gap catalog), `yqr-f007` §6 (the deferred grammar work),
`yqr-m004` (crates.io publishing)

## 1. Scope

Bump `noyalib = "0.0.21"` to `0.0.22` and **delete code**. This release contains
exactly one functional change, and it is yqr's own contribution
([noyalib#261](https://github.com/sebastienrousseau/noyalib/pull/261)) — so the
whole feature is a pin bump plus the removal of the local workaround that
contribution makes redundant.

**In scope:** the pin bump (§3.1); deleting the `yqr-b009` CRLF restore from
`NoyalibWriter` (§3.2); re-aiming its five tests at the engine (§3.3); the
evidence that the deletion is safe rather than merely plausible (§4); recording
the upstream close-out (§5).

**Out of scope:** everything in `yqr-f007` §6 — the comment/rename/reorder
grammar, collection right-hand sides, and keys holding `.` or `[`. None of them
moved with this release. Contributing upstream's two remaining `#221` gaps
(§6).

## 2. What 0.0.22 contains

Released 2026-08-14, one day after 0.0.21. The changelog has one `Fixed` entry
and one `Changed` entry:

| Change | Reachable from yqr? |
|---|---|
| **Splices adopt the document's own line break instead of assuming `\n`** (#261) | **Yes** — it is the fix for `yqr-b009`, filed against yqr's own two insert paths |
| Install snippets across the upstream READMEs moved from `0.0.18` to `0.0.22` | No — documentation only |

So unlike `yqr-f013` (a capability bump) and `yqr-f014` (a corrective bump with
three fixes of its own to consume), this one is single-purpose. Nothing else in
the release touches a path yqr calls.

**noyalib#261 was merged unmodified.** One commit (`0e647db`), all three files
as submitted — `cst/annotated.rs`, `cst/document.rs`, and the 185-line
`tests/cst_crlf_splices.rs` — with no review changes requested. There is
therefore no divergence between what yqr verified on the PR branch and what the
published crate does, which is what makes §4's earlier cross-check still valid
as evidence.

## 3. Work items

### 3.1 Pin bump

`Cargo.toml:47` and the comment above it, then `cargo check` to refresh
`Cargo.lock`.

**Done.** Lockfile-only fallout: no transitive crates added or removed (0.0.21's
`hashbrown` / `libm` stay), and upstream MSRV holds at 1.86.0 against yqr's
pinned 1.97.1 toolchain. The lockfile checksum
`213c922c1762f1e25cdd85773b2a7968b96e5b88058265733a847e2be3b589ce` matches the
SHA256 published on the v0.0.22 release for `noyalib-0.0.22.crate`, so the
resolved artifact is the released one.

### 3.2 Delete the `yqr-b009` workaround

`src/fidelity/write.rs` loses four things:

- the `crlf: Vec<bool>` field on `NoyalibWriter` and its doc comment
- the `is_all_crlf` call in `open`, which computed it per document
- the `zip` / restore branch in `emit`, which is now a plain concatenation
- the `is_all_crlf` and `restore_crlf` free functions

The file goes 66 lines deleted against 20 added, and all but the §3.3 test edits
are production code. `emit` gains a doc comment saying what it no
longer does and why, because "this used to post-process and deliberately
stopped" is the kind of fact a future reader is otherwise likely to re-derive
the hard way.

The argument for removing it rather than keeping it as belt-and-braces is
`yqr-b009` §6's: the workaround was a pass over the emitted string that
second-guessed the engine's line endings. Two mechanisms agreeing today is not
a reason to keep both — it is one more place to disagree the next time either
side changes. The restore was also *exact* only under an assumption that no
longer holds (that every bare `\n` in the output of a wholly-CRLF document came
from an edit); now that the engine emits CRLF at the edit site, the pass is a
no-op that still walks every byte of every edited document.

### 3.3 What the five CRLF tests now pin

They stay, unchanged in their assertions, and change meaning: they pinned
yqr's own pass, and now pin the engine's behaviour. One rename —
`an_lf_document_is_untouched_by_the_crlf_restore` → `an_lf_document_stays_lf`,
since there is no restore left to be untouched by.

That they are the *only* thing guarding this property is worth stating, and the
test comment now does: `tests/corpus_validation.rs` and `tests/fidelity.rs`
never edit a CRLF document, so a regression here would be invisible to both.
This is the same argument `yqr-f014` §5 made for pinning the two `Emit` probes
whose fix arrived through `Cargo.toml` — a fix yqr consumes but does not own
needs a yqr-side assertion, or nothing local catches its return.

### 3.4 Upstream close-out

`yqr-b009` §6 moves from "fixed upstream by noyalib#261 (open)" to released,
and drops its "delete it when upstream lands" instruction, which is now carried
out. `yqr-b004` §6.4 and `yqr-f013` §6 are unaffected — they track #226, which
shipped in 0.0.19.

## 4. Verification — the control, run against the published crate

The claim that needed testing is not "0.0.22 preserves CRLF" but "0.0.22
preserves CRLF *well enough that yqr's workaround is redundant*", and the only
way to see that is to remove the workaround and check what fails.

**Before removal**, on the published 0.0.22: the full suite passes. Expected —
`restore_crlf` is idempotent over output that is already correct, so the bump
alone is invisible.

**After removal**, the two runs that matter:

| Pin | `cargo test --lib fidelity::write` |
|---|---|
| `0.0.22` | 48 passed, 0 failed |
| `0.0.21` (temporary, control) | 45 passed, **3 failed** |

The three that fail on 0.0.21 are `inserting_a_key_keeps_a_crlf_document_crlf`,
`appending_an_item_keeps_a_crlf_document_crlf`, and
`a_multiline_insert_into_a_crlf_document_uses_crlf_throughout` — exactly the
three the PR's cross-check predicted, reproduced here against crates.io rather
than a branch. The other two (`an_lf_document_stays_lf`,
`a_mixed_ending_document_is_left_alone`) pass on both pins, because both are
`\n`-default cases that neither the workaround nor the upstream fix alters.

So the deletion is load-bearing-free by measurement: the property is still
enforced, and it is the engine enforcing it.

Full suite on 0.0.22 with the workaround gone: **262 tests, 0 failed** (163
lib + 99 across the integration targets), and `bash .github/scripts/local-ci.sh`
clean including `cargo audit`.

## 5. Upstream status — the #221 correction was accepted

`yqr-f014` §4 recorded a disagreement: the maintainer's 2026-08-11 status update
on [noyalib#221](https://github.com/sebastienrousseau/noyalib/issues/221) listed
comment mutation, extended `remove` and fragment quoting as still open, and yqr
replied on 2026-08-13 with per-item evidence from the published crates that all
three had shipped.

**That reply was accepted.** The maintainer's
[2026-08-14 update](https://github.com/sebastienrousseau/noyalib/issues/221#issuecomment-5297805246)
now records gaps 1 (comment mutation, 0.0.21), 2 (`rename_key`, 0.0.18) and 3
(sequence reorder) as shipped, and re-scopes 4 and 5 as *partial* rather than
open. `yqr-f014` §4's finding therefore stands as settled rather than contested,
and `yqr-b004`'s status-tracker entry no longer needs to flag the update as
inaccurate.

Two gaps remain open upstream, neither blocking yqr:

- **Gap 5 — `Emit` is not wired into the fragment mutators.** `set` /
  `insert_entry` / `push_back` still splice a caller's string verbatim, so a
  fragment containing a `:` or a leading `-` can restructure a document in a way
  the guard cannot catch (it rejects *invalid* YAML, not
  *valid-but-misinterpreted* YAML). Upstream tracks this as roadmap item A4 and
  calls it the last correctness hazard. **yqr is not exposed**: `yqr-f014` §3.2
  moved every call to the typed `_value` tier, and the module doc in
  `src/fidelity/write.rs` records that as a standing rule. This is a
  contribution opportunity on the #222 / #223 / #226 / #261 pattern, not a yqr
  need.
- **Gap 4 remainder — `remove_subtree`, sole-entry and flow-member removal.**
  The maintainer's standing offer to take yqr's `replace_span` fallback upstream
  is repeated, and remains moot as written for the reason `yqr-f014` §4 gave:
  upstream `remove` has handled multi-line and nested shapes since 0.0.18, and
  what differed was trivia, which yqr's noyalib#226 already fixed. What is
  genuinely unbuilt — sole-entry and flow deletes — **yqr also refuses**, so
  there is no yqr-side driver and nothing to port.

## 6. Acceptance criteria

- [x] `Cargo.toml` pins `noyalib = "0.0.22"`, the adjacent comment names 0.0.22
      and says what it brings, and `Cargo.lock` is refreshed.
- [x] The `yqr-b009` workaround is gone from `src/fidelity/write.rs`: no `crlf`
      field, no `is_all_crlf`, no `restore_crlf`, and `emit` is a plain
      concatenation.
- [x] The five CRLF tests survive the removal with their assertions unchanged,
      and the comment above them states that they now pin the engine.
- [x] The control is recorded: the same three tests fail against a temporary
      0.0.21 pin with the workaround removed (§4).
- [x] `bash .github/scripts/local-ci.sh` passes, including `cargo audit`.
- [x] `tests/fidelity.rs` and `tests/corpus_validation.rs` pass unchanged.
- [x] `yqr-b009` records noyalib#261 as merged unmodified and released in
      0.0.22, and no longer carries the delete-when-upstream-lands instruction.
- [x] `yqr-f014` §4 records that the #221 correction was accepted.
- [x] `CHANGELOG.md` records the bump and the workaround removal.

## 7. Non-goals

- No new filter grammar, and no change to the write surface. The bytes yqr
  produces are identical before and after; only which layer produces them
  changes.
- No behaviour change to the read path or `--normalize`.
- Not a release. `yqr-m001` governs when a version ships; `cargo publish` stays
  separately authorized (`yqr-m004`).
