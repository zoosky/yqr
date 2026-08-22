# Bug b020 — A write to a merge-produced entry reports `path not found` for a path yqr just read

> **Historical: resolved.** yqr no longer behaves as described below. The
> **Status** line records what fixed it and when; the rest is kept as the
> reproduction and the reasoning, written in the present tense of the time it
> was filed.

**Status:** Resolved — filed and fixed 2026-08-22, the day `yqr-b019` landed.
Route 1 of §4, narrowed: yqr owns the message for the **merged-key** arm only,
which §6 explains is not the two-voices trade §4 feared. The guide now quotes
the refusal it previously had to paraphrase
**Severity:** Low — the refusal is correct and the file is untouched; what is
wrong is that the reason given contradicts the tool's own read
**Component:** `src/fidelity/write.rs`, `NoyalibWriter::set_value` — the
message comes from noyalib's `Document::write_span` and is passed through
**Related:** `yqr-b019` (which made this reachable on more inputs and hit it in
the docs), `yqr-b012` (a diagnostic asserting a cause instead of describing an
observation — the same class), `yqr-a002` §4

## 1. Summary

A key reached through a `<<` merge reads fine and refuses to be written, which
is right. The refusal says the path does not exist, which is not:

```console
$ printf 'base: &m\n  k: 1\nc:\n  <<: *m\n' | yqr '.c.k'
1
$ printf 'base: &m\n  k: 1\nc:\n  <<: *m\n' | yqr '.c.k = 9'
yqr: runtime error: cannot assign at "c.k": YAML parse error: path not found: c.k
```

One command prints the value at `c.k`; the next says there is no `c.k`. Both
are yqr, on the same file, seconds apart.

Two smaller faults ride along: `YAML parse error` prefixes something that is
not a parse failure (the document parsed — that is how the value was read), and
the message offers no next step. The sibling alias refusal does:
*"edit the anchor definition or replace the alias explicitly"*.

## 2. What is actually true

`c` has no `k` entry. It has a `<<` that pulls one in from `m`. So there is
nothing at `c.k` to overwrite, and writing one would mean **creating** an entry
that shadows the merge — a different edit from the one asked for, and one the
user should choose deliberately.

That is a statement about the document's shape, and it is what the message
should say.

## 3. Why it is worth fixing rather than absorbing

`yqr-b019` made it reachable whenever the assigned value *matches*, not only
when it differs, so a user who runs `.c.k = 1` to confirm a value now meets it.
It was also the one example in `yqr-b019`'s documentation pass that had to be
written as prose, because quoting the real output next to "you can read this
path" reads as a contradiction — a fair signal that the message is wrong rather
than the docs.

## 4. Fix route, and the reason it is not in `yqr-b019`

yqr already establishes the fact, in `FidelityWriter::value_is_borrowed`: a
key-terminated path with no `key_span` is one a merge or an alias expansion
produced. So the message can be yqr's own and accurate.

`yqr-b019` deliberately did **not** take that route. Its guard falls *through*
to the writer precisely so the diagnostic stays upstream's and there is no
second copy to drift — and `value_is_borrowed` is conservative, answering "not
established" for shapes it cannot measure (an alias-valued sequence item).
Refusing with yqr's own words where it is established and upstream's where it
is not would give one class of failure two voices, which is worse than one
imprecise voice.

So the fix is a decision, not a patch:

1. **Take the message over**, and accept that yqr refuses borrowed sites in its
   own words. Needs `value_is_borrowed`'s residual gap closed first, or the two
   voices problem is real.
2. **Fix it upstream.** `write_span` distinguishes the alias case from the
   not-found case already — it returns a tailored message for the first and
   falls back to `path not found` for the second. A merge-produced key is
   reached through the same typed cache, so the information is there.

Option 2 is the same shape as `yqr-b012` and most of the noyalib bugs yqr has
filed: the library's own code already knows the answer at the point it gives
the wrong one. Measure it against noyalib's current source before filing.

## 6. What the measurement changed

Measured, as §4 asked. Two findings moved the decision to option 1.

### 6.1 Upstream cannot re-word this; it would have to resolve differently

`write_span` does not *choose* the wrong message. `resolve_span` — the single
resolver behind both it and `span_at` — returns a bare `None` for a
merge-produced key, which is the same answer it gives for a key that is not
there:

```rust
// noyalib 0.0.27, src/cst/document.rs:626
let ((s, e), through_alias) = resolve_span(value, span_tree, &segments)
    .ok_or_else(|| Error::Parse(format!("path not found: {path}")))?;
```

Confirmed at the seam yqr can observe: `span_at("c.k")` is `None` on the
reproduction, and `span_at` reaches it through the same call. So option 2 is
not a re-wording upstream can make locally; it needs the resolver to carry a
third outcome. Worth filing, and larger than this bug.

### 6.2 Upstream already words it correctly — in `rename_key`

The information exists upstream, and one of its own mutators uses it:

```console
$ printf 'base: &m
  k: 1
c:
  <<: *m
' | yqr 'key(.c.k) = "x"'
yqr: runtime error: cannot rename key at "c.k": YAML parse error: rename_key:
key "k" was produced by a `<<` merge key and has no entry of its own to rename
in this mapping
```

So `rename_key` and `set_value` answer the same question about the same bytes
two different ways — the `yqr-b012` shape, and the argument to make upstream
with. yqr's message is worded from that sentence rather than invented, so the
two stay recognisably the same fact if upstream later closes the gap.

### 6.3 The two-voices objection was to a wider fix than the one needed

§4 rejected option 1 because `value_is_borrowed` is conservative. That is true,
and it does not apply to the half this bug is about. The predicate rests on two
independent facts, and they partition by *what is wrong*, not by whether yqr
managed to notice:

- **The key is not in the source** (`key_span` is `None` on a key-terminated
  path). Exact and complete: upstream documents `key_span` as `None` for
  exactly the shapes with no key token of their own, so there is no
  merged-in key this misses. It never falls through.
- **The value's bytes are the anchor's** (a value span ahead of its own key).
  This one has the sequence-item gap `yqr-b019` §6 records.

Taking the message over for **both** would give the alias class two voices.
Taking it over for the **first** gives each fact one voice, which is what
shipped. `Borrowed` is an enum rather than a bool for that reason; a unit test
pins both arms, because collapsing them is the change that would reintroduce
the problem.

### 6.4 Verified as a message change and nothing else

96 comparisons of the pre-fix and post-fix binaries across seven documents
(merge, alias-valued entry, alias-expanded mapping, alias in a sequence,
plain scalars, a dotted key, multi-document) × the paths that reach each shape
× `=` and `|=` × a differing and a matching value: **zero** differences in exit
code or stdout, 16 differences in the message — the merged-key cases, and only
those.

### 6.5 One thing this does not cover

An **implicit null** (`a:` with no value) gets the same false `path not found`,
from a different cause: it has a key span but no value span. `b020`'s check
deliberately does not reach it, and a test pins that. It is filed as
`yqr-b021`, because there the refusal itself is questionable — unlike a merged
key, the entry is really there — so it is a write yqr should probably be able
to make, not a sentence to re-word.

## 5. Reproduction

```console
$ printf 'base: &m\n  k: 1\nc:\n  <<: *m\n' | yqr '.c.k = 9'
yqr: runtime error: cannot assign at "c.k": YAML parse error: path not found: c.k
```

Control — the alias face of the same refusal, which words it well:

```console
$ printf 'a: &x 1\nb: *x\n' | yqr '.b = 2'
yqr: runtime error: cannot assign at "b": YAML parse error: cannot set `b`: its
value is (or resolves through) an alias reference; edit the anchor definition
or replace the alias explicitly
```
