# Bug b035 — An entry whose value is an alias cannot be deleted or overwritten

**Status:** Open — filed 2026-09-19, found while writing `yqr-f036`'s
removed-anchor refusal
**Severity:** Medium — nothing is corrupted, but a file with aliases has
entries yqr cannot change at all, and yqr's own refusals point at them
**Component:** write tier — `src/fidelity/write/delete.rs` (block path),
`backend.rs` `set_value`; upstream `Document::remove` and `set_value`
**Related:** `yqr-f036` (whose refusal names such an entry), `yqr-b026`
(the anchor-definition write), `yqr-f025` (a refusal names a remedy that
runs)

## 1. Summary

Every write that would change `j` in `a: &x 1` / `j: *x` is refused.

| input | filter | result |
|---|---|---|
| `a: &x 1` / `j: *x` / `z: 2` | `del(.j)` | refused: "its source layout is not supported" |
| `a: &x 1` / `j: *x  # c` / `z: 2` | `del(.j)` | the same |
| `l:` / `  - &x 1` / `  - *x` / `  - 3` | `del(.l[1])` | the same |
| `a: &x 1` / `m: {j: *x, z: 2}` | `del(.m.j)` | the engine's refusal: "its source bytes belong to the anchor, not to this entry" |
| `a: &x 1` / `j: *x` / `z: 2` | `.j = 5` | the engine's refusal: "edit the anchor definition or replace the alias explicitly" |

Measured on noyalib 0.0.45, on `main` and on the `f036` branch alike.

## 2. Why it matters now

`yqr-f036` refuses an edit that would remove an `&x` definition while a
`*x` outside the edit still refers to it. That refusal is right: the
alternatives are an alias pointing at nothing or yqr rewriting bytes the
path does not name. Its natural remedy is to remove or change the `*x`
first, and yqr cannot do either. So the message asks the user to edit the
file by hand, where `yqr-f025` wants a remedy that runs.

## 3. Likely cause

For an alias, `span_at` returns the **anchor's** bytes, not the `*x`
token's. `yqr-f037` §2.1 met the same fact on the comment side. The block
delete derives its range from that span, so it lands somewhere that is not
the entry, and the layout check refuses. The engine's own `remove` and
`set_value` know the bytes are not the entry's and say so.

Not yet confirmed by stepping through `delete_entry`; confirm before
fixing.

## 4. What a fix looks like

The alias token is the entry's value in the source, and the engine already
lists it: `Document::aliases()` gives each `*name` with its byte span.

- **Delete:** derive the range from the key (or the `-`) and the alias
  token's span, and let the existing re-parse and value checks run as
  they do. The expected value is the document without the entry, and no
  other entry refers to an alias, so nothing else may change.
- **Assign:** splice the rendered scalar over the alias token, which
  replaces the reference with a value of its own. That is what "replace
  the alias explicitly" means, and it touches only the addressed entry.

Both reach bytes the path names and nothing else, so neither needs a new
policy. Once they work, `f036`'s removed-anchor message can name `del` as
its remedy.

## 5. Acceptance criteria

- [ ] `del` removes a block mapping entry and a block sequence item whose
      value is an alias, with and without a trailing comment.
- [ ] `=` replaces an alias with a scalar at a mapping key and a sequence
      item.
- [ ] The removed-anchor refusal in `anchor.rs` names the remedy that now
      runs, and a test runs it.
- [ ] `local-ci.sh` clean.
