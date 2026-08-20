# Implementation m006 — House writing style

**Status:** In Progress — adopted 2026-08-20; applies to text written from now
on, not retroactively (§4)
**Owner:** yqr maintainers
**Last updated:** 2026-08-20
**Related:** `yqr-f010` (the website this governs), `yqr-f012` (CLI
diagnostics), ground rules 12, 15 and 19 in `AGENT.md` (what to document,
where, and what must not leak into it)

## 1. Decision

yqr follows the [Google developer documentation style
guide](https://developers.google.com/style) for every piece of text it
produces: specs, docs pages, CLI output, commit messages, PR bodies, and code
comments.

The guide's own framing applies here too — it says *"This guide contains
guidelines, not rules. Depart from it when doing so improves your content."*
So does this spec. Consistency matters more than compliance.

## 2. What this means in practice

Six things carry most of the weight:

- **Second person, active voice, present tense.** "Run the filter", not "the
  filter should be run" or "the filter will be run".
- **Say it once, in the fewest words that stay accurate.** Cut throat-clearing
  ("it is worth noting that", "in order to"). Prefer the short word.
- **Sentence case for headings.** "Editing a comment", not "Editing A
  Comment".
- **Plain language over jargon**, and define a term the first time it appears.
- **Lead with the point.** The first sentence of a section says what the
  section is for.
- **Concrete over abstract.** A measured example beats a description of one.

## 3. Where accuracy outranks brevity

Concise is not the same as terse, and this is the line yqr draws.

The specs under `specs/` exist to record *why* a decision was made, what was
measured, and what was deliberately not done. That reasoning is the artifact;
cutting it to save words makes the spec cheaper to read and worthless to use.
So:

- **Keep the argument.** A decision without its reason has to be re-litigated.
- **Keep the measurement.** Numbers, commands run, versions checked.
- **Keep what was declined, and why.** Ground rule 20's principle applied to
  design: an unstated limit reads as an oversight.
- **Cut the padding around all three.** Brevity is won on how something is
  said, not on dropping what is said.

A spec that states a finding in one sentence instead of three is better. One
that omits the finding is not.

## 4. Scope

Applies to text written from now on. Existing specs are **not** rewritten:
churning a hundred markdown files produces a large diff, no behaviour change,
and a worse git blame. Edit a document's style when you are already editing it
for another reason.

## 5. Verification

There is no linter for this. The check is review — the writing gate is the
same one the code has, which is that another reader has to be able to use it.
