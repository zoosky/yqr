# Feature f027 — Upstream the anchor policy and span model; shrink the definition-write surgery

**Status:** Draft — filed 2026-09-03, from `yqr-f026` §6
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f026` (the adoption that created the delta this spec
retires), `yqr-b026` (the span defect, fixed downstream, cause still
upstream), `yqr-b025` (the precedent: noyalib#372/#373 filed and adopted
one release later), `yqr-b020` (whose remedy the definition write keeps
working)

## 1. Scope

`yqr-f026` absorbed noyalib 0.0.29's anchor-write policy (#338) by
writing anchor definitions itself: guarded span surgery in
`src/fidelity/write/anchor.rs`, triggered by a property-led value span or
by recognizing the upstream refusal through the `materialise_aliases_of`
marker in its message. That delta works and is pinned, but three of its
parts exist only because upstream lacks something, and each has a small,
separable upstream ask. File the two issues below (§3, §4) and the PR-size
ask (§5) against `sebastienrousseau/noyalib`; when a release carries any
of them, adopt it in an `f0NN` adoption spec and delete the matching yqr
code (§6).

Filing is the yqr owner's action — issues and PRs go upstream under the
Zoosky account, signed commits required. The drafts in §3 and §4 are
ready to file verbatim; every behavioural claim in them was reproduced
through noyalib's own API on the published 0.0.31 (probe results in §2).

Upstream PRs are never release-gating; yqr ships its surgery either way.

## 2. Measured on noyalib 0.0.31

Each case runs noyalib's public API directly, no yqr involved:

| # | input | call | result |
|---|---|---|---|
| 1 | `a: &x 1\nb: *x\n` | `span_at("a")` | bytes `&x 1` — the span includes the anchor property |
| 2 | `a: &x 1\nb: *x\n` | `set_value("a", 2)` | `Err: unknown anchor: x at line 2, column 4` — the edit removed the definition, the re-parse tripped on the alias it orphaned |
| 3 | `a: &x 1\n` | `set_value("a", 2)` | `Ok`, source `a: 2\n` — the anchor is silently dropped |
| 4 | `base: &m\n  k: 1\nc:\n  <<: *m\n` | `set_value("base.k", 9)` | `Err: set_value: `base.k` is inside the value anchored by `&m` … call materialise_aliases_of("m") first` — the #338 guard |
| 5 | `a: !!str 1\n` | `span_at("a")` | bytes `!!str 1` — same shape for a tag property |

Cases 2–4 show the policy is also *inconsistent*: the guard protects an
entry inside an anchored mapping (4) but not the anchored scalar itself,
which corrupts-and-rolls-back with a misleading error (2) or silently
loses the anchor (3).

## 3. Issue draft: writes at an anchor definition

> **Title:** set_value at an anchor definition: the guard refuses the
> one edit an anchor is for, and misses the anchored scalar entirely
>
> ADR-0011 has every mutator refuse a write into a value that live
> `*name` sites share, pointing at `materialise_aliases_of`. For a write
> reached *through* an alias that is the right call — the edit landing
> at every site is surprising. But the guard also covers the anchor's
> **own definition**, where the shared edit is not surprising, it is the
> YAML meaning of an anchor: change the value once, at its source, and
> every `*name` site follows. `materialise_aliases_of` is the opposite
> of that ask — it makes N copies precisely so they stop being shared.
>
> On 0.0.31 the definition write behaves three different ways depending
> on shape (all through the public API):
>
> ```rust
> // 1. Entry inside an aliased anchored mapping: refused by the guard.
> let mut d = parse_document("base: &m\n  k: 1\nc:\n  <<: *m\n")?;
> d.set_value("base.k", &Value::from(9));
> // Err: set_value: `base.k` is inside the value anchored by `&m` …
> //      call materialise_aliases_of("m") first
>
> // 2. Anchored scalar with a live alias: not caught by the guard;
> //    the rewrite span includes the `&x` property, so the edit deletes
> //    the definition and the re-parse blames the alias it orphaned.
> let mut d = parse_document("a: &x 1\nb: *x\n")?;
> d.set_value("a", &Value::from(2));
> // Err: unknown anchor: x at line 2, column 4
>
> // 3. Anchored scalar with no alias: the anchor is silently dropped.
> let mut d = parse_document("a: &x 1\n")?;
> d.set_value("a", &Value::from(2))?;
> assert_eq!(d.source(), "a: 2\n"); // expected: "a: &x 2\n"
> ```
>
> Ask: exempt the definition from the alias-shared refusal — a write at
> the anchor's own node (or inside its value, addressed by the
> definition's path rather than through an alias) rewrites the value,
> keeps the property, and lets aliases follow, which case 1's own error
> message half-promises when it says the edit "would land at every
> `*name` site too". If the current refusal is wanted as a safety, an
> explicit opt-in (a config flag or a `set_value_at_definition`) serves
> the same end. Either way cases 2 and 3 need the property kept: the
> anchor is a property of the node, not part of its value, and the value
> is what the caller assigned.
>
> Downstream context: yqr works around all three today by splicing the
> definition itself via `replace_span` with its own re-parse guard, but
> that reimplements rendering and guard logic the mutators already have.

## 4. Issue draft: value spans include node properties

> **Title:** span_at includes `&anchor`/`!tag` properties in a scalar's
> value span, so span-based rewrites destroy them
>
> ```rust
> let d = parse_document("a: &x 1\nb: *x\n")?;
> assert_eq!(span_bytes(&d, "a"), "&x 1"); // the value is `1`
> let d = parse_document("a: !!str 1\n")?;
> assert_eq!(span_bytes(&d, "a"), "!!str 1"); // the value is `1`
> ```
>
> Measured on 0.0.31. A consumer that resolves a span to rewrite the
> value (the documented use of `span_at` + `replace_span`) rewrites the
> properties with it: the anchor definition or tag is destroyed, which
> is the mechanism behind the `set_value` failures reported separately.
> 0.0.30's location change (3e85e15, "tagged/anchored node locations
> anchor at the properties") settled where *diagnostics* point; this is
> the same question for the *value span*.
>
> Ask: either exclude leading properties from the value span, or add a
> value-only accessor (`value_span_at`?) beside the current one, so a
> span-based rewrite can preserve `&x` / `!!str` without re-lexing the
> property itself. Downstream, yqr currently re-lexes the leading
> property by hand before splicing; a value-only span deletes that code.

## 5. PR-size ask: a typed variant for the alias-shared refusal

The #338 refusal is a bare `Error::Parse(String)`. yqr recognizes it by
the `materialise_aliases_of` marker in the message — pinned by tests,
but a rewording upstream silently disables the fallback until the next
bump's test run. A dedicated variant (or a stable error code) removes
the string match for every downstream consumer. Small enough to go as a
PR with the §3 issue referencing it.

## 6. What each landing deletes in yqr

| upstream lands | yqr deletes |
|---|---|
| §3 exemption (or opt-in API) | the `materialise_aliases_of` marker match in `write.rs::set_value`; most of `assign_at_definition` — the mutator handles the definition write, quote matching included |
| §4 value-only spans | `skip_anchor_property` and `leading_tag` in `write/anchor.rs`; b026's property re-lexing; possibly the tagged-scalar refusal becomes upstream's decision |
| §5 typed variant | the string match, kept only as a fallback for older pins if any remain |

The reflection-aware guard (`changes_are_the_assignment`) stays in any
case: yqr's write tier owns proving an edit changed only what was asked,
whoever renders the bytes.

## 7. Acceptance criteria

- [ ] §3 and §4 filed upstream (issue numbers recorded here), §5 offered
      as a PR or folded into §3's issue.
- [ ] Upstream answers recorded; if the §3 exemption is declined, note
      the rationale and mark the yqr surgery as the permanent design.
- [ ] On each landing release: an `f0NN` adoption spec, the §6 deletions,
      suite green.
