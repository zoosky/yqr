# Feature f018 — Adopt noyalib 0.0.24: the sole-entry head comment, and what is left of the delegation question

**Status:** Done — 0.0.24 adopted, §4 measured, §5 decided (2026-08-19)
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f016` (the 0.0.23 adoption this succeeds, and whose §5.2
argument this release retires), `yqr-f007` §5.1/§6 (the delete-delegation
decision this re-measures), `yqr-b006` (the trivia class the fix belongs to),
`yqr-b014` (what the re-measurement found), `yqr-m003` (the corpus write tier
that runs against the new pin), `yqr-m004` (crates.io publishing)

## 1. Scope

Bump `noyalib = "0.0.23"` to `0.0.24` and re-run the one question the release
changes the premise of: whether yqr's **sole-entry** delete should now be
delegated to `Document::remove`, which this release makes agree with
`delete_entry` on the case that decided it last time.

**In scope:** the pin bump; verifying the fix against the published crate
(§3); re-running the `yqr-f016` §5 sole-entry decision (§4); recording the
outcome (§5).

**Out of scope:** the flow-member class, already delegated by `yqr-f016` §5 and
untouched by this release. The `yqr-a002` comment/rename slices — nothing here
touches the comment mutators, `rename_key`, or the addressing grammar, so
`yqr-b012`'s insert-anchor blocker and `yqr-b013`'s quote-style vote stand as
measured on 0.0.23. `yqr-b011`, which this release does not fix.

## 2. What 0.0.24 contains

One functional change, and it is yqr's report:

> `fix(cst): remove() takes a sole entry's head comment — closes #280`

Released 2026-08-18 as `noyalib v0.0.24`, crate checksum
`665102745aff776f400df932eaabb91860795fd6cff70ed9fc12c9298160e5eb`, which is
the checksum `Cargo.lock` now records. The rest of the release is CI and
supply-chain work (release-artefact signing, `codeql`/`taiki-e` action bumps)
plus a `hashbrown` bump, `0.15.5` to `0.17.1`.

**Attribution, stated precisely.** noyalib#280 was filed by yqr (`zoosky`,
2026-08-17) with the mechanism diagnosed — the two `Removal` arms deriving
their range differently, `Line` through `owned_entry_range` and so absorbing
the comment run, `SoleEntry` through the *collection's* span, which starts
below it. The commit that fixes it (`29c0739`) is the maintainer's, and its
message credits the report. yqr contributed the diagnosis, not the patch; the
four earlier upstream fixes yqr *did* author are listed in `yqr-f016` §6.

The `hashbrown` bump is visible in the lockfile and is an improvement: noyalib
was the last crate pinning `0.15.5`, so the duplicate is gone and the graph
loses a crate rather than gaining one.

## 3. Verified against the published crate

Probed directly against 0.0.24, on the four shapes `yqr-f016` §4.4 measured:

| Input | 0.0.23 | 0.0.24 |
|---|---|---|
| sole entry with a head comment | comment stranded above `{}` | **taken with the entry** |
| sole entry, two contiguous comment lines | stranded | **taken** |
| sole entry, blank-**detached** comment | left in place | left in place |
| entry with a sibling | taken | taken |

That is `delete_entry`'s rule exactly, including the blank-detached exclusion
that is the easy half to get wrong. The `yqr-b006` trivia disagreement between
the two implementations is, on this class, gone.

## 4. The measurement, run 2026-08-19

`yqr-f016` §5 sent the sole-entry class to `delete_entry` for one stated
reason: upstream's path stranded the entry's head comment. That reason no
longer exists, so the decision is re-run rather than assumed — the same
discipline `yqr-f007` §6 imposes ("reopen only on a *new* argument"), applied
in the direction that costs yqr code rather than saves it.

**Method.** The sole-entry branch of `delete_entry` routed to
`Document::remove` on a throwaway patch, whole suite run, patch reverted. The
tree as committed contains only the pin bump.

### 4.1 Two failures, one shape

**242 of 244 lib tests pass**, and every integration suite passes untouched
(76 corpus incl. the new write tier, cli, fidelity, integration). Both
failures are the same shape — the sole item of a block sequence written at its
key's own column, the GitHub Actions `on:` / `- push` idiom:

| | yqr | upstream |
|---|---|---|
| `on:`<br>`- push`<br>`jobs: {}` | `on:`<br>`␣␣[]`<br>`jobs: {}` | `on:`<br>`[]`<br>`jobs: {}` |

An empty collection is a block-mapping **value**, so it must sit strictly
deeper than its key. Upstream writes it at the removed item's own column,
which for this layout is the key's column.

### 4.2 The failures are not assertion nitpicks

Checked against two independent implementations rather than argued from the
spec text:

```console
$ python3 -c 'import yaml; yaml.safe_load("on:\n[]\njobs: {}\n")'
yaml.scanner.ScannerError: while scanning a simple key
$ ruby -ryaml -e 'YAML.safe_load("on:\n[]\njobs: {}\n")'
Psych::SyntaxError: could not find expected ':' while scanning a simple key
```

Both accept yqr's spelling. So delegation would write, at **exit 0**, a file
that the rest of the YAML ecosystem cannot read — with `-i`, straight to the
user's disk.

Nothing in the loop notices: noyalib's parser accepts the shape, so upstream's
own guard passes, yqr's re-parse guard passes (it re-parses with noyalib), and
`yqr validate --strict` reports the file clean. That last one is a false
negative in yqr's own validator and is filed with the rest as `yqr-b014`.

This is the `yqr-b006`/`yqr-b010` failure class a fourth time, and the first
one found by *considering* a delegation rather than by shipping one.

## 5. The decision

**Sole-entry delete stays in `delete_entry`.** The reason has changed
completely, which is worth stating plainly rather than presenting the outcome
as unchanged:

- `yqr-f016` §5.2's reason — the head comment — is **retired**. Upstream is
  correct on it as of this release, and yqr no longer keeps anything by
  keeping its own path there.
- The new reason is §4.2: on the same-column layout, delegation produces YAML
  two libyaml-based parsers reject, silently. That is a strictly worse defect
  than the one just fixed, since a stranded comment at least leaves a readable
  file.

The `yqr-f007` §6 standing argument is unchanged and now has a fourth data
point: the independent implementation is the thing that made the divergence
visible, before it reached anyone.

**Revisit when** `yqr-b014` is fixed upstream. At that point the sole-entry
class has no measured divergence left, and the question becomes the general
one `yqr-f007` §6 already answers on its own terms (differential oracle,
alternative implementation, asymmetric trade) rather than a case-specific one.

## 6. Acceptance criteria

- [x] 0.0.24 published to crates.io; the pin moves and `Cargo.lock` shows
      noyalib moving, `hashbrown 0.15.5` dropping out, and nothing else.
- [x] The `#280` fix verified against the **published** crate, per shape (§3),
      not taken from the release notes.
- [x] §4's measurement run per test, and its two failures checked against an
      implementation that is not noyalib (§4.2).
- [x] §5's decision recorded, with the retired reason called out as retired.
- [x] Full suite green on the new pin with yqr's code unchanged; `cargo audit`
      clean; site build clean.
- [x] What §4.2 found filed as `yqr-b014`, both faces of it.
