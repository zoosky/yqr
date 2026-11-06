# Bug b037 — The home page's instructional highlights are gone

**Status:** Resolved — 2026-11-06, by adopting accent v0.28.1 (the first
pinned release carrying accentcms `f399`, which shipped in v0.27.0) and
reauthoring the lost emphasis as `mark` components in
`content/index.md`; see §5
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

- [x] An accent release carrying accentcms `f399` is pinned.
- [x] The filter portions, token tints and the hit line render again,
      authored in `content/index.md` as marks, not HTML.
- [x] The chart and outcome panes are content again.
- [x] A Playwright pass against the pre-f042 baseline shows the
      emphasis restored.

## 5. Resolution (2026-11-06)

`ACCENT_VERSION` in `pages.yml` moved v0.26.1 -> v0.28.1 (local
development was already there); accentcms `f399` shipped in v0.27.0 as
filed — `{% process=true %}` fences plus a `mark` component — and the
asset naming and checksum flow are unchanged. The three losses came
back as authored marks, plus one §1 did not track:

- The filter portion of every recipe command carries
  `{% mark role="filter" %}` (16 commands; the `validate` recipe has no
  filter). The `recipe` component's `cmd`/`out` attributes are gone:
  its body now opens with the command as a processed console fence —
  an attribute string cannot carry marks — and output lines are marked
  `role="out"`. The `$` prompt became theme chrome (`::before` on the
  pane's `code`), so the fence holds only what the user types and
  copies.
- The 3 explicit-card snippets mark their shown keys (`role="key"`),
  and the chart's hit line is a whole-line `role="hit"` mark with the
  key and value marked inside it — marks nest.
- The chart's YAML pane and both outcome panes moved from template art
  into content (`f042` §2's preferred end state): `chart` grew a
  markdown body, and the outcome strip is a new `outcome` child
  component (badge, kind) whose body is the transcript fence. Only the
  chart's label and animated route stay template-side.
- The Dockerfile pane's comment and keyword tints, flattened by the
  same conversion but not listed in §1, are back as `role="comment"` /
  `role="keyword"` marks.

The theme's `mark` component renders
`<span class="mark mark-<role>" data-part="mark">`, and the home style
block maps the roles onto the colors it always had. Verified against
the pre-f042 markup (the span-by-span source of truth) with a
Playwright pass on `accent serve`: all mark counts match the original
spans, the hit line renders with its background, border and value
weight in both themes, `accent validate` and the build are clean, and
the page has no horizontal overflow at 375px. `llms-full.txt` and the
search index carry the fences' plain text with no mark syntax, as
`f399` promised. One pre-existing observation, out of scope here: under
`accent serve` 0.28.1's nonce CSP, the theme's own inline scripts
(theme-toggle init, diagram loader) are blocked in dev serve; the
deployed static site serves no CSP header and is unaffected. Filed and
fixed as `yqr-b039` — the cause was this theme (forked before accent
0.26) never stamping `nonce="{{ nonce }}"` on its script tags, not an
accent gap.
