# Feature f019 — Adopt noyalib 0.0.25: four bugs closed, and the delegation question answered

**Status:** Done — 0.0.25 adopted, all four open engine bugs verified fixed
against the published crate, the `f018` §5 revisit run and decided (2026-08-20)
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f018` (the 0.0.24 adoption this succeeds, and whose §5
"revisit when `b014` is fixed upstream" this discharges), `yqr-b011`,
`yqr-b012`, `yqr-b013`, `yqr-b014` (the four bugs this release closes),
`yqr-b015` (what verifying `b011` uncovered underneath it), `yqr-f007` §6 (the
standing delegation argument), `yqr-m003` (the corpus write tier that pinned
`b012`/`b013` as they behaved)

## 1. Scope

Bump `noyalib = "0.0.24"` to `0.0.25` and settle what the release changes.

Unlike every previous pin bump, this one is **not** about a single fix. All
four of yqr's open engine bugs were filed upstream on 2026-08-19 and all four
are in this release (§2), so the work is verification rather than adoption:
confirm each fix against the published crate per bug, flip the corpus cases
that were pinned as-they-behaved, and re-run the one decision `f018` deferred
to this moment.

**In scope:** the pin bump; per-bug verification against the published crate
(§3); the `f018` §5 sole-entry delegation revisit (§4); the corpus and CLI
regression tests that replace the pins (§5); closing `b011`, `b012`, `b013`,
`b014`.

**Out of scope:** the `yqr-f007` §6 addressing limit. `b012`'s *insert* face is
fixed, but `set`, `del`, `key(...)` and the reorder verbs still route a key
through `parse_query_path`, so writing an **existing** dotted key stays refused
— with an accurate diagnostic, which is the part that changed. That limit is
`f007` §6's, not this release's, and §6 there governs when it is reopened.

## 2. What 0.0.25 contains

Published to crates.io 2026-08-20T09:12:33Z. Upstream landed it as
[noyalib#287](https://github.com/sebastienrousseau/noyalib/pull/287) —
*"release: v0.0.25 — four fixes from @zoosky (#283, #285, #288, #290)"* —
merged to `main`, with yqr's three commits cherry-picked with authorship
intact and the fourth written by the maintainer:

| Upstream | yqr bug | Author | What changed |
|---|---|---|---|
| #284 → #283 | `b014` §3.1 | yqr | a sole-entry replacement is indented past its key |
| #286 → #285 | `b011` | yqr | a wrapped flow collection may close at the parent's column |
| #289 → #288 | `b012` | yqr | the insert anchor comes from the span tree, not a re-parsed path string |
| #290 | `b013` | maintainer | the quote vote is scored at the edit site |

The three PRs read `closed`/`merged=false` on GitHub because the release
branch carried the commits rather than the merge button; the maintainer said
so on each (*"Merged in #287 as part of the v0.0.25 release — your commit is
on `main` with authorship intact"*), and `#287`'s commit list confirms it.
That distinction matters only for how the state is read, not for what shipped;
§3 verifies the behaviour rather than the provenance.

`b013` is the one yqr filed **without** a patch, on the argument that the
dominance heuristic has a public API attached and what it counts is the
maintainer's call (`b013` §4.1). The maintainer took the second of the two
options the filing offered — score at the edit site — rather than the first.

## 3. Verification, run 2026-08-20 against the published crate

Per bug, on the reproduction each spec states, not on the release notes.

### 3.1 `b011` — a wrapped flow collection reads

```console
$ printf 'ports: [\n  80,\n  443,\n]\n' | yqr '.' | diff - ports.yaml
$ echo $?
0
```

Byte-identical, and the closer at the parent's column is accepted for the
mapping form (`cfg: {` / `  x: 1,` / `}`) and when nested. The narrowing holds:
under-indented flow **content** is still refused, which is what makes this a
spec-conformant fix rather than a relaxation —

```console
$ printf 'ports: [\n80,\n]\n' | yqr '.'
yqr: io error: failed to parse YAML input: YAML parse error: flow content must
be indented more than the surrounding block
```

Reading these files also brings the write path to them for the first time.
`set` is byte-exact at the site; `del` is not, and §3.5 records what that
turned up.

### 3.2 `b012` — a key inserts beside a dotted one

```console
$ yqr '.labels.tier = "web"' labels.yaml
labels:
  app.kubernetes.io/name: web
  app.kubernetes.io/component: frontend
  tier: web
```

The misleading `<<` merge diagnostic is gone with the refusal that produced
it. Writing an existing dotted key is still refused, and now says what is
actually wrong: *"cannot address key `app.kubernetes.io/name`: it uses
characters the write path cannot express"* — `f007` §6's limit, stated as
itself.

### 3.3 `b013` — an inserted scalar is spelled like its neighbours

```console
$ printf 'quoted: "30"\nlabels:\n  app: web\n' | yqr '.labels.tier = "web"'
quoted: "30"
labels:
  app: web
  tier: web
```

The line four rows up no longer decides the edit's spelling. The behaviour the
vote exists for survives: a quoted *neighbour* still carries, so
`labels:` / `  app: "web"` gains `  tier: "web"`. That asymmetry is the point
of the fix and is pinned as its own assertion, so a future "simplification" to
plain-always fails a test.

### 3.4 `b014` §3.1 — the sole-entry writer

Not reachable from yqr (§4 is why), so this one is measured by calling
`Document::remove` directly:

| input | 0.0.24 | 0.0.25 | yqr's `delete_entry` |
|---|---|---|---|
| `on:` / `- push` / `jobs: {}` | `on:` / `[]` / `jobs: {}` | `on:` / `␣␣[]` / `jobs: {}` | same |
| `on:` / `- push` | `on:` / `[]` | `on:` / `␣␣[]` | same |

Both parsers that rejected the old output accept the new one:

```console
$ python3 -c 'import yaml; print(yaml.safe_load("on:\n  []\njobs: {}\n"))'
{True: [], 'jobs': {}}
$ ruby -ryaml -e 'p YAML.safe_load("on:\n  []\njobs: {}\n")'
{true=>[], "jobs"=>{}}
```

Checked with a leading BOM, with CRLF, with a head comment above the entry,
and nested one level — all indent past the key. The BOM case is the one the
patch was force-pushed for: measuring the key's column as a byte distance from
the line start counted the BOM's three bytes, which is the mistake noyalib#123
fixed once already in the scanner.

The other face, `b014` §3.2 — noyalib's parser *accepting* the shape — is
unchanged and still yqr's to catch. `validate` reports `Y103` in default mode
and does so on this release too, which is the check the bug's own §3.2 fix
installed.

### 3.5 What verifying `b011` uncovered — filed as `yqr-b015`

`del` on a wrapped flow collection leaves the removed item's indentation
behind as a whitespace-only line:

```console
$ printf 'ports: [\n  80,\n  443,\n]\n' | yqr 'del(.ports[0])' | sed -n l
ports: [$
  $
  443,$
]$
```

Upstream's, not yqr's — the flow class is delegated to `Document::remove`
(`f016` §5), and calling it directly produces the same bytes. It loads back
correctly, so this is cosmetic; it was invisible until now for the plainest
possible reason, that the file could not be parsed. Filed as `yqr-b015`, and upstream the same day as noyalib#294 with a fix in noyalib#296.

That is the second time in two releases that fixing a refusal exposed a
defect behind it, and it is worth naming as a pattern rather than a
coincidence: a parse refusal hides every downstream bug for that shape, so a
read fix should be followed by walking the write verbs over the shape it
unblocked, not just the read.

## 4. The `f018` §5 revisit — the sole-entry delegation

`f018` §5 committed to re-running this the moment `b014` was fixed upstream,
because at that point *"the sole-entry class has no measured divergence left,
and the question becomes the general one `f007` §6 already answers on its own
terms"*.

**Method**, identical to `f018` §4 so the two are comparable: the sole-entry
branch of `delete_entry` routed to `Document::remove` on a throwaway patch,
whole suite run, patch reverted. The tree as committed contains no trace of it.

**Result: 382 tests, 0 failures.** The two `f018` §4.1 failures — both the
same-column `on:` / `- push` shape — are gone, and nothing else moved. The
head-comment divergence `f016` §5.2 kept the class for was retired by 0.0.24;
this release retires the last one. **Upstream and `delete_entry` now agree on
every case yqr tests.**

### 4.1 The decision: it stays in `delete_entry` anyway

For the first time this is decided on the standing argument alone, with no
case-specific reason left, so the argument is worth restating rather than
cited:

- **The divergences were found *by* the independent implementation.** Four
  now (`b006`, `b010`, `b014`, and `b004` §6's trivia set), and in each case
  the thing that made the defect visible was having two implementations of
  the same operation to disagree. Deleting one ends that, and it ends it
  precisely when the disagreements have stopped — which is the moment it
  looks safest and is worth the least.
- **The trade is asymmetric.** Keeping `delete_entry` costs a module yqr
  already has, already tests, and does not plan to grow. Delegating saves
  that and risks writing a user's file wrongly at exit 0, which is the
  failure mode `a001` exists to prevent and the one `b006`/`b010`/`b014` all
  were.
- **`b015` is the live proof, in this same release.** The flow class *is*
  delegated, and this is the second upstream defect to reach yqr's output
  through it. Whatever else it argues, it argues against widening delegation
  in the same week.

**Revisit when** the trade changes — `delete_entry` needing real work to keep
up, or upstream growing a differential oracle of its own. Not on a further
release of agreement, which is what this measurement now supplies and which
`f007` §6 already declines to reopen on.

## 5. Tests

The corpus write tier pinned `b012` and `b013` **as they behaved**, with the
comment saying what each would become when fixed (`m003`'s rule: pin the bug,
do not hide it). Both were flipped, and the ids renamed to describe behaviour
rather than a refusal:

| case | was | now |
|---|---|---|
| `write/insert/new-key-under-a-nested-mapping` | `tier: "web"` | `tier: web` |
| `write/insert/refuses-a-mapping-whose-keys-hold-a-dot` | `Err(5)` | `write/insert/mapping-whose-keys-hold-a-dot`, `Rewrites` |
| `write/append/sequence-item-at-the-site-indent` | `- "billing"` | `- billing` |

New coverage, since three of the four bugs had no yqr-side test at all (they
were pinned upstream or unreachable):

- `tests/fidelity.rs` gains a **`wrapped-flow`** case. The harness is one case
  per `b001` formatting dimension and had none for a wrapped flow, because
  until now the engine could not read one.
- `tests/cli.rs` gains five: the wrapped-flow read (byte-for-byte) and edit,
  the still-refused under-indented content, the dotted-key insert, and the
  quote-style triple — plain document, quoted line elsewhere, quoted
  neighbour.

## 6. Acceptance criteria

- [x] 0.0.25 published to crates.io; the pin moves and `Cargo.lock` shows
      noyalib moving and nothing else.
- [x] Each of the four fixes verified against the **published** crate on the
      reproduction its bug states (§3), not from the release notes.
- [x] `b014` §3.1 measured directly against `Document::remove`, including the
      BOM, CRLF, head-comment and nested variants, and the output checked
      against PyYAML and Psych (§3.4).
- [x] The `f018` §5 revisit run by `f018` §4's method, and decided with the
      reason stated (§4).
- [x] The `m003` write-tier pins flipped, and regression tests added for the
      three bugs that had none (§5).
- [x] What §3.5 found filed as `yqr-b015` rather than left in this spec, and
      taken upstream as noyalib#294 / noyalib#296.
- [x] `b011`, `b012`, `b013`, `b014` moved to Resolved with the release
      recorded; `yqr-b000` summary updated.
- [x] Full suite green on the new pin with yqr's own code unchanged;
      `local-ci.sh` clean.
