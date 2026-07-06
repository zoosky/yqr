# Bug b002 — noyalib CST deficiencies: span boundaries, duplicate-key policy, and the string-only key model

**Status:** Open (upstream; yqr-side mitigations shipped; deficiency 2.1 RESOLVED in noyalib 0.0.13 and consumed by yqr; 2.2–2.7 each now have an upstream PR **open** — [#147](https://github.com/sebastienrousseau/noyalib/pull/147)–[#152](https://github.com/sebastienrousseau/noyalib/pull/152) — awaiting review/merge/release)
**Severity:** Medium — every hazard is contained by mitigations in yqr's engine adapter, but fidelity or semantics degrade where noyalib's model falls short
**Owner:** yqr maintainers
**Last updated:** 2026-07-05
**Affects:** the `--engine noyalib` fidelity read path (`yqr-f002`); irrelevant to the default pipeline
**Component:** `noyalib` 0.0.13 (`cst::Document::span_at`, `Document::as_value`, the `Value` mapping model)
**Related:** `yqr-f002` §4a (mitigations), `yqr-r002` (evaluation), `yqr-m002` §7.2 (backend C), upstream precedent [noyalib#118](https://github.com/sebastienrousseau/noyalib/pull/118)/[#123](https://github.com/sebastienrousseau/noyalib/pull/123) (BOM fix, merged); deficiency 2.1 fix ([noyalib#143](https://github.com/sebastienrousseau/noyalib/pull/143), closed) folded into the **noyalib 0.0.13** release (PR #145); deficiencies 2.2–2.7 fixed on the `zoosky/noyalib` fork and submitted as [#147](https://github.com/sebastienrousseau/noyalib/pull/147) (2.7), [#148](https://github.com/sebastienrousseau/noyalib/pull/148) (2.3), [#149](https://github.com/sebastienrousseau/noyalib/pull/149) (2.6), [#150](https://github.com/sebastienrousseau/noyalib/pull/150) (2.2), [#151](https://github.com/sebastienrousseau/noyalib/pull/151) (2.4), [#152](https://github.com/sebastienrousseau/noyalib/pull/152) (2.5)

## 1. Summary

Implementing and adversarially reviewing yqr's fidelity read floor surfaced
seven deficiencies in noyalib's CST layer (all empirically confirmed on
0.0.12). None invalidates the core byte-identity property — `parse_stream`
round-trips remained byte-exact on 45+ adversarial inputs — but each degrades
either **span faithfulness** (`span_at` returning ranges that do not denote the
selected node's value) or **typed-view semantics** (the string-only mapping key
model). yqr's adapter guards against all of them (§4); this spec records the
upstream root causes so they can be reported and, when fixed, the guards
simplified.

Upstream has GitHub issues disabled, so each item is a **PR-with-fix
candidate**, following the accepted #118/#123 precedent.

## 2. Deficiencies (each confirmed on 0.0.12)

### 2.1 `span_at` resolves duplicate keys first-wins; `as_value` is last-wins — **RESOLVED (noyalib 0.0.13)**

```text
k: one
k: two
```

`as_value()` yielded `k = "two"` (last-wins, matching YAML loaders and jq), but
`span_at("k")` returned the span of the **first** `k`'s value. The two accessors
of one document disagreed about which node a path denotes — the consumer got
bytes of a node the typed view did not select (wrong-node hazard).

**Upstream ask:** make `span_at` resolution policy match `as_value`
(last-wins), or expose occurrence-aware addressing.

**Upstream status: RESOLVED.** Submitted as
[noyalib#143](https://github.com/sebastienrousseau/noyalib/pull/143); that PR was
closed and its fix **folded into the 0.0.13 release** (commit `a472e14`, PR #145,
crediting the original author) — `fix(loader): duplicate mapping keys last-wins
across span views`. The root cause was deeper than `span_at` alone: the loader
appended a span entry per source occurrence while the `IndexMap` collapsed
duplicates, de-syncing `Value::Mapping` from its parallel `SpanTree`, so
`span_at`/`get`/`Spanned<T>`/`remove` all mis-paired for keys at or after a
duplicate. The fix makes both the loader (replace span entry in place) and the
green-tree walker (`walk_mapping`) last-wins.

**yqr status: consumed in 0.0.13.** yqr bumped `noyalib` to 0.0.13; the re-parse
guard now verifies the correct last-wins bytes, so a duplicate-key projection
emits real bytes instead of degrading. Tests
`duplicate_keys_resolve_to_last_occurrence` /
`duplicate_collection_keys_resolve_to_last_occurrence` pin the new behavior. The
guard is retained for the residual cases (2.2 implicit-null indicator spans, 2.5
keep-chomped block scalars, aliases).

### 2.2 Implicit nulls yield degenerate indicator spans

```text
c:
other: 1
```

`span_at("c")` returns a 1-byte span covering the `:` indicator (and `- ` for
an empty sequence item) — bytes that neither denote the null value nor parse
as YAML. Expected: no span (the node has no bytes of its own).

**Upstream PR: [#150](https://github.com/sebastienrousseau/noyalib/pull/150) (open).**
The parser synthesizes an absent value as an empty plain scalar carrying the
`:` / `-` indicator's span, which the loader materialized as a `SpanTree::Leaf`.
The fix marks such a scalar (empty value, plain style, no anchor/tag) with a
**zero-width leaf**, and `resolve_span` reports `None` for a zero-width leaf.
Explicit nulls (`~`, `null`) and quoted empties (`''`) keep their real spans.

### 2.3 Keep-chomped block scalars: span excludes kept trailing blank lines

```text
key: |+
  kept


```

The value is `"kept\n\n\n"` (chomping `+` keeps trailing newlines), but
`span_at("key")` ends after `kept\n` — the returned slice re-parses to a
*different value* (`"kept\n"`). Content-bearing bytes are outside the span.

**Upstream PR: [#148](https://github.com/sebastienrousseau/noyalib/pull/148) (open).**
Root cause was *not* in the scanner (the green leaf carries the correct
`[|+ … kept\n\n\n]` span) but in `span_at`, which unconditionally ran every
resolved span through `trim_trailing_blank`. For a keep-chomped scalar those
trailing `\n` are content. The fix routes value spans through `trim_value_span`,
which leaves `|+` / `>+` spans intact and trims clip/strip and every other node
as before.

### 2.4 Block-collection spans start at the first key

```text
config:
  # lead
  debug: true
  # mid
  level: info
```

Two consequences: (a) the first line's **indentation lies outside the span**,
so the raw slice is mis-indented — a downstream parser either rejects it or
silently re-nests it (`- alpha\n    - beta` re-parses as `["alpha - beta"]`);
(b) **leading interior comments** (`# lead`, between the parent key line and
the first child) are excluded while later interior comments (`# mid`) are
included — inconsistent comment retention for subtree extraction.

**Upstream ask:** offer a span variant covering the node's full block (line
start, leading trivia included), or document the boundary rule.

**Upstream PR: [#151](https://github.com/sebastienrousseau/noyalib/pull/151) (open).**
Fixes **2.4a**: `resolve_value_in_entry` / `resolve_value_in_item` widen a block
collection's start to its line start (over inline indentation), but only when
the value begins its own line — a value sharing its line with `-`/`:` (e.g. the
inner sequence of `- - a`) is untouched. The slice is then uniformly indented
and re-parses to the selected value. **2.4b** (leading interior comments) is
left as-is: a documented boundary, not a correctness issue for value extraction.
Consumption note: this also makes a *last-wins duplicate* block value emit with
its indentation (`  a: 2` rather than `a: 2`) — the yqr guard test
`duplicate_collection_keys_resolve_to_last_occurrence` must update its expected
bytes when the pin is bumped.

### 2.5 String-only mapping key model loses key types and can lose entries

`noyalib::Mapping` is `FxIndexMap<String, Value>`. Consequences:

- `true: yes` is keyed by the string `"true"` — consumers that model typed
  keys (yqr's classic pipeline keys it by `Bool(true)`) get different lookup
  semantics for the same document.
- Distinct YAML keys that share a spelling — `1: a` and `"1": b` — **collide**;
  one entry is silently dropped from `as_value()`. That is data loss inside
  the typed view itself.

**Upstream ask:** a typed-key mapping variant (`MappingAny` exists internally)
or at minimum a collision diagnostic on `as_value`.

**Upstream PR: [#152](https://github.com/sebastienrousseau/noyalib/pull/152) (open).**
Takes the collision-diagnostic route (a `Value::MappingAny` variant would be a
semver-major touching every exhaustive `match`). The spanned loader retains each
entry's original typed key and raises the new `Error::KeyCollision` when a string
key is produced by a *different* typed key (`1`/`"1"`, `true`/`"true"`,
`~`/`"null"`); a genuine authored duplicate (same typed key) still follows
`DuplicateKeyPolicy`. **Scope:** patches the spanned `load_one` path
(`parse_document`/`parse_stream`/`as_value`) that yqr consumes; the no-span
`load_all_no_spans` path and the serde `Value` deserializer collapse the same
keys through independent code and are noted in the PR as a follow-up.

### 2.6 Alias references slice as dangling `*name`

`b: *anc` → `span_at("b")` is the `*anc` bytes: standalone they are a dangling
alias (unparseable out of context), asymmetric with anchor *definitions*
(`&x 1`), whose slice is self-contained. A "resolve through alias" or
"no span for alias references" policy would be more consumer-friendly.

**Upstream PR: [#149](https://github.com/sebastienrousseau/noyalib/pull/149) (open).**
Takes the **resolve-through** route (strictly more useful than `None`). The
green-tree walker lumped `AliasMark` with anchor/tag prefixes and returned the
alias's own bytes. It now bails (`None`) on `AliasMark` so `span_at` falls to the
typed cache, whose `SpanTree` already carries the alias node's cloned
anchor-definition span. `span_at("b")` on `a: &anc [1, 2]\nb: *anc` returns
`[1, 2]` (re-parseable); anchor definitions are unchanged.

### 2.7 Parser strictness deltas

Classic-Mac CR-only line endings (`a: 1\rb: 2\r`) are rejected
(`inconsistent indentation`) though other YAML parsers accept them. Minor;
worth an upstream test case.

**Upstream PR: [#147](https://github.com/sebastienrousseau/noyalib/pull/147) (open).**
The scanner tracked line breaks by matching `\n` only in **three** places (not
one, as first assumed): `advance`, `advance_by`, and the block-mapping simple-key
line-start scan. A lone `\r` grew the column instead of resetting it and made a
following key's column over-count, splitting the mapping and yielding a spurious
"stray content after document". All three now treat a lone CR as a line break
(YAML 1.2.2 §5.4). Byte offsets/spans are untouched, so round-trip is unaffected;
CRLF is consumed via `advance_by`'s trailing `\n` and is unchanged.

## 3. Impact on yqr

Without mitigation, 2.1–2.4 caused **wrong or misleading engine output**
(bytes of unselected nodes, `:` as a projection result, value-changing
slices, silently re-nesting output) and 2.5 caused **filter-result divergence
and silent entry loss** in engine mode. All were caught by the f002
adversarial review passes, none by the happy-path suite — a reminder that
span APIs need adversarial corpora.

## 4. yqr-side mitigations (shipped in f002)

| Deficiency | Mitigation in `src/fidelity/noyalib.rs` |
|---|---|
| 2.1 dup-key wrong node | **Fixed upstream in noyalib 0.0.13** (`span_at` last-wins): a duplicate-key projection now emits the last occurrence's real bytes. The re-parse guard is retained for 2.2/2.5/aliases and still verifies the (now correct) slice |
| 2.2 indicator spans | empty/whitespace slices and value-mismatched slices → `Synthetic` (renders `null`) |
| 2.3 `\|+` kept blanks | slice re-parses to `"kept\n"` ≠ typed `"kept\n\n\n"` → `Synthetic` (value-correct) |
| 2.4a first-line indent | span **extended to line start** when the prefix is pure indentation; emitted bytes are uniformly indented and verified in emitted form |
| 2.4b leading comments | documented boundary rule (README, f002); comments above the first key stay with the parent |
| 2.5 key model | entry-count cross-check against the default loader at `open()`; collisions are refused loudly. Non-colliding non-string keys are allowed and documented as a semantic divergence |
| 2.6 dangling alias | fails the re-parse guard → `Synthetic` (typed expansion) |
| 2.7 strictness | documented as an engine limitation |

## 5. Suggested upstream plan

Ordered by value to yqr (each is a small, testable PR in the #118 mold). All
seven are now submitted; 2.1 is released, 2.2–2.7 are open PRs:

1. **2.1** `span_at` last-wins (or occurrence-aware) — removes the whole
   wrong-node class at the source. **DONE: [noyalib#143](https://github.com/sebastienrousseau/noyalib/pull/143) folded into 0.0.13 (PR #145); yqr bumped the pin and updated the tests.**
2. **2.3** include kept trailing blanks in keep-chomped block-scalar spans. **PR [#148](https://github.com/sebastienrousseau/noyalib/pull/148) (open).**
3. **2.2** return `None` for byte-less implicit nodes. **PR [#150](https://github.com/sebastienrousseau/noyalib/pull/150) (open).**
4. **2.4** block-collection span starts at its line start (2.4a). **PR [#151](https://github.com/sebastienrousseau/noyalib/pull/151) (open);** 2.4b leading-trivia inclusion deferred.
5. **2.5** collision diagnostic (`Error::KeyCollision`) on the spanned path. **PR [#152](https://github.com/sebastienrousseau/noyalib/pull/152) (open).**
6. **2.6** resolve alias references through to the anchor value span. **PR [#149](https://github.com/sebastienrousseau/noyalib/pull/149) (open).**
7. **2.7** lone-CR line breaks. **PR [#147](https://github.com/sebastienrousseau/noyalib/pull/147) (open).**

## 6. Acceptance criteria

- [x] Each deficiency reported upstream (PR or discussion, issues disabled).
      2.1 done (noyalib#143 → folded into 0.0.13 via PR #145); 2.2–2.7 submitted
      as [#147](https://github.com/sebastienrousseau/noyalib/pull/147)–[#152](https://github.com/sebastienrousseau/noyalib/pull/152)
      (fork `zoosky/noyalib`), each with a regression test and verified against
      yqr's `backend-noyalib` fidelity suite before submission.
- [ ] For every upstream fix that lands and releases: bump the pin, remove or
      simplify the corresponding adapter guard, and keep the regression tests
      (they must pass against the fixed backend too). Pending merge/release of
      #147–#152. Consumption deltas to apply then: 2.3 & 2.6 guard tests flip
      `Synthetic → Found`; 2.4 changes
      `duplicate_collection_keys_resolve_to_last_occurrence`'s expected bytes to
      `  a: 2`; 2.5 lets `open()` drop its rust-yaml entry-count cross-check;
      2.7 lets the line-ending guard add a CR-only case.
- [ ] Residual limitations that upstream declines to change remain documented
      in README and `yqr-f002`. Open sub-items even if #147–#152 merge: 2.4b
      (leading interior comments) and 2.5 on the no-span / serde paths.
