# Implementation m007 — The reference testbed

**Status:** In Progress — built 2026-09-09 with five implementations and
twelve cases. The case table grows as questions come up; that is the point
**Owner:** yqr maintainers
**Related:** `yqr-a001` (the fidelity guarantee the testbed measures peers
against), `yqr-m003` (the shared corpus, which this is not), `yqr-b014`,
`yqr-b022`, `yqr-b029`, `yqr-b031` (four decisions that needed this
evidence and got it by hand), noyalib#425

## 1. Why

Sixteen spec files cite another implementation, sixty-five times between
them, almost always to settle a question yqr could not answer from its own
engine: is this document one anybody else reads, whose comment is this,
what should an emptied block become. Every one of those measurements was
made by hand, at a keyboard, on a machine whose library versions nobody
recorded, and none of them can be re-run.

That was tolerable while the question was always the same one — *does
PyYAML accept this?* — and stopped being tolerable at noyalib#425, where
the question was **whose comment is this** and neither PyYAML nor Psych
has an opinion, because neither models comments at all. The evidence
needed a library that does, and finding one meant starting from scratch.

The testbed is that work, done once and kept.

## 2. What it is not

**Not a conformance suite.** yqr does not have to agree with these
libraries and often should not: `--normalize` is deliberately lossy where
ruamel is not, and yqr refuses edits go-yaml performs. Divergence is
information, not failure.

**Not the shared corpus** (`yqr-m003`). The corpus pins *yqr's own*
behaviour so a change to it fails a test. The testbed records what
*others* do so a design question has a floor. The corpus runs in CI on
every commit; the testbed runs when someone has a question.

## 3. Shape

```
testbed/
  cases.yaml     the questions, each with the decision that raised it
  run.py         the runner, and `--check` for drift
  answers.json   the record: every answer, keyed by case, op and library
  answers.md     the same, rendered for reading
  adapters/      one per implementation, all speaking the same protocol
```

Two artifacts because they have two jobs. `answers.md` is what a person
reads and what a spec links to; `answers.json` is what `--check` compares,
because comparing rendered markdown makes a formatting change look like a
finding.

**The protocol is four lines of JSON.** An adapter reads
`{"op", "source", "path"}` on stdin and writes one of `{"result"}`,
`{"error"}` or `{"unsupported"}` on stdout. A path is a list of segments
rather than a string, so no adapter needs a path parser. Adding a sixth
implementation is one file and one line in `run.py`.

**`unsupported` is an answer.** It is what PyYAML, Psych and js-yaml say
to every comment question, and recording it is what stops a future reader
citing PyYAML to settle one.

### 3.1 The five

| implementation | what it is good for |
|---|---|
| **PyYAML** | the most-run YAML parser there is; the reference for *is this document well formed* |
| **Psych** | Ruby's, and a libyaml front end, so a second judge over the C parser many tools embed |
| **js-yaml** | what JavaScript parses YAML with, run on Bun; a third independent judge, and the one that follows YAML 1.2's core schema where the others follow 1.1 |
| **go-yaml v3** | models comments (`HeadComment`, `LineComment`, `FootComment`) and is what `yq` edits through, so it is the implementation yqr is most often compared against in practice |
| **ruamel.yaml** | the round-trip editor: keeps comments, quoting and key order. The closest peer to what yqr does, and the only other one whose whole point is editing without reformatting |

The first three answer *what does this mean*. The last two also answer
*where does this comment go*, which is the gap that prompted the work.

`yq` is deliberately **not** an adapter. It is a tool over go-yaml with
its own printing and its own edit semantics, and testing it would measure
yq's choices rather than the library's. The difference is not academic:
deleting an entry through go-yaml's node API takes the head comment with
it, and deleting through `yq` leaves it behind.

## 4. Running it

```bash
python3 testbed/run.py            # rewrite testbed/answers.md
python3 testbed/run.py --check    # fail if the answers have drifted
```

`--check` compares **per implementation, and only against its own
version**. That distinction is the whole design, and the first run in CI
is what taught it: the machine that wrote the record had Psych 3.1.0,
the runner had 5.1.2, and Psych 5 emits `k:` where Psych 3 emits `k: `
with a trailing space. Comparing the files wholesale made a true and
uninteresting fact — two machines have different Rubies — indistinguishable
from the thing worth catching.

So there are three outcomes, and only one of them is an error:

| situation | outcome |
|---|---|
| same version, same answer | compared, silent |
| **same version, different answer** | **drift, exit 1**: a library changed behaviour without changing version, or the record was edited |
| different version | reported, not an error, and the answers that differ are named |

The third line matters as much as the second. An upgrade changing an
answer a spec leans on is exactly what the record exists to surface, and
it must not be silent just because it cannot honestly be an error. A
missing implementation reports as missing rather than failing, so a
partial answer is still an answer.

It is **not** in the pull-request CI job, which stays a single Rust build;
five language runtimes on every commit would buy a class of failure that
has nothing to do with yqr. It runs from `.github/workflows/testbed.yml`
on demand, weekly, and on any push touching `testbed/`, which is the
cadence the evidence actually changes at.

## 5. What the first run settled

Recorded in full in `testbed/answers.md`. The four that mattered
immediately:

- **A comment after a colon with no value belongs to the entry.** go-yaml
  files it under the key's line comment, ruamel after the value, and both
  carry it away when the entry is deleted. That is the open question in
  noyalib#425, and it now has two independent answers agreeing with the
  position the filing argues.
- **A value at its key's own column is not a document anybody reads.** All
  five refuse `on:` / `[]` and `k:` / `5`. That is `yqr-b014`'s class and
  `yqr-b029`'s corrupt output, and the unanimity is why `validate`'s `Y103`
  is on by default rather than behind `--strict`.
- **An emptied block should be written out.** Deleting the sole entry
  gives `spec: {}` in both editing implementations, which is what
  `delete_entry` does. `yqr-f016` §5 argued that from first principles; it
  now has company.
- **Byte fidelity is the minority position, and the editors hold it.**
  `0640`, `1.10` and `007` survive a round trip in go-yaml and ruamel and
  are canonicalized by PyYAML, Psych and js-yaml. The split is exactly
  along the editing/processing line `yqr-a001` draws.

One divergence worth knowing: js-yaml reads `0640` as **640** where
PyYAML, Psych and go-yaml read **416**. That is YAML 1.2's core schema
against 1.1's octal, and it is a real interoperability hazard in a
Kubernetes file, where `0640` is a permission.

## 6. Adding a case

A case is worth adding when a decision turned on what other
implementations do. Give it an `id`, the `source`, the `ops`, and a `why`
that names the decision, then re-run. The `why` is the part that ages
well: a year later the answer is only useful if the question is still
legible.
