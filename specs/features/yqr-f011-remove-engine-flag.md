# Feature f011 — Remove `--engine`: noyalib is the only engine

**Status:** Done
**Epic:** Fidelity-first architecture (a001)
**Owner:** yqr maintainers
**Related:** `yqr-m005` (single-engine consolidation — this closes its §6
"engine plurality" provision), `yqr-f004` (engine parity — superseded; the
flag itself is removed here), `yqr-f009` (which had repurposed `--engine`
for the default read), `yqr-m002` (the internal engine seam, which stays)

## 1. Problem

`yqr-m005` consolidated on noyalib as the sole YAML engine but deliberately
kept the `--engine` seam pluggable: `BackendId` listed `NoyalibCst` plus a
`Skald` placeholder that resolved by name and then reported it was only
built on the `feat/skald-engine` branch. That hedge is no longer wanted —
alternate engines are not needed. What remained was pure surface area:

- a CLI flag with exactly one valid value, whose only observable behaviors
  were a no-op (`--engine noyalib`) and an error (anything else);
- a `BackendId` enum, name parser, and two dispatch `match`es whose skald
  arms were unreachable-by-design error returns;
- per-backend plumbing in the corpus (`Engine`, `engines:` per case), the
  benches, and the tests.

## 2. Change

Remove the runtime engine selection end to end; noyalib is wired directly.

- **CLI:** the `--engine <ENGINE>` flag is removed. clap now rejects it like
  any unknown flag (usage error), mirroring the retired `--preserve`.
- **Library API (breaking):** `fidelity::BackendId` is deleted;
  `fidelity::open(input)`, `fidelity::run(filter, input, raw)`,
  `fidelity::run_ast(ast, input, raw)`, and
  `fidelity::write::apply(mutation, input)` lose their backend parameter.
  The `FidelityEngine` trait drops `backend_id()`.
- **Seam kept, choice point gone:** the object-safe `FidelityEngine` /
  `FidelityWriter` traits remain as the boundary between the driver and the
  engine's API surface (`yqr-m002`); what is gone is the runtime choice.
- **skald retired on `main`:** the placeholder arm, its error message, and
  the manifest note pointing at `feat/skald-engine` are removed. The name is
  no longer recognized anywhere.
- **Corpus/benches:** `EngineCase` loses its per-backend `engines` field;
  the corpus `Engine` enum and the backend-mapping helpers in
  `tests/corpus_validation.rs` and `benches/corpus_bench.rs` are gone.
  Benchmark names (`corpus/engine_all`, `corpus/scale_engine_identity`) are
  unchanged, so the `gh-pages` baseline history stays comparable.
- **Docs:** README usage/options, the site's fidelity callout, and the demo
  (README + `yqr-demo.sh` section 6) no longer mention `--engine`;
  CHANGELOG records the removal under Unreleased.

## 3. Acceptance criteria

- [x] `yqr --engine noyalib '.'` fails argument parsing (no silent no-op);
      covered black-box (`tests/cli.rs`) and at the clap layer
      (`src/cli.rs`), mirroring the `--preserve` removal tests.
- [x] No `BackendId` or `--engine` reference remains in `src/`, `tests/`,
      `benches/`, `README.md`, or `docs/content/` (traceability comments and
      the removal tests themselves excepted).
- [x] Default read, `--normalize`, and the write tier behave identically to
      before (no behavioral change without the flag): full test suite green.
- [x] Benchmark ids are unchanged so the continuous-benchmark baseline
      remains comparable.
- [x] `yqr-m005` §6 and the feature tracker reflect the collapse.
