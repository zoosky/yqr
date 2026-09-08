# Feature f032 — Collection right-hand sides

**Status:** Done — shipped 2026-09-08
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f007` §6 (where this was the last open scope item),
`yqr-a002` §8 (which declined to decide it), `yqr-b008` (the typed `Emit`
tier that made it possible), `yqr-b029` and `yqr-b030` (two defects the
measurement found), `yqr-f025` (a refusal names a remedy that works),
`yqr-m003`

## 1. The limit

`.a = .b`, `.m.new = .b` and `.xs += .b` refused whenever `.b` named a
mapping or a sequence:

```console
$ yqr '.m.new = .src' config.yaml
yqr: runtime error: the right-hand side of '+=' or a new-key assignment must be
a scalar (number, string, boolean, or null); collections are not yet supported
```

One predicate, `insertable`, enforced it at all three write sites. It was
a **scope** limit, not a backend one, and had been since `yqr-b008`: the
typed tier the write path uses has spelled a nested collection since
noyalib#223. `yqr-f007` §6 recorded that and left it as the last open item
on the epic once `yqr-f030` closed dotted-key addressing.

There is no collection literal in the grammar, so the right-hand side is
always a path (`= .b`) or a filter's result (`|= to_entries`). That is not
a limitation of this feature; it is what `yqr-a003` scopes out of the
language.

## 2. What upstream does, measured

Every write site was driven with a collection value on the pinned noyalib
0.0.41 before any refusal was lifted, over block and flow targets, at
depth, on CRLF, and with values the emitter has to think about.

| site | call | result |
|---|---|---|
| new key | `insert_entry_value` | writes the block at the site's own indent, with the load-back oracle |
| `+=` | `push_back_value` | writes a mapping or a nested sequence as an item; into a flow sequence as a flow member |
| `=` / `\|=` over a **collection** | `set_value` collection arm (#328) | replaces the block, keeps the key's own inline comment and everything after it |
| `=` / `\|=` over a **scalar** | `set_value` | **refuses**: "cannot replace a scalar with a collection (use `set` with a fragment)" |

Values inside a copied collection are the emitter's business and it gets
them right: `007` comes back quoted, a multi-line string becomes a block
scalar, a dotted key stays plain, `null`/`true`/`1.5` keep their types.

Two things it gets wrong, both found here and both filed as bugs of their
own because neither is about collections:

- **`yqr-b029`** — a *scalar* written over a block collection lands at the
  key's own column, giving `k:` a value line PyYAML and Psych reject. Live
  on the shipped 0.8.0 at exit 0.
- **`yqr-b030`** — a multi-line replacement's own lines end with LF whatever
  the document uses, so a CRLF file gains mixed endings. Live on the
  shipped 0.8.0 through multi-line **string** assignment; the collection
  arm inherits it.

## 3. Design

`insertable` no longer refuses; it is the `Value` → `noyalib::Value`
conversion and nothing else. Three checks take its place, each where it
has what it needs.

**A scalar cannot become a collection** (`refuse_scalar_to_collection`).
This is the one shape upstream has no typed route for, and its own message
names `set` and "fragment", an API yqr does not expose. The check sits in
`set_value_unless_unchanged`, the one caller holding both the current
value and the new one — `=` and `|=` reach it alike. The remedy it names is
the one that works, and the test runs it: remove the entry, then assign,
because a *new key* is the insertion path and that one spells a
collection.

**A block collection cannot become a scalar** (`yqr-b029`). Refused before
the document is touched, so the message is yqr's for every shape rather
than for the flat ones only.

**The result must not be structurally worse than the document it started
from** (`check_integrity`). Measured on either side of the write:

| property | why the existing guards miss it |
|---|---|
| `Y103` sites, a block mapping value at its key's own column | the result parses **for this parser**, so the re-parse guard sees nothing. This is `yqr-b014`'s class, and `validate` already had the scanner |
| line feeds with no carriage return before them | a line ending is not structure at all |

Returning `Err` is what undoes the write: `apply` emits nothing unless
every document succeeded. The rule keys on a *worsening*, so a document
that already carries a violation stays editable, and it holds a document
to its line endings only when it is wholly CRLF — an all-LF file
legitimately gains a line feed per inserted line.

The pre-checks give the message; the post-check is the net. That is the
division `yqr-a002` §9 arrived at for comments, for the same reason: a
list of shapes to refuse is upstream's to change, and a property is not.

### 3.1 What a copied collection carries

The right-hand side is a **value**, so the block yqr writes is the
engine's spelling of it, not the source's bytes. Comments inside the
copied collection do not travel, quote style is chosen at the destination,
and the block is laid out at the destination's indent. This is the same
rule `to_entries` output follows (`yqr-f017` §6) and it is the honest one:
the pairs a filter computes exist in no file, and neither does a value
lifted out of one and put somewhere else. Every byte outside the edit is
untouched, which is the guarantee `a001` actually makes.

## 4. Coverage

- **Unit** (`src/fidelity/write.rs`): a mapping and a sequence into a new
  key, a mapping appended as a sequence item, a replacement at depth with
  the tail comment intact, the no-op guard holding for an equal collection,
  and the scalar refusal with its remedy executed. Plus `yqr-b029`'s three
  shapes and the two it must leave alone, and `yqr-b030`'s refusal with the
  paths that stay correct.
- **CLI** (`tests/cli.rs`): `|= to_entries` writes the pairs; a collection
  over a scalar refuses and the named remedy is run.
- **Corpus** (`yqr-m003`): three writes on the deployment — a new key, an
  append, a replacement — and three refusals: a collection over a scalar,
  a scalar over a block collection, and a multi-line write into the CRLF
  document. Two command-line cases on the tenants shape, one `-i` write
  copying a tenant's block and one refusal leaving the file untouched.
- **Flipped**: `write/update/refuses-a-collection-result` becomes
  `write/update/collection-result-over-a-collection`, and the CLI test that
  pinned `|= to_entries` as a refusal now pins its output. Both were
  correct records of the old scope.

## 5. Not done here

- **A collection literal** (`{a: 1}`, `[1, 2]` in a filter). That is
  language work, not write-tier work: `yqr-a003` scopes it out, and every
  form here takes its value from the document.
- **Replacing a scalar with a collection in place.** Refused, §3. It needs
  a fragment write with an integrity oracle, which is `yqr-b029`'s route
  too; if either is built, both are.
- **Merging rather than replacing.** `=` replaces. There is no `*=` or
  deep-merge form and this feature does not propose one.

## 6. Acceptance criteria

- [x] A mapping and a sequence can be assigned to a new key, appended with
      `+=`, and written over an existing collection with `=` and `|=`.
- [x] Every other byte of the document is unchanged, pinned on a real
      manifest by the corpus write tier.
- [x] A collection over a scalar is refused in yqr's words, and the remedy
      the message names is executed by a test.
- [x] `yqr-b029` and `yqr-b030` filed, guarded, and covered.
- [x] Guide, README and `CHANGELOG.md` updated; `yqr-f007` §6 and
      `yqr-a002` §8 record the item as closed.
- [x] Full suite green; `cargo clippy --all-targets --all-features -D
      warnings` clean; `local-ci.sh` clean.
