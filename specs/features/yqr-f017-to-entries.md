# Feature f017 — `to_entries`: enumerate a mapping without losing the keys

**Status:** Draft (scoped; not started)
**Epic:** jq-style YAML processor (`f001`) — promoted out of the M2 builtin core
**Owner:** yqr maintainers
**Related:** `yqr-r003` (the usage report that promoted this), `yqr-r001` §5
(the builtin gap table this is the first entry pulled from), `yqr-f007` §7 /
`yqr-a002` (the `key(...)` selector that proves the plumbing), `yqr-a001` (the
fidelity contract this deliberately steps outside of), `yqr-f001` (the M1/M2
roadmap this jumps the queue of)

## 1. Scope

One builtin: `to_entries`, which turns a mapping into a sequence of key/value
pairs so a filter can act on both halves at once.

**In scope:** the builtin, its grammar position, its fidelity semantics, and
`from_entries`' deliberate exclusion.

**Out of scope, and each stays queued behind M1/M2 as `yqr-r001` has it:**
`select`, `map`, `with_entries`, `keys`, string concatenation and
interpolation. This feature makes the pairing *available*; making it
*transformable* is the language work, and pulling one builtin forward is not an
argument for pulling the rest.

## 2. The problem

`.services[]` iterates a mapping's **values**, so the keys are gone by the time
anything downstream can use them. On the most ordinary YAML layout there is —
a mapping of named things — yqr can produce the data and cannot say what it is
about. `yqr-r003` records a real session that hit exactly this and left for a
Python script.

`key(...)` (`yqr-f007` §7) closes half of it: `key(.services[])` enumerates the
names, and because a missing field yields `null` rather than skipping, the
names and the values stay index-aligned, so `paste` pairs them
(`yqr-r003` §5).

Two aligned streams are not pairs, though. Nothing *inside* yqr can act on a
key and its value together, which is what a single filter needs in order to
grow `select` or string building later.

## 3. Why this is not blocked on M1 construction

The assumption that kept `to_entries` queued: jq's version returns objects
(`[{"key": k, "value": v}]`), yqr has no object-construction syntax, therefore
`to_entries` waits for M1.

**The gate is imaginary.** Object *construction syntax* is what M1 owes —
`{a: .x}` written by a user. A builtin does not need it: it constructs a
`Value` in Rust, and yqr's renderer already emits any `Value`, mappings
included. Verified on the shipped binary, which renders a mapping value
through exactly that path today:

```console
$ yqr --normalize '.a' f.yaml
x: 1
y: two
```

So the only genuinely new machinery is the builtin itself.

## 4. Surface

```console
$ yqr '.services | to_entries'
- key: alpha
  value:
    domain: alpha.example.com
    tier: edge
- key: beta
  value:
    domain: beta.example.com
    tier: core

$ yqr -r '.services | to_entries[] | .key'
alpha
beta
```

`key` / `value` are jq's field names, kept deliberately: the shape is worth
nothing if it does not transfer.

Order is the mapping's **document order**, not sorted. yqr's value model is
order-preserving (`indexmap`), the read path is order-preserving, and a
`to_entries` that sorted would silently break the §2 pairing against
`.services[].domain`. jq sorts object keys; yqr does not, and this is one of
the places that difference is load-bearing rather than cosmetic.

## 5. Grammar

`to_entries` is a **bare identifier in term position**, matching jq, and it
costs no reserved word — the same argument `yqr-a002` §2.3 makes for `key`,
reached from the other side:

```text
term := path ('?')*
      | builtin ('?')*          ; new
builtin := 'to_entries'
```

Every yqr path starts with `.`, so an identifier at the start of a term cannot
be one. `.to_entries` keeps reading a field named `to_entries`, exactly as
`.key` and `.del` still do — and that is the property to pin with a test, not
to assume.

Note the difference from `key(...)`: a *selector* wraps a path and is
recognised by the `(` that follows it; a *builtin* takes its input from the
pipe and is recognised by being an identifier where a path was expected. Both
avoid reserving the word. The AST gains an `Ast::Builtin` variant rather than
overloading `Ast::Field`.

`to_entries[]` must parse as "the builtin, then iterate", so the postfix
bracket suffix has to apply to a builtin term as well as a path term. This is
the one place the grammar change is more than additive, and it is worth getting
right in the first slice rather than discovering it in the second.

## 6. Fidelity semantics

`to_entries` produces a **computed** value: it has no single node in the source
document, so it has no path and no byte span. That is not a gap, it is the
existing contract — `yqr-a001` §4 and `fidelity::run_ast`'s `None` arm already
render computed results through the typed renderer.

Two consequences worth stating so nobody reads them as bugs:

- **Output is normalized, not byte-preserved.** Comments, quote styles and
  scalar spellings inside the emitted pairs are the renderer's, not the file's.
  `to_entries` is a *query* form; the fidelity guarantee is about reads that
  name a node and about edits, and this names no node.
- **It is read-only.** `to_entries` cannot appear on the left of `=`, `+=` or
  inside `del(...)`. There is nothing to write back to — the pairs are a view
  yqr invented. Refused at parse, with a message that says so rather than a
  generic one (the `yqr-a002` §8 pattern).

## 7. What is refused

| Case | Result |
|---|---|
| `to_entries` on a sequence | error naming the type: jq refuses this too |
| `to_entries` on a scalar or `null` | same |
| `to_entries = ...`, `del(to_entries)` | parse error: it names no node to write |
| `.to_entries` | **unchanged** — a field access |

## 8. `from_entries` is deliberately excluded

The inverse looks like the natural pair, and it is not, for this release:
`from_entries` is only useful once a filter can *build* the pairs it consumes,
which needs object construction and `map` — both M1/M2. Shipping it now would
add an operation whose only possible input is `to_entries`' unmodified output,
i.e. the identity function spelled in two builtins.

Recorded so it does not read as an oversight, on the precedent of upstream
declining `remove_subtree` for the same reason (`yqr-b004` §7).

## 9. Acceptance criteria

- [ ] `.m | to_entries` yields one `{key, value}` mapping per entry, in
      document order.
- [ ] `to_entries[]` parses and streams the pairs.
- [ ] `.to_entries` still parses as a field access, with a test that would fail
      if the word were reserved.
- [ ] Non-mapping input is refused with a message naming the actual type.
- [ ] Every write form (`=`, `+=`, `del`) is refused at parse with a reason.
- [ ] The order property is pinned against a mapping whose keys are not in
      sorted order, so a future sort cannot land silently.
- [ ] The `yqr-r003` §2 task is a single yqr invocation, and that invocation is
      a corpus case.
- [ ] `docs/content/guide/` gains the enumeration idiom (CLAUDE.md rule 15),
      alongside the `key(...)` two-stream form, with the difference stated.

## 10. Open question

**Whether `key(...)` and `to_entries` should stay two ways to enumerate.**
After this ships, `key(.m[])` and `.m | to_entries[] | .key` both produce a
mapping's keys. That is one more way than a small tool wants.

They are not redundant: `key(...)` reads the document's own key **token**
(quotes included — `yqr-a002` §7.2), while `to_entries` yields the decoded
string, because its output is a computed value with no bytes. So they differ
exactly where fidelity does. That is a real distinction and also a subtle one,
and it should be settled and documented in this feature rather than discovered
by a user comparing two outputs that differ by a pair of quotes.
