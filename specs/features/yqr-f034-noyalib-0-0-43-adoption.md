# Feature f034 — Adopt noyalib 0.0.43: two lockstep releases, no source change

**Status:** Done — adopted 2026-09-11
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f031` (the 0.0.41 adoption this follows), `yqr-b032`,
`yqr-b033`, `yqr-b034` (open, measured on 0.0.41)

## 1. Scope

Bump `noyalib = "0.0.41"` to `0.0.43`, two releases published the same
day, and verify yqr against the published crate. `Cargo.lock` moves
noyalib's version and checksum and nothing else.

| upstream | release | what | yqr effect |
|---|---|---|---|
| ddc7c0a | 0.0.42 | lockstep release for the language server's VS Code extension: bundling, an icon, a licence the packager recognises, a Marketplace publish step; no core code change | none (§2) |
| 0d120de | 0.0.43 | lockstep release hardening that extension's publish step; no core code change | none (§2) |

The reason to adopt a release that changes nothing is to keep the pin on
upstream's latest, so the release that carries yqr's open fixes (§3) is a
one-step bump measured against a known base.

## 2. Measured

**The published crates.** A `diff -r` of the two crates.io sources under
`~/.cargo/registry/src/`, which is what yqr compiles, rather than of the
git tags. Five files differ, all packaging:

| file | difference |
|---|---|
| `Cargo.toml`, `Cargo.toml.orig` | `version` |
| `README.md` | the version in two install snippets |
| `Cargo.lock`, `.cargo_vcs_info.json` | lockfile and commit metadata |

Every file under `src/` is byte-identical, so yqr builds the same engine
code it built on 0.0.41. The upstream changelog's "no core code change" is
confirmed on the artifact rather than taken from the notes.

**The suite.** The bare bump passes `cargo test --all-targets --locked`,
546 tests, without an expectation moving; that includes the whole
`yqr-m003` corpus, every write case with its byte-identity assertion.

**The CLI.** `.`, `--normalize .`, `validate --strict`, `.zzz_new = 1` and
`-r keys` over the production values file (`tests/data/values.yaml`),
through a `main` build on 0.0.41 and this branch on 0.0.43, comparing
stdout, stderr and exit code: identical. Five comparisons is a small
sample; it confirms the source identity above rather than standing in for
it.

## 3. What 0.0.43 does not carry

None of yqr's four open upstream PRs is in a release. All four branch from
0.0.43 itself, so no published crate can contain them:

| PR | fixes | yqr spec |
|---|---|---|
| noyalib#422 | a replacement adopts the document's line break | `yqr-b030`, resolved in yqr by a refusal guard |
| noyalib#424 | a scalar written over a block collection keeps its indent | `yqr-b029`, resolved in yqr by a refusal guard |
| noyalib#426 | comments on an entry written with no value | `yqr-b031`, resolved in yqr; commenting such an entry is still refused |
| noyalib#427 | an insert stops at the last line its anchor entry owns | `yqr-b032`, open |

So `b032` and `b034` stay open, and every measurement the bug specs record
"on the pinned 0.0.41" holds for 0.0.43 unchanged, because the source they
measured is identical. One correction for the next adoption:
`yqr-b032`'s status reads "Fixed upstream the same day in noyalib#427";
on 2026-09-11 #427 is open and unmerged. Until it merges the fix is
proposed, not landed.

## 4. Benchmarks

Not run. The compiled engine source is byte-identical (§2), so a timing
difference between the two builds could only be machine noise. The
`yqr-f031` numbers stand for 0.0.43.

## 5. Acceptance criteria

- [x] The pin moves to `0.0.43`; `Cargo.lock` shows noyalib moving and
      nothing else.
- [x] The published sources compared: every file under `src/` identical
      (§2).
- [x] Full suite green on the bare bump, no expectation moved.
- [x] CLI output compared against the 0.0.41 build (§2): identical.
- [x] Benchmarks: not run, and why (§4).
- [x] `Cargo.toml` pin comment and `CHANGELOG.md` say what the bump
      bought; tracker updated; `target/doc-md` regenerated for 0.0.43.
- [x] `local-ci.sh` clean.
