# Marketing k001 — Content plan: four task pages on the fidelity axis

**Status:** Done (2026-08-14) — all four pages shipped; §6 records what was
built and what deliberately was not.
**Owner:** yqr maintainers
**Last updated:** 2026-08-14
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

Where the two genuinely differ is whole-file normalisation. Identity
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

So the honest axis is **diff noise**: lines you did not touch change, and a
reviewer has to read them. That is a narrow claim, it is true, and it is the
one thing yqr does that yq does not. Every page is written on it.

**Corollary:** do not invite a feature-matrix comparison. yq wins that by a
wide margin, and pretending otherwise makes the fidelity claim look like
spin too.

## 3. What the pages may promise

Bounded by what ships today (`yqr-f001` M0 plus the write tier):

**Available:** identity, field access, `.["key"]`, indexing including
negative, `.[]`, pipe, `f?`; `=`, `+=`, new-key assignment, `del(...)`; `-i`;
`validate` and `validate --strict`; `-r`; `--normalize`.

**Not available, and not to be implied:** object/array construction, string
interpolation, the comma operator (M1); builtins and arithmetic (M2); `|=`
(`yqr-f008`); comment editing, key rename, sequence reorder (`yqr-f007` §6);
collection right-hand sides; keys containing `.` or `[`.

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

**Deliberately not done:**

- **`noindex` on `/specs/*`.** Still 39 of 45 indexed URLs. Recommended, but
  it changes what the site publishes and is the owner's call, not a content
  decision. The hook is `page.custom.noindex`.
- **A feature matrix against yq.** §2 explains why.
- **Anything requiring M1/M2 grammar.** §3.

## 7. Maintenance

The pages state a version-sensitive claim (what yq does to a file). When yq
changes, that claim needs re-measuring rather than re-asserting — the yq
version is named on the comparison page so a reader can tell how stale it is.
