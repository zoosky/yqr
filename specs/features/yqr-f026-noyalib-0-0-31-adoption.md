# Feature f026 — Adopt the noyalib release that carries #373: close b025 on the default path

**Status:** Done — adopted 2026-09-03 on noyalib 0.0.31, the first release
carrying #373. §6 records the adoption decisions and two behaviour changes
the release surfaced beyond the table in §1
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-b025` (the bug this closes; its §6 holds the verified
adoption diff), `yqr-f023` (the 0.0.27 adoption this follows), `yqr-b020`
(whose named remedy 0.0.29 refuses, §3), `yqr-f025` (built on that remedy),
`yqr-b026` (re-check on the new spans), `yqr-m003`

## 1. Scope

Bump `noyalib = "0.0.28"` to the first release carrying noyalib#373
(`cst::parse_document_with_config` / `cst::parse_stream_with_config`, and
a `Document` that keeps its configuration across edits), apply the
adoption change verified in `yqr-b025` §6, and close `b025`.

Unlike f020 and f023, this bump crosses two releases yqr has not adopted.
0.0.29 and 0.0.30 carry behaviour changes that land in territory yqr
tests. They were found by building yqr against the #373 branch, which sits
on 0.0.30, and each reproduces on plain 0.0.30 with yqr's code unchanged:

| upstream | release | what | yqr effect |
|---|---|---|---|
| #373 | next | the CST entry points take a `ParserConfig`; a `Document` keeps it for every re-parse of its own source | closes `b025` on the default read, `validate`, and every write |
| #338 | 0.0.29 | every mutator refuses a write into a value that live alias sites share, `set_value` included | `.base.k = 9` under `base: &m` is refused; `b020`'s named remedy stops working and two `tests/cli.rs` tests fail |
| #351 | 0.0.29 | `from_str` refuses a multi-document stream instead of returning the first document | `--normalize` fails on any multi-document input; corpus case `multidoc/classic-first-document` fails |
| commit 3e85e15 | 0.0.30 | tagged and anchored node locations anchor at the properties (marked breaking) | nothing in the suite; re-check `b026` and every located diagnostic |

## 2. The b025 adoption

The diff in `yqr-b025` §6, unchanged: one `cst_config()` in
`src/fidelity/noyalib.rs` with the ratio heuristic disabled, used by
`parse_lossless_stream`, `reparses_to`, `render_key`, the delete re-parse
guard, and `validate::check_str`. Delete the `AliasAnchorRatio` special
case in `parse_lossless_stream` and the matching `Y001` help arm in
`validate`, both unreachable once the ratio is off. Flip the two
`merge_heavy_document_*` refusal tests to expect success.

Whether `eval_ast_str` (the classic pipeline) shares that configuration
or keeps its own is a judgment call for the adopter: one function is
simpler, but the two paths have never shared parser settings and the
classic one is about to need `load_all_with_config` (§4).

## 3. #338: writes at an anchor definition

Two tests pin that `.base.k = 9` on `base: &m\n  k: 1\nc:\n  <<: *m\n`
writes at the definition. That is the one remedy `b020`'s refusal message
names ("assign where the key is defined") and the premise of `f025`. On
0.0.29 and later, noyalib's `set_value` refuses it and points at
`materialise_aliases_of`, the opposite of what the user asked for: they
want the shared value changed once, at its source.

Decide when adopting, in this order:

1. Read upstream ADR-0011 (the policy behind #338). If it offers a
   deliberate-edit path for the definition itself, use it.
2. Otherwise ask upstream whether the guard should exempt the anchor's own
   entries: a write reached *through* an alias or merge is the surprising
   one; a write at the definition is the YAML meaning of an anchor.
3. If neither lands, yqr writes the definition itself through
   `replace_span`, as it already does for structural delete (`f007`,
   `f019` §4) and for the same reason.

Whatever the outcome, `b020`'s message keeps its own rule: it names only
remedies that work.

## 4. #351: multi-document input on the classic pipeline

`eval_ast_str` parses with `from_str_with_config`, which now refuses a
stream with more than one document. The corpus expects the first document,
as before. Switch to `load_all_with_config` and evaluate the first document;
say so in the error if the stream is empty.

## 5. Acceptance criteria

- [x] The release is published; the pin moves; `Cargo.lock` shows noyalib
      moving and only what it newly requires.
- [x] `b025` verified against the **published** crate on the field file
      (`tests/data/values.yaml`): default read exit 0 and byte-identical,
      `validate` exit 0, and a write at a path outside every anchored value
      applies.
- [x] The `parse_lossless_stream` special case and the `validate` help arm
      deleted; the two refusal tests flipped (§2).
- [x] #338 decided (§3, outcome in §6); `b020`'s message still names only
      remedies that work; both tests green.
- [x] #351 handled (§4); the corpus green.
- [x] `b026` re-checked on the new spans — and fixed by the same span
      surgery §3's decision required (§6).
- [x] `b025` moved to Resolved in `yqr-b000`; the `Cargo.toml` pin comment
      and `CHANGELOG.md` say what the bump bought.
- [x] Full suite green; `local-ci.sh` clean.

## 6. Adoption record (2026-09-03)

**§2 as written.** One `cst_config()` lives in `src/fidelity/noyalib.rs`
(re-exported crate-wide from `src/fidelity/mod.rs`) and feeds every CST
parse: `parse_lossless_stream`, `reparses_to`, `render_key`, the delete
re-parse guard, and `validate::check_str`. The `AliasAnchorRatio` special
case and the `Y001` help arm are gone; the refusal tests flipped to
success, including a byte-identity read. The classic pipeline keeps its
own configuration — the two paths have never shared parser settings, and
the classic one now builds a `DocumentIterator` (§4) rather than a CST.

**§3 resolved by option 3.** Upstream ADR-0011 offers no deliberate-edit
path for the definition and does not discuss exempting the anchor's own
entries, so yqr writes the definition itself through `replace_span`
(`src/fidelity/write/anchor.rs`): resolve the value span, keep a leading
`&name` property, splice the rendered scalar (matching the slot's quote
style), and commit only when the result re-parses to the original value
with the assignment applied — at the target and, identically, at the
alias sites that share the anchored value. The `set_value` refusal is
recognized by the `materialise_aliases_of` marker it names (it is a bare
`Error::Parse`, no variant to match); the `.base.k = 9` tests pin the
marker. The option-2 question — should the guard exempt the definition's
own entries — is still worth asking upstream, but nothing here waits on
the answer; `yqr-f027` carries it, together with the span-model ask and a
typed variant for the refusal, as ready-to-file issue drafts.

**§4 as written.** `eval_ast_str` parses with `load_all_with_config` and
evaluates the first document; an empty stream is refused with "the stream
is empty" (previously `from_str` returned `null` — the corpus never
pinned that, and a loud refusal matches `validate`'s posture on empty
input).

**b026 closed by the same surgery.** The span noyalib resolves for an
anchored scalar still starts at the `&name` property on 0.0.31, so
`set_value` deleted the definition (silently with no aliases, with an
"unknown anchor" complaint otherwise). The write adapter now routes any
property-led target through the definition write above, which skips the
property: `.a = 2` over `a: &x 1` yields `a: &x 2` and every alias
follows. The tagged case is settled as a refusal naming the tag.

Two behaviour changes beyond the §1 table, both re-baselined with a
comment at the test:

- The emitter drops quotes a plain scalar does not need
  (`"6.7.0-RC.5-2eb4505e"` now emits unquoted); values are unchanged and
  round-trip. Affected `--normalize` expectations updated.
- `set_value` accepts a flow-collection target (`imageTag: {}`), which
  0.0.28 refused; block-collection targets already wrote, so this lifts a
  gap rather than changing a contract. The corpus case flipped from a
  refusal to the rewrite.
