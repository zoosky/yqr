# The reference testbed

Five YAML implementations, one protocol, one recorded answer sheet.

```bash
python3 run.py            # rewrite answers.md
python3 run.py --check    # fail if the answers have drifted
```

When a design question turns on what other implementations do — whose
comment is this, is that document one anybody else reads, what should an
emptied block become — this is where the answer comes from, and
`answers.md` is what it said, on which versions, on which day.

It is not a conformance suite. yqr disagrees with these libraries on
purpose in places, and the disagreements are the interesting part.

The full rationale, the case-writing convention and what the first run
settled are in `specs/implementation/yqr-m007-reference-testbed.md`.
