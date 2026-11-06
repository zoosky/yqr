# Feature f047 — Put JSON Schema validation behind a `schema` cargo feature

**Status:** Done (2026-11-07)
**Epic:** Library surface and wasm (`f046`–`f050`)
**Owner:** yqr maintainers
**Related:** `yqr-f040` (the schema validation this gates), `yqr-f046`
(the same cut for the CLI), `yqr-f049` (the wasm build whose size this
is for)

## Why

`jsonschema` is the heaviest slice of yqr's dependency graph: it pulls
`regex`, `fancy-regex`, `referencing`, `fluent-uri`, and
`email_address`. Everything else yqr's lib needs is small (noyalib,
indexmap). A browser editor's lint loop needs `validate::check_str` —
syntax and strict-mode diagnostics — not JSON Schema validation, so a
wasm bundle should be able to leave the whole stack out. The 2026-11-06
wasm compile check confirmed `jsonschema` 0.58 does build for
`wasm32-unknown-unknown`, so this is a size cut, not a correctness
need.

## What

1. Add a `schema` cargo feature, on by default:

   ```toml
   [features]
   default = ["cli", "schema"]
   schema = ["dep:jsonschema", "dep:serde_json"]
   ```

2. Gate `src/validate/schema.rs` and the `schema` parameter of
   `validate::check` on the feature. `check_str` stays unconditional.
3. The CLI's `validate --schema` flag requires the feature. The shipped
   binary carries `default`, so nothing changes for users.
4. Decide the signature cut so the no-schema lib is not a different
   API shape: prefer keeping `check(source, strict, schema:
   Option<&Schema>)` and gating only the `Schema` type and its
   constructor, over a `cfg`-dependent arity.

## Decisions (2026-11-07, as landed)

- **Signature cut.** `check(source, strict, schema: Option<&Schema>)`
  keeps its shape and is gated whole: without the feature the function
  does not exist, so no build ever sees a different arity. The shared
  logic lives in a private `base_checks`, which `check_str` (always
  present) and `check` both call. The `Code` enum keeps `Y201`–`Y203`
  unconditionally — codes are a stable contract, not feature surface.
- **`cli` implies `schema`.** A built binary always carries full
  validation, so the `validate --schema` flag needs no `cfg` in the CLI
  layer and no untested `cli`-without-`schema` combination exists.

## Acceptance criteria

- [x] `cargo check --lib --no-default-features` compiles with neither
      `jsonschema` nor `serde_json` in `cargo tree` (17 crates remain
      in the `-e normal` graph, down from ~90).
- [x] `cargo test` with default features passes unchanged, including
      every `validate --schema` CLI case.
- [x] The CI quality gate covers both feature states of the lib: the
      default state through every build/test step, the no-default state
      through its own clippy (`-D warnings`) and lib-test runs, plus a
      compile check of the schema-only editor profile
      (`--no-default-features --features schema`). The dependency guard
      refuses `clap`, `jsonschema` and `serde_json` in the minimal graph
      and fails on a grep error instead of passing vacuously.
- [x] The wasm size delta is measured and recorded below.

## Measured wasm size (2026-11-07)

A `cdylib` shim over the lib (`opt-level = "z"`, fat LTO, strip, no
`wasm-opt`), exporting one probe per entry point (`eval_str`,
`check_str`, `fidelity::run`, and with the feature also
`Schema::compile` + `check`), built for `wasm32-unknown-unknown`
against this commit:

| Feature state | `.wasm` size |
|---------------|--------------|
| `--no-default-features` | 351,679 bytes (344 KiB) |
| `--no-default-features --features schema` | 2,425,015 bytes (2.31 MiB) |

The `schema` feature is 85% of the editor-profile bundle; leaving it
out is the difference between a module that loads instantly and one
that does not.
