# Feature f021 — Split the public site from the spec site

**Status:** In Progress — the split is done; the `llms.txt` content items it
enables are tracked in §6
**Epic:** Project website (`f010`)
**Owner:** yqr maintainers
**Related:** `yqr-f010` (the site this divides in two), `yqr-m001` (the
workflow summary that had to follow), `yqr-b007` (the `--strict-links` gate
that caught the one link this broke)

## 1. Decision

yqr serves two sites from one repository:

| | config | content | deployed |
|---|---|---|---|
| Public | `docs/config.yaml` | `docs/content` | GitHub Pages, `/yqr/` |
| Internal | `specs/config.yaml` | `docs/content` **plus** `specs/` at `/specs` | never; `localhost:4401` |

The public site loses the spec tree. The internal site is what the combined
site used to be, and it reads its guide pages from `docs/content`, so a page
is edited once and both sites pick it up.

## 2. Why

A review of the published `llms.txt` and `llms-full.txt`, read as an LLM
consumer would read them, found the packaging at fault rather than the prose.
The spec tree was most of what got published:

- `llms-full.txt` was **612 KB**, the bulk of it spec text. A reader with a
  context limit truncated inside a bug report rather than after the usage
  documentation.
- `llms.txt` listed `a001`, `a002`, the bug index, then `b001`–`b005`, and
  stopped. Five bugs resolved in June and July, and **zero** capability
  documentation — the worst possible sample.
- Spec entries were the page's first ~120 characters, so they read
  `Status: Accepted Owner: yqr maintainers Last updated: ...`.

The tree is written for contributors. It uses internal identifiers
(`yqr-b010`, `noyalib#296`, section numbers), records decisions later
reversed, and carries resolved bug reports describing behaviour the product no
longer has. None of that is wrong; it is simply not addressed to the reader
the public site is for.

### 2.1 Why removal rather than demotion

The review proposed demoting the tree to `llms.txt`'s `## Optional` section
and excluding it from `llms-full.txt`, explicitly keeping it on the site. That
is the smaller change and it needs generator support that does not exist yet
(an ordering key and a path-glob exclude, `f021` §6).

Removing the mount does the same job today with configuration yqr already
controls, and it does more: the tree also stops appearing in the sitemap, the
search index, and the site's URL count. The `noindex,follow` rule the theme
carried for spec pages was a symptom of publishing something meant for
contributors; the split treats the cause.

**What is not lost:** the tree stays in the repository, stays browsable on
GitHub, and stays a full CMS site with search and sidebars — just a local one.

## 3. What changed

- **`specs/config.yaml`** — new. Port 4401, no path prefix (the public site's
  `/yqr/` prefix comes from its `site.url`; this one serves at a port root).
  Mounts `../specs` rather than `.`, because the config and the build output
  live in `specs/` and mounting the directory onto itself would walk them —
  verified by clean-rebuilding twice with `output/` present.
- **`docs/config.yaml`** — the `content.mounts` block is gone, and the `llms`
  description now carries the synopsis (§4).
- **`pages.yml`** — no longer triggers on `specs/**`. A spec-only change
  cannot affect the public site, so building it would be noise.
- **`404.html.jinja`** — its "Specs" link pointed at `/specs/`, which no
  longer exists on the public site. Now "Guide", which is what a lost visitor
  wants and is valid on both sites. This was the **only** break, and
  `--strict-links` found it rather than a reader.
- **`head_meta.html.jinja`** — the `noindex,follow` branch for spec pages is
  now unreachable on the public site. Kept, with the comment saying why: the
  two sites share a theme, and a future decision to publish part of the tree
  should inherit the rule rather than rediscover the need for it.
- **`.gitignore`** — `specs/output/`.

## 4. The synopsis (review item A1)

Neither published file said how to install yqr or how to invoke it. The
`llms.txt` summary spent its last clause describing the site. It now carries
the install line, the invocation shape, the flag set, the exit codes and links
to the repository and the crates.io page — so a reader who fetches `llms.txt`
and nothing else can use the tool.

Exit codes were previously scattered across four guide pages and never listed
together. They are measured against the binary, not transcribed:

| code | meaning |
|---|---|
| 0 | success |
| 1 | `validate` found a problem in the file |
| 3 | the filter could not be lexed or parsed |
| 5 | runtime or I/O error, including a refused edit |

## 5. Result

| | before | after |
|---|---|---|
| public pages | 66 | 9 |
| `llms-full.txt` | 612 KB, mostly specs | **33 KB** — guide, compare and demo |
| spec URLs in `llms.txt` | 8 of ~20 entries | 0 |
| spec URLs in `sitemap.xml` | 57 | 0 |
| synopsis in either file | none | install, invocation, flags, exit codes, links |

The 95% cut to `llms-full.txt` is the acceptance criterion the review cared
about most: a context-limited reader now truncates, if at all, after the usage
documentation rather than inside a June bug report.

Both sites build clean under `--strict-links`.

## 6. What this does not fix

The split solves the packaging. Four review items are content, and stay open:

- **A2** — authored `description:` frontmatter per page. Every `llms.txt`
  entry is still a mid-word truncation of the body. Blocked on the generator
  preferring `description:` over body text (review item B1); the frontmatter
  is inert until then, so the two land together.
- **A3** — internal HTML comments survive into `llms-full.txt` as visible
  text, including one that leaks a `specs/marketing/...` path. This
  **conflicts with ground rule 19**, which requires exactly those comments in
  `docs/content/`; the rule's stated purpose is that they stay invisible, and
  `llms-full.txt` breaks that premise. Needs a decision, not a patch.
- **A4** — `yqr-b001` is headed **Resolved** and then describes the broken
  behaviour in the present tense for ~2000 words. A model landing mid-document
  reports the inverse of the product claim. Now internal-only, which lowers
  the stakes without removing them: the internal site is exactly what a local
  model reads. Other resolved bug specs need the same check.
- **A5** — `normalises` (compare/yq) against `normalized` (guide, specs).

The generator items (B1–B5) belong to accent and are unaffected by this
feature, except that B3's `llms` path-glob exclude is no longer needed by yqr.

## 7. Acceptance criteria

- [x] Two configs, one shared content directory, no duplicated guide pages.
- [x] The public site has no `/specs` URLs — measured in `llms.txt`,
      `llms-full.txt` and the page count, not assumed from the config.
- [x] The internal site serves what the combined site served, on port 4401.
- [x] Both build clean under `--strict-links`, with the one broken link found
      by the gate and fixed.
- [x] Mounting `specs/` from a config inside `specs/` does not walk the build
      output — verified by clean-rebuilding twice.
- [x] `pages.yml`, `yqr-m001` and `AGENT.md` agree with the new arrangement.
- [x] `llms.txt` carries the synopsis (§4), with the exit codes measured
      against the binary.
- [ ] §6's content items — tracked here, not silently dropped.
