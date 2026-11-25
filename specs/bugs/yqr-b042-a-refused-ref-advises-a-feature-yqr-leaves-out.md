# Bug b042 — A refused `$ref` advises a feature yqr leaves out

**Status:** Open — filed 2026-11-26
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

## 2. Proposed fix

In `Schema::compile`, recognize the unretrievable-reference error and
word it in yqr's terms. In jsonschema 0.63.1 it arrives as
`ValidationErrorKind::Referencing(referencing::Error::Unretrievable { uri, .. })`.
For example:

```text
error[Y202]: schema `$ref`s 'https://example.com/s.json', which yqr never
fetches; only the built-in meta-schemas resolve
```

Keep the existing help line. Every other compile error stays as it is.

## 3. Acceptance criteria

- [ ] An unretrievable `$ref` is a `Y202` whose message names the URI and
      says yqr never fetches it, with no mention of `resolve-http`.
- [ ] Other compile errors keep the upstream text.
- [ ] A unit test pins the reworded message for an arbitrary URL and for
      the Draft 3 meta-schema.
