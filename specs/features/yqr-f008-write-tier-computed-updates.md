# Feature f008 — Write tier: computed updates (`|=`)

**Status:** Done — both slices shipped 2026-08-22
**Epic:** Fidelity write tier (`f006`–`f008`) — the last piece
**Owner:** yqr maintainers
**Related:** `yqr-b018` (the fidelity bug slice 1's first test found),
`yqr-f006` (write tier v1 — the assignment/`-i` core this
extends), `yqr-a001` §6 (the number model, already ratified), `yqr-a003` (which
turned the old "gated on `f001` M2" into a concrete requirement), `yqr-f017`
(the precedent for measuring a gate before believing it), `yqr-m002` §4/§6.2
(write-tier seam)

## 1. Scope

jq's **update-assignment** operator: `p |= f` sets the node at path `p` to the
result of running filter `f` on that node's **current value**.

```console
$ yqr '.spec.replicas |= (. + 1)' deploy.yaml
$ yqr '.metadata.name |= .' deploy.yaml      # identity: byte-exact
```

The motivating case is the one yqr's own Kubernetes guide names as a
limitation: *"You cannot say 'increment the replica count'; you say what it
should become."*

**In scope:** the operator, its evaluation model, and the minimal arithmetic
that makes the motivating case work. **Out of scope:** the rest of jq's
expression language — comparisons, boolean operators, `//`, and the string and
collection builtins. `yqr-a003` made those a menu, and this feature draws one
item from it, not the menu.

## 2. The gate was half imaginary

`f008` sat as a stub gated on "the expression evaluator", which sounded like
M2 in its entirety. Measured, it splits in two:

**The operator needs nothing new.** `resolve_rhs` (`src/eval.rs:291`) already
evaluates an `Ast` against a `Value` and returns exactly one result. `=`
evaluates its right-hand path against the **document root**; `|=` evaluates
against the **node at the target path**. That is the whole difference, and no
new evaluation machinery is required for it.

**It inherits `=`'s boundary exactly, which is narrower than it first looks.**
An earlier draft of this section claimed *"every filter yqr already has works
as a `|=` right-hand side today — `.m |= to_entries`, `.a |= .b.c`"*. Measured,
neither does: `set_value` writes **scalar leaves**, so a filter returning a
collection is refused with `the right-hand side ... must be a scalar`, exactly
as `.c = .a` is refused when `.a` is a mapping. The claim was written from the
shape of the code rather than from running it.

That leaves the honest statement, which is also a better property: **`|=` works
wherever `=` works** — scalar leaf in, scalar out — and widening it is
`f006`'s collection-RHS limitation to lift, not `f008`'s.

**Only the arithmetic is genuinely missing.** `. + 1` does not parse, because
`+` is lexed only as part of `+=`.

This is `yqr-f017` §3's pattern a second time: a feature queued behind
machinery it turned out to need only part of. Recorded because the lesson is
now twice-confirmed — **measure the gate before believing it**.

## 3. Two slices

**Slice 1 — the operator.** `Mutation::Update { target, rhs: Ast }`, resolved
by evaluating `rhs` against the value at `target` and writing the result
through `f006`'s guarded `set_value`. Ships `|=` wherever `=` already works —
a scalar leaf in, a scalar out (§2).

**Slice 2 — minimal arithmetic.** Binary `+ - * /` over numbers, and `+` over
strings, with parentheses and jq's precedence. Makes `(. + 1)` work, which is
what anyone typing `|=` is reaching for.

They ship **together**. Slice 1 alone would give a `|=` whose headline use —
`.n |= (. + 1)` — still fails with a lexer error, which is a worse surface than
the current honest refusal.

## 4. Semantics

### 4.1 The right-hand side sees the node, not the document

`p |= f` binds `.` inside `f` to the value at `p`. `.spec.replicas |= (. + 1)`
reads `.spec.replicas`, adds one, writes it back. A `|=` whose right-hand side
ignores `.` is legal and degenerate — `.a |= 5` is `.a = 5` spelled longer.

### 4.2 The number model is already decided

`yqr-a001` §6 ratified it, and this feature is the first to need it:

> **Preserve types.** `Int op Int → Int` when exact; `Float` only when
> genuinely fractional. Fidelity forbids silently turning `replicas: 3` into
> `3.0`; large `i64` IDs must not lose precision. Compare/sort by mathematical
> value.

Concretely: `3 + 1 → 4`, `4 / 2 → 2`, `3 / 2 → 1.5`, `1.5 + 1 → 2.5`. An
`Int` result that overflows `i64` is an error, not a silent promotion to
`Float` — promotion would lose the precision the rule exists to protect.

### 4.3 What refuses

| case | result |
|---|---|
| `|=` on a path that selects no node | document unchanged, exit 0 (matches `f006`'s absent-path rule) |
| the right-hand filter errors | document unchanged, exit 5 |
| the right-hand filter yields 0 or >1 values | exit 5, naming the count |
| the right-hand filter returns a collection | exit 5 — `=`'s boundary, inherited (§2) |
| arithmetic on a non-number (`"a" - 1`) | exit 5, naming the types |
| division by zero | exit 5 |
| a builtin on the left (`to_entries \|= .`) | exit 3, via `Ast::builtin()` — the **fifth** mutation site, after the four `yqr-f017` §11.5 enumerated |

### 4.4 Structural integrity

Unchanged from `f006` §7: the computed value goes through the guarded
`set_value`, so only the target node's bytes change or the edit is refused.
A computed string is written with `set_value`'s style-matched quoting, so
`.name |= .` is byte-identical.

## 5. Acceptance criteria

**Slice 1 — the operator**

- [x] `.n |= .` is byte-identical to the input, including quote style.
- [x] **The right-hand side sees the node, not the document.** Pinned by a
      discriminator rather than by an identity filter: on `x: 9` / `a: {t: 0}`,
      `.a.t |= .x` **errors** (`.x` on an `Int`) while `.a.t = .x` writes `9`.
      Same filter, two operators, opposite outcomes — which is the property,
      and an identity filter cannot show it.
- [x] `|=` inherits `=`'s scalar boundary: a right-hand side returning a
      collection is refused with the same message `.c = .a` gives.
- [x] An erroring right-hand side leaves the file untouched with `-i`.
- [x] A `|=` selecting 0 or >1 nodes refuses, naming the count.
- [x] An absent path leaves the document unchanged at exit 0, as `=` does.
- [x] `to_entries |= .` refuses at parse — the **fifth** mutation site, after
      the four `yqr-f017` §11.5 enumerated.
- [x] `key(.a) |= .` refuses at parse, naming `|=` rather than reporting an
      unexpected token.

**Slice 2 — arithmetic**

- [x] `.n |= (. + 1)` increments, and the Kubernetes guide's "no arithmetic"
      limitation is deleted rather than reworded.
- [x] `Int op Int` stays `Int` when exact; `3 / 2` is `1.5`; `4 / 2` is `2`.
- [x] `i64` overflow is an error, not a `Float`.
- [x] Division by zero is an error.
- [x] Precedence and parentheses: `1 + 2 * 3` is `7`, `(1 + 2) * 3` is `9`.
- [x] String `+` concatenates; number `+` string refuses, naming both types.
- [x] Arithmetic works in a read filter too (`.a + 1` as a query), since the
      evaluator is shared — it is not a `|=`-only construct.

**Both**

- [x] Corpus cases in the `m003` write tier, so benches pick them up.
- [x] `docs/content/guide/kubernetes.md` gains the increment idiom.
- [x] `local-ci.sh` clean.

## 6. Deliberately not in this feature

- **Comparisons and booleans.** `|=` does not need them, and `select` — the
  thing that does — is a separate menu item with its own justification.
- **`+=` unification.** `f006`'s `+=` appends to a sequence; jq's `+=` is
  `a = a + b`. They collide, and reconciling them is a breaking change to a
  shipped operator. Out of scope, and worth its own spec if anyone wants it.
- **`-=`, `*=`, `/=`.** jq has them; nothing has asked for them, and `a003`'s
  bar is evidence rather than symmetry.

## 7. What building it found

### 7.1 Two spec claims that were written from the code's shape, not from running it

§2 originally said *"every filter yqr already has works as a `|=` right-hand
side today — `.m |= to_entries`, `.a |= .b.c`"*. Neither does: `set_value`
writes scalar leaves, so a collection result is refused. The section now
carries the correction and the narrower, truer claim — `|=` works wherever `=`
works.

Worth recording because it is the same failure mode a code review caught in
`yqr-a003` a day earlier: reasoning about behaviour from the shape of the code
instead of running it. The measurement discipline this project applies to
*upstream* claims has to apply to its own.

### 7.2 A fidelity bug, found by the first acceptance test

`.n |= .` on `n: 0640` emitted `n: 640`. The typed model cannot carry the
spelling, so re-emitting the same value canonicalises it — on the exact scalar
the fidelity guide leads with.

`|=` guards it: when the computed value equals the current one, the write is
skipped, so the identity update is byte-exact by construction rather than by
luck. `=` has the same hazard and is **not** fixed here — it is filed as
`yqr-b018`, because widening the fix means changing a shipped operator's write
path, which does not belong inside a feature about `|=`.

The general lesson is the narrow one: **an acceptance criterion that says
"byte-identical" finds things a criterion that says "works" does not.**

### 7.3 The `-` ambiguity, resolved by adjacency

`-` opens a negative literal and is also the subtraction operator. Resolved in
the lexer by adjacency: glued to a digit it is part of the number, followed by
anything else it is the operator. So `.[-1]` and `. - 1` both mean what they
look like, and `.a -1` is two terms and a parse error — the same corner jq has,
resolved the same way. Pinned in `lexer.rs` and again at the CLI level, since a
regression there would silently change what `.[-1]` means.

