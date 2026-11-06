# Feature f045 — Adopt noyalib 0.0.58: a hardening release, nothing observable moves

**Status:** Done — shipped 2026-11-06
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f043` (the 0.0.57 pin this bump replaces), `yqr-b025`
(whose configuration-retention contract the release hardens),
`yqr-m001` (quality gates)

## 1. What the release carries

noyalib 0.0.58 (2026-10-15) is a hardening release with no API change
on any path yqr calls:

- **Hardened parser, CST and loader boundaries.** `cst::Document` edits
  re-parse under the document's own configuration (the contract yqr has
  leaned on since 0.0.31's noyalib#373, now enforced against a
  duplicate-key bypass through YAML 1.1 booleans), CST parses and edits
  run `Policy::check_value`, and `!include` / `${...}` expansion counts
  against `max_total_scalar_bytes`. yqr uses neither includes nor
  expansion, and its write path already parsed and re-parsed under one
  configuration.
- **Located unknown and missing field errors** from `from_str` — the
  typed deserializer path, which yqr does not call (values cross the
  boundary as yqr's own `Value`).
- **MSRV 1.89.0** across the noyalib family. yqr pins the 1.97.1
  toolchain, comfortably above the floor.
- **`serde_yaml_ng` retired from upstream's dev-dependencies** — a
  change to noyalib's own test harness, invisible to consumers.

## 2. What moved in yqr, measured

Nothing. `cargo update -p noyalib` moved exactly one package in the
lockfile — no new transitive dependency. The full gate
(`local-ci.sh`: fmt, clippy, build, both test passes, bench compile,
doc, package contents, audit) passes with **zero expectations flipped**:
all 621 tests, including every corpus, CLI, fidelity and values pin,
are byte-identical on 0.0.58. The policy checks now running on CST
parses and edits do not trip on the 221-tenant values file (`b025`'s
shape), whose ratio heuristic stays disabled through
`parse_stream_with_config`.

## 3. Acceptance criteria

- [x] Pin at 0.0.58; the lockfile moves only noyalib.
- [x] `local-ci.sh` clean with no pinned expectation flipped.
- [x] The `b025` values shape still reads, validates and writes under
      the hardened CST policy path.
