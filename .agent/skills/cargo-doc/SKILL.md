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
  clap/index.md               # Crate root docs
  clap/builder.md             # clap::builder module
  rust_yaml/index.md          # rust_yaml crate root
  criterion/index.md          # criterion crate root
```

To find docs for a crate, read `target/doc-md/<crate_name>/index.md`.
For a specific module, read `target/doc-md/<crate_name>/<module>.md`.
Hyphens in crate names become underscores in directory names (e.g., `rust-yaml` -> `rust_yaml`).

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
| clap | CLI argument parser (derive) | `target/doc-md/clap/` |
| rust-yaml | YAML parser / emitter | `target/doc-md/rust_yaml/` |
| criterion | Benchmark harness (dev-dependency) | `target/doc-md/criterion/` |
