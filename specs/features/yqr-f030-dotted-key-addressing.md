# Feature f030 — Address any mapping key: bracket-quoted segments and the `."a.b"` field

**Status:** Done — shipped 2026-09-07 on noyalib 0.0.39
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f028` §4 (the upstream half, noyalib 0.0.33 #388),
`yqr-f007` §6 and §7.3 (where the limit was tracked), `yqr-a002` §5.3, §5.5,
§7.3 and §8 (the grammar spec that inherited it), `yqr-m002` (the seam this
changes), `yqr-b012` (the insert half, fixed in noyalib 0.0.25), `yqr-m003`
(the corpus)

## 1. The limit

A mapping key holding `.`, `[`, `]` or `*`, or the empty key, could not be
written or named. `to_noyalib_path` returned `None` for it, the read path
reported `Resolved::Unaddressable` and fell back to the typed value, and
every write verb refused with "cannot address key". The Kubernetes label
block is the everyday case: `app.kubernetes.io/name` was readable through
`.["app.kubernetes.io/name"]` only, and the guide listed changing, deleting
or renaming it under "what is not here yet".

The cause was in the engine's path grammar. noyalib addresses a node through
a string path, `a.b[0].c`, and until 0.0.33 that grammar had no way to spell
a key that contains its own metacharacters. yqr's `PathSeg::key_is_plain`
existed to turn that gap into a deterministic refusal instead of a silent
mis-resolution (`yqr-m002` §3.5).

noyalib 0.0.33 (#388, adopted in `yqr-f028`) added bracket-quoted key
segments to the grammar — `labels["app.kubernetes.io/name"]`, either quote
style, `\` escaping the next character — and three helpers:
`path::quote_key`, `path::push_key` and `path::join_keys`. `f028` §4
recorded that the remaining half was yqr's. This is that half.

## 2. What was measured first

Every `cst::Document` call yqr makes was driven with a quoted path on the
pinned 0.0.39 crate before any refusal came out, because "upstream has the
call" and "upstream has yqr's semantics" are different questions
(`yqr-f007` §5.1). One document with a dotted key, a `*` key, the empty key,
a key holding `"` and one holding `\`, and a second document with a sequence
and a mapping *below* a dotted key.

| call | result on a quoted segment |
|---|---|
| `span_at`, `key_span`, `comments_at` | the node's own bytes, key token and comments, for every key including `""` |
| `set_value` | the value on the dotted key's line, inline comment kept |
| `remove` | the entry and its line |
| `rename_key` to a dotted key | the key token only |
| `rename_key` to `""` where `""` exists | refused as a duplicate, so upstream's collision check sees the empty key |
| `insert_entry_value` with a dotted key | appended, spelled plain like its neighbours |
| `set_inline_comment`, `remove_inline_comment`, `set_leading_comment`, `remove_leading_comment` | as on a plain key |
| `swap_items` on a sequence below a dotted key, `insert_entry_value` into a mapping below one, `insert_after_value` below one | as on a plain path |
| `push_key` | quotes exactly the keys `key_is_plain` rejected (its predicate is the same four characters and the empty key) and nothing else: `a b/c` and `-x` stay plain |

Nothing needed a workaround. The measurement is what justified making the
lowering total rather than adding a second predicate beside the first.

## 3. Design

Three changes, each removing something rather than adding a branch.

**Lowering is total.** `to_noyalib_path` (`src/fidelity/noyalib.rs`)
returns `String`, composing every `PathSeg::Key` with `push_key` and every
`PathSeg::Index` as `[i]`. yqr never composes a segment by hand, so it
cannot disagree with the engine about what one means — the property the
old predicate protected, now supplied by the engine's own helper. Every
caller that had a `let Some(path) = ... else { return ... }` guard lost it:
`resolve`, `comment_body`, `key_bytes`, `borrowed_site`, `current_comment`,
`delete_entry`, the reorder path, and the write adapter's `noyalib_path`
wrapper, which is gone.

**The grammar gains `."a.b"`.** The parser accepts a string token where it
accepts an identifier after a dot, at the head of a path and in the chain
that follows a path or a builtin. It is the same `Ast::Field` as
`.["a.b"]`, so nothing downstream changes. The parse error for a stray dot
now says "expected a field name, a quoted key or '['".

**The seam loses an arm.** `Resolved::Unaddressable`, the `Unaddressable`
enum, `PathSeg::is_plain` and `PathSeg::key_is_plain` are removed. A
variant no backend can produce is a promise the type makes and the code
does not keep. `Resolved` has three arms; `yqr-m002` is updated to match.

**The empty key.** `yqr-a002` §5.3 refused `key(.a) = ""` on one argument:
no yqr filter could name the result, so the addressable set would not be
closed under rename. `.[""]` names it — the parser accepted the empty
string in brackets already, and the engine resolves `[""]` — so the
argument is gone and the check with it. The closure property still holds
and now has a test: rename to `""`, then assign through `.[""]`.

### 3.1 What changed for users

- A read of a dotted key emits the node's own bytes, quotes included,
  where it used to render the typed value: `'a.b': 'x'` under `.["a.b"]`
  prints `'x'`, not `x`. This is the fidelity contract, applied to one more
  node class.
- `key(...)` on a dotted key reads its token where it read `null`
  (`yqr-a002` §9, slice 1, second note).
- Every write verb reaches a dotted key, including `-i`.
- `."a.b"` parses.

## 4. Coverage

- **Unit:** six new engine tests (`src/fidelity/noyalib.rs`) — a dotted, a
  `*` and a bracketed key resolve; the empty key at the root and nested; a
  key holding `"` and one holding `\`; a path below a dotted key; the
  lowering quotes only what the plain spelling would misread; `key_bytes`
  of a dotted key is its token. Four on the write path — assignment in
  place with an inline comment kept, creation of a dotted key beside dotted
  keys, rename to a dotted key and back through `."..."`, rename to the
  empty key and back through `.[""]`. One reorder below a dotted key. Five
  parser tests and one lexer test for `."a.b"`, including the chain, the
  builtin chain, a mutation target and the stray-dot refusal.
- **Corpus (`yqr-m003`):** one classic case (`."..."` on the deployment),
  two engine cases (`key(...)` of a dotted key reads its token; the quoted
  field on the engine path), seven write cases on the deployment covering
  assign, the quoted-field spelling, insert, delete, rename, inline and
  head comment, and two CLI cases on the production values file (a read
  and an `-i` write of a Helm-style dotted key).
- **Removed:** the three tests that pinned the refusal (`unaddressable_key_is_reported`,
  `rename_refuses_an_empty_key_as_yqrs_own_precheck`,
  `rename_refuses_a_key_the_path_grammar_cannot_address`), the reorder
  refusal test, and `plain_segments`; `special_char_key_degrades_to_typed_rendering`
  became `special_char_key_reads_its_source_bytes`.

## 5. Not done here

- **Collection right-hand sides** (`yqr-f007` §6) — unrelated to
  addressing; still open.
- **Non-string keys** (`1: x`, `true: y`). yqr's path model holds keys as
  strings and the typed lookup goes by string, which is unchanged by this
  feature.
- **A key holding a newline.** Upstream refuses a rename to one, naming the
  code point; yqr forwards that as before.

## 6. Acceptance criteria

- [x] `to_noyalib_path` is total and composes keys with `push_key`.
- [x] `."a.b"` parses to the same AST as `.["a.b"]`, at the head of a path,
      after any later dot, and after a builtin's chain.
- [x] A dotted key is read as its own bytes and reached by assign, insert,
      `del`, `key(...)`, `line_comment`, `head_comment`, `swap` and `move`,
      each pinned in the corpus on a genuine document.
- [x] `key(...)` on a dotted key reads its token.
- [x] `Resolved::Unaddressable`, `Unaddressable`, `PathSeg::is_plain` and
      `PathSeg::key_is_plain` are removed; `yqr-m002` reflects the
      three-arm seam.
- [x] The empty key is addressable through `.[""]` and a rename to it
      round-trips.
- [x] Guide, jq comparison, front page table, README and `CHANGELOG.md`
      updated; `yqr-f007`, `yqr-f028` and `yqr-a002` carry resolution notes.
- [x] Full suite green; `cargo clippy --all-targets --all-features -D
      warnings` clean; `local-ci.sh` clean.
