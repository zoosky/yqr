# Feature f042 — The home page in Markdoc: components over hand-authored HTML

**Status:** Done — shipped 2026-10-09, with `yqr-f041` (the accent 0.26.1
adoption whose component contract this answers)
**Epic:** Project website (`f010`)
**Owner:** yqr maintainers
**Related:** `yqr-f041`, `yqr-f010` (which introduced the hand-authored
home page), `yqr-f022` (traceability out of published bodies)

## 1. Why

`content/index.md` was 420 lines of hand-authored HTML behind
`process.markdown: false` — the only page on the site whose content was
markup. Every structure on it repeats: 17 recipes, 5 section heads, 3
callouts, 3 card kinds, grids. accent 0.26 turned components into a
contract a theme is checked against, which is the vocabulary those
structures were waiting for. The page is now markdown with Markdoc
calls; the structures live in the theme as components; the design stays
where it always was, in the `home` template's style block.

## 2. The component set

**Canonical (nine):** the accent 0.26 default theme's own
implementations, copied at the pinned release
(`site-dev/themes/default/shortcodes` at v0.26.0), so `data-canonical` /
`data-part` landmarks and attribute names match what `accent validate`
checks. The copied `callout` gains one theme addition, an optional `id`,
because the home page's callouts are in-page link targets (`#engines`).

**Site (ten):** `hero`, `chart`, `outcomes`, `section-head`,
`home-section`, `grid`, `recipe`, `loc`, `path-card`, `home-card` — each
a manifest plus a template emitting the same classes the home style
block already styles. `recipe` uses 0.26's list-manifest support for its
optional `out` lines. `chart` and the terminal panes of `outcomes` stay
template-side art: token-by-token highlighting has no markdown
vocabulary, and the page places them with one self-closing call — which
is the separation the page's own frontmatter always claimed ("the design
lives in the theme").

## 3. What changed visibly, and what did not

Measured with a Playwright before/after harness (four pages, two
viewports, full-page screenshots, pixel diff):

- The three measured guide/compare pages are **pixel-identical**.
- The home page converged to a 3px height delta on desktop after two
  fix rounds the diff caught: markdown-rendered children needed their
  own rules (`.note p`, `.outcomes-note`, callout paragraphs), grid
  children needed `min-width: 0` (the mobile viewport blew out to 665px
  without it), and fenced snippets rendered with the site's dark
  highlighter inside paper cards — they are plain fences with the card
  pre styling now.
- Accepted normalizations: command text in recipes is uniform (the
  hand-placed filter-highlight spans had no markdown vocabulary),
  callout titles are block lines instead of run-in `strong`, and
  snippet token coloring is plain text. The missing vocabulary is
  filed upstream as accentcms `f399` (span marks in code: Markdoc's
  own in-fence tags through `{% process=true %}` fences plus a `mark`
  component -- the engine half is measured working in accent-proust
  already); if it lands, the three normalizations are recoverable
  from markdown.

## 4. Acceptance criteria

- [x] `content/index.md` is markdown with Markdoc components;
      `process.markdown: false` is gone and no raw HTML remains in it.
- [x] `accent validate` reports no findings: the theme implements the
      canonical contract and every manifest validates.
- [x] The build is warning-free (manifest body kinds, block-level tag
      placement).
- [x] The measured guide/compare pages are pixel-identical before and
      after; the home page's deltas are enumerated in §3 and accepted.
- [x] The anchors (`#paths`, `#engines`, `#edits`, `#validate`,
      `#recipes`, `#beyond`, `#further`, `#grammar`) survive, so header
      and in-page links keep working.
