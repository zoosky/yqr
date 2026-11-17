# Feature f052 — Adopt noyalib 0.0.62: the b014 leniency closes, a BOM in text is refused

**Status:** Done — shipped 2026-11-17
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-b014` (the leniency this release closes), `yqr-f012`
(the validator and its `Y103` check), `yqr-f040` (schema validation),
`yqr-f045` (the last adoption spec), `yqr-m001` (quality gates)

## 1. What the release carries

noyalib 0.0.62 (2026-11-17) follows 0.0.60 directly: upstream folded
0.0.61 into it and never published it. Its changelog lists a long run of
breaking parse changes, most of them yaml-test-suite conformance fixes
and compatibility-profile work. Two reach yqr's input:

- **An under-indented block mapping value is refused.** `on:` followed by
  `[]` at column 1 and a sibling key used to parse; noyalib refused it only
  when no sibling followed. The changelog does not name this change; it was
  found by the measurement in §2.
- **A byte order mark inside a plain or block scalar, a comment, or a
  directive line is refused** under the default profile, per YAML 1.2.2
  production [27] (`nb-char` excludes U+FEFF). A BOM opening the stream or a
  document, or one inside a quoted scalar, still reads. The `serde_yaml`
  profile keeps the old behavior through the new
  `ParserConfig::byte_order_marks_in_text` flag.

The rest (bracketed sequence indices in error paths, the `compat::serde_yaml`
error spelling, integer-overflow handling under the compatibility profile,
the serializer quoting long radix integers, the removed `compare-saphyr`
feature) is on paths yqr does not call.

## 2. What moved in yqr, measured

`cargo update -p noyalib` moved exactly one package in the lockfile. Four
tests failed, all built on `b014`'s shape, and nothing else moved: every
corpus, fidelity, contract, values, and write case is byte-identical.

Ten shapes were run through `yqr validate` and a plain read on a 0.0.60
control build (main at `2bc20d4`) and on 0.0.62:

| Input | 0.0.60 | 0.0.62 | Read on 0.0.62 |
|-------|--------|--------|----------------|
| `on:` / `[]` / `jobs: {}` | Y103 | Y001 at 2:1 | exit 5 |
| `on:` / `[]` (no sibling) | Y001 | Y001 | exit 5 |
| `on:` / `{}` / `jobs: {}` | Y103 | Y001 | exit 5 |
| `on:` / `foo` / `b: 1` | Y103 | Y001 | exit 5 |
| `on:` / `"q"` / `b: 1` | Y103 | Y001 | exit 5 |
| nested `steps:` / `  on:` / `  []` / `x: 1` | Y103 | Y001 at 3:3 | exit 5 |
| `on:` / `[a, b]` / `jobs: 1` | Y103 | Y001 | exit 5 |
| `on:` / `- push` / `jobs: {}` | clean | clean | exit 0 |
| `a: x<U+FEFF>y` | clean | Y001 | exit 5 |
| `k: 0o7777777777777777777777777` | clean | clean | exit 0 |

Every `Y103` shape is now a parse error at the position `Y103` used to
report. The leniency `b014` recorded is gone, and noyalib no longer
disagrees with itself between the sole-entry and the sibling case. PyYAML
and Psych both refuse these shapes, so yqr now agrees with them.

The BOM case goes the other way: PyYAML and Psych both read `a: x<U+FEFF>y`
as a string holding the BOM. yqr takes upstream's refusal as shipped,
following the precedent of 0.0.36 and 0.0.38, whose new refusals were pinned
rather than worked around. Opting into `byte_order_marks_in_text` would keep
the old reading, at the cost of carrying a non-default profile flag through
every parse and re-parse.

## 3. What changed in yqr

- **`Y103` stays, as a backstop.** The scan and the write path's integrity
  count (`b029`) no longer fire for any known input. Both stay in place: the
  fidelity engine's re-parse guard re-parses with the same engine, so a later
  parser release that read the shape again would be invisible to it. The
  docs present the shape as a syntax error and keep `Y103` reserved.
- **Tests retargeted.** The two unit tests and the CLI test built on the
  shape now pin `Y001` at the same position, plus the exit-5 read. The
  schema test that needed a lint-class finding beside a schema finding now
  uses a strict-mode duplicate key (`Y101`).
- **Two new CLI pins** for the BOM change: the plain-scalar refusal, and the
  leading-BOM and quoted-scalar cases that still read.

## 4. Acceptance criteria

- [x] Pin at 0.0.62; the lockfile moves only noyalib.
- [x] `local-ci.sh` clean; the only expectations moved are the four built
      on `b014`'s shape.
- [x] The new `Y001` verdict and position pinned in unit and CLI tests.
- [x] The BOM refusal and the cases that still read pinned in the CLI suite.
- [x] Usage docs, changelog, `Cargo.toml` notes, and `b014` updated.
