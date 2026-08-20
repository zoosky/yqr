# Bug b012 — A new key cannot be inserted into a mapping whose keys all hold a `.`


> **Historical: resolved.** yqr no longer behaves as described below. The
> **Status** line records what fixed it and when; the rest is kept as the
> reproduction and the reasoning, written in the present tense of the time it
> was filed.

**Status:** Resolved — filed upstream 2026-08-19 as noyalib#288, fixed in
noyalib#289, **released in noyalib 0.0.25** (2026-08-20) and verified against
the published crate by `yqr-f019` §3.2. `.metadata.labels.<new> = ...` on a
Kubernetes manifest writes, and the `<<` merge diagnostic is gone with the
refusal that produced it. The `yqr-f007` §6 addressing limit it sat on top of
is **not** closed by this: `set`, `del`, `key(...)` and the reorder verbs still
route a key through `parse_query_path`, so writing an *existing* dotted key is
still refused — now saying so accurately ("it uses characters the write path
cannot express")
**Severity:** Medium — a refusal, not damage (exit 5, file untouched), but the
shape is the standard Kubernetes label/annotation block, and the diagnostic
names a cause that is not present in the document
**Component:** noyalib's `Document::mapping_insert_anchor` (upstream), reached
from yqr's new-key assignment via `insert_key`
**Related:** `yqr-f007` §6 (the "keys that hold `.` or `[`" item — this is its
insert-anchor face, and the measurement there recorded `insert_key` refusing
without recording *why*), `yqr-a002` §7.3, `yqr-b004`, `yqr-m003`

## 1. Summary

Assigning a new key under a mapping whose every existing key contains a `.`
is refused, and the refusal blames a merge key the document does not have:

```console
$ cat labels.yaml
labels:
  app.kubernetes.io/name: web
  app.kubernetes.io/component: frontend
$ yqr '.labels.tier = "web"' labels.yaml
yqr: runtime error: cannot insert key "tier": YAML parse error: no entry of the
mapping at `labels` has source bytes of its own to anchor indentation on (every
key is inherited through a `<<` merge) — use `set` with a fragment instead
$ echo $?
5
```

There is no `<<` in the file. The same insert into a mapping with plain keys
succeeds:

```console
$ printf 'labels:\n  app: web\n' | yqr '.labels.tier = "web"'
labels:
  app: web
  tier: web
```

## 2. Root cause

`mapping_insert_anchor` (noyalib 0.0.23, `src/cst/document.rs`) picks the
indentation anchor by walking the mapping's keys from the back and asking for
each one's span:

```rust
let anchor = keys.iter().rev().find_map(|key| {
    let child = if path.is_empty() { key.clone() } else { format!("{path}.{key}") };
    self.span_at(&child)
});
```

The candidate path is composed as a **string** and re-parsed. `span_at` routes
through `parse_query_path`, which splits on `.`, `[` and `*` unconditionally
and has no escape or quoting form (`yqr-f007` §6). So `labels.app.kubernetes.io/name`
is read as five segments, resolves to nothing, and every key of a dotted
mapping is skipped. With no anchor left, the error path fires — and its message
states the *only* cause the function was written to expect, a mapping whose
keys are all inherited through `<<`.

Two defects, then, one inside the other: an addressing limit that reaches a
mapping it was never meant to reach, and a diagnostic that asserts a cause
rather than describing the observation ("no entry has bytes of its own").

## 3. Impact

- `.metadata.labels.<new> = ...` and `.metadata.annotations.<new> = ...` on a
  Kubernetes manifest — the most common mapping in the ecosystem — cannot be
  written by yqr.
- The message sends the reader looking for a merge key, and the suggested
  workaround (`use set with a fragment instead`) names an API yqr does not
  expose on this path.
- Nothing is silently wrong: the edit is refused, `-i` leaves the file alone.

## 4. Fix routes — 1 and 2 taken 2026-08-19

1. **Upstream, narrow:** re-word the error to describe what was observed.
   **Done** as part of noyalib#289.
2. **Upstream, real:** give `mapping_insert_anchor` a path-free way to reach a
   key's span. **Done** in noyalib#289, and by a route this spec did not
   anticipate: not the green tree but the **span tree**, which already holds
   the target mapping's entries with their spans. The fix adds `resolve_tree`
   — the span-tree twin of the existing `resolve_span`, for callers that want
   the addressed node's structure rather than its span — and the anchor search
   walks those entries directly. No path is composed, so the key's spelling
   stops mattering.
3. **yqr-side:** own the anchor arithmetic. Not taken, and now moot for insert.

### 4.1 What §3's measurement missed

This spec recorded "all three write paths refuse". That was right, but it hid
that the insert refusal has **two** independent causes in upstream, not one:
`mapping_insert_anchor` composes a path per candidate key, and `insert_entry`
duplicates the same logic inline rather than calling it. Fixing only the first
left `insert_entry` failing with a *different* message
(`could not resolve last entry span`). The fix consolidates them.

### 4.2 Three things the fix settles beyond this bug

Each is a shape the anchor search reached only once it stopped going through
path strings:

- Keys holding `[` or `*`, and quoted keys such as `"a.b"`, insert correctly
  for the same reason.
- A mapping whose last entry is an **implicit null** (`b:` with no value) now
  anchors on that entry's own key line, so a new sibling lands after it.
  `insert_entry` used to refuse such a mapping outright and `insert_entry_value`
  inserted *above* the null.
- A mapping with both a `<<` merge and an entry of its own anchors on the
  entry. A merge-only mapping still refuses, with a message that now describes
  the observation.

### 4.3 What is still out of reach

Insert only. `set_value`, `delete`, `rename_key` and the reorder verbs still
address through `parse_query_path`, so on the 0.0.24 pin yqr still answers

```console
$ yqr '.metadata.labels["app.kubernetes.io/name"] = "api"' deploy.yaml
yqr: runtime error: cannot address key "app.kubernetes.io/name": it uses
characters the write path cannot express
```

Whether the path grammar grows an escape form is the `yqr-f007` §6 question and
is untouched here. §3's "all three write paths refuse" therefore becomes "two
of the three still do".

## 5. Regression coverage

`tests/corpus/mod.rs`, case `write/insert/refuses-a-mapping-whose-keys-hold-a-dot`,
pins the refusal (exit 5) against `K8S_DEPLOYMENT`. Its sibling
`write/insert/new-key-under-a-nested-mapping` pins the working insert into the
plain-keyed `.spec.template.metadata.labels` block of the same document, so the
two differ only in the keys' spelling. When this is fixed, the first case turns
into a `Rewrites` expectation and fails until it is updated — which is the
intent.

**That trigger fired, on purpose.** Run against the patched crate through a
path override, the corpus is green except that one case, which now reports the
successful insert instead of the refusal:

```text
[write/insert/refuses-a-mapping-whose-keys-hold-a-dot] expected a refusal:
"...    app.kubernetes.io/component: frontend\n    tier: \"web\"\n..."
```

So the case flips to `Rewrites` the day a release carries the fix — and note
the inserted value arrives double-quoted, which is `yqr-b013` and unrelated to
this one. `docs/content/guide/kubernetes.md` also carries a limitation note
naming this bug, and it comes out with the same change.
