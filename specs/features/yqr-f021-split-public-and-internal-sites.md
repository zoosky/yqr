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

The split solves the packaging. Of the four content items, three are done and one needs a decision:

- **A2 — done.** See §9.
- **A3** — internal HTML comments survive into `llms-full.txt` as visible
  text, including one that leaks a `specs/marketing/...` path. This
  **conflicts with ground rule 19**, which requires exactly those comments in
  `docs/content/`; the rule's stated purpose is that they stay invisible, and
  `llms-full.txt` breaks that premise. Needs a decision, not a patch.
- **A4 — done.** See §8.
- **A5 — done.** Two outliers, `normalises` in `compare/yq` and
  `normalisation` in `yqr-k001`, now match the flag they describe
  (`--normalize`). The wider `-ise`/`-ize` mix in the spec tree
  (`behaviour` 49, `behavior` 18) is **not** swept: `yqr-m006` §4 says
  existing documents are not rewritten for style alone.

**B5 was half yqr's**, contrary to a first reading that assigned all of B1–B5
to the generator. A section heading in `llms.txt` is the directory's index-page
*title*, so `yqr Demo` against `yqr guide` was inconsistent because the demo
page's `# yqr Demo` heading was written that way. Now `yqr demo`, matching
`yqr guide` — product name lowercase, common noun sentence case.
`compare/README.md` keeps its full descriptive title: it is correctly cased
already and is the better page title, and shape consistency across three
headings is not worth degrading it. What stays the generator's is the
duplication — the index page appears as the heading *and* as its own first
entry.

The rest of B1–B5 were read against accent's source and are catalogued in
**`yqr-b017`**, which finds two of the five are not defects at all: B1 (the
feature exists under another key) and B4 (a documented `max_links_per_section`
cap). B2 is confirmed real — the pages carry `menu.order` and `llms.txt`
publishes directory order regardless. B3 half exists: the glob exclude covers
both files already; demotion to `## Optional` does not.

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

## 8. Review item A4 — resolved-bug specs in the present tense

`yqr-b001` is headed **Resolved** and then describes the broken behaviour in
the present tense for ~2000 words: *"the round trip **rewrites the entire
file**"*, *"the product promise is unmet today"*. A model landing mid-document
reports the inverse of the product claim.

Checking the other bug specs, as the review asked, found the same shape is
**systemic rather than a b001 one-off** — `yqr-b010` opens *"noyalib's
sequence-reorder mutators exchange the items' value bytes and nothing else"*,
and the four specs resolved this week read the same way. Every bug spec is
written in the present tense of the day it was filed, and every one of them
outlives that day.

So the fix is a convention, not fourteen rewrites:

- **`b001` gets both**, because it is the one asserting a product-level
  guarantee is broken, at 465 lines: a banner naming `f009` and the
  `--normalize` opt-out, plus §1 and §2 past-tensed, plus a marker on §5
  ("Observed behavior"), the other section long enough to be landed in.
- **Every other resolved spec gets a one-line banner** directly under its
  title, saying that yqr no longer behaves as described and pointing at
  **Status** for what fixed it. Fourteen files, one uniform sentence.

Past-tensing all fourteen bodies was considered and rejected: it is a large
diff, a worse `git blame`, and it fights `yqr-m006` §4. It also would not
fully solve the stated failure mode, since a model landing mid-document is
reading whatever chunk it landed in either way. The banner is the review's own
alternative and it is what the top of every chunked document carries.

**The convention going forward:** a bug spec moving to Resolved gets the
banner in the same change. Recorded here rather than in `AGENT.md`, because it
is one line of bookkeeping rather than a ground rule.

## 9. Review item A2 — authored entries

Every `llms.txt` entry was the page body's first ~120 characters, truncated
mid-word. Six of the eight were unreadable as summaries; the spec entries were
worse, but the split had already removed those.

This spec first recorded A2 as **blocked** on the generator preferring an
authored description over body text. That was wrong, and `yqr-b017` §2 records
the correction: accent already prefers an authored entry text. The frontmatter
key is **`lead:`**, not `description:`. yqr had set `description:` on eight
pages, which feeds the `<meta>` tags and is silently ignored by `llms.txt`, so
the pages carried a good sentence and published a truncated excerpt anyway.

All nine public pages now carry a `lead:` saying what the page **answers**,
which is the form the review asked for. Zero entries end in an ellipsis.

### 9.1 The 120-character budget is real, and it binds authored text too

`truncate_lead` applies `LEAD_TRUNCATE = 120` to whatever `Page::lead()`
returns, authored or derived, and cuts at character 119 with no word-boundary
handling. So a 140-character authored sentence is published cut mid-word
exactly like a body excerpt — the fix is only a fix if the sentence fits.

Every lead here was written to that budget and measured against it (99–115
characters). Recorded because it is invisible: nothing warns, and the symptom
is identical to not having authored one.

### 9.2 What did not change

`description:` still wins the `<meta>` chain, so the meta descriptions of the
eight pages that had one are byte-identical. The demo index, which had neither
a `description:` nor a `title:`, gains a meta description it previously
lacked — an improvement, and the only rendered-page change in this item.
