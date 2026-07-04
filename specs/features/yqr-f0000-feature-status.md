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

Progress: seam + backend C (noyalib) shipped behind `backend-noyalib`; backend A
tracks [rust-yaml#73](https://github.com/elioetibr/rust-yaml/pull/73).

## Summary

- Total features: 2
- In Progress: 1
- Done: 1
