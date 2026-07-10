# yqr.b005 — `cargo audit` fails on `crossbeam-epoch` advisory (via `criterion`)

**Status:** Open
**Severity:** Low
**Related:** `yqr-m001` (CI/release), `.agent/skills/dep-upgrade`

## Summary

`bash .github/scripts/local-ci.sh` (and the `security` CI job) fails at the
`cargo audit` step with **RUSTSEC-2026-0204**:

```
Crate:     crossbeam-epoch
Version:   0.9.18
Title:     Invalid pointer dereference in `fmt::Pointer` impl for `Atomic` and
           `Shared` when the underlying pointer is invalid
Solution:  Upgrade to >=0.9.20
Dependency tree:
crossbeam-epoch 0.9.18
└── crossbeam-deque 0.8.6
    └── rayon-core 1.13.0
        └── rayon 1.12.0
            └── criterion 0.8.2
                └── yqr 0.3.0
```

## Impact

- **Dev-dependency only.** `crossbeam-epoch` reaches yqr solely through
  `criterion` (the benchmark harness). It is not linked into the shipped `yqr`
  binary or the library, so end users are unaffected.
- The vulnerable path is the `fmt::Pointer` debug formatting of crossbeam's
  `Atomic`/`Shared` with an already-invalid pointer — not a path the benchmark
  code exercises.
- Pre-existing: the advisory was published 2026-07-06 and lands via the advisory
  DB, independent of any yqr code change. CI on `main` fails on it until the
  lockfile is refreshed.

## Fix

Refresh the transitive pin so `crossbeam-epoch >= 0.9.20` is selected:

```bash
cargo update -p crossbeam-epoch
cargo audit        # expect clean
bash .github/scripts/local-ci.sh
```

If `cargo update -p crossbeam-epoch` cannot reach `>=0.9.20` under the current
`criterion`/`rayon` versions, bump `rayon`/`criterion` (dev-dependency) enough to
pull a fixed `crossbeam-epoch`, one crate at a time per the `dep-upgrade` skill,
and re-run the quality gate.

## Acceptance criteria

- [ ] `cargo audit` reports no vulnerabilities.
- [ ] `Cargo.lock` selects `crossbeam-epoch >= 0.9.20`.
- [ ] `bash .github/scripts/local-ci.sh` is fully green.
