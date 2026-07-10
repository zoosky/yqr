# Feature Status Tracker

Single source of truth for the state of every `yqr-fNNN` feature spec. Update
this file in the same change that advances a feature (CLAUDE.md rule 17).

**Status legend:** Draft · In Progress · Done · Superseded · Historical

## Epic: jq-style YAML processor (f001)

| Feature | Title | Status |
|---------|-------|--------|
| [f001](yqr.f001-yaml-jq-m0.md) | yqr: a Swiss Army knife for YAML (M0 foundation) | In Progress (M0 done; M1+ open) |

Progress: M0 foundation landed (lexer/parser/eval/CLI, tests, CI); M1-M4 open.

## Epic: Fidelity-first architecture (a001)

| Feature | Title | Status |
|---------|-------|--------|
| [f002](yqr-f002-fidelity-read-floor.md) | Fidelity read floor (`FidelityEngine` seam + noyalib backend) | Done |
| [f003](yqr-f003-fidelity-backend-a-rustyaml.md) | Fidelity backend A (rust-yaml fork `RoundTripDocument` adapter) | Superseded (`yqr-m005`) |
| [f004](yqr-f004-engine-parity-runtime-switch.md) | Engine parity: both backends default-on and runtime-switchable, from the zoosky forks | Superseded (`yqr-m005`) |
| [f005](yqr-f005-preserve-flag-decouple.md) | Decouple byte/comment preservation from backend selection (`--preserve`) | Done |

Progress: the `FidelityEngine` seam + the noyalib CST backend shipped (f002).
The rust-yaml fork backend (f003) and the two-engine parity/runtime-switch story
(f004) were **superseded** when yqr consolidated on noyalib as its sole YAML
engine (`yqr-m005`) — removing the rust-yaml dependencies and unblocking the
crates.io publish. noyalib round-trips the b001 corpus byte-for-byte. The
byte-preserving read is now driven by its own `--preserve` flag, with `--engine`
reduced to backend selection (f005).

## Summary

- Total features: 5
- In Progress: 1
- Done: 2
- Superseded: 2 (f003, f004 — single-engine consolidation, `yqr-m005`)
