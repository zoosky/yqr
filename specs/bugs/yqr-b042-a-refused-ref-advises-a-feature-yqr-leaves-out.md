# Bug b042 — A refused `$ref` advises a feature yqr leaves out

**Status:** Resolved — filed and fixed 2026-11-26
**Severity:** Low — the verdict is right (`Y202`, exit 1); only the
message misleads
**Component:** `src/validate/schema.rs` (`Schema::compile` passes the
compile error's text through)
**Related:** `yqr-f040` (no build resolves an external `$ref`), `yqr-f053`
(the jsonschema 0.63 adoption whose code review found it)

## 1. What was observed

A schema that `$ref`s anything the validator does not bundle is refused,
as `f040` intends, but the message tells you to enable a resolver:

```text
$ printf '$ref: "https://example.com/s.json"\n' > schema.yaml
$ yqr validate --schema schema.yaml doc.yaml
error[Y202]: schema does not compile as JSON Schema 2020-12: Resource
'https://example.com/s.json' is not present in a registry and retrieving
it failed: `resolve-http` feature or a custom resolver is required to
resolve external schemas via HTTP
```

`resolve-http` is a jsonschema cargo feature that yqr leaves out by design.
You cannot turn it on, and the validate guide says the refusal is "by
construction". The advice points you at something that does not exist
for you.

jsonschema 0.61.2 already gave this advice for an arbitrary URL. On
0.63.1 it also gives it for the Draft 3 and `draft/next` meta-schema URIs.
0.61.2 said "Retrieving external resources is not supported once the
registry is populated" for those two.

## 2. What the validator reports

A probe that compiles each shape with jsonschema 0.63.1, built as yqr
builds it (default features off), measured on 2026-11-26. Every refused
reference arrives as the same error kind,
`ValidationErrorKind::Referencing(ReferencingError::Unretrievable { uri, .. })`,
with different advice:

| Reference | `uri` | Upstream advice |
|---|---|---|
| `https://example.com/s.json`, or `http://` | as written | enable `resolve-http` |
| Draft 3, `draft/next`, `https://json-schema.org/draft-07/schema#` | without the `#` | enable `resolve-http` |
| `other.json` beside `$id: https://example.com/root` | `https://example.com/other.json` | enable `resolve-http` |
| `file:///tmp/s.json` | as written | enable `resolve-file`, also left out |
| `other.json` with no `$id` | `other.json` | "No base URI is available" |
| `urn:example:s` | as written | "Unknown scheme urn" |
| `$dynamicRef` to a URL | the URL, without the fragment | "not supported once the registry is populated" |
| A nested `$ref`, under `properties` | the URL, without the fragment | enable `resolve-http` |

`#/$defs/missing` is a different kind, `PointerToNowhere`
("Pointer '/$defs/missing' does not exist"). That message is accurate and
stays as it is.

## 3. The fix

`Schema::compile` now words the compile error through `compile_message`.
An `Unretrievable` error becomes:

```text
error[Y202]: schema refers to 'https://example.com/s.json', which yqr never
fetches; references resolve only within the schema file and to the
built-in meta-schemas
```

Every other compile error keeps the upstream text after
`schema does not compile as JSON Schema 2020-12:`. The help line and the
exit code are unchanged.

The wording departs from the first draft of this spec ("schema `$ref`s
'<uri>' ...; only the built-in meta-schemas resolve") for two reasons:

- A reference within the schema file also resolves, so "only the
  built-in meta-schemas" was wrong.
- The same error covers `$dynamicRef`, so "`$ref`s" was too narrow.

The match goes through `jsonschema::ReferencingError`, the crate's own
re-export, so yqr adds no dependency. If a jsonschema release renames the
kind or its fields, the match stops compiling. If a release reports these
references under another kind, the match falls back to the upstream text
and the unit test fails.

No `docs/content` page quotes the old message, so the guide is unchanged.

## 4. Acceptance criteria

- [x] An unretrievable `$ref` is a `Y202` whose message names the URI and
      says yqr never fetches it, with no mention of `resolve-http`.
- [x] Other compile errors keep the upstream text.
- [x] A unit test pins the reworded message for an arbitrary URL and for
      the Draft 3 meta-schema
      (`refused_ref_names_the_uri_without_resolver_advice`, which also
      covers a `file:` URI and a relative path, and pins a missing pointer
      keeping its upstream text).
