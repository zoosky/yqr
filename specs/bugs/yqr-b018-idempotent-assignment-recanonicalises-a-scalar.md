# Bug b018 — `.n = .n` rewrites `0640` as `640`: an assignment that changes nothing still re-spells the scalar


> **Historical: resolved.** yqr no longer behaves as described below. The
> **Status** line records what fixed it and when; the rest is kept as the
> reproduction and the reasoning, written in the present tense of the time it
> was filed.

**Status:** Resolved — 2026-08-22. Found by `yqr-f008`'s first acceptance
test, which fixed the `|=` half; the `=` half is fixed here, and the rule now
lives in **one** place both operators reach
**Severity:** Medium — silent, at exit 0, on the example the product's own
documentation uses to explain why yqr exists
**Component:** `src/fidelity/write.rs`, the `Mutation::Assign` /
`AssignTarget::Existing` arm
**Related:** `yqr-a001` §1 (the guarantee this breaks), `yqr-f006` (the write
tier this is in), `yqr-f008` (which hit it and guards its own path),
`yqr-b001` (the class: a round trip through the typed model losing spelling)

## 1. Summary

Assigning a value equal to the one already there rewrites the scalar in
canonical form:

```console
$ printf 'n: 0640\n' | yqr '.n = .n'
n: 640
```

`0640` and `640` are the same `Int` in the typed model, so `set_value`
re-emits the value it was given and the leading zero is gone. Exit 0, no
warning.

## 2. Why this one matters more than its severity suggests

`yqr-a001` §1 states the guarantee as:

> Comments, key ordering, and invisible characters … present in the input are
> preserved in the output. **yqr never rewrites bytes it did not change.**

An assignment whose value is already the value in the file changes nothing, so
this is that sentence's own counter-example.

And the scalar is not chosen at random. `0640` is the case the fidelity guide
leads with — *"Kubernetes spells file permissions in octal. If a tool re-types
that scalar as a number and prints it back, you get `640`, which is a different
permission."* The guide is right about reads. This is the write path doing
what the guide says other tools do.

It reproduces wherever the **typed model cannot carry the spelling**, which is
numbers. `1.10` — a version pin — becomes `1.1`, which is the guide's other
headline example:

```console
$ printf 'v: 1.10\n' | yqr '.v = .v'
v: 1.1
```

It does **not** reproduce for a quoted string: `q: "30"` survives `.q = .q`,
because the value carries its own content and `set_value` matches the
neighbouring quote style. Worth stating, because the obvious framing —
"assignment re-canonicalises scalars" — is broader than what was measured.

## 3. Scope: `=` only

`|=` had the identical hazard and is **fixed** in `yqr-f008`: when the
computed value equals the current one, the write is skipped, so `.n |= .` is
byte-exact by construction. That fix is one comparison, and it is in `f008`
because `f008`'s first acceptance criterion demanded it.

`=` is not fixed, deliberately. The two arms differ in what they have in hand:
`|=` resolves the target *and its current value* (`resolve_update_target`),
while `=` resolves only an `AssignTarget`, which for the create-a-key case has
no current value to compare against. Widening the fix means giving the `=` arm
the same lookup, which is a change to a shipped operator's write path and
belongs in its own change rather than smuggled into a feature about `|=`.

## 4. Fix route

Skip the write when the resolved value equals the value already at the target,
in the `AssignTarget::Existing` arm. `AssignTarget::NewKey` is unaffected —
there is nothing there to be equal to.

Two things to get right, neither obvious:

- **Equality is the typed model's**, so `0640 == 640` is true and the write is
  correctly skipped. That is the desired outcome here, and it is also why the
  bug exists: the model cannot tell the two spellings apart. The guard works
  *because* of that, not despite it.
- **A skip must stay a success.** `.n = .n` is a no-op, not a refusal; exit 0
  with the document unchanged, matching the absent-path rule.

## 5. What this is not

**Not an argument for comparing bytes instead of values.** yqr cannot write
`0640` back as `0640` by re-emitting it — the spelling is not in the typed
model. The fix is to not write at all, which is why it is a guard rather than
an emitter change.

**Not `yqr-b001`.** That was the read path re-serializing everything, fixed by
making fidelity the default. This is the write path re-emitting one scalar it
was asked to write. Same class, different half of the tool, and the fix here is
much smaller.

## 6. Reproduction

```console
$ printf 'n: 0640\n' | yqr '.n = .n'      # n: 640     -- wrong
$ printf 'n: 0640\n' | yqr '.n |= .'      # n: 0640    -- fixed by f008
$ printf 'v: 1.10\n' | yqr '.v = .v'      # v: 1.1     -- same class
$ printf 'q: "30"\n' | yqr '.q = .q'      # q: "30"    -- strings unaffected
```

Pinned with the fix rather than before it, on `yqr-b015` §5's call: a test
asserting `640` would have read as the intended behaviour. Six hold it: three
CLI tests, one unit test, and two corpus cases (one per operator).

The CLI test walks five spellings, and **three** of them are the pin —
`0640` → `640`, `1.10` → `1.1`, and `1.0` → `1` all reproduce with the guard
removed. The other two are controls, not coverage: a quoted `"30"` and
`i64::MAX` survive either way, and they are in the table to record §2's point
that the bug is narrower than "assignment re-canonicalises scalars".

The corpus cases needed a document to hold a spelling the typed model cannot
carry, which none had; `APP_CONFIG` gained `file_mode: 0640` for it. The first
attempt targeted an already-canonical `containerPort: 9090` and passed with the
guard removed — a case that pins nothing, caught in review.

## 7. What the fix turned up

### 7.1 The rule is now in one place, which is the actual fix

`f008` guarded `|=` inline and `=` was left unguarded — two operators, one
rule, one copy each, and the copy that did not exist was the bug. Rather than
add a second copy, both now go through `set_value_unless_unchanged`, reached
by their different resolvers.

That is the same argument yqr made against accentcms in `b190` §1 and §5 a day
earlier — duplicated logic drifting from a sibling that already applies the
rule. Worth noting that the project produced an instance of it while filing
one.

`AssignTarget::Existing` changed from a bare `Path` to `{ path, current }`,
because the guard needs to know what is there. That is the honest model:
"this node exists" is less than the resolver actually knows.

### 7.2 A no-op now wins over an unaddressable key

`.["a.b"] = 1` on a document where `a.b` is already `1` used to fail with
*"cannot address key … characters the write path cannot express"*
(`yqr-f007` §6's limit). It now succeeds, because the guard runs before the
writer and nothing needed writing.

Deliberate, and the reasoning is the same as the absent-path skip: no
limitation was reached, because no write was attempted. The refusal still
fires the moment the value actually differs, and **both halves are pinned** —
`a_no_op_write_is_skipped_before_the_key_is_addressed` and
`unaddressable_key_is_reported`, the latter now asking for a value that
differs so it really reaches the writer.

Recorded rather than absorbed, because it is a behaviour change outside this
bug's statement, found by an existing test rather than a new one.

### 7.3 The guard was placed too early, and §7.2 is why that was missed

§7.2 reads as though "the guard runs before the writer" were the design. It is
not — it was where one refusal happened to land, and generalising it silently
swallowed the others. An alias site refuses whatever value it is handed, so
`.b = 1` over `b: *x` went from exit 5 to exit 0 with the alias still in place.

`yqr-b019` corrects the ordering and states the line: the guard may skip past
yqr's own **expressive** limits, because the document already holds what was
asked for, but not past a property of the **document**, where writing would
have done real work. §7.2's outcome stands under that rule; the alias one does
not.
