# Feature f049 — wasm target support: a CI lane and a bindings crate

**Status:** Draft
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

## Acceptance criteria

- [ ] CI checks `wasm32-unknown-unknown` for the lib on every PR, in
      the two feature states named above.
- [ ] `yqr-wasm` builds with wasm-pack and exposes `eval`,
      `fidelityRun`, and `validate`.
- [ ] The headless smoke test runs the three calls against corpus
      inputs and asserts byte-identical fidelity output.
- [ ] The optimized size for the editor profile is recorded in this
      spec, and CI fails when the ceiling is crossed.
