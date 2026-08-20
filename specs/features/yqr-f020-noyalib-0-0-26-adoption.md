# Feature f020 — Adopt noyalib 0.0.26: the wrapped-flow delete, and the one bug it does not carry

**Status:** Done — 0.0.26 adopted and `b015` verified against the published
crate; `b016` stays open, deliberately (2026-08-20)
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f019` (the 0.0.25 adoption this succeeds, and the release
whose verification found `b015`), `yqr-b015` (the bug this release closes),
`yqr-b016` (the bug it does **not**), `yqr-b011` (the parse refusal that hid
`b015`), `yqr-f016` §5 (the delegation that makes the flow class upstream's)

## 1. Scope

Bump `noyalib = "0.0.25"` to `0.0.26` and verify what it carries.

The short version, and the reason this spec is short: **0.0.26 carries one
functional change**, yqr's own noyalib#296, which fixes `b015`. That is the
whole of it — noyalib#299 added tests and docs for the same fix, and
noyalib#300 cut the release.

**In scope:** the pin bump; verifying `b015` against the published crate (§3);
the yqr-side regression test `b015` §5 deliberately deferred until a fix
existed (§4); closing `b015`.

**Out of scope, and worth naming because the obvious assumption is the
opposite:** `b016`. It is filed (noyalib#297) and fixed (noyalib#298, green),
but **#298 is not merged and is not in this release**. Two open bugs, one
release, one of them closed — the pin does not close both, and §5 says what
still holds.

## 2. What 0.0.26 contains

Published to crates.io 2026-08-20T20:20:05Z. Upstream's release commit is
`7e36e1a` — *"chore(release): v0.0.26 — @zoosky's wrapped-flow fix"*.

| Upstream | yqr bug | What changed |
|---|---|---|
| #296 → #294 | `b015` | a flow member alone on its line takes the line with it |
| #299 | — | tests and docs for the above |
| #300 | — | the release |

Compare with `f019`, where the release carried four fixes and the work was
verification across four reproductions. Here there is one, so most of this
feature is the regression test that was waiting on it.

## 3. Verification, run 2026-08-20 against the published crate

On the reproduction `b015` §1 states, not on the release notes.

```console
$ printf 'ports: [\n  80,\n  443,\n]\n' | yqr 'del(.ports[0])' | sed -n l
ports: [$
  443,$
]$
```

The whitespace-only line is gone. The last-member and flow-mapping forms
behave as the fix specified:

| filter | result |
|---|---|
| `del(.ports[1])` on the wrapped list | `ports: [` / `␣␣80,` / `]` |
| `del(.cfg.a)` on the wrapped mapping | `cfg: {` / `␣␣b: 2,` / `}` |

The last one keeps the comma on the line above, which is the deliberate
narrowing: a trailing comma before `]` is valid, both PyYAML and Psych read it
back, and reaching up a line to delete a separator the removal does not own
would be reformatting rather than removing. All three outputs were loaded back
under PyYAML and Psych.

### 3.1 The controls held

The fix's condition is "the member is alone on its line", not "the collection
is wrapped", and every shape where something else survives on the line is
byte-identical to 0.0.25:

- single-line `ports: [80, 443]` — untouched
- the opening indicator on the line — kept
- the closing indicator on the line — kept
- a comment on the line — kept, comment and all

That last one is the shape the fix deliberately did not decide (an orphaned
comment is a `b010`-class semantics question), and it is pinned so a later
change has to argue with a test rather than with prose.

## 4. The regression test `b015` §5 was waiting for

`b015` §5 declined to pin the delete half at filing time, on the grounds that
pinning a whitespace-only line as expected output invites a future reader to
preserve it. With the fix released, the pin states the right thing, so
`tests/cli.rs` gains both halves:

- `deleting_from_a_wrapped_flow_collection_takes_the_whole_line` — the three
  shapes, each asserting the exact bytes *and* that no line carries trailing
  whitespace.
- `a_flow_delete_leaves_a_line_it_does_not_own_standing` — the four controls.

The second is the more valuable of the two. The first would pass on a fix that
stripped whitespace indiscriminately; only the controls distinguish the rule
that was actually implemented from the one that would have been easier.

## 5. `b016` stays open, and the pin that records it stays

`tests/cli.rs::to_entries_output_carries_the_emitters_trailing_space` still
passes, because the emitter defect is still there. That is not an oversight in
this adoption — it is the pin doing its job across a release that did not fix
it, and the `yqr-m003` rule working as intended: a bug pinned as it behaves
tells you, on every future bump, whether the bump changed it.

The guide's note about the trailing space on a `to_entries` pair stays too.
It goes when a release carries noyalib#298, not when the PR turns green.

## 6. Acceptance criteria

- [x] 0.0.26 published to crates.io; the pin moves and `Cargo.lock` shows
      noyalib moving and nothing else.
- [x] `b015` verified against the **published** crate on its own reproduction,
      including the last-member and flow-mapping forms, with the outputs loaded
      back under PyYAML and Psych (§3).
- [x] The four controls checked byte-identical to 0.0.25 (§3.1).
- [x] The regression test `b015` §5 deferred is added, controls included (§4).
- [x] `b016`'s pin and the guide's note **left in place**, with §5 stating why
      rather than leaving it to read as an oversight.
- [x] `b015` moved to Resolved with the release recorded; `yqr-b000` updated.
- [x] Full suite green on the new pin with yqr's own code unchanged;
      `local-ci.sh` clean.
