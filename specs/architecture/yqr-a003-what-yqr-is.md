# Architecture a003 — What yqr is: a fidelity-first editor with a query language

**Status:** Accepted — ratified 2026-08-21; `yqr-f001` closed as Superseded
and §7's consequences carried out in the same change
**Owner:** yqr maintainers
**Last updated:** 2026-08-21
**Related:** `yqr-f001` (the roadmap this re-scopes), `yqr-a001` (the decision
`f001`'s milestones were never reconciled with), `yqr-r001` §9 (where the
reprioritization was recorded), `yqr-r003` (the field evidence, which stopped
short of this conclusion), `yqr-f008` (Draft, gated on a milestone this spec
retires)

## 1. The question

`yqr-f001` has been **In Progress** since 2026-06-21 and cannot be closed. Its
§7 lists four remaining milestones covering most of the jq language; nothing in
the repository is working towards them, and at the current rate they are years
of work.

This spec asks what yqr is, answers it from what was built, and proposes
retiring a milestone list that no longer describes the project.

## 2. The reprioritization happened, and `f001` was never reconciled with it

**This is a bookkeeping failure, not a drift.** `yqr-a001` (2026-06-26)
reordered the plan and said so, and `yqr-r001` §9 — *"Prioritization update
(a001)"* — records it plainly:

> `yqr-a001` makes **fidelity the top priority for Cohort B**. This reorders
> the near-term plan: the source-preserving **read path (slice-on-emit)** and
> the **`yqr .` byte-for-byte round-trip** property come *before* the
> construction/builtin features above.

`f001` §2 and §3 carry a001 amendments too. What never happened is the last
step: **§7's milestone list was left as written**, so the spec's plan section
still sequences M1–M4 as the near-term work while its own goals section defers
to a001. `f001` has since reported *In Progress* against the deferred half.

### 2.1 Which is confirmed by measuring both plans

Against the shipped binary, 2026-08-21.

**`f001` §7 — the jq language: 4 of 31.**

| milestone | shipped | of |
|---|---|---|
| M1 construction & literals | 0 | 4 |
| M2 builtins & arithmetic | 1 (`to_entries`) | 15 |
| M3 multi-doc & emission | 2 (multi-doc I/O byte-exact; comment preservation, as the default) | 3 |
| M4 advanced | 1 (`=`) | 9 |

**`yqr-a001`'s priorities — complete.** Every YAML-native gap `r001` §5
catalogued is closed: multi-document streams and byte-exact multi-document
output, comment preservation, anchors and aliases, tags, and output style —
the last solved by not re-emitting at all. The number model, which `r001` §5's
table listed as open, was **ratified in `a001` §6** (*"Preserve types. `Int op
Int → Int` when exact; `Float` only when genuinely fractional... Compare/sort
by mathematical value"*); `r001` §8 records it as Resolved and a001's header
says it supersedes the question.

So a001's programme finished and `f001` §7's did not. The three M3 and M4 items
that did ship arrived through **other specs** — `f006`, `f017` — reaching
milestone entries incidentally rather than by working the list.

## 3. `f001` is stale in ways that hide this

- **§1's summary sentence is stale.** It states the goal as YAML preserved "as
  faithfully as the underlying parser allows"; a001 replaced that with a
  byte-exact guarantee that does not depend on the parser. §2 and §3 were
  amended for a001 and §1 was not, which is the same reconciliation gap as §7's
  — worth noting because it means the staleness is **localised and fixable**,
  not a spec silently describing the wrong product throughout.
- **§4 and §6 describe rust-yaml** — `load_str`, `dump_all_str`,
  `rust_yaml::Value`, and an architecture diagram built on them. `yqr-m005`
  removed that dependency entirely.
- **M3 asks for "comment-preserving mode (`load_str_with_comments`)"**, an
  opt-in flag over an API yqr no longer depends on. Comment preservation is the
  default, reached another way. The milestone is satisfied and unsatisfiable at
  once.

## 4. The decision proposed

**yqr is a fidelity-first YAML editor that has a query language, not a jq clone
that happens to preserve bytes.**

The query language exists to *name the thing you want to read or change*. It
grows when a real user cannot say what they mean, and not on a schedule.

This is not new. It is `a001` §1, followed through to the one section that was
left behind:

- **`yqr-f001` closes as Superseded**, with this spec carrying the scope. M0
  stays recorded as the foundation it was.
- **M1–M4 stop being a plan.** They become a menu — a catalogue of jq
  capabilities yqr may adopt, each needing its own spec and its own reason.
  `yqr-r001` already is that catalogue and stays the reference.
- **The bar for adopting one** is `yqr-r003`'s: field evidence that someone hit
  the gap doing real work, plus a check that the feature is not gated on
  machinery yqr has declined to build. `yqr-f017` is the worked example — it
  jumped the queue on both counts, and `f017` §3 found the gate keeping it
  queued was imaginary.

### 4.1 What this is not

**Not a decision to stop adding language features.** Nine shipped in 2026 —
`=`, `+=`, `del`, `key(...)`, `line_comment`, `head_comment`, `swap`, `move`,
`to_entries` — the most recent the day before this spec. It is a decision about
what justifies the next one.

**Not a claim that jq parity is worthless.** It is a claim that parity is not a
*goal*, so a feature yqr lacks is not thereby a debt. `select` and `map` are
absent; if a user hits them the way `r003`'s hit `to_entries`, they get specs.
Until then they are not owed.

**Not a re-litigation of `r003`.** That report explicitly declined to reorder
M1 and M2 wholesale, and was right to on the evidence it had. This spec is not
more evidence about `to_entries`; it is the observation that a001's
reprioritization was recorded everywhere except the section that schedules the
work.

## 5. Consequences

- **`yqr-f008`** (`|=`, Draft) is gated on "`f001` M2". With M2 retired the gate
  is restated concretely: `|=` needs a way to compute a new value from an old
  one, so it needs arithmetic or a builtin that returns one. The **semantics
  are already settled** — `a001` §6's preserve-types rule governs any
  arithmetic yqr adds — so what `f008` waits on is an implementation decision
  about scope, not a design question.
- **`yqr-r001`** is a good catalogue and a stale plan: §3's inventory and §5's
  gap table remain useful, §7's priorities are superseded by its own §9. Worth
  re-labelling as reference material rather than leaving at Draft, where it
  reads as pending work.
- **`--slurp` and JSON output** are the two remaining M3 items with a live user
  story (interop with jq pipelines). They deserve their own spec on their own
  merits; this one does not pre-judge it.

## 6. There is no blocking design decision

An earlier draft of this spec claimed the Int/Float number model was
"genuinely blocking" and undecided, presenting it as a discovery that `r001`
§5 and `f001` §9 stated the same open question without noticing.

**That was wrong, and the correction belongs in the record.** `a001` §6
ratified it two months ago, `a001`'s header states that it supersedes `r001`
§8, and `r001` §8 is marked *Resolved* with the ruling quoted. The draft
reached the opposite conclusion by measuring the code — seeing `Int(i64)` and
`Float(f64)` still distinct, and no arithmetic — and inferring that the
question must be open, without reading the spec that answered it.

It is worth keeping this paragraph rather than deleting the section, because
the failure is the one this spec is about: **a decision recorded in one
document and not reflected in another looks exactly like an open question.**
That is `f001` §7's situation, and it is how a draft arguing the point managed
to repeat it.

## 7. Acceptance criteria

- [x] §4's decision ratified (2026-08-21).
- [x] `yqr-f001` set to Superseded; §1's summary sentence and §4/§6's rust-yaml
      references marked historical; M0 preserved as the record of the
      foundation, M1–M4 re-labelled as the menu §4 makes them.
- [x] `yqr-r001` re-labelled from Draft to Historical (reference), so it stops
      reading as pending work.
- [x] `yqr-f008`'s gate restated in terms of what it needs rather than which
      milestone it sits behind.
- [x] The feature tracker shows no feature In Progress without someone working
      on it.
