# Feature f049 — wasm target support: a CI lane and a bindings crate

**Status:** Done (2026-11-07)
**Epic:** Library surface and wasm (`f046`–`f050`)
**Owner:** yqr maintainers
**Related:** `yqr-f046` / `yqr-f047` (the feature gates that keep the
wasm graph small), `yqr-f048` (the diagnostics the bindings return),
`yqr-f050` (the typed edit API the bindings will expose next),
accentcms `f430` (the first consumer: the admin authoring editor)

## Finding: the lib compiles for wasm today

Measured on 2026-11-06 against 0.11.0 (noyalib 0.0.58): `cargo check
--lib --target wasm32-unknown-unknown` passes with no code change, no
feature flag, and no patched dependency. No lib module touches the file
system, the clock, or an RNG; even `jsonschema` (default features off)
compiles for the target. The property exists by accident, though —
nothing defends it. This feature turns it into a contract.

## What

1. **A CI lane that pins the property.** The quality gate gains
   `cargo check --lib --target wasm32-unknown-unknown` for the feature
   states that matter: `--no-default-features` (the editor profile,
   after `f046`/`f047`) and `--no-default-features --features schema`.
   A dependency bump or new lib code that breaks the target fails the
   PR, not the consumer.
2. **A `yqr-wasm` bindings crate**, a workspace member that is not
   published to crates.io initially (its artifact is the `.wasm` + JS
   glue, released as a build product). wasm-bindgen exports, kept to
   the editor's needs:
   - `eval(filter, yaml)` — `eval_str` + `render`, values out as JSON;
   - `fidelityRun(filter, yaml)` — byte-preserving execution,
     the edited document out as a string;
   - `validate(yaml, strict)` — diagnostics out as JSON (`f048`).
   Errors cross the boundary as structured objects, not thrown strings.
3. **Runtime proof, not just a compile check.** A `wasm-pack test
   --headless` (or Node-based) smoke run executes a representative
   corpus slice — one read, one fidelity edit, one validate with
   diagnostics — so a regression that compiles but misbehaves at run
   time is caught.
4. **A size budget.** Record the optimized (`wasm-opt -Oz`, or
   `opt-level = "z"` + lto) `.wasm` size for the editor profile in this
   spec when the crate lands, and gate CI on a generous ceiling so the
   size cannot drift unnoticed. The budget number is set from the first
   measurement, not guessed here.

## Why a separate crate

wasm-bindgen types and `#[wasm_bindgen]` exports do not belong in the
lib: the lib stays a pure-Rust API any host embeds (a server links it
natively — accentcms will — and only a browser needs the JS boundary).
The bindings crate is an adapter, free to track wasm-bindgen's cadence
without touching yqr's semver.

## As landed (2026-11-07)

- The CI lane checks three lib states for the target, not two: the
  `serde` state joined the list because it is the profile the bindings
  actually build (`default-features = false, features = ["serde"]`,
  from `f048`).
- The runtime proof runs in Node (`wasm-pack test --node`), not a
  headless browser — the spec allowed either, and Node is preinstalled
  on the CI runner. Five smoke tests cover a read, a byte-identical
  identity read, an edit that rewrites only its target bytes, validate
  with structured findings (code, position, byte offset), and a filter
  error crossing as a structured object with its span.
- Errors cross as an `ErrorEnvelope { kind, message, span }`; values
  and diagnostics serialize to plain JS objects
  (`serde-wasm-bindgen`'s json-compatible mode). An integer past JS's
  safe range (2^53 - 1) is refused (`kind: "data"`), not silently
  rounded, and so is a mapping keyed by a non-string — a defensive arm
  only, since the engine string-keys every parsed mapping (the smoke
  suite pins both the refusal and the string-keying).
- wasm-opt needed `--enable-bulk-memory` and
  `--enable-nontrapping-float-to-int`: current rustc emits those
  post-MVP features by default and wasm-pack's bundled wasm-opt
  refuses the module without the flags (set in `yqr-wasm/Cargo.toml`).
- `yqr-wasm` is a workspace member with `publish = false`, excluded
  from the published yqr crate; `yqr-wasm/pkg/` is git-ignored build
  output.

## Measured size and budget (2026-11-07)

`wasm-pack build --target web --release` (`-Oz` via wasm-opt, thin
LTO, strip from the workspace release profile), editor profile:

| Artifact | Size |
|----------|------|
| `yqr_wasm_bg.wasm`, optimized | 676,767 bytes (661 KiB) |
| CI ceiling | 900,000 bytes |

The bindings cost ~330 KiB over `f047`'s bare-lib shim (344 KiB):
wasm-bindgen glue, the serde facade, and serde-wasm-bindgen.

## Acceptance criteria

- [x] CI checks `wasm32-unknown-unknown` for the lib on every PR, in
      the feature states named above (plus `serde`).
- [x] `yqr-wasm` builds with wasm-pack and exposes `eval`,
      `fidelityRun`, and `validate`.
- [x] The smoke test runs the three calls against a corpus-shaped
      input inside a wasm runtime and asserts byte-identical fidelity
      output (Node-based; see "As landed").
- [x] The optimized size for the editor profile is recorded in this
      spec, and CI fails when the 900,000-byte ceiling is crossed.
