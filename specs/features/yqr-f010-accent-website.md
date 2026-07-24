# yqr.f010 — Project website: Accent CMS site over docs/ and specs/, on GitHub Pages

**Status:** Done
**Epic:** Project website (f010)
**Owner:** yqr maintainers
**Related:** `yqr-m003` (the demo content served at `/demo`), the former
`docs/content/home.html` (ported to the site home page), `benchmark.yml`
(shares the `gh-pages` branch with the site deploy)

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
- **Home page:** `docs/content/index.md` — the former `home.html` content
  ported to markdown; the Accent theme owns all styling.
- **Spec tree:** `../specs` is mounted at `/specs` via `content.mounts` and
  served as-is; each category directory carries a `README.md` index so it
  appears as a sidebar section. Relative `*.md` links between specs resolve
  to clean page URLs automatically.
- **Theme:** the Accent CMS default docs theme (sidebar, search, dark mode,
  code copy), vendored into `docs/themes/default/` and de-branded for yqr
  (compass logo/favicon, yqr footer, sidebar rooted at `/specs/`).
- **Sub-path serving:** GitHub Pages serves the repo at
  `https://zoosky.github.io/yqr/`, but Accent emits root-absolute links with
  no sub-path setting. `.github/scripts/pages-path-prefix.sh` rewrites every
  root-absolute URL form in the build output (HTML `href`/`src`, the
  HTML-escaped variant, meta-refresh redirect stubs, the DocFind search
  assets and index, `llms.txt`) and fails the build if an unprefixed link
  survives.
- **Deploy:** `.github/workflows/pages.yml` fetches the pinned accent binary
  from the upstream GitHub release (checksum-verified), builds, rewrites, and
  pushes the output to the `gh-pages` branch — preserving `dev/bench`, the
  Criterion benchmark dashboard that `benchmark.yml` publishes to the same
  branch. Pull requests touching `docs/`, `specs/`, or the workflow build and
  verify without deploying.

Local development uses the same binary: `cd docs && accent serve` (or
`accent build --clean` plus the rewrite script to reproduce CI output).

## 3. Acceptance criteria

- [x] `cd docs && accent build --clean` renders the full site: home page,
      `/demo`, and every spec under `/specs/<category>/<spec-name>`.
- [x] The home page is the ported `home.html` content, styled entirely by the
      Accent theme (the hand-written HTML file is removed).
- [x] The specs tree is served from `specs/` unmodified (flat-file model,
      no content restructuring), with sidebar sections per category.
- [x] Cross-spec relative `*.md` links resolve to working site URLs.
- [x] After `pages-path-prefix.sh`, no unprefixed root-absolute link remains
      in the output (the script self-verifies), and the site serves correctly
      under a `/yqr` sub-path.
- [x] The Pages workflow installs accent from
      `https://github.com/AccentCMS/accent/releases` with checksum
      verification, and only deploys on pushes to `main`.
- [x] The deploy preserves the benchmark dashboard at `/dev/bench`.

## 4. Out of scope / follow-ups

- The repository homepage metadata still points at the old
  `docs/content/home.html` path; update it to the site root after the first
  deploy.
- A custom domain (root serving) would make the sub-path rewrite unnecessary;
  revisit if a domain is assigned.
- `accent validate` does not traverse content mounts, so spec pages are only
  link-checked at build time.
