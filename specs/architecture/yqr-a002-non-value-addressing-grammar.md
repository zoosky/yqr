# Architecture a002 — Addressing what is not a value: comments, keys, and sequence order

**Status:** Accepted (grammar settled; the three slices are staged in §9, and
one of them is blocked on an upstream defect — `yqr-b010`, §6)
**Owner:** yqr maintainers
**Last updated:** 2026-08-16
**Decides:** the "final syntax TBD" left open by `yqr-f007` §4 and §6 for
comment editing, key rename, and sequence reorder
**Affects:** `src/lexer.rs`, `src/parser.rs`, `src/ast.rs`, `src/eval.rs`,
`src/fidelity/write.rs`, `src/fidelity/mod.rs`, `docs/content/guide/`
**Related:** `yqr-a001` (the fidelity guarantee these edits must not break),
`yqr-f007` (the three deferred slices this unblocks), `yqr-b004` §2.1–2.3 (the
original upstream gap catalog), `yqr-b010` (the reorder trivia disagreement that blocks
slice 3, measured out of §6), `yqr-f015` (the noyalib 0.0.22 pin every form
here calls into)

## 1. The problem the three slices share

`yqr-f007` defers comment editing, key rename, and sequence reorder for one
stated reason each — "needs a comment-addressing syntax", "needs a rename
syntax", "needs a reorder syntax". Treated separately those look like three
naming bikesheds. They are one problem:

> **yqr's path grammar addresses value nodes, and only value nodes. All three
> of these edits target something that is not a value node.**

That is not an accident of the implementation. `PathSeg` is `Key(String) |
Index(usize)`, a `Path` resolves through `Document::span_at` to the byte span
of a *value*, and the read seam emits that span verbatim. The whole fidelity
architecture (`yqr-a001` §4) is built on "a path names a value; a value has
bytes; print or replace those bytes". A comment has bytes but is not a value; a
key has bytes but is not the value the path names; an ordering has no bytes of
its own at all.

So the question is not "what should the rename operator be called". It is
**how does a filter name something attached to the node at a path**, and the
answer has to cover all three or yqr grows three unrelated syntaxes.

## 2. Decision

Two shapes, because the three slices are two kinds of thing.

**2.1 A *naming function* wraps a path and names something attached to it.**
The result is a target: it can be read, assigned with `=`, or removed with
`del(...)`.

```text
line_comment(<path>)    the '# ...' comment after the value on the entry's own line
head_comment(<path>)    the run of comment lines immediately above the entry
key(<path>)             the key token of the mapping entry
```

**2.2 A *reorder verb* takes a path and two indices.** There is no single node
to name, so there is nothing for shape 2.1 to wrap.

```text
swap(<path>; <i>; <j>)  exchange two items of the block sequence at <path>
move(<path>; <from>; <to>)  move one item, shifting the rest
```

### 2.3 Grammar

Added to the productions in `src/parser.rs`:

```text
program   := 'del' '(' target ')'              ; remove
           | target '=' rhs                     ; set
           | pipeline '+=' rhs                  ; append (unchanged)
           | reorder                            ; reorder verb
           | target                             ; read (bare pipeline, unchanged,
                                                ;  or bare selector)

target    := selector
           | pipeline                           ; a value node (unchanged)
selector  := 'line_comment' '(' path ')'
           | 'head_comment' '(' path ')'
           | 'foot_comment' '(' path ')'        ; parses only to be refused (§8)
           | 'key'          '(' path ')'
reorder   := ('swap' | 'move') '(' path ';' int ';' int ')'
```

The value arm of `target` is `pipeline`, not `path`, because that is what
`del` takes **today** — `src/parser.rs` documents `program := 'del' '('
pipeline ')'` and `parse_del` calls `parse_pipeline`, so `del(.a | .b)`
parses and deletes (verified on the shipped binary). Narrowing it to `path`
would be a silent regression; this document adds an alternative to `del`'s
argument and changes nothing about the existing one.

Two checks run **after** the target is built, not in the grammar, because the
grammar admits both forms:

- `del` refuses a `Key` target — a key cannot outlive its entry.
- `foot_comment` refuses in every position. It is in the productions solely so
  that the error can name the reason (§8) instead of being a generic
  unexpected-token report.

`+=` keeps a bare `pipeline` by grammar, so no selector reaches it.

One new token, `;`. No new reserved words: like `del` today, each word is
recognized **only in function position** — an identifier immediately followed
by `(`, in a position where a program or a target may begin. That is the
shipped rule (`src/parser.rs`: `peek() == Ident && peek_at(1) == LParen`)
generalized from "at the start of a program" to "at the start of a program or
immediately after `del(`", which is what `del(line_comment(.a))` needs. In
every other position — after `.`, as a mapping key, anywhere inside a path —
the word is an ordinary identifier.

Verified on the shipped binary, not assumed, against the words this document
actually spends: `.line_comment`, `.head_comment`, `.foot_comment`, `.key`,
`.swap`, `.move` and `.del` all read their fields today and continue to.
`swap` and `move` are ordinary YAML field names, so this is the check that
matters most.

`;` is the argument separator because that is jq's (`sub("a";"b")`, verified on
jq 1.8.2), and because `,` is spoken for: the comma stream operator is an M1
roadmap item (`yqr-r001` §5 table). Spending `,` here would cost that later.

### 2.4 The surface, end to end

| Intent | Filter |
|---|---|
| Set the trailing comment | `line_comment(.spec.replicas) = "tuned for peak"` |
| Change it | same form; the body is replaced in place |
| Remove it | `del(line_comment(.spec.replicas))` |
| Read it | `line_comment(.spec.replicas)` |
| Set the comment block above an entry | `head_comment(.spec) = "why this exists"` |
| Multi-line block | `head_comment(.spec) = "line one\nline two"` |
| Remove it | `del(head_comment(.spec))` |
| Rename a key | `key(.metadata.labels.app) = "application"` |
| Read a key | `key(.metadata.name)` |
| Swap two list items | `swap(.spec.containers; 0; 2)` |
| Move a list item | `move(.spec.containers; 0; 2)` |

## 3. Why a naming function, and not the alternatives

### 3.1 Rejected: yq's operator forms

yq is the only real precedent, and the closest neighbour in the market, so it
got measured rather than recalled. Everything below is yq **v4.53.3**, run
locally.

It offers postfix operators for comments and a pipe-into-`key` form for rename:

```console
$ yq '.a line_comment="hi"' y.yaml          # a: 1 # hi          works
$ yq '(.a | key) = "z"' y.yaml              # z: 1               works
```

Three measured problems make it the wrong thing to copy.

**The two families are inconsistent, in opposite directions.** The spelling
that works for comments fails for keys, and vice versa:

```console
$ yq '(.a | line_comment) = "piped"' y.yaml
a: 1
b: 2                     # exit 0, comment silently not set
$ yq '.a key = "z"' y.yaml
Error: bad expression, please check expression syntax
```

A user who learns one form and applies it to the other gets a parse error in
one direction and a **silent no-op** in the other.

**`head_comment` attaches to the wrong side of the entry.** On `a: 1\nb: 2`:

```console
$ yq '.a head_comment="above"' y.yaml
a: 1
# above
b: 2
```

The comment lands *below* the entry it was addressed to — which, in a file with
siblings, means it now documents the next one. Exit 0. This is the same
silent-re-attribution failure yqr refused to inherit from upstream `remove`
(`yqr-f007` §5.1), arriving from the other direction: `head_comment` addressed
the *value node* where the user meant the *entry*.

**The `|` overload does not fit yqr's evaluator.** In yqr a pipe stage receives
a value and produces values; `Ast::Pipe` is threaded with a `Path` so the read
seam can resolve a span. A comment is not in the value model at all, so
`.a | line_comment` would have to make `|` mean something different on its
right-hand side depending on the word there. The function form keeps `|`
meaning exactly one thing.

What yqr does keep from yq: **the words**. `line_comment`, `head_comment` and
`key` are already in the fingers of the population most likely to try yqr, and
the shape being different is the honest signal that the semantics are too.
`head_comment(.a)` puts the block above `a`, which is what the word says.

### 3.2 Rejected: path-suffix pseudo-fields

`.spec.replicas.@comment`, or a `#`-sigil equivalent. Attractive because it is
purely additive to the path grammar and composes wherever a path does.

It is ambiguous, and ambiguous in a way YAML makes real: `@comment` is a legal
mapping key, so `.a.@comment` cannot be told from a document that actually has
one. Every sigil has this problem — YAML keys are arbitrary strings. Escaping
out of it (`.a.["@comment"]` for the literal key) puts the *common* case behind
the escape and the rare one in the plain spelling, which is backwards. And it
does nothing for reorder, which has no node to suffix.

### 3.3 Rejected: CLI subcommands

`yqr comment set .a.b "text"`, alongside `validate`. No grammar change at all,
and the argument list makes `swap`'s two indices trivial.

It forks the mutation surface in two. Today every edit is a filter, which means
every edit composes with `-i`, with stdin, with multi-document streams, and with
the same exit-code contract, because there is exactly one code path. A
subcommand family would have to re-earn each of those, and "is this edit a
filter or a subcommand?" becomes a thing users have to remember. `validate` is
not a counterexample: it evaluates no filter and mutates nothing.

### 3.4 Why the function form wins

- It is **already in the language**. `del(<path>)` is a function of a path that
  is not itself a path. The three selectors are the same move; `swap`/`move`
  are the same move with arguments.
- It **scales to reorder**, which neither 3.1 nor 3.2 does. One shape covers
  five operations rather than three plus a fork.
- Reads come out **free and streaming**: `line_comment(.items[])` yields one
  comment per item, because the path inside the parentheses is an ordinary path
  and iterates like one. Under a postfix operator this would need a second rule
  about how the pipe interacts with the operator.
- It **cannot be spelled ambiguously**, so there is no silent no-op to have.
  `line_comment` outside function position is a field access; inside it, the
  parser knows what it is.

## 4. Semantics

Each form lowers to one noyalib 0.0.22 call. yqr adds the path resolution, the
refusals, and the read normalization; it does not add byte arithmetic.
(`yqr-f007` §2's "own the arithmetic" route stays the route of last resort, and
none of these take it.)

| Form | Upstream call |
|---|---|
| `line_comment(p) = t` | `Document::set_inline_comment(path, text)` |
| `del(line_comment(p))` | `Document::remove_inline_comment(path)` |
| `head_comment(p) = t` | `Document::set_leading_comment(path, text)` |
| `del(head_comment(p))` | `Document::remove_leading_comment(path)` |
| `key(p) = k` | `Document::rename_key(path, new_key)` |
| `swap(p; i; j)` | `Document::swap_items(path, i, j)` |
| `move(p; f; t)` | `Document::move_item(path, from, to)` |
| `line_comment(p)` / `head_comment(p)` read | `Document::comments_at(path)` |
| `key(p)` read | `Document::key_span(path)`, emitted through the read seam |

"Guarded" is load-bearing in both directions. Upstream's own guard covers the
*set* direction only: the four comment mutators and `rename_key` re-parse and
require the typed value to be unchanged. The two **removers do not refuse at
all** — `remove_inline_comment` and `remove_leading_comment` return `Ok(())`
on an unresolved path, on a missing comment, and on every shape the setters
reject (measured; the source is a `let … else { return Ok(()) }` in each).
So every `del(...)` refusal in §5 is a yqr-side pre-check, not a forwarded
upstream error, and it has to be written even where the matching `=` form
needs nothing.

### 4.1 The comment forms address the *entry*, not the value node

`head_comment(.a)` writes above the line that begins `a:`. Measured on 0.0.22:

```text
in:   a: 1\nb: 2
out:  # above\na: 1\nb: 2
```

which is the opposite of yq's placement in §3.1. Nested keys are indented to
the key's own column (`top.a` puts `  # note` inside `top:`), and a CRLF
document gets CRLF-terminated comment lines.

yqr adopts that **placement**. It does not adopt upstream's definition of
which comment block belongs to the entry, and the difference is not cosmetic.

**4.1.1 A blank-detached block is absorbed upstream.** `comments_at().before`
walks upward from the entry's line collecting comment-only lines and
*skipping blank ones* — documented as "an interleaved blank line does not
break the run — only another content node does" — and both leading mutators
edit exactly that range. Measured:

```text
in                 set_leading_comment("a","new")   remove_leading_comment("a")
------------       -----------------------------    ---------------------------
# detached         # new                            (blank line)
(blank line)       (blank line)                     a: 1
a: 1               a: 1
```

The set **replaces** a comment that visually documents whatever came before,
and the remove **deletes** it and leaves a stray blank line. Both at exit 0.

yqr's own `delete_entry` (`src/fidelity/write/delete.rs`) already draws the
line the other way: the entry owns the *contiguous same-indent run
immediately above it*, and a blank-detached comment is deliberately excluded
(`yqr-b006`). §6 leans on that arithmetic as the reference implementation for
reorder; the comment slice cannot then quietly adopt the opposite rule for
the same question.

So yqr pre-checks: when `comments_at(p).before` is non-empty and a blank line
separates it from the entry, `head_comment(p)` reads `null`, and both
`head_comment(p) = t` and `del(head_comment(p))` **refuse (exit 5)** naming
the detached block, rather than letting upstream rewrite it. This is a
divergence in a documented upstream choice, not an upstream defect, so it is
not routed anywhere — it is the third clause of the `yqr-f007` §5.1 reminder
("upstream has yqr's semantics") applied prospectively for once, before
adoption rather than after.

**4.1.2 An entry whose value starts on the next line is not commentable
inline.** `line_comment` guards on whether the *value span* contains a
newline, and a nested entry with a single child has a single-line value span.
So the guard does not fire, and the comment lands on the **child's** line:

```text
in                 set_inline_comment("a","X")
----------         ---------------------------
a:                 a:
  b: 1               b: 1  # X
```

`Ok`, exit 0, and the comment now documents `a.b` rather than `a`. The
removal direction is the same defect: `remove_inline_comment("a")` on
`a:\n  b: 1  # child\n` deletes the child's comment. Both measured on 0.0.22.

yqr refuses (exit 5) whenever the value span does not begin on the key's own
line — one comparison between `key_span(p)` and `span_at(p)`, both already
in the seam. A sequence item is unaffected: `xs[0]` sits on its own line, and
`line_comment(.xs[0])` is supported and correct (measured).

### 4.2 An empty comment is a bare `#`, not a removal

`line_comment(.a) = ""` writes `a: 1  #`. Removal is `del(line_comment(.a))`.

This diverges from yq, where `line_comment=""` removes (measured). The
divergence is deliberate: upstream distinguishes the two, a bare `#` is a thing
people write, and yqr already owns an unambiguous removal spelling. Conflating
them would make one of the two unreachable to buy a keystroke.

### 4.3 Comment text is normalized on read so the surface round-trips

`comments_at` reports the comment body with its leading space intact —
`a: 1  # the note` reads back as `" the note"`. Writing that value back would
render `#  the note` and grow a space per cycle. yqr therefore strips **one**
leading space on read, which makes this a falsifiable property in the `a001`
tradition:

```text
yqr 'line_comment(.a) = T' f.yaml | yqr 'line_comment(.a)'   ==   T
```

for every single-line `T`, and the same for `head_comment` with `\n`-joined
lines. This is an acceptance criterion, not a note (§9).

### 4.4 Reads are total; mutations are loud

A read yields `null` where there is nothing to report — no comment, or a `key(...)`
on a sequence item — matching `.missing`, which yields `null` today. A read must
never fail a batch.

**This holds even where the matching write refuses**, and §5 is a catalog of
writes, not of reads. `key(.xs[0])` is `null`, not an error, though
`key(.xs[0]) = "k"` refuses; `head_comment(.xs[0])` reports the block above
the item, though nothing can set or remove it. Every §5 row is read as "in
this direction, for this case" — the table says which.

A mutation refuses (exit 5) rather than no-op, with one exception yqr already
has: a *document* in a multi-document stream whose target does not resolve is
skipped, exactly as `del` behaves now. Selecting more than one node stays the
existing hard error ("a mutation must target exactly one node, but the filter
selected N").

`del(...)` is a mutation for this purpose, which is the whole reason §4's note
about the unguarded removers matters: upstream's `Ok(())`-on-anything is a
no-op, and this rule forbids one.

### 4.5 Indices

`swap`/`move` take yqr indices, not upstream's `usize`: negatives count from the
end, as `.[-1]` does. yqr resolves them against the sequence before the call, so
`swap(.xs; 0; -1)` swaps first and last.

## 5. What is refused

Measured against noyalib 0.0.22 by driving each call directly, not read off its
docs, which several rows below contradict: §5.4's three outright, and §5.1 /
§5.2 wherever "multi-line" or "nested" in the docs turns out to mean "the
**value span** contains a newline" — which an entry can fail to do while its
own entry plainly spans two lines.

**Every row is a direction**, because upstream's behaviour is not symmetric:
the setters are guarded and the removers are not (§4), so a case can refuse
under `=` and silently no-op under `del(...)`. Reads are total by §4.4 and
appear here only where the interesting answer is not "the obvious one".

### 5.1 `line_comment`

| Case | Direction | noyalib 0.0.22 | What yqr does |
|---|---|---|---|
| value span covers 2+ lines (`a:\n  b: 1\n  c: 2`, `a: \|\n  text`) | `=` | refuses | forwards: the node has no line of its own; comment its entries |
| same | `del` | **`Ok`, no-op** | yqr pre-checks and refuses with that message |
| same | read | `inline` is `None` | `null` |
| value starts on the line *below* the key (`a:\n  b: 1`) | `=`, `del` | **`Ok`, and wrong** — edits the child's line (§4.1.2) | yqr refuses: compare `key_span` against `span_at` |
| same | read | reports the *child's* comment | `null`, on the same test |
| single-line sequence item | all | **supported and correct** | — |

### 5.2 `head_comment`

| Case | Direction | noyalib 0.0.22 | What yqr does |
|---|---|---|---|
| sequence item | `=` | refuses | forwards: leading comments attach to mapping keys only |
| same | `del` | **`Ok`, no-op** | yqr pre-checks and refuses with that message |
| same | read | reports the block above the item | reports it — reads are total (§4.4) |
| value span covers 2+ lines | `=` | refuses | forwards |
| same | `del` | **`Ok`, no-op** | yqr pre-checks and refuses |
| nested entry whose value fits one line (`a:\n  b: 1`) | `=`, `del` | **supported and correct** — the block lands above `a:` at its indent | — |
| the block above is blank-detached | `=` | **`Ok`** — replaces it (§4.1.1) | yqr refuses |
| same | `del` | **`Ok`** — deletes it, leaving a stray blank line | yqr refuses |
| same | read | reports it | `null` — it is not this entry's block |

### 5.3 `key`

| Case | Direction | noyalib 0.0.22 | What yqr does |
|---|---|---|---|
| sequence item | `=` | refuses | forwards: a sequence item has no key |
| same | read | `key_span` is `None` | `null` |
| key produced by a `<<` merge | `=` | refuses (its own message — the entry lives at the anchor) | forwards |
| same | read | `key_span` is `None` | `null` |
| key reached through an alias (`*name` site) | `=` | refuses | forwards: rename the anchor's own entry |
| new key collides with an existing sibling | `=` | refuses | forwards: the rename would create a duplicate |
| new key is `<<` | `=` | refuses | forwards: it would become a merge directive |
| new key holds a non-printable character (`\n` included) | `=` | refuses, naming the code point | forwards |
| new key is **empty** | `=` | **`Ok`** — writes `"": 1` | yqr refuses: `key_is_plain` rejects it, so yqr could never address the result again (§7.3) |

`rename_key` also refuses a bracket segment that is not a non-negative integer
(`servers[web]`), which is unreachable from yqr: the evaluator resolves every
segment before lowering, so `to_noyalib_path` emits `[i]` for a `usize` only.
Recorded so the seam does not grow a hand-built path string later.

### 5.4 `swap` / `move`

| Case | noyalib 0.0.22 | What yqr does |
|---|---|---|
| **flow** sequence | **succeeds** (`[one, two, three]` -> `[three, two, one]`) — doc comment lists it as an error | — |
| multi-line items | **succeeds** — doc comment lists it as an error | — |
| differently-indented items (`- a\n-   b`) | **succeeds** — doc comment lists it as an error | — |
| either item carries a comment | succeeds, and is **wrong** — `yqr-b010` | yqr must refuse; §6 |
| index out of range | refuses, naming the length | forwards |

### 5.5 Refused by yqr before any call

| Case | What yqr does |
|---|---|
| any form, key containing `.`, `[`, `]`, `*`, or empty | inherited refusal, §7.3 |
| `foot_comment(...)` | refused when the target is built; no upstream mutator exists (§8) |
| `del(key(.a))` | refused when the target is built (§2.3): a key cannot outlive its entry; use `del(.a)` |
| `key(...) += ...`, `line_comment(...) += ...` | refused at parse: `+=` takes a value path |
| a selector over more than one node | the existing multi-target refusal (§4.4) |

Three documented upstream refusals turn out to be stale, all in §5.4:
`swap_items`' own doc comment lists a flow sequence, multi-line items **and**
differently-indented items among its errors, and 0.0.22 refuses none of the
three. Nothing yqr plans depends on them.

They are recorded because `yqr-f007` §5.1's standing reminder has three clauses
now — "upstream has the call", "upstream does what its docs say" and "upstream
has yqr's semantics" are three different questions. §5.4 is the second clause
answered "no" three times. The third clause is answered "no" in **nine rows**
of §5.1–§5.3: every row whose right-hand column is a yqr pre-check rather than
a forward. That is the real cost estimate for the comment slice, and none of it
was visible from the docs.

## 6. Sequence reorder is blocked: upstream moves values, not entries

The grammar for reorder is settled. The slice is not shippable on 0.0.22, and
this was found by measuring rather than by review.

`swap_items` and `move_item` exchange **value bytes** and nothing else, so
every comment stays attached to the position rather than to the item it
documents — at `Ok` and exit 0, and past upstream's own integrity guard by
construction, since that guard compares typed values and a comment is not in
one. `-i` would write it to the user's file.

**The measurement, the guard argument and the route are recorded once, in
`yqr-b010`**, which is the open bug the upstream filing will hang on. They are
not restated here; what belongs in this document is the consequence for the
grammar it settles.

That consequence is: the reorder verb is designed and staged, and it does not
ship on 0.0.22. This is the `yqr-b006` failure class and the exact defect yqr
already refused to inherit once — `yqr-f007` §5.1 declines upstream `remove`
because a head comment "survives, silently re-attributed to the next sibling".
Reorder re-attributes every comment in the affected range. It is also the case
that most needs the fix: a commented list item is the normal shape of the files
yqr targets (`spec.containers`, GitHub Actions `steps`, Ansible tasks).

Until `yqr-b010` lands, `swap`/`move` either stay unimplemented or ship with a
yqr-side refusal when either item carries a comment. The refusal is honest and
small, but it declines the common case, which is why §9 sequences this slice
last rather than shipping a stub.

## 7. Compiler surface

### 7.1 AST

The two shapes in §2 are the two shapes in the AST — the type carries the
argument:

```rust
/// What a mutation addresses: a value node, or something attached to one.
pub enum Target {
    Value(Ast),
    LineComment(Ast),
    HeadComment(Ast),
    /// Parsed only so the refusal can name a reason (§8); never lowered.
    FootComment(Ast),
    Key(Ast),
}

pub enum Mutation {
    Assign { target: Target, rhs: Rhs },   // was `path: Ast`
    Append { path: Ast, rhs: Rhs },        // value-only, unchanged
    Delete { target: Target },             // was `path: Ast`
    Reorder { path: Ast, op: ReorderOp, from: i64, to: i64 },
}

pub enum ReorderOp { Swap, Move }
```

`Assign` and `Delete` taking a `Target` is what makes `del(line_comment(.a))`
fall out of the existing `del` production instead of needing a rule of its own —
`Target::Value` carries the pipeline `del` already accepted, so the existing
form is the `Value` arm and nothing about it changes. `Append` keeps a bare path
because there is nothing to append to a comment or a key.

The two target-kind checks of §2.3 live here: `Delete` refuses `Target::Key`,
and every form refuses `Target::FootComment`.

This is a breaking change to the public `Mutation` enum, so it bumps the minor
pre-1.0 (`yqr-m001` §3).

### 7.2 Seam

`FidelityWriter` (`src/fidelity/write.rs`) gains one method per row of §4's
table; each is a path lowering plus one upstream call plus an error map, in the
shape `set_value` already has, plus the pre-checks §5 assigns to yqr.

Reads add two methods to `FidelityEngine`: one for the comment bundle, one for
the key span.

`key(...)` must **not** be answered from the resolved `Path`'s last segment,
which is the obvious shortcut and is wrong. `PathSeg::Key(String)` is
documented (`src/fidelity/mod.rs`) as "stored decoded (no quotes, escapes
resolved)" — it is the string the *filter* named, not the bytes the *document*
holds. Reading it back would make `key(.a)` echo the query: a document
authoring the key as `"a"` or `'a'` would report `a`, and a key reached
through a `<<` merge would report a key that has no token in the file at all
(the same path `rename_key` refuses for exactly that reason). That contradicts
§1's own framing — a path names a value, a value has bytes, print those bytes —
for the one selector whose entire subject *is* a token.

So the read goes through `Document::key_span` and the existing read seam, like
every other read: it emits the key token verbatim, quotes included, and yields
`null` where there is no token (`None` for a sequence item and for a
merge-produced key, both measured). `key_span` is the API `yqr-b004` §2.2 had
added upstream and shipped in 0.0.18 for precisely this; the shortcut would
have left it unused.

Comment and rename writes are guarded upstream (`set_inline_comment` and
friends re-parse and require the typed value to be unchanged; `rename_key`
re-parses and compares against an expected value) — in the **set** direction
only; §4 covers the unguarded removers. Reorder's guard is the one §6 shows to
be insufficient, and is the reason that slice is not simply a call.

### 7.3 Inherited limitation

Every form here lowers through `to_noyalib_path`, so every form inherits yqr's
existing refusal for keys that `PathSeg::key_is_plain` rejects — a key
containing `.`, `[`, `]`, or `*`, **or an empty key**. The first four are the
Kubernetes label case, which is the *other* open item in `yqr-f007` §6 and is
orthogonal to this document: `line_comment(.metadata.labels["app.kubernetes.io/name"])`
will report the unaddressable-key error until that is settled. Worth knowing
before the comment slice is demoed on a Kubernetes manifest.

The empty key is what makes this a constraint on `key(...) = k` and not only
on the path: upstream accepts `key(.a) = ""` and writes `"": 1` (measured),
after which no yqr path can address the entry again. §5.3 refuses it on the
same predicate the path lowering already uses, so the addressable set is
closed under rename.

## 8. What this does not decide

- **The `.`/`[` key-addressing escape** (§7.3). Orthogonal; noyalib's
  `parse_query_path` has no escape form at all, so it is an upstream grammar
  question, not a yqr one.
- **`foot_comment`.** It is a production in §2.3 and a `Target` variant, so the
  parser builds it and then refuses it with a reason instead of reporting an
  unexpected token. There is no upstream mutator and no design here.
- **Bulk forms.** `line_comment(.items[]) = "x"` stays a
  targets-more-than-one-node refusal. Lifting it is a general mutation question
  (it applies equally to `.items[].a = 5`), not a comment question.
- **`|=` on any of these.** Computed update is `yqr-f008`, gated on M2.
- **Collection right-hand sides** for `+=` / new-key assignment — the third open
  `yqr-f007` §6 item, unrelated to addressing.

## 9. Staging and acceptance criteria

Three slices, ordered so the grammar lands under the simplest semantics.

**Slice 1 — `key(...)` rename.** The whole new grammar path (selector, `Target`,
one upstream call) under the operation with the fewest cases and the
best-guarded upstream mutator.

- [ ] `key(<path>) = "new"` renames in place; value, inline comment, and head
      comment stay byte-identical, and every other byte in the file does.
- [ ] `key(<path>)` reads the key **token from the document** via `key_span`
      (§7.2): a key authored `"a"` reads back `"a"` with its quotes, not `a`.
- [ ] Each §5.3 `key` row refuses with exit 5 and a message naming the reason;
      `-i` leaves the file untouched on refusal. The empty-key row is yqr's own
      pre-check, not a forwarded error.
- [ ] `key(.xs[0])` and a merge-produced key read `null` and do not fail the
      batch (§4.4), though the matching `=` refuses.
- [ ] `.key`, `.swap` and `.move` still parse as field accesses.
- [ ] `del(.a | .b)` still parses and deletes (§2.3 keeps `del`'s existing
      argument), and `del(key(.a))` refuses with the §5.5 message.

**Slice 2 — `line_comment` / `head_comment`.** Adds the second and third
selectors, `del(...)` composition, and the read path.

- [ ] Set, change, and remove, for both kinds, byte-exact elsewhere.
- [ ] `head_comment` places the block **above** the addressed entry, at the
      entry's own indent, with the document's line terminator.
- [ ] The §4.3 round-trip property holds, including for a `\n`-joined
      `head_comment` and for a comment body with leading spaces of its own.
- [ ] `= ""` writes a bare `#`; only `del(...)` removes (§4.2).
- [ ] Each §5.1 / §5.2 row behaves as its **direction** column says. In
      particular every `del(...)` row refuses (exit 5) rather than inheriting
      upstream's `Ok`-and-no-op, and `-i` leaves the file untouched.
- [ ] `line_comment` on an entry whose value starts on the next line refuses in
      both directions and never touches the child's line (§4.1.2) — the read on
      that entry is `null`, not the child's comment.
- [ ] A blank-detached comment block above the entry is never replaced, deleted
      or reported (§4.1.1); all three directions refuse or read `null`, and the
      `delete_entry` case it mirrors keeps its existing behaviour.
- [ ] Reads yield `null`, never an error, where there is no comment — including
      where the matching write refuses (§4.4).
- [ ] `.line_comment`, `.head_comment` and `.foot_comment` still parse as field
      accesses; `foot_comment(.a)` refuses with the §8 reason, not a generic
      parse error.

**Slice 3 — `swap` / `move`.** Blocked on `yqr-b010` (§6). Ships when the
upstream reorder moves an entry's trivia with it, or with an explicit
refusal-when-commented and that limit documented.

- [ ] `swap`/`move` on an uncommented block sequence, byte-exact elsewhere.
- [ ] An item's inline and head comments travel with the item.
- [ ] Negative indices resolve as `.[-1]` does (§4.5).
- [ ] Out-of-range indices refuse with exit 5.

Each slice updates `docs/content/guide/` (CLAUDE.md rule 15) and the yq
comparison page, whose "not in the released grammar yet" note for renames and
comment edits is what these slices retire.

## 10. Provenance

Every behavioural claim here was measured on 2026-08-15, not recalled:

- **yq v4.53.3** (§3.1) — the postfix/pipe asymmetry, the silent no-op on
  `(.a | line_comment) = "..."`, the `head_comment` misplacement below the
  entry, `line_comment=""` removing, and the absence of any `swap`/`move`/
  `reorder` builtin.
- **jq 1.8.2** (§2.3) — `;` as the argument separator, `,` as the stream
  operator.
- **noyalib 0.0.22** (§4, §5, §6) — the pinned crate driven directly through a
  throwaway integration test: the §5 catalog probed **per direction** (set,
  `del`, read) rather than per case, which is what turned up the unguarded
  removers and the nested-node misattribution; then the reorder trivia
  behaviour isolated across inline comments, head comments, one-sided comments,
  `move_item`, anchors, quote styles, blank lines and block scalars.
- **yqr at `41041d4`** (§2.3, §4.4) — `.del`, `.key`, `.line_comment`,
  `.head_comment`, `.foot_comment`, `.swap` and `.move` all parsing as field
  accesses; `del(.a | .b)` parsing and deleting; `.missing` yielding `null`;
  `del(.absent)` being a no-op; and the exact multi-target refusal text.

Re-measured 2026-08-16 under review, which corrected §4.1, §5 and §7.2. The
correction that matters is methodological: the first pass probed each case in
the direction its *setter* takes and generalized the answer to the form, and
upstream is not symmetric — the removers refuse nothing, and one guard fires on
the value span where the user named the entry. §5's direction column exists so
that a later reader cannot repeat it.
