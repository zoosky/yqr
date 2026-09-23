# Feature f039 — The changelog on the website

**Status:** Done — shipped 2026-09-23, filed the same day after the v0.8.1
release
**Epic:** Website and documentation (`f010`, `f021`, `f022`, `f024`)
**Owner:** yqr maintainers
**Related:** `yqr-f021` (the public site and what it deliberately excludes),
`yqr-k001` §8 (a published page that drifted from the tool), `yqr-m001` §3
(the release process that edits `CHANGELOG.md`)

## 1. Why

The site documents what yqr does. It said nothing about what changed
between releases, so a visitor deciding whether to upgrade had to leave
for the GitHub releases page, and a reader arriving from crates.io had no
route to the notes at all.

`CHANGELOG.md` already carries that text, in the release process's own
words (`yqr-m001` §3 makes writing it a release step). The question was
only how to publish it.

## 2. Options

1. **Copy it into `docs/content/`.** A second source of truth. It is
   correct on the day it is written and stale the next release, which is
   exactly the failure `yqr-k001` §8 recorded for the comparison pages --
   four pages promised behaviour yqr did not have, and nobody noticed for
   nine days.
2. **Generate the page from `CHANGELOG.md` with a script, and gate the
   drift in CI.** No duplication that survives a build, but it adds a
   script, a gate, and a generated file in the tree.
3. **Mount `CHANGELOG.md` as content.** Accent supports a single-file
   content mount pointing at a file outside the content tree
   (accentcms f197c, released in accent 0.20.0; the site pins v0.25.0).
   No copy, no script, no gate: the page a visitor reads and the file the
   release process edits are the same bytes.

**Option 3 shipped.** It is the only one with nothing to keep in sync.

## 3. What shipped

`docs/config.yaml`:

```yaml
content:
  directory: "./content"
  mounts:
    - source: "../CHANGELOG.md"
      mount: "/changelog"
```

`source` resolves against the config file, so it climbs out of `docs/`;
the mount is what makes that legal, since `content.directory` itself may
not be escaped. Accent fails the build when the file is missing, so a
renamed or deleted changelog cannot deploy a 404 in silence.

The file has no frontmatter. The title comes from its leading `#
Changelog` heading, and with no `menu.order` it sorts after the ordered
sections in the header nav, which is where a changelog belongs. The page
picks up the theme's table of contents, the search index, `sitemap.xml`
and `llms.txt` like any other page.

### 3.1 The workflow trigger, which is the part that could have rotted

`pages.yml` ran on `docs/**` only. A release edits `CHANGELOG.md` and
nothing under `docs/`, so the deploy would not have fired and the page
would have served the previous release's notes until some unrelated docs
change happened to rebuild it -- a stale page that looks maintained,
which is worse than no page. `CHANGELOG.md` is now in the `paths` filter
for both `push` and `pull_request`.

## 4. Rule 19 check

Publishing the changelog makes it user-facing output, so it falls under
rule 19 (no internal spec references). The whole file was grepped for
`yqr-NNN` identifiers, `specs/` paths and bare feature IDs: one hit, a
packaging note naming the directories excluded from the published crate,
which is a fact about the crate rather than a tracker reference. The file
passes as it stands; a future entry must keep that property, since every
line of it is now published.

## 5. Acceptance criteria

- [x] `/changelog` renders every release section from `CHANGELOG.md`.
- [x] The page reaches the header nav, the search index, `sitemap.xml`
      and `llms.txt`.
- [x] `accent build --clean --strict-links` passes.
- [x] A release that edits only `CHANGELOG.md` triggers a site deploy.
- [x] No copy of the changelog exists under `docs/content/`.

## 6. Known cosmetics, not fixed here

- The `[Unreleased]` heading renders with nothing under it between
  releases. It is the Keep a Changelog convention and the file is the
  source of truth, so the site shows what the file says.
- Inline code renders with literal backticks around it (`` `like this` ``).
  That is the `.prose` typography rule in accent's bundled utility CSS
  (`utilities.css`: `.prose :where(code)::before/::after { content: "`" }`),
  it applies to every page on the site, and it predates this feature.
  Worth its own change if it is worth fixing.
