# Architecture a003 — What yqr is: a fidelity-first editor with a query language

**Status:** Proposed — the decision in §4 needs ratifying before `yqr-f001`
can be closed
**Owner:** yqr maintainers
**Last updated:** 2026-08-21
**Related:** `yqr-f001` (the roadmap this re-scopes), `yqr-a001` (the fidelity
guarantee that changed the premise), `yqr-r001` §5 (the roadmap that actually
got executed), `yqr-r003` (the field evidence, which stopped short of this
conclusion), `yqr-f008` (Draft, gated on a milestone this spec retires)

## 1. The question

`yqr-f001` has been **In Progress** since 2026-06-21 and cannot be closed. Its
§7 lists four remaining milestones covering most of the jq language; at the
current rate they are years of work, and nothing in the repository is working
towards them.

That is usually a sign the plan is wrong rather than the work is slow. This
spec asks what yqr is, answers it from what was built rather than from what
was planned, and proposes retiring the milestone list that no longer describes
either.

## 2. Two roadmaps, measured

`yqr-f001` §7 (M1–M4) and `yqr-r001` §5 were written five days apart. Both are
lists of what yqr should gain. Only one was built.

**`f001` §7 — the jq language.** Measured against the shipped binary
2026-08-21:

| milestone | shipped | of |
|---|---|---|
| M1 construction & literals | 0 | 4 |
| M2 builtins & arithmetic | 1 (`to_entries`) | 15 |
| M3 multi-doc & emission | multi-doc read | 3 |
| M4 advanced | `=` | 7 |

Two of roughly twenty-nine, and both arrived through other specs — `to_entries`
as `yqr-f017`, `=` as `yqr-f006`.

**`r001` §5 — the YAML-native gaps.** The same measurement:

| gap | state |
|---|---|
| Multi-document streams | solved |
| Multi-document output | solved, byte-exact |
| Comment preservation | solved, and it is the default |
| Anchors / aliases round-trip | solved |
| Tag handling | solved |
| Output style control | solved, by not re-emitting at all |
| Int/Float vs jq's number model | **open** — undecided because there is no arithmetic |

Six of seven.

`r001` §5 called these "arguably higher-leverage than chasing jq's long tail,
because they are the reason to pick `yqr` over `jq | yq`". That is exactly what
happened. **The roadmap that got executed is `r001` §5's, and nobody wrote that
down** — so `f001` has spent two months reporting progress against a list the
project stopped using.

## 3. `f001` is stale in ways that hide this

Beyond the milestones, three parts of `f001` describe a product that no longer
exists:

- **§1's premise is now false.** It states the goal as YAML preserved "as
  faithfully as the underlying parser allows". `yqr-a001` and `yqr-f009`
  replaced that with a byte-exact guarantee that does not depend on the parser
  at all. The product's central claim changed after the spec was written.
- **§4 and §6 describe rust-yaml** — `load_str`, `dump_all_str`,
  `rust_yaml::Value`, and an architecture diagram built on them. `yqr-m005`
  removed that dependency entirely.
- **M3 asks for "comment-preserving mode (`load_str_with_comments`)"**, an
  opt-in flag over an API yqr no longer depends on. Comment preservation is
  the default, reached by a different route. The milestone is satisfied and
  unsatisfiable at the same time.

## 4. The decision proposed

**yqr is a fidelity-first YAML editor that has a query language, not a
jq clone that happens to preserve bytes.**

The query language exists to *name the thing you want to read or change*. It
grows when a real user cannot say what they mean, and not on a schedule.

Concretely:

- **`yqr-f001` closes as Superseded**, with this spec carrying the scope. M0 is
  done and stays recorded as the foundation it was.
- **M1–M4 stop being a plan.** They become a menu — a catalogue of jq
  capabilities yqr may adopt, each needing its own spec and its own reason.
  `yqr-r001` already is that catalogue and stays the reference.
- **The bar for adopting one** is `yqr-r003`'s: field evidence that someone hit
  the gap doing real work, plus a check that the feature is not gated on
  machinery yqr has declined to build. `yqr-f017` is the worked example — it
  jumped the queue on both counts, and `f017` §3 found the gate keeping it
  queued was imaginary.

### 4.1 What this is not

**Not a decision to stop adding language features.** Four shipped this year;
`to_entries` shipped last week. It is a decision about what justifies the next
one.

**Not a claim that jq parity is worthless.** It is a claim that parity is not a
*goal*, so a feature yqr lacks is not thereby a debt. `select` and `map` are
absent today; if a user hits them the way `r003`'s hit `to_entries`, they get
specs. Until then they are not owed.

**Not a re-litigation of `r003`.** That report explicitly declined to reorder
M1 and M2 wholesale, and it was right to on the evidence it had — one session,
one gap. This spec is not more evidence about `to_entries`; it is the
observation that two months later the *milestones* have not moved while the
product has.

## 5. Consequences

- **`yqr-f008`** (`|=` computed updates, Draft) is gated on "`f001` M2". With
  M2 retired the gate becomes concrete rather than positional: `|=` needs a way
  to compute a new value from an old one, which needs arithmetic or builtins,
  which needs §6's decision. `f008` stays Draft and its gate is restated.
- **`yqr-r001`** moves from Draft to Reference. It is a good catalogue and a
  bad plan; saying so is what lets it stay useful.
- **The `--slurp` and `--json` items in M3** are the two remaining pieces of
  `f001` with a live user story (interop with jq pipelines). They are worth
  their own spec on their own merits, and this spec does not pre-judge it.

## 6. The one design decision that is genuinely blocking

`r001` §5's seventh gap and `f001` §9's open question are the same thing:
**does yqr adopt jq's single number type, or keep YAML's Int/Float
distinction?**

It is undecided because nothing has needed it — there is no arithmetic. But it
gates arithmetic, comparisons, `add`, `min`/`max`, `sort`, and therefore
`f008`. `src/value.rs` keeps `Int(i64)` and `Float(f64)` distinct today, which
is the right default for a tool whose product claim is that `0640` stays
`0640` — jq's single `f64` would make that claim unkeepable for large integers.

Recording it here so that whoever reaches for arithmetic finds the decision
already framed, rather than discovering it half-way through an implementation.

## 7. Acceptance criteria

- [ ] §4's decision ratified or rejected. If rejected, `f001` needs a schedule
      and an owner rather than a status.
- [ ] `yqr-f001` set to Superseded, its §1/§4/§6 staleness noted rather than
      silently left, and M0 preserved as the record of the foundation.
- [ ] `yqr-r001` re-labelled as a reference catalogue, not a roadmap.
- [ ] `yqr-f008`'s gate restated in terms of what it needs rather than which
      milestone it sits behind.
- [ ] The feature tracker shows no feature In Progress without someone
      working on it.
