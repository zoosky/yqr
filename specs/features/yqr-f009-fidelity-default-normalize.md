# yqr.f009 — Byte fidelity by default; classic pipeline behind `--normalize`

**Status:** Done
**Epic:** Fidelity-first architecture (a001)
**Owner:** yqr maintainers
**Related:** `yqr-f002` (fidelity read floor / engine seam), `yqr-f005`
(the `--preserve`/`--engine` split — superseded by this spec), `yqr-f006`
(write tier — already fidelity-by-default), `yqr-b001` (the lossy-default bug
this closes), `yqr-a001` (Fidelity-First architecture)

## 1. Problem

The read path defaulted to the *lossy* classic pipeline and hid byte fidelity
behind an opt-in `--preserve` (`-p`) flag. That inverted the product's own
promise. Two concrete symptoms:

1. **The brand is fidelity, but first contact was lossy.** `yqr-a001` ratifies
   "yqr never rewrites bytes it did not change" as the top priority. Yet a new
   user running `yqr '.' config.yaml` got comments stripped, `007` retyped to
   `7`, and quoting normalized — the exact behaviour `b001` files as a
   High-severity bug.
2. **The tool contradicted itself.** The *write* tier (`yqr-f006`) is already
   byte-faithful by default — a mutating filter always runs through the fidelity
   engine, no flag required. Reads defaulting to lossy while writes defaulted to
   faithful was an asymmetry that could not be explained to users.

Because noyalib is both the classic pipeline's parser/emitter **and** the
lossless CST behind the fidelity read, byte preservation was never a heavyweight
alternate path — it is the native mode of the engine already running. The
opt-in flag was scaffolding from incremental delivery (`f002` → `f005`), not an
architectural boundary.

## 2. Design

Invert the default and rename the opt-out.

| Before (`f005`) | After (`f009`) |
|-----------------|----------------|
| `yqr '.'` → re-serialized (lossy) | `yqr '.'` → byte-preserving (fidelity) |
| `yqr -p '.'` → byte-preserving | `--preserve` / `-p` removed |
| (no opt-out) | `yqr --normalize '.'` → re-serialized (lossy) |
| `--engine` names the backend for `-p` reads | `--engine` names the backend for the **default** read |

- **Default read = fidelity.** The query path runs `fidelity::run_ast`:
  untouched nodes are emitted as their original source bytes; computed, absent,
  and unaddressable nodes fall back to typed rendering **per node** (the
  existing `Resolved::Synthetic`/`Absent`/`Unaddressable` seam).
- **`--normalize` (short `-N`) = classic pipeline.** Opts into `eval_ast_str` +
  `render`, which re-serializes from the typed value (comments dropped, scalars
  canonicalized). This is the previous default, unchanged, now explicit.
- **`--engine <name>`** selects the backend parser for the default
  byte-preserving read (default `noyalib`). Under `--normalize` the classic
  pipeline is backend-independent, so the engine choice has no observable effect
  beyond the up-front name validation. Unknown names are still diagnosed before
  input is read.
- **Writes are unchanged.** A mutating filter (`=`, `+=`, `del`, `-i`) always
  goes through the fidelity write path, independent of `--normalize`.

### No new error surface

Flipping the default does **not** make any previously-succeeding read start
failing. The classic pipeline (`from_str::<Value>`) and the fidelity read
(`NoyalibEngine::open` + `value`) share the same noyalib value model, so the
narrow whole-document refusals (e.g. distinct keys colliding after string
conversion, `1` vs `"1"`) already fail *identically* under both paths (verified:
exit 5, same message). Per-node non-representability (merges via `<<`, aliases,
special-character keys) degrades visibly to typed rendering rather than erroring.
`--normalize` remains available as the escape hatch for any input a user would
rather see re-serialized.

### Clean break (no back-compat shim)

Pre-1.0, `--preserve` / `-p` are **removed**, not deprecated. clap rejects them
(exit 2) rather than silently accepting. All first-party callers (demo, tests,
docs, README, landing page) move to the no-flag default, and to `--normalize`
where they specifically exercise the re-serializing pipeline.

```
# OLD: yqr -p '.'          -> preserved bytes
#      yqr '.'             -> re-serialized (lossy)
# NEW: yqr '.'             -> preserved bytes (default)
#      yqr --normalize '.' -> re-serialized (lossy)
#      yqr --engine noyalib '.' -> preserved bytes, explicit backend
```

## 3. Acceptance criteria

- [x] `yqr '.' file` reproduces `file` byte-for-byte with **no flag** (comments,
      blank lines, quoting, block scalars, CRLF, BOM, multi-document all survive).
- [x] `yqr '.zip'` on `zip: 007` prints `007` by default (original spelling kept).
- [x] `--normalize` (and its short flag `-N`) runs the classic pipeline:
      comments dropped, `007` → `7`.
- [x] `--engine <name>` selects the backend for the default read; `--engine
      noyalib '.'` equals bare `'.'`. Under `--normalize` the engine choice has
      no observable effect beyond validation.
- [x] An unknown `--engine` value is diagnosed before input is read (exit 5).
- [x] `--preserve` and `-p` are removed; clap rejects them (non-zero exit).
- [x] Mutating filters remain byte-faithful by default, independent of
      `--normalize`.
- [x] `--help` documents `--normalize` and `--engine` with no internal spec
      references.
- [x] Demo, black-box CLI tests, README fidelity section, the CHANGELOG, and
      `docs/content/home.html` reflect the flipped default.
