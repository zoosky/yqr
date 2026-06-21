---
name: e2e-perf
description: Run Playwright-based HTTP performance tests against all content pages in dev and production modes.
---

# E2E Performance Tests

Run browser-level HTTP performance tests that measure real TTFB and response times
across all content pages.

## When to Use

- Before merging performance-sensitive changes to the HTTP stack
- When comparing dev mode vs production mode performance
- To establish a performance baseline after architectural changes
- When investigating latency regressions not visible in Criterion benchmarks

## Running Tests

```bash
cd tests/playwright
npm run test:perf
```

Override iteration count (default 5):

```bash
PERF_ITERATIONS=10 npm run test:perf
```

## Output

Results are written to `tests/playwright/results/` as JSONL:

- `perf-dev-<timestamp>.jsonl` - Dev mode measurements
- `perf-production-<timestamp>.jsonl` - Production mode measurements

Each line is a JSON object with: `url`, `status`, `ttfb_ms`, `response_ms`,
`body_bytes`, `iteration`, `mode`, `timestamp`.

The last line of each file is a summary with `avg_ttfb_ms`, `p50_ttfb_ms`,
`p95_ttfb_ms`, `p99_ttfb_ms`, and `total_duration_ms`.

## Comparing Runs

```bash
# View summary from latest production run
tail -1 tests/playwright/results/perf-production-*.jsonl | python3 -m json.tool

# Compare average TTFB between runs
grep '"summary"' tests/playwright/results/perf-production-*.jsonl
```

## Key Metrics

| Metric | Target (production) | Notes |
|--------|-------------------|-------|
| p50 TTFB | < 10ms | Median request latency |
| p95 TTFB | < 50ms | Tail latency |
| p99 TTFB | < 200ms | Worst-case latency |

## Architecture

- Uses port 3200 (no conflict with parity tests on 3100/3101)
- Runs dev mode first, then production mode sequentially
- First request per server start is recorded as warmup (iteration 0)
- Warmup requests are excluded from summary statistics
