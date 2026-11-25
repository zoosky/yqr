# Feature f053 — Adopt jsonschema 0.63: bundled meta-schemas, ECMA 262 patterns, clearer messages

**Status:** Done — 2026-11-26
**Epic:** Editing-loop tooling (`f012`)
**Owner:** yqr maintainers
**Related:** `yqr-f040` (`validate --schema`), `yqr-f047` (the `schema`
cargo feature), `yqr-f052` (the last adoption spec, same method),
`yqr-m001` (quality gates); supersedes Renovate PR #207

## 1. What the releases carry

`jsonschema` moves from 0.61.2 to 0.63.1, across three releases: 0.62.0
(2026-11-21), 0.63.0 (2026-11-22), and 0.63.1 (2026-11-23). Their
changelog lists more than a hundred entries. Most of them are on paths
yqr does not call: `evaluate` output, the `#[jsonschema::validator]`
macro, `dereference`, canonicalization, HTTP options, and the new
`json-text`, `known_formats`, and number-format APIs.

yqr calls only `draft202012::new`, `Validator::iter_errors`, and each
`ValidationError`'s `Display`, `masked()`, and `instance_path()`
(`src/validate/schema.rs`). So a change reaches yqr only through five
things: whether a schema compiles (`Y202`), whether an instance passes,
the message text, and the order and the count of the errors on one node.

The one breaking removal, `ValidationErrorKind::RegexEngineFailure`, is a
kind yqr never matches on. The build needed no source change.

The lockfile moves seven packages: `jsonschema`, `jsonschema-regex`,
`jsonschema-value`, and `referencing` to 0.63.1, and `regex` (1.13.2),
`regex-automata` (0.4.19), and `regex-syntax` (0.8.12) beside them. The
`regex` crates reach yqr only through `jsonschema`, so the filter language
is not affected. The lockfile matches Renovate's #207.

## 2. What moved in yqr, measured

Every existing test passes unchanged, locally and on #207's CI. Each shape
below targets one changelog entry. Each ran through
`yqr validate --schema` on a 0.61.2 control build (main at `26bde17`) and
on 0.63.1, with Draft 2020-12 as the root dialect. The first pass missed
the lookbehind, negative-lookahead and widening rows; the PR's code review
found them, and they were re-measured on the same two builds.

| Shape (schema, instance) | 0.61.2 | 0.63.1 |
|--------------------------|--------|--------|
| `enum: []`, `x: 1` | Y201 `{"x":1} is not one of ` | Y201 `Empty "enum" does not allow {"x":1}` |
| `contains` string, `minContains: 2`, `[a, 1]` | Y201 `None of ["a",1] are valid under the given schema` | Y201 `["a",1] has less than 2 items valid under the given schema` |
| `contains` string, `maxContains: 1`, `[a, b]` | Y201 `None of ["a","b"] are valid ...` | Y201 `["a","b"] has more than 1 item valid ...` |
| `properties: {a: true}`, `required: [b]`, `additionalProperties: false`, `{a, c}` | additionalProperties error, then required | required error, then additionalProperties |
| the same without `properties` | required, then additionalProperties | unchanged |
| `allOf: [{$ref: "#"}]`, `type: string`, `1` | three Y201 | two Y201 |
| `$ref` to the Draft 7 meta-schema, `type: 5` | Y202, not present in a registry | Y201 at `/type` |
| `pattern: ^(?<n>x)\k<n>$`, `"xx"` | Y202, `is not a "regex"` | passes |
| `pattern: ^(\w)\1$`, `"éé"` | **passes** | **Y201**, does not match |
| `pattern: ^(\d)\1$`, `"١١"` | **passes** | **Y201** |
| `pattern: (?<=a)\w$`, `"aé"` | **passes** | **Y201** |
| `pattern: (?<!x)\w$`, `"é"` | **passes** | **Y201** |
| `pattern: ^(?!x)\w$`, `"é"` | **passes** | **Y201** |
| `pattern: (?<=a)\d$`, `"a١"` | **passes** | **Y201** |
| `pattern: ^(\s)\1$`, two U+0085 | **passes** | **Y201** |
| `pattern: ^(.)\1$`, `"\r\r"` and two U+2028 | **passes** | **Y201** |
| `pattern: (?<=a)\s$`, `a` and U+0085 | **passes** | **Y201** |
| `pattern: ^(?!x).$`, `"\r"` | **passes** | **Y201** |
| `pattern: ^(\W)\1$`, `"éé"` | Y201 | **passes** |
| `pattern: ^(\D)\1$`, `"١١"` | Y201 | **passes** |
| `pattern: (?<=a)\W$`, `"aé"` | Y201 | **passes** |
| `patternProperties: {^(\w)\1$: {type: integer}}`, `éé: s` | Y201 | **passes**: the key no longer matches, so its value is unchecked |
| `pattern: ^(?=.)\w+$`, `"é"` | Y201 | Y201, unchanged |
| `pattern: ^(\s)\1$`, two U+3000 | passes | passes, unchanged |
| `pattern: ^(?=.)\s$` and `^\s$`, U+0085 | Y201 | Y201, unchanged |
| `pattern: ^(?=\r).$` and `^.$`, `"\r"` | Y201 | Y201, unchanged |
| `not: {$ref: "#"}`, `x: 1` | Y201, rejects every instance | passes |
| `$ref` to the Draft 3 meta-schema | Y202 | Y202, reworded to name the missing `resolve-http` resolver |
| `unevaluatedProperties: false` beside a failing `anyOf` branch | Y201 | Y201, unchanged |
| `unevaluatedProperties: false` beside a failing `$ref` target | Y201 | Y201, unchanged |
| `pattern: ^\d+$` and `^(?=.)\d+$`, `"١٢"` | Y201 | Y201, unchanged |
| `type: [null, string]`, `5` | Y201 | Y201, unchanged |
| `$dynamicRef` to a recursive node | Y201 | Y201, unchanged |
| `propertyNames: false`, `{a, b}` | Y201 | Y201, unchanged |

A `$ref` to each meta-schema URI, measured the same way:

| `$ref` target | 0.61.2 | 0.63.1 |
|---------------|--------|--------|
| `https://json-schema.org/draft/2020-12/schema` | resolves | resolves |
| `https://json-schema.org/draft/2019-09/schema` | Y202 | resolves |
| `http://json-schema.org/draft-07/schema#` | Y202 | resolves |
| `http://json-schema.org/draft-06/schema#` | Y202 | resolves |
| `http://json-schema.org/draft-04/schema#` | Y202 | resolves |
| `http://json-schema.org/draft-03/schema#` | Y202 | Y202 |
| `https://json-schema.org/draft-07/schema#` (not the `$id`) | Y202 | Y202 |
| `http://json-schema.org/draft/2020-12/schema` (not the `$id`) | Y202 | Y202 |
| `https://json-schema.org/draft/next/schema` | Y202 | Y202 |
| `https://example.com/s.json` | Y202 | Y202 |

The bundled meta-schemas are compiled into the crate, so resolving one
fetches nothing. A `$ref` must spell the meta-schema's own `$id`: `http://`
for Drafts 4 to 7, `https://` for 2019-09 and 2020-12. yqr's no-network
posture (`f040`) holds: Draft 3, `draft/next`, another spelling of a
bundled URI, and every other URI are still refused.

Every refused `$ref` carries upstream's advice that a `resolve-http`
feature or a custom resolver is needed, which a yqr user cannot act on.
0.61.2 already said this for an arbitrary URL; 0.63.1 says it for the
Draft 3 and `draft/next` URIs too. It is filed as `yqr-b042`.

The two `--schema` console blocks in `docs/content/guide/validate.md`
print byte-identical output on 0.63.1.

### Pattern classes: both directions

JSON Schema patterns are ECMA 262. In a pattern with a backreference, a
lookbehind, or a negative lookahead, 0.61 matched `\w`, `\d`, `\s`, and
`.` with a regex engine's Unicode classes instead. 0.63 matches what
ECMA 262 defines: `\w` and `\d` are ASCII, `\s` leaves out U+0085, and
`.` does not match a line terminator. A pattern with none of those
features, and one with only a positive lookahead, already matched all
four classes as ECMA 262 defines them on 0.61 (the "unchanged" rows).

The verdicts move both ways:

- **Narrowing.** `\w`, `\d`, `\s`, and `.` match less, so a string that
  passed can fail. A file that validated before can now exit 1.
- **Widening.** `\W` and `\D` match more, so a string that failed can
  pass. A `patternProperties` key pattern matches fewer keys, so the value
  under a non-ASCII key is no longer checked at all.

The narrowing is the same class of change that made v0.14.0 a minor
release. yqr takes the fix as shipped, following `f052`: it conforms to
the specification the schema is written against, and keeping the old
match would need a non-ECMA regex path of yqr's own. The widening is the
cost of the same conformance and is documented rather than worked around.

`not: {$ref: "#"}` also flips, from rejecting every instance to passing
it. The schema contradicts itself, so neither answer is wrong; 0.63
matches how the same schema reached through a `$ref` behaves.

## 3. What changed in yqr

- **Five pins** in `src/validate/schema.rs`, each marked
  `// Feature f053`. Each fails on 0.61.2 and passes on 0.63.1:
  - `bundled_meta_schema_of_another_draft_resolves`: the Draft 7
    meta-schema resolves under its `$id`. Draft 3 and the `https://`
    spelling of Draft 7 stay a `Y202`.
  - `named_backreference_in_a_pattern_compiles`: `\k<n>` compiles and
    matches.
  - `pattern_classes_follow_ecma_262_beside_a_backreference`: `"éé"` is a
    violation, and `"aa"` passes.
  - `pattern_classes_follow_ecma_262_beside_a_lookbehind`: `"aé"` is a
    violation, and `"ab"` passes.
  - `pattern_property_key_outside_ecma_262_goes_unchecked`: the widening,
    where `éé: s` passes and `aa: s` is still checked.
- **Usage docs.** The validate guide says which meta-schemas are built in,
  under which URI, next to the existing rule that an external `$ref` is a
  `Y202`.
- **`Cargo.toml` notes and the changelog** record what moved.

Not pinned: the reworded messages, the error order, and the error count
for a self-referencing schema. Each is upstream output that a later
release can change without changing a verdict. The verdicts are what
scripts act on.

Found during the measurement and filed, since both predate this bump:
`yqr-b041` (a lone `additionalProperties: false` reports only the first
extra key) and `yqr-b042` (the refused-`$ref` advice above).

## 4. Acceptance criteria

- [x] Pin at 0.63. The lockfile moves only the seven packages in §1.
- [x] `local-ci.sh` clean, with every existing test unchanged.
- [x] The verdict changes pinned in both directions. Each pin fails on
      0.61.2.
- [x] The validate guide's `--schema` blocks re-run, and the meta-schema
      sentence added.
- [x] Changelog, `Cargo.toml` notes, and the status tracker updated.
