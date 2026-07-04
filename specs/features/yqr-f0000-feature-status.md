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
| [f003](yqr-f003-fidelity-backend-a-rustyaml.md) | Fidelity backend A (rust-yaml fork `RoundTripDocument` adapter) | Done |

Progress: seam + backend C (noyalib) shipped behind `backend-noyalib` (f002);
backend A (rust-yaml fork `RoundTripDocument`, the rust-yaml#73 substrate)
shipped behind `backend-rust-yaml` (f003). Both round-trip the b001 corpus
byte-for-byte.

## Summary

- Total features: 3
- In Progress: 1
- Done: 2
