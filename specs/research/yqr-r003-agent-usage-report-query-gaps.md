# Research r003 — First external agent usage report: where yqr sends people back to a script

**Status:** Accepted (the report is recorded and its claims re-measured; the
one actionable item is scoped as `yqr-f017`)
**Owner:** yqr maintainers
**Last updated:** 2026-08-17
**Measured against:** yqr **0.5.1** (the released binary the session used) and
`yqr-f007` §7's `key(...)`, which landed after it
**Related:** `yqr-r001` (the jq feature-gap table this report is field evidence
for), `yqr-f001` (the M1/M2 roadmap that owns most of the gaps),
`yqr-f017` (the one gap this report promotes), `yqr-a002` (the addressing
grammar whose slice 1 closes half of the headline complaint)

## 1. Why this is worth a spec

`yqr-r001` catalogues yqr's distance from jq feature by feature, from the
outside in. This is the first record of someone **hitting** that distance while
doing real work, and it is a different kind of evidence: it says which gap was
reached first, what the user did next, and how far they got before giving up.

The session was an agent (Copilot, Opus-class model) asked to extract a value
from every entry of a Helm values file and report it alongside the entry it
belonged to. It reached for yqr, hit a wall, and **fell back to a Python
script**. The report it wrote afterwards is the primary source for §3.

**Source data is not reproduced here.** The file lived in a private repository;
every transcript below was re-measured against a synthetic fixture of the same
*shape*, and that fixture is what the spec quotes.

```yaml
services:
  alpha:
    domain: "alpha.example.com"
    tier: edge
  beta:
    domain: "beta.example.com"
    tier: core
  gamma:            # deliberately missing `domain` — see §5
    tier: core
```

The real file was that shape at a couple of dozen entries, inside a deeper
tree, with anchors, aliases and a merge key.

## 2. The task, and how far yqr got

**Task:** for each named entry, output the entry's name and its `domain`.

The value half is a one-liner and always was:

```console
$ yqr -r '.services[].domain'
alpha.example.com
beta.example.com
null
```

The name half is what failed. `.services[]` iterates the mapping's *values*, so
the keys are gone by the time the filter can act on them. That is the whole
report in one sentence: **yqr could produce the data but not say what it was
about.**

## 3. The report's claims, re-measured

All five reproduce exactly as reported on 0.5.1. Nothing below is disputed.

| # | Claim | Measured on 0.5.1 |
|---|---|---|
| 1 | no `keys` / `to_entries` | `keys` -> `parse error: expected Dot but found Ident("keys")`; same for `to_entries` |
| 2 | no string `+` | `.a + .b` -> `lex error: unexpected character '+' (did you mean '+=' ?)` |
| 3 | no `select` / `map` / `\|=` | `select` and `map` fail as unknown identifiers; `\|=` reports its own "not yet supported" |
| 4 | filter is mandatory, jq-subset only | correct |
| 5 | `-i` needs a mutating filter and a real file | correct, and deliberate (`yqr-f006`) |

Claim 2's error message is worth noting as a small success: the lexer's
`did you mean '+='?` hint is what let the reporter conclude, correctly, that
`+` exists only as a compound-assignment operator rather than being unlexable.
A generic "unexpected character" would have left that ambiguous.

## 4. Claim 1 is already half wrong, one day later

`yqr-a002` slice 1 shipped `key(...)` (`yqr-f007` §7) after the session ran. It
was designed for renaming a key, and it answers `keys` as a side effect,
because the selector wraps an ordinary path and therefore iterates like one:

```console
$ yqr -r 'key(.services[])'
alpha
beta
gamma
```

That is the enumeration the report calls impossible. Paired with the value
half, the task no longer needs a script:

```console
$ paste <(yqr -r 'key(.services[])' f.yaml) <(yqr -r '.services[].domain' f.yaml)
alpha   alpha.example.com
beta    beta.example.com
gamma   null
```

**What it does not give is `to_entries`.** Two aligned streams are not pairs:
nothing downstream of yqr can filter, reshape, or build a string from them
without leaving yqr. The reporter's own framing — *"`.services[].domain` gives
you values but throws away which entry they belong to"* — is now half-answered,
and the remaining half is the one §6 promotes.

## 5. The alignment is a property, not a coincidence

The `paste` idiom above is only trustworthy if the two streams cannot drift out
of step. They cannot, and the fixture's `gamma` is why it is in the fixture: a
missing field yields `null` rather than being skipped, so both streams emit
exactly one line per entry.

```text
keys:    alpha  beta  gamma
domains: alpha.example.com  beta.example.com  null
```

This follows from `.a` on a mapping without `a` yielding `null` (jq's rule,
`yqr-f001` M0), and it is worth stating because the obvious alternative — a
filter that silently skipped keyless entries — would make the idiom produce
*plausible, wrongly-paired output*, which is the failure class yqr exists to
refuse. Recorded as a property to keep, not an accident to rely on.

## 6. What this changes, and what it does not

**Most of the report is the roadmap working as designed.** `r001` already
classes `keys`/`select`/`map` as M2 builtins and string interpolation as M1
construction; `f001` sequences them. Nothing here argues for reordering M1 and
M2 wholesale, and the report's own conclusion — that yqr is for surgical
format-preserving edits and jq/yq are for querying — is the conclusion yqr's
own documentation reaches.

**One item is promoted out of that queue: `to_entries`.** Three reasons, none
of which apply to the rest of the M2 builtin core:

1. **It is the shape that actually defeated a real user**, and it defeated them
   on the single most common YAML layout there is — a mapping of named things.
2. **The path plumbing already exists.** `key(...)` proves the evaluator can
   recover an entry's key at read time; `to_entries` needs no new addressing
   work, only a value to hand back.
3. **It is not gated on M1 construction**, which is the assumption that would
   have kept it queued. jq's `to_entries` returns objects, and yqr has no
   object-construction *syntax* — but a builtin constructs a `Value` in Rust,
   and `render` already emits any `Value` including mappings. The gate is
   imaginary. See `yqr-f017` §3.

Scoped as **`yqr-f017`**.

**Explicitly not promoted:** `select`, `map`, string concatenation and
interpolation. Each is genuinely M1/M2, each is a bigger language commitment
than one builtin, and the report reached them only *after* `to_entries` had
already sent it to Python — so they are evidence of where the wall is, not of
what to build next.

## 7. The compliment is load-bearing too

The report's positive half was not measured by its author, so it was measured
here, against the same private file:

- `yqr '.'` on the real file is **byte-identical** to the input — anchors,
  aliases, merge keys, comment blocks and trailing comments all intact.
- A targeted edit changed exactly one line of the file and nothing else.

Both held. That is the property `yqr-a001` exists for, confirmed on a real
document yqr had never seen, which is worth recording precisely because the
rest of this spec is about gaps.

## 8. Provenance

- The agent session and its written report, 2026-08-17. Not reproduced;
  §3 restates its claims and §1 describes the input's shape.
- yqr **0.5.1** (`cargo install`ed release) for every §3 transcript, re-run
  against the synthetic fixture on 2026-08-17.
- `yqr-f007` §7's `key(...)` for §4, from the feature branch, since it is not
  in 0.5.1.
