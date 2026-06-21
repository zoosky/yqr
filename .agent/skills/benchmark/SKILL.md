---
name: benchmark
description: Run and analyze performance benchmarks. Use when working on performance-critical code or comparing changes.
---

# Performance Benchmarking

Run Criterion benchmarks and analyze performance.

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench -- markdown
cargo bench -- template
cargo bench -- cache
cargo bench -- e2e
```

## Comparing Against Baseline

```bash
# Save baseline before changes
cargo bench -- --save-baseline before

# Make your changes...

# Compare against baseline
cargo bench -- --baseline before
```

## Key Benchmarks

| Benchmark | File | What It Measures |
|-----------|------|------------------|
| markdown | `benches/markdown.rs` | Markdown-to-HTML conversion |
| template | `benches/template.rs` | MiniJinja template rendering |
| cache | `benches/cache.rs` | Cache read/write operations |
| e2e | `benches/e2e.rs` | Full request-response cycle |

## Performance Guidelines

- No regressions in hot paths (>5% slower requires justification)
- Markdown rendering: target <1ms for typical pages
- Cache hits: target <100μs
- Template rendering: target <500μs

## When to Use

- Before merging performance-sensitive changes
- When optimizing code
- To establish baseline before refactoring
- During performance investigations
