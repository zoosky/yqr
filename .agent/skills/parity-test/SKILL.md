---
name: parity-test
description: Run all serve-vs-build parity tests, including pagination parity tests.
---

# Parity Tests

Run Playwright tests that verify HTML output is byte-identical between `accent serve`
and `accent build` static output.

## When to Use

- After modifying template rendering, build command, or serve handlers
- After changing content collections or pagination logic
- Before merging any PR that touches `src/render/`, `src/commands/build.rs`, or templates
- When adding new content pages to verify they appear in both modes

## Running Tests

### All parity tests (default content)

```bash
cd tests/playwright
npm test
```

Uses default site content on ports 3100 (serve) / 3101 (static build).

### Pagination parity tests (test content)

```bash
cd tests/playwright
npm run test:pagination
```

Uses test content (`site/content/test/`) on ports 3102 (serve) / 3103 (static build).
Tests paginated blog pages with `content.limit: 2` producing 3 pages.

### Run both

```bash
cd tests/playwright
npm test && npm run test:pagination
```

## What Gets Tested

### Default parity (`npm test`)

- HTML byte-parity for all main site pages
- Root index.html redirect parity
- CSS asset parity
- No dev artifacts in production serve
- Structural integrity (200 status, DOCTYPE)

### Pagination parity (`npm run test:pagination`)

- HTML byte-parity for all test content pages (home, features, demo, blog, 5 blog posts)
- Paginated page parity: serve `/blog/page:N` vs build `blog/page/N/index.html` read from disk
- Pagination structure: correct page numbers, prev/next links
- Pagination content: correct posts per page in descending date order
- Build output: all expected paginated files exist on disk

## Architecture

- Default parity uses ports 3100/3101
- Pagination parity uses ports 3102/3103
- No conflict with performance tests (port 3200)
- Paginated pages are compared via direct file read (npx serve cannot resolve colon URLs)
