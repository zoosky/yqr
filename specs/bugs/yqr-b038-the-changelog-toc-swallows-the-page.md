# Bug b038 — The changelog's table of contents swallows the page

**Status:** Resolved — filed and fixed 2026-10-09, reported from the
deployed site
**Severity:** Medium — the page is correct but unreadable at desk:
1,675px of table of contents before any content, which makes a 1440px
desktop window read like a phone
**Component:** `docs/themes/default/templates/toc.html.jinja` (renders
every entry at every level, blind to scale), `_components.scss`
**Related:** `yqr-f039` (the changelog mount this bites), `yqr-f048`-era
TOC partial, `yqr-f041` (the 0.26 pin the page renders under)

## 1. Symptom

On `/changelog` at 1440px, the inline "On this page" box held 56
entries — 16 release headings plus every Added/Fixed/Changed
subsection — at 1,675px tall, pushing all content a screen and a half
down. Measured in the browser on the deployed site; the guide pages,
with 7 to 13 entries, read fine through the same partial.

The mount is the trigger: `CHANGELOG.md` deliberately carries no
frontmatter (`f039` — the page and the release-edited file are the
same bytes), so no per-page TOC setting can exist for it, and the
partial rendered everything.

## 2. Fix

The partial chooses its rendering by scale, since configuration is not
available by design: a page with more than 12 entries gets a compact
index — top-level entries only, flowing as wrapped chips — and
everything else keeps the full nested list. The changelog indexes its
16 releases in four chip rows; `compare/yq` (13 entries) tightens to a
7-chip index; every guide is under the threshold and unchanged.

## 3. Measured

Playwright at 1440px before and after: the changelog's first screen
goes from all-TOC to the release index, the intro and the newest
release's notes. The chips use theme tokens, so both color schemes
hold, and the flex wrap keeps phone width clean.
