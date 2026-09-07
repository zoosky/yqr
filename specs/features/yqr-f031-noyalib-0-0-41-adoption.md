# Feature f031 — Adopt noyalib 0.0.41: two fixes on paths yqr does not take

**Status:** Done — adopted 2026-09-07
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f029` (the 0.0.39 adoption this follows), `yqr-f030`
(shipped the same day on 0.0.39), `yqr-m003` (the corpus, which gains a
document), `yqr-m002` §6 (where tags leave the pipeline)

## 1. Scope

Bump `noyalib = "0.0.39"` to `0.0.41`, two releases published the same
day, and verify yqr against the published crate. `Cargo.lock` moves
noyalib and nothing else.

| upstream | release | what | yqr effect |
|---|---|---|---|
| fcd962a | 0.0.40 | `ser`: a tag introduced by `%TAG` is held as a bare URI; the emitter wrote it bare, so the document read back as a plain scalar. It is now written verbatim, `!<uri>` | none: yqr's `--normalize` emits yqr's own `Value`, and `lower_value` drops a tag before the emitter sees it (§3) |
| d796cd0 | 0.0.40 | `cst::format`: a mapping used as an explicit key had its continuation lines emitted at the outer indent; a keep-chomped scalar gained a newline per format | none: yqr does not call the formatter (§3) |
| fdae72d | 0.0.40 | tests for the six "index out of bounds" messages of the CST editing API; no code change | none: yqr's own range check runs first (`yqr-a002` §9 slice 3), measured unchanged (§2) |
| — | 0.0.40 | corpus tests: every parsed fixture survives the serializer and the formatter; the streaming reader agrees with the batch loader across the suite; diagnostics checked through both loaders | none |
| — | 0.0.41 | lockstep GitHub Actions bumps; no core code change | none |

The bare bump is green: the full suite passes without a single
expectation moving.

## 2. Measured

Same inputs through a `main` build on 0.0.39 and this branch on 0.0.41,
comparing stdout, stderr and exit code. Four documents built for the
release notes' shapes — a `%TAG` directive with a resolved, a `!!str` and
a local tag; a mapping as an explicit key with a keep-chomped scalar
beside it; keep- and strip-chomped scalars with entries after them; a
two-item sequence for out-of-range indices — plus the corpus deployment
and the production values file.

| shape | filters | result |
|---|---|---|
| `%TAG` document | `.`, `--normalize .`, `validate --strict`, `.foo`, `-r .plain`, `--normalize .foo`, `.local = "y"` | identical; the write is refused on both with yqr's own tagged-scalar guard ("rewriting the scalar under it could change what the tag makes of it"), exit 5 |
| explicit mapping key, `\|+` beside it | `.`, `--normalize .`, `validate --strict`, `.k`, `.next = 2`, `line_comment(.next) = "c"`, `to_entries[].key`, `--normalize .k` | identical |
| keep- and strip-chomped scalars | `.`, `--normalize .`, `validate --strict`, `.b = 2`, `del(.a)`, `.a`, `head_comment(.b) = "..."`, `.c = "x"`, `key(.a) = "z"`, `--normalize .a` | identical |
| out-of-range indices | `swap(.xs; 0; 5)`, `move(.xs; 0; 5)`, `.xs[5] = "c"`, `del(.xs[5])`, `.xs[5] \|= .`, `line_comment(.xs[5]) = "c"`, `key(.xs[5]) = "z"`, `.xs[-3]`, `swap(.xs; 0; -3)` | identical: the reorders refuse (exit 5) from yqr's range check, the others are absent-path no-ops or `null` |
| the deployment | ten writes: scalar assign, nested assign, mapping delete, item delete, append, self-swap, rename, line comment, dotted-key assign, a refused head comment | byte-identical |
| the values file | `.`, `--normalize .`, `validate --strict`, `.preImage = "x"` | byte-identical |

44 comparisons, no difference.

## 3. Why the fixes cannot reach yqr

Both fixes are real and both are behind a boundary yqr's pipeline does
not cross.

**Tags.** `--normalize` re-serializes yqr's own `Value` (`src/value.rs`),
not noyalib's. `lower_value` (`src/fidelity/noyalib.rs`) maps
`noyalib::Value::Tagged(t)` to the lowering of `t.value()`, so no tag
exists by the time anything is emitted: `--normalize .` on the `%TAG`
document prints `foo: baz`, `plain: "007"`, `local: x` on both builds.
The default read never emits at all; it slices the document's bytes, and
a `%TAG` document round-trips byte for byte on both builds. A write under
a tag is refused before any splice by yqr's own guard, measured above.

**The formatter.** `cst::format` is noyafmt's entry point. yqr's emit
primitives are `Document::source()` slices on read and the typed
mutators plus `replace_span` on write; `grep` finds no call into the
formatter. `yqr-f029` recorded the same fact for the 0.0.38 formatter
fix.

So the adoption changes no behaviour. What it adds is a pin: a corpus
document (`TAG_DIRECTIVE_EXPLICIT_KEY`, `tests/corpus/docs.rs`) holding a
`%TAG` directive, a mapping as an explicit key and a keep-chomped scalar,
read byte for byte on the engine path and through `.foo` on the classic
one. If a later release moves either fix onto the emitter yqr does use,
or the CST parse of these shapes changes, the identity case fails
instead of the change passing silently — the `yqr-m003` rule that a
shape is pinned as it behaves.

## 4. Benchmarks

`cargo bench --bench eval`, the target `benchmark.yml` tracks, 0.0.39 on
the `main` worktree against 0.0.41 on this branch, back to back on the
same machine.

| benchmark | 0.0.39 | 0.0.41 | change |
|---|---|---|---|
| `parse/nested_path` (filter parse, no YAML) | 499.7 ns | 501.0 ns | none |
| `eval_str/field_access` | 3.713 µs | 3.596 µs | -3.2%, within run-to-run spread |
| `eval_str/iterate_100` | 148.2 µs | 149.0 µs | within noise |

Nothing to justify; neither release touched the parser's hot path.

## 5. Acceptance criteria

- [x] The pin moves to `0.0.41`; `Cargo.lock` shows noyalib moving and
      nothing else.
- [x] Full suite green on the bare bump, no expectation moved.
- [x] Every shape the release notes name compared against the 0.0.39
      build (§2): identical.
- [x] The three shapes pinned in the corpus (§3).
- [x] Benchmarks compared (§4).
- [x] `Cargo.toml` pin comment and `CHANGELOG.md` say what the bump
      bought; tracker updated; `target/doc-md` regenerated for 0.0.41.
- [x] `cargo clippy --all-targets --all-features -D warnings` clean;
      `local-ci.sh` clean.
