# Research r002 — Evaluating `noyalib` as a fidelity engine for yqr

**Status:** Draft
**Owner:** yqr maintainers
**Last updated:** 2026-06-26
**Subject:** Does `noyalib` 0.0.8 solve the round-trip fidelity problem (`yqr-b001`) better than `rust-yaml` 1.1.0?
**Related:** `yqr-b001` (the fidelity bug), `yqr-a001` (Fidelity-First architecture), `yqr-r001` §5/§9 (YAML-native gaps)
**Evaluated:** `noyalib` 0.0.8 (workspace at `../yqr-deps/noyalib`, crate `crates/noyalib`); crates.io max version 0.0.8 (2026-06-17, independently verified)

## 1. Question

`yqr-b001` proved that `rust-yaml`'s `text → Value → dump_str` pipeline is a
lossy semantic round-trip: it cannot satisfy the `yqr-a001` guarantee ("yqr never
rewrites bytes it did not change"). `yqr-a001` §4 prescribes building a
source-preserving, span-based layer ourselves on top of `rust-yaml`'s scanner
tokens.

This document asks whether **`noyalib`** — which ships a CST "tooling" API that
claims to "reproduce the source byte-for-byte" and to edit YAML by rewriting only
the touched span — is a better foundation than either today's `rust-yaml` path or
a hand-built span layer.

## 2. Method

Three independent, cross-checked lines of evidence:

1. **Empirical** — the exact `yqr-b001` reproduction corpus (14 files, one per
   fidelity dimension) was run through `noyalib::cst::parse_document(input)`
   `→ .to_string()` and diffed byte-for-byte against the input, plus a surgical
   edit (`doc.set`) and a multi-document (`parse_stream`) probe. Built against the
   local checkout via a throwaway path-dependency harness.
2. **Source review** — a multi-agent pass over `noyalib`'s source and docs (CST
   mechanism, trivia capture, API fit, maturity, dependencies, comparison), with
   an **adversarial verification stage** that re-checked every load-bearing claim
   against the code. The reproduced BOM defect (§5.3) was found by this pass and
   confirmed empirically.
3. **External facts** — `noyalib`'s crates.io publication, versions, and dates
   were fetched and verified directly (not taken from the README).

## 3. Recommendation (TL;DR)

**The architecture is right; the dependency is not yet trustworthy enough to be
the foundation of a hard guarantee. Recommended path: HYBRID, GATED — prototype,
do not yet commit.**

- `noyalib`'s CST **is** the `yqr-a001` §4 architecture (original text +
  node→byte-span index + splice) **already implemented, exercised, and
  `#![forbid(unsafe_code)]`**. Empirically it round-trips **13 of 14** `b001`
  corpus files byte-for-byte and performs the exact a001 §4.2 surgical edit —
  where `rust-yaml` fails **13 of 14** (§5).
- **But** three findings block adopting it as-is: (1) a **reproduced BOM bug** —
  a BOM followed by any multi-node document is a hard parse error in 0.0.8
  (**fix now in-flight upstream**: PR
  [sebastienrousseau/noyalib#118](https://github.com/sebastienrousseau/noyalib/pull/118),
  open, not yet merged/released); (2) it is **0.0.8 with a self-declared "every
  minor bump may be breaking" policy** and 1.0 deferred to ~2028,
  single-maintainer; (3) **no large-file CST performance/memory data exists** and
  each edit copies the whole source.
- `noyalib` has **no jq evaluator**, so it can never fully replace yqr's engine —
  only a **hybrid** (noyalib for the fidelity/CST layer, yqr's own evaluator) is
  viable.

**Action:** keep yqr's evaluator; spike `noyalib`'s CST behind a thin internal
adapter (so a break or abandonment is contained); pin the exact version; and gate
real adoption on the three checks in §13. If the gates pass (and the BOM bug is
fixed upstream or pre-normalized by yqr), adopt as a hybrid. If not, build the
span layer in-house **using `noyalib`'s design as the proven blueprint** rather
than `rust-yaml`'s scanner.

## 4. What `noyalib` is

A YAML 1.2 library (`serde`-integrated, zero `unsafe`) that exposes **two APIs
over one parser** (`README.md` "Two APIs, one parser"):

- **Data binding** — `from_str` / `to_string` / `Value`. The round-trip travels
  through a `Value`, so "comments and exact whitespace are not preserved." This is
  the same lossy shape as `rust-yaml` and is **not** the interesting part.
- **Tooling / automation** — `noyalib::cst::parse_document` / `parse_stream` →
  `Document`. "Read YAML into a side-table CST that reproduces the source
  byte-for-byte, then run targeted edits like `doc.set("version", "0.0.2")` —
  only the touched span is rewritten, every comment and the original indentation
  is left alone." This is the candidate fidelity engine.

The `Document` holds three coordinated views of one input: a **green tree** that
reproduces the source verbatim, a typed `Value` for data access, and a **span
tree** mapping any path to a byte range (`examples/lossless_edit.rs`).

## 5. Empirical results (the `b001` corpus through `noyalib`'s CST)

### 5.1 Round-trip matrix

`parse_document(input).to_string()` vs input, byte-for-byte:

| Corpus file (dimension) | `rust-yaml` (`b001`) | `noyalib` CST |
|---|---|---|
| 01 comments + blank lines | DIFFERS (all lost) | **IDENTICAL** |
| 02 blank lines | DIFFERS | **IDENTICAL** |
| 03 indent width (4-space) | DIFFERS | **IDENTICAL** |
| 04 quote styles (`'`/`"`) | DIFFERS | **IDENTICAL** |
| 05 block scalars (`\|`/`>`) | DIFFERS | **IDENTICAL** |
| 06 numbers (`007`, etc.) | DIFFERS (`007`→`7`) | **IDENTICAL** |
| 07 flow style `{}`/`[]` | DIFFERS (→block) | **IDENTICAL** |
| 08 key order | IDENTICAL | **IDENTICAL** |
| 09 anchors / merge `<<` | DIFFERS (expanded) | **IDENTICAL** (not expanded) |
| 10 CRLF | DIFFERS (→LF) | **IDENTICAL** |
| 12 trailing whitespace | DIFFERS (stripped) | **IDENTICAL** |
| 13 multi-document | DIFFERS (2nd doc dropped) | **IDENTICAL** |
| 14 realistic k8s manifest | DIFFERS | **IDENTICAL** |
| 11 BOM + multiple keys | "identical" bytes but key corrupted | **PARSE ERROR** (see §5.3) |

`noyalib`: **13 / 14 byte-identical**; the one failure is a hard error, not silent
corruption. `rust-yaml`: **1 / 14**.

### 5.2 Surgical edit (the a001 §4.2 property)

`doc.set("replicas", "5")` on the commented file rewrote only that value:

```
# Top-level header comment
name: my-app   # inline comment on a scalar

# Section: replicas
replicas: 5            # <- only this byte changed; every comment + blank line intact

config:
  # nested comment
  debug: true
  level: info
```

Comment preservation across the edit: header `true`, inline `true`, nested
`true`. Multi-document via `parse_stream` parsed 2 documents and re-joined
byte-identically.

### 5.3 The one failure: BOM + multi-node = hard parse error (reproduced)

A UTF-8 BOM followed by **any multi-node** document fails to parse in 0.0.8:

```
"\u{FEFF}a: 1\n"            → round-trips OK (single node)
"\u{FEFF}a: 1\nb: 2\n"      → ERROR: stray content after document …
"\u{FEFF}- 1\n- 2\n"        → ERROR (sequence)
"\u{FEFF}a:\n  b: 1\n"      → ERROR (nested)
```

A `Bom` trivia leaf kind exists (`cst/syntax.rs:33-34`) and a single-node BOM file
round-trips, but the parser mis-scopes the document after a BOM — so essentially
every real-world BOM-prefixed file would be **rejected**. This is one of the very
`b001` dimensions adoption is meant to fix, and upstream docs/tests never disclose
it (their BOM tests are single-key only). Failing loudly is better than
`rust-yaml`'s silent corruption, but it is still a blocker until fixed upstream or
pre-normalized by yqr (stripping a leading BOM before parse is trivial).

**Root cause and upstream fix (in-flight).** The scanner consumes the BOM but
counts its three bytes toward the column of the following content (and treats the
BOM's last byte as the character before a first-line `#`), so the first node lands
at column 3 and a following sibling at column 0 is misread as a dedent below the
document. Fix submitted upstream as PR
[sebastienrousseau/noyalib#118](https://github.com/sebastienrousseau/noyalib/pull/118)
(the project has GitHub issues disabled, so a PR is the only channel): it makes a
leading BOM zero-width in the three column/comment sites and adds scanner
regression tests. Verified locally against this corpus — all BOM forms
(multi-key, sequence, nested, BOM+CRLF, BOM+comment, single) then round-trip
byte-identically. **Status: open, not yet merged or released**, so 0.0.8 from
crates.io still exhibits the bug; the blocker is downgraded but not cleared until
a fixed version ships and yqr pins it.

## 6. How the fidelity works (mechanism, verified in source)

`noyalib`'s CST is byte-faithful **by construction**, not by careful emitter
tuning:

- The `Document` owns the original input **once** as `source: Arc<str>`
  (`cst/document.rs:54`).
- The green tree stores **no text** — each leaf is `Token { kind, len: u32 }`,
  only a byte length (`cst/green.rs:64-69`). Every input byte, including all
  trivia (whitespace, newlines incl. `\r\n`, comments, BOM, directives), is
  recorded as a leaf (`cst/syntax.rs:26-37`); distinct scalar leaf kinds
  (`PlainScalar` / `SingleQuotedScalar` / `DoubleQuotedScalar` / `LiteralScalar` /
  `FoldedScalar`, `cst/syntax.rs:67-77`) preserve quote and block-scalar style.
- `Display`/`to_string()` walks the tree concatenating `source[offset..offset+len]`
  slices (`cst/document.rs:911-913`, `cst/green.rs:128-137`). For an unmodified
  parse this **equals the input**. The project asserts this as a round-trip
  property test over the YAML test suite (`tests/cst_round_trip.rs:140-178`),
  excluding an 18-case `SKIP_LIST` (mostly `%TAG`/tag-shorthand/empty-key edges).
- Edits are a true splice: `replace_span(start,end,repl)` builds
  `source[..start] + repl + source[end..]` (`cst/document.rs:320-324`), then
  **locally re-parses only the smallest affected sub-tree** and reuses every
  untouched sibling via O(1) `Arc` clones (`cst/builder.rs:573-575`), guarded by a
  contract that the re-parsed fragment has the expected kind and exact length
  (`cst/document.rs:406-408`). On any parse failure the document is left unchanged
  (`cst/document.rs:349-355`).
- The higher-level mutators all resolve a path to a span and delegate to
  `replace_span`: `set:458`, `set_value:500`, `remove:557`, `push_back:593`,
  `insert_entry:767`, `insert_after:883`, with `span_at:175` and `get:281` for
  reads, and `materialise_aliases_of` (`cst/anchor.rs:262`) for opt-in alias
  inlining (all `cst/document.rs` unless noted).

### Caveats on the mechanism

- **"Only the touched span" is not universal.** Edits touching/introducing
  anchors, aliases, or tags escalate to a **full-document re-parse**
  (`cst/document.rs:369-376`), as do edits whose local re-parse does not fit. Still
  byte-faithful on unedited regions; just not O(1).
- **`set()` splices verbatim** — no auto-formatting (`cst/document.rs:435-441`).
  `set_value()` does scalar style-matching but is scalar-oriented and rejects
  collections (`cst/document.rs:474-487`); exact block-scalar handling is ambiguous
  between the doc comment and the code. **yqr still owns correct fragment
  generation** (quoting, indentation, block-scalar emission) for non-trivial
  transforms.
- **Optimistic commit.** A locally-repairable but globally-invalid edit (e.g.
  `set("name","[")`) can return `Ok` and commit while `validate()` would fail; only
  edits that escalate to the full re-parse are atomic (`cst/document.rs:213-250`).
  **yqr must call `validate()` after edits.**

## 7. `b001` dimension coverage

| Dimension | Covered by `noyalib` CST? | Evidence |
|---|---|---|
| Comments (leading/inline/nested) | Yes | `Comment` leaf (`syntax.rs:26-37`); empirical 01/14 |
| Blank lines | Yes | Whitespace/Newline trivia tile every byte; empirical 02 |
| Indent width | Yes | leading indent = Whitespace trivia; empirical 03 |
| Single vs double quotes | Yes | distinct quoted-scalar leaf kinds; empirical 04 |
| Literal/folded block scalars | Yes | `LiteralScalar`/`FoldedScalar` incl. header/chomping; empirical 05 |
| Flow vs block | Yes | separate Flow*/Block* composites; empirical 07 |
| CRLF | Yes (alone) | `Newline` leaf verbatim `\r\n`; empirical 10. Caveat: CRLF+BOM hits the BOM bug |
| Trailing whitespace | Yes | Whitespace trivia; empirical 12 |
| Multi-document | Yes | `parse_stream` slices byte ranges; empirical 13 — fixes b001's silent drop |
| Anchors/aliases/merge keys | Yes (preserved, not expanded) | source bytes kept; edits near `&`/`*`/`!` escalate to full re-parse; empirical 09 |
| Number/type fidelity | Yes (CST path) | never re-serialized; `007`, out-of-i64, hi-precision floats round-trip. Note: the **typed `Value`** path can still lose precision — yqr must not route untouched numbers through it |
| Key order (+ duplicate keys) | Yes | verbatim emission; empirical 08 |
| **BOM** | **No / broken in 0.0.8** (fix in-flight, PR #118) | single-node OK; BOM+multi-node = parse error in 0.0.8; fixed by [#118](https://github.com/sebastienrousseau/noyalib/pull/118), pending merge/release (§5.3) |

## 8. API fit for yqr

`noyalib` supplies exactly the two primitives `yqr-a001` §4 needs — a byte-offset
splice (`replace_span`) and a path→span resolver (`span_at`) — plus multi-doc
slicing (`parse_stream`). But the fit is **sharp, not seamless**:

- **No jq query language** (confirmed by grep). yqr keeps its own evaluator and
  must translate each selected node's concrete path into `noyalib`'s string-path
  syntax.
- **Path syntax** is `foo.bar` / `items[0]` / `items[0].name` — **no wildcards,
  no recursive descent** (`cst/document.rs:160-163`). yqr's iteration / `map` /
  recursive selections must be lowered into per-element concrete paths before
  calling `span_at`.
- **String paths only, no key escaping** — the structured `QuerySegment` is
  `pub(crate)`. Special-character keys (dots/brackets/stars in key names) are not
  addressable via `span_at`; yqr would need a fallback that walks the public
  `GreenNode` tree directly to compute spans — partially re-implementing what
  `noyalib` keeps private.
- **Two `Value` models.** `noyalib::Value` (7 variants incl. `Tagged`) differs
  from yqr's; a conversion/bridge layer is unavoidable, or a larger migration of
  `eval.rs` onto `noyalib::Value`.

Net: `noyalib` gives yqr the hard part (lossless CST + spans + splice) but yqr
must still build a path-translation/enumeration adapter and own fragment
formatting.

## 9. Maturity and risk

Verified facts (crates.io, fetched 2026-06-26): published, **max version 0.0.8**
(2026-06-17), 37,601 downloads, created 2026-05-10, **8 versions** (0.0.1–0.0.4
all within ~2 days), none yanked. docs.rs present.

Strong engineering posture: `#![forbid(unsafe_code)]` (`lib.rs:342`, 0 `unsafe`
tokens in src), 9 fuzz targets, Miri, proptest, a high coverage gate,
`cargo audit`/`deny`/`vet`, CodeQL, cosign+SLSA, high OpenSSF Scorecard.

But material risks for a hard-guarantee dependency:

- **Pre-1.0, self-declared breakage.** `doc/POLICIES.md` states every minor bump
  may be breaking; stable 1.0 is targeted ~2028 (CHANGELOG); the maintainer
  documents planned SemVer-breaking refactors (compact-string keys, arena
  lifetimes, **eliminating the `Value` AST**). yqr would build its core guarantee
  on an API permitted to break every release.
- **Bus factor 1.** Single author; security disclosure to a personal email; the
  OpenSSF Code-Review check is satisfied mechanically (auto-approve Dependabot).
- **Docs are demonstrably unreliable** (MSRV stated as 1.75 while the manifest is
  edition-2024 / 1.85; inconsistent YAML-suite pass counts across files; stale
  module docs; a CHANGELOG entry falsely claiming an inconsistency was fixed). The
  README's unqualified "reproduces source byte-for-byte" therefore cannot be
  trusted on faith — which is exactly why we ran our own corpus, surfacing the BOM
  bug the docs never mention.
- **Stricter parser.** The eager green-tree builder is stricter than a lazy
  `from_str` (tab-indentation rejected; ~94 suite cases skipped as parse-rejects).
  Files yqr accepts today via `rust-yaml` could become **hard parse errors** — a
  behavior change, not just a fidelity upgrade. Must be quantified against real
  inputs.

## 10. Dependency and build footprint

- The CST API is gated only on the `std` feature (default on), so the minimal
  line is:

  ```toml
  noyalib = { version = "0.0.8", default-features = false, features = ["std"] }
  ```

- **`serde` is a mandatory, non-feature-gateable dependency** of `noyalib`.
  Adopting it **forfeits yqr's current "no mandatory serde" property** (yqr's
  production tree has no `serde` today).
- Genuinely-new crates are ~5 (`noyalib`, `serde`, `serde_derive`, `rustc-hash`,
  `smallvec`); the proc-macro toolchain and `indexmap`/`memchr` are already pulled
  via `clap_derive` / `rust-yaml`. If `noyalib` fully replaced `rust-yaml`, net
  crate count could **drop ~3** (removing `rust-yaml`'s regex/memmap2/libc/base64
  subtree). During any transition where both are present, the tree grows by ~5.
- MSRV 1.85 < yqr's 1.96 (compatible).

These counts are reasoned from `Cargo.lock`/manifests, not from a resolved
`yqr + noyalib` build (noyalib is not yet wired into yqr).

## 11. Performance — unverified, a real gap

- `doc/BENCHMARKS.md` has **no CST or large-file numbers** (it tops out at
  ~500-item / k8s fixtures via the `Value` path) and **no direct
  noyalib-vs-rust-yaml** comparison. Any "faster" assumption is unsupported.
- `replace_span` copies the **entire source** on every edit (O(n) per edit;
  `cst/document.rs:320-324`); many edits on a large file are O(n × edits).
- `parse_document` eagerly materializes three views (green tree + typed `Value` +
  span tree), so peak memory is higher than a single semantic tree.

yqr targets large files, so CST parse time, edit cost, and peak memory **must be
measured** before committing (§13).

## 12. Options comparison

| Criterion | A: `rust-yaml` + own span layer (a001 §4) | B: full `noyalib` | C: hybrid `noyalib` CST + yqr evaluator |
|---|---|---|---|
| Solves `b001` fidelity | If we build & prove it | Yes (12/13 dims) | **Yes (12/13 dims)** |
| Surgical edit (a001 §4.2) | Build ourselves | Built | **Built (reuse)** |
| Multi-doc / anchors | Build ourselves | Built | **Built (reuse)** |
| jq evaluator | Keep yqr's | **Missing — infeasible** | **Keep yqr's** |
| Implementation effort | High (re-implement + prove) | n/a | **Medium (adapter + bridge)** |
| Dependency risk | Low (stay on current dep) | High (0.0.x core) | **Medium (contained behind adapter)** |
| BOM | We control it | Broken in 0.0.8 (fix in-flight #118) | Broken in 0.0.8 until #118 ships, or pre-normalized |
| Large-file perf | We control it | Unknown | **Unknown — must measure** |
| Foundation built on | the lib that caused `b001` | proven CST design | proven CST design |

Option B is infeasible (no evaluator). The real choice is **A vs C**. C reuses a
working implementation of a001's architecture; A re-implements it on the very
library that caused `b001`. C wins on effort and on correctness-of-design, and
loses on dependency maturity — which the gating plan in §13 is designed to manage.

## 13. Decisive risks and the gating plan

**Blockers before any adoption:**

1. **BOM bug** (§5.3) — fix submitted upstream (PR
   [#118](https://github.com/sebastienrousseau/noyalib/pull/118), open); adoption
   waits on it merging and a release, or yqr pre-strips a leading BOM and restores
   it on emit. Cheap to work around but must be explicit.
2. **0.0.x churn** — pin the **exact** version; wrap the CST behind a thin
   internal `yqr` adapter trait so a break/abandonment is contained and Option A
   stays reachable.
3. **Unverified large-file behavior** — measure.

**Gates (run before committing to C):**

- [ ] Run `noyalib::cst::parse_document` over the full `b001` corpus **and real
      yqr inputs** (K8s/Helm/CI YAML) to quantify: BOM/multi-node failures, and
      how many files yqr accepts today that `noyalib` **hard-rejects** (stricter
      parser).
- [ ] Measure CST parse time, single- and multi-edit cost, and peak memory on
      representative **large** files vs the current `rust-yaml` path.
- [ ] Confirm the `validate()`-after-edit guard closes the optimistic-commit hole.
- [ ] Diff `crates/noyalib/src/cst` across 0.0.1→0.0.8 tags to quantify
      CST-specific API churn (CHANGELOG focuses on the data-binding/serializer
      side and claims "no public API change" for 0.0.7/0.0.8).

**Decision rule:** if the BOM bug is resolved/worked-around and the corpus + perf
gates pass, adopt **C (hybrid)** behind the adapter with a pinned version.
Otherwise, build **A** in-house — but use `noyalib`'s green-tree-of-lengths +
`Arc<str>` source + splice design (§6) as the proven blueprint, which is more
complete than what `rust-yaml`'s scanner tokens offer.

## 14. Follow-up actions

- [x] **Done** — upstream fix for the BOM + multi-node parse error submitted as
  PR [sebastienrousseau/noyalib#118](https://github.com/sebastienrousseau/noyalib/pull/118)
  (issues are disabled on the repo, so a PR is the only channel). Track it to
  merge/release; when a fixed `noyalib` version ships, bump yqr's pin and flip the
  `bom-multinode` expectation in `tests/fidelity.rs` from `Error` to `Identical`.
- File a `yqr` implementation/feature spec for the **fidelity adapter trait**
  (the seam in `yqr-a001` §4.3) so Option A and Option C share one interface.
- Fold the §9 corrections about `noyalib`'s scanner/CST into the eventual
  source-preserving read-path spec (`f002`, proposed in `yqr-r001` §9).

## 15. Open questions

- Can yqr's jq evaluator paths be lowered to `noyalib` string-paths cheaply for
  the common cases, with a green-tree-walk fallback for special-character keys and
  iteration? (api-fit §8.)
- Is the typed-`Value` precision loss fully avoidable by always emitting untouched
  nodes from CST bytes and only synthesizing for genuinely new values? (§7 number
  note.)
- What is the real large-file cost of the three-view eager parse and O(n)-per-edit
  splice at yqr's target sizes? (§11.)

## 16. Appendix — citation index

| Topic | Location |
|---|---|
| Source owned once as `Arc<str>` | `crates/noyalib/src/cst/document.rs:54` |
| Green leaf stores only length | `crates/noyalib/src/cst/green.rs:64-69` |
| `to_string` concatenates source slices | `cst/document.rs:911-913`; `cst/green.rs:128-137` |
| Trivia + scalar leaf kinds (Bom, Comment, quotes, block) | `cst/syntax.rs:26-37,67-77` |
| `replace_span` splice + local repair + safety net | `cst/document.rs:308,320-324,337-355` |
| Anchor/alias/tag edits → full re-parse | `cst/document.rs:369-376` |
| Untouched-sibling reuse via `Arc` clone | `cst/builder.rs:573-575` |
| Splice length/kind contract | `cst/document.rs:406-408` |
| Mutators (set/set_value/remove/push_back/insert_*) | `cst/document.rs:458,500,557,593,767,883` |
| `span_at` / `get` | `cst/document.rs:175,281` |
| Path syntax (no wildcard/recursive) | `cst/document.rs:160-163` |
| `set` splices verbatim; `set_value` scalar-only | `cst/document.rs:435-441,474-487` |
| Optimistic-commit / `validate` | `cst/document.rs:213-250` |
| `parse_stream` multi-doc slicing | `cst/document.rs:984-997` |
| `materialise_aliases_of` | `cst/anchor.rs:262` |
| Round-trip property test + 18-case skip | `tests/cst_round_trip.rs:23-26,140-178` |
| Zero unsafe | `crates/noyalib/src/lib.rs:342` |
| `serde` mandatory; CST under `std` | `crates/noyalib/Cargo.toml` (`[dependencies]`, `[features]`) |
| Breakage policy / 1.0 target | `doc/POLICIES.md`; `CHANGELOG.md` |
| crates.io 0.0.8 / 8 versions / 37,601 dl | crates.io API (verified 2026-06-26) |

*Method note: CST-mechanism and trivia claims were verified against the 0.0.8
source by an adversarial review pass; the `b001`-corpus round-trip, surgical edit,
multi-doc, and BOM results were obtained by running `noyalib` over the corpus in a
path-dependency harness; crates.io metadata was fetched directly.*
