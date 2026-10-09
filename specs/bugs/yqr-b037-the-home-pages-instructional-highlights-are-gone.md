# Bug b037 — The home page's instructional highlights are gone

**Status:** Open — filed 2026-10-09, from `yqr-f042`'s accepted
normalizations; fix gated on accentcms `f399`
**Severity:** Low — nothing is wrong as data or layout, but the page
teaches less: the emphasis that told a reader *which part* of a command
or snippet to look at is flattened
**Component:** `docs/content/index.md` (the converted body),
`docs/themes/default/shortcodes/` (`recipe`, `chart`, `outcomes`)
**Related:** `yqr-f042` (which accepted these losses, knowingly and
temporarily), accentcms `f399` (span marks in processed fences — the
vocabulary whose absence caused them), `yqr-f041` (the accent 0.26.1
pin the fix will arrive through)

## 1. What was lost, exactly

The hand-authored home page colored substrings inside code by editorial
intent. The Markdoc conversion (`yqr-f042`) had no vocabulary for that,
so three kinds of emphasis flattened:

- **The filter portion of 17 recipe commands.** `yqr -r
  '.items[] | .metadata.name'` rendered the quoted filter teal
  (`span.filter`), so the eye landed on the part of the pipeline the
  page teaches. Commands are now one color. (The `$` prompt survived:
  the `recipe` component's template renders it.)
- **Token tints in the 3 explicit-card snippets.** `tok-key` /
  `tok-val` spans toned keys and the one value each snippet exists to
  show.
- **The hit line.** One YAML line marked `span.hit` as *the* line the
  filter reaches. This emphasis is editorial, not lexical — no grammar
  highlighter can recover it.

Two blocks (the chart, the two outcome panes) kept their full
highlighting only by moving verbatim into theme templates; their markup
belongs back in reach of content once the vocabulary exists.

## 2. Why it was accepted, and why temporarily

Inside a fenced code block, markdown is literal by specification and
Markdoc has no raw-HTML passthrough, so at conversion time the choice
was attribute-noise micro-syntax or uniform rendering. `yqr-f042` §3
chose uniform and recorded it. This bug is the other half of that
record: the highlights are valuable for understanding and come back as
soon as they are expressible — they are parked, not abandoned.

## 3. The fix, when it unblocks

accentcms `f399` makes span marks Markdoc-native: a fence opts in with
`{% process=true %}` and a `mark` component wraps a range —
measured already working in accent-proust with a seven-line schema,
for both the inline-span and whole-line cases. When an accent release
carries it:

- the 17 recipe commands become processed fences (or the `recipe`
  component's `cmd` grows a marked body) with
  `{% mark role="filter" %}` around the filter;
- the 3 snippets mark their keys and shown values
  (`role="key"` / `role="value"`), and the hit line wraps in a
  whole-line `role="hit"` mark;
- the home style block already has the colors — `.filter`, `.tok-key`,
  `.tok-val`, `.hit` — so the theme work is mapping `mark-<role>`
  classes onto them;
- the chart and outcome panes move back from template art into
  content, which `yqr-f042` §2 names as the preferred end state.

Reapply against the before screenshots in the `yqr-f042` Playwright
baseline: the target is the *original* page's emphasis, not the
flattened interim.

## 4. Acceptance criteria

- [ ] An accent release carrying accentcms `f399` is pinned.
- [ ] The filter portions, token tints and the hit line render again,
      authored in `content/index.md` as marks, not HTML.
- [ ] The chart and outcome panes are content again.
- [ ] A Playwright pass against the pre-f042 baseline shows the
      emphasis restored.
