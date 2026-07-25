# yqr.f010 — Project website: Accent CMS site over docs/ and specs/, on GitHub Pages

**Status:** Done
**Epic:** Project website (f010)
**Owner:** yqr maintainers
**Related:** `yqr-m003` (the demo content served at `/demo`),
`docs/content/home.html` (served verbatim as the site home page),
`benchmark.yml` (shares the `gh-pages` branch with the site deploy),
`docs-pages.yml` (the retired raw `docs/content/` mirror workflow --
removed by this feature, its job subsumed by the Accent build)

## 1. Problem

yqr's public face was a single hand-styled HTML file
(`docs/content/home.html`) served raw from the `gh-pages` branch, plus a spec
tree that was only readable inside the repository. There was no navigable
website: no way to browse the feature/bug/architecture specs, no search, and
every visual change meant hand-editing embedded CSS.

## 2. Design

Build the website with [Accent CMS](https://github.com/AccentCMS/accent) — a
single-binary markdown CMS with a static build mode — from the content that
already exists in the repository, using the flat-file content model (a file is
a page; no wrapper folders, no required frontmatter):

- **Site root:** `docs/` holds `config.yaml`, `content/`, and the vendored
  theme under `themes/default/`.
- **Home page:** the hand-authored `docs/content/home.html` is served
  **verbatim** — copied to `output/index.html` after the build, keeping its
  own embedded design exactly as authored. It contains no root-absolute
  URLs, so sub-path serving needs no changes to it — the deployed file is
  byte-identical to the source.
- **Spec tree:** `../specs` is mounted at `/specs` via `content.mounts` and
  served as-is; each category directory carries a `README.md` index so it
  appears as a sidebar section. Relative `*.md` links between specs resolve
  to clean page URLs automatically.
- **Theme:** the Accent CMS default docs theme (sidebar, search, dark mode,
  code copy), vendored into `docs/themes/default/` and restyled with the
  home page's design system: the ink/brass/teal palette (dark mode is the
  home page's ink theme verbatim; light mode its paper card palette),
  Georgia serif display type, system sans body, JetBrains Mono code, and
  the brass-and-teal compass as logo/favicon. Sidebar rooted at `/specs/`;
  yqr footer.
- **Sub-path serving:** GitHub Pages serves the repo at
  `https://zoosky.github.io/yqr/`. Accent v0.23.0+ handles this natively:
  the deployment path prefix is derived from the `--base-url`/`site.url`
  path component (or set via `site.base_path`/`--base-path`), every
  internal URL is emitted prefixed, and `accent build` fails its built-in
  conformance check if an unprefixed root-absolute link slips through.
  The interim post-build rewrite script (`pages-path-prefix.sh`) that
  patched six URL surfaces by hand was deleted once this landed upstream;
  the vendored theme was updated to the upstream templates that route
  hardcoded paths through the `url()` helper.
- **Deploy:** `.github/workflows/pages.yml` fetches the pinned accent binary
  from the upstream GitHub release (checksum-verified), builds, and pushes
  the output to the `gh-pages` branch — preserving `dev/bench`, the
  Criterion benchmark dashboard that `benchmark.yml` publishes to the same
  branch. Pull requests touching `docs/`, `specs/`, or the workflow build and
  verify without deploying. The pre-existing `docs-pages.yml` workflow,
  which mirrored `docs/content/` raw onto `gh-pages`, is removed: its job is
  subsumed by the Accent build, and left in place it would race the deploy
  (no shared concurrency group) and keep resurrecting the retired raw
  `/docs/content/` paths that `pages.yml`'s `rsync --delete` clears.

Local development uses the same binary: `cd docs && accent serve` (or
`accent build --clean` plus the home-page copy to reproduce CI output;
`accent serve-static --dir output` previews the build under the `/yqr`
prefix).

## 3. Acceptance criteria

- [x] `cd docs && accent build --clean` renders `/demo` and every spec under
      `/specs/<category>/<spec-name>`; the post-build copy step installs the
      home page at `/`.
- [x] The home page is `docs/content/home.html` served verbatim at `/` —
      byte-identical to the source file, its embedded design untouched.
- [x] The theme's light and dark palettes, fonts, and logo derive from the
      home page's design tokens (core hues verbatim; auxiliary tints and
      shades, e.g. hover states and alert panels, derived from them).
- [x] The specs tree is served from `specs/` unmodified (flat-file model,
      no content restructuring), with sidebar sections per category.
- [x] Cross-spec relative `*.md` links resolve to working site URLs.
- [x] No unprefixed root-absolute link remains in the output (`accent
      build`'s base-path conformance check enforces this), and the site
      serves correctly under a `/yqr` sub-path
      (`accent serve-static --dir output` mounts it there for preview).
- [x] The Pages workflow installs accent from
      `https://github.com/AccentCMS/accent/releases` with checksum
      verification, and only deploys on pushes to `main`.
- [x] The deploy preserves the benchmark dashboard at `/dev/bench`.
- [x] The legacy `docs-pages.yml` raw-mirror workflow is removed;
      `pages.yml` and `benchmark.yml` are the only `gh-pages` writers.

## 4. Out of scope / follow-ups

- The repository homepage metadata still points at the old
  `docs/content/home.html` path; update it to the site root after the first
  deploy.
- Moving to a custom domain (root serving) only requires changing
  `site.url` — the derived path prefix disappears with it.
- `accent validate` does not traverse content mounts, so spec pages are only
  link-checked at build time.
