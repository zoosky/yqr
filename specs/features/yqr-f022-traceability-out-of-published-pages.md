# Feature f022 — Move docs traceability out of the published page body

**Status:** Done — 2026-08-21
**Epic:** Project website (`f010`)
**Owner:** yqr maintainers
**Related:** `yqr-f021` §6 (review item A3, which this closes), `yqr-b017`
(the accent findings, one of which this depends on), `yqr-f136` / ground rule
19 (the rule this amends)

## 1. The problem with rule 19 as written

Ground rule 19 required feature and bug IDs in `docs/content/` to be wrapped
in HTML comments, "so they are invisible in rendered HTML but preserved for
grep". The premise is true of a browser and false of everything else:
`llms-full.txt` publishes each page's body verbatim, so all fourteen comments
were published as visible text. One of them named an internal document:

```text
<!-- Every command below was run against a real build; see specs/marketing/yqr-k001-content-plan.md -->
```

The rule was not being broken. **The rule was wrong**, in the specific sense
that its stated mechanism — invisibility — did not hold for the consumer that
mattered most, and nothing in the rule said which consumers it had been
checked against.

## 2. The fix: frontmatter, not the body

Frontmatter is stripped before a page's body is published, so a **YAML comment
inside the frontmatter** is invisible to every consumer and still greppable:

```yaml
---
# Traceability: Feature f017 (to_entries).
# Bug b016 -- the emitter's trailing space, pinned in tests/cli.rs.
title: Enumerating a mapping without losing the keys
---
```

Verified rather than assumed: a `# Feature f012` line added to one page's
frontmatter left `llms-full.txt` with zero occurrences of it, and the page
rendered unchanged.

This is better than the two options originally weighed — deleting the
traceability, or accepting the noise — because it gives up neither. The rule's
purpose was findability, not per-paragraph anchoring, and `grep -rn 'f017'
docs/` still answers "which pages relate to f017".

### 2.1 What was given up

Anchoring. `<!-- Feature f007 -->` sat directly above the section it described,
three times in the Kubernetes guide; the frontmatter carries one note for the
page. That is the right trade for the rule's actual question — *which page*,
not *which paragraph* — and where a marker carried more than an ID, the note
keeps the sentence rather than the position.

## 3. Scope

All fourteen comments across six pages: `index.md` (3, including the home
page's orientation block), `guide/enumerate.md` (2), `guide/fidelity.md` (2),
`guide/kubernetes.md` (4), `guide/validate.md` (2), `compare/yq.md` (2).

`compare/yq.md`'s was not traceability but an instruction to whoever edits the
page next — *"Re-measure rather than re-assert when yq changes"*. It moves to
frontmatter with the rest, since it is addressed to an editor and not to a
reader, and the spec path it cited is now the bare `yqr-k001`.

## 4. The upstream half

The generator should not publish HTML comments either, whatever a site does
about its own. Filed as accent `b190` §5 (`yqr-b017`): `llms-full.txt` emits
`page.raw_content` verbatim, while `html_strip::strip_html_blocks` — which
drops a comment whole — already exists in that repository and is already used
by `Page::lead()` in the same feature.

The two halves are independent on purpose. This feature stops yqr leaking
regardless of what upstream does; the upstream fix protects every other accent
site, including the ones that never read rule 19.

## 5. Acceptance criteria

- [x] No `<!--` in `docs/content/`.
- [x] No `<!--` in the published `llms-full.txt`.
- [x] Every ID removed from a body comment survives in the page's frontmatter,
      so `grep` still finds it.
- [x] The internal `specs/marketing/...` path is gone from published output.
- [x] Ground rule 19 states the new mechanism **and why the old one failed**,
      so the next person does not reintroduce it reasoning from "invisible in
      HTML".
- [x] Both sites build clean under `--strict-links`; no rendered page changes.
