# Feature f002 — Fidelity read floor (`FidelityEngine` seam + noyalib backend)

**Status:** Done
**Owner:** yqr maintainers
**Last updated:** 2026-07-03
**Implements:** `yqr-m002` §4/§9 (read floor), `yqr-a001` §4.1 (slice-on-emit)
**Related:** `yqr-b001` (the bug this closes for the engine path), `yqr-r002` (backend evaluation), `yqr-f004` (superseded the off-by-default gating below: the noyalib backend is now on by default, runtime-switchable, and pinned to the `zoosky/noyalib` fork)

## 1. Summary

Implement the read-only half of the `yqr-m002` fidelity seam inside yqr:

- `src/fidelity/` — the backend-agnostic `FidelityEngine` trait plus its value
  types (`Span`, `Path`/`PathSeg`, `Resolved`, `Unaddressable`, `BackendId`)
  and the `open()` factory.
- A first concrete backend: **noyalib's CST** (backend C of `yqr-m002` §7.2),
  behind the existing `backend-noyalib` feature so the default build stays
  dependency-minimal.
- **Provenance threading** in the evaluator: each produced value carries the
  concrete path it was derived from (`None` for computed values), so emission
  can slice original bytes instead of re-serializing.
- A CLI switch (`--engine noyalib`) that routes execution through the fidelity
  engine. The default pipeline is unchanged.

With the engine selected, `yqr '.' f` reproduces `f` **byte-for-byte** (the
`yqr-a001` §2 north-star property), multi-document streams are no longer
silently truncated, and path projections emit the selected node's original
bytes (comments, quotes, indentation intact).

## 2. Deviation from m002 §9 (ordering)

`yqr-m002` §9 planned backend A (rust-yaml span layer) first. That layer now
lives upstream as [rust-yaml#73](https://github.com/elioetibr/rust-yaml/pull/73)
(open, unreleased), while noyalib 0.0.12 is **released and verified** (`yqr-r002`
update, BOM fix confirmed). So backend C ships the read floor first; backend A
becomes a thin adapter over the upstream API once #73 lands. Two further
refinements relative to the m002 canonical sketch:

- `PathSeg::Index(usize)` (resolved), not `Index(i64)`: yqr's evaluator already
  resolves negative indices against the live length, and it is the seam's only
  caller.
- The write tier (`FidelityEdit`, `Replacement`, `QuoteHint`, `Loaded`) is not
  declared yet — yqr has no assignment grammar. `open()` returns
  `Box<dyn FidelityEngine>` directly.

## 3. Semantics (per m002 §5)

| `resolve` outcome | Emission |
|---|---|
| `Found` at the **root** path | document slice **verbatim** (no added newline) — the identity/`cat` case |
| `Found` elsewhere | original bytes of the node, newline-terminated |
| `Absent` | `null` (jq semantics) |
| `Synthetic` / `Unaddressable` | lossy render of the typed value (visible fallback, per node) |
| no path (computed value) | lossy render of the typed value |

`--raw-output` keeps jq semantics: a top-level string result prints its
*value* (typed), not its quoted source bytes.

Multi-document streams: the filter runs against **every** document, in order;
identity output is the concatenation of the per-document slices (byte-equal to
the input).

## 4. Acceptance criteria

- [x] `src/fidelity/` module: trait + types + `open()`, documented, no feature
      IDs in doc comments.
- [x] `NoyalibEngine` behind `backend-noyalib`: `parse_stream`-based multi-doc,
      byte-offset rebasing, typed-value conversion (noyalib `Value` → yqr
      `Value`), resolve with `Found`/`Synthetic`/`Absent`/`Unaddressable`.
- [x] Evaluator threads `Option<Path>` provenance; existing `eval` behavior
      unchanged (all pre-existing tests pass).
- [x] CLI `--engine noyalib` (feature-gated; helpful error when not compiled
      in); default path untouched.
- [x] `yqr --engine noyalib '.' f == cat f` byte-for-byte across the b001
      corpus dimensions (comments, blanks, indent, quotes, block scalars,
      numbers, flow, key order, anchors/merge, CRLF, trailing ws, BOM,
      multi-doc), enforced in `tests/fidelity_engine.rs`.
- [x] Projections emit original bytes; computed/absent/unaddressable results
      degrade visibly to typed rendering.
- [x] Quality gates green in both feature profiles (fmt, clippy `-D warnings`,
      tests, bench compile).

## 4a. The wrong-node guard (added after adversarial review)

An adversarial review of the implementation confirmed that noyalib's span
resolver and its typed view apply different duplicate-key policies (first-wins
vs last-wins), and that spans can be degenerate (an implicit null's `:`
indicator) or value-lossy (`|+` kept blank lines excluded; `*alias` slices
dangle). One mechanism closes all four: every non-root `Found` is **verified**
— the slice (tried verbatim, then with its original leading columns restored so
block slices re-indent) must re-parse to exactly the typed value the evaluator
selected; on any disagreement the engine degrades to `Synthetic` (visible typed
fallback). The identity/root path is exempt (byte-exact by construction; held
on 45+ adversarial inputs).

A second (xhigh) review pass hardened the guard further:

- **Emitted == verified.** The guard originally verified a *padded*
  reconstruction but emitted the unpadded slice — a nested block sequence
  printed as `- alpha\n    - beta`, which downstream parsers silently re-parse
  as `["alpha - beta"]`. Now the span is **extended to the line start** when
  the prefix is pure indentation, so the emitted bytes are uniformly indented
  verbatim source and are verified in exactly the emitted form.
- **Key-collision refusal.** The engine's value model has string-only mapping
  keys; distinct YAML keys colliding after string conversion (`1` and `"1"`)
  would silently drop an entry. `open()` cross-checks collection entry counts
  against the default loader (best effort) and refuses loudly instead.
- **Per-document content check** in `open()` (not just summed lengths), and
  absent-node provenance unified across field/index in the evaluator.
- CI and the local mirror now run the test suite with `--all-features`, so the
  gated backend tests actually execute in the pipeline.

Known, documented limitations that remain (details and upstream asks in
`yqr-b002`): non-string keys are matched by spelling (filter results can
differ from the classic pipeline even without collisions); comments above a
block collection's first key belong to the parent's range; empty input emits
nothing (byte-identity) where the classic pipeline prints `null`; the noyalib
parser rejects a few inputs the default engine accepts (CR-only line endings);
anchor/tag property bytes (`&x 1`, `!!str 007`) are part of a node's slice by
design.

(The duplicate-key wrong-node hazard — `span_at` first-wins vs the last-wins
typed view — was resolved upstream in **noyalib 0.0.13** (b002 §2.1); yqr
consumed the bump, and a duplicate-key projection now emits the last
occurrence's real bytes.)

Review findings deliberately **not** code-changed (recorded follow-ups):
`run()` accumulates its output in a `String` rather than streaming to the
writer, and `value(doc)` returns an owned clone of the lowered tree — both
acceptable at CLI scale, revisit for large files (see `yqr-m002` §12). The
backend re-derives the typed node internally (`walk_value`) instead of taking
it from the caller: deliberate, so the `resolve` seam stays backend-agnostic.

## 5. Non-goals (deferred)

- Mutation (`FidelityEdit`) — no assignment grammar in yqr yet.
- Backend A (rust-yaml) — an adapter over upstream #73 when it merges/releases
  (`yqr-m002` §7.1 update); in-yqr token walk only if #73 is rejected.
  **Update (2026-07-04): shipped as `yqr-f003`** (the fork's `RoundTripDocument`
  behind `--engine rust-yaml`).
- Per-node fidelity warnings on stderr (candidate follow-up).
- `docs/content/` usage pages — the repo has no docs site yet; README carries
  the usage notes for now.
