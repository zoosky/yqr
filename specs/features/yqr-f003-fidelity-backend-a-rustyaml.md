# Feature f003 — Fidelity backend A (rust-yaml fork `RoundTripDocument` adapter)

**Status:** Done
**Owner:** yqr maintainers
**Last updated:** 2026-07-04
**Implements:** `yqr-m002` §7.1 (backend A), `yqr-a001` §4.1 (slice-on-emit)
**Related:** `yqr-f002` (the seam + backend C this parallels), `yqr-b001` (the
bug this closes on the engine path), `yqr-m002` (the adapter mapping),
`yqr-b003` (a fork parser limitation surfaced by this feature's review)

## 1. Summary

Add the second concrete fidelity backend — **backend A** — over our rust-yaml
fork's source-preserving `RoundTripDocument` API, exactly parallel to the
`f002` noyalib backend:

- `src/fidelity/rustyaml.rs` — `RustYamlEngine`, a `FidelityEngine` adapter over
  `rust_yaml_rt::RoundTripDocument` (`parse_all` multi-doc, `source()`,
  `value()`, `span_of(&[PathSegment])`), behind a new `backend-rust-yaml`
  feature so the default build stays dependency-minimal.
- `BackendId::RustYamlRoundTrip` + `open()` dispatch + the CLI switch
  `--engine rust-yaml` (feature-gated, with a helpful error when not compiled
  in). The default pipeline and the noyalib backend are unchanged.
- A comparison backend in the round-trip harness (`tests/fidelity.rs`) and an
  end-to-end engine test file (`tests/fidelity_engine_rustyaml.rs`).

The fork is the **same crate** yqr already depends on from crates.io
(`rust-yaml` 1.1.0), brought in under the distinct name `rust-yaml-rt` from the
`feat/roundtrip-document` branch (which carries the additive `RoundTripDocument`
submitted upstream as rust-yaml#73). The two `Value` enums are structurally
identical, so lowering the fork's value into yqr's evaluation model is a
one-to-one structural map — the same shape as the noyalib lowering.

With `--engine rust-yaml`, `yqr '.' f` reproduces `f` **byte-for-byte** across
every `yqr-b001` corpus dimension, and path projections emit the selected
node's original bytes.

## 2. Why this is the m002 §7.1 adapter (not the in-yqr token walk)

`yqr-m002` §7.1 planned backend A as either (a) a thin adapter over the upstream
`RoundTripDocument` if rust-yaml#73 lands, or (b) an in-yqr token/span-index
walk if #73 is rejected. #73 is implemented on our fork and verified over the
yaml-test-suite (1,501 accepted inputs reproduced byte-for-byte, 0 violations,
per `yqr-b001` §8), so this feature ships path (a): the adapter. The span-index
construction lives in the fork, not in yqr.

Mapping onto the seam (`yqr-m002` §7.1):

| Seam method | Fork API |
|---|---|
| `source()` | owned input `String`, verbatim |
| `doc_count` / `doc_span` | `parse_all` document slices, rebased by cumulative byte offset |
| `value(doc)` | `RoundTripDocument::value()` lowered `rust_yaml_rt::Value` → `rust_yaml::Value` |
| `resolve(doc, path)` | root → whole-doc span; else `span_of(&[PathSegment])`; `None` → `Absent`/`Synthetic` via the typed view |
| `Unaddressable` | **never produced** — the fork addresses keys by resolved scalar text, so special-character keys resolve to their bytes |

## 3. How backend A differs from backend C (noyalib)

The fork's model is a closer fit for the seam, so several noyalib mitigations
are unnecessary here:

- **Full typed keys.** The fork keeps `Value` mapping keys (not string-only), so
  distinct keys that share a spelling (`1` and `"1"`) never collide — the
  entry-count collision guard the noyalib backend needs is dropped. Non-string
  keys are keyed by their full value, matching the classic pipeline (a
  string-spelled filter like `.true` misses a boolean `true:` key, as in jq).
- **Last-wins duplicates.** The span index and the typed value both resolve
  duplicate keys to the last occurrence, so a duplicate-key projection emits its
  real bytes (the noyalib backend degrades here because its span layer is
  first-wins while the typed view is last-wins).
- **Addressable special-character keys.** `.["a.b"]` emits the original quoted
  bytes rather than degrading to typed rendering.
- **Anchored scalars.** The fork indexes a scalar's value token separately from
  its `&anchor` property, so an anchored scalar projects its **value** bytes
  (the anchor label is dropped); the CST backend keeps the property bytes.

The wrong-node guard (`verified_found`: the emitted slice must re-parse to the
selected value, with block-collection spans extended to the line start so the
emitted bytes are uniformly indented) is retained unchanged from `f002` §4a —
it is cheap insurance and still fires on block-collection re-indentation.

## 4. Acceptance criteria

- [x] `src/fidelity/rustyaml.rs`: `RustYamlEngine` over `RoundTripDocument`
      (multi-doc via `parse_all`, byte-offset rebasing, value lowering, resolve
      with `Found`/`Synthetic`/`Absent`), documented, no feature IDs in doc
      comments.
- [x] `BackendId::RustYamlRoundTrip`, `open()` dispatch, CLI `--engine rust-yaml`
      (feature-gated behind `backend-rust-yaml`; helpful error when not compiled
      in); default path and the noyalib backend untouched.
- [x] `Cargo.toml`: `rust-yaml-rt` optional git dependency (fork branch
      `feat/roundtrip-document`), `backend-rust-yaml` feature; `Cargo.lock`
      committed.
- [x] `yqr --engine rust-yaml '.' f == cat f` byte-for-byte across the b001
      corpus dimensions, enforced in `tests/fidelity_engine_rustyaml.rs` and the
      `rust_yaml_round_trip_is_faithful` matrix test in `tests/fidelity.rs`.
- [x] Projections emit original bytes; merged/alias/implicit-null/computed
      results degrade visibly to typed rendering; missing paths → `null`.
- [x] Quality gates green in the default and `--all-features` profiles (fmt,
      clippy `-D warnings`, tests, bench compile).

## 5. Non-goals (deferred)

- Mutation (`FidelityEdit` / `set` / `replace_span`) — no assignment grammar in
  yqr yet. The fork exposes `set`/`replace_span`; wiring them is the write-tier
  feature, tracked when yqr grows an assignment grammar (`yqr-m002` §6.2).
- Replacing the default (crates.io) pipeline with the fork — the fork is added
  additively and off by default; the default build is unchanged.
- Per-node fidelity warnings on stderr (candidate follow-up, shared with f002).

## 6. Notes / deviations

- **Module name.** `yqr-m002` §8 sketched the file as `src/fidelity/rust_yaml.rs`.
  It is `src/fidelity/rustyaml.rs` instead: a `mod rust_yaml` would shadow the
  `rust_yaml` extern crate that `src/fidelity/mod.rs` imports (`use
  rust_yaml::Value`), so the underscore-free name avoids the collision.
- **Dependency naming.** The fork is pulled in as `rust-yaml-rt` (Cargo
  `package = "rust-yaml"` rename) so it coexists with the crates.io `rust-yaml`
  the default build uses; in code it is `rust_yaml_rt`.
- **Verification loader.** `verified_found` re-parses candidate slices with the
  fork's own `RoundTripDocument::parse`, so acceptance matches the pass that
  produced the spans.

## 7. Adversarial review outcomes

A multi-agent adversarial review (eight category finders against the real
binary, each finding independently verified) surfaced two defect classes:

- **Block re-indentation guard bypass (fixed here).** The wrong-node guard first
  tried the *raw* span and only extended to the line start on failure. Because
  the fork's own loader is lenient enough to accept some first-line-dedented
  block slices (e.g. `.a.b` on `a:\n  b:\n    c: 1\n    d: 2` yielded
  `c: 1\n    d: 2`, which PyYAML / Ruby Psych / go-yaml all reject), the guard
  waved through a slice that stricter downstream parsers reject. `verified_found`
  now **prefers the line-start-extended slice** whenever the node begins after
  pure indentation and never falls back to the mis-indented raw slice. Regression
  tests: `deeply_nested_block_mapping_projection_*`,
  `int_first_two_space_block_mapping_extends_to_line_start`,
  `anchored_block_mapping_projection_is_uniformly_indented`, and the e2e
  `deeply_nested_block_mapping_projection_reparses_uniformly`.
- **Trailing `...` after a block collection (fork bug, tracked as `yqr-b003`).**
  The fork's `parse_all` mis-accounts a phantom EOF boundary and errors on
  `a: 1\n...\n`. Not fixable in the adapter (the whole parse fails); documented
  as a limitation and pinned by
  `document_end_marker_after_block_is_a_known_limitation`, to be converted to an
  identity assertion when the fork is fixed.

Findings deliberately **not** treated as defects (documented/expected): an
anchored scalar projecting its value without the `&anchor` label; empty input →
empty output; non-string keys keyed by full value so a string-spelled filter
misses; merge/alias/implicit-null degrading to typed rendering.
