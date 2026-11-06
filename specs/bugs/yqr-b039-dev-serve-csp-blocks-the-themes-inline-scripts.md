# Bug b039 — Dev serve's CSP blocks the theme's own scripts

**Status:** Resolved — 2026-11-06, the day it was root-caused, by
stamping `nonce="{{ nonce }}"` on every executable script tag in the
theme
**Severity:** Low — dev serve only: the theme toggle and the diagram
loader did not run under `accent serve`, with two CSP errors on every
page load; the deployed static site was unaffected
**Component:** `docs/themes/default/templates/` (`base.html.jinja`,
`partials/mermaid-loader.html.jinja`, `docs-islands.html.jinja`,
`islands-demo.html.jinja`, `contact-island.html.jinja`)
**Related:** `yqr-b037` (whose verification surfaced it), `yqr-f041`
(the site's CSP posture), accentcms f185 (the 0.26 script lockdown)

## 1. What was observed

Every page load under `accent serve` (0.26.1 through 0.28.1) logged two
CSP violations and blocked the scripts behind them: the theme-toggle
init and the Mermaid dev loader. The dev server appends
`script-src 'self' 'nonce-...'` to each response with a fresh nonce and
no `'unsafe-inline'`, so an inline script without the nonce never runs.
The b037 verification recorded this as a pre-existing observation and
first read it as an upstream gap — accent "not stamping" its nonce onto
theme scripts.

## 2. Root cause: ours, not accent's

Review from the accentcms side corrected the diagnosis:

- Nonce stamping is explicit **by design**. `{{ nonce }}` is available
  in every template, partial, and shortcode component; the engine
  renders a per-process placeholder and the security middleware swaps
  in the per-response nonce, so cached bodies and ETags stay stable.
  The theme's job is to write `nonce="{{ nonce }}"` on each `<script>`.
- The opt-in is the security property. The nonce attribute is what
  distinguishes template-authored scripts from content-injected ones
  (the placeholder is random per process, so content cannot forge it).
  Auto-noncing every inline script in rendered HTML would bless
  injected ones too and defeat the CSP.
- It is documented twice: the theming guide's "Scripts and the content
  security policy" section (v0.26 docs onward, with a migration recipe
  and the `script_policy.report_only` / `enabled: false` knobs), and a
  bold warning in the 0.26.0 changelog that custom themes with inline
  scripts need the same change.
- yqr's theme is a fork of accent's default theme from **before 0.26**;
  accent's current default theme carries the nonce on every script tag,
  yqr's copy carried none. Nothing to file upstream.

The static build is not "unaffected by luck" either: `accent build`
records a `'sha256-...'` hash for every marked inline script and emits
header artifacts naming them. GitHub Pages simply does not serve those
headers.

## 3. The fix

`nonce="{{ nonce }}"` on all 12 script tags across the five templates:
the theme-toggle init, the Tailwind CDN pair, the analytics loader, the
KaTeX pair, the theme-asset loop, the heading-anchor-copy loader, the
two island-script branches, the Mermaid dev loader, the islands-demo
inline script and its search loader, and the contact island's two
loaders. The JSON-LD blocks are data, not executed script, so CSP never
blocks them and they carry no nonce — matching accent's default theme.

## 4. Verification

- `accent build --clean` and `accent validate` pass; the static output
  is unchanged (the build strips the nonce attribute), so the deployed
  site does not change from this fix.
- Under `accent serve` 0.28.1 the served page carries the per-response
  nonce on every theme script, the browser console is clean (previously
  two CSP errors per load), and the theme toggle — one of the two
  blocked scripts — switches `data-theme` both ways.
- The remaining un-nonced tags are the two JSON-LD data blocks and
  accent's own injected search script, which `'self'` allows.

As a bonus, the scripts are now allowed by hash in the static header
artifacts, should the site ever deploy somewhere that serves them.

## 5. Acceptance criteria

- [x] No CSP errors in the console under `accent serve`.
- [x] The theme toggle and Mermaid dev loader execute in dev serve.
- [x] The static output is byte-unchanged by the fix.
- [x] `yqr-b037` §5's misdiagnosis is corrected in place.
