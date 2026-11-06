# Feature f046 — Put the CLI's dependencies behind a `cli` cargo feature

**Status:** Draft
**Epic:** Library surface and wasm (`f046`–`f050`)
**Owner:** yqr maintainers
**Related:** `yqr-f047` (the same cut for schema validation), `yqr-f049`
(the wasm builds that want the smaller graph), accentcms's library
adoption (tracked there as `f430`)

## Why

`clap` is an unconditional dependency, but only the binary uses it:
`src/main.rs` declares `mod cli`, and no module under `src/lib.rs`
imports clap. A library consumer — accentcms evaluating filters
server-side, or a wasm build for a browser editor — compiles `clap`,
`clap_derive`, and their proc-macro stack for nothing. Measured on
2026-11-06: `cargo check --lib --target wasm32-unknown-unknown`
succeeds on 0.11.0 but builds clap and clap_derive along the way.

## What

1. Add a `cli` cargo feature, on by default so `cargo install yqr` and
   every existing workflow are unchanged:

   ```toml
   [features]
   default = ["cli"]
   cli = ["dep:clap"]

   [dependencies]
   clap = { version = "4.6.1", features = ["derive"], optional = true }

   [[bin]]
   name = "yqr"
   path = "src/main.rs"
   required-features = ["cli"]
   ```

2. `cargo check --lib --no-default-features` compiles without clap in
   the graph (verify with `cargo tree --no-default-features`).
3. The local gate and CI get a `--no-default-features` lib check, so a
   later change cannot quietly re-couple the lib to a CLI dependency.
4. docs.rs builds with all features (`[package.metadata.docs.rs]
   all-features = true`), so the documented surface does not shrink.

## What does not change

- The published binary: `default` carries `cli`, so release builds and
  `cargo install` are identical.
- The lib API: no module moves; `src/cli.rs` stays where it is, since
  only `main.rs` includes it.

## Acceptance criteria

- [ ] `cargo check --lib --no-default-features` passes, and `cargo
      tree --no-default-features` shows no `clap`.
- [ ] `cargo build` (default features) produces the binary as before;
      the CLI suite passes unchanged.
- [ ] `cargo package --list` is unchanged except for `Cargo.toml`.
- [ ] The CI quality gate runs the no-default-features lib check.
