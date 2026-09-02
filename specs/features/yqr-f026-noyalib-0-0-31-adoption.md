# Feature f026 — Adopt the noyalib release that carries #373: close b025 on the default path

**Status:** Draft — filed 2026-09-02, waiting on the first noyalib release
after 0.0.30
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

- [ ] The release is published; the pin moves; `Cargo.lock` shows noyalib
      moving and only what it newly requires.
- [ ] `b025` verified against the **published** crate on the field file
      (`tests/data/values.xml`): default read exit 0 and byte-identical,
      `validate` exit 0, and a write at a path outside every anchored value
      applies.
- [ ] The `parse_lossless_stream` special case and the `validate` help arm
      deleted; the two refusal tests flipped (§2).
- [ ] #338 decided (§3); `b020`'s message still names only remedies that
      work; both tests green.
- [ ] #351 handled (§4); the corpus green.
- [ ] `b026` re-checked on the new spans and updated either way.
- [ ] `b025` moved to Resolved in `yqr-b000`; the `Cargo.toml` pin comment
      and `CHANGELOG.md` say what the bump bought.
- [ ] Full suite green; `local-ci.sh` clean.
