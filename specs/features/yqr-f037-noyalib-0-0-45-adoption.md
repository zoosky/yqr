# Feature f037 — Adopt noyalib 0.0.45: the leading comment anchors on the key, b033 closes

**Status:** Done — adopted 2026-09-18
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f035` (the 0.0.44 adoption this follows), `yqr-b033`
(resolved here), `yqr-a002` §4.4 and `yqr-f007` §8.4 (a read must be
total), `yqr-f007` §8.2 (the count guard)

## 1. Scope

Bump `noyalib = "0.0.44"` to `0.0.45` and verify yqr against the published
crate. `Cargo.lock` moves noyalib's version and checksum and nothing else.

The release carries five fixes. One of them is on a yqr path:

| upstream | what | yqr effect |
|---|---|---|
| noyalib#442 | a leading comment is measured from the entry's key line, not from its value | **`b033` closes** (§2.1); head-comment writes on a block-valued key go through (§2.2) |
| #443 | `set` refuses a fragment that introduces a duplicate key | none: yqr never calls the fragment `set` |
| #443 | the `set` shape fingerprint records sibling shapes (`walk_all` was dead code) | none, for the same reason |
| #443 | the borrowed reader honours `!!str` | none: yqr does not use `from_str_borrowed` |
| #441 | `Spanned<T>` carries the parser toggles | none: yqr does not deserialize `Spanned` |

noyalib#442 was yqr's own PR, the upstream ask `b033` §3 described. It
shows as closed rather than merged: the maintainer carried its commit
verbatim into the release PR, #443, after reproducing all four faces on
`main`.

The release also routes each mutator's value-level integrity check
through a new `oracle_rejects` function, so the rollback behind it can be
fault-injected in upstream's tests. Outside `#[cfg(test)]` it is the same
`!=` comparison, so yqr's writes are guarded exactly as before.

## 2. Measured

Each row is a command run against a debug build of `main` on 0.0.44 and
this branch on 0.0.45, nothing else changed.

### 2.1 `b033` — a head comment above a block-valued key

| document | filter | 0.0.44 | 0.0.45 |
|---|---|---|---|
| `# doc for k` / `k: 1` | `head_comment(.k)` | `doc for k` | `doc for k` |
| `# doc for k` / `k:` / `  n: 1` | `head_comment(.k)` | `null` | `doc for k` |
| `# doc for k` / `k:` / `  - 1` | `head_comment(.k)` | `null` | `doc for k` |
| `a: &b 1` / `# c` / `c: *b` | `head_comment(.c)` | `null` | `c` |
| `k:` / `  # about n` / `  n: 1` | `head_comment(.k)` | `null` | `null` |
| `k:` / `  # about n` / `  n: 1` | `head_comment(.k.n)` | `about n` | `about n` |

The comment above a key now reads whatever the value's shape. The alias
row is the same cause: upstream anchored on the value's span, which for an
alias is the anchor's, so the upward walk started on line one. The last
two rows are the guard that matters: a comment on the first child still
belongs to the child alone.

### 2.2 Writes on a block-valued key

| document | filter | 0.0.44 | 0.0.45 |
|---|---|---|---|
| `a: 1` / `k:` / `  n: 1` | `head_comment(.k) = "new"` | written: a one-line block passed upstream's single-line check | written, unchanged |
| HELM values, `service:` with two children | `head_comment(.service) = "..."` | refused: upstream's leading-comment mutator took single-line entries only | written above `service:` at the key's indent |
| `# old` / `k:` / `  n: 1` | `head_comment(.k) = "new"` | refused by yqr's count guard | `# new` replaces `# old` |
| `# old` / `k:` / `  - 1` | `del(head_comment(.k))` | refused by yqr's count guard | the comment is removed |
| `k:` / `  # about n` / `  n: 1` | `del(head_comment(.k))` | refused, blaming a blank line there is none of | refused: "the entry has no comment block above it" |
| `# section` / blank / `k:` / `  n: 1` | `head_comment(.k) = "x"` | written, below the blank line | refused, as a scalar-valued `k` always was |

The fifth row is the face the #443 description calls the one that
matters. On 0.0.44 upstream reported `# about n` as the leading comment of
both `k` and `k.n`, and `remove_comment("k", Before)` deleted it. yqr never
reached that delete: its `check_comment_site` count guard found no
comment above `k:` against upstream's one and refused, though with a
message about blank lines that did not fit. Now both readers agree there
is nothing above `k`, and the refusal says so.

The last row changes in the refusing direction, and it is the documented
rule applied uniformly. A comment block separated from an entry by a blank
line documents what precedes the entry, and yqr refuses to rewrite it
(`yqr-a002` §4.1.1, `yqr-f007` §8.2, and the Kubernetes guide). 0.0.44
enforced that only for scalar-valued keys, because for a block-valued one
upstream saw no comment run at all; 0.0.45 sees it, so the same guard
fires.

### 2.3 The count guard's other branch

`comment_body` returns `None` when yqr's owned count exceeds upstream's
`before`, because slicing a longer tail once panicked. The alias-valued
entry was the one measured route in, and it is closed. A sweep of 30
shapes (keyed, sequence, flow, alias, merge, tagged, anchored, explicit
key, multi-document) found no input reaching `owned > before.len()` on
0.0.45; every remaining disagreement runs the other way, which the guard
already refuses. The branch stays as a backstop, the reason `b030`'s guard
stayed in `f035` §2.3, and its test now covers the remaining disagreement.

### 2.4 The published crates

A `diff -r` of the two crates.io sources:

| file | changed lines | on a yqr path |
|---|---|---|
| `src/cst/document.rs` | 201 | partly: `leading_comment_anchor` (#442); about half is `#[cfg(test)]` fault injection; the `set` guards are not |
| `src/cst/annotated.rs` | 64 | yes: #442's call sites in `comments_at` and the leading-comment mutators |
| `src/de/deserializer.rs` | 41 | no: `Spanned` |
| `src/borrowed.rs` | 32 | no: the borrowed reader |
| `src/lib.rs` | 21 | no: `no_std` target documentation |
| `src/value/serde_impl.rs` | 6 | no: `Spanned` |

### 2.5 The suite

`cargo test --all-targets --locked` is green. Four expectations moved,
each a defect pinned as it behaved:

| file | expectation |
|---|---|
| `src/fidelity/noyalib.rs` | the alias-valued read reports its comment |
| `tests/corpus/mod.rs` | `head_comment(.service)` on HELM values is written |
| `tests/corpus/values.rs` | `head_comment(.argo.tenants.t8)` reads the comment |
| `tests/corpus/values.rs` | `head_comment(.argo.tenants.t8)` is replaced |

The two `t8` cases carried a wrong explanation: they said a blank line
separated the comment from `t8:`. The blank line is *above*
`# default block o1: ...`, which sits directly on the key; `b033` alone
was hiding it.

Three tests are added for `b033`: a unit test reading a head comment above
a mapping, a sequence and a multi-entry mapping and checking a first
child's comment stays the child's, and two CLI tests, one editing and
deleting a block-valued entry's comment and one pinning the accurate
refusal of §2.2's fifth row.

No production code changed. Two comments in `src/` that named the alias
route as current are corrected.

## 3. Benchmarks

`cargo bench --bench eval`, 0.0.44 saved as a baseline and 0.0.45 compared
against it on the same machine in the same session. The eval suite does
not exercise comments, the only yqr path this release moves.

| benchmark | 0.0.44 | 0.0.45 | criterion's verdict |
|---|---|---|---|
| `parse/nested_path` | 505 ns | 523 ns | regressed 3.8% |
| `eval_str/field_access` | 3.73 µs | 3.81 µs | regressed 2.3% |
| `eval_str/iterate_100` | 152 µs | 153 µs | within noise (+0.8%) |

This is the drift `yqr-f035` §3 characterised, not the release.
`parse/nested_path` calls only yqr's own parser and moved the most, and
every row sits under the roughly 5% this suite cannot resolve. The order
swap was not repeated: f035 already showed the second run of a pair
losing about 4% whichever release it is, and nothing on the benchmarked
paths changed in this release to argue otherwise.

## 4. Documentation

`docs/content/guide/kubernetes.md` listed "an entry whose value is a
block" as refused for `head_comment`, with `head_comment(.spec)` as the
example. That is now false. The bullet is replaced by a sentence saying
such a comment is the key's and a comment on the first child stays the
child's, with an example heading `spec:`.

## 5. Acceptance criteria

- [x] The pin moves to `0.0.45`; `Cargo.lock` shows noyalib moving and
      nothing else.
- [x] `b033` verified fixed against the published crate by its own §1
      reproduction (§2.1), and its write face measured (§2.2).
- [x] The count guard's `owned > before` branch re-examined: no known
      route in, kept as a backstop (§2.3).
- [x] Every other fix in the release checked against yqr's call sites
      (§1).
- [x] Full suite green; every moved expectation is a pinned defect
      flipping, and no production code changed (§2.5).
- [x] Benchmarks run and compared against 0.0.44 (§3).
- [x] The Kubernetes guide no longer claims a block-valued entry's
      `head_comment` is refused (§4).
- [x] `Cargo.toml` pin comment and `CHANGELOG.md` say what the bump
      bought; bug and feature trackers updated.
- [x] `local-ci.sh` clean.
