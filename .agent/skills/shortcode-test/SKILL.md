---
name: shortcode-test
description: Run Playwright browser tests for infobox and tabs shortcodes in both serve and build modes.
---

# Shortcode Browser Tests

Run Playwright tests that verify infobox and tabs shortcodes render correctly,
with interactive behavior (tab switching, group sync, keyboard nav, localStorage
persistence) tested in a real browser.

## When to Use

- After modifying `src/render/shortcode/builtin.rs` (infobox or tabs logic)
- After changing shortcode CSS in `_shortcodes.scss`
- After modifying `tabs.js`
- Before merging any PR that touches shortcode rendering or theme assets
- When adding new shortcode types or parameters

## Running Tests

```bash
cd tests/playwright
npx playwright test tests/shortcodes.spec.ts
```

This runs 26 tests (13 serve + 13 build mode).

### Prerequisites

Playwright and browsers must be installed:

```bash
cd tests/playwright
npm install
npx playwright install chromium
```

## What Gets Tested

### Infobox (5 tests per mode)

- ARIA attributes (`role="complementary"`) render on infobox elements
- All 5 type variants (info, prereq, security, api, version) render with correct CSS classes
- Custom title text renders in `.infobox-title` element
- Collapsible variant renders as `<details>` with `<summary>` and ARIA attributes
- Collapsible toggle opens and closes on click

### Tabs (8 tests per mode)

- Tab navigation renders with `role="tablist"` and correct ARIA on buttons
- First tab panel is visible by default, others are hidden
- Clicking a tab switches the visible panel
- Tab group synchronization: clicking in one `[tabs group="os"]` block syncs another
- Independent tab groups (`group="os"` vs `group="lang"`) do not interfere
- Keyboard arrow navigation cycles through tabs with wrapping
- `tabs.js` is served as a theme asset with expected content
- `localStorage` persists tab selection across page reloads

## Architecture

- Uses the same global setup as parity tests (ports 3100/3101)
- Tests run against `/markdown-extensions-test` page in site-dev content
- Both serve and build modes are tested for full parity coverage
