# Bug b012 — A new key cannot be inserted into a mapping whose keys all hold a `.`

**Status:** Open (found 2026-08-18 by the `yqr-m003` write tier; not filed
upstream as part of this change)
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

## 4. Fix routes

1. **Upstream, narrow:** re-word the error to describe what was observed and
   list the keys that could not be addressed. Removes the false cause; leaves
   the limit.
2. **Upstream, real:** give `mapping_insert_anchor` a path-free way to reach a
   key's span (walk the green tree, or take a `&GreenNode` rather than a
   composed string). This also removes the limit for `set_value`, `remove`,
   `rename_key` and `swap_items`, which compose paths the same way — the
   `yqr-f007` §6 item in full.
3. **yqr-side:** own the anchor arithmetic, on the `f007` §2 last-resort route.
   Not preferred: it duplicates upstream's indentation model for one case.

Route 2 is the one worth filing; route 1 is worth filing regardless, being a
message change with no semantics attached.

## 5. Regression coverage

`tests/corpus/mod.rs`, case `write/insert/refuses-a-mapping-whose-keys-hold-a-dot`,
pins the refusal (exit 5) against `K8S_DEPLOYMENT`. Its sibling
`write/insert/new-key-under-a-nested-mapping` pins the working insert into the
plain-keyed `.spec.template.metadata.labels` block of the same document, so the
two differ only in the keys' spelling. When this is fixed, the first case turns
into a `Rewrites` expectation and fails until it is updated — which is the
intent.
