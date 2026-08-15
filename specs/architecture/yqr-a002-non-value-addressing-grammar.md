# Architecture a002 — Addressing what is not a value: comments, keys, and sequence order

**Status:** Accepted (grammar settled; the three slices are staged in §9, and
one of them is blocked on an upstream defect measured here — §6)
**Owner:** yqr maintainers
**Last updated:** 2026-08-15
**Decides:** the "final syntax TBD" left open by `yqr-f007` §4 and §6 for
comment editing, key rename, and sequence reorder
**Affects:** `src/lexer.rs`, `src/parser.rs`, `src/ast.rs`, `src/eval.rs`,
`src/fidelity/write.rs`, `src/fidelity/mod.rs`, `docs/content/guide/`
**Related:** `yqr-a001` (the fidelity guarantee these edits must not break),
`yqr-f007` (the three deferred slices this unblocks), `yqr-b004` §2.1–2.3 (the
original upstream gap catalog) and §6.5 (the reorder defect §6 below measured),
`yqr-f015` (the noyalib 0.0.22 pin every form here calls into)

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
           | path '+=' rhs                      ; append (unchanged)
           | reorder                             ; reorder verb
           | pipeline                            ; read-only query (unchanged)
           | selector                            ; read-only trivia/key query

target    := path                                ; a value node (unchanged)
           | selector
selector  := 'line_comment' '(' path ')'
           | 'head_comment'  '(' path ')'
           | 'key'           '(' path ')'
reorder   := ('swap' | 'move') '(' path ';' int ';' int ')'
```

One new token, `;`. No new reserved words: like `del` today, each word is
recognized **only in function position** — an identifier immediately followed
by `(`, at the start of a program. Verified on the shipped binary, not
assumed: `.del`, `.key`, and `.comment` all read their fields today and
continue to.

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

Each form lowers to one guarded noyalib 0.0.22 call. yqr adds the path
resolution, the refusals, and the read normalization; it does not add byte
arithmetic. (`yqr-f007` §2's "own the arithmetic" route stays the route of last
resort, and none of these take it.)

| Form | Upstream call |
|---|---|
| `line_comment(p) = t` | `Document::set_inline_comment(path, text)` |
| `del(line_comment(p))` | `Document::remove_inline_comment(path)` |
| `head_comment(p) = t` | `Document::set_leading_comment(path, text)` |
| `del(head_comment(p))` | `Document::remove_leading_comment(path)` |
| `key(p) = k` | `Document::rename_key(path, new_key)` |
| `swap(p; i; j)` | `Document::swap_items(path, i, j)` |
| `move(p; f; t)` | `Document::move_item(path, from, to)` |
| reads | `Document::comments_at(path)`, and the resolved path's last segment |

### 4.1 The comment forms address the *entry*, not the value node

`head_comment(.a)` writes above the line that begins `a:`. Measured on 0.0.22:

```text
in:   a: 1\nb: 2
out:  # above\na: 1\nb: 2
```

which is the opposite of yq's placement in §3.1, and is the reason yqr can adopt
the upstream call directly instead of correcting it. Nested keys are indented to
the key's own column (`top.a` puts `  # note` inside `top:`), and a CRLF
document gets CRLF-terminated comment lines.

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

A mutation refuses (exit 5) rather than no-op, with one exception yqr already
has: a *document* in a multi-document stream whose target does not resolve is
skipped, exactly as `del` behaves now. Selecting more than one node stays the
existing hard error ("a mutation must target exactly one node, but the filter
selected N").

### 4.5 Indices

`swap`/`move` take yqr indices, not upstream's `usize`: negatives count from the
end, as `.[-1]` does. yqr resolves them against the sequence before the call, so
`swap(.xs; 0; -1)` swaps first and last.

## 5. What is refused

Measured against noyalib 0.0.22 by driving each call directly, not read off its
docs — two rows below contradict the upstream doc comment.

| Case | Result | Message yqr surfaces |
|---|---|---|
| `line_comment` on a multi-line/nested node | refused upstream | the node has no line of its own; comment its entries |
| `line_comment` on a single-line sequence item | **supported** | — |
| `head_comment` on a sequence item | refused upstream | leading comments attach to mapping keys only |
| `head_comment` on a multi-line/nested entry | refused upstream | same |
| `key(...)` on a sequence item | refused upstream | a sequence item has no key |
| `key(...) = "<<"`, or a non-printable key | refused upstream | cannot be spelled or would become a merge directive |
| `key(...)` colliding with an existing sibling | refused upstream | the rename would create a duplicate |
| `key(...)` reached through an alias, or inside an anchored value with aliases | refused upstream | rename the anchor's own entry |
| `swap`/`move` on a **flow** sequence | **succeeds** (`[one, two, three]` -> `[three, two, one]`) | — |
| `swap`/`move` over multi-line items | **succeeds** | — |
| `swap`/`move` where either item carries a comment | succeeds, and is **wrong** — see §6 | yqr must refuse; see §6 |
| any form, key containing `.` or `[` | refused by yqr | inherited, see §7.3 |
| `foot_comment(...)` | refused at parse | no upstream mutator exists; recognized so the error names the reason |
| `del(key(.a))` | refused at parse | a key cannot outlive its entry; use `del(.a)` |
| `key(...) += ...`, `line_comment(...) += ...` | refused at parse | `+=` appends to a sequence |

The last two upstream rows are worth stating plainly: `swap_items`' own doc
comment lists a flow sequence and differently-indented items among its errors,
and neither is refused in 0.0.22. Nothing depends on those refusals here, but
`yqr-f007` §5.1's standing reminder applies — "upstream has the call" and
"upstream does what its docs say" are different questions, and both are
different from "upstream has yqr's semantics".

## 6. Sequence reorder is blocked: upstream moves values, not entries

The grammar for reorder is settled. The slice is not shippable on 0.0.22, and
this was found by measuring rather than by review.

`swap_items` and `move_item` swap **value bytes**. Everything else stays where
it was — including the comments:

```text
in                       swap_items("", 0, 1)        move_item("", 0, 2)
- one  # first           - two  # first              - b  # ca
- two  # second          - one  # second             - c  # cb
                                                     - a  # cc
```

Head comments behave the same way: `# about one` stays above index 0 while the
value that it described moves to index 1.

Every one of these returns `Ok`. They pass the upstream integrity guard *by
construction*: the guard compares typed values, and a comment is not in the
typed value, so a guard that compares them can never see this. The exit code is
0 and `-i` would write it to the user's file.

This is the `yqr-b006` failure class and the exact defect yqr already refused to
inherit once — `yqr-f007` §5.1 declines upstream `remove` because a head comment
"survives, silently re-attributed to the next sibling". Reorder re-attributes
every comment in the affected range.

It is also the case that most needs the fix: a commented list item is the normal
shape of the files yqr targets (`spec.containers`, GitHub Actions `steps`,
Ansible tasks).

**Route.** Upstream, on the `PR-with-fix` precedent (`yqr-b004` §5), because
yqr already owns the reference implementation: `delete_entry`
(`src/fidelity/write/delete.rs`) computes exactly the range an entry owns —
value span, continuation lines, and the contiguous same-indent head-comment run
above it, with a blank-detached comment correctly excluded. A trivia-aware
reorder is two of those ranges exchanged. That arithmetic was written, argued
and tested for delete in `yqr-b006`; it transfers.

Until that lands, `swap`/`move` either stay unimplemented or ship with a yqr-side
refusal when either item carries a comment. The refusal is honest and small, but
it declines the common case, which is why §9 sequences this slice last rather
than shipping a stub. Recorded upstream-side in `yqr-b004` §6.5.

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
fall out of the existing `del` production instead of needing a rule of its own.
`Append` keeps a bare path because there is nothing to append to a comment or a
key.

This is a breaking change to the public `Mutation` enum, so it bumps the minor
pre-1.0 (`yqr-m001` §3).

### 7.2 Seam

`FidelityWriter` (`src/fidelity/write.rs`) gains one method per row of §4's
table; each is a path lowering plus one upstream call plus an error map, in the
shape `set_value` already has. Reads add one method to `FidelityEngine` for the
comment bundle; `key(...)` needs no engine support at all, since the resolved
`Path`'s last segment already holds it.

Comment and rename writes are guarded upstream (`set_inline_comment` and
friends re-parse and require the typed value to be unchanged; `rename_key`
re-parses and compares against an expected value). Reorder's guard is the one
§6 shows to be insufficient, and is the reason that slice is not simply a call.

### 7.3 Inherited limitation

Every form here lowers through `to_noyalib_path`, so every form inherits yqr's
existing refusal for keys containing `.`, `[`, `]`, or `*` — the Kubernetes
label case, which is the *other* open item in `yqr-f007` §6 and is orthogonal to
this document. `line_comment(.metadata.labels["app.kubernetes.io/name"])` will
report the unaddressable-key error until that is settled. Worth knowing before
the comment slice is demoed on a Kubernetes manifest.

## 8. What this does not decide

- **The `.`/`[` key-addressing escape** (§7.3). Orthogonal; noyalib's
  `parse_query_path` has no escape form at all, so it is an upstream grammar
  question, not a yqr one.
- **`foot_comment`.** Reserved in function position so the error can say why,
  but there is no upstream mutator and no design here.
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
- [ ] `key(<path>)` reads the key.
- [ ] Each §5 `key` row refuses with exit 5 and a message naming the reason;
      `-i` leaves the file untouched on refusal.
- [ ] `.key` still parses as a field access.

**Slice 2 — `line_comment` / `head_comment`.** Adds the second and third
selectors, `del(...)` composition, and the read path.

- [ ] Set, change, and remove, for both kinds, byte-exact elsewhere.
- [ ] `head_comment` places the block **above** the addressed entry, at the
      entry's own indent, with the document's line terminator.
- [ ] The §4.3 round-trip property holds, including for a `\n`-joined
      `head_comment` and for a comment body with leading spaces of its own.
- [ ] `= ""` writes a bare `#`; only `del(...)` removes (§4.2).
- [ ] Each §5 comment row refuses with a message naming the reason.
- [ ] Reads yield `null`, never an error, where there is no comment.

**Slice 3 — `swap` / `move`.** Blocked on §6. Ships when the upstream reorder
moves an entry's trivia with it, or with an explicit
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
  throwaway integration test: fourteen probes for the refusal catalog, then nine
  more isolating the reorder trivia behaviour across inline comments, head
  comments, one-sided comments, `move_item`, anchors, quote styles, blank lines,
  and block scalars.
- **yqr at `41041d4`** (§2.3, §4.4) — `.del`/`.key`/`.comment` parsing as
  fields, `.missing` yielding `null`, `del(.absent)` being a no-op, and the
  exact multi-target refusal text.
