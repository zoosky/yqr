# Bug b034 — Adding a key beside a kept block scalar is refused, and the refusal blames a merge

**Status:** Open — filed 2026-09-10. The cause is upstream, **filed as
noyalib#429**; the refusal's wording is yqr's and can be fixed here
**Severity:** Low — the write is refused and the file is left unchanged,
so nothing is corrupted. The cost is a capability that ought to work and
a message that names two causes, neither of them the real one
**Component:** write tier — `src/fidelity/write/backend.rs:265`
(`insert_entry_value`), and upstream `trim_value_span`
**Related:** `yqr-b032` (found in the same round, the other face of the
insert anchor), `yqr-b024` and `yqr-b020` (refusals that assert a cause
instead of describing what was seen), `yqr-f025` (a refusal names a
remedy that runs)

## 1. Summary

A block scalar written `|+` or `>+` keeps the blank lines at its end as
part of its value. Adding a key to the mapping **above** such a scalar is
refused:

```console
$ cat config.yaml
a:
  b: |+
    x

$ yqr '.c = "2"' config.yaml
yqr: runtime error: cannot insert key "c": YAML parse error:
insert_entry_value: inserting `c` into `` failed the integrity check — the
spliced entry did not load back as the value given (e.g. a key the mapping
already inherits through a `<<` merge, or a layout the emitter could not
reproduce at this indent); the document was left unchanged
$ echo $?
5
```

The file has no merge key and the emitter reproduced the layout fine. The
message names the two causes the check was written for and not the one
that fired.

## 2. Extent, measured on the pinned 0.0.41

| what is added | result |
|---|---|
| `.c = "2"` (root, beside the nested `|+`) | **refused** |
| `.a.z = "2"` (inside `a`, beside the scalar) | works, blank kept |
| `.c = "2"` where `a: \|+` is the root's own last entry | works, blank kept |
| `>+` in either position | same as `\|+` |
| `\|` or `\|-` anywhere | works |

So the trigger is narrow: the anchor entry's value must *contain* a
keep-chomped scalar at its tail, rather than be one.

**Nothing is corrupted.** The refusal comes from upstream's typed
load-back oracle, which compares the spliced document against the value
it was given, notices the scalar lost a line, and rolls back. The guard
is doing exactly its job.

## 3. Cause

Upstream's `trim_value_span` asks `is_keep_chomped_block_scalar` about
the **anchor entry's own value**. Here that value is a mapping, so the
answer is no and the trailing blank lines are trimmed off the span as
trivia. They are not trivia: they belong to `a.b`, one level down. The
splice then lands above them, the scalar loses a line, and the oracle
refuses.

The check is right about the value it is handed and wrong about the one
that owns the bytes at the tail. Filed as **noyalib#429** with the
reproduction, the four affected shapes and the scope note that only the
unguarded `insert_entry` tier ships the truncation — yqr uses
`insert_entry_value` and gets the refusal instead.

## 4. How it was found

Reviewing yqr's own noyalib#427. That patch walks the anchor's lines and
skips blank ones as trivia, which broke the case where the keep-chomped
scalar **is** the anchor's value — turning a working insert into a
silently truncated one. The review caught it; the patch now asks whether
the span holds a keep-chomped header anywhere before treating a blank
line as trivia, and eight tests hold it.

Sweeping 374 shapes to confirm that fix turned up eight that still lose a
value. They fail identically with the patch, without it, and on the
published 0.0.43, which is what identifies them as a separate,
pre-existing defect rather than fallout.

**The lesson worth keeping.** The sweep that found the regression asserts
two things per insert: that the output differs from the input only by the
added line, and that the document minus the added key loads back exactly
as before. The first is the `yqr-a001` byte property and it passes on
every one of these — the blank lines are all still there, on the wrong
side of the new key. Only the second sees the damage. A byte-fidelity
check is not a value-fidelity check, and for a block scalar the two come
apart.

## 5. What yqr can fix without upstream

Not the capability — yqr does not compute the anchor and cannot make the
insert work. But the **message** is yqr's to improve, and it is the worse
half. Today it forwards upstream's sentence verbatim, which asserts two
causes as examples and reads as an internal step.

The `yqr-b020` route applies: yqr already knows things upstream's
diagnostic does not, and can say what it sees rather than guess why.
Before forwarding an integrity-check failure from an insert, yqr can look
for a keep-chomped block scalar under the anchor and, finding one, say
that the entry above a kept block scalar cannot take a new sibling yet,
naming `noyalib#429`'s shape in the user's terms rather than the
library's. The remedy that works is in the table in §2: add the key
*inside* the mapping that holds the scalar, which is a different path
than the one refused.

Deliberately **not done in this filing**. It is a message change with a
test that runs the remedy, worth its own round, and it wants the upstream
answer first in case #429 removes the refusal entirely.

## 6. Tests

Three CLI tests, added with this filing rather than deferred, because the
working rows are one line of engine code away from data loss and nothing
else in yqr pins them. yqr covers a keep-chomped scalar in `del` already
(`delete.rs`) and covered nothing on the insert path.

- The working shape, `a: |+` as the anchor itself, asserting the document
  **and** reading the scalar back. The read is the point: yqr's own
  noyalib#427 first placed the key above the blank, which took a line off
  the value at exit 0 with a byte diff that still looked like a clean
  insertion. Only a value assertion sees that.
- Clip and strip as controls, which own no trailing blank and must not
  move when this bug is fixed.
- The refusal itself, asserting exit 5 and that nothing was written. It
  deliberately does **not** assert the message, since the message is the
  part expected to change.
