# Feature f012 — `yqr validate`: actionable YAML correctness checking

**Status:** Done
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
yqr validate [--strict] FILES...
```

- Stdin is explicit: `-` (rendered as `<stdin>` in diagnostics), accepted
  at most once — a second `-` would re-read an exhausted stream as an
  empty, vacuously valid input. **No files at all is a usage error (exit
  2), never a silent stdin fallback**: a validation gate whose argument
  expansion came up empty (`yqr validate $CHANGED_YAML` with nothing
  changed) must fail loudly, not report "all valid" having checked
  nothing. (Amended from the draft, which read stdin when files were
  omitted — the review found the false-green CI hazard.)
- Multiple files are validated in argument order; validation never
  fail-fasts across files — every input gets a verdict in one run.
- Success is silent (Unix convention; composable in CI).
- `--strict` promotes the lint-class findings of §3.3 to errors.

This is yqr's first subcommand. The filter form stays the default: clap gains
an optional subcommand with `subcommand_negates_reqs` so `yqr validate a.yaml
b.yaml` parses as the subcommand while `yqr '.a' f.yaml` is untouched (and
the filter still renders as required `<FILTER>` in usage). There is no
ambiguity to inherit: a bare word is not a valid filter (`yqr validate x`
today is a filter parse error, exit 3), so no currently-working invocation
changes meaning. Two guard rails keep that promise honest: clap's
auto-generated `help` subcommand is **disabled**, so `yqr help` keeps
failing as an invalid filter instead of becoming an exit-0 success a
wrapper script would mistake for output; and a flag before the word
(`yqr -r validate f.yaml`, which commits clap to the filter form) is
answered with a usage hint naming the subcommand instead of a baffling
filter parse error.

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

One additional check, aimed at damage that editing sessions introduce and
parsers hide:

1. **Duplicate mapping keys** (`a: 1` twice in one mapping, `Y101`). The
   YAML spec requires key uniqueness; parsers resolve last-wins silently,
   which after a bad edit means silently dropped data. Detection walks
   noyalib's lossless green tree (`Document::syntax()`) directly, so it
   reports **every** duplicate in one run — nested mappings, flow
   mappings, quoted respellings of the same key (`a` vs `"a"`), and
   duplicate `<<` merge keys included — each with the source positions of
   both occurrences (primary on the repeat, a note on the first). The
   value layer cannot do any of this: its `DuplicateKeyPolicy::Error`
   stops at the first offence and exempts merge keys, and by the time a
   `Value` exists the duplicates are already resolved away.

> **Amended during implementation.** The draft listed stringified-key
> collisions (`1:` vs `"1":`) as a second strict check. Empirically,
> noyalib's CST parser refuses collisions outright — no yqr read can
> process such a file at all — so the finding belongs to the **default**
> checks and is reported there as `Y102`, with its precise code instead of
> a generic syntax error (in a multi-document stream the affected document
> is named when it can be identified unambiguously). A first
> implementation of `Y101` on the value layer's duplicate-key policy was
> replaced after review: it reported only the first duplicate per document
> and missed `<<` merge keys entirely. The green-tree walk above has
> neither limitation and provides the key spans the draft originally
> promised — hand-rolled, since noyalib computes but does not expose them
> (the read-side sibling of the `yqr-b004` §2.2 gap).

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
  `Y001` syntax error, `Y002` stream-integrity failure, `Y003` non-UTF-8
  input, `Y101` duplicate key, `Y102` stringified-key collision. Codes are
  documented on the site and never renumbered, so scripts may match on
  them. I/O failures are not coded (plain `error: failed to read "f.yaml":
  ...`).
- **Location line** `--> file:line:col`, 1-based — the clickable rustc/cargo
  convention editors and terminals already linkify — whenever a position is
  known; a handful of parser errors carry none and render a bare
  `--> file`. Every position is derived from a **byte offset** through
  yqr's own line model (which counts `\r\n`, `\n`, and lone `\r`, like
  YAML), so CR-only files get correct line numbers; the parser's own
  line/column, which ignores lone CR, is never trusted directly.
- **Source window**: gutter with line numbers, the offending line (tabs
  expanded so the caret stays aligned), and a caret. End-of-input errors —
  the parser points one past the last line — clamp to the end of the last
  line so a truncated file still shows its context. Hand-rolled with **no
  new dependency** (matching the `error.rs` posture); noyalib's optional
  `miette` feature stays off.
- **`= help:`** line whenever a concrete fix can be suggested. When a
  syntax error strikes a file containing merge-conflict markers anywhere
  (`<<<<<<<`, `=======`, `>>>>>>>` at line start), the help names the
  first marker and the diagnostic anchors there if the parser gave no
  location — the parser usually reports a conflict block as an unlocated
  indentation error, so scanning only the error line would miss the case
  the feature exists for. An unknown anchor with a close candidate gets a
  "a similar anchor is declared at line N" help.
- Per file, syntax reports the first parse error only (YAML error recovery
  is not reliable enough to trust follow-on errors); strict findings may be
  multiple.
- **Non-UTF-8 input is a finding, not an I/O error** (`Y003`, exit 1):
  inputs are read as bytes and decoded by the validator, the diagnostic
  pointing one past the longest valid prefix. A wrongly-encoded file is a
  content defect its owner must fix — exit 5 stays reserved for
  environment problems (missing file, permissions).

### 3.5 Exit codes

| Code | Meaning | Agent action |
|------|---------|--------------|
| 0 | every input valid | proceed |
| 1 | at least one input failed validation (any `Ynnn` finding) | fix the content at the diagnostic's span |
| 5 | an input could not be read | fix the path/permissions |

Mixed outcomes take the highest applicable code (5 beats 1 beats 0); all
diagnostics are still printed. Usage errors are clap's exit 2 — including
an empty file list and a repeated `-` (§3.1). The filter pipeline's
jq-style 3/5 taxonomy is untouched — exit 1 exists only in validate mode,
following the linter/grep convention that "findings" are distinct from
"the tool failed".

### 3.6 The editing loop, end to end

```
yqr -i '.spec.replicas = 5' deploy.yaml   # surgical edit (guarded)
vi deploy.yaml                            # unguarded human edit
yqr validate --strict deploy.yaml         # verdict + actionable diagnostics
```

A merge conflict left in a file — the classic agent trap — fails Y001 with
a help line naming the first `<<<<<<<` marker, anchored at that line even
when the parser itself reports the breakage as an unlocated indentation
error elsewhere.

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

### 5.1 Schema validation — sized follow-up (own spec, builds on f012)

Out of f012's scope, but surveyed and sized so it can be picked up as the
next feature (a separate spec, next free `yqr-fNNN`), landing as a
`--schema <FILE>` flag on this subcommand — not a new command.

**What upstream provides.** noyalib's `validate-schema` cargo feature wraps
the standard `jsonschema` crate: JSON Schema **2020-12** validation of a
parsed `Value` tree via `validate_against_schema`, with the schema document
itself authorable in YAML. Violations carry RFC 6901 JSON-pointer instance
paths (`/items/0/name`) but **no source line/column** — validation runs on
the value tree, not the source.

**Design shape.**

- `yqr validate --schema schema.yaml [FILES]...` — default checks (§3.2)
  run first; each document in each stream is then validated against the
  schema. New codes in the registry: `Y201` schema violation, `Y202` the
  schema file itself is not a valid JSON Schema. Findings exit 1 like every
  other validation failure.
- Call `jsonschema`'s `iter_errors` directly for structured violations (one
  diagnostic per violation) rather than parsing noyalib's aggregated
  error string.
- **The differentiator — span mapping.** A violation's JSON pointer
  translates mechanically into a fidelity `Path` (`/spec/containers/0/image`
  becomes `Key/Key/Index/Key`; RFC 6901 `~0`/`~1` unescaping), and
  `FidelityEngine::resolve` already maps that to a byte span in the
  original source. Schema violations therefore render as the same
  rustc-style diagnostics as Y001 — `--> deploy.yaml:14:9`, offending line,
  caret — on the original bytes, which no kubeconform-class tool offers.
  Missing-required-property violations point at the parent mapping's span.

**Costs and decisions to make in that spec.**

- Dependency weight: the feature pulls `schemars`, `serde_json`, and
  `jsonschema` (plus tree) into a deliberately minimal-deps project —
  decide between a yqr cargo feature and an accepted binary-size bump.
- Remote `$ref` resolution stays off — a validator must not touch the
  network.
- Dialect expectations: 2020-12 covers SchemaStore-style schemas (GitHub
  Actions, docker-compose) well; Kubernetes CRD/OpenAPI schemas are an
  older dialect with partial compatibility and remain kubeconform's job.

**Effort estimate** (on top of an implemented f012, whose renderer and exit
contract it reuses wholesale): flag + per-violation diagnostics with
pointer paths, roughly one day; pointer-to-span mapping through the
fidelity engine, one to two more; dependency audit, docs, and corpus
cases, half a day to a day — **about 2 to 4 days total**, splittable into
two PRs (paths first, spans second).

### 5.2 Not planned
- **Style linting** (indentation width, line length, quoting preferences) —
  yamllint exists.
- **`--format json`** — rustc-style text is the machine interface for v1;
  structured output is a follow-up if a consumer materializes.
- **Auto-fix** — validate reports; the write tier edits.
- **jq's `empty` filter** — tracked with the jq feature gap (`yqr-r001`).

## 6. Acceptance criteria

- [x] `yqr validate f.yaml` on a valid file prints nothing and exits 0;
      stdin is explicit (`yqr validate -`), accepted at most once. No
      inputs at all, or a repeated `-`, is a usage error (exit 2) — never
      a silent stdin fallback (§3.1 amendment).
- [x] Invalid YAML exits 1 with a rustc-style diagnostic: `error[Y001]`,
      `--> file:line:col` (1-based, whenever a position is known), numbered
      source window with caret, and `= help:` where a suggestion exists.
      End-of-input errors clamp their window to the last line; CR-only
      files render correct line numbers; tabs keep the caret aligned;
      located variants never repeat the location inside the message.
- [x] A file containing merge-conflict markers anywhere fails with a help
      line naming the first marker, anchored there when the parser reports
      no location — covering full three-marker conflict blocks, not just a
      marker on the error line.
- [x] A stream-integrity failure reports `error[Y002]` and exits 1
      (unreachable through the real parser; pinned by a unit test on the
      check itself).
- [x] Non-UTF-8 input is a coded finding (`Y003`, exit 1) pointing one past
      the valid prefix — not an exit-5 environment error.
- [x] Multiple files: every input is validated in one run, each diagnostic
      names its file, and the exit code is the highest applicable (5 over 1
      over 0).
- [x] An unreadable input exits 5 with an uncoded error; remaining files are
      still validated.
- [x] `--strict` reports **every** duplicate key (`Y101`) — nested, flow,
      quoted respellings, and duplicate `<<` merge keys included — each
      with the positions of both occurrences, exit 1; without `--strict`
      duplicates pass. Stringified-key collisions (`Y102`) are reported by
      the **default** checks — the parser refuses them outright (§3.3
      amendment) — naming the affected document of a stream when
      unambiguous.
- [x] The filter form is behaviorally untouched (flags, exit codes 0/3/5,
      byte-identical output, `<FILTER>` still rendered as required in
      usage); `yqr validate <word>` was a filter parse error before f012,
      so no valid invocation changes meaning. clap's auto `help`
      subcommand is disabled (`yqr help` stays an invalid filter, exit 3),
      bare `yqr` remains a usage error (exit 2), and a flag typed before
      `validate` gets a usage hint naming the subcommand.
- [x] Diagnostic codes Y001/Y002/Y003/Y101/Y102 are documented in the site
      docs and README alongside the validate usage (rule: content
      documentation), with no feature IDs in CLI output or doc comments,
      and the docs promise positions only where they exist.
- [x] Corpus and CLI tests cover every diagnostic code and exit path;
      rendering is pinned by golden tests, and every corpus document must
      validate cleanly in both modes (the no-false-positives guard).
- [x] No new dependencies (noyalib's `miette` feature stays off; the
      renderer and the duplicate-key green-tree scan are hand-rolled).
