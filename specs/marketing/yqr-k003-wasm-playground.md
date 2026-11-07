# Marketing k003 — The playground: yqr compiled to WebAssembly, in the reader's browser

**Status:** Done (2026-11-07) — `/playground/` shipped; every acceptance
criterion in §8 verified in a real browser (Playwright over
`accent serve --no-tls`) before the PR. §7 (distribution) added the same
day: the wasm package ships as a release artifact, npm deferred.
**Owner:** yqr maintainers
**Last updated:** 2026-11-07
**Related:** `yqr-f049` (the wasm target and the bindings crate this runs),
`yqr-f050` (the typed edit API, deliberately not surfaced here), `yqr-k001`
(the measure-first rule §4 follows), `yqr-k002` (the jq on-ramp the presets
echo), `yqr-f010` (the Accent site), `yqr-m001` (the pages workflow this
extends)

## 1. Why

The site makes one claim everywhere: edits touch only the bytes they must.
The fidelity guide proves it with recorded console blocks, the demo page
proves it with a script the reader must install yqr to run. Both put the
proof on the far side of an installation.

The wasm bindings (`yqr-f049`) removed the technical half of that barrier
months ago — the full engine compiles to a 700 KB module with `eval`,
`fidelityRun` and `validate` exports — but nothing user-facing runs it. The
crate ships for editor embedders and is otherwise invisible.

A playground closes the gap: the reader types a filter, the same engine the
CLI uses runs in their browser, and the fidelity claim becomes something
they can try to falsify in ten seconds without installing anything. The
precedent is accent-proust's browser demo, which exists for the same two
reasons — verification in the environment the bindings target, and a
starting point for embedders.

## 2. What it is

One page, `/playground/`, served by the existing Accent site. It holds a
hand-commented sample document (comments, an anchor, a merge key, aliases,
deliberate odd indentation, a quoted version string), a filter input, and a
row of preset filters that walk the surface in the order `yqr-k002` found
jq users learn it: read, iterate, edit, delete, update, and one preset that
fails on purpose.

Three output surfaces, each mapped to one wasm export:

- **CLI output** (`fidelityRun`): exactly what `yqr '<filter>' file`
  prints. For a mutating filter that is the whole edited document, shown
  with a byte-diff against the input: the changed region is highlighted
  and a counter states how many bytes are the reader's original bytes.
  For a read it is the value sliced out of the original bytes, and the
  counter says so instead of diffing a fragment against a document. The
  two cases are told apart by prefix/suffix overlap (an edited document
  shares most of its bytes with the input; a sliced value does not).
- **Values** (`eval`): the filter's output values, the query half of the
  tool.
- **Checks** (`validate`): live diagnostics over the sample as the reader
  edits it, rendered compiler-style with code, line and column — the same
  findings `yqr validate` prints.

A filter that does not compile renders its error with the `ErrorEnvelope`
span from the bindings: the offending token is underlined with a caret line
beneath the filter input, in the browser, exactly as the CLI would print
it. The error path is part of the product and the playground treats it as a
feature, not a failure screen.

## 3. Why a byte diff and not a caption

"Comments survived" as prose is the same assertion the rest of the site
already makes. The diff makes it checkable: the reader can paste their own
YAML — the document pane is editable — run their own edit, and count the
highlighted bytes. The diff is computed as the longest common prefix and
suffix of input and output, which is honest for single-site edits (the only
kind the presets perform) and cheap enough to run on every keystroke.

Everything runs locally. No document leaves the page, and the page says so:
for configuration files, which is what yqr is for, that line is load-bearing.

## 4. The measurement

Per `yqr-k001` §2, every preset was executed before any page prose was
written. Run 2026-11-07 against yqr v0.11.0 (commit 3fc4e46) on the sample
document that ships with the page:

| Preset | Verified behaviour |
|---|---|
| `.server.port` | prints `8080` |
| `.services[].name` | streams `api`, `worker` |
| `.services[0].image` | resolves through the `<<: *defaults` merge key |
| `.versions.api` | prints `"1.4.2"` with its quotes |
| `line_comment(.server.port)` | prints the comment text on that line |
| `.server.port = 9090` | one line changes; the trailing comment on it survives |
| `del(.features[1])` | removes the entry and its comment, nothing else |
| `.replicas \|= . + 1` | in-place update, `2` becomes `3` |
| `.server \|` | parse error, exit 3: `expected Dot but reached end` |

`yqr '.'` over the sample is byte-identical to the input (verified with
`diff`). The wasm exports call the same `eval_str`, `fidelity::run_program`
and `validate::check_str` entry points, so the CLI measurement carries
over; the bindings' own smoke tests pin that equivalence in CI.

## 5. Integration mechanics

The theme already has an islands architecture: `island-loader.js` hydrates
`<accent-island>` elements, a page declares its components in `islands:`
frontmatter, and `base.html.jinja` loads each component's script
fingerprinted. The playground is one more island, not a new mechanism:

- `docs/content/playground.md` — the page: prose, frontmatter
  (`template: playground`, `islands: [{component: yqr-playground}]`,
  menu order 5).
- `docs/themes/default/templates/playground.html.jinja` — renders the
  island element with its no-JS fallback (install instructions and a
  pointer to the demo script) and passes the wasm package URL through
  `data-props`, so the GitHub Pages `/yqr/` prefix is handled by the
  theme's own `url()` helper.
- `docs/themes/default/assets/js/islands/yqr-playground.js` — the island.
  It dynamically imports the wasm-bindgen glue and instantiates the module
  on hydration; if either step fails, the server-rendered fallback stays.
- `docs/themes/default/assets/css/_playground.scss` — styles, both themes,
  brand amber `#c68a3e` and teal `#4fb3ac` only.

**The wasm package is a build artifact, not a committed file.**
`yqr-wasm/pkg/` is gitignored and stays that way: a committed binary
drifts from the source the moment either changes.
`.github/scripts/playground-wasm.sh` builds it (`wasm-pack build yqr-wasm
--target web --release`, the same invocation and pinned wasm-pack 0.15.1
as `ci.yml`) and copies the glue and module into
`docs/themes/default/assets/wasm/`, which is also gitignored. `pages.yml`
runs the script before `accent build`; locally, one script run makes
`accent serve` complete.

**Trigger paths.** `pages.yml` additionally deploys on pushes to `main`
touching `src/**`, `yqr-wasm/**` or `Cargo.lock`, so the deployed
playground tracks the engine rather than freezing at the last docs edit.
Pull requests keep the docs-only paths: an engine PR's wasm behaviour is
already verified by `ci.yml`'s wasm lane, and a site build adds nothing to
it.

## 6. What it deliberately does not do

- **No `--normalize` toggle.** The bindings do not export the
  re-serializing pipeline, and the playground exists to show the default.
- **No typed edit API surface.** `applyAll` (`yqr-f050`) is for programs,
  not people; a JSON ops editor would demo plumbing.
- **No permalinks or sharing.** State is two strings; copy and paste does
  the job the first version needs. Revisit if anyone asks.
- **No fallback engine.** Without JS or wasm the island never hydrates and
  the static fallback — install lines and a link to the demo script —
  is the whole page. That is the progressive-enhancement contract the
  theme's islands already follow.

## 7. Distribution: a release artifact, not npm (added 2026-11-07)

The playground's embedder pitch raised the question of an npm release,
accent-sass-style. Decided against, for now:

- **No consumer needs it.** The playground builds the module from source;
  accentcms consumes nothing yet, and its planned adoption (accent `f430`)
  is the Rust crate, not a JS package. accent-sass pays npm's costs —
  publish ordering, version lockstep, token custody, a wasm job that
  twice shipped empty artifacts — because its npm package has users.
- **Pre-1.0 churn.** yqr breaks its surface on minors; npm consumers
  would inherit that immediately.
- Both names (`yqr`, `yqr-wasm`) were free on the registry on
  2026-11-07, so waiting costs little.

What ships instead: each release attaches `yqr-wasm-pkg.tar.gz` (module,
JS glue, `.d.ts`, `package.json`) plus its `.sha256` — a pinned,
checksummed artifact with no registry custody. It is a dist
extra-artifact (`Cargo.toml` `[[workspace.metadata.dist.extra-artifacts]]`
running `.github/scripts/wasm-pkg-tarball.sh`); `dist plan`, which
`release.yml` runs on every PR, verifies the configuration, and `ci.yml`'s
wasm lane keeps the underlying build green. The module inside carries
`yqr-wasm`'s own version (the adapter tracks its own surface, per the
crate's manifest comment); the release tag names the engine version.

Revisit npm when one of these fires: an external embedder asks, the
accent browser editor adopts the module, or 1.0 stabilizes the surface.
Then follow accent-sass's shape (a `release.sh` step, npm last, unscoped
name, trusted publishing). Like `cargo publish` (`yqr-m004`), an npm
publish is a separately authorized step, never inferred.

## 8. Acceptance criteria

- [x] `/playground/` builds and validates in both site configs
      (`accent build` in `docs/`, `accent validate -c specs/config.yaml`).
- [x] Every preset in §4 produces its verified behaviour in a real
      browser via the wasm module.
- [x] `.` reports the document byte-identical; `.server.port = 9090`
      highlights only that line's value bytes (636 of 639 untouched); a
      fragment read is labelled as sliced output, never diffed against
      the document.
- [x] A filter error underlines the offending span beneath the input.
- [x] With the wasm assets absent, the page still builds and shows the
      fallback (verified by renaming the assets directory and reloading).
- [x] `pages.yml` builds the wasm package before the site on both PR
      verification and deploy.
- [x] The demo page links to the playground.
