---
name: benchmark
description: Run and analyze performance benchmarks. Use when working on performance-critical code or comparing changes.
---

# Performance Benchmarking

Run the Criterion benchmarks and analyze performance.

## Running Benchmarks

```bash
# Run the whole suite
cargo bench

# Run a single benchmark by name filter
cargo bench --bench eval -- parse
cargo bench --bench eval -- eval_str/field_access
cargo bench --bench eval -- eval_str/iterate_100
```

## Comparing Against Baseline

```bash
# Save a baseline before changes
cargo bench --bench eval -- --save-baseline before

# Make your changes...

# Compare against the baseline
cargo bench --bench eval -- --baseline before
```

## Benchmarks

| Benchmark group | File | What it measures |
|-----------------|------|------------------|
| `parse/*` | `benches/eval.rs` | Compiling a filter (`parser::parse`) |
| `eval_str/*` | `benches/eval.rs` | End-to-end `eval_str` (parse filter + load YAML + evaluate) |

## Performance Guidelines

- No regressions in hot paths (>5% slower requires justification)
- Keep filter parsing allocation-light
- Prefer iterating over buffering when handling large documents

## When to Use

- Before merging performance-sensitive changes
- When optimizing the lexer / parser / evaluator
- To establish a baseline before refactoring
