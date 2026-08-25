# Marketing k001 — Content plan: four task pages on the fidelity axis

**Status:** Done (2026-08-14) — all four pages shipped; §6 records what was
built and what deliberately was not. Refreshed 2026-08-24: §7's re-measure
trigger fired and one claim about yq had to be withdrawn (§8).
**Owner:** yqr maintainers
**Last updated:** 2026-08-24
**Related:** `yqr-f010` (the Accent site these pages live in), `yqr-f012`
(`validate`, which page 4 documents), `yqr-f009` (fidelity by default, which
page 3 explains), `yqr-f001` (the filter grammar that bounds what the pages
can honestly show)

## 1. Why

The site has three real pages — home, demo, specs index — against 39 indexed
spec pages. Search engines therefore see a site mostly about internal bug
tracking, and a reader who arrives has nowhere to go between "what is this"
and "read the specs".

Two things follow from that, and only one of them is marketing. The pages
below are also documentation yqr owes its users regardless: `validate` shipped
in v0.5.0 and is documented nowhere outside the spec tree.

## 2. The positioning, corrected by measurement

The obvious pitch — "yq reformats your YAML, yqr doesn't" — **is not true**,
and a content plan built on it would collapse the first time a reader tried
yq. Measured against yq v4.53.3 before writing anything:

On a realistic Deployment edit (`.spec.replicas = 5`), yq preserved comments,
quoting, key order and indentation. On scalar reads it matches yqr exactly:
`0640` stays `0640`, `1.10` stays `1.10`. yq is a careful, capable tool and
the pages must say so.

Where the two genuinely differ is whole-file normalization. Identity
round-trip (`'.'`) over a file with anchors and blank lines:

| | result |
|---|---|
| `yqr '.' fid.yaml` | **byte-identical to the input** |
| `yq '.' fid.yaml` | three changes |

```
  mode: 0640      # octal   ->   mode: 0640 # octal     comment alignment collapsed
  <blank line>              ->   (removed)
  <<: *d                    ->   !!merge <<: *d         tag injected
```

**As measured on 2026-08-14 against yq v4.53.3.** The third change is gone
as of yq v4.53.4 — see §8. The axis survives on the first two, which is the
point of choosing an axis with more than one leg under it.

So the honest axis is **diff noise**: lines you did not touch change, and a
reviewer has to read them. That is a narrow claim, it is true, and it is the
one thing yqr does that yq does not. Every page is written on it.

**Corollary:** do not invite a feature-matrix comparison. yq wins that by a
wide margin, and pretending otherwise makes the fidelity claim look like
spin too.

## 3. What the pages may promise

Bounded by what ships today. **Re-checked 2026-08-24 against v0.7.1**, because
this list is the one part of the plan that rots on every release — and it had:
six of the eight things the 2026-08-14 version told the pages not to imply have
since shipped.

**Available:** identity, field access, `.["key"]` (including keys holding a `.`
or a `/`), indexing including negative, `.[]`, pipe, `f?`; `=`, `|=`, `+=`,
new-key assignment, `del(...)`; arithmetic `+ - * / %`, with `+` also
concatenating strings, and computed right-hand sides (`.n = .n + 1`);
`to_entries`; comment editing (`line_comment`, `head_comment`), key rename
(`key(...)`), sequence reorder (`swap`, `move`); `-i`; `validate` and
`validate --strict`; `-r`; `--normalize`.

**Not available, and not to be implied:** object/array construction, string
interpolation, the comma operator; builtins beyond `to_entries` — no `select`,
`map`, `length`, `keys`; comparisons and boolean operators (`yqr-f008` §6);
conditionals; more than one edit per run; format conversion (JSON, XML, TOML);
collection right-hand sides.

Every example on every page must be runnable against the released binary. The
home page already holds itself to this; the new pages inherit it.

## 4. The pages

| Page | URL | Intent it serves |
|---|---|---|
| 1. Compare with yq | `/compare/yq` | "yq vs yqr", "yaml editor that keeps comments" |
| 2. Kubernetes recipes | `/guide/kubernetes` | "update image tag yaml without reformatting" |
| 3. Byte-for-byte guide | `/guide/fidelity` | "yaml edit preserve formatting", explains `--normalize` |
| 4. Validate | `/guide/validate` | "validate yaml command line", "find duplicate yaml keys" |

Page 1 is the ranking bet and the one that concedes most to yq. Page 4 is the
pure documentation debt. Pages 2 and 3 are the connective tissue: 2 is the
concrete task, 3 the concept it rests on.

**Page 1 restructured 2026-08-25.** It was organised as an argument — "here
is the thing yqr does that yq does not", building through agreement to
divergence. It is now organised as **routing**: a job-to-tool table up front,
then a section of evidence for each tool's jobs. Same measurements, same
concessions, and yq still gets the longer list of jobs; what changed is that
a reader arrives asking "which one do I use" and gets the answer in the first
screen instead of at the end of an argument. §2's corollary still holds — the
table routes by *job*, never by feature count, because a feature matrix is a
comparison yq wins.

## 5. Information architecture

```
/                 home
/guide/           fidelity, kubernetes, validate
/compare/         yq
/demo/            existing
/specs/           existing, mounted
```

Two new nav entries rather than four, so the header stays legible.

## 6. Outcome

**Shipped.** All four pages, plus `/guide/` and `/compare/` section indexes.
Every command in them was run against a real build and its output pasted
verbatim; the yq comparisons were run against yq v4.53.3.

**`noindex` on the spec tree — done.** The site now offers 8 indexable pages
(home, guide index and its three pages, compare index and its page, demo) and
noindexes all 41 spec URLs.

Implemented in the theme's `head_meta` partial rather than per file:
`page.custom.noindex` is frontmatter-only, so the alternative was adding
frontmatter to 39 specs and to every spec written afterwards. The rule matches
the `/specs` path segment rather than a hardcoded `/yqr` prefix, which changes
if the site moves to its own domain.

It emits **`noindex,follow`**, not the `noindex,nofollow` used for drafts. The
pages should still be crawled: that is how the noindex is seen at all, and
links out of specs still count.

**Known gap — closed 2026-08-24, by a change made for another reason.** The
gap was that spec URLs stayed in `sitemap.xml`, 40 of 49 entries, because
accent 0.24.0 had no `sitemap.exclude` key (verified then by adding one and
watching it be silently ignored) and the specs took their `noindex,follow`
from a URL-prefix rule in the theme rather than from frontmatter, which is
the only exclusion accent applies.

None of that was solved. `yqr-f021` split the public site from the spec site,
so the spec tree is no longer mounted in `docs/` at all — it has its own
local-only site on port 4401 that nothing deploys. The public sitemap is now
**9 entries, none of them specs**, measured against the current build. The
upstream ask from this paragraph (per-mount default frontmatter, or a sitemap
exclusion keyed on URL prefix) is no longer something yqr needs; it is left
recorded because the next site that mounts a tree will want it.

**Deliberately not done:**

- **A feature matrix against yq.** §2 explains why.
- **Anything requiring M1/M2 grammar.** §3.

## 7. Maintenance

The pages state a version-sensitive claim (what yq does to a file). When yq
changes, that claim needs re-measuring rather than re-asserting — the yq
version is named on the comparison page so a reader can tell how stale it is.

**Open trigger:** the compare index measures `noyafmt 0.0.27`; noya-cli is at
0.0.28 as of 2026-08-24. Unmeasured, so the page still names 0.0.27, which is
the honest stamp — bumping the number without re-running the commands is the
failure this section exists to prevent.

## 8. Re-measured 2026-08-24 — one claim withdrawn

§7 fired for the first time. yq moved v4.53.3 → **v4.53.6** and the comparison
was re-run in full, against a scratch 4.53.6 binary rather than an upgraded
system one, so the old version stayed available for the diff.

**The `!!merge` claim is dead.** yq no longer injects a tag on a round trip:

```console
$ yq-4.53.3 '.' anchors.yaml | diff anchors.yaml -   # 3 changes
$ yq-4.53.6 '.' anchors.yaml | diff anchors.yaml -   # 2 changes
```

It was a **regression on yq's side**, fixed upstream in v4.53.4 — its release
notes name it: *"Fix !!merge tag regression for yq (#2705)"*, dated
2026-08-19, five days after this plan measured it. yqr had been publishing a
bug that no longer existed.

Everything else held: the replicas edit still collapses only the comment
gutter, reads still agree on `0640` and `1.10`, the blank line is still
deleted, the comment-shape-on-a-key is still a silent no-op, yq's
`head_comment` still lands below the entry, and a `reverse` still re-emits the
document.

**What the page says now.** Two changes, not three. The withdrawn claim is
**gone from the page entirely**, not annotated: a reader deciding between two
tools has no use for what one of them used to do, and a paragraph explaining a
correction reads as a page arguing with itself. The correction belongs here,
in the record, which is what this section is. §2's original table is kept as
the 2026-08-14 measurement with a pointer to §8.

**Two stale claims in yqr's own favour were found in the same pass**, both on
`/compare/yq` and both understating yqr: *"No `map`, `select`, `length`, `+`
on values"* (arithmetic shipped in v0.7.0) and *"computed right-hand sides
live on the yq side of the line"* (`.n = .n + 1` and `.n |= (. + 1)` both
work). Under-promising is the cheaper error but it is still an inaccuracy, and
it comes from the same cause as the yq one: a page measured once and left.

**The lesson for §7.** Both directions rot. A release makes yqr's own limits
list wrong; an upstream release makes the competitor claims wrong. The check
belongs in the release checklist, not in whoever happens to look.
