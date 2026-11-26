# Bug b041 — A lone `additionalProperties: false` reports only the first extra key

**Status:** Open — filed 2026-11-26; reported upstream as
[jsonschema #2046](https://github.com/Stranger6667/jsonschema/issues/2046),
fix proposed in [#2047](https://github.com/Stranger6667/jsonschema/pull/2047)
**Severity:** Low — the verdict is right (exit 1), but the report names one
value instead of the keys, and leaves the other keys out. You find them
one run at a time.
**Component:** jsonschema's `AdditionalPropertiesFalseValidator`
(`crates/jsonschema/src/keywords/additional_properties.rs` upstream);
`src/validate/schema.rs` only renders what `iter_errors` yields
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
An empty `properties: {}` does the same, measured on yqr 0.15.0 on
2026-11-26, so a schema author can get the full report today.

## 2. Where it comes from

The cause is in jsonschema. yqr renders every error `iter_errors` yields
and has no deduplication.

Without `properties` or `patternProperties`, `additionalProperties: false`
compiles to `AdditionalPropertiesFalseValidator`:

- Its `validate` returns a false-schema error for the first member's value,
  located at the mapping.
- It does not override `collect_errors`. The default pushes the one error
  from `validate`, so `iter_errors` yields that error alone.

Every other `false` variant in that file reports an `additionalProperties`
error that names the unexpected keys. jsonschema 0.8.0 (2021) moved those
variants to that message and left this one; the code at the `rust-v0.8.0`
tag shows the split. The `#[jsonschema::validator]` codegen has the same
gap and reports the whole object instead, but yqr does not use it.

A program that links jsonschema 0.63.1, the latest release, directly
reproduces it. `{"additionalProperties": false}` yields
`False schema does not allow 1`, and
`{"properties": {}, "additionalProperties": false}` yields
`Additional properties are not allowed ('a', 'c' were unexpected)`.
Upstream `master` at `f23ff311` (2026-11-26) behaves the same. 0.61.2 gave
the same single error.

## 3. What to do

- Reported upstream as #2046 on 2026-11-26. None of the nine open issues
  covered it, and a search of the closed ones found no report.
- The fix, proposed in #2047, makes the lone case report as the
  `properties` variants do. `validate` names the first key, and
  `iter_errors` names every key in one `additionalProperties` error.
  `is_valid` and `evaluate` are unchanged. The error kind changes from
  `FalseSchema` to `AdditionalProperties`.
- Do not work around it in yqr. Rewriting the schema to add an empty
  `properties` would change what the user wrote, and a fix upstream
  needs no yqr change.
- When a release carries the fix, adopt it, measuring as in `f053`. Add a
  pin in `src/validate/schema.rs` that the two-key document above gives one
  `Y201` naming both keys, as `properties: {}` does today.

## 4. Acceptance criteria

- [x] Reported upstream (#2046, fix proposed in #2047).
- [ ] Fixed in a published jsonschema release and adopted.
- [ ] A pin that each extra key under a lone `additionalProperties: false`
      is reported.
