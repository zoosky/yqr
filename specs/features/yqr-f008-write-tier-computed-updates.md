# Feature f008 — Write tier: computed updates (`|=`)

**Status:** Draft (stub — gated on `f001` M2)
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f006` (write tier v1 — the assignment/`-i` core this extends),
`yqr.f001` M2 (arithmetic + builtins — the right-hand evaluator this needs),
`yqr-m002` §4/§6.2 (write-tier seam)

> **Stub.** This feature cannot be built until the expression evaluator exists,
> so it is a roadmap marker only. It is scoped here to keep the epic's shape
> complete; detail is deferred until `f001` M2 lands.

## 1. Scope

jq's **update-assignment** operator: `p |= f`, which sets the node at path `p`
to the result of running filter `f` on its current value — e.g.:

- `.spec.replicas |= (. + 1)` (increment)
- `.image |= sub("nginx:.*"; "nginx:1.28")` (string transform)
- `.metadata.labels.env |= ascii_upcase`

Unlike `f006`'s `=` (whose RHS is a literal or a copied path), `|=` computes the
new value from the old one, so it **requires the expression evaluator** —
arithmetic, comparisons, and the string/collection builtins delivered by
`f001` M2.

## 2. Dependency

Hard-gated on **`f001` M2** (arithmetic/builtins). Until then, `f006` rejects
`|=` with a "not yet supported" message. Once M2 exists, `|=` reuses:

- `f006`'s path → span → `set_value` write path and its re-parse guard, and
- the M2 evaluator to compute the RHS value from the selected node.

## 3. Structural-integrity contract

Identical to `f006` §7 — the computed value is written via the guarded
`set_value`, so only the target node's bytes change or the edit is refused.

## 4. Acceptance criteria (outline)

- [ ] `.n |= (. + 1)` rewrites the scalar, byte-exact elsewhere.
- [ ] A `|=` whose RHS filter errors leaves the document unchanged.
- [ ] Style-matched quoting via `set_value` for computed string results.

_(Criteria firm up alongside the `f001` M2 evaluator.)_
