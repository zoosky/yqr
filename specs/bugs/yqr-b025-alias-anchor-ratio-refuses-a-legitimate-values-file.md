# Bug b025 — The alias-to-anchor ratio heuristic refuses a legitimate values file, and the refusal reads as a syntax error

**Status:** Resolved — closed 2026-09-03 by `yqr-f026`, which adopted
noyalib 0.0.31 (the first release carrying noyalib#373, filed for this bug
on 2026-09-02, the day the bug itself was filed). Every path — the default
byte-preserving read, `validate`, and the write tier — now parses with the
ratio heuristic disabled and the absolute budgets intact; verified against
the published crate on the field file (`tests/data/values.yaml`): default
read exit 0 and byte-identical, `validate` exit 0, writes apply. The
special-cased refusal wording in `parse_lossless_stream` and the `Y001`
help arm are gone, being unreachable
**Severity:** Medium — a valid, ordinary production file cannot be read at
all on the default path, and the message implied the file was broken
**Component:** upstream `noyalib::cst::parse_document` /
`parse_stream` (hardcoded `ParseConfig::default()`); yqr surfaces it in
`src/fidelity/noyalib.rs`, `src/lib.rs::eval_ast_str`, and
`src/validate/mod.rs`
**Related:** `yqr-b022` (the precedent: `validate` calling valid YAML
invalid is the worst face of a parser defect), `yqr-b024` (an upstream
message reaching the user with a misleading reason), `yqr-f023` §4 (the
upstream patch workflow), `yqr-f026` (the adoption), `yqr-b026` (found
while verifying this one), research `yqr-r002`

## 1. Summary

A Helm-style tenants values file — 22 anchored default blocks, merged into
221 tenant entries with `<<: *anchor` — is valid YAML that every other
consumer in the pipeline reads. yqr refused it outright:

```console
$ yqr -r '.' values.yaml
yqr: io error: failed to parse YAML input: alias_anchor_ratio heuristic tripped: 221 aliases / 22 anchors > 10
```

The trip is noyalib's billion-laughs fingerprint heuristic: when resolved
aliases exceed `ratio × anchors` (default ratio 10.0), the parse aborts.
221 / 22 = 10.05. One more tenant than ten per default block, and the file
stops opening. The threshold is exact — the same file with 220 aliases
parses.

Two defects, one upstream and one in the wording:

1. **The heuristic mis-fires on the normal case, and the CST path cannot
   tune it.** Reusing each anchor more than ten times is not an attack
   fingerprint; it is what anchors are for, and merge-key-heavy
   Kubernetes/Helm values files do it routinely. noyalib makes the ratio
   configurable (`ParserConfig::alias_anchor_ratio`, `None` disables) and
   yqr's classic pipeline can use that — but `noyalib::cst::parse_document`
   and `parse_stream`, the entry points behind yqr's default byte-preserving
   engine, its write path, and `validate`, construct
   `ParseConfig::default()` internally and take no configuration
   (confirmed in 0.0.28 and 0.0.29).
2. **The refusal reads as a syntax error.** "failed to parse YAML input"
   plus jargon (`alias_anchor_ratio heuristic tripped`) sends the user
   hunting for a defect in a correct file. `validate` was worse: it
   reported `Y001` — the syntax-error code — on the same input, the exact
   face `yqr-b022` called the worst one, because that command's whole job
   is to answer whether a file is correct.

## 2. Why disabling the ratio loses no protection

The ratio is a heuristic layered over absolute budgets that bound
amplification directly, all of which stay in force:

- `max_alias_expansions` (1024): total aliases resolved across the
  document — the counter increments per alias, so 1025 aliases refuse
  regardless of anchor count.
- `max_events` (1,000,000), `max_nodes` (250,000),
  `max_total_scalar_bytes` (64 MB): cap the event stream, the authored
  tree, and the post-expansion scalar payload — the quantities a
  billion-laughs attack actually inflates.
- `max_merge_keys` (10,000), `max_depth` (128), `max_document_length`
  (64 MB).

A document that passes all of those but fails the ratio is, in every case
measured, an ordinary file with well-reused anchors. The heuristic's only
marginal contribution is refusing amplification *earlier* than the absolute
caps would; its cost is refusing legitimate files forever.

## 3. What yqr shipped now

- **`src/lib.rs::eval_ast_str`** (the classic pipeline, `--normalize`):
  parses with `from_str_with_config` and the ratio heuristic disabled.
  Every absolute budget stays at its default; a test pins that 1025
  aliases still refuse on the alias-expansion cap.
- **`src/fidelity/noyalib.rs::parse_lossless_stream`** (shared by the
  default read engine and the write path): when the CST parse fails on
  exactly this breach, the error now says it is a parser resource
  heuristic, not a YAML syntax rule, and points at `--normalize` for value
  queries.
- **`src/validate/mod.rs`**: the `Y001` diagnostic for this breach carries
  a `help` line saying the same. It still exits 1 — `validate` cannot
  certify a file its parser did not finish reading.

Interim workaround for affected files: value queries work with
`--normalize` (output is re-serialized: comments dropped, scalars
canonicalized). Byte-preserving reads and all writes remain refused until
the upstream fix lands.

Deliberately not done: falling back from the CST engine to the classic
pipeline on this error. A silent engine swap would change output formatting
depending on input shape, against the f009 contract that the default read
is byte-preserving or loudly refused.

## 4. Upstream fix needed (noyalib)

File against `noyalib` (the CST configuration gap; the heuristic default is
a second, softer ask):

1. **Configurable CST entry points** — `parse_document_with_config` /
   `parse_stream_with_config` taking the existing `ParserConfig` (or
   `ParseConfig`), mirroring `from_str_with_config`. This is the complete
   fix: yqr then disables the ratio on every path and drops the special
   case in `parse_lossless_stream`.
2. **Reconsider the default** — exclude merge-key aliases from the
   numerator, or raise/drop the default ratio, since the absolute budgets
   (§2) already bound amplification. Benefits every downstream consumer,
   including the ones that never discover the config.

Filed 2026-09-02 as noyalib#372; PR noyalib#373 implements item 1 and
raises item 2 as a question for the maintainer. The PR adds
`cst::parse_document_with_config` / `cst::parse_stream_with_config`
taking a `ParserConfig`, and makes a `Document` keep the configuration it
was opened with for every re-parse of its own source: the typed-cache
refresh after a local-repair edit, `validate`, the `replace_span` safety
net, the comment-edit value guard, and the schema-coercion snapshot. That
second half is what makes the API usable for edits. Without it a document
that only opens under a relaxed budget opens, accepts its first `set`, and
panics on the next read, because the cache refresh re-parses under the
defaults and treats the refusal as a broken local-repair invariant. Eleven
upstream tests pin it, including that `max_alias_expansions` still refuses
1025 merges with the ratio disabled.

When a release carries it: `yqr-f026`. The adoption diff is in §6, already
verified against the branch.

## 5. Reproduction

```console
$ python3 - <<'EOF'
anchors, aliases = 22, 221
lines = ["defaults:"]
for i in range(anchors):
    lines += [f"  d{i}: &a{i}", f"    k: v{i}"]
lines.append("tenants:")
for j in range(aliases):
    lines += [f"  t{j}:", f"    <<: *a{j % anchors}"]
open("trip.yaml", "w").write("\n".join(lines) + "\n")
EOF
$ yqr -r '.tenants.t0.k' trip.yaml        # refused (exit 5), message names the workaround
$ yqr --normalize -r '.tenants.t0.k' trip.yaml
v0
```

Pinned by `classic_pipeline_accepts_a_merge_heavy_document` and
`classic_pipeline_keeps_the_absolute_alias_budget` (tests/integration.rs),
and the three `merge_heavy_document_*` tests (tests/cli.rs).

## 6. Verification against the upstream branch (2026-09-02)

yqr was built once against the #373 branch (`[patch.crates-io]` on the
local checkout, pin bumped to 0.0.30 to satisfy the version requirement)
with the adoption change below applied, and run over the field file itself
(`tests/data/values.xml`, 282 KB, the 22-anchor / 221-alias tenants
file). Nothing from that build is committed; the tree went back to the
0.0.28 pin afterwards.

| command | on 0.0.28 | on the branch |
|---|---|---|
| `yqr '.' values.xml` | exit 5, ratio message | exit 0, output byte-identical to the input |
| `yqr -r '.preImage' values.xml` | exit 5 | `6.7.0-RC.5-2eb4505e` |
| `yqr validate values.xml` | exit 1, `Y001` | exit 0 |
| `yqr --normalize '.' values.xml` | exit 0 | exit 0 |

The write path opens the file too, but no write was verified on it: both
paths tried sit inside anchored values, which 0.0.29 refuses for an
unrelated reason (`yqr-f026` §3). The edit-after-open case is pinned
upstream instead, by #373's own tests.

The adoption change, kept out of the tree until the release ships:

- `src/fidelity/noyalib.rs`: one `cst_config()` returning
  `ParserConfig::new().alias_anchor_ratio(None)`, re-exported from
  `src/fidelity/mod.rs`; `parse_lossless_stream` calls
  `parse_stream_with_config` with it and loses the `AliasAnchorRatio`
  special case; `reparses_to` calls `parse_document_with_config`.
- `src/fidelity/mod.rs::render_key` and the re-parse guard in
  `src/fidelity/write/delete.rs`: the same, so an edited merge-heavy
  document re-parses under the limits it was opened with.
- `src/validate/mod.rs::check_str`: `parse_stream_with_config`; the
  `Y001` help arm for the ratio becomes unreachable and goes.
- `src/lib.rs::eval_ast_str` keeps its own disabled ratio; sharing one
  configuration between the classic and CST paths is f026's call.
- `tests/cli.rs`: the two `merge_heavy_document_*` refusal tests flip to
  exit 0 and `v0`.

On that build the b025 tests pass and clippy is clean. Three other tests
fail, and all three reproduce on plain 0.0.30 with yqr's code unchanged,
so they are 0.0.29/0.0.30 behaviour changes yqr has not adopted, not
effects of the PR. `yqr-f026` §1 lists them with their upstream numbers:
the two merged-key write tests in `tests/cli.rs` (noyalib#338 refuses a
`set_value` at an anchor definition) and the corpus case
`multidoc/classic-first-document` (noyalib#351 makes `from_str` refuse a
multi-document stream).
