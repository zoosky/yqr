---
name: local-bench
description: Run cargo benchmarks locally and publish results to the gh-pages dashboard. Use after performance work to record local MacBook Pro benchmark data.
---

# Local Benchmark Collection

Run all Criterion benchmark suites on the local dev machine and publish results
to a dedicated GitHub Pages dashboard, separate from CI benchmark data.

## Dashboards

| Source          | URL                                                        |
| --------------- | ---------------------------------------------------------- |
| CI (ubuntu)     | https://zoosky.github.io/accent/dev/bench/index.html       |
| Local (MacBook) | https://zoosky.github.io/accent/local-dev/bench/index.html |

## Running

### Full run (benchmark + publish to gh-pages)

```bash
./scripts/local-bench.sh
```

### Dry run (benchmark only, no push)

```bash
./scripts/local-bench.sh --dry
```

### Re-publish existing results

```bash
./scripts/local-bench.sh --skip
```

## Flags

| Flag     | Effect                                                   |
| -------- | -------------------------------------------------------- |
| `--dry`  | Run benchmarks but do not push to `gh-pages`             |
| `--skip` | Skip benchmarks, reuse existing `/tmp/bench-results.txt` |

## How It Works

1. Runs `cargo bench` with `--output-format bencher` across all suites
   (cache, content, e2e, markdown, template)
2. Parses bencher-format output into JSON via Python 3
3. Checks out `gh-pages` in a temporary git worktree
4. Merges new entry into `local-dev/bench/data.js` (deduplicates by commit SHA)
5. Commits and pushes to `gh-pages`
6. Cleans up the temporary worktree

## When to Use

- After completing performance-sensitive changes, to record local numbers
- Before and after optimization work, to track improvements on real hardware
- Periodically to maintain a local performance baseline

## Key Files

| File                                                      | Purpose                |
| --------------------------------------------------------- | ---------------------- |
| `scripts/local-bench.sh`                                  | Main collection script |
| `specs/implementation/m003-local-benchmark-collection.md` | System specification   |

## Dependencies

- Python 3 (for JSON parsing, included on macOS)
- Git worktree support (Git 2.5+)
- Criterion 0.5 with bencher output format
