# Feature f041 — Adopt accent 0.26.1: the component contract answered, the rest measured

**Status:** Done — shipped 2026-10-09
**Epic:** Project website (`f010`)
**Owner:** yqr maintainers
**Related:** `yqr-f024` (the 0.25.0 adoption and the llms.txt findings),
`yqr-f021` (the public/spec site split), `yqr-f039` (the changelog mount)

## 1. What the release is

accent 0.26.0 (2026-09-24) locks scripts down everywhere accent renders a
page, turns components into a contract a theme is checked against, makes
`accent build` fail on a broken internal link, and makes directory
previews navigable sites with search. 0.26.1 (2026-10-04) is its security
respin: wasmtime 48.0.5 fixes ten advisories in the plugin runtime, one
critical. Local development had already moved to 0.26.1; this adoption
moves the CI pin (`pages.yml` `ACCENT_VERSION`) to match, which is the
binary that builds the deployed site.

## 2. What required action

- **The canonical-component contract.** `accent validate` on 0.26 holds a
  theme to nine canonical components and reported nine errors against the
  vendored theme, whose two `[shortcode]`-era templates (alert, button)
  were unreferenced carryovers of a syntax 0.26 removed. The first answer
  was `components: none`; it lasted one review cycle — the home page's
  hand-authored HTML was the real debt, and converting it (`yqr-f042`)
  means the theme now implements the contract instead: the nine canonical
  components are the accent default theme's own implementations, copied
  at the pinned release so the markup landmarks and attribute names match
  what validate checks. `accent validate` is clean with no findings.
- **`--strict-links` is deprecated.** 0.26 makes a broken internal link
  fail the build by default and 0.27 removes the flag, so the build step
  and the docs drop it.
- Nothing else. The breaking changes land as no-ops here, each checked
  rather than assumed: broken internal links already fail this site's
  builds (`--strict-links` since `yqr-b007`); no `[shortcode]` syntax
  appears in content; no plugins are installed, so the plugin API bump and
  the wasmtime advisories are unreachable; and the static-output script
  policy ships as header artifacts this site does not emit — the built
  pages carry no CSP `<meta>` and no headers file, and GitHub Pages serves
  no custom headers, so the unmarked theme scripts keep working. Revisit
  the `nonce` marking if the site ever moves behind `accent serve-static`.

## 3. What the site gains without changes

- **Search results highlight the term** on the page they open, via text
  fragments — no script, works on the static deploy as rebuilt.
- **Spec-site reading improvements** for `accent serve` sessions
  (`specs/config.yaml`, port 4401): the preview search box, the light/dark
  toggle, and `.gitignore`-aware directory walks.

## 4. Evaluated and deferred

- **`nav_tree()`**: the theme's header nav is a deliberate flat loop whose
  depth arithmetic is measured against `url('/')` so it survives sub-path
  deployments, with a comment explaining exactly that. The function would
  replace working, documented code with equivalent code — churn, not a
  gain. Adopt when the nav grows levels.
- **Builtin components** (`{% download %}`, `{% video %}`, `{% diagram %}`,
  `{% search %}`): no current page needs one; the search box is already in
  the theme header.
- **musl CI asset**: the gnu binary works on `ubuntu-latest`; switching
  buys nothing today.

## 5. Acceptance criteria

- [x] `ACCENT_VERSION` in `pages.yml` is v0.26.1, with the asset naming and
      checksum verification confirmed against the published release.
- [x] `accent validate` is clean under 0.26.1 (0 errors, 0 warnings).
- [x] `accent build --clean --strict-links` passes and the deployed-layout
      output carries no CSP artifacts that GitHub Pages cannot serve.
- [x] The stale pin notes in `CLAUDE.md` and `pages.yml` match the new pin.
- [x] The deferred features are recorded with reasons (§4), so the next
      accent bump re-evaluates instead of rediscovering.
