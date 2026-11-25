# Bug b041 — A lone `additionalProperties: false` reports only the first extra key

**Status:** Open — filed 2026-11-26
**Severity:** Low — the verdict is right (exit 1), but the report names one
value instead of the keys, and leaves the other keys out. You find them
one run at a time.
**Component:** `src/validate/schema.rs` (consumes `iter_errors`), and
jsonschema's `additionalProperties` validator
**Related:** `yqr-f040` (`validate --schema`), `yqr-f053` (the jsonschema
0.63 adoption whose code review found it)

## 1. What was observed

With `additionalProperties: false` and no `properties` beside it, a mapping
with two keys gets one violation:

```text
$ cat schema.yaml
additionalProperties: false
$ printf 'a: 1\nc: 2\n' > doc.yaml
$ yqr validate --schema schema.yaml doc.yaml
error[Y201]: False schema does not allow 1
  --> doc.yaml:1:1
  |
1 | a: 1
  | ^
  = note: at the document root
```

The message names the value `1`, not the key `a`, and nothing is reported
for `c`.

With `properties: {x: true}` beside it, the same document gets one
violation that names both keys:
`Additional properties are not allowed ('a', 'c' were unexpected)`.

## 2. Where it comes from

yqr renders every error `iter_errors` yields and has no deduplication. A
probe that links jsonschema directly shows `iter_errors` itself yields one
error, `False schema does not allow 1` at the root, on both 0.61.2 and
0.63.1. The cause is upstream, and it predates `f053`.

A search of the upstream issues on 2026-11-26, for
`False schema does not allow` and for `additionalProperties false`, found
no report of it.

## 3. What to do

- Report it upstream with the two-schema reproduction above.
- Do not work around it in yqr. Rewriting the schema to add an empty
  `properties` would change what the user wrote, and a fix upstream
  needs no yqr change.
- When a release fixes it, add a pin in `src/validate/schema.rs` that
  each extra key is reported.

## 4. Acceptance criteria

- [ ] Reported upstream.
- [ ] Fixed in a published jsonschema release and adopted.
- [ ] A pin that each extra key under a lone `additionalProperties: false`
      is reported.
