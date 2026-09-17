# Feature f035 — Adopt noyalib 0.0.44: four upstream fixes land, two yqr bugs close

**Status:** Done — adopted 2026-09-17
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f034` (the 0.0.43 adoption this follows), `yqr-b032` and
`yqr-b034` (resolved here), `yqr-b033` (re-measured, still open),
`yqr-b029` and `yqr-b030` (whose upstream halves land here, having been
resolved in yqr by guards)

## 1. Scope

Bump `noyalib = "0.0.43"` to `0.0.44`, the first release carrying yqr's
four open upstream PRs, and verify yqr against the published crate.
`Cargo.lock` moves noyalib's version and checksum and nothing else.

`yqr-f034` adopted a release that changed nothing, so that this one would
be a single step measured against a known base. This is that step.

| upstream | what | yqr effect |
|---|---|---|
| noyalib#427 | an insert stops at the last line its anchor entry owns | **`b032` closes** (§2.1) |
| noyalib#437 | a kept block scalar's blank lines are content, not trivia (#429); a tab-indented comment is treated alike after a plain and a quoted scalar (#428) | **`b034` closes** (§2.2) |
| noyalib#422 | a replacement adopts the document's line break, like an insertion | a multi-line write into a CRLF file now works instead of being refused (§2.3) |
| noyalib#424 | a scalar written over a block collection keeps its indent | yqr's refusal stays (§2.4) |
| noyalib#426 | comments on an entry written with no value | **no effect** — `b033` is unmoved (§2.5) |

Also in the release and not on any yqr path: a `perf(cst)` change bounding
the header scan, the serializer's struct-field reparenting fix, and CI
work.

## 2. Measured

Each row below is a command run against a debug build on 0.0.43 and the
same build on 0.0.44, nothing else changed.

### 2.1 `b032` — an insert stops at its anchor's last line

```console
$ cat config.yaml
a:
  b:
    n: 1
# why z matters
z: 3
```

| | 0.0.43 | 0.0.44 |
|---|---|---|
| `.a.c = "2"` | the key lands **below** `# why z matters` | it lands **above** it |
| `head_comment(.z)` after that write | `null` | `why z matters` |

The comment keeps the key it documents. Five assertions flip with it, all
written for this moment; they are listed in `yqr-b032` §7.

### 2.2 `b034` — a key beside a nested kept block scalar

```console
$ cat config.yaml
a:
  b: |+
    x

```

| | 0.0.43 | 0.0.44 |
|---|---|---|
| `.c = "2"` | refused, exit 5, message names a `<<` merge the file has not got | written, exit 0 |
| `.a.b` read back after | — | `x\n\n`, byte-identical to before the write |

The value assertion is the one that matters: the blank lines a `|+` keeps
are content, and a byte diff cannot tell them from trivia on the wrong
side of the new key. `yqr-b034` §5's proposed message fix is moot, since
there is no refusal left to word.

### 2.3 `b030` — a multi-line write into a CRLF document

`.logging.level = "warn\nverbose"` over a wholly-CRLF file was refused by
yqr's own guard, because the replacement joined its own lines with LF and
would have left the file mixed. On 0.0.44 it is written, and every line of
the emitted block scalar ends `\r\n`.

**The guard is kept.** It is stated over the *result* — a document that was
wholly CRLF and now holds a bare line feed — not over the mutator that
produced it, so it costs nothing and remains the backstop if a path
regresses. No yqr path reaches it now, which is recorded in the code
beside it.

### 2.4 `b029` — a scalar over a block collection: the refusal stays

yqr refuses `.k = 5` where `k` holds a block mapping, and the refusal names
the remedy (`del` then assign). Upstream #424 fixes the half that made this
*invalid*: with yqr's pre-emptive guard bypassed, 0.0.43 emits `k:` / `5`
at column 0, which other parsers reject, and 0.0.44 emits `k:` / `  5`,
which is valid and means `k: 5`.

Valid is not the same as right. The result is still a mapping key whose
scalar value sits on the next line at a deeper indent — a layout no author
writes, produced by an edit that named a value. Relaxing the refusal is a
behaviour change with its own remedy text and its own tests, so it wants
a feature spec rather than a line in an adoption. Filed as `yqr-f036`.

### 2.5 `b033` — unchanged, as its filing predicted

noyalib#426 merged and shipped here. `yqr-b033` §3 argued it would not fix
the block-collection shape, because `comment_anchor_span` returns a block
collection's value span unchanged. Re-measured on 0.0.44, the `b033` §2
table is identical: `head_comment(.k)` reads the comment above `k: 1` and
`null` above the same comment on `k:` / `  n: 1`. It stays open, and is
now the only open yqr bug.

### 2.6 The published crates

A `diff -r` of the two crates.io sources under `~/.cargo/registry/src/`,
which is what yqr compiles. Four source files change materially:

| file | changed lines | on a yqr path |
|---|---|---|
| `src/cst/document.rs` | 729 | yes — every §2 row above |
| `src/ser.rs` | 71 | `--normalize` only |
| `src/parser/scanner.rs` | 51 | yes — the #428 half of #437 |
| `src/cst/annotated.rs` | 12 | yes — #426's call sites |
| `src/error.rs` | 149 | no — a `#[cfg(test)]` module |
| `src/lib.rs` | 10 | no — a `cfg(kani)` lint allowance |

### 2.7 The suite

`cargo test --all-targets --locked` is green. Eight expectations moved,
every one of them a defect pinned as it behaved with the flip written
beside it:

| file | expectations |
|---|---|
| `src/fidelity/write/backend.rs` | 3 (`b032`) |
| `src/fidelity/write/guards.rs` | 1 (`b030`) |
| `tests/cli.rs` | 2 (`b032`, `b034`) |
| `tests/corpus/mod.rs` | 2 (`b032`, `b030`) |

No production code changed. `local-ci.sh` is clean.

## 3. Benchmarks

Run, because this is the first pin in five releases whose engine source
actually moved, and one of the commits is a `perf(cst)` change.
`cargo bench --bench eval`, 0.0.43 saved as a baseline and 0.0.44 compared
against it on the same machine in the same session:

| benchmark | 0.0.43 | 0.0.44 | criterion's verdict |
|---|---|---|---|
| `parse/nested_path` | 527 ns | 576 ns | regressed 4.9% |
| `eval_str/field_access` | 3.89 µs | 4.01 µs | regressed 2.5% |
| `eval_str/iterate_100` | 157 µs | 160 µs | regressed 4.1% |

**Those verdicts are drift, and the suite proves it on itself.**
`parse/nested_path` calls `yqr::parser::parse` and nothing else — it never
enters noyalib, so no noyalib release can move it. It moved 4.9%.

Running the pair in the other order settles it. With 0.0.44 saved as the
baseline and 0.0.43 compared against it:

| benchmark | criterion's verdict for **0.0.43** |
|---|---|
| `parse/nested_path` | regressed 5.2% |
| `eval_str/field_access` | no change (p = 0.85) |
| `eval_str/iterate_100` | improved 3.7% |

The engine-independent benchmark regresses by about the same margin
whichever release runs second, and `iterate_100` changes sign between the
two orderings. What both runs measure is the second half of a pair being
slower than the first on this machine, by roughly 4%. At this benchmark's
resolution there is no difference between the two releases to find, and a
4% claim in either direction would be unsupportable.

Worth noting for the next adoption: a single before/after run of this
suite cannot resolve anything under about 5%, and `parse/nested_path` is
the control that says so, because nothing but yqr's own parser can move it.

## 4. Acceptance criteria

- [x] The pin moves to `0.0.44`; `Cargo.lock` shows noyalib moving and
      nothing else.
- [x] `b032` and `b034` verified fixed against the published crate, by
      the reproductions their filings recorded (§2.1, §2.2).
- [x] `b033` re-measured and confirmed unmoved (§2.5).
- [x] The two guards whose upstream halves land here are each decided:
      `b030`'s kept as a backstop, `b029`'s kept and the relaxation
      filed as `f036` (§2.3, §2.4).
- [x] Full suite green; every moved expectation is a pinned defect
      flipping, and no production code changed (§2.7).
- [x] Benchmarks run and compared against 0.0.43 (§3).
- [x] `Cargo.toml` pin comment and `CHANGELOG.md` say what the bump
      bought; bug and feature trackers updated.
- [x] `local-ci.sh` clean.
