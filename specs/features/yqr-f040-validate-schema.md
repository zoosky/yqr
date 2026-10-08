# Feature f040 — `yqr validate --schema`: JSON Schema validation with source spans

**Status:** Done — shipped 2026-10-08; filed the same day from
`yqr-f012` §5.1, the sized follow-up that section kept open
**Epic:** Editing-loop tooling (`f012`)
**Owner:** yqr maintainers
**Related:** `yqr-f012` (the subcommand, renderer and exit contract this
reuses wholesale), `yqr-m002` (the fidelity read seam whose `resolve` maps
a violation to source bytes), `yqr-r001` §6 (the validation gap table)

## 1. Why this is open

`yqr validate` answers "is this correct YAML?". The question an editing
loop asks next is "is it the YAML this system accepts?" — a Kubernetes
manifest with `replicas: "three"`, a GitHub Actions workflow missing
`runs-on`. Today that answer needs a second tool, and the mainstream ones
(kubeconform, check-jsonschema) report violations as JSON-pointer paths
with no source location: `/spec/replicas: expected integer`. The user
then greps for the key the pointer names.

yqr already has the half those tools are missing: the fidelity engine
maps a concrete path to the byte span of the node in the original source
(`yqr-m002`). A schema violation can therefore render as the same
rustc-style diagnostic as every other finding — `--> deploy.yaml:14:9`,
offending line, caret — which no kubeconform-class tool offers.

## 2. Design

### 2.1 CLI surface

```
yqr validate [--strict] --schema <FILE> FILES...
```

One new flag on the existing subcommand, exactly as `f012` §5.1 sized it —
not a new command. The schema document is itself written in YAML (JSON is
a subset of YAML, so a `.json` schema file reads unchanged). The default
checks (`f012` §3.2) and `--strict` keep their meaning and run first;
each document of each input stream is then validated against the schema.

### 2.2 Dialect and dependency

- JSON Schema **2020-12**, built with the `jsonschema` crate pinned at
  0.58, `default-features = false`, plus `serde_json` (the instance model
  `jsonschema` validates). Violations come from `iter_errors`, one
  diagnostic per violation, as §5.1 prescribed — never from an aggregated
  error string.
- **No network, no filesystem `$ref` resolution.** The crate's
  `resolve-http` / `resolve-file` features stay off; a schema referencing
  an external resource fails to compile and reports as `Y202`. A
  validator must not touch the network.
- **Always on, not a cargo feature.** §5.1 left this open. Measured on
  this machine (aarch64-darwin, release profile), the dependency adds 44
  crates to the lockfile — none of them network, TLS or async — and the
  binary grows by the amount recorded in §5. A cargo feature was
  rejected because it gates a CLI flag: the corpus CLI suite would need
  feature-gated cases, the docs would need a "if your build has it"
  qualifier, and the differentiating feature would be dead code in every
  default build. `cargo audit` stays green and gates the tree in CI.
- Dialect expectations: 2020-12 covers SchemaStore-style schemas (GitHub
  Actions, docker-compose) well. Kubernetes CRD/OpenAPI schemas are an
  older dialect with partial compatibility and remain kubeconform's job
  — out of scope, as §5.1 already decided.

### 2.3 New diagnostic codes

The registry (`f012` §3.4) gains three codes. Codes are CLI contract:
never renumbered, never reused.

- **`Y201` — schema violation.** One per `iter_errors` violation. The
  message is the violation's own text (`"three" is not of type
  "integer"`); the note carries the RFC 6901 instance path and, in a
  multi-document stream, the document; the position is the span the
  pointer resolves to (§2.4). The validator's message embeds the
  offending instance as JSON, so past a 256-character budget the masked
  form is rendered instead (`value is not of type "array"`) — a
  root-level violation over a whole manifest must not print the
  document as its own diagnostic header (§6).
- **`Y202` — the schema itself is unusable.** The schema file is not
  valid YAML or UTF-8, holds no document or more than one, or does not
  compile as a JSON Schema 2020-12 document (including an external
  `$ref`). Every `Y202` carries the same help line. Rendered
  once against the schema file, then the run stops at exit 1: validating
  files against no schema and reporting "0 schema findings" would be the
  false green `f012` §3.1 exists to prevent. An unreadable schema file
  stays an uncoded read error at exit 5, like every other unreadable
  input.
- **`Y203` — the document leaves the JSON data model.** JSON Schema
  validates JSON instances. The one YAML value the parse boundary can
  produce that JSON cannot represent is a non-finite float (`.nan`,
  `.inf`); mapping keys are already strings at this boundary (the value
  layer is string-keyed). Coercing to `null` would silently change what
  the schema sees, so the document is reported instead, with the path of
  the offending value. One finding per document, and that document skips
  schema validation.

### 2.4 Span mapping — the differentiator

A violation's instance path arrives from `jsonschema` as typed segments
(property / index), which map one-to-one onto the fidelity engine's
`PathSeg::Key` / `PathSeg::Index` — no pointer-string parsing. The path
resolves through `FidelityEngine::resolve` against the document that
produced the instance:

- **`Found`** — the diagnostic points at the span's first byte, in the
  original source, through the same line model as every other finding.
- **`Synthetic` or `Absent`** (a merge-expanded or alias-expanded node
  with no bytes of its own; a missing-required-property pointer names
  the parent object, which is `Found`) — segments are popped until the
  path resolves, so the diagnostic points at the nearest enclosing node
  that has bytes. The root always resolves, so every `Y201` has a
  position.

The note always carries the exact instance path, so nothing is lost when
the caret lands on an ancestor.

### 2.5 Interaction with the default checks

All checks run over **one parse** per input: `validate::check` parses
the stream once, runs the default (and strict) checks on it, and hands
the same documents to the schema pass through the engine's
`open_parsed` constructor. The schema pass runs whenever that parse is
trustworthy — `Y103` or `Y101` findings do not suppress it, since the
tree is usable and each finding is independently actionable. An input
that fails to parse, or fails the tiling check, reports its `Y001` or
`Y002` and skips the schema pass: the gate is the reported finding
itself, never a coincidence of two parse paths agreeing. If the engine
nevertheless refuses the pair the pass reports a backstop finding
rather than skipping — a schema run that says nothing must mean every
document conformed, never that none was checked. Exit codes are
unchanged (`f012` §3.5): any finding is exit 1, the worst outcome
across inputs wins.

## 3. Out of scope

- **Remote or filesystem `$ref` resolution** — refused by construction
  (§2.2).
- **OpenAPI / Kubernetes CRD dialects** — kubeconform's job (§2.2).
- **`--schema` on the filter pipeline** — validation is `validate`'s;
  the filter form's contract is untouched.
- **SchemaStore auto-discovery** (matching a file name to a catalog
  schema) — a possible follow-up once field evidence asks for it; it
  adds a catalog format, not a mechanism.

## 4. Acceptance criteria

- [x] `yqr validate --schema schema.yaml file.yaml` validates every
  document of every input against the schema; a conforming input stays
  silent at exit 0.
- [x] A violation renders as `error[Y201]` with the violation message, a
  `--> file:line:col` into the original source, the source window and
  caret, and a note carrying the instance path (and the document, in a
  stream).
- [x] A missing required property points at the mapping that lacks it.
- [x] A violation inside merge-expanded content points at the nearest
  node with source bytes and still names the full instance path.
- [x] A schema that is unreadable exits 5; one that is invalid YAML, a
  multi-document stream, or not a valid 2020-12 schema renders one
  `Y202` against the schema file and exits 1.
- [x] A document holding a non-finite float renders `Y203` and skips the
  schema pass for that document only.
- [x] An input that fails the default checks still reports them; the
  schema pass runs whenever the input parses.
- [x] A schema with an external `$ref` fails as `Y202`; no build of yqr
  can resolve one over the network.
- [x] CLI corpus cases cover the flag end to end; unit tests cover the
  pointer-to-span mapping including the pop-to-ancestor rule.
- [x] Documentation: the validate page documents the flag, the three new
  codes, and the dialect expectations.

## 5. Measurements

Recorded at implementation time:

- Release binary (aarch64-darwin, default release profile): 1.49 MiB
  before, 5.05 MiB after — a +3.56 MiB bump, larger than the flag-only
  half of §5.1's estimate anticipated, and accepted under §2.2's
  reasoning. Most of it is the validator's regex engine and number
  tower.
- Dependency count: 95 crates in `Cargo.lock` before, 139 after. None
  of the 44 additions is a network, TLS or async crate (verified by
  tree inspection: no reqwest, hyper, tokio, url, idna or TLS crate in
  `cargo tree -p jsonschema`), and `cargo audit` is clean.

## 6. Code review

A full-branch review (2026-10-08) returned eight findings; all eight
were applied before merge:

1. **Unbounded `Y201` message.** The validator's message embeds the
   offending instance; a root-level violation over a large file printed
   the whole document on one line. Past a 256-character budget the
   masked message is rendered instead; two tests pin both sides of the
   budget.
2. **Latent false green.** The schema pass skipped silently when the
   fidelity engine refused an input its own parse had accepted — safe
   only while the two parse paths agreed. §2.5's single-parse design
   removes the second path, the gate is now the reported parse finding,
   and the engine arms report a backstop finding instead of skipping.
3. **Dangling root pointer.** A schema whose root value leaves the JSON
   model rendered "…cannot represent at " — the root's RFC 6901 pointer
   is the empty string. The location moved to a note that names the
   root explicitly.
4. **Inconsistent `Y202` help.** A non-UTF-8 schema kept the
   input-oriented re-encode help; every `Y202` now carries the same
   schema help line, built by one constructor.
5. **"The file holds 0".** An empty schema file now says "schema file
   holds no document" instead of wording shaped for the multi-document
   case.
6. **Duplicated tests.** Five black-box CLI tests repeated corpus cases
   one for one, against the corpus's authored-once rule; they are
   removed, and the remaining CLI tests cover only what a corpus case
   cannot express (stdin input, interleaving, the unparseable skip, the
   multi-file sweep, invalid schema bytes).
7. **Double parsing.** Each input was parsed twice (the checks, then
   the schema pass's own `open`), and the schema file twice in
   `compile`. `open_parsed` builds the engine from the already-parsed
   stream; every input and the schema are now parsed once.
8. **`parent()` belonged on `Path`.** The pop-to-ancestor helper
   rebuilt each prefix segment-by-segment; `fidelity::Path::parent` is
   now the shared, discoverable form.
