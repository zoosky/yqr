# Bug b018 — `.n = .n` rewrites `0640` as `640`: an assignment that changes nothing still re-spells the scalar

**Status:** Open — found 2026-08-22 by `yqr-f008`'s first acceptance test; the
`|=` half is fixed there, the `=` half is this bug
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

Not pinned in `tests/cli.rs` yet: a test asserting `640` would read as the
intended behaviour. It goes in as a regression test with the fix, on
`yqr-m003`'s rule that a pin states what the bug does and this one's shape
makes the pin worse than the prose — the same call `yqr-b015` §5 made.
