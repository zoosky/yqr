# Bug b025 — The alias-to-anchor ratio heuristic refuses a legitimate values file, and the refusal reads as a syntax error

**Status:** In Progress — the classic pipeline (`--normalize`) now parses
these files and every refusal explains itself, but the default
byte-preserving read still fails: noyalib's CST entry points hardcode their
parser configuration, so the fix for that half is upstream. Filed
2026-09-02 from a field report
**Severity:** Medium — a valid, ordinary production file cannot be read at
all on the default path, and the message implied the file was broken
**Component:** upstream `noyalib::cst::parse_document` /
`parse_stream` (hardcoded `ParseConfig::default()`); yqr surfaces it in
`src/fidelity/noyalib.rs`, `src/lib.rs::eval_ast_str`, and
`src/validate/mod.rs`
**Related:** `yqr-b022` (the precedent: `validate` calling valid YAML
invalid is the worst face of a parser defect), `yqr-b024` (an upstream
message reaching the user with a misleading reason), `yqr-f023` §4 (the
upstream patch workflow), research `yqr-r002`

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

When either ships, pin the release, delete the `parse_lossless_stream`
special case, and flip
`merge_heavy_document_default_read_names_the_heuristic_and_the_workaround`
(tests/cli.rs) to expect success.

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
