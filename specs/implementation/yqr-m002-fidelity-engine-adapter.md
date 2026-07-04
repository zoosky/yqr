# Implementation m002 — Fidelity-Engine Adapter (the a001 §4.3 seam)

**Status:** In Progress (read floor shipped on backend C via `yqr-f002`; backend A pending rust-yaml#73; write tier unstarted)
**Owner:** yqr maintainers
**Last updated:** 2026-07-03
**Implements:** `yqr-a001` §4.3 (the source/span implementation seam)
**Related:** `yqr-b001` (the fidelity bug this unblocks), `yqr-r001` §9, `yqr-r002` (the rust-yaml-vs-noyalib backend decision this abstracts over)

## 1. Purpose

`yqr-a001` makes a hard guarantee — **yqr never rewrites bytes it did not
change** — and §4.3 calls for an *implementation seam*: a backend-agnostic
interface so yqr's own jq evaluator can drive fidelity-preserving reads and edits
over **either** of the two engines evaluated in `yqr-r002`:

- **Backend A** — yqr's own source/span layer built on `rust-yaml`'s public
  scanner `Token` stream (no new dependency; this is what yqr ships today, and
  fixing `b001` is exactly building this).
- **Backend C** — `noyalib`'s lossless CST (`cst::Document`), adopted behind a
  feature flag once its maturity/BOM gates clear (`yqr-r002` §13).

This spec defines that seam: a Rust trait pair (`FidelityEngine` +
`FidelityEdit`), the supporting types, the per-node resolution semantics, the
backend mappings, the module layout, and the first shippable increment. It is the
single interface both backends satisfy so the decision between A and C never
leaks into yqr's evaluator.

## 2. Goals / non-goals

**Goals**

- One object-safe interface that yqr holds as `Box<dyn FidelityEngine>`, chosen at
  startup, with **read fidelity shippable before any mutation exists**.
- Enforce a001 §2 *by construction*: a selected node is emitted by **slicing its
  original bytes**, never re-serialized; only synthesized values are formatted.
- Keep yqr's evaluator unchanged in spirit: it still runs over
  `rust_yaml::Value`; the seam only adds a `path → span` resolution it threads
  alongside each produced value.
- Make every fidelity compromise **visible per node**, never silent (the a001
  "never silently rewrite" promise).
- Multi-document, BOM, and CRLF are first-class from day one (a001 §5).

**Non-goals**

- A query language. yqr keeps its own lexer/parser/eval; the trait never parses
  jq and never interprets jq semantics (iteration, negative indices, `select`).
- Structural edits (insert/append/delete an entry) in the first write surface —
  those are a later, separately-specified extension on top of `splice`.
- A borrowing zero-copy value model — `value()` returns an owned `Value` (see
  §12 risk on allocation cost).

## 3. Design decisions

Chosen by a three-stance design panel (read-first-minimal / full-read-mutate /
capability-oriented) with an adversarial realizability check that every method
maps to **both** backends. The synthesis:

1. **Layered supertrait, not capability flags.** `FidelityEngine` is the read
   floor every backend implements; `FidelityEdit: FidelityEngine` is an additive
   supertrait a backend opts into. A read-only backend is a *complete, compiling*
   implementation with no `unimplemented!` stubs — "can it mutate?" is a
   compile-time fact (does it implement `FidelityEdit`?), surfaced at runtime via
   the `Loaded` enum.
2. **Per-node `Resolved` enum is the fidelity-correctness core.** A four-way
   outcome — `Found` / `Synthetic` / `Absent` / `Unaddressable` — keeps jq-`null`
   (`Absent`) cleanly separate from "this backend cannot slice it faithfully"
   (`Unaddressable`). The latter triggers *visible* lossy fallback for that one
   node; every other byte stays faithful. This is what lets two structurally
   different backends coexist without a lowest-common-denominator.
3. **One structural parse owns both `value()` and `resolve()`.** A contract, not a
   hint: the typed value the evaluator walks and the spans the emitter slices must
   come from the *same* parse, so a path valid against `value()` can never resolve
   to the wrong span (duplicate keys, merge keys, anchors).
4. **Parsing lives outside the object-safe traits** (the `open()` factory), so the
   trait objects carry no lifetime and each engine owns its source bytes.
5. **Structured `Path`/`PathSeg` with `is_plain()`** turns noyalib's
   "no-key-escaping" limit into a *deterministic* `Unaddressable::SpecialCharKey`
   instead of a silent mis-resolution.

## 4. The interface (canonical)

> The Rust below is the canonical seam. When implemented, cross-references to
> `a001`/this spec must live in plain `//` comments, **not** `///`/`//!` doc
> comments (CLAUDE.md rule 19 — spec IDs must not render in `cargo doc`). The
> doc-comment spec references shown here are for the spec only.

```rust
use std::borrow::Cow;
use crate::error::Result;
use rust_yaml::Value; // yqr's evaluation currency (lib.rs `pub use rust_yaml::Value`)

/// A half-open byte range `[start, end)` into `FidelityEngine::source`.
/// Byte offsets line up with rust-yaml `Position.index` and noyalib `(usize,usize)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span { pub start: usize, pub end: usize }

impl Span {
    pub const fn new(start: usize, end: usize) -> Self { Self { start, end } }
    pub const fn len(self) -> usize { self.end - self.start }
    pub const fn is_empty(self) -> bool { self.start == self.end }
    /// The entire read-path emit primitive: print a node with
    /// `print!("{}", span.slice(engine.source()))` — no re-serialization.
    pub fn slice<'s>(&self, source: &'s str) -> &'s str { &source[self.start..self.end] }
}

/// One step of a concrete, fully-resolved access path. yqr's evaluator desugars a
/// filter into these *after* resolving dynamic forms; the key is logical (decoded).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSeg {
    Key(String),
    Index(i64), // may be negative; backend resolves against live length
}

impl PathSeg {
    /// Expressible through a plain dotted/bracketed string-path backend (noyalib)
    /// with no key escaping. Lets such a backend return `Unaddressable`
    /// deterministically instead of mis-resolving a key like `"a.b"`.
    pub fn is_plain(&self) -> bool {
        match self {
            PathSeg::Index(_) => true,
            PathSeg::Key(k) => !k.is_empty() && !k.contains(['.', '[', ']', '*']),
        }
    }
}

/// A concrete path root→node. Empty == document root (how `.` round-trips).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Path(pub Vec<PathSeg>);

impl Path {
    pub fn root() -> Self { Path(Vec::new()) }
    pub fn is_root(&self) -> bool { self.0.is_empty() }
    pub fn child(&self, seg: PathSeg) -> Self { let mut n = self.0.clone(); n.push(seg); Path(n) }
    pub fn segments(&self) -> &[PathSeg] { &self.0 }
    pub fn is_fully_plain(&self) -> bool { self.0.iter().all(PathSeg::is_plain) }
}

/// Why a node that genuinely exists cannot be sliced from source on this backend.
/// NOT absence (jq `null`) — "no faithful span available here".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unaddressable {
    /// Key uses characters a string-path backend cannot express. The rust-yaml
    /// backend addresses these natively (token-text match) and never returns this
    /// — a declared, honest inter-backend gap.
    SpecialCharKey(String),
    /// Reachable only through an alias/merge; no stable source bytes without
    /// materialising (write-path concern).
    AliasIndirect,
    /// Recognised path, no span index for this node kind yet (e.g. flow interior
    /// in the first increment). `&str` names the kind for diagnostics.
    Unindexed(&'static str),
}

/// Outcome of resolving a concrete path against one document. The four arms are
/// mutually exclusive and each drives a distinct emit choice in yqr.
#[derive(Debug)]
pub enum Resolved<'a> {
    /// Found. `bytes` is the exact original source (read path emits verbatim);
    /// `span` locates it for a later `FidelityEdit::splice`.
    Found { span: Span, bytes: &'a str },
    /// Valid path selecting an implicit node with no source bytes (implicit null);
    /// caller re-serializes from its own `Value`.
    Synthetic,
    /// Path does not resolve here. jq `null`, not an error.
    Absent,
    /// Real node this backend cannot address faithfully. yqr falls back to lossy
    /// `crate::render` for THIS node only (and may warn); every other byte stays faithful.
    Unaddressable(Unaddressable),
}

/// Stable backend id (diagnostics + the `open` factory).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BackendId { RustYamlSpans, NoyalibCst }

/// The read/query seam (a001 §4.1) — the floor every backend implements.
/// Object-safe: every method takes `&self`; parsing is kept out (see `open`).
///
/// CONTRACT: `value()` and `resolve()` MUST derive from the SAME structural parse.
pub trait FidelityEngine {
    fn backend_id(&self) -> BackendId;

    /// The entire original input, byte-for-byte (BOM, CRLF, trailing ws, `---`/`...`).
    /// `yqr '.' f` emits exactly this; the a001 §2 gate is `source() == cat f` unedited.
    fn source(&self) -> &str;

    /// Number of logical documents (always >= 1).
    fn doc_count(&self) -> usize;

    /// Byte span of document `doc`, including the inter-doc trivia it owns by the
    /// §6.1 boundary policy. Concatenating every `doc_span` slice reproduces `source()`.
    fn doc_span(&self, doc: usize) -> Option<Span>;

    /// Typed value of document `doc`. The only point a native model (noyalib's
    /// 7-variant Value incl. Tagged) lowers into yqr's Value; intentionally lossy,
    /// never used for faithful emit (fidelity comes from `resolve`).
    fn value(&self, doc: usize) -> Result<Value>;

    /// Resolve a concrete path against document `doc`. Root → `Found` over the
    /// whole-doc span. Missing key → `Absent` (jq null), not an error.
    fn resolve(&self, doc: usize, path: &Path) -> Result<Resolved<'_>>;
}

/// Preferred quoting when a typed scalar replacement is restyled to the site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteHint { MatchSite, Plain, SingleQuoted, DoubleQuoted }

/// What a splice puts at a target span. Mirrors noyalib `replace_span`/`set`
/// (verbatim) vs `set_value` (typed, restyled).
#[derive(Debug, Clone)]
pub enum Replacement {
    Verbatim(String),
    Scalar { value: Value, quote: QuoteHint },
}

/// The write seam (a001 §4.2), layered additively. Read fidelity ships WITHOUT it.
pub trait FidelityEdit: FidelityEngine {
    /// Splice `repl` into `span` of `doc`, copy every other byte verbatim, then
    /// re-validate that the result still parses as the same number of documents.
    /// On failure the stream is left untouched.
    fn splice(&mut self, doc: usize, span: Span, repl: &Replacement) -> Result<()>;

    /// The current (possibly edited) stream. `Cow` so the unedited identity path
    /// is allocation-free and byte-identical to `source()`.
    fn render(&self) -> Result<Cow<'_, str>>;
}

/// A loaded source at whichever capability tier its backend supports.
pub enum Loaded {
    ReadOnly(Box<dyn FidelityEngine>),
    Mutable(Box<dyn FidelityEdit>),
}

/// Parse `source` with the chosen backend, keeping bytes verbatim. Parsing lives
/// here (outside the object-safe traits) so engines own their source. yqr sets
/// `want_mutation` after checking whether the filter assigns/deletes; a read-only
/// backend asked to mutate errors early.
pub fn open(backend: BackendId, source: impl Into<String>, want_mutation: bool) -> Result<Loaded>;
```

## 5. Per-node resolution semantics

`resolve(doc, path)` is where a001's "never silently rewrite" guarantee is
enforced, one node at a time:

| `Resolved` arm | When | yqr's emit choice |
|---|---|---|
| `Found { span, bytes }` | node exists with original source bytes | **slice** `bytes` verbatim (zero re-serialization) |
| `Synthetic` | valid path, implicit node (e.g. implicit null value) | re-serialize from the typed `Value` |
| `Absent` | path does not resolve | emit `null` (jq semantics) — not an error |
| `Unaddressable(_)` | real node this backend cannot slice | **visible** lossy fallback via `crate::render` for this node only; everything else faithful |

The critical separation is `Absent` (jq `null`) vs `Unaddressable` (a fidelity
compromise). Collapsing them — as a naive `Option<Span>` would — is exactly the
silent-rewrite failure a001 forbids.

## 6. Read path and write path

### 6.1 Read (a001 §4.1) — slice-on-emit
yqr's evaluator threads a `Path` next to each `Value` it produces, extending it
with `Path::child` on field/index steps and branching it on `.[]` iteration
(negative indices are resolved to non-negative offsets in yqr *before* a `PathSeg`
is built). To emit a result, yqr calls `resolve(doc, path)`:
- a **path-derived** result (untouched projection) → `Found` → slice;
- a **computed** result (literal, `{...}`, `[...]`, arithmetic) carries no path →
  re-serialize via `crate::render`.

This requires a small **evaluator change** (not a trait change): each produced
value carries `Option<Path>`, set to `None` for computed values. That provenance
threading is the prerequisite for wiring `resolve()` into emit.

**Document-boundary policy (must be specified and identical across backends):**
leading trivia (BOM, comments, blank lines before the first `---`) belongs to
document 0; trivia between `...`/`---` and the next node belongs to the following
document; the last document extends to EOF including trailing trivia.
Concatenating `doc_span` slices must reproduce `source()` exactly.

### 6.2 Write (a001 §4.2) — splice
A single edit is `(span, Replacement)`. yqr already holds the `span` from a prior
`resolve`, so `splice` is uniform: replace those bytes, copy the rest, re-validate
by re-parse. `Replacement::Verbatim` is universal; `Replacement::Scalar` is
restyled to the site (indent + quote) by backends that can, else the caller
pre-formats to `Verbatim`. Structural edits are a later extension.

## 7. Backend mappings

### 7.1 Backend A — rust-yaml token+span layer (ships first)

> **Update (2026-07-03): backend A may reduce to a thin adapter.** The span
> layer described below was implemented **upstream** and submitted as
> [rust-yaml#73](https://github.com/elioetibr/rust-yaml/pull/73)
> (`rust_yaml::RoundTripDocument`: source verbatim + event×token span index +
> `span_of`/`get`/`set`/`replace_span` + `parse_all` multi-doc + BOM handling;
> yaml-test-suite fidelity property 1,501/0). If #73 merges and releases, this
> backend becomes an adapter mapping the seam onto the upstream API —
> `source()` → `RoundTripDocument::source()`, `doc_*` → `parse_all` slices,
> `value()` → `RoundTripDocument::value()` (upstream honors the parse-once
> contract via `load_all_str`), `resolve()` → `span_of(&[PathSegment])` with
> `None` mapped to `Absent`/`Synthetic`/`Unaddressable` by consulting `value()`,
> `splice()` → `replace_span`. Caveats vs. this spec: upstream paths are
> `PathSegment::Key(&str)`/`Index(usize)` (no negative indices — resolve in
> yqr), and merge-key/alias-interior nodes resolve to no span (map to
> `Unaddressable`, matching §5). Only if #73 is rejected does the in-yqr build
> below proceed.
>
> **Update (2026-07-04): backend A SHIPPED as the adapter** (`yqr-f003`).
> `src/fidelity/rustyaml.rs` maps the seam onto `rust_yaml_rt::RoundTripDocument`
> from the fork's `feat/roundtrip-document` branch, behind the
> `backend-rust-yaml` feature and the `--engine rust-yaml` switch. It never
> produces `Unaddressable` (keys resolve by scalar text), keeps full typed keys
> (no collision guard), and resolves duplicates last-wins. The in-yqr token
> walk below was not needed.

| Method | Implementation (in-yqr build, if #73 does not land) |
|---|---|
| `source()` | the owned input `String`, verbatim |
| `doc_count` / `doc_span` | from `DocumentStart`/`DocumentEnd` token positions (marker-less input = one implicit doc; last doc → EOF incl. trailing trivia) |
| `value(doc)` | `rust_yaml::Value` built from the **same** structural token walk that builds the span index — **do not** call `Yaml::load_str` a second time (the §3.3 parse-once contract) |
| `resolve(doc, path)` | walk the span index by segment; `Key` via **token-text match** (handles special-char keys natively → never `SpecialCharKey`); `Index` resolves negatives against length; root → whole-doc span; missing → `Absent`; not-yet-indexed kinds → `Unaddressable::Unindexed` |
| `splice` (`FidelityEdit`) | edit the owned `String`, re-validate by re-parse |

### 7.2 Backend C — noyalib CST (feature-gated, after r002 gates)

| Method | Implementation |
|---|---|
| `source()` | reconstructed whole-stream bytes (`parse_stream` docs each own a slice) |
| `value(doc)` | lower `Document::as_value()` (noyalib `Value`) into `rust_yaml::Value` |
| `resolve(doc, path)` | if `path.is_fully_plain()` → build noyalib string path (`a.b`, `items[0].name`), call `span_at` → `Found`; else → `Unaddressable::SpecialCharKey`; root → whole-doc span; missing → `Absent` |
| `splice` (`FidelityEdit`) | `Document::replace_span` (`Replacement::Scalar` → `set_value`, scalar-only); `render` via `to_string` |

## 8. Module layout

`src/fidelity/` as a directory module (CLAUDE.md rule 9, 500-line split),
re-exported from `lib.rs` via `pub mod fidelity;`:

- `src/fidelity/mod.rs` — the seam: the two traits + `Span`, `Path`/`PathSeg`,
  `Resolved`/`Unaddressable`, `Replacement`/`QuoteHint`, `BackendId`, `Loaded`,
  `open()`. If it nears ~500 lines, split the plain data types into
  `src/fidelity/types.rs` and `pub use types::*;` to keep import paths stable.
- `src/fidelity/rust_yaml.rs` — `RustYamlEngine` (backend A). The token-stream
  span-index builder is the bulk; pre-plan a further split
  (`src/fidelity/rust_yaml/span_index.rs`) before it crosses the limit.
- `src/fidelity/noyalib.rs` — `NoyalibEngine` (backend C), behind a
  `noyalib-backend` feature.

## 9. First increment

> **Update (2026-07-03): the read floor SHIPPED — on backend C, inverting the
> ordering below.** Implemented by `yqr-f002`: `src/fidelity/` (seam + `open` +
> `run` driver), the noyalib backend behind `backend-noyalib`, provenance
> threading in the evaluator, and the CLI `--engine noyalib` switch. The
> ordering inverted because noyalib 0.0.12 is released and verified while
> backend A's substrate ([rust-yaml#73](https://github.com/elioetibr/rust-yaml/pull/73))
> is still in review — backend A follows as an adapter when #73 lands (§7.1).
> Deviations from the sketch in this section are recorded in `yqr-f002` §2.

**Original plan: ship the read floor only, on backend A (rust-yaml spans) — no
new dependency; this is exactly what `b001` needs.**

1. Add `src/fidelity/mod.rs` (traits + value types + `open`).
2. Implement `FidelityEngine` for `RustYamlEngine`: `source()` verbatim;
   `doc_count`/`doc_span` from document-marker tokens; `value()` + `resolve()` from
   one shared token walk.
3. Thread `Option<Path>` through the evaluator and switch emit to slice-on-`Found`.
4. Land the `b001` fidelity corpus as the gate (§10).

Deferred to later increments: `FidelityEdit` (mutation), flow-collection interior
spans, the noyalib backend, structural edits, a batched multi-splice API.

## 10. Contract and acceptance

This seam is correct when, on backend A:

- [ ] `engine.source() == cat f` for every file in the `b001` corpus (the a001 §2
      identity gate), enforced by `tests/fidelity.rs` extended to drive the engine.
- [ ] A path projection (`.a.b`) emits the selected node's **original bytes**
      (comments/quotes/indent intact), via `Found`.
- [ ] A computed result (`{x: 1}`, `1+1`) re-serializes (carries no path).
- [ ] A missing path emits `null` (`Absent`), distinct from any `Unaddressable`.
- [ ] Multi-document, BOM, and CRLF inputs round-trip (doc-boundary policy holds).
- [ ] (When `FidelityEdit` lands) a single-scalar assignment changes only that
      span; `diff` shows exactly one hunk.

## 11. Relationship to `yqr-r002`

This seam is what makes the r002 A-vs-C decision **reversible and contained**:
both options implement the same trait, so yqr can ship backend A now (Option A,
no dependency, fixes `b001`) and later add backend C (Option C, noyalib) behind a
feature flag without touching the evaluator — exactly the "thin internal adapter"
r002 §3/§13 recommends. If noyalib's gates never clear, backend A stands alone; if
they do, backend C drops in.

## 12. Risks and open issues

- **Span-boundary policy is load-bearing and unspecified.** The two backends WILL
  slice different bytes for the same path unless the trailing-trivia rule is
  pinned: noyalib's `span_at` trims trailing blanks; a rust-yaml token walk may
  include or exclude them. Define the policy once (in §6.1) and conformance-test
  both backends against it.
- **rust-yaml collection spans via `BlockEnd` are uncertain.** `BlockEnd` can be a
  zero-width token positioned at the following dedented line, so a collection's
  union span may over-cover trailing blank lines/comments. Validate before relying
  on collection-level `Found`.
- **BOM/CRLF byte-accuracy on backend A is unverified.** rust-yaml has no BOM-skip
  (a leading `U+FEFF` is scanned as ordinary content); `Position.index` advances
  by `len_utf8` so arithmetic stays consistent, but span *meaning* near a BOM must
  be checked against the corpus.
- **The parse-once contract must actually be honored** in backend A (build `Value`
  from the token walk; do not re-`load_str`), or `value()`/`resolve()` can diverge
  on duplicate/merge keys.
- **Multi-doc boundary parity** between backend A's hand-rolled splitter and
  noyalib's `parse_stream` inter-doc trivia ownership must match.
- **Inter-backend fidelity divergence is real and by design:** special-char/quoted
  keys are `Found` on backend A (token match) yet `Unaddressable::SpecialCharKey`
  → lossy on backend C. Documented, not hidden.
- **Provenance threading is an evaluator change** (`eval` carries `Option<Path>`
  beside each `Value`, `None` for computed) — a prerequisite, not part of the trait.
- **`value()` returns an owned `Value` clone per document** (object-safe; no borrow
  across noyalib's `Ref` and rust-yaml's owned `Value`) — a per-call allocation on
  large files; a borrowing variant is a possible later optimization.
- **`splice` re-validates by full re-parse** (O(edits × parse)); a batched
  non-overlapping multi-splice API that re-indexes once is a likely follow-up,
  deliberately excluded from the first write surface.
