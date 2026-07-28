# Feature f012 — `yqr validate`: actionable YAML correctness checking

**Status:** Draft
**Epic:** Editing-loop tooling (f012)
**Owner:** yqr maintainers
**Related:** `yqr-a001` (the fidelity invariant validate re-uses), `yqr-f006`/`yqr-f007`
(the write tier — the editing loop this closes), `yqr-m005` (noyalib is the one
engine), `yqr-r001` (jq feature gap — jq's `empty`-filter idiom belongs there),
`yqr-b004` §"string-only key model" (the collision `--strict` surfaces)

## 1. Problem

yqr's editing loop — human or agent — is read, edit, and then... trust. yqr's
own write tier refuses edits whose result would re-parse differently, but yqr
is not the only writer of a YAML file: hand edits, other tools, templating,
file concatenation, and half-resolved merge conflicts all produce files whose
correctness is unknown at the moment an agent most needs to know it — right
after the edit.

Today the only check is the exit-code idiom `yqr '.' f > /dev/null`, which is
deficient in exactly the ways that matter for that loop:

- Invalid YAML and I/O failure share exit 5, so a script cannot tell "the
  file is broken" (fix the content) from "the file is missing/unreadable"
  (fix the environment).
- One file per invocation; validating a directory means a shell loop.
- The error is a single flat line (`yqr: io error: failed to parse YAML
  input: ...`) — the structured location noyalib reports is stringified away
  at the engine boundary (`src/fidelity/noyalib.rs` maps the parse error to
  `YqrError::io(format!(...))`), so nothing is machine-actionable.
- The document itself is emitted to stdout unless redirected.

Agents act well on two things: exit codes they can branch on, and diagnostics
with a precise location, a labeled span, and a suggested fix — the shape they
already know from rustc and cargo. That is the target output format.

## 2. Prior art

- **jq** has no validate command; the canonical idiom is `jq empty f.json` —
  parse, print nothing, exit code speaks. (yqr has no `empty` filter; that
  gap stays with `yqr-r001`.)
- **yq v3** shipped a dedicated `yq validate`; **v4 dropped it** and the docs
  recommend `yq --exit-status 'tag == "!!map" or tag == "!!seq"' f >
  /dev/null` — an idiom, again, because bare YAML validity is so lenient.
- **yamllint** is the dedicated checker and the only tool in this family that
  catches duplicate keys — invalid per the YAML spec's key-uniqueness
  requirement, silently accepted last-wins by virtually every parser.
- **xmllint --noout** is the long-standing precedent for a built-in
  well-formedness check with no output and a meaningful exit code.

Decision: a dedicated subcommand with rustc-style diagnostics. The idiom
approach is what f012 exists to replace, and a flag on the filter form cannot
express multi-file validation.

## 3. Design

### 3.1 CLI surface

```
yqr validate [FILES]...
```

- No `FILES`, or `-`, reads stdin (rendered as `<stdin>` in diagnostics).
- Multiple files are validated in argument order; validation never
  fail-fasts across files — every input gets a verdict in one run.
- Success is silent (Unix convention; composable in CI).
- `--strict` promotes the lint-class findings of §3.3 to errors.

This is yqr's first subcommand. The filter form stays the default: clap gains
an optional subcommand with `subcommand_negates_reqs` so `yqr validate a.yaml
b.yaml` parses as the subcommand while `yqr '.a' f.yaml` is untouched. There
is no ambiguity to inherit: a bare word is not a valid filter (`yqr validate
x` today is a filter parse error, exit 3), so no currently-working invocation
changes meaning.

### 3.2 What is checked (default mode)

1. **Syntax** — every document in the stream parses on noyalib's CST.
2. **Stream integrity** — the parsed documents tile the input byte-for-byte
   (the `yqr-a001` invariant the fidelity engine already asserts at open).
   A pass therefore certifies not just "parses" but "parses and
   round-trips losslessly" — a stronger guarantee than other validators give.

Nothing else. Valid-but-unusual YAML (empty file, scalar root, duplicate
keys) passes by default, matching the leniency of every mainstream parser —
default validate answers "is this YAML?", not "is this the YAML you meant?".

### 3.3 `--strict`: findings an edited file almost never wants

Two additional checks, both aimed at damage that editing sessions introduce
and parsers hide:

1. **Duplicate mapping keys** (`a: 1` twice in one mapping). The YAML spec
   requires key uniqueness; parsers resolve last-wins silently, which after
   a bad edit means silently dropped data. Diagnostic spans both
   occurrences: primary label on the second key, secondary note on the
   first.
2. **Stringified-key collisions** (`1:` and `"1":` in one mapping) — valid
   YAML that yqr's engine refuses at query time (`yqr-b004`, string-only key
   model). An agent should hear about this at validate time, not on the
   next read.

The strict list is closed for v1; candidates like tab indentation belong to a
future lint tier, if ever (yamllint's territory — see §5).

### 3.4 Diagnostics: rustc style

All diagnostics go to stderr, in the shape agents and humans already parse:

```
error[Y001]: mapping values are not allowed in this context
  --> deploy.yaml:12:14
   |
12 |   ports: 80: 443
   |              ^ second ':' on one line
   |
   = help: quote the value ("80: 443") or split it into its own mapping
```

Components and rules:

- **Severity + stable code.** `error[Ynnn]` with a small closed registry:
  `Y001` syntax error, `Y002` stream-integrity failure, `Y101` duplicate
  key, `Y102` stringified-key collision. Codes are documented on the site
  and never renumbered, so scripts may match on them. I/O failures are not
  coded (plain `error: failed to read "f.yaml": ...`).
- **Location line** `--> file:line:col`, 1-based — the clickable rustc/cargo
  convention editors and terminals already linkify.
- **Source window**: gutter with line numbers, the offending line, and a
  caret span under the offending bytes. noyalib's core error API provides
  everything needed — `Error::ParseWithLocation`, `Location` (line, column,
  byte index), and `CroppedRegion::extract` (a windowed slice of source
  lines around a location) — with **no new dependency**: the renderer is
  hand-rolled (matching the `error.rs` posture), and noyalib's optional
  `miette` feature stays off.
- **`= help:`** line whenever a concrete fix can be suggested (from
  noyalib's message or yqr's own mapping for Y101/Y102).
- Per file, syntax reports the first parse error only (YAML error recovery
  is not reliable enough to trust follow-on errors); strict findings may be
  multiple.

### 3.5 Exit codes

| Code | Meaning | Agent action |
|------|---------|--------------|
| 0 | every input valid | proceed |
| 1 | at least one input failed validation (Y001/Y002, or strict findings) | fix the content at the diagnostic's span |
| 5 | an input could not be read | fix the path/permissions |

Mixed outcomes take the highest applicable code (5 beats 1 beats 0); all
diagnostics are still printed. Usage errors remain clap's exit 2. The filter
pipeline's jq-style 3/5 taxonomy is untouched — exit 1 exists only in
validate mode, following the linter/grep convention that "findings" are
distinct from "the tool failed".

### 3.6 The editing loop, end to end

```
yqr -i '.spec.replicas = 5' deploy.yaml   # surgical edit (guarded)
vi deploy.yaml                            # unguarded human edit
yqr validate --strict deploy.yaml         # verdict + actionable diagnostics
```

A merge conflict left in a file — the classic agent trap — fails Y001 with
the `<<<<<<<` line in the source window, named by file, line, and column.

## 4. Implementation notes

- New module `src/validate.rs` (diagnostic types, checks, renderer), kept
  under the 500-line rule; `src/cli.rs` gains the optional subcommand;
  `src/main.rs` dispatches and owns the exit-code mapping.
- The engine boundary today throws away noyalib's structured error
  (`format!`). Validate must consume `Error::ParseWithLocation` /
  `Location` directly. As a side benefit, the ordinary read path's parse
  errors can adopt the same renderer later without a new spec (same Y001
  shape, exit 5 as today) — explicitly optional, not part of f012's
  acceptance.
- Duplicate-key detection walks the CST's mapping entries per node with
  decoded keys and both spans; noyalib 0.0.15's loader key-collision guard
  is worth investigating first so yqr does not re-implement what upstream
  exposes.
- Corpus (`yqr-m003`): add validation cases — one valid case per corpus
  document, plus one case per diagnostic code — driving both
  `tests/corpus_validation.rs` and black-box CLI tests (exit codes, the
  `-->` line, code presence). Diagnostic rendering gets golden tests.

## 5. Out of scope / follow-ups

- **Schema validation** (JSON Schema, Kubernetes). noyalib ships schema
  machinery upstream (`validate-schema` / `noyavalidate` features) — a
  natural separate feature when demand appears.
- **Style linting** (indentation width, line length, quoting preferences) —
  yamllint exists.
- **`--format json`** — rustc-style text is the machine interface for v1;
  structured output is a follow-up if a consumer materializes.
- **Auto-fix** — validate reports; the write tier edits.
- **jq's `empty` filter** — tracked with the jq feature gap (`yqr-r001`).

## 6. Acceptance criteria

- [ ] `yqr validate f.yaml` on a valid file prints nothing and exits 0;
      stdin works both bare (`yqr validate`) and explicit (`yqr validate -`).
- [ ] Invalid YAML exits 1 with a rustc-style diagnostic: `error[Y001]`,
      `--> file:line:col` (1-based), numbered source window with caret span,
      and `= help:` where a suggestion exists.
- [ ] A stream-integrity failure reports `error[Y002]` and exits 1.
- [ ] Multiple files: every input is validated in one run, each diagnostic
      names its file, and the exit code is the highest applicable (5 over 1
      over 0).
- [ ] An unreadable input exits 5 with an uncoded error; remaining files are
      still validated.
- [ ] `--strict` reports duplicate keys (`Y101`) and stringified-key
      collisions (`Y102`) with primary and secondary spans, exit 1; without
      `--strict` both pass.
- [ ] The filter form is behaviorally untouched (flags, exit codes 0/3/5,
      byte-identical output); `yqr validate <word>` was a filter parse error
      before f012, so no valid invocation changes meaning.
- [ ] Diagnostic codes Y001/Y002/Y101/Y102 are documented in the site docs
      and README alongside the validate usage (rule: content documentation),
      with no feature IDs in CLI output or doc comments.
- [ ] Corpus and CLI tests cover every diagnostic code and exit path;
      rendering is pinned by golden tests.
- [ ] No new dependencies (noyalib's `miette` feature stays off; the
      renderer is hand-rolled).
