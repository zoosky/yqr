# Feature f024 — Adopt accent 0.25.0: the five `llms.txt` findings, fixed

**Status:** Done — 0.25.0 adopted, all five findings verified against the
released binary, `b017` closed (2026-08-21)
**Epic:** Project website (`f010`)
**Owner:** yqr maintainers
**Related:** `yqr-b017` (the findings this closes), `yqr-f021` (the site split
that came out of the same review), `yqr-f022` (which stopped yqr depending on
finding 5), `yqr-m001` (the workflow pin)

## 1. Scope

Move `ACCENT_VERSION` from `v0.24.0` to `v0.25.0` and close `b017`.

0.25.0 carries the five findings yqr filed as accentcms `b190`. All five are
verified against the **released binary**, per finding rather than from the
release notes.

## 2. Verification, run 2026-08-21 against accent 0.25.0

### 2.1 The index page is no longer its own first entry

Before, a section's heading and its first entry were the same page, spending
one of ten capped slots on a duplicate title. Now the heading stands alone and
the entries enumerate the section:

```text
## yqr guide
- [Byte-for-byte YAML editing, explained](/yqr/guide/fidelity): ...
- [Editing Kubernetes manifests without reformatting them](/yqr/guide/kubernetes): ...
```

**One edge case behaves sensibly and is worth recording:** `/demo` has no child
pages, so its index *is* the section. It is still listed, because dropping it
would leave an empty heading. Correct, and not what the finding asked to
change.

### 2.2 Sections order by `menu.order`

yqr's pages carry guide 2, compare 3, demo 4, specs 5. Published order is now
guide, compare, demo, specs — previously alphabetical by directory, which put
a competitor comparison first and the guide third.

### 2.3 A capped section says so

The internal site's specs section is 61 pages against a cap of 10:

```text
- [Bug b003 — rust-yaml fork `RoundTripDocument::parse_all` errors ...](/specs/bugs/...): ...
- ... and 51 more
```

A reader can no longer mistake the sample for the whole.

### 2.4 An over-long entry cuts on a word boundary

Tested by temporarily lengthening one page's `lead:` past the 120-character
budget:

```text
... what each of the exit codes means in…
```

Cut after a whole word, not mid-word. This is the finding yqr found by *acting*
on its own report — writing nine leads and hitting the budget.

### 2.5 HTML comments stay out of `llms-full.txt`

Tested with a page authored for the purpose, since `yqr-f022` had already
removed every comment from `docs/content`:

| | |
|---|---|
| `<!-- INTERNAL -->` in `llms-full.txt` | **0** |
| the same comment in the rendered HTML | 1 |

Both correct. A comment belongs in HTML and does not belong in a text feed for
machines.

## 3. What this does not undo

`yqr-f022` moved traceability from HTML comments into frontmatter YAML
comments, and **that stays**. The generator fix makes it defence in depth
rather than the only defence, which is the right order: a site should not have
to depend on its generator declining to publish something the site did not
want published.

Ground rule 19's wording stands for the same reason. It now describes a
mechanism that is correct on its own merits rather than one working around a
defect.

## 4. One loose end closed along the way

The internal site's spec section was headed `yqr Specs` against `yqr guide`
and `yqr demo` — the same casing inconsistency `yqr-f021` fixed on the public
site and missed here, because the section only exists on the internal one.
Now `yqr specs`, and all four headings agree.

## 5. `b017` closes

All five findings fixed and verified. What remains open upstream is
accentcms `f328` — the `llms.optional_paths` demotion — which yqr filed and
does not need: `yqr-f021` moved the spec tree to a separate site, which takes
those pages out of the sitemap and the search index too. It stays a real gap
for anyone who wants the pages published but deprioritized.

## 6. Acceptance criteria

- [x] `ACCENT_VERSION` moved to `v0.25.0`; the release verified to exist at
      the URL `pages.yml` fetches from, with the Linux asset `pages.yml` names.
- [x] All five findings verified against the released binary, per finding
      (§2), not taken from release notes.
- [x] The `/demo` edge case checked and recorded rather than reported as a
      regression.
- [x] `docs/config.yaml`'s and `pages.yml`'s version-floor comments say what
      each floor buys, so the next bump can tell load-bearing from incidental.
- [x] `b017` moved to Resolved, with `f328`'s status stated rather than left
      implied.
- [x] Both sites build clean under `--strict-links`.
