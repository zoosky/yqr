# Feature f047 — Put JSON Schema validation behind a `schema` cargo feature

**Status:** Draft
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

## Acceptance criteria

- [ ] `cargo check --lib --no-default-features` compiles with neither
      `jsonschema` nor `serde_json` in `cargo tree`.
- [ ] `cargo test` with default features passes unchanged, including
      every `validate --schema` CLI case.
- [ ] The CI quality gate covers both feature states of the lib.
- [ ] The wasm size delta (with and without `schema`) is measured once
      and recorded in this spec when the feature lands.
