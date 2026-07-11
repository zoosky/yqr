# yqr.f005 — Decouple byte/comment preservation from backend selection (`--preserve`)

**Status:** Superseded (`yqr-f009`)
**Epic:** Fidelity-first architecture (a001)
**Owner:** yqr maintainers
**Related:** `yqr-f002` (fidelity read floor / engine seam), `yqr-f004` (engine
parity — superseded), `yqr-f009` (fidelity-by-default; removes `--preserve`),
`yqr-m005` (single-engine consolidation)

> **Superseded by `yqr-f009`.** This spec introduced `--preserve` / `-p` as an
> opt-in for byte fidelity, keeping the classic re-serializing pipeline as the
> default. `yqr-f009` inverts that: byte fidelity is now the **default** read and
> the classic pipeline moved behind `--normalize`, so `--preserve` / `-p` were
> removed. The `--engine` design below carries over unchanged (it now selects the
> backend for the *default* read). The rest of this document is retained as the
> historical record of the two-flag split it replaced.

## 1. Problem

The `--engine <name>` flag conflated two orthogonal concerns:

1. **Which** YAML backend parses the input (a library choice: `noyalib`, and
   the experimental `skald`).
2. **Whether** untouched nodes are emitted as their original source bytes
   (the byte/comment-preserving "fidelity" mode).

In the shipped binary the *presence* of `--engine` switched on preservation and
its *value* chose the backend. Because noyalib is both the default parser and
the only fidelity backend, `--engine noyalib` was in practice just a verbose way
to say "preserve formatting". That is the wrong mental model: selecting a
backend and asking for byte fidelity are independent decisions, and a user who
wants preservation should not have to know a backend name to get it.

## 2. Design

Split the single flag into two orthogonal ones:

| Flag | Meaning |
|------|---------|
| `--preserve` / `-p` | Turn on byte/comment-preserving reads (fidelity mode). |
| `--engine <name>` | Select the backend used for `--preserve` reads. Defaults to `noyalib`. |

- `--preserve` alone uses the default backend (`noyalib`) — the common case.
- `--engine <name>` names the backend for the preserve path. The classic
  (re-serializing) pipeline is backend-independent today, so `--engine` without
  `--preserve` has no observable effect beyond validating the name.
- Unknown engine names are still diagnosed **before** any input is read, so a
  typo fails fast. Engine *availability* (built into this binary) is checked
  when the engine is opened, i.e. under `--preserve`.

### Clean break (no back-compat shim)

Pre-1.0 (0.2.x), `--engine noyalib` **no longer** implies preservation. The old
one-flag behavior is removed rather than deprecated, and all first-party
callers (demo, tests, docs) move to `--preserve`.

```
# OLD: yqr --engine noyalib '.'   -> preserved bytes
# NEW: yqr --preserve '.'          -> preserved bytes (default backend)
#      yqr --engine noyalib '.'    -> re-serialized (no-op vs default)
#      yqr -p --engine noyalib '.' -> preserved bytes, explicit backend
```

## 3. Acceptance criteria

- [x] `--preserve` / `-p` turns on byte-preserving reads with the default
      (noyalib) backend; `yqr -p '.' file` reproduces `file` byte-for-byte.
- [x] `--engine` selects the backend for the preserve path and defaults to
      noyalib; `-p --engine noyalib` equals bare `-p`.
- [x] `--engine <name>` **without** `--preserve` runs the classic pipeline
      (re-serializes; comments dropped).
- [x] An unknown `--engine` value is diagnosed before input is read (exit 5).
- [x] `--help` documents both flags with the split responsibilities and no
      internal spec references.
- [x] Demo, black-box CLI tests, the README byte-preserving section, and the
      `docs/content/home.html` landing page use `--preserve`; the landing page
      links to the runnable demo.
