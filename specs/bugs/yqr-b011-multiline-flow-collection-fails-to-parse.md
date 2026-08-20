# Bug b011 — A multi-line flow collection is valid YAML that yqr cannot read at all

**Status:** Resolved — filed upstream 2026-08-19 as noyalib#285, fixed in
noyalib#286, **released in noyalib 0.0.25** (2026-08-20) and verified against
the published crate by `yqr-f019` §3.1. yqr pins 0.0.25; a wrapped flow
collection now reads byte-for-byte, and under-indented flow *content* is still
refused, which is the narrowing that makes the fix spec-conformant rather than
a relaxation. Fixing the read exposed a delete defect behind it, filed as
`yqr-b015`
**Severity:** Medium — an input yqr refuses outright, on the **read** path, so
no filter runs; not silent, but a hard "cannot open your file"
**Component:** noyalib's parser (upstream), reached through every yqr entry
point — `parse_stream` is the first thing both the read engine and the write
engine call
**Related:** `yqr-a001` (the fidelity contract, which presumes yqr can read the
file), `yqr-f016` §5 (found while reviewing the flow-delete work, which is
where the shape came up), `yqr-b004` (the upstream gap catalog)

## 1. Summary

A flow collection spread over several lines is ordinary, valid YAML. noyalib
refuses to parse it, so yqr cannot read such a document, let alone edit one:

```console
$ cat ports.yaml
ports: [
  80,
  443,
]
$ yqr '.' ports.yaml
yqr: io error: failed to parse YAML input: YAML parse error: flow content must
be indented more than the surrounding block
```

The identity filter is enough to trigger it — this is not about any particular
operation.

## 2. It is valid

PyYAML accepts it, with the closing bracket at column 0 and with it indented:

```console
$ python3 -c "import yaml; print(yaml.safe_load(open('ports.yaml')))"
{'ports': [80, 443]}
```

Per the YAML spec, flow content inside a block context must be more indented
than the block — which `  80,` and `  443,` are. The closing `]` is an
indicator rather than content, and every other implementation accepts it at the
parent's column.

The message names the right rule and applies it to the wrong token.

**Refined 2026-08-19, while fixing it.** "PyYAML accepts it" is true but is the
weaker half of the argument, and taking it as the standard would be wrong: the
libyaml family is *more lenient than the spec* here, accepting under-indented
flow **content** (`ports: [` / `80,` / `]`) that the yaml-test-suite marks as an
error (9C9N, VJP3). The defensible claim is narrower — the closing indicator is
not content, so the rule that governs content does not reach it — and §4.3
records the suite survey that establishes it.

## 3. Why it matters more than the refusal suggests

It is loud, which is the good half: exit 5 with a message, no damage. But it is
a **whole-file** refusal on the read path, so:

- The `yqr-a001` promise — "reads never rewrite bytes" — is vacuous for these
  files, because there is no read.
- `yqr validate` reports the file as unreadable rather than valid, which is
  wrong in the one direction a validator must not be wrong.
- Multi-line flow sequences are common in hand-maintained config (a long
  `args:` or `ports:` list wrapped for width), so this is not an exotic shape.

## 4. Route — taken 2026-08-19

Upstream, on the `yqr-b004` §5 `PR-with-fix` precedent: filed as **noyalib#285**
and fixed in **noyalib#286**. The estimate held — it is a parser indentation
check that exempts the closing indicator — and this is yqr's first contribution
to noyalib's parser rather than its mutators.

### 4.1 What the fix does

The scanner's rule refuses a flow continuation line whose content column is at
or below the surrounding block's indent. Its own comment states the reason:
under-indented content would be ambiguous with sibling block content (the
yaml-test-suite case 9C9N). That reason is about *content*; a line whose first
character is `]` or `}` cannot begin block content, so the rule was reaching the
terminator as well as the content it was written for. The patch exempts such a
line and changes nothing else.

The asymmetry was already visible upstream: the same closer at column 0 is
accepted at the **root**, where the tracked indent is `-1`, and upstream's own
counter-example test carries `[\n  a,\n  b\n]\n`. Only a flow inside a block
mapping refused it.

### 4.2 What the fix deliberately does not do

Under-indented **content** stays refused, so `ports: [` / `80,` / `]` is still an
error even though libyaml, PyYAML and Psych all accept it. That is 9C9N's rule.
This matters for §2 of this spec, which was written from the libyaml family's
behaviour: the ecosystem is *more* lenient than the spec here, and the fix
follows the spec, not the ecosystem. The `]`-at-the-parent's-column shape is
the part that is defensible on the spec's own terms.

### 4.3 The evidence the fix rests on

Every yaml-test-suite case with a closing flow indicator on a line of its own
— 28 of them — split cleanly, and the split is about content rather than the
closer:

| Case | Shape | Expect |
|---|---|---|
| 9C9N | `flow: [a,` / `b,` / `c]` — content at col 0 | error |
| VJP3 | `k: {` / `k` / `:` / `v` / `}` — content at col 0 | error |
| 87E4, L9U5, LQZ7 (spec 7.4/7.8/7.11) | key col 0, content col 2, closer col 1 | pass |
| ZF4X (spec 2.6) | content col 4, closer col 2 | pass |
| 4ABK, C2DT, DFF7, FRK4, QF4Y, WZ62, NKF9 | root flow, content and closer col 0 | pass |

No case pins a closer at column 0 under a block key at column 0 in either
direction, while three spec examples deliberately put the closer *below* the
content.

## 5. Provenance

Found while reviewing the `yqr-f016` §5 flow-delete work: the reviewer noted
that flow deletes only work on single-line flow collections, and the reason
turned out to be that yqr cannot open a multi-line one. So the delete
limitation is a symptom, and this is the defect.

## 6. Acceptance

- [x] Filed upstream with the §2 evidence — noyalib#285, with the fix as
      noyalib#286 and the §4.3 suite survey behind it.
- [ ] A released noyalib parses a multi-line flow collection, closing indicator
      at the parent's column included.
- [ ] `yqr '.'` round-trips such a file byte-for-byte, pinned in the `a001`
      fidelity harness.
- [ ] A corpus case covers a wrapped flow sequence, since the shape is common
      enough that its absence is why this went unnoticed.

Measured against the patched crate through a path override, so the last three
are ready to land the day a release carries the fix: `yqr '.'` returns the file
byte-identical, `.ports[1]` answers `443`, an edit to a neighbouring key leaves
the wrapped list and its inline comment untouched, `validate --strict` passes,
and yqr's whole suite is green.
