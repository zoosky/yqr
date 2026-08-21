# Bug b017 — accent's `llms.txt` generation: one defect, two gaps, two non-defects


> **Historical: resolved.** yqr no longer behaves as described below. The
> **Status** line records what fixed it and when; the rest is kept as the
> reproduction and the reasoning, written in the present tense of the time it
> was filed.

**Status:** Resolved — measured 2026-08-20 against `accentcms` `b2c2eec3`,
filed 2026-08-21 as accentcms `b190` (five findings, two measured non-defects)
and `f328` (the demotion gap), and **released in accent 0.25.0** the same day.
All five findings verified against the released binary by `yqr-f024` §2, per
finding. `f328` stays open upstream and yqr does not need it — `yqr-f021` moved
the spec tree to its own site
**Severity:** Low — nothing is wrong with the published site; what is affected
is how well a machine reader can use `llms.txt`
**Component:** `accentcms`, `src/render/llmstxt.rs` and `src/config/llms.rs`
**Related:** `yqr-f021` (the site split this came out of), `yqr-f010` (the
site), `yqr-b007` (the previous accent bug yqr filed)

## 1. Summary

A review of yqr's published `llms.txt` and `llms-full.txt` proposed five
generator changes (items B1–B5). Read against the source, **two of the five
are not defects**, one is a real defect, and two are real gaps.

| Review item | Verdict |
|---|---|
| B1 — prefer authored description over body truncation | **Not a defect.** It already does; the key is `lead:` |
| B2 — section ordering control | **Gap.** Sections are alphabetical by URL segment; `menu.order` is not consulted |
| B3 — path-glob exclude / demote | **Half exists.** Exclude works for both files; demote to `## Optional` does not |
| B4 — spec listing cut-off | **Not a defect.** `max_links_per_section`, default 10 |
| B5 — heading casing and duplication | **Casing was the consumer's.** The duplicate entry is a defect |

Each verdict below is measured against the source or against a build, not
inferred from the output.

## 2. B1 — not a defect, but the field name is a trap

The review's diagnosis was that entries are the page body's first ~120
characters and that an authored description should win. The second half
already holds. `Page::lead()` (`src/content/page/mod.rs:395`) returns the
`lead:` frontmatter field, and body extraction only happens when that field is
absent:

```rust
// src/content/page/mod.rs:255 — "Auto-generate lead from first paragraph if not set"
if frontmatter.lead.is_none() {
    frontmatter.lead = meta::extract_excerpt(&parsed.content);
    frontmatter.derived.lead = frontmatter.lead.is_some();
}
```

Confirmed by experiment rather than by reading. Adding one line to yqr's guide
index:

```yaml
lead: Task-shaped pages that teach you to query and edit YAML with yqr.
```

changed the published entry from a mid-word truncation of the body to that
sentence exactly.

**What is worth reporting** is the trap, not a bug: `description:` is the
obvious name for this, yqr set it on three pages, and it feeds the `<meta>`
tags while `llms.txt` silently ignores it. A page can carry an authored
description and still publish a truncated body excerpt, with nothing to
indicate why.

**Suggested:** fall back to `description:` when `lead:` is absent, before
falling back to the body. It costs one line, breaks nothing (`lead:` still
wins), and removes a silent failure whose symptom is indistinguishable from
"the feature does not exist" — which is exactly how it was diagnosed here.

### 2.1 The 120-character budget binds authored text too

`truncate_lead` applies `LEAD_TRUNCATE = 120` (`llmstxt.rs:29`) to whatever
`Page::lead()` returns, authored or derived, cutting at character 119 with no
word-boundary handling. A 140-character authored sentence is therefore
published cut mid-word, exactly like a body excerpt.

That makes "author a `lead:`" a fix only for someone who happens to write
short enough. Nothing warns, and the symptom is identical to not having
authored one at all. yqr's nine leads were written to the budget and measured
(99–115 characters), which is not a thing an author should have to know.

**Suggested, and smaller than it sounds:** truncate on a word boundary. It
does not raise the limit or change the contract; it stops the output looking
like the author wrote half a sentence. Not filed with the accentcms report —
found after it went out (§8).

## 3. B2 — a real gap: sections are alphabetical, `menu.order` is ignored

`src/render/llmstxt.rs:84` groups pages into a `BTreeMap` keyed by the
top-level URL segment, so section order is alphabetical by *directory name*:

```rust
let mut grouped: std::collections::BTreeMap<String, Vec<&&Page>> = ...;
for page in &eligible {
    if let Some(segment) = top_level_segment(&page.url) { ... }
}
```

yqr's pages already carry `menu.order` — guide 2, compare 3, demo 4 — and
`llms.txt` publishes compare, demo, guide. So the ordering the site navigation
uses exists and is not consulted here.

It matters because the first section is what a context-limited reader is
surest to see, and for yqr that is a page comparing the tool to a competitor
rather than the guide that teaches its use.

**Suggested:** honour `menu.order` for section ordering, falling back to
alphabetical. Renaming directories is the only workaround and it changes
published URLs.

## 4. B3 — half of it exists; demotion does not

`llms.exclude_paths` already accepts globs and already covers **both** files —
`is_llms_eligible` is called at `llmstxt.rs:81` for `llms.txt` and at `:199`
for `llms-full.txt`. `noindex`, draft and review status, and error pages are
filtered there too.

What does not exist is **demotion**. The `## Optional` section is emitted only
from `llms.extra_links`, so there is no way to say "keep these pages in the
file but below the fold". The llms.txt convention gives `## Optional` the
meaning *skip when short on context*, which is the right home for reference
material a reader can do without.

**Suggested:** an `llms.optional_paths` glob list, rendered into `## Optional`
alongside `extra_links`.

yqr no longer needs this — `yqr-f021` moved the spec tree to a separate
local-only site, which removes it from the sitemap and the search index too,
not just from these two files. The gap is still real for anyone who wants the
pages published but deprioritized.

## 5. B4 — not a defect: it is a documented cap

The review asked whether the spec listing stopping after `b005` was a
per-section cap, pagination, or a sort hitting a limit. It is the cap, and it
is configured:

```rust
// src/config/llms.rs:52 — "Sections beyond this cap are truncated"
#[serde(default = "default_max_links")]
pub max_links_per_section: usize,   // default 10
```

Combined with the sort at `llmstxt.rs:102` — `parsed_date` descending, then
title, then URL — spec pages carry no date, so they tie on date and sort by
title. Ten alphabetically-first spec pages, then a stop. Exactly what was
observed.

The review's complaint that the sample is misleading stands; the fix is
configuration, not code. Worth noting only because `llms.txt` gives no
indication that a section was truncated.

**Suggested, optional:** when a section hits the cap, say so. One line —
`- ... and N more` — would stop a reader inferring the list is exhaustive.

## 6. B5 — the casing was the consumer's; the duplication is a defect

`section_title` (`llmstxt.rs:306`) takes the heading from the section's index
page title, falling back to a title-cased segment:

```rust
if let Some(index) = group.iter().find(|p| p.url == index_url) && !index.title.is_empty() {
    return index.title.clone();
}
title_case(segment)
```

So `yqr Demo` against `yqr guide` was yqr's own inconsistency, in the pages'
titles. Fixed in `yqr-f021`, not here.

**The duplication is real.** The index page supplies the heading *and* stays in
the group, so it is also emitted as the section's first entry:

```text
## How yqr compares to other YAML tools
- [How yqr compares to other YAML tools](/yqr/compare): There are good YAML tools already...
```

The heading and the entry are the same page. A reader sees the title twice and
spends one of a capped ten entry slots on it.

**Suggested:** when the index page supplies the heading, drop it from the
group's entries. Its content is the section, which the following entries
enumerate.

## 7. Impact on yqr

None that is live. `yqr-f021` addressed the packaging by splitting the sites,
and B1's finding **unblocks** the one item that was waiting: yqr's pages need
`lead:` frontmatter, which works today. That is `yqr-f021` §6 item A2, which
this spec corrects — it was recorded as blocked on B1, and it is not.

## 8. Route

Taken 2026-08-21, and by accentcms's own convention rather than as an issue:
that repository tracks bugs as specs under `specs/bugs/` and features under
`specs/features/`, so the report is **accentcms `b190`** and the demotion gap
is **accentcms `f328`**, both in accentcms#1240.

The split follows §8's reasoning. `b190` carries §3, §4 and §6 — a missing
key, a papercut and a defect in the same twenty lines — and states §2 and §5
as measured non-defects, since a report listing five bugs where two do not
exist would waste the maintainer's time and cost the other three their
credibility. The demotion gap became `f328` rather than a fourth bullet,
because it is a feature request and does not belong beside defects.

Two things the filing added that this spec lacked:

- **`b131` is the precedent for the `description:` fallback.** accentcms
  already fixed the mirror of §2 — `resolve_description` was taught to consult
  the typed `lead:` field, on the grounds that a page should not author the
  same sentence twice. The same argument runs the other way, which turns a
  suggestion into a consistency argument against their own code.
- **`f122` (Open) adds a third field to the same chain**, a `summary`
  frontmatter key that is specified to take precedence over `lead` and
  `description` in `llms.txt`. Whichever of the two ships second has to check
  the other's assumptions; `f328` says so rather than leaving it to collide.
