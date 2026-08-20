# Feature f017 — `to_entries`: enumerate a mapping without losing the keys

**Status:** Done — implemented 2026-08-20; §10's open question settled in §11
**Epic:** jq-style YAML processor (`f001`) — promoted out of the M2 builtin core
**Owner:** yqr maintainers
**Related:** `yqr-r003` (the usage report that promoted this), `yqr-r001` §5
(the builtin gap table this is the first entry pulled from), `yqr-f007` §7 /
`yqr-a002` (the `key(...)` selector that proves the plumbing), `yqr-a001` (the
fidelity contract this deliberately steps outside of), `yqr-f001` (the M1/M2
roadmap this jumps the queue of), `yqr-b016` (the emitter wart this feature's
own output made routine)

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

As shipped, each `value:` line above carries a **trailing space** — an emitter
defect this shape reaches, not a property of the pairs. It is `yqr-b016`, and
§11.4 records why it is carried rather than worked around.

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

- [x] `.m | to_entries` yields one `{key, value}` mapping per entry, in
      document order.
- [x] `to_entries[]` parses and streams the pairs. So do `to_entries[].key` and
      `to_entries[0]` — the chain that follows a path now follows a builtin,
      which was §5's one non-additive grammar change.
- [x] `.to_entries` still parses as a field access, with a test that would fail
      if the word were reserved — `.to_entries`, `.a.to_entries` and
      `.["to_entries"]`, plus a corpus case.
- [x] Non-mapping input is refused with a message naming the actual type,
      pinned for array / number / string / null.
- [x] Every write form is refused at parse with a reason, via `Ast::builtin()`
      — at **four** sites, not the three §6 named: `=`, `+=`, `del(...)` and
      the reorder verbs. §11.5 records why the fourth was the one worth
      missing.
- [x] The order property is pinned against a mapping whose keys are not in
      sorted order — `zebra`, `apple`, `mango`, which no sort in either
      direction produces — and again in the corpus against `web` / `db`.
- [x] The `yqr-r003` §2 task is a single yqr invocation, and that invocation is
      a corpus case (`to_entries/pairs-a-named-mapping`, on the compose
      document's named services). §11.2 states what "the task" now means
      precisely, since exact projection still needs M1/M2.
- [x] `docs/content/guide/enumerate.md` gains the enumeration idiom, the
      `key(...)` comparison with the difference stated, and `from_entries`'
      absence.

## 10. Open question — settled in §11.1

**Whether `key(...)` and `to_entries` should stay two ways to enumerate.**
After this ships, `key(.m[])` and `.m | to_entries[] | .key` both produce a
mapping's keys. That is one more way than a small tool wants.

They are not redundant: `key(...)` reads the document's own key **token**
(quotes included — `yqr-a002` §7.2), while `to_entries` yields the decoded
string, because its output is a computed value with no bytes. So they differ
exactly where fidelity does. That is a real distinction and also a subtle one,
and it should be settled and documented in this feature rather than discovered
by a user comparing two outputs that differ by a pair of quotes.

## 11. What implementing it settled

### 11.1 Both stay, and the difference is one line

Measured rather than argued, on `m:` / `"quoted": 1` / `plain: 2`:

| filter | output |
|---|---|
| `key(.m[])` | `"quoted"` / `plain` |
| `.m \| to_entries[] \| .key` | `quoted` / `plain` |
| either, with `-r` | `quoted` / `plain` |

`key(...)` is what your file says; `to_entries` is what it means. The third row
is what makes this teachable rather than a trap: `-r` collapses the difference,
because asking for raw output *is* asking for the value rather than the
spelling. A user who never leaves `-r` never meets the distinction, and one who
does meets it with a rule that fits in a sentence.

A third measurement fixes which side each is on. On `m:` / `"1": 3` both print
`"1"` without `-r` — `key` because that is the token, `to_entries` because the
renderer must quote a string that would otherwise re-type as a number. Same
output, opposite reasons, and the reasons are what the guide teaches.

`docs/content/guide/enumerate.md` carries this as its own section, which is
what §10 asked for.

### 11.2 What "the r003 task in one invocation" actually delivers

Precisely: `.services | to_entries` puts the name and the value in one stream,
so the `paste` of two aligned streams is gone and nothing has to hold the
alignment property in its head. What it does **not** do is project — "the name
and *just* the domain" is `with_entries` or object construction, both M1/M2 as
`yqr-r001` has them and as §1 puts out of scope. The honest form of the claim
is that the *pairing* is now internal to yqr; the *shaping* is still the
language work this feature declined to pull forward.

### 11.3 The key is a string, and that was decided elsewhere

`to_entries` clones the key it is handed. On a parsed document that key is
always a `Value::String`, including for `1:` and `true:`, because noyalib's
typed mapping is string-keyed and the conversion at the parse boundary has
already decided (`yqr-b002` §2.7). Worth stating because the obvious reading of
`Value::Mapping` — yqr's own model *is* `Value`-keyed — suggests otherwise. A
builtin that re-typed the key would be making the engine's decision over again,
in the wrong place and quietly.

### 11.4 What the first output found: `yqr-b016`

`to_entries` produces a sequence of mappings whose values are mappings, and
that shape hits an emitter defect: a block collection reached through a
sequence item is written with a **trailing space** after its `key:`.
Pre-existing, reachable through `--normalize` on any such document since long
before this feature, and upstream — `render` calls `noyalib::to_string_value`
and only trims the final newline.

Filed as `yqr-b016` and **not worked around**. The obvious local fix — strip
trailing whitespace per rendered line — silently changes a block scalar whose
content legitimately ends a line with spaces (`"a␣␣\nb"` becomes `"a\nb"`),
measured rather than assumed. Altering a string is strictly worse than a
cosmetic space, so the output is pinned as it behaves in `tests/cli.rs` and the
guide says so plainly.

That is twice in two features that shipping something exposed a defect
underneath it, after `yqr-f019` §3.5. The pattern is the same both times: a
shape nobody had produced before is a shape nobody had emitted, parsed, or
edited before.

### 11.5 The fourth mutation site, and why it hid

§6 says `to_entries` "cannot appear on the left of `=`, `+=` or inside
`del(...)`", and the first implementation guarded exactly those three. The
reorder verbs are a fourth, and a code review found them.

The failure was not a missing error — it was a **success**:

```console
$ yqr 'swap(.m | to_entries; 0; 1)' m.yaml
m:
  a: 1
  b: 2
$ echo $?
0
```

Exit 0, document printed unchanged, and with `-i` the file left alone with no
complaint. That is the plausible-wrong-output class `yqr-a001` exists to
refuse, arrived at from an unusual direction: the write driver is *specified*
to leave an **absent** path alone at exit 0, and a builtin path resolves to
nothing, so it was read as absent. Absent and unwritable are different things,
and only one of them deserves silence.

The other three sites could not hide this way — each reaches a resolver that
errors — which is exactly why this was the one to miss. The `Ast::builtin()`
doc comment now says so, so the next mutation form added inherits the warning
rather than the bug.
