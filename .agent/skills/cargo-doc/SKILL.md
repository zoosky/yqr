---
name: cargo-doc-md
description: Look up Rust crate documentation in Markdown format from target/doc-md/.
---

## When to Use

Use this skill when you need to look up API signatures, types, or usage examples
for any Rust crate used in this project. Prefer local docs in `target/doc-md/`
over training data or web lookups -- they match the exact versions in `Cargo.lock`.

## Looking Up Documentation

Docs are organized as one directory per crate with Markdown files per module:

```
target/doc-md/
  index.md                    # Master index of all crates
  axum/index.md               # Crate root docs
  axum/routing.md             # axum::routing module
  tokio/sync/index.md         # tokio::sync module
  serde_json/index.md         # serde_json crate root
```

To find docs for a crate, read `target/doc-md/<crate_name>/index.md`.
For a specific module, read `target/doc-md/<crate_name>/<module>.md`.
Hyphens in crate names become underscores in directory names (e.g., `tower-http` -> `tower_http`).

## Regenerating Docs

Docs should be regenerated when `Cargo.lock` is newer than `target/doc-md/index.md`,
which means dependencies were updated.

### Full regeneration (all dependencies, including private items)

```bash
cargo +nightly doc-md --include-private
```

### Targeted regeneration (specific crates, faster)

```bash
cargo +nightly doc-md --include-private -p <crate1> -p <crate2>
```

### First-time setup (if cargo-doc-md is not installed)

```bash
rustup install nightly
cargo +nightly install cargo-doc-md
```

## Key Crates in This Project

| Crate | Purpose | Doc Path |
|-------|---------|----------|
| axum | HTTP framework | `target/doc-md/axum/` |
| tokio | Async runtime | `target/doc-md/tokio/` |
| minijinja | Template engine | `target/doc-md/minijinja/` |
| pulldown-cmark | Markdown parser | `target/doc-md/pulldown_cmark/` |
| extism | WASM plugin runtime | `target/doc-md/extism/` |
| serde / serde_json | Serialization | `target/doc-md/serde/`, `target/doc-md/serde_json/` |
| notify | File system watcher | `target/doc-md/notify/` |
| moka | Concurrent cache | `target/doc-md/moka/` |
| toml | TOML parsing | `target/doc-md/toml/` |
| tracing | Structured logging | `target/doc-md/tracing/` |
| clap | CLI argument parser | `target/doc-md/clap/` |
| tower-http | HTTP middleware | `target/doc-md/tower_http/` |
