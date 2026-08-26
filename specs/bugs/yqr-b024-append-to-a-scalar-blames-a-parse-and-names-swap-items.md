# Bug b024 — Appending to a scalar reports a parse error and names an unrelated internal function

**Status:** Resolved **2026-08-26**, the day it was filed. Found while
measuring the `+=` divergence for `yqr-k002`'s jq on-ramp page — the page
states the outcome and deliberately does not quote the message, which is what
a page does when a tool's own words are the problem. See §6; §7 records the
one shape left
**Severity:** Low — a refusal, correctly refused; the defect is entirely in how
it is explained
**Component:** `src/fidelity/write.rs`, `NoyalibWriter::append` — the wording
comes from noyalib's `push_back_value`
**Related:** `yqr-b020` (an upstream message reaching the user with a false
reason, and the precedent for yqr taking one over), `yqr-b021`, `yqr-k002` §3.2

## 1. Summary

`+=` means "append one element to a sequence". Pointed at a scalar it is
rightly refused, and the refusal says two untrue things:

```console
$ printf 'a: 1\n' | yqr '.a += 1'
yqr: runtime error: cannot append to "a": YAML parse error: swap_items: `a` does not address a sequence
```

1. **"YAML parse error"** — nothing was mis-parsed. The document is valid and
   yqr read it fine; `.a` resolves and prints `1`. The category is wrong, and
   it is the category a user acts on: it sends them to look at their file
   instead of at their filter.
2. **"swap_items:"** — the name of an unrelated internal primitive. Reordering
   is not what was asked for. A user appending an item has no idea what
   `swap_items` is and no reason to.

The one true clause is the last one, `does not address a sequence`, and it is
the only part worth showing.

## 2. Why this is worth filing rather than shrugging at

It is the first thing a jq user hits. In jq, `+=` on a number is addition and
the natural spelling of "increment"; in yqr it is a sequence append
(`yqr-k002` §3.2 measures the three shapes). So `.replicas += 1` is a
predictable early mistake, and the message that meets it blames the file and
names a function from a different feature.

`yqr-b020` established that a wrong *reason* is worth a fix even when the
*refusal* is right, and that yqr may take a message over from upstream where
it can tell the case apart. This is the same shape, and the case is easy to
tell apart: the writer knows it asked to append.

## 3. Cause

`append` wraps whatever `push_back_value` returns:

```rust
// src/fidelity/write.rs
.map_err(|e| YqrError::eval(format!("cannot append to {path_str:?}: {e}")))
```

Upstream's error carries both faults — the `YAML parse error` prefix is its
Display, and `swap_items` is the internal it shares with the reorder path. yqr
adds an accurate outer clause and passes the rest through verbatim.

## 4. Fix route

Two options, and unlike `b020` the cheaper one is probably right:

- **Take the message over in yqr.** `append` already knows the operation and
  the path; the target's shape is one `span_at`/type check away. That yields
  one sentence naming what was asked, what was found, and the working
  spelling — `.a |= (. + 1)` for the increment a jq user meant.
- **Fix it upstream.** The `swap_items` label is wrong for every caller that
  is not a reorder, so noyalib would want a per-operation label regardless.
  Worth filing there too; it is not blocking, because the outer message is
  yqr's to write either way.

Proposed wording, to be measured rather than assumed:

```
yqr: runtime error: cannot append at "a": `+=` appends to a sequence, and "a"
holds a scalar. To compute a new value in place, use `.a |= (. + 1)`
```

`yqr-f025` is the precedent for the constraint on that last clause: name only
a remedy that works, and pin it with a test that runs it.

## 5. Reproduction

```console
$ printf 'a: 1\n' | yqr '.a'        # 1 -- the path resolves, the file is fine
$ printf 'a: 1\n' | yqr '.a += 1'   # exit 5, "YAML parse error: swap_items: ..."
$ printf 'a: [1]\n' | yqr '.a += 2' # exit 0 -- the operation itself is fine
```

## 6. Fix

§4's cheaper option, and the measurement widened it first: the defect is not
one message but **every** append refusal. Measured before changing anything:

| Target | Before |
|---|---|
| scalar / null / mapping | `YAML parse error: swap_items: \`a\` does not address a sequence` |
| empty sequence | `YAML parse error: push_back_value: … use \`set\` with a fragment instead` |
| flow sequence | `YAML parse error: only block sequences are supported (no \`-\` anchor …)` |

All three call a valid document a parse error. Two name an internal, and the
second recommends `set` with a fragment — an API yqr does not expose. That is
the same defect `b020` §3 recorded for `set_value`, which is guarded for
exactly this reason. **The append path never got the same guard**, and that,
rather than one bad sentence, is what this bug turned out to be.

So yqr now checks its own preconditions against the typed model before calling
the engine, in `append_item` (`src/fidelity/write.rs`). The `Append` arm
resolves through `resolve_update_target` rather than `resolve_target` — same
contract, and it hands back the current value the check needs.

Each refusal names a remedy, and the remedy differs by what is there:

```console
$ printf 'a: 1\n' | yqr '.a += 2'
yqr: runtime error: cannot append at "a": `+=` appends an item to a sequence,
and this is a number. Use `|=` to compute a new value in place, as in `.a |= (. + 1)`

$ printf 'a: x\n' | yqr '.a += 2'      # a string names `|=` with no arithmetic
$ printf 'a:\n'   | yqr '.a += 2'      # null names `=`, which b021 made work
$ printf 'a:\n  k: 1\n' | yqr '.a += 2'  # a mapping grows by assignment
```

**Only the numeric arm carries `(. + 1)`**, because only there does it work:
`(. + 1)` over a string is a type error, and `yqr-f025`'s rule is that a
message names a remedy that works. `every_remedy_the_append_refusal_names_actually_works`
(`tests/cli.rs`) runs all four rather than asserting they were mentioned.

Four tests: the refusal wording across five shapes, the remedies executed, the
empty-sequence message with its `set`/fragment leak gone, and the control that
a block sequence still appends with every other byte intact.

## 7. What is left: the flow sequence

A non-empty **flow** sequence (`a: [1, 2]`) still falls through to the engine
and still wears `YAML parse error`. yqr's typed model cannot see the
difference — a flow sequence and a block sequence are both `Value::Sequence` —
so the precondition check cannot catch it, and detecting flow layout would
mean new CST work rather than a guard.

Its content is at least accurate and names no internal, which is why it was
left rather than papered over. Two things would close it: a flow check on the
CST, or a category fix upstream so a refusal over a well-formed document stops
calling itself a parse error. The second is the better one, and it is not
yqr's to make.