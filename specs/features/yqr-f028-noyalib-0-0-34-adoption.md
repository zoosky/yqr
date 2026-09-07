# Feature f028 — Adopt noyalib 0.0.34: the located key collision, and the stream position it exposed

**Status:** Done — adopted 2026-09-06
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f026` (the 0.0.31 adoption this follows), `yqr-b028`
(the bug this found and fixed), `yqr-f007` §6 and `yqr-a002` §7.3 (the
dotted-key limit 0.0.33 unblocks upstream), `yqr-m003`

## 1. Scope

Bump `noyalib = "0.0.31"` to `0.0.34`, three releases, and verify yqr
against the published crate. `Cargo.lock` moves noyalib and nothing else.

| upstream | release | what | yqr effect |
|---|---|---|---|
| #375 | 0.0.32 | `span_at` for a block sequence in value position reports the sequence's full extent (marked breaking on the spans axis) | none: the corpus write tier is green, and nine sequence writes on one document (append, item assign, item delete, whole delete, flow delete, swap, and the three anchored-sequence refusals) are byte-identical to 0.0.31 |
| #378 | 0.0.33 | `Error::KeyCollisionAt` and `Error::DuplicateKeyAt`, the located forms; the CST parser raises them | `validate` matched the bare `KeyCollision` only, so a collision degraded to a generic `Y001` — the two failures the suite reported. §2 |
| #388 | 0.0.33 | bracket-quoted key segments in the path grammar, and `noyalib::path::{quote_key, push_key, join_keys}` | none yet; the upstream half of the dotted-key limit, §4 |
| #383, #385, #386, #387 | 0.0.33 | block scalars the serializer wrote in a shape its parser read back differently | `--normalize` output changes, §3; no pinned expectation moved |
| — | 0.0.33 | the parser drops a per-key clone and a per-mapping vector (upstream: 20.2 MB to 16.6 MB, 180k to 150k allocations on its corpus) | §5 |
| — | 0.0.34 | an unterminated verbatim tag (`!<` with no `>`) is refused; `arbitrary` feature; fuzz targets; CI | none: yqr passes the parser's answer through |

## 2. The adoption diff

`syntax_diagnostic` matches both collision variants and takes the position
from the located one, so `error[Y102]` points at the colliding key like
every other syntax finding. Doing that on the existing stream test showed
line 3 for a collision on line 6, which is `yqr-b028`: the parser's
locations count from the failing document, not the stream, and `validate`
had rendered them unoffset since `f012` — for every located error, not
only this one. The fix is in that spec; its effect here is that the
collision's document note comes from the same lookup, and the old
re-parsing `collision_document_note` is gone.

Tests: the two failures flipped (`key_collision_is_a_y102_by_default_with_document_note`
now asserts the position; `validate_key_collision_is_reported_by_default`
asserts the rendered `:2:1`), three stream tests re-baselined from `3:3`
to `3:7` with the reason at the assertion, and three new tests (stream
positions, the marker rule, the CLI stream case).

## 3. Emitter changes, measured

`--normalize` re-serializes; the byte-preserving read and every write keep
the file's layout and are unaffected. Values are unchanged in every row
(each output parses back to the input's value on both versions).

| input | 0.0.31 wrote | 0.0.34 writes |
|---|---|---|
| `a: \|\n  text\nb: 1\n` | a blank line between the block scalar and `b` | no blank line (#385) |
| `a: \|+\n  text\n\nb: 1\n` | two blank lines: the value grew by a newline per round trip | one, the value's own (#385) |
| `list:\n  - \|\n    body\n  - x\n` | body two indent steps past the dash | one step, the column an indentation indicator counts from (#387) |
| `a: "\n"\nb: 1\n` | `a: \|` followed by two empty lines, which read back as `""` | `a: \|+` and one empty line (#383) |
| `a: " \nx"\n` | `\|2-` with the space-only line | unchanged |

## 4. Dotted keys: what 0.0.33 unblocks

`to_noyalib_path` (`src/fidelity/noyalib.rs`) still returns `None` for a
key that is not plain, which is the `yqr-f007` §6 / `yqr-a002` §7.3
addressing limit: a key holding `.`, `[`, `]` or `*` cannot be written or
named. Before 0.0.33 that was upstream's limit too. Now `path::quote_key`
spells any key in a form every noyalib mutator and locator reads back, so
the remaining half is yqr's: the filter grammar has no quoted-key form
(`."a.b"` is a parse error, exit 3), and `to_noyalib_path` would compose
segments with `push_key` instead of `.`. That is a feature of its own on
`yqr-a002`'s grammar, not part of an adoption; it is recorded here so the
next reader of §7.3 knows the upstream side is done.

**Done 2026-09-07 as `yqr-f030`:** `to_noyalib_path` composes with
`push_key` and is total, `."a.b"` parses, and the `Unaddressable` arm of
the seam is gone.

## 5. Benchmarks

`cargo bench --bench eval`, the target `benchmark.yml` tracks, debug-free
release profile, same machine, runs back to back with nothing else
running. 0.0.31 is the `main` worktree; 0.0.34 this branch.

| benchmark | 0.0.31 | 0.0.34 | change |
|---|---|---|---|
| `parse/nested_path` (filter parse, no YAML) | 501.4 ns | 496.7 ns | within noise |
| `eval_str/field_access` | 3.745 µs | 3.616 µs | -3.5% |
| `eval_str/iterate_100` | 155.5 µs | 141.4 µs | -9.1% |

The two end-to-end cases parse YAML and improve; the filter parser does not
touch noyalib and does not move. Consistent with 0.0.33's per-key
allocation drop (§1). No regression to justify.

## 6. Acceptance criteria

- [x] The pin moves to `0.0.34`; `Cargo.lock` shows noyalib moving and
      nothing else.
- [x] `validate` reports a stringified-key collision as `Y102` with a
      position; both regression tests green.
- [x] `yqr-b028` filed and resolved; every located stream error positioned
      from the stream.
- [x] The emitter changes measured against the 0.0.31 build (§3) and
      recorded in `CHANGELOG.md`.
- [x] Sequence writes compared against 0.0.31 for #375 (§1): no
      difference.
- [x] Benchmarks compared (§5).
- [x] `Cargo.toml` pin comment says what the bump bought; trackers
      updated.
- [x] Full suite green; `cargo clippy --all-targets --all-features -D
      warnings` clean; `local-ci.sh` clean.
