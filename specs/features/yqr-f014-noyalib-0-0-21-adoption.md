# Feature f014 — Adopt noyalib 0.0.21: the silent-corruption fixes and the typed insertion tier

**Status:** Done
**Epic:** Fidelity write tier (`f006`–`f008`)
**Owner:** yqr maintainers
**Related:** `yqr-f013` (the 0.0.18 adoption this succeeds, whose §3.4 hand-off
this consumes), `yqr-b008` (the corruption this bump's typed tier fixes),
`yqr-b004` (the mutation-API gap catalog, whose §6.4 follow-up shipped here),
`yqr-b006` (delete trivia), `yqr-f007` (structural edits), `yqr-m004`
(crates.io publishing)

## 1. Scope

Bump `noyalib = "0.0.18"` to `0.0.21` and consume what the two intervening
releases bring. Unlike `yqr-f013`, this bump is **corrective, not
preparatory**: three of the four defects it addresses were reachable from
yqr's shipped CLI.

**In scope:** the pin bump and its lockfile/audit fallout (§3.1); routing the
two untyped insert paths through the typed tier, which is the `yqr-b008` fix
(§3.2); recording that noyalib#226 shipped (§3.3); correcting the upstream
status update for the record (§4).

**Out of scope:** the comment-edit, key-rename and sequence-reorder grammar
(still `yqr-f007` §6 — the APIs have been available since 0.0.18 and the
blocker remains grammar, not backend). Lifting the collection restriction on
`+=` / new-key assignment, which the typed tier now makes expressible (§3.4).

## 2. What the two releases contain

0.0.20 was merged upstream but never tagged, so the published sequence is
0.0.18 → **0.0.19** (2026-08-11) → **0.0.21** (2026-08-13).

### 2.1 noyalib 0.0.19

- **CST `remove()` took trivia it did not own (#226)** — yqr's own PR, filed
  against the divergences `yqr-b004` §6.1 measured. This is the release that
  closes `yqr-b004` §6.4.
- **`from_str_strict` rejected every populated `Option` field (#239)** — not a
  path yqr uses.
- **Bare `nan` / `inf` spellings destroyed a scalar's text** — a key `nAn`
  came back as `nan`, so it did not round-trip. yqr's own fidelity harness
  never caught this because the byte-fidelity path does not resolve plain
  scalars; it would have surfaced under `--normalize`.

### 2.2 noyalib 0.0.21

Three silent-corruption fixes in the mutators, each of which returned `Ok`
while damaging the document, and two `Emit` defects:

| Upstream fix | Reachable from yqr? |
|---|---|
| `remove` deleted a whole flow collection (`remove("a.x")` on `a: {x: 1, y: 2}` deleted the document) | **No** — `yqr-f013` §3.2 stopped calling `remove`, and yqr's flow pre-check refuses first |
| `set` fragment could reach outside its path | **No** — yqr calls `set_value`, never `set` |
| `push_back` / `insert_after` had the same hole | **Partly** — see §3.2; yqr's exposure was a different failure of the same class |
| `Emit` trailing colon (`"a:"` emitted as `a: a:`) | **Yes** — `.k = "a:"` errored out |
| `Emit` lone newline (`"\n"` emitted as a `\|` header with no body) | **Yes** — `.k = "\n"` silently became the string `"\|"` |

The two `Emit` defects reached yqr through `set_value`, the API documented as
the *safe* route. Both are fixed by the bump alone, with no yqr change:

| Probe | On 0.0.18 | On 0.0.21 |
|---|---|---|
| `.k = "a:"` | `runtime error: mapping values are not allowed in this context` (exit 5) | `k: "a:"` |
| `.k = "\n"` | `k: \|` — reads back as `"\|"` (exit 0) | `k: "\n"` |

That `yqr-f013`'s decision to keep yqr's own delete path also kept yqr clear of
the flow-`remove` corruption is worth recording. It was decided on trivia
grounds (`yqr-b006`), and the safety was incidental — but it is the second time
that decision has paid.

## 3. Work items

### 3.1 Pin bump

`Cargo.toml:41` and the comment above it, then `cargo check` to refresh
`Cargo.lock` and the full local CI mirror.

**Done.** `Cargo.lock` moves noyalib 0.0.18 → 0.0.21 and adds two transitive
crates, `hashbrown 0.15.5` and `libm 0.2.16` — both arriving via 0.0.21's
bare-metal `no_std` work (#210), which needed a hasher and float intrinsics
that `core` does not provide. `cargo audit` is clean. MSRV upstream is 1.86.0
against yqr's pinned 1.97.1 toolchain, so no impact. `tests/fidelity.rs` and
`tests/corpus_validation.rs` pass untouched: byte fidelity is unaffected.

### 3.2 Route the untyped insert paths through the typed tier (`yqr-b008`)

The bump alone does **not** fix yqr's `+=` and new-key inserts. Upstream's
`push_back` fix guards a fragment that escapes its *path*; yqr's fragment did
not escape its path, it was mis-indented within it — the same class, a
different symptom, and one only yqr can fix, because yqr is the one building
the fragment.

Full case analysis in `yqr-b008`. In short: `value_fragment` rendered the RHS
to a string, and a string containing a newline renders to a multi-line block
scalar whose continuation lines the fragment mutators splice verbatim. `+=`
produced unparseable output; new-key assignment produced a wrong value. Both
exited 0, so `-i` wrote the damage and reported success.

`value_fragment` is replaced by `insertable` (lowering to `::noyalib::Value`),
`insert_entry` → `insert_entry_value` and `push_back` → `push_back_value`. The
typed tier owns the indentation and holds the splice to a load-back oracle.

This is exactly the adoption `yqr-f013` §3.4 identified as having "a latent
correctness argument … Evaluate it as f007's first slice". The argument turned
out not to be latent, so it lands here as a bug fix rather than waiting on
grammar.

### 3.3 noyalib#226 shipped — close out `yqr-b004` §6.4

Merged 2026-08-05, released in 0.0.19. `yqr-b004` §6.4 and `yqr-f013` §6 both
carried it as "open / awaiting review"; both now record the release.

This re-opens the question `yqr-f013` §3.2 deferred: with the trivia
divergences fixed upstream, is option (b) — call `remove`, keep a trivia
pre-pass — now correct? **Not taken here, and not for the original reason.**
0.0.21's flow-`remove` corruption (§2.2) is a reminder that the delete path is
where upstream has had the most churn, and yqr's version is covered by the
`yqr-f007` §5.4 tests with no open defects. Re-evaluating is cheap and can
happen any time; doing it in the same change as a corruption fix is not worth
the coupling. Recorded as still-open in `yqr-f007` §6.

### 3.4 Now expressible, deliberately not taken

The typed tier can spell a nested collection, so the "collections are not yet
supported" refusal on `+=` / new-key assignment is no longer a backend
constraint. It stays, as a scope limit: allowing `.a.b = {…}` is a user-facing
surface change that belongs to `yqr-f007` / `yqr-f008` with its own tests and
docs. The code comment says so, so the next reader does not mistake it for a
limitation that still exists.

## 4. Correction to the upstream status update (for the record)

On 2026-08-11 the maintainer posted a status update on
[noyalib#221](https://github.com/sebastienrousseau/noyalib/issues/221), keeping
it open as the umbrella for gaps 1, 4 and 5:

> **Done on `main`:** `rename_key`, `key_span`, `swap_items`, `move_item`.
> **Still open:** comment mutation; extended `remove`; fragment quoting.

Verified against the published `.crate` files for both 0.0.18 and 0.0.21, that
update **undercounts what has shipped** on all three "still open" items:

| #221 item | Status update | In the published crate |
|---|---|---|
| 1. comment mutation | still open | `set_inline_comment` / `set_leading_comment` / `remove_inline_comment` / `remove_leading_comment` present in **0.0.18**; 0.0.21 adds `set_comment` / `remove_comment` + `CommentPosition` |
| 4. extended `remove` | "still declines multi-line/nested" | 0.0.18 accepts both — behaviourally proven, since four yqr delete tests failed on the 0.0.18 bump *because* `remove` stopped refusing (`yqr-f013` §3.2) |
| 5. fragment quoting | "still the deferred `Emit` work" | `Emit` / `EmitCtx` and `insert_entry_value` / `push_back_value` / `insert_after_value` present in **0.0.18** — noyalib#223, yqr's own PR; 0.0.21's release notes fix two `Emit` defects, which presupposes it exists |

The practical consequence is the maintainer's standing offer, repeated on
noyalib#226: *"Your offer to port the `replace_span` fallback for
multi-line/nested removal upstream stands welcome."* That offer is moot as
written — upstream `remove` has handled those shapes since 0.0.18, and yqr's
own path exists for **trivia** reasons, which noyalib#226 already addressed.

What is genuinely still unaccepted upstream is **sole-entry** and **flow**
deletes, which yqr also refuses. Those are the remaining real gap, and the
honest reply upstream is the evidence table above plus an offer on those two
cases. **Not posted** — deferred by decision; this section is the record so the
reply can be written from it later.

## 5. Acceptance criteria

- [x] `Cargo.toml` pins `noyalib = "0.0.21"`, the adjacent comment names 0.0.21,
      and `Cargo.lock` is refreshed.
- [x] `bash .github/scripts/local-ci.sh` passes, including `cargo audit`.
- [x] `tests/fidelity.rs` and `tests/corpus_validation.rs` pass unchanged.
- [x] `insert_key` and `append` route through the typed tier; `yqr-b008`'s
      three regression tests pass and assert bytes **and** loaded-back value.
- [x] The two `Emit` probes (`.k = "a:"`, `.k = "\n"`) are correct on 0.0.21.
- [x] `yqr-b004` §6.4 and `yqr-f013` §6 record noyalib#226 as released in
      0.0.19; `yqr-b008` is filed and marked Fixed.
- [x] `CHANGELOG.md` records the bump and the corruption fix.

## 6. Non-goals

- No new filter grammar. `del`, `=`, `+=` and the existing surface are
  unchanged.
- No behaviour change to the read path or `--normalize`.
- Not a release. `yqr-m001` governs when a version ships; `cargo publish` stays
  separately authorized (`yqr-m004`).
- No upstream comment posted (§4).
</content>
</invoke>
