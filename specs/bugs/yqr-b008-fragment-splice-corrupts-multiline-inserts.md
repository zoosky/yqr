# Bug b008 — hand-built fragments corrupt `+=` and new-key inserts of a multi-line string

**Status:** Fixed (2026-08-13) — both insert paths now route through noyalib's
typed insertion tier (`insert_entry_value` / `push_back_value`), which owns the
indentation and holds the splice to a load-back oracle. Shipped with the
`noyalib 0.0.21` pin (`yqr-f014`).
**Severity:** High — silent data corruption on a shipped code path. One case
produced output yqr itself cannot re-parse; the other produced a wrong value.
Both exited **0**, so `-i` wrote the damage to the user's file and reported
success.
**Owner:** yqr maintainers
**Last updated:** 2026-08-13
**Affects:** `src/fidelity/write.rs` — `insert_key` (new-key assignment) and
`append` (`+=`), i.e. `.a.b = <string>` where `b` is new, and `.s += <string>`.
Existing-key assignment (`set_value`) was never affected. Present since the
write tier shipped (`yqr-f006`, v0.4.0) and through the 0.0.18 pin.
**Component:** yqr's own fragment construction, not the engine.
**Related:** `yqr-f006` (the write tier that introduced `value_fragment`),
`yqr-b004` §2.5 (the fragment-quoting gap this is the concrete instance of),
`yqr-f013` §3.4 (which flagged the typed tier as having "a latent correctness
argument" — this bug is that argument, no longer latent), `yqr-f014` (the
0.0.21 pin the fix ships with), `yqr-f007` (structural edits)

## 1. Summary

`value_fragment` rendered the right-hand side to a YAML string and handed it to
noyalib's **fragment-taking** mutators, `insert_entry(parent, key, &fragment)`
and `push_back(path, &fragment)`. Those splice the fragment verbatim: they
synthesise indentation for the *first* line only.

A string containing a newline has no single-line spelling, so `crate::render`
produced a **block scalar** — several lines, whose continuation lines carry the
indentation of the rendering, not of the insertion site. Spliced verbatim, the
result was a differently-shaped document.

The `Ok` return was not a guard against this. The fragment mutators' re-parse
guard rejects *invalid* YAML; it does not reject YAML that is valid but means
something else, which is exactly `yqr-b004` §2.5. In the sequence case here the
result was not even valid, and it still committed.

## 2. Reproduction

Measured against the 0.0.18 pin, before the fix. Both exit **0**.

### 2.1 `+=` into a block sequence — output does not re-parse

```console
$ cat t.yaml
keep: 0
s:
  - one

$ yqr '.s += "v\nqq: 7"' t.yaml
keep: 0
s:
  - one
  - |-
  v
  qq: 7
$ echo $?
0
```

The block scalar's content is at the same column as its `- |-` indicator, so it
has no content at all and the following lines are read as siblings. Feeding the
result back:

```console
$ yqr '.' out.yaml
yqr: io error: failed to parse YAML input: YAML parse error at line 5,
column 3: expected block sequence entry or end
```

With `-i`, that unparseable text is what lands in the file.

### 2.2 New-key assignment — wrong value, silently

```console
$ yqr '.m.b = "v\nqq: 7"' t2.yaml
keep: 0
m:
  a: 1
  b:
    |-
      v
      qq: 7
```

This parses, which is worse. The block-scalar *header* was pushed onto its own
line, so it is no longer a header — `.m.b` reads back as the literal string
`"|-\n      v\n      qq: 7"` instead of `"v\nqq: 7"`.

### 2.3 Control — the typed path was always correct

```console
$ yqr '.m.a = "v\nqq: 7"' t.yaml    # `a` already exists -> set_value
keep: 0
m:
  a: |-
    v
    qq: 7
```

`set_value` takes a typed `&Value` and lets the engine spell it. That is the
whole difference, and it is what the fix generalises to the other two paths.

## 3. Root cause

`src/fidelity/write.rs` had two layers, each individually defensible:

1. `value_fragment` refused collections, on the reasoning that "splicing its
   multi-line rendering would silently mis-shape the document". Correct — but
   the property that matters is **multi-line**, not **collection**. A scalar
   string with a newline renders multi-line too, and slipped through.
2. The fragment mutators' `Result` was treated as the structural-integrity
   guard (the module doc said so). It is not a sufficient one for a fragment
   the caller built; `yqr-b004` §2.5 had already recorded this and the write
   tier did not act on it.

## 4. Fix

`value_fragment` is replaced by `insertable`, which lowers the value to
`::noyalib::Value` instead of to a string, and the two call sites move to the
typed tier:

| Path | Before | After |
|------|--------|-------|
| new-key assignment | `insert_entry(parent, key, &fragment)` | `insert_entry_value(parent, key, &ny)` |
| `+=` | `push_back(path, &fragment)` | `push_back_value(path, &ny)` |

The typed mutators come from `Emit` (noyalib#223 — yqr's own upstream
contribution, released in 0.0.18 and hardened in 0.0.21). They compute the
insertion site's indentation, choose the spelling, and hold the result to a
**load-back oracle**: after the splice the document must load as the pre-edit
value with exactly that one insertion applied, or the edit is rolled back. That
is the guard §3.2 was missing.

The collection refusal is kept verbatim. The typed tier *can* express a
collection, so it is now a deliberate scope limit on `+=` / new-key assignment
rather than a backend constraint — lifting it is `yqr-f007` / `yqr-f008` work,
not a bug fix, and it is called out as such in the code.

## 5. Verification

Three regression tests in `src/fidelity/write.rs`, each asserting the emitted
bytes **and** the loaded-back value, so a future regression to a
plausible-but-wrong spelling fails too:

- `appended_multiline_string_is_indented_for_its_insertion_site` — §2.1;
- `inserted_multiline_string_is_indented_for_its_insertion_site` — §2.2;
- `inserted_string_is_quoted_when_its_plain_spelling_would_change_type` — the
  neighbouring property the typed tier brings (`"8080"` stays a string).

Post-fix output for §2.1 and §2.2:

```yaml
keep: 0          keep: 0
s:               m:
  - one            a: 1
  - |-             b: |-
      v                v
      qq: 7            qq: 7
```

Both load back as `"v\nqq: 7"`; `keep` is byte-identical.

## 6. Why the existing suite missed it

Every insert/append test used a single-line scalar — `Value::Int(9090)`,
`Value::String("prod")`. The corpus (`yqr-m003`) has no case that assigns a
multi-line string either. The shape was untested rather than tested-and-wrong,
and the tests added in §5 close that gap for both mutators.

The `-i` exposure is worth stating plainly: a guard that returns `Ok` on a
corrupt result is worse than no guard, because the write-back path trusts it.
</content>
</invoke>
