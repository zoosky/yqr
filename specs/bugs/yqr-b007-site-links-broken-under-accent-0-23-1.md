# Bug b007 — Website: two internal links break under accent v0.23.1


> **Historical: resolved.** yqr no longer behaves as described below. The
> **Status** line records what fixed it and when; the rest is kept as the
> reproduction and the reasoning, written in the present tense of the time it
> was filed.

**Status:** Resolved
**Severity:** Low
**Related:** `yqr-f010` (the Accent CMS website), `.github/workflows/pages.yml`

## 1. Symptom

Accent CMS v0.23.1 (the serve/build-parity bugfix release,
<https://github.com/AccentCMS/accent/releases/tag/v0.23.1>) tightened the
build-time link checker and aligned static-build media output with serve-time
behavior. Building the yqr site with it surfaces two broken internal links
that v0.23.0 let through silently:

```
WARN broken link: 404.html -> /docs
WARN broken link: demo/index.html -> /content-media/demo/yqr-demo.sh
```

1. **Demo script link.** `docs/content/demo/README.md` linked
   `yqr-demo.sh` via the page-local media route
   (`/content-media/demo/yqr-demo.sh`). Accent refuses to serve script
   extensions (`.sh`) as media on every route — page-local, bare page-URL
   alias, and the shared media directory alike (verified against `accent
   serve` 0.23.1: the sibling `.yaml` files return 200, the `.sh` returns
   404). v0.23.1 makes the static build match, so the file is no longer
   emitted into the output and the deployed link would 404.
2. **404 page navigation.** The vendored theme's error template
   (`docs/themes/default/templates/error/404.html.jinja`) linked
   `url('/docs')` — an upstream-theme leftover; this site has never had a
   `/docs` section. Live today: the deployed 404 page's "Documentation"
   link itself 404s.

## 2. Fix

- `docs/content/demo/README.md`: the `yqr-demo.sh` table entry links to the
  file on GitHub (`blob/main/docs/content/demo/yqr-demo.sh`) instead of the
  media route. The two `.yaml` sample links stay page-local (allowed and
  byte-identical to the repo files).
- `404.html.jinja`: the dead `/docs` link becomes `url('/specs/')`,
  labeled "Specs" — the site's actual documentation tree.
- `.github/workflows/pages.yml`: `ACCENT_VERSION` bumped `v0.23.0 ->
  v0.23.1`, matching the local development binary, and the build step gains
  the new `--strict-links` flag so any internal link whose target is missing
  from the output fails the build instead of warning. This class of defect
  (works in `accent serve`, 404 on the deployed site) is now caught in CI on
  every PR touching `docs/` or `specs/`.

Also of note: v0.23.1 ships the upstream fix for the copy-button label
leaking into copied clipboard text — the vendored theme already carries the
equivalent local fix in
`docs/themes/default/assets/js/islands/copy-code.js`, so no theme change is
needed.

## 3. Verification

`cd docs && accent build --clean --strict-links --base-url
https://zoosky.github.io/yqr` (accent 0.23.1) completes with zero link-check
warnings; the base_path conformance check passes; `output/404.html` links to
the prefixed `/yqr/specs/`; `output/content-media/demo/` contains the two
`.yaml` samples.
