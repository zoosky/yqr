# Domain docs

How the engineering skills consume this repo's domain documentation when
exploring the codebase.

## Before exploring, read these

- **`GLOSSARY.md`** at the repo root
- **`specs/architecture/yqr-aNNN-*.md`**: this repo's architecture decision
  records. Read the ones that touch the area you're about to work in.

If `GLOSSARY.md` doesn't exist, **proceed silently**. Don't flag its
absence; don't suggest creating it upfront. The `/domain-modeling` skill
creates it lazily when terms actually get resolved.

## File structure

Single-context repo:

```
/
├── GLOSSARY.md
└── specs/architecture/
    ├── yqr-a001-....md
    └── yqr-a002-....md
```

New decisions recorded via `/domain-modeling` are written as the next free
`specs/architecture/yqr-aNNN-*.md` spec with a `**Status:**` field
(ground rule 16), not as `docs/adr/NNNN-*.md` files.

## Use the glossary's vocabulary

When your output names a domain concept (in a spec title, a refactor
proposal, a hypothesis, a test name), use the term as defined in
`GLOSSARY.md`. Don't drift to synonyms the glossary explicitly avoids.

If the concept you need isn't in the glossary yet, that's a signal: either
you're inventing language the project doesn't use (reconsider) or there's a
real gap (note it for `/domain-modeling`).

## Flag ADR conflicts

If your output contradicts an existing architecture spec, surface it
explicitly rather than silently overriding:

> _Contradicts yqr-a001 (byte-for-byte fidelity), but worth reopening
> because…_
