# Bug b020 — A write to a merge-produced entry reports `path not found` for a path yqr just read

**Status:** Open — filed 2026-08-22 while documenting `yqr-b019`; the message
was the one thing that could not be shown in the guide
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
